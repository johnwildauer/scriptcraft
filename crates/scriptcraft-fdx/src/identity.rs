//! Shared FDX identity-name and raw attribute mapping helpers.

use crate::constants::FDX_ROOT;
use crate::diagnostics::{map_preservation_error, FdxError};
use crate::reader::{attribute, RawAttribute, RawElement, RawNode};
use crate::{content, MAX_IMAGE_BYTES, MAX_TOTAL_IMAGE_BYTES};
use scriptcraft_model::preservation::{
    is_user_paragraph_data_lookalike, BoardEntity, BoardEntityKind, BoardGraph, ForeignIdDomain,
    ForeignIdentity, ForeignReference, ReferencePolicy, SectionKind,
};
use std::collections::{BTreeMap, BTreeSet};

/// Map a raw FDX element name to its established foreign identity domain.
pub(crate) fn identity_domain(name: &str) -> ForeignIdDomain {
    match name {
        "FinalDraft" | "DocumentRef" => ForeignIdDomain::Document,
        "Paragraph" => ForeignIdDomain::Paragraph,
        "SceneProperties" => ForeignIdDomain::Scene,
        "SceneArcBeat" | "Beat" => ForeignIdDomain::Beat,
        "CastMember" => ForeignIdDomain::CastMember,
        "Character" => ForeignIdDomain::Character,
        "ListItem" => ForeignIdDomain::ListItem,
        "DisplayBoard" => ForeignIdDomain::Board,
        "Image" => ForeignIdDomain::Image,
        "Revision" => ForeignIdDomain::Revision,
        "Tag" => ForeignIdDomain::Tag,
        other => ForeignIdDomain::Other(other.to_owned()),
    }
}

/// Return a unique unqualified identity attribute, matching the existing
/// case-insensitive raw FDX policy.
pub(crate) fn raw_identity_attribute(element: &RawElement) -> Option<&str> {
    raw_attribute_ci(&element.attributes, "id")
}

/// Count all unqualified case-insensitive identity attributes, including
/// duplicates so callers can reject ambiguous raw identity declarations.
pub(crate) fn raw_identity_attribute_count(element: &RawElement) -> usize {
    element
        .attributes
        .iter()
        .filter(|attribute| {
            attribute.name.namespace_uri.is_none()
                && attribute.name.local_name.eq_ignore_ascii_case("id")
        })
        .count()
}

/// Return a unique unqualified, case-insensitive attribute value.
pub(crate) fn raw_attribute_ci<'a>(attributes: &'a [RawAttribute], name: &str) -> Option<&'a str> {
    let mut found = None;
    for attribute in attributes {
        if attribute.name.namespace_uri.is_none()
            && attribute.name.local_name.eq_ignore_ascii_case(name)
        {
            if found.is_some() {
                return None;
            }
            found = Some(attribute.value.as_str());
        }
    }
    found
}

fn is_fdx_element(element: &RawElement, name: &str, root_ns: Option<&str>) -> bool {
    element.name.local_name == name && element.name.namespace_uri.as_deref() == root_ns
}

/// Validate bounded UserParagraphData context without interpreting unsupported
/// placements.
pub(crate) fn validate_user_paragraph_data_locations(
    root: &RawElement,
    root_namespace: Option<&str>,
) -> Result<(), FdxError> {
    fn visit(
        element: &RawElement,
        parent: Option<&RawElement>,
        grandparent: Option<&RawElement>,
        parent_accepts_attachment: bool,
        depth: usize,
        section: SectionKind,
        in_title_content: bool,
        root_namespace: Option<&str>,
    ) -> Result<(), FdxError> {
        if is_user_paragraph_data_lookalike(&element.name) {
            // Correctly placed attachments retain their established owner
            // relationship. Other occurrences remain generic opaque source.
            return Ok(());
        }

        let current_section = if depth == 1 && is_fdx_element(element, "Content", root_namespace) {
            SectionKind::Content
        } else if depth == 1 && is_fdx_element(element, "TitlePage", root_namespace) {
            SectionKind::TitlePage
        } else {
            section
        };
        let title_content = in_title_content
            || (depth == 2
                && current_section == SectionKind::TitlePage
                && is_fdx_element(element, "Content", root_namespace));
        let is_paragraph = is_fdx_element(element, "Paragraph", root_namespace);
        let body_owner = current_section == SectionKind::Content
            && depth == 2
            && is_paragraph
            && content::is_managed_direct_body_paragraph(element, root_namespace)?;
        let title_owner = current_section == SectionKind::TitlePage
            && is_paragraph
            && title_content
            && depth == 3;
        let dual_member_owner = current_section == SectionKind::Content
            && depth == 4
            && is_paragraph
            && parent.is_some_and(|parent| is_fdx_element(parent, "DualDialogue", root_namespace))
            && grandparent.is_some_and(|wrapper| {
                is_fdx_element(wrapper, "Paragraph", root_namespace)
                    && attribute(wrapper, "Type") == Some("General")
                    && wrapper.children.iter().any(|child| {
                        matches!(child, RawNode::Element(child)
                            if is_fdx_element(child, "DualDialogue", root_namespace))
                    })
            })
            && matches!(content::paragraph_type_key(element), Ok(Some(_)))
            && grandparent.is_some_and(|wrapper| {
                content::is_supported_dual_dialogue_source(wrapper, root_namespace)
            });
        let accepts_attachment = body_owner || title_owner || dual_member_owner;

        for child in &element.children {
            if let RawNode::Element(child) = child {
                visit(
                    child,
                    Some(element),
                    parent,
                    accepts_attachment,
                    depth.saturating_add(1),
                    current_section,
                    title_content,
                    root_namespace,
                )?;
            }
        }
        let _ = parent_accepts_attachment;
        Ok(())
    }

    visit(
        root,
        None,
        None,
        false,
        0,
        SectionKind::Other,
        false,
        root_namespace,
    )
}

/// Collect supported typed definitions and board references in source order.
/// FDX import uses these results to analyze source identities and board references.
pub(crate) fn collect_identities(
    root: &RawElement,
) -> Result<(Vec<ForeignIdentity>, BoardGraph), FdxError> {
    let mut identities = Vec::new();
    let mut board_graph = BoardGraph::default();
    let mut seen = BTreeSet::new();
    let mut scoped_title_page_paragraphs = BTreeMap::new();
    let mut definitions = Vec::new();
    walk_elements(root, &mut |element,
                              nearest_board,
                              in_title_page,
                              in_title_content,
                              depth,
                              is_alternate_copy| {
        if raw_identity_attribute_count(element) > 1 {
            return Err(FdxError::AmbiguousIdentity);
        }
        if matches!(
            element.name.local_name.as_str(),
            "ListItem" | "DisplayBoard" | "Image"
        ) && raw_identity_attribute(element).is_none()
        {
            return Err(FdxError::AmbiguousIdentity);
        }
        if nearest_board.is_some()
            && matches!(element.name.local_name.as_str(), "Item" | "Picture")
            && raw_identity_attribute(element).is_none()
        {
            return Err(FdxError::DanglingReference);
        }
        let Some(value) = raw_identity_attribute(element) else {
            return Ok(());
        };
        if nearest_board.is_some() && matches!(element.name.local_name.as_str(), "Item" | "Picture")
        {
            return Ok(());
        }
        if is_alternate_copy {
            // Exact AltCollection copies remain opaque and never compete
            // with global Paragraph definitions.
            return Ok(());
        }
        let domain = identity_domain(&element.name.local_name);
        let identity =
            ForeignIdentity::new(domain, value.to_owned()).map_err(map_preservation_error)?;
        let scoped_title_page_paragraph = in_title_page
            && in_title_content
            && element.name.local_name == "Paragraph"
            && depth == 3;
        if scoped_title_page_paragraph {
            *scoped_title_page_paragraphs
                .entry(identity.clone())
                .or_insert(0usize) += 1;
            definitions.push((
                element.name.local_name.clone(),
                identity,
                element.attributes.clone(),
                true,
            ));
            return Ok(());
        }
        if !seen.insert(identity.clone()) {
            return Err(FdxError::AmbiguousIdentity);
        }
        definitions.push((
            element.name.local_name.clone(),
            identity,
            element.attributes.clone(),
            false,
        ));
        Ok(())
    })?;

    // A single scoped title identity enters source-use accounting without
    // shadowing global definitions; repeated scoped occurrences coalesce.
    for (identity, count) in &scoped_title_page_paragraphs {
        if seen.contains(identity) {
            return Err(FdxError::AmbiguousIdentity);
        }
        if *count == 1 {
            seen.insert(identity.clone());
        }
    }

    let mut source_order = 0u32;
    let mut registered_repeated_title_identities = BTreeSet::new();
    for (name, identity, attributes, scoped_title_page) in &definitions {
        let repeated_scoped_title_page = *scoped_title_page
            && scoped_title_page_paragraphs
                .get(identity)
                .is_some_and(|count| *count > 1);
        if repeated_scoped_title_page
            && !registered_repeated_title_identities.insert(identity.clone())
        {
            continue;
        }
        let kind = match name.as_str() {
            "ListItem" => Some(BoardEntityKind::ListItem),
            "DisplayBoard" => Some(BoardEntityKind::DisplayBoard),
            "Image" => Some(BoardEntityKind::Image),
            _ => None,
        };
        if let Some(kind) = kind {
            source_order = source_order.checked_add(1).ok_or(FdxError::ResourceLimit)?;
            let source_path = raw_attribute_ci(attributes, "Path").map(str::to_owned);
            let declared_image_length = if kind == BoardEntityKind::Image {
                raw_attribute_ci(attributes, "Length")
                    .map(|value| value.parse::<u64>().map_err(|_| FdxError::InvalidValue))
                    .transpose()?
            } else {
                None
            };
            if declared_image_length.is_some_and(|length| length > MAX_IMAGE_BYTES as u64) {
                return Err(FdxError::ResourceLimit);
            }
            board_graph.entities.push(BoardEntity {
                identity: identity.clone(),
                kind,
                source_order,
                anchor: None,
                // Payload remains encoded in retained source; paths are data only.
                image_payload: None,
                declared_image_length,
                source_path,
            });
        } else {
            identities.push(identity.clone());
        }
    }

    let defined = definitions
        .iter()
        .filter(|(_, identity, _, scoped_title_page)| {
            !(*scoped_title_page
                && scoped_title_page_paragraphs
                    .get(identity)
                    .is_some_and(|count| *count > 1))
        })
        .map(|(_, identity, _, _)| identity.clone())
        .collect::<BTreeSet<_>>();
    let mut references = Vec::new();
    walk_elements(root, &mut |element, nearest_board, _, _, _, _| {
        let board = if element.name.local_name == "DisplayBoard" {
            raw_identity_attribute(element)
                .map(|value| ForeignIdentity::new(ForeignIdDomain::Board, value.to_owned()))
                .transpose()
                .map_err(map_preservation_error)?
        } else {
            nearest_board.cloned()
        };
        if let Some(source) = board {
            let target_domain = match element.name.local_name.as_str() {
                "Item" => Some(ForeignIdDomain::ListItem),
                "Picture" => Some(ForeignIdDomain::Image),
                _ => None,
            };
            if let Some(target_domain) = target_domain {
                if let Some(target_value) = raw_identity_attribute(element) {
                    let target = ForeignIdentity::new(target_domain, target_value.to_owned())
                        .map_err(map_preservation_error)?;
                    if !defined.contains(&target) {
                        return Err(FdxError::DanglingReference);
                    }
                    references.push(ForeignReference {
                        source,
                        target,
                        policy: ReferencePolicy::RequiredInternal,
                    });
                }
            }
        }
        Ok(())
    })?;
    board_graph.references = references;
    Ok((identities, board_graph))
}

fn walk_elements<F>(element: &RawElement, visitor: &mut F) -> Result<(), FdxError>
where
    F: FnMut(
        &RawElement,
        Option<&ForeignIdentity>,
        bool,
        bool,
        usize,
        bool,
    ) -> Result<(), FdxError>,
{
    fn classify(
        element: &RawElement,
        parent: Option<&RawElement>,
        grandparent: Option<&RawElement>,
        depth: usize,
        in_title_page: bool,
        in_title_content: bool,
        section: SectionKind,
        root_namespace: Option<&str>,
    ) -> content::SourceClass {
        let content_class = content::classify_preserved_element(
            element,
            depth,
            section,
            in_title_content,
            root_namespace,
        );
        if matches!(
            content_class,
            content::SourceClass::Managed
                | content::SourceClass::RecognizedAttachment
                | content::SourceClass::Structural
        ) {
            return content_class;
        }

        let Some(parent) = parent else {
            return if depth == 0 && is_fdx_element(element, FDX_ROOT, root_namespace) {
                content::SourceClass::Structural
            } else {
                content::SourceClass::GenericOpaque
            };
        };

        // Only the exact supported DualDialogue container beneath a direct
        // General body Paragraph is structural for identity traversal.
        if depth == 3
            && is_fdx_element(element, "DualDialogue", root_namespace)
            && is_fdx_element(parent, "Paragraph", root_namespace)
            && attribute(parent, "Type") == Some("General")
            && grandparent.is_some_and(|content_element| {
                is_fdx_element(content_element, "Content", root_namespace)
            })
            && content::is_supported_dual_dialogue_source(parent, root_namespace)
        {
            return content::SourceClass::Structural;
        }

        let body_parent = depth == 3
            && is_fdx_element(parent, "Paragraph", root_namespace)
            && matches!(content::paragraph_type_key(parent), Ok(Some(key)) if
                scriptcraft_model::element_definition::SemanticCapabilities::for_key(&key)
                    .scene_boundary)
            && grandparent
                .is_some_and(|content| is_fdx_element(content, "Content", root_namespace));
        if body_parent && is_fdx_element(element, "SceneProperties", root_namespace) {
            return content::SourceClass::KnownIdentity;
        }

        // Supported board entities and their required-reference children are
        // recognized only at these exact root/board paths.
        let root_child = depth == 1 && is_fdx_element(parent, FDX_ROOT, root_namespace);
        let board_container = root_child
            && matches!(
                element.name.local_name.as_str(),
                "DisplayBoards" | "ListItems" | "Images"
            );
        if board_container {
            return content::SourceClass::Structural;
        }
        if root_child
            && matches!(
                element.name.local_name.as_str(),
                "DisplayBoard" | "ListItem" | "Image"
            )
            && element.name.namespace_uri.as_deref() == root_namespace
        {
            return content::SourceClass::KnownIdentity;
        }
        let under_board_container = depth == 2
            && parent.name.namespace_uri.as_deref() == root_namespace
            && matches!(
                parent.name.local_name.as_str(),
                "DisplayBoards" | "ListItems" | "Images"
            );
        let board_entity = under_board_container
            && matches!(
                element.name.local_name.as_str(),
                "DisplayBoard" | "ListItem" | "Image"
            )
            && element.name.namespace_uri.as_deref() == root_namespace;
        if board_entity {
            return content::SourceClass::KnownIdentity;
        }
        if depth == 2
            && is_fdx_element(parent, "DisplayBoard", root_namespace)
            && matches!(element.name.local_name.as_str(), "Item" | "Picture")
            && element.name.namespace_uri.as_deref() == root_namespace
        {
            return content::SourceClass::KnownIdentity;
        }

        // Document lineage uses an exact FinalDraft -> DocumentRef -> XRef
        // edge; lookalike descendants elsewhere stay opaque.
        if depth == 1
            && is_fdx_element(parent, FDX_ROOT, root_namespace)
            && is_fdx_element(element, "DocumentRef", root_namespace)
        {
            return content::SourceClass::KnownIdentity;
        }
        if depth == 2
            && grandparent.is_some_and(|root| is_fdx_element(root, FDX_ROOT, root_namespace))
            && is_fdx_element(parent, "DocumentRef", root_namespace)
            && is_fdx_element(element, "XRef", root_namespace)
        {
            return content::SourceClass::KnownIdentity;
        }

        let inside_scene_properties = is_fdx_element(parent, "SceneProperties", root_namespace);
        let inside_scene_arcs = is_fdx_element(parent, "SceneArcBeats", root_namespace)
            && grandparent
                .is_some_and(|scene| is_fdx_element(scene, "SceneProperties", root_namespace));
        if inside_scene_properties && is_fdx_element(element, "SceneArcBeats", root_namespace) {
            return content::SourceClass::Structural;
        }
        if (inside_scene_properties
            && matches!(element.name.local_name.as_str(), "CastMember" | "Character")
            && element.name.namespace_uri.as_deref() == root_namespace)
            || (inside_scene_arcs
                && matches!(element.name.local_name.as_str(), "SceneArcBeat" | "Beat")
                && element.name.namespace_uri.as_deref() == root_namespace)
        {
            return content::SourceClass::KnownIdentity;
        }
        if depth == 5
            && is_fdx_element(parent, "SceneArcBeats", root_namespace)
            && grandparent
                .is_some_and(|scene| is_fdx_element(scene, "SceneProperties", root_namespace))
            && is_fdx_element(element, "CharacterArcBeat", root_namespace)
        {
            return content::SourceClass::Structural;
        }
        if depth == 6
            && is_fdx_element(parent, "CharacterArcBeat", root_namespace)
            && grandparent.is_some_and(|arcs| is_fdx_element(arcs, "SceneArcBeats", root_namespace))
            && is_fdx_element(element, "Paragraph", root_namespace)
        {
            return content::SourceClass::KnownIdentity;
        }
        if is_fdx_element(parent, "SceneArcBeat", root_namespace)
            && matches!(element.name.local_name.as_str(), "CastMember" | "Character")
            && element.name.namespace_uri.as_deref() == root_namespace
        {
            return content::SourceClass::KnownIdentity;
        }

        let dual_member = depth == 4
            && is_fdx_element(element, "Paragraph", root_namespace)
            && is_fdx_element(parent, "DualDialogue", root_namespace)
            && grandparent.is_some_and(|outer| {
                is_fdx_element(outer, "Paragraph", root_namespace)
                    && attribute(outer, "Type") == Some("General")
                    && content::is_supported_dual_dialogue_source(outer, root_namespace)
            })
            && matches!(content::paragraph_type_key(element), Ok(Some(_)));
        if dual_member {
            return content::SourceClass::Managed;
        }

        // Title flags never grant typed status on their own.
        let _ = in_title_page;
        content::SourceClass::GenericOpaque
    }

    fn visit<F>(
        element: &RawElement,
        parent: Option<&RawElement>,
        grandparent: Option<&RawElement>,
        nearest_board: Option<&ForeignIdentity>,
        in_title_page: bool,
        in_title_content: bool,
        depth: usize,
        section: SectionKind,
        root_namespace: Option<&str>,
        visitor: &mut F,
    ) -> Result<(), FdxError>
    where
        F: FnMut(
            &RawElement,
            Option<&ForeignIdentity>,
            bool,
            bool,
            usize,
            bool,
        ) -> Result<(), FdxError>,
    {
        let class = classify(
            element,
            parent,
            grandparent,
            depth,
            in_title_page,
            in_title_content,
            section,
            root_namespace,
        );
        if class == content::SourceClass::GenericOpaque
            || class == content::SourceClass::RecognizedAttachment
        {
            return Ok(());
        }
        let local_board = if element.name.local_name == "DisplayBoard"
            && element.name.namespace_uri.as_deref() == root_namespace
        {
            raw_identity_attribute(element)
                .map(|value| ForeignIdentity::new(ForeignIdDomain::Board, value.to_owned()))
                .transpose()
                .map_err(map_preservation_error)?
        } else {
            None
        };
        let active_board = local_board.as_ref().or(nearest_board);
        let exact_alternate_copy = element.name.local_name == "Paragraph"
            && element.name.namespace_uri.as_deref() == root_namespace
            && parent.is_some_and(|parent| {
                parent.name.local_name == "Alt"
                    && parent.name.namespace_uri.as_deref() == root_namespace
            })
            && grandparent.is_some_and(|grandparent| {
                grandparent.name.local_name == "AltCollection"
                    && grandparent.name.namespace_uri.as_deref() == root_namespace
            });
        let identity_bearing = class != content::SourceClass::Structural
            || (depth == 0 && is_fdx_element(element, FDX_ROOT, root_namespace))
            || (depth == 1 && is_fdx_element(element, "DocumentRef", root_namespace));
        if identity_bearing {
            visitor(
                element,
                active_board,
                in_title_page,
                in_title_content,
                depth,
                exact_alternate_copy,
            )?;
        }

        let child_section = if depth == 1 && is_fdx_element(element, "Content", root_namespace) {
            SectionKind::Content
        } else if depth == 1 && is_fdx_element(element, "TitlePage", root_namespace) {
            SectionKind::TitlePage
        } else {
            section
        };
        for child in &element.children {
            if let RawNode::Element(child) = child {
                let child_in_title_page = in_title_page
                    || (depth == 0 && is_fdx_element(child, "TitlePage", root_namespace));
                let child_in_title_content = in_title_content
                    || (in_title_page
                        && depth == 1
                        && is_fdx_element(element, "TitlePage", root_namespace)
                        && is_fdx_element(child, "Content", root_namespace));
                visit(
                    child,
                    Some(element),
                    parent,
                    active_board,
                    child_in_title_page,
                    child_in_title_content,
                    depth.saturating_add(1),
                    child_section,
                    root_namespace,
                    visitor,
                )?;
            }
        }
        Ok(())
    }

    visit(
        element,
        None,
        None,
        None,
        false,
        false,
        0,
        SectionKind::Other,
        element.name.namespace_uri.as_deref(),
        visitor,
    )
}

/// Enforce encoded image text budgets without decoding payloads or resolving
/// source paths.
pub(crate) fn validate_image_payload_budgets(
    root: &RawElement,
    root_namespace: Option<&str>,
) -> Result<(), FdxError> {
    fn text_size(element: &RawElement) -> Result<usize, FdxError> {
        let mut size = 0usize;
        for child in &element.children {
            match child {
                RawNode::Text(value) | RawNode::CData(value) => {
                    size = size
                        .checked_add(value.len())
                        .ok_or(FdxError::ResourceLimit)?;
                }
                RawNode::Element(element) => {
                    size = size
                        .checked_add(text_size(element)?)
                        .ok_or(FdxError::ResourceLimit)?;
                }
                RawNode::Comment(_) | RawNode::ProcessingInstruction { .. } => {}
            }
        }
        Ok(size)
    }

    let mut total_image_text = 0usize;
    walk_elements(root, &mut |element, _, _, _, _, _| {
        if !is_fdx_element(element, "Image", root_namespace) {
            return Ok(());
        }
        let encoded = text_size(element)?;
        let per_image_encoded_limit = MAX_IMAGE_BYTES
            .checked_mul(4)
            .and_then(|value| value.checked_add(2))
            .map(|value| value / 3)
            .ok_or(FdxError::ResourceLimit)?;
        total_image_text = total_image_text
            .checked_add(encoded)
            .ok_or(FdxError::ResourceLimit)?;
        let total_encoded_limit = MAX_TOTAL_IMAGE_BYTES
            .checked_mul(4)
            .and_then(|value| value.checked_add(2))
            .map(|value| value / 3)
            .ok_or(FdxError::ResourceLimit)?;
        if encoded > per_image_encoded_limit || total_image_text > total_encoded_limit {
            return Err(FdxError::ResourceLimit);
        }
        Ok(())
    })
}

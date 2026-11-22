//! Bounded raw XML ingress for FDX.

use crate::constants::FDX_ROOT;
use crate::constants::UTF8_BOM;
use crate::content::{self, ImportedSemanticParagraph};
use crate::diagnostics::{map_model_error, FdxError};
use crate::document::{FdxDocument, FdxVersionStatus};
use crate::identity::{
    collect_identities, validate_image_payload_budgets, validate_user_paragraph_data_locations,
};
use crate::lineage::raw_document_ref_datetime;
use crate::settings::{normalize_root_page_layout_colors, raw_root_page_layout};
use crate::text_state::raw_root_text_state;
use crate::title_page::make_title_page_paragraph;
use crate::tree::{preserve_misc_node, preserve_opaque_element};
use crate::window_state::raw_root_window_state;
use crate::{
    MAX_ATTRIBUTES_PER_ELEMENT, MAX_SOURCE_BYTES, MAX_SOURCE_DEPTH, MAX_SOURCE_EVENTS,
    MAX_SOURCE_NAME_OR_VALUE_BYTES, MAX_SOURCE_TEXT_BYTES,
};
use quick_xml::escape::unescape;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::{Namespace, ResolveResult};
use quick_xml::reader::NsReader;
use quick_xml::XmlVersion;
use scriptcraft_model::formatting::SourceLayoutSettings;
use scriptcraft_model::preservation::{
    is_user_paragraph_data_lookalike, AnchorRelation, AnchorTarget, DependencyConfidence,
    NamespaceBinding, PreservationState, PreservedAttribute, PreservedElement, PreservedNode,
    PreservedSection, SectionKind, SharedBytes, SourceAnchor, SourceAnchorId, SourceEncoding,
    SourceName, SourceOwnership, SourceSnapshot,
};
use scriptcraft_model::{BodyOwner, Project, SourceFormat};
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub(crate) struct RawAttribute {
    pub(crate) name: SourceName,
    pub(crate) value: String,
}

#[derive(Debug, Clone)]
pub(crate) struct RawElement {
    pub(crate) name: SourceName,
    pub(crate) attributes: Vec<RawAttribute>,
    pub(crate) namespace_bindings: Vec<NamespaceBinding>,
    pub(crate) children: Vec<RawNode>,
}

#[derive(Debug, Clone)]
pub(crate) enum RawNode {
    Element(RawElement),
    Text(String),
    CData(String),
    Comment(String),
    ProcessingInstruction { target: String, data: String },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceClass {
    Managed,
    RecognizedAttachment,
    KnownIdentity,
    Structural,
    GenericOpaque,
    InvalidUnsafe,
}

/// Classify source using the same structural and managed boundaries used by
/// retained-source projection and contextual identity collection.
pub(crate) fn classify_preserved_element(
    element: &RawElement,
    depth: usize,
    section: SectionKind,
    in_title_content: bool,
    root_ns: Option<&str>,
) -> SourceClass {
    if is_user_paragraph_data_lookalike(&element.name) {
        return SourceClass::RecognizedAttachment;
    }
    if (depth == 0 && is_fdx_element(element, FDX_ROOT, root_ns))
        || (depth == 1
            && (is_fdx_element(element, "Content", root_ns)
                || is_fdx_element(element, "TitlePage", root_ns)))
        || (depth == 2
            && section == SectionKind::TitlePage
            && is_fdx_element(element, "Content", root_ns))
    {
        return SourceClass::Structural;
    }
    if is_fdx_element(element, "Paragraph", root_ns) {
        let body = section == SectionKind::Content
            && depth == 2
            && content::is_managed_direct_body_paragraph(element, root_ns)
                .is_ok_and(|managed| managed);
        let title = section == SectionKind::TitlePage && in_title_content && depth == 3;
        if body || title {
            return SourceClass::Managed;
        }
    }
    SourceClass::GenericOpaque
}

/// Deserialize a bounded UTF-8 FDX Version 6 document without touching a session
/// or filesystem. DTDs and all non-predefined entities are rejected; no external
/// entity, URL, or recorded image path is resolved.
pub fn read_v6(input: &[u8]) -> Result<FdxDocument, FdxError> {
    if input.len() > MAX_SOURCE_BYTES {
        return Err(FdxError::SourceTooLarge);
    }
    std::str::from_utf8(input).map_err(|_| FdxError::InvalidEncoding)?;

    let parsed = parse_document(input)?;
    let (root_index, root) = parsed
        .iter()
        .enumerate()
        .find_map(|(index, node)| match node {
            RawNode::Element(element) => Some((index, element)),
            _ => None,
        })
        .ok_or(FdxError::InvalidRoot)?;
    let root_for_text_state = root.clone();
    if parsed
        .iter()
        .enumerate()
        .any(|(index, node)| index != root_index && matches!(node, RawNode::Element(_)))
    {
        return Err(FdxError::InvalidRoot);
    }
    if root.name.local_name != FDX_ROOT || attribute(root, "DocumentType") != Some("Script") {
        return Err(FdxError::InvalidRoot);
    }
    match crate::version::version_status(root)? {
        FdxVersionStatus::Version6 => {}
        FdxVersionStatus::ExplicitNon6 { .. } => return Err(FdxError::UnsupportedVersion),
        FdxVersionStatus::Missing => return Err(FdxError::MissingVersion),
        FdxVersionStatus::Malformed => return Err(FdxError::InvalidVersion),
        FdxVersionStatus::Duplicate => return Err(FdxError::DuplicateVersion),
    }
    let document_ref_datetime = raw_document_ref_datetime(root);
    let window_state = raw_root_window_state(root);

    let root_namespace = root.name.namespace_uri.as_deref();
    let imported_element_settings =
        crate::element_settings_import::read_element_settings(root, root_namespace)?;
    let mut content_count = 0usize;
    let mut title_count = 0usize;
    for child in &root.children {
        let RawNode::Element(element) = child else {
            continue;
        };
        if is_fdx_element(element, "Content", root_namespace) {
            content_count += 1;
        } else if is_fdx_element(element, "TitlePage", root_namespace) {
            title_count += 1;
        }
    }
    if content_count == 0 {
        return Err(FdxError::MissingContent);
    }
    if content_count > 1 || title_count > 1 {
        return Err(FdxError::DuplicateSection);
    }

    let page_layout = raw_root_page_layout(root);
    let mut project = Project::empty_projection_with_page_layout(page_layout)
        .map_err(map_model_error)?;
    crate::element_settings_import::complete_document_definitions(
        root,
        &imported_element_settings,
        &mut project,
    )?;
    let mut source_layout_settings = SourceLayoutSettings::default();
    validate_image_payload_budgets(root, root_namespace)?;
    validate_user_paragraph_data_locations(root, root_namespace)?;
    let (identities, board_graph) = collect_identities(root)?;
    let mut anchors = Vec::new();
    let mut next_id = 1usize;
    let mut next_anchor = 1usize;
    let mut root_owned = RawNode::Element(root.clone());
    if let RawNode::Element(root_owned) = &mut root_owned {
        crate::element_settings_import::normalize_winning_font_colors(
            root_owned,
            root_namespace,
            &imported_element_settings,
        )?;
        normalize_root_page_layout_colors(root_owned, root_namespace);
    }
    let preserved_root = preserve_node(
        &mut root_owned,
        0,
        SectionKind::Other,
        false,
        root_namespace,
        &mut project,
        &mut source_layout_settings,
        &mut anchors,
        &mut next_id,
        &mut next_anchor,
    )?;

    let mut preserved_children = parsed
        .into_iter()
        .enumerate()
        .map(|(index, node)| {
            if index == root_index {
                Ok(preserved_root.clone())
            } else {
                preserve_misc_node(node)
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let document_name = SourceName {
        namespace_uri: None,
        local_name: "Document".to_owned(),
        qualified_name: None,
    };
    let mut preservation = PreservationState::from_source_parts(
        SourceSnapshot {
            format: SourceFormat::Fdx,
            original_bytes: SharedBytes::source(input.to_vec()),
            encoding: SourceEncoding::Utf8,
        },
        source_layout_settings,
        vec![PreservedSection {
            name: document_name,
            kind: SectionKind::Other,
            ordinal: 0,
            children: std::mem::take(&mut preserved_children).into(),
        }],
        anchors,
        identities,
        Vec::new(),
        board_graph,
    );
    for element in &project.elements {
        if let Some(overrides) = preservation
            .source_layout_settings()
            .direct_paragraph_overrides
            .get(&element.id)
        {
            project
                .direct_paragraph_overrides
                .insert(element.id.clone(), overrides.clone());
        }
    }
    for group in &project.dual_dialogue_groups {
        if let Some(overrides) = preservation
            .source_layout_settings()
            .direct_paragraph_overrides
            .get(&group.id)
        {
            project
                .dual_dialogue_paragraph_overrides
                .insert(group.id.clone(), overrides.clone());
        }
    }
    preservation.rebuild_indexes();
    project.preservation = Some(preservation);
    let preservation = project
        .preservation
        .as_mut()
        .ok_or(FdxError::InvalidValue)?;
    crate::element_settings::anchor_document_element_settings(
        preservation,
        &imported_element_settings,
    )?;
    preservation.rebuild_indexes();
    project.validate().map_err(map_model_error)?;
    let text_state = raw_root_text_state(&root_for_text_state, &project);
    Ok(FdxDocument {
        project,
        import_diagnostics: imported_element_settings.diagnostics,
        document_ref_datetime,
        window_state,
        text_state,
    })
}

fn preserve_node(
    node: &mut RawNode,
    depth: usize,
    section: SectionKind,
    in_title_content: bool,
    root_ns: Option<&str>,
    project: &mut Project,
    source_layout_settings: &mut SourceLayoutSettings,
    anchors: &mut Vec<SourceAnchor>,
    next_id: &mut usize,
    next_anchor: &mut usize,
) -> Result<PreservedNode, FdxError> {
    let RawNode::Element(element) = node else {
        return preserve_misc_node(node.clone());
    };
    if is_user_paragraph_data_lookalike(&element.name) {
        return preserve_opaque_element(element.clone());
    }
    let current_section = if depth == 1 && is_fdx_element(element, "Content", root_ns) {
        SectionKind::Content
    } else if depth == 1 && is_fdx_element(element, "TitlePage", root_ns) {
        SectionKind::TitlePage
    } else {
        section
    };
    let paragraph = is_fdx_element(element, "Paragraph", root_ns);
    let title_content_paragraph =
        current_section == SectionKind::TitlePage && in_title_content && depth == 3 && paragraph;
    let body_paragraph = current_section == SectionKind::Content && depth == 2 && paragraph;

    // A supported dual wrapper is recognized only at the direct managed body
    // Paragraph boundary and only in the document namespace. Any unsupported
    // occurrence remains a complete opaque source subtree.
    let dual_children = element
        .children
        .iter()
        .filter_map(|child| match child {
            RawNode::Element(child) if is_fdx_element(child, "DualDialogue", root_ns) => {
                Some(child)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let has_dual_name = element.children.iter().any(
        |child| matches!(child, RawNode::Element(child) if child.name.local_name == "DualDialogue"),
    );
    if element.name.local_name == "DualDialogue" {
        return preserve_opaque_element(element.clone());
    }
    if !dual_children.is_empty() || has_dual_name {
        if !body_paragraph
            || attribute(element, "Type") != Some("General")
            || dual_children.len() != 1
            || has_dual_name != !dual_children.is_empty()
        {
            return preserve_opaque_element(element.clone());
        }
        let preserved = content::preserve_dual_dialogue_wrapper(
            element,
            root_ns,
            project,
            source_layout_settings,
            anchors,
            next_id,
            next_anchor,
        );
        return match preserved {
            Ok(preserved) => Ok(PreservedNode::Element(preserved)),
            // Unsupported topology is source, not a document-level import
            // failure. Identity ambiguity is checked separately in the
            // contextual identity pass before projection.
            Err(FdxError::UnsupportedConstruct) => preserve_opaque_element(element.clone()),
            Err(error) => Err(error),
        };
    }

    let managed = if title_content_paragraph {
        Some(ImportedSemanticParagraph::Title(make_title_page_paragraph(
            element,
            u32::try_from(project.title_page.len()).map_err(|_| FdxError::ResourceLimit)?,
            next_id,
        )?))
    } else if body_paragraph {
        if let Some(element_type) = content::paragraph_type_key(element)? {
            Some(ImportedSemanticParagraph::Body(
                content::make_semantic_element(element, element_type, current_section, next_id)?,
            ))
        } else {
            None
        }
    } else {
        None
    };
    let classification =
        classify_preserved_element(element, depth, current_section, in_title_content, root_ns);
    if managed.is_none() && classification != SourceClass::Structural {
        // Unknown and unsupported source is retained as one opaque boundary.
        // No descendant local name can escape this branch into projection.
        return preserve_opaque_element(element.clone());
    }
    let anchor_id = if managed.is_some() {
        let value = format!("fdx-anchor-{}", *next_anchor);
        *next_anchor = next_anchor.checked_add(1).ok_or(FdxError::ResourceLimit)?;
        Some(SourceAnchorId::new(value).map_err(|_| FdxError::InvalidValue)?)
    } else {
        None
    };
    let ownership = managed
        .as_ref()
        .map(|semantic| SourceOwnership::Managed(semantic.id().clone()))
        .unwrap_or(SourceOwnership::Opaque);
    let mut children = Vec::with_capacity(element.children.len());
    let child_in_title_content = in_title_content
        || (depth == 2
            && current_section == SectionKind::TitlePage
            && is_fdx_element(element, "Content", root_ns));
    for child in &mut element.children {
        children.push(preserve_node(
            child,
            depth.saturating_add(1),
            current_section,
            child_in_title_content,
            root_ns,
            project,
            source_layout_settings,
            anchors,
            next_id,
            next_anchor,
        )?);
    }
    let source_type = attribute(element, "Type").map(str::to_owned);
    let preserved = PreservedElement {
        name: element.name.clone(),
        attributes: element
            .attributes
            .iter()
            .map(|attribute| PreservedAttribute {
                name: attribute.name.clone(),
                value: attribute.value.clone(),
            })
            .collect(),
        namespace_bindings: element.namespace_bindings.clone(),
        source_type,
        ownership,
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: anchor_id.clone(),
        children: children.into(),
    };
    if let (Some(semantic), Some(anchor_id)) = (managed, anchor_id) {
        if let ImportedSemanticParagraph::Body(body) = &semantic {
            if let Some(overrides) = content::direct_paragraph_overrides(element) {
                source_layout_settings
                    .direct_paragraph_overrides
                    .insert(body.id.clone(), overrides);
            }
        }
        anchors.push(SourceAnchor {
            id: anchor_id,
            target: AnchorTarget::ProjectElement(semantic.id().clone()),
            relation: AnchorRelation::Represents,
        });
        match semantic {
            ImportedSemanticParagraph::Body(body) => project.elements.push(BodyOwner::shared(body)),
            ImportedSemanticParagraph::Title(title) => project.title_page.push(title),
        }
    }
    Ok(PreservedNode::Element(preserved))
}

fn is_fdx_element(element: &RawElement, name: &str, root_ns: Option<&str>) -> bool {
    element.name.local_name == name && element.name.namespace_uri.as_deref() == root_ns
}

pub(crate) fn parse_document(input: &[u8]) -> Result<Vec<RawNode>, FdxError> {
    let bytes = input.strip_prefix(UTF8_BOM).unwrap_or(input);
    let mut reader = NsReader::from_reader(bytes);
    let config = reader.config_mut();
    config.trim_text(false);
    config.check_end_names = true;
    config.allow_unmatched_ends = false;
    config.check_comments = true;
    reader
        .resolver_mut()
        .set_max_declarations_per_element(MAX_ATTRIBUTES_PER_ELEMENT);

    let mut document_nodes = Vec::new();
    let mut stack: Vec<RawElement> = Vec::new();
    let mut buffer = Vec::new();
    let mut event_count = 0usize;
    let mut retained_count = 0usize;
    let mut text_bytes = 0usize;
    loop {
        let event = reader
            .read_event_into(&mut buffer)
            .map_err(|_| FdxError::MalformedXml)?;
        event_count = event_count.checked_add(1).ok_or(FdxError::ResourceLimit)?;
        if event_count > MAX_SOURCE_EVENTS {
            return Err(FdxError::ResourceLimit);
        }
        match event {
            Event::Start(start) => {
                if stack.len() >= MAX_SOURCE_DEPTH {
                    return Err(FdxError::ResourceLimit);
                }
                let element = make_element(&reader, &start, stack.is_empty())?;
                add_retained_budget(
                    &mut retained_count,
                    1usize
                        .saturating_add(element.attributes.len())
                        .saturating_add(element.namespace_bindings.len()),
                )?;
                stack.push(element);
            }
            Event::Empty(start) => {
                let element = make_element(&reader, &start, stack.is_empty())?;
                add_retained_budget(
                    &mut retained_count,
                    1usize
                        .saturating_add(element.attributes.len())
                        .saturating_add(element.namespace_bindings.len()),
                )?;
                append_node(&mut stack, &mut document_nodes, RawNode::Element(element))?;
            }
            Event::End(_) => {
                let element = stack.pop().ok_or(FdxError::MalformedXml)?;
                append_node(&mut stack, &mut document_nodes, RawNode::Element(element))?;
            }
            Event::Text(text) => {
                let decoded = text.decode().map_err(|_| FdxError::InvalidEncoding)?;
                let value = unescape(&decoded).map_err(|_| FdxError::UnsupportedEntity)?;
                add_text_budget(&mut text_bytes, value.len())?;
                add_retained_budget(&mut retained_count, 1)?;
                validate_xml_chars(&value)?;
                append_node(
                    &mut stack,
                    &mut document_nodes,
                    RawNode::Text(value.into_owned()),
                )?;
            }
            Event::GeneralRef(reference) => {
                let reference = reference.decode().map_err(|_| FdxError::InvalidEncoding)?;
                let encoded = format!("&{reference};");
                let value = unescape(&encoded).map_err(|_| FdxError::UnsupportedEntity)?;
                add_text_budget(&mut text_bytes, value.len())?;
                add_retained_budget(&mut retained_count, 1)?;
                validate_xml_chars(&value)?;
                append_node(
                    &mut stack,
                    &mut document_nodes,
                    RawNode::Text(value.into_owned()),
                )?;
            }
            Event::CData(text) => {
                let value = text.decode().map_err(|_| FdxError::InvalidEncoding)?;
                add_text_budget(&mut text_bytes, value.len())?;
                add_retained_budget(&mut retained_count, 1)?;
                validate_xml_chars(&value)?;
                append_node(
                    &mut stack,
                    &mut document_nodes,
                    RawNode::CData(value.into_owned()),
                )?;
            }
            Event::Comment(comment) => {
                let value = comment.decode().map_err(|_| FdxError::InvalidEncoding)?;
                add_text_budget(&mut text_bytes, value.len())?;
                add_retained_budget(&mut retained_count, 1)?;
                validate_xml_chars(&value)?;
                append_node(
                    &mut stack,
                    &mut document_nodes,
                    RawNode::Comment(value.into_owned()),
                )?;
            }
            Event::PI(instruction) => {
                if instruction.target().len() > MAX_SOURCE_NAME_OR_VALUE_BYTES {
                    return Err(FdxError::ResourceLimit);
                }
                let target = utf8(instruction.target())?.to_owned();
                let data = reader
                    .decoder()
                    .decode(instruction.content())
                    .map_err(|_| FdxError::InvalidEncoding)?;
                add_text_budget(&mut text_bytes, target.len().saturating_add(data.len()))?;
                add_retained_budget(&mut retained_count, 1)?;
                let data = data.as_ref();
                let data = if data
                    .as_bytes()
                    .first()
                    .is_some_and(|byte| matches!(*byte, b' ' | b'\t' | b'\n' | b'\r'))
                {
                    &data[1..]
                } else {
                    data
                };
                validate_xml_chars(data)?;
                append_node(
                    &mut stack,
                    &mut document_nodes,
                    RawNode::ProcessingInstruction {
                        target,
                        data: data.to_owned(),
                    },
                )?;
            }
            Event::Decl(declaration) => validate_declaration(&declaration)?,
            Event::DocType(_) => return Err(FdxError::ForbiddenDoctype),
            Event::Eof => break,
        }
        buffer.clear();
    }
    if !stack.is_empty() {
        return Err(FdxError::MalformedXml);
    }
    let root_count = document_nodes
        .iter()
        .filter(|node| matches!(node, RawNode::Element(_)))
        .count();
    if root_count != 1 {
        return Err(FdxError::InvalidRoot);
    }
    add_retained_budget(&mut retained_count, 1)?; // Preservation section wrapper.
    Ok(document_nodes)
}

fn append_node(
    stack: &mut [RawElement],
    document_nodes: &mut Vec<RawNode>,
    node: RawNode,
) -> Result<(), FdxError> {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    } else {
        if let RawNode::Text(text) = &node {
            if !text.chars().all(char::is_whitespace) {
                return Err(FdxError::MalformedXml);
            }
        }
        document_nodes.push(node);
    }
    Ok(())
}

fn make_element<R: std::io::BufRead>(
    reader: &NsReader<R>,
    start: &BytesStart<'_>,
    document_root: bool,
) -> Result<RawElement, FdxError> {
    let (resolved, local) = reader.resolver().resolve_element(start.name());
    let (namespace_uri, local_name) = resolved_name(resolved, local.as_ref())?;
    if start.name().as_ref().len() > MAX_SOURCE_NAME_OR_VALUE_BYTES {
        return Err(FdxError::ResourceLimit);
    }
    let qualified_name = utf8(start.name().as_ref())?.to_owned();
    let mut attributes = Vec::new();
    let mut namespace_bindings = Vec::new();
    let mut seen_raw_attributes = BTreeSet::new();
    let mut seen_expanded_attributes = BTreeSet::new();
    for item in start.attributes().with_checks(false) {
        let attribute = item.map_err(|_| FdxError::MalformedXml)?;
        if attributes.len() + namespace_bindings.len() >= MAX_ATTRIBUTES_PER_ELEMENT {
            return Err(FdxError::ResourceLimit);
        }
        if attribute.key.as_ref().len() > MAX_SOURCE_NAME_OR_VALUE_BYTES
            || attribute.value.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES
        {
            return Err(FdxError::ResourceLimit);
        }
        let raw_name = utf8(attribute.key.as_ref())?.to_owned();
        if !seen_raw_attributes.insert(raw_name.clone())
            && (!document_root || raw_name != "Version")
        {
            return Err(FdxError::MalformedXml);
        }
        let (attribute_ns, attribute_local) = reader.resolver().resolve_attribute(attribute.key);
        let (attribute_namespace, local_name) =
            resolved_name(attribute_ns, attribute_local.as_ref())?;
        let value = attribute
            .decoded_and_normalized_value(XmlVersion::Implicit1_0, reader.decoder())
            .map_err(|_| FdxError::UnsupportedEntity)?;
        validate_xml_chars(&value)?;
        if raw_name == "xmlns" || raw_name.starts_with("xmlns:") {
            let prefix = raw_name.strip_prefix("xmlns:").unwrap_or("").to_owned();
            namespace_bindings.push(NamespaceBinding {
                prefix,
                namespace_uri: value.into_owned(),
            });
        } else {
            if !seen_expanded_attributes.insert((attribute_namespace.clone(), local_name.clone()))
                && (!document_root || attribute_namespace.is_some() || local_name != "Version")
            {
                return Err(FdxError::MalformedXml);
            }
            attributes.push(RawAttribute {
                name: SourceName {
                    namespace_uri: attribute_namespace,
                    local_name,
                    qualified_name: Some(raw_name),
                },
                value: value.into_owned(),
            });
        }
    }
    Ok(RawElement {
        name: SourceName {
            namespace_uri,
            local_name,
            qualified_name: Some(qualified_name),
        },
        attributes,
        namespace_bindings,
        children: Vec::new(),
    })
}

fn resolved_name(
    resolved: ResolveResult<'_>,
    local_name: &[u8],
) -> Result<(Option<String>, String), FdxError> {
    if local_name.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES {
        return Err(FdxError::ResourceLimit);
    }
    let namespace = match resolved {
        ResolveResult::Bound(Namespace(uri)) => Some(utf8(uri)?.to_owned()),
        ResolveResult::Unbound => None,
        ResolveResult::Unknown(_) => return Err(FdxError::MalformedXml),
    };
    Ok((namespace, utf8(local_name)?.to_owned()))
}

fn validate_declaration(declaration: &quick_xml::events::BytesDecl<'_>) -> Result<(), FdxError> {
    if let Some(encoding) = declaration.encoding() {
        let encoding = encoding.map_err(|_| FdxError::MalformedXml)?;
        let encoding = utf8(&encoding)?;
        if !encoding.eq_ignore_ascii_case("UTF-8") {
            return Err(FdxError::InvalidEncoding);
        }
    }
    Ok(())
}

fn add_text_budget(total: &mut usize, bytes: usize) -> Result<(), FdxError> {
    *total = total.checked_add(bytes).ok_or(FdxError::ResourceLimit)?;
    if *total > MAX_SOURCE_TEXT_BYTES {
        return Err(FdxError::ResourceLimit);
    }
    Ok(())
}

fn add_retained_budget(total: &mut usize, count: usize) -> Result<(), FdxError> {
    *total = total.checked_add(count).ok_or(FdxError::ResourceLimit)?;
    if *total > MAX_SOURCE_EVENTS {
        return Err(FdxError::ResourceLimit);
    }
    Ok(())
}

pub(crate) fn validate_xml_chars(value: &str) -> Result<(), FdxError> {
    if value.chars().all(|character| {
        matches!(character, '\u{9}' | '\u{a}' | '\u{d}')
            || ('\u{20}'..='\u{d7ff}').contains(&character)
            || ('\u{e000}'..='\u{fffd}').contains(&character)
            || ('\u{10000}'..='\u{10ffff}').contains(&character)
    }) {
        Ok(())
    } else {
        Err(FdxError::InvalidValue)
    }
}

fn utf8(bytes: &[u8]) -> Result<&str, FdxError> {
    std::str::from_utf8(bytes).map_err(|_| FdxError::InvalidEncoding)
}

pub(crate) fn attribute<'a>(element: &'a RawElement, name: &str) -> Option<&'a str> {
    let mut found = None;
    for attribute in &element.attributes {
        if attribute.name.namespace_uri.is_none() && attribute.name.local_name == name {
            if found.is_some() {
                return None;
            }
            found = Some(attribute.value.as_str());
        }
    }
    found
}

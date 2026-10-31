//! Bounded FDX Version 6 codec for the evidenced element subset.
//!
//! The reader returns a semantic editing projection plus the complete parsed
//! source tree and original bytes. The writer reconciles supported element edits
//! with that tree, or emits a minimal new document for the evidenced role labels.
//! ScriptCraft-only identity, schema, complete profile and edit-policy mappings
//! are not established; these APIs do not enable safe native save/reopen or claim
//! Final Draft interoperability.

use crate::preservation::{
    BoardEntity, BoardEntityKind, BoardGraph, DependencyConfidence, ForeignIdDomain,
    ForeignIdentity, ForeignReference, NamespaceBinding, PreservationError, PreservationState,
    PreservedAttribute, PreservedElement, PreservedNode, PreservedSection, ReferencePolicy,
    SectionKind, SourceAnchor, SourceAnchorId, SourceEncoding, SourceName, SourceOwnership,
    SourceSnapshot,
};
use crate::{
    ModelError, PageProfile, PrintVisibility, Project, Role, RunAttribute, ScreenplayElement,
    SourceFormat, SourceProvenance, SourceSection, StableId, TextRun, CURRENT_SCHEMA_VERSION,
};
use quick_xml::escape::unescape;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::{Namespace, ResolveResult};
use quick_xml::reader::NsReader;
use quick_xml::XmlVersion;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub use crate::preservation::{
    MAX_ATTRIBUTES_PER_ELEMENT, MAX_IMAGE_BYTES, MAX_SOURCE_BYTES, MAX_SOURCE_DEPTH,
    MAX_SOURCE_EVENTS, MAX_SOURCE_NAME_OR_VALUE_BYTES, MAX_SOURCE_TEXT_BYTES,
    MAX_TOTAL_IMAGE_BYTES,
};

const UTF8_BOM: &[u8] = b"\xef\xbb\xbf";
const FDX_ROOT: &str = "FinalDraft";
const FDX_VERSION: &str = "6";

/// Bounded, privacy-safe reader failures. Variants intentionally carry no source
/// text, identifiers, file paths, or image payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FdxError {
    SourceTooLarge,
    InvalidEncoding,
    MalformedXml,
    ForbiddenDoctype,
    UnsupportedEntity,
    UnsupportedVersion,
    MissingVersion,
    InvalidVersion,
    DuplicateVersion,
    InvalidRoot,
    MissingContent,
    DuplicateSection,
    ResourceLimit,
    InvalidValue,
    AmbiguousIdentity,
    DanglingReference,
    UnsupportedConstruct,
    UnsupportedAppState,
    Model(ModelErrorKind),
    Preservation(PreservationErrorKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelErrorKind {
    InvalidModel,
    InvalidIdentifier,
    InvalidText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreservationErrorKind {
    InvalidState,
    InvalidIdentity,
    InvalidAnchor,
    InvalidReference,
    ResourceLimit,
}

impl fmt::Display for FdxError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::SourceTooLarge => "FDX source exceeds the configured size limit",
            Self::InvalidEncoding => "FDX source is not valid UTF-8",
            Self::MalformedXml => "FDX XML is malformed or unsupported",
            Self::ForbiddenDoctype => "FDX document types are not accepted",
            Self::UnsupportedEntity => "FDX contains an unsupported entity reference",
            Self::UnsupportedVersion => "FDX is not declared as Version 6",
            Self::MissingVersion => "FDX version declaration is missing",
            Self::InvalidVersion => "FDX version declaration is not a decimal integer",
            Self::DuplicateVersion => "FDX contains duplicate version declarations",
            Self::InvalidRoot => "FDX root element is not a Script document",
            Self::MissingContent => "FDX Content section is missing",
            Self::DuplicateSection => "FDX contains ambiguous duplicate managed sections",
            Self::ResourceLimit => "FDX exceeds a configured parser resource limit",
            Self::InvalidValue => "FDX contains an invalid value",
            Self::AmbiguousIdentity => "FDX contains an invalid or ambiguous identity",
            Self::DanglingReference => "FDX contains an unresolved internal reference",
            Self::UnsupportedConstruct => {
                "FDX contains a construct that cannot be projected safely"
            }
            Self::UnsupportedAppState => "ScriptCraft-only state has no supported FDX mapping",
            Self::Model(_) => "FDX semantic projection failed model validation",
            Self::Preservation(_) => "FDX preservation state failed validation",
        })
    }
}

impl std::error::Error for FdxError {}

/// A Version 6 reader result. `project` is an editing projection only; it uses
/// ScriptCraft's default profile and a generated application ID where the FDX
/// samples do not establish reversible mappings. The source-owned preservation
/// state retains the FDX values independently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FdxDocument {
    pub project: Project,
}

/// Result of bounded version preflight. Non-6 numeric versions are explicit
/// migration candidates; the caller must present the migration warning and
/// obtain consent before invoking the conversion path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FdxVersionStatus {
    Version6,
    ExplicitNon6 {
        declared: String,
        direction: FdxVersionDirection,
    },
    Missing,
    Malformed,
    Duplicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FdxVersionDirection {
    Older,
    Newer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FdxMigrationConsent {
    Continue,
    Cancel,
}

/// Indicates the bounded conversion attempted for an imported candidate. This
/// is not a claim of native save/reopen safety or Final Draft compatibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FdxMigrationKind {
    OlderLossy { declared: String },
    NewerBestEffort { declared: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FdxImportCandidate {
    pub document: FdxDocument,
    pub migration: Option<FdxMigrationKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FdxImportOutcome {
    Cancelled { declared_version: String },
    Candidate(Box<FdxImportCandidate>),
}

/// Securely parse and classify the root version without building an editing
/// candidate. Call this before presenting a warning for an explicit non-6
/// declaration. Input is bounded and never written or resolved externally.
pub fn preflight_version(input: &[u8]) -> Result<FdxVersionStatus, FdxError> {
    if input.len() > MAX_SOURCE_BYTES {
        return Err(FdxError::SourceTooLarge);
    }
    std::str::from_utf8(input).map_err(|_| FdxError::InvalidEncoding)?;
    let parsed = parse_document(input)?;
    let root = parsed
        .iter()
        .find_map(|node| match node {
            RawNode::Element(element) => Some(element),
            _ => None,
        })
        .ok_or(FdxError::InvalidRoot)?;
    if root.name.local_name != FDX_ROOT || attribute(root, "DocumentType") != Some("Script") {
        return Err(FdxError::InvalidRoot);
    }
    version_status(root)
}

/// Import Version 6 directly, or attempt a typed migration after the caller has
/// shown the required warning and supplied explicit consent. Older files retain
/// only TitlePage and Content, with unrecognized paragraph types converted to
/// General. Newer files use the Version 6 importer best-effort and retain source
/// sections where representable. Neither path mutates the source or a session.
pub fn import_fdx(
    input: &[u8],
    consent: FdxMigrationConsent,
) -> Result<FdxImportOutcome, FdxError> {
    match preflight_version(input)? {
        FdxVersionStatus::Version6 => {
            Ok(FdxImportOutcome::Candidate(Box::new(FdxImportCandidate {
                document: read_v6(input)?,
                migration: None,
            })))
        }
        FdxVersionStatus::ExplicitNon6 {
            declared,
            direction,
        } => {
            if consent == FdxMigrationConsent::Cancel {
                return Ok(FdxImportOutcome::Cancelled {
                    declared_version: declared,
                });
            }
            let migrated = prepare_non_v6_source(input, direction)?;
            let document = read_v6(&migrated)?;
            // Require the Version 6 writer's complete bounded reconciliation and
            // reread before publishing any staged candidate.
            write_v6(&document.project)?;
            let migration = match direction {
                FdxVersionDirection::Older => FdxMigrationKind::OlderLossy { declared },
                FdxVersionDirection::Newer => FdxMigrationKind::NewerBestEffort { declared },
            };
            Ok(FdxImportOutcome::Candidate(Box::new(FdxImportCandidate {
                document,
                migration: Some(migration),
            })))
        }
        FdxVersionStatus::Missing => Err(FdxError::MissingVersion),
        FdxVersionStatus::Malformed => Err(FdxError::InvalidVersion),
        FdxVersionStatus::Duplicate => Err(FdxError::DuplicateVersion),
    }
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
    match version_status(root)? {
        FdxVersionStatus::Version6 => {}
        FdxVersionStatus::ExplicitNon6 { .. } => return Err(FdxError::UnsupportedVersion),
        FdxVersionStatus::Missing => return Err(FdxError::MissingVersion),
        FdxVersionStatus::Malformed => return Err(FdxError::InvalidVersion),
        FdxVersionStatus::Duplicate => return Err(FdxError::DuplicateVersion),
    }

    let root_namespace = root.name.namespace_uri.as_deref();
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

    let mut project = Project {
        id: StableId::new("fdx-projection").map_err(|_| FdxError::InvalidValue)?,
        schema_version: CURRENT_SCHEMA_VERSION,
        metadata: Default::default(),
        title_page: Vec::new(),
        page_profile: PageProfile::default(),
        elements: Vec::new(),
        preservation: None,
    };
    apply_profile_settings(root, root_namespace, &mut project.page_profile)?;
    let mut identities = Vec::new();
    let mut board_graph = BoardGraph::default();
    validate_image_payload_budgets(root)?;
    collect_identities(root, &mut identities, &mut board_graph)?;
    let mut anchors = Vec::new();
    let mut next_id = 1usize;
    let mut next_anchor = 1usize;
    let mut root_owned = RawNode::Element(root.clone());
    let preserved_root = preserve_node(
        &mut root_owned,
        0,
        SectionKind::Other,
        root_namespace,
        &mut project,
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
    let preservation = PreservationState {
        source: SourceSnapshot {
            format: SourceFormat::Fdx,
            original_bytes: input.to_vec(),
            encoding: SourceEncoding::Utf8,
        },
        sections: vec![PreservedSection {
            name: document_name,
            kind: SectionKind::Other,
            ordinal: 0,
            children: std::mem::take(&mut preserved_children),
        }],
        anchors,
        identities,
        references: Vec::new(),
        board_graph,
    };
    project.preservation = Some(preservation);
    project.validate().map_err(map_model_error)?;
    Ok(FdxDocument { project })
}

/// Serialize the 17 evidenced screenplay roles to UTF-8 Version 6 XML.
///
/// For imported documents, managed paragraphs are reconciled into the retained
/// ordered source tree; unknown and source-owned nodes are re-emitted in place.
/// For a new document, only default app state and the semantic title/body content
/// are accepted. This is a codec operation, not native save/reopen support:
/// ScriptCraft identity, schema, full profile, and editing policies do not have
/// an established durable FDX mapping. Output is structurally re-read through
/// the bounded reader before it is returned. No Final Draft application
/// interoperability is implied. Imported XML is retained semantically, not
/// lexically: the XML declaration, entity spelling, quote style and exact
/// attribute interleaving with namespace declarations are not guaranteed to be
/// byte-identical. UTF-8 is emitted; supported tree order, expanded names,
/// attributes, text, comments, processing instructions and opaque children are
/// checked against the re-read output.
pub fn write_v6(project: &Project) -> Result<Vec<u8>, FdxError> {
    project.validate().map_err(map_model_error)?;
    validate_project_output_budgets(project)?;

    let (nodes, expected_preserved_sections) = if let Some(state) = &project.preservation {
        state.validate().map_err(map_preservation_error)?;
        if state.source.format != SourceFormat::Fdx || state.source.encoding != SourceEncoding::Utf8
        {
            return Err(FdxError::UnsupportedConstruct);
        }
        let original = read_v6(&state.source.original_bytes)?;
        if project.id != original.project.id
            || project.schema_version != original.project.schema_version
            || project.metadata != original.project.metadata
            || project.page_profile != original.project.page_profile
        {
            return Err(FdxError::UnsupportedAppState);
        }
        let original_state = original
            .project
            .preservation
            .as_ref()
            .ok_or(FdxError::UnsupportedConstruct)?;
        if state.identities != original_state.identities
            || state.references != original_state.references
            || state.board_graph != original_state.board_graph
        {
            return Err(FdxError::UnsupportedConstruct);
        }

        let mut sections = state.sections.clone();
        set_preserved_root_version(&mut sections)?;
        reconcile_project_elements(&mut sections, project)?;
        let expected_sections = sections.clone();
        let nodes = sections
            .into_iter()
            .flat_map(|section| section.children)
            .collect::<Vec<_>>();
        (nodes, Some(expected_sections))
    } else {
        if project.schema_version != CURRENT_SCHEMA_VERSION
            || project.metadata != Default::default()
            || project.page_profile != PageProfile::default()
        {
            return Err(FdxError::UnsupportedAppState);
        }
        (make_new_document_nodes(project)?, None)
    };

    let mut output = String::new();
    for node in &nodes {
        write_node(node, &mut output, 0)?;
        if output.len() > MAX_SOURCE_BYTES {
            return Err(FdxError::SourceTooLarge);
        }
    }
    if output.len() > MAX_SOURCE_BYTES {
        return Err(FdxError::SourceTooLarge);
    }
    let bytes = output.into_bytes();
    let written = read_v6(&bytes)?;
    if !same_semantic_elements(&project.title_page, &written.project.title_page)
        || !same_semantic_elements(&project.elements, &written.project.elements)
    {
        return Err(FdxError::UnsupportedConstruct);
    }
    if let Some(expected_sections) = expected_preserved_sections {
        let actual_sections = written
            .project
            .preservation
            .as_ref()
            .map(|state| &state.sections)
            .ok_or(FdxError::UnsupportedConstruct)?;
        if normalize_sections(actual_sections) != normalize_sections(&expected_sections) {
            return Err(FdxError::UnsupportedConstruct);
        }
    }
    Ok(bytes)
}

fn same_semantic_elements(left: &[ScreenplayElement], right: &[ScreenplayElement]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.role == right.role
                && left.text == right.text
                && if left.runs.is_empty() {
                    right.runs.iter().all(|run| run.attributes.is_empty())
                } else {
                    left.runs == right.runs
                }
        })
}

fn set_preserved_root_version(sections: &mut [PreservedSection]) -> Result<(), FdxError> {
    let mut roots = sections
        .iter_mut()
        .flat_map(|section| &mut section.children)
        .filter_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == FDX_ROOT => Some(element),
            _ => None,
        });
    let root = roots.next().ok_or(FdxError::InvalidRoot)?;
    if roots.next().is_some() {
        return Err(FdxError::InvalidRoot);
    }
    let mut versions = root.attributes.iter_mut().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Version"
    });
    let version = versions.next().ok_or(FdxError::MissingVersion)?;
    if versions.next().is_some() {
        return Err(FdxError::DuplicateVersion);
    }
    version.value = FDX_VERSION.to_owned();
    Ok(())
}

fn validate_project_output_budgets(project: &Project) -> Result<(), FdxError> {
    let mut text_bytes = 0usize;
    let mut event_count = 3usize;
    for element in project.title_page.iter().chain(&project.elements) {
        event_count = event_count.saturating_add(1);
        text_bytes = text_bytes.saturating_add(element.text.len());
        let run_count = element.runs.len().max(usize::from(element.runs.is_empty()));
        event_count = event_count.saturating_add(run_count);
        if element.runs.is_empty() && !element.text.is_empty() {
            event_count = event_count.saturating_add(1);
        }
        for run in &element.runs {
            if !run.text.is_empty() {
                event_count = event_count.saturating_add(1);
            }
            if run.attributes.len() > MAX_ATTRIBUTES_PER_ELEMENT {
                return Err(FdxError::ResourceLimit);
            }
            event_count = event_count.saturating_add(run.attributes.len());
            for attribute in &run.attributes {
                if attribute.name.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES
                    || attribute.value.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES
                {
                    return Err(FdxError::ResourceLimit);
                }
                validate_qname(&attribute.name)?;
                validate_xml_chars(&attribute.value)?;
            }
        }
        if text_bytes > MAX_SOURCE_TEXT_BYTES || event_count > MAX_SOURCE_EVENTS {
            return Err(FdxError::ResourceLimit);
        }
    }
    Ok(())
}

fn normalize_sections(sections: &[PreservedSection]) -> Vec<PreservedSection> {
    sections
        .iter()
        .cloned()
        .map(|mut section| {
            section.children = normalize_text_nodes(&section.children);
            section
        })
        .collect()
}

fn normalize_text_nodes(nodes: &[PreservedNode]) -> Vec<PreservedNode> {
    let mut normalized = Vec::<PreservedNode>::new();
    for node in nodes {
        let normalized_node = match node {
            PreservedNode::Element(element) => {
                let mut element = element.clone();
                element.children = normalize_text_nodes(&element.children);
                PreservedNode::Element(element)
            }
            other => other.clone(),
        };
        if let PreservedNode::Text(text) = normalized_node {
            if let Some(PreservedNode::Text(previous)) = normalized.last_mut() {
                previous.push_str(&text);
            } else {
                normalized.push(PreservedNode::Text(text));
            }
        } else {
            normalized.push(normalized_node);
        }
    }
    normalized
}

fn reconcile_project_elements(
    sections: &mut [PreservedSection],
    project: &Project,
) -> Result<(), FdxError> {
    let body = project
        .elements
        .iter()
        .map(|element| (element.id.clone(), element))
        .collect::<BTreeMap<_, _>>();
    let title = project
        .title_page
        .iter()
        .map(|element| (element.id.clone(), element))
        .collect::<BTreeMap<_, _>>();
    let mut body_order = Vec::new();
    let mut title_order = Vec::new();
    let mut seen = BTreeSet::new();
    let mut roots = 0usize;
    let root_namespace = sections
        .iter()
        .flat_map(|section| &section.children)
        .find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == FDX_ROOT => {
                element.name.namespace_uri.clone()
            }
            _ => None,
        });
    for section in sections.iter_mut() {
        for node in &mut section.children {
            reconcile_node(
                node,
                SectionKind::Other,
                0,
                root_namespace.as_deref(),
                &body,
                &title,
                &mut body_order,
                &mut title_order,
                &mut seen,
                &mut roots,
            )?;
        }
    }
    if roots != 1
        || body_order
            != project
                .elements
                .iter()
                .map(|element| element.id.clone())
                .collect::<Vec<_>>()
        || title_order
            != project
                .title_page
                .iter()
                .map(|element| element.id.clone())
                .collect::<Vec<_>>()
    {
        return Err(FdxError::UnsupportedConstruct);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn reconcile_node(
    node: &mut PreservedNode,
    section: SectionKind,
    depth: usize,
    root_namespace: Option<&str>,
    body: &BTreeMap<crate::StableId, &ScreenplayElement>,
    title: &BTreeMap<crate::StableId, &ScreenplayElement>,
    body_order: &mut Vec<crate::StableId>,
    title_order: &mut Vec<crate::StableId>,
    seen: &mut BTreeSet<crate::StableId>,
    roots: &mut usize,
) -> Result<(), FdxError> {
    let PreservedNode::Element(element) = node else {
        return Ok(());
    };
    if depth == 0 && element.name.local_name == FDX_ROOT {
        *roots = roots.checked_add(1).ok_or(FdxError::ResourceLimit)?;
    }
    let is_fdx_element = element.name.namespace_uri.as_deref() == root_namespace;
    let current_section = if depth == 1 && is_fdx_element && element.name.local_name == "Content" {
        SectionKind::Content
    } else if depth == 1 && is_fdx_element && element.name.local_name == "TitlePage" {
        SectionKind::TitlePage
    } else {
        section
    };
    if let SourceOwnership::Managed(id) = &element.ownership {
        if !seen.insert(id.clone()) {
            return Err(FdxError::UnsupportedConstruct);
        }
        let semantic = match current_section {
            SectionKind::Content => {
                body_order.push(id.clone());
                body.get(id)
            }
            SectionKind::TitlePage => {
                title_order.push(id.clone());
                title.get(id)
            }
            SectionKind::Other => return Err(FdxError::UnsupportedConstruct),
        }
        .copied()
        .ok_or(FdxError::UnsupportedConstruct)?;
        reconcile_managed_paragraph(element, semantic)?;
        return Ok(());
    }
    for child in &mut element.children {
        reconcile_node(
            child,
            current_section,
            depth.saturating_add(1),
            root_namespace,
            body,
            title,
            body_order,
            title_order,
            seen,
            roots,
        )?;
    }
    Ok(())
}

fn reconcile_managed_paragraph(
    paragraph: &mut PreservedElement,
    semantic: &ScreenplayElement,
) -> Result<(), FdxError> {
    let role_type = role_to_fdx(semantic.role);
    if paragraph.source_type.as_deref() != Some(role_type) {
        return Err(FdxError::UnsupportedConstruct);
    }
    let type_attributes = paragraph
        .attributes
        .iter()
        .filter(|attribute| {
            attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Type"
        })
        .collect::<Vec<_>>();
    if type_attributes.len() != 1 || type_attributes[0].value != role_type {
        return Err(FdxError::UnsupportedConstruct);
    }

    let mut text_elements = paragraph
        .children
        .iter_mut()
        .filter_map(|node| match node {
            PreservedNode::Element(element)
                if element.name.local_name == "Text"
                    && element.name.namespace_uri == paragraph.name.namespace_uri =>
            {
                Some(element)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if text_elements.len() != semantic.runs.len() {
        if text_elements.is_empty() && semantic.runs.is_empty() && semantic.text.is_empty() {
            return Ok(());
        }
        return Err(FdxError::UnsupportedConstruct);
    }
    for (text_element, run) in text_elements.iter_mut().zip(&semantic.runs) {
        let source_attributes = text_element
            .attributes
            .iter()
            .map(|attribute| RunAttribute {
                name: attribute
                    .name
                    .qualified_name
                    .clone()
                    .unwrap_or_else(|| attribute.name.local_name.clone()),
                value: attribute.value.clone(),
            })
            .collect::<Vec<_>>();
        if source_attributes != run.attributes
            || text_element
                .children
                .iter()
                .any(|child| !matches!(child, PreservedNode::Text(_) | PreservedNode::CData(_)))
        {
            return Err(FdxError::UnsupportedConstruct);
        }
        if let Some(first) = text_element.children.first_mut() {
            match first {
                PreservedNode::Text(value) | PreservedNode::CData(value) => {
                    *value = run.text.clone();
                }
                _ => return Err(FdxError::UnsupportedConstruct),
            }
            for child in text_element.children.iter_mut().skip(1) {
                match child {
                    PreservedNode::Text(value) | PreservedNode::CData(value) => value.clear(),
                    _ => return Err(FdxError::UnsupportedConstruct),
                }
            }
        } else if !run.text.is_empty() {
            text_element
                .children
                .push(PreservedNode::Text(run.text.clone()));
        }
    }
    Ok(())
}

fn make_new_document_nodes(project: &Project) -> Result<Vec<PreservedNode>, FdxError> {
    let mut root = preserved_element(
        "FinalDraft",
        vec![("DocumentType", "Script"), ("Version", FDX_VERSION)],
    );
    let mut content = preserved_element("Content", Vec::new());
    let mut next_id = 1usize;
    for element in &project.elements {
        content.children.push(new_paragraph(element, &mut next_id)?);
    }
    root.children.push(PreservedNode::Element(content));
    if !project.title_page.is_empty() {
        let mut title_page = preserved_element("TitlePage", Vec::new());
        for element in &project.title_page {
            title_page
                .children
                .push(new_paragraph(element, &mut next_id)?);
        }
        root.children.push(PreservedNode::Element(title_page));
    }
    Ok(vec![PreservedNode::Element(root)])
}

fn new_paragraph(
    semantic: &ScreenplayElement,
    next_id: &mut usize,
) -> Result<PreservedNode, FdxError> {
    let mut paragraph = preserved_element(
        "Paragraph",
        vec![("Type", role_to_fdx(semantic.role)), ("id", "")],
    );
    paragraph.attributes[1].value = format!("scriptcraft-generated-{}", *next_id);
    *next_id = next_id.checked_add(1).ok_or(FdxError::ResourceLimit)?;
    let mut runs = semantic.runs.clone();
    if runs.is_empty() {
        runs.push(TextRun {
            text: semantic.text.clone(),
            attributes: Vec::new(),
        });
    }
    for run in runs {
        validate_xml_chars(&run.text)?;
        let mut text = preserved_element("Text", Vec::new());
        for attribute in run.attributes {
            validate_qname(&attribute.name)?;
            if attribute.name.contains(':') {
                return Err(FdxError::UnsupportedConstruct);
            }
            validate_xml_chars(&attribute.value)?;
            text.attributes.push(PreservedAttribute {
                name: SourceName {
                    namespace_uri: None,
                    local_name: attribute.name.clone(),
                    qualified_name: Some(attribute.name),
                },
                value: attribute.value,
            });
        }
        if !run.text.is_empty() {
            text.children.push(PreservedNode::Text(run.text));
        }
        paragraph.children.push(PreservedNode::Element(text));
    }
    Ok(PreservedNode::Element(paragraph))
}

fn preserved_element(name: &str, attributes: Vec<(&str, &str)>) -> PreservedElement {
    PreservedElement {
        name: SourceName {
            namespace_uri: None,
            local_name: name.to_owned(),
            qualified_name: Some(name.to_owned()),
        },
        attributes: attributes
            .into_iter()
            .map(|(name, value)| PreservedAttribute {
                name: SourceName {
                    namespace_uri: None,
                    local_name: name.to_owned(),
                    qualified_name: Some(name.to_owned()),
                },
                value: value.to_owned(),
            })
            .collect(),
        namespace_bindings: Vec::new(),
        source_type: None,
        ownership: SourceOwnership::Opaque,
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: None,
        children: Vec::new(),
    }
}

fn role_to_fdx(role: Role) -> &'static str {
    match role {
        Role::SceneHeading => "Scene Heading",
        Role::Action => "Action",
        Role::Character => "Character",
        Role::Dialogue => "Dialogue",
        Role::Parenthetical => "Parenthetical",
        Role::Transition => "Transition",
        Role::Note => "Note",
        Role::NewAct => "New Act",
        Role::EndOfAct => "End of Act",
        Role::CastList => "Cast List",
        Role::General => "General",
        Role::Shot => "Shot",
        Role::Sequence => "Sequence",
        Role::Outline1 => "Outline 1",
        Role::Outline2 => "Outline 2",
        Role::Outline3 => "Outline 3",
        Role::Summary => "Summary",
    }
}

fn write_node(node: &PreservedNode, output: &mut String, depth: usize) -> Result<(), FdxError> {
    if depth > MAX_SOURCE_DEPTH || output.len() > MAX_SOURCE_BYTES {
        return Err(FdxError::ResourceLimit);
    }
    match node {
        PreservedNode::Element(element) => write_element(element, output, depth),
        PreservedNode::Text(text) => escape_text(text, output),
        PreservedNode::CData(text) => {
            validate_xml_chars(text)?;
            if text.contains("]]>") {
                return Err(FdxError::InvalidValue);
            }
            append_output(output, "<![CDATA[")?;
            append_output(output, text)?;
            append_output(output, "]]>")?;
            Ok(())
        }
        PreservedNode::Comment(comment) => {
            validate_xml_chars(comment)?;
            if comment.contains("--") || comment.ends_with('-') {
                return Err(FdxError::InvalidValue);
            }
            append_output(output, "<!--")?;
            append_output(output, comment)?;
            append_output(output, "-->")?;
            Ok(())
        }
        PreservedNode::ProcessingInstruction { target, data } => {
            validate_qname(target)?;
            validate_xml_chars(data)?;
            if data.contains("?>") {
                return Err(FdxError::InvalidValue);
            }
            append_output(output, "<?")?;
            append_output(output, target)?;
            if !data.is_empty() {
                append_output(output, " ")?;
                append_output(output, data)?;
            }
            append_output(output, "?>")?;
            Ok(())
        }
    }
}

fn write_element(
    element: &PreservedElement,
    output: &mut String,
    depth: usize,
) -> Result<(), FdxError> {
    if output.len() > MAX_SOURCE_BYTES {
        return Err(FdxError::SourceTooLarge);
    }
    let name = element
        .name
        .qualified_name
        .as_deref()
        .unwrap_or(&element.name.local_name);
    validate_qname(name)?;
    if element.name.namespace_uri.is_some() && element.name.qualified_name.is_none() {
        return Err(FdxError::UnsupportedConstruct);
    }
    let mut attribute_names = BTreeSet::new();
    let mut expanded_attribute_names = BTreeSet::new();
    let mut bindings = BTreeSet::new();
    append_output(output, "<")?;
    append_output(output, name)?;
    for binding in &element.namespace_bindings {
        if !bindings.insert(binding.prefix.as_str()) {
            return Err(FdxError::InvalidValue);
        }
        let name = if binding.prefix.is_empty() {
            "xmlns".to_owned()
        } else {
            format!("xmlns:{}", binding.prefix)
        };
        validate_qname(&name)?;
        append_output(output, " ")?;
        append_output(output, &name)?;
        append_output(output, "=\"")?;
        escape_attribute(&binding.namespace_uri, output)?;
        append_output(output, "\"")?;
    }
    for attribute in &element.attributes {
        let name = attribute
            .name
            .qualified_name
            .as_deref()
            .unwrap_or(&attribute.name.local_name);
        validate_qname(name)?;
        if name.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES
            || attribute.value.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES
            || !attribute_names.insert(name)
            || !expanded_attribute_names.insert((
                attribute.name.namespace_uri.clone(),
                attribute.name.local_name.clone(),
            ))
        {
            return Err(FdxError::InvalidValue);
        }
        append_output(output, " ")?;
        append_output(output, name)?;
        append_output(output, "=\"")?;
        escape_attribute(&attribute.value, output)?;
        append_output(output, "\"")?;
    }
    if element.children.is_empty() {
        append_output(output, "/>")?;
        return Ok(());
    }
    append_output(output, ">")?;
    for child in &element.children {
        write_node(child, output, depth.saturating_add(1))?;
    }
    append_output(output, "</")?;
    append_output(output, name)?;
    append_output(output, ">")?;
    Ok(())
}

fn escape_text(value: &str, output: &mut String) -> Result<(), FdxError> {
    validate_xml_chars(value)?;
    for character in value.chars() {
        match character {
            '&' => append_output(output, "&amp;")?,
            '<' => append_output(output, "&lt;")?,
            '>' => append_output(output, "&gt;")?,
            '\r' => append_output(output, "&#xD;")?,
            _ => append_character(output, character)?,
        }
    }
    Ok(())
}

fn escape_attribute(value: &str, output: &mut String) -> Result<(), FdxError> {
    validate_xml_chars(value)?;
    for character in value.chars() {
        match character {
            '&' => append_output(output, "&amp;")?,
            '<' => append_output(output, "&lt;")?,
            '"' => append_output(output, "&quot;")?,
            '\r' => append_output(output, "&#xD;")?,
            '\n' => append_output(output, "&#xA;")?,
            '\t' => append_output(output, "&#x9;")?,
            _ => append_character(output, character)?,
        }
    }
    Ok(())
}

fn append_output(output: &mut String, value: &str) -> Result<(), FdxError> {
    if output.len().saturating_add(value.len()) > MAX_SOURCE_BYTES {
        return Err(FdxError::SourceTooLarge);
    }
    output.push_str(value);
    Ok(())
}

fn append_character(output: &mut String, value: char) -> Result<(), FdxError> {
    if output.len().saturating_add(value.len_utf8()) > MAX_SOURCE_BYTES {
        return Err(FdxError::SourceTooLarge);
    }
    output.push(value);
    Ok(())
}

fn validate_qname(name: &str) -> Result<(), FdxError> {
    let mut parts = name.split(':');
    let first = parts.next().ok_or(FdxError::InvalidValue)?;
    let second = parts.next();
    if first.is_empty() || parts.next().is_some() || second.is_some_and(str::is_empty) {
        return Err(FdxError::InvalidValue);
    }
    for part in std::iter::once(first).chain(second) {
        let mut characters = part.chars();
        let Some(initial) = characters.next() else {
            return Err(FdxError::InvalidValue);
        };
        if !is_xml_name_start(initial) || !characters.all(is_xml_name_char) {
            return Err(FdxError::InvalidValue);
        }
    }
    Ok(())
}

fn is_xml_name_start(character: char) -> bool {
    matches!(character, 'A'..='Z' | '_' | 'a'..='z')
        || matches!(character as u32,
            0xC0..=0xD6 | 0xD8..=0xF6 | 0xF8..=0x2FF | 0x370..=0x37D
            | 0x37F..=0x1FFF | 0x200C..=0x200D | 0x2070..=0x218F
            | 0x2C00..=0x2FEF | 0x3001..=0xD7FF | 0xF900..=0xFDCF
            | 0xFDF0..=0xFFFD | 0x10000..=0xEFFFF)
}

fn is_xml_name_char(character: char) -> bool {
    is_xml_name_start(character)
        || matches!(character, '-' | '.' | '0'..='9' | '\u{B7}' | '\u{0300}'..='\u{036F}' | '\u{203F}'..='\u{2040}')
}

#[derive(Debug, Clone)]
struct RawAttribute {
    name: SourceName,
    value: String,
}

#[derive(Debug, Clone)]
struct RawElement {
    name: SourceName,
    attributes: Vec<RawAttribute>,
    namespace_bindings: Vec<NamespaceBinding>,
    children: Vec<RawNode>,
}

#[derive(Debug, Clone)]
enum RawNode {
    Element(RawElement),
    Text(String),
    CData(String),
    Comment(String),
    ProcessingInstruction { target: String, data: String },
}

fn parse_document(input: &[u8]) -> Result<Vec<RawNode>, FdxError> {
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
                validate_xml_chars(&data)?;
                append_node(
                    &mut stack,
                    &mut document_nodes,
                    RawNode::ProcessingInstruction {
                        target,
                        data: data.into_owned(),
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

fn validate_xml_chars(value: &str) -> Result<(), FdxError> {
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

fn attribute<'a>(element: &'a RawElement, name: &str) -> Option<&'a str> {
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

fn version_status(root: &RawElement) -> Result<FdxVersionStatus, FdxError> {
    let mut declarations = root.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Version"
    });
    let Some(declaration) = declarations.next() else {
        return Ok(FdxVersionStatus::Missing);
    };
    if declarations.next().is_some() {
        return Ok(FdxVersionStatus::Duplicate);
    }
    let declared = declaration.value.as_str();
    if declared.is_empty() || !declared.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok(FdxVersionStatus::Malformed);
    }
    let normalized = declared.trim_start_matches('0');
    let normalized = if normalized.is_empty() {
        "0"
    } else {
        normalized
    };
    if normalized == FDX_VERSION {
        return Ok(FdxVersionStatus::Version6);
    }
    let direction = if normalized.len() < FDX_VERSION.len()
        || (normalized.len() == FDX_VERSION.len() && normalized < FDX_VERSION)
    {
        FdxVersionDirection::Older
    } else {
        FdxVersionDirection::Newer
    };
    Ok(FdxVersionStatus::ExplicitNon6 {
        declared: declared.to_owned(),
        direction,
    })
}

fn prepare_non_v6_source(
    input: &[u8],
    direction: FdxVersionDirection,
) -> Result<Vec<u8>, FdxError> {
    let mut parsed = parse_document(input)?;
    let root_index = parsed
        .iter()
        .position(|node| matches!(node, RawNode::Element(_)))
        .ok_or(FdxError::InvalidRoot)?;
    let root = match &mut parsed[root_index] {
        RawNode::Element(root) => root,
        _ => return Err(FdxError::InvalidRoot),
    };
    if root.name.local_name != FDX_ROOT || attribute(root, "DocumentType") != Some("Script") {
        return Err(FdxError::InvalidRoot);
    }
    let FdxVersionStatus::ExplicitNon6 {
        direction: actual_direction,
        ..
    } = version_status(root)?
    else {
        return Err(FdxError::UnsupportedVersion);
    };
    if actual_direction != direction {
        return Err(FdxError::UnsupportedVersion);
    }
    let version_attribute = root.attributes.iter_mut().find(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Version"
    });
    if let Some(attribute) = version_attribute {
        attribute.value = FDX_VERSION.to_owned();
    } else {
        return Err(FdxError::MissingVersion);
    }

    if direction == FdxVersionDirection::Older {
        let root_namespace = root.name.namespace_uri.clone();
        root.children.retain(|node| match node {
            RawNode::Element(element) => {
                is_fdx_element(element, "Content", root_namespace.as_deref())
                    || is_fdx_element(element, "TitlePage", root_namespace.as_deref())
            }
            _ => true,
        });
        for child in &mut root.children {
            let RawNode::Element(section) = child else {
                continue;
            };
            if !is_fdx_element(section, "Content", root_namespace.as_deref())
                && !is_fdx_element(section, "TitlePage", root_namespace.as_deref())
            {
                continue;
            }
            let is_content = is_fdx_element(section, "Content", root_namespace.as_deref());
            for item in &mut section.children {
                let RawNode::Element(paragraph) = item else {
                    continue;
                };
                if paragraph.name.local_name != "Paragraph" {
                    if is_content {
                        return Err(FdxError::UnsupportedConstruct);
                    }
                    continue;
                }
                if paragraph.name.namespace_uri != root_namespace {
                    return Err(FdxError::UnsupportedConstruct);
                }
                let paragraph_type = attribute(paragraph, "Type");
                if paragraph_type.is_some_and(|value| role_from_fdx(value).is_some()) {
                    continue;
                }
                if let Some(type_attribute) = paragraph.attributes.iter_mut().find(|attribute| {
                    attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Type"
                }) {
                    type_attribute.value = "General".to_owned();
                } else {
                    paragraph.attributes.push(RawAttribute {
                        name: SourceName {
                            namespace_uri: None,
                            local_name: "Type".to_owned(),
                            qualified_name: Some("Type".to_owned()),
                        },
                        value: "General".to_owned(),
                    });
                }
            }
        }
    }

    let mut output = String::new();
    for node in parsed {
        let preserved = preserve_misc_node(node)?;
        write_node(&preserved, &mut output, 0)?;
        if output.len() > MAX_SOURCE_BYTES {
            return Err(FdxError::SourceTooLarge);
        }
    }
    Ok(output.into_bytes())
}

fn is_fdx_element(element: &RawElement, name: &str, root_ns: Option<&str>) -> bool {
    element.name.local_name == name && element.name.namespace_uri.as_deref() == root_ns
}

fn preserve_misc_node(node: RawNode) -> Result<PreservedNode, FdxError> {
    match node {
        RawNode::Element(element) => preserve_opaque_element(element),
        RawNode::Text(text) => Ok(PreservedNode::Text(text)),
        RawNode::CData(text) => Ok(PreservedNode::CData(text)),
        RawNode::Comment(text) => Ok(PreservedNode::Comment(text)),
        RawNode::ProcessingInstruction { target, data } => {
            Ok(PreservedNode::ProcessingInstruction { target, data })
        }
    }
}

fn preserve_opaque_element(element: RawElement) -> Result<PreservedNode, FdxError> {
    let source_type = attribute(&element, "Type").map(str::to_owned);
    let children = element
        .children
        .into_iter()
        .map(preserve_misc_node)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PreservedNode::Element(PreservedElement {
        name: element.name,
        attributes: element
            .attributes
            .into_iter()
            .map(|attribute| PreservedAttribute {
                name: attribute.name,
                value: attribute.value,
            })
            .collect(),
        namespace_bindings: element.namespace_bindings,
        source_type,
        ownership: SourceOwnership::Opaque,
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: None,
        children,
    }))
}

#[allow(clippy::too_many_arguments)]
fn preserve_node(
    node: &mut RawNode,
    depth: usize,
    section: SectionKind,
    root_ns: Option<&str>,
    project: &mut Project,
    anchors: &mut Vec<SourceAnchor>,
    next_id: &mut usize,
    next_anchor: &mut usize,
) -> Result<PreservedNode, FdxError> {
    let RawNode::Element(element) = node else {
        return preserve_misc_node(node.clone());
    };
    let current_section = if depth == 1 && is_fdx_element(element, "Content", root_ns) {
        SectionKind::Content
    } else if depth == 1 && is_fdx_element(element, "TitlePage", root_ns) {
        SectionKind::TitlePage
    } else {
        section
    };
    let managed_role = if depth == 2
        && matches!(
            current_section,
            SectionKind::Content | SectionKind::TitlePage
        )
        && is_fdx_element(element, "Paragraph", root_ns)
    {
        attribute(element, "Type").and_then(role_from_fdx)
    } else {
        None
    };

    let managed = if let Some(role) = managed_role {
        Some(make_semantic_element(
            element,
            role,
            current_section,
            next_id,
        )?)
    } else {
        None
    };
    let anchor_id = if managed.is_some() {
        let value = format!("fdx-anchor-{}", *next_anchor);
        *next_anchor = next_anchor.checked_add(1).ok_or(FdxError::ResourceLimit)?;
        Some(SourceAnchorId::new(value).map_err(|_| FdxError::InvalidValue)?)
    } else {
        None
    };
    let ownership = managed
        .as_ref()
        .map(|semantic| SourceOwnership::Managed(semantic.id.clone()))
        .unwrap_or(SourceOwnership::Opaque);
    let mut children = Vec::with_capacity(element.children.len());
    for child in &mut element.children {
        children.push(preserve_node(
            child,
            depth.saturating_add(1),
            current_section,
            root_ns,
            project,
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
        children,
    };
    if let (Some(semantic), Some(anchor_id)) = (managed, anchor_id) {
        anchors.push(SourceAnchor {
            id: anchor_id,
            target: crate::preservation::AnchorTarget::ProjectElement(semantic.id.clone()),
            relation: crate::preservation::AnchorRelation::Represents,
        });
        match current_section {
            SectionKind::Content => project.elements.push(semantic),
            SectionKind::TitlePage => project.title_page.push(semantic),
            SectionKind::Other => return Err(FdxError::UnsupportedConstruct),
        }
    }
    Ok(PreservedNode::Element(preserved))
}

fn make_semantic_element(
    element: &RawElement,
    role: Role,
    section: SectionKind,
    next_id: &mut usize,
) -> Result<ScreenplayElement, FdxError> {
    let id = format!("fdx-element-{}", *next_id);
    *next_id = next_id.checked_add(1).ok_or(FdxError::ResourceLimit)?;
    let source_provenance = SourceProvenance {
        format: SourceFormat::Fdx,
        section: if section == SectionKind::TitlePage {
            SourceSection::TitlePage
        } else {
            SourceSection::Content
        },
        ordinal: u32::try_from(*next_id - 2).map_err(|_| FdxError::ResourceLimit)?,
        foreign_id: identity_attribute(element).map(str::to_owned),
        foreign_role: attribute(element, "Type").map(str::to_owned),
    };
    let mut runs = Vec::new();
    for child in &element.children {
        let RawNode::Element(text_element) = child else {
            continue;
        };
        if text_element.name.local_name != "Text"
            || text_element.name.namespace_uri != element.name.namespace_uri
        {
            continue;
        }
        let mut text = String::new();
        for inline in &text_element.children {
            match inline {
                RawNode::Text(value) | RawNode::CData(value) => text.push_str(value),
                RawNode::Element(_) => return Err(FdxError::UnsupportedConstruct),
                RawNode::Comment(_) | RawNode::ProcessingInstruction { .. } => {
                    return Err(FdxError::UnsupportedConstruct)
                }
            }
        }
        let attributes = text_element
            .attributes
            .iter()
            .map(|attribute| RunAttribute {
                name: attribute
                    .name
                    .qualified_name
                    .clone()
                    .unwrap_or_else(|| attribute.name.local_name.clone()),
                value: attribute.value.clone(),
            })
            .collect();
        runs.push(TextRun { text, attributes });
    }
    if runs.is_empty() {
        // A paragraph with no Text child is still faithfully represented as an
        // empty semantic paragraph; all source structure remains in preservation.
    }
    ScreenplayElement::with_runs(id, role, runs, Some(source_provenance)).map_err(map_model_error)
}

/// Apply only explicit source settings that have direct counterparts in the
/// model. Unknown settings remain source-owned in the preservation tree.
fn apply_profile_settings(
    root: &RawElement,
    root_ns: Option<&str>,
    profile: &mut PageProfile,
) -> Result<(), FdxError> {
    let mut seen_roles = BTreeSet::new();
    for child in &root.children {
        let RawNode::Element(setting) = child else {
            continue;
        };
        if !is_fdx_element(setting, "ElementSettings", root_ns) {
            continue;
        }
        let Some(source_role) = attribute(setting, "Type") else {
            continue;
        };
        let Some(role) = role_from_fdx(source_role) else {
            continue;
        };
        if !seen_roles.insert(role) {
            return Err(FdxError::DuplicateSection);
        }
        for child_setting in &setting.children {
            let RawNode::Element(child_setting) = child_setting else {
                continue;
            };
            if child_setting.name.namespace_uri.as_deref() != root_ns {
                continue;
            }
            if child_setting.name.local_name == "FontSpec" {
                if let Some(style) = attribute(child_setting, "Style") {
                    let hidden = style.split('+').any(|token| token == "HiddenText");
                    profile.role_print_visibility[role_index(role)] = if hidden {
                        PrintVisibility::Hidden
                    } else {
                        PrintVisibility::Visible
                    };
                }
            } else if child_setting.name.local_name == "ParagraphSpec" {
                if let Some(starts_new_page) = attribute(child_setting, "StartsNewPage") {
                    profile.role_starts_new_page[role_index(role)] = match starts_new_page {
                        "Yes" => true,
                        "No" => false,
                        _ => return Err(FdxError::InvalidValue),
                    };
                }
            }
        }
    }
    Ok(())
}

fn role_index(role: Role) -> usize {
    Role::ALL
        .iter()
        .position(|candidate| *candidate == role)
        .unwrap_or(0)
}

fn role_from_fdx(value: &str) -> Option<Role> {
    Some(match value {
        "Scene Heading" => Role::SceneHeading,
        "Action" => Role::Action,
        "Character" => Role::Character,
        "Dialogue" => Role::Dialogue,
        "Parenthetical" => Role::Parenthetical,
        "Transition" => Role::Transition,
        "Note" => Role::Note,
        "New Act" => Role::NewAct,
        "End of Act" => Role::EndOfAct,
        "Cast List" => Role::CastList,
        "General" => Role::General,
        "Shot" => Role::Shot,
        "Sequence" => Role::Sequence,
        "Outline 1" => Role::Outline1,
        "Outline 2" => Role::Outline2,
        "Outline 3" => Role::Outline3,
        "Summary" => Role::Summary,
        _ => return None,
    })
}

fn collect_identities(
    root: &RawElement,
    identities: &mut Vec<ForeignIdentity>,
    board_graph: &mut BoardGraph,
) -> Result<(), FdxError> {
    let mut seen = BTreeSet::new();
    let mut definitions = Vec::new();
    walk_elements(root, &mut |element, nearest_board| {
        if identity_attribute_count(element) > 1 {
            return Err(FdxError::AmbiguousIdentity);
        }
        if matches!(
            element.name.local_name.as_str(),
            "ListItem" | "DisplayBoard" | "Image"
        ) && identity_attribute(element).is_none()
        {
            return Err(FdxError::AmbiguousIdentity);
        }
        if nearest_board.is_some()
            && matches!(element.name.local_name.as_str(), "Item" | "Picture")
            && identity_attribute(element).is_none()
        {
            return Err(FdxError::DanglingReference);
        }
        let Some(value) = identity_attribute(element) else {
            return Ok(());
        };
        if nearest_board.is_some() && matches!(element.name.local_name.as_str(), "Item" | "Picture")
        {
            return Ok(());
        }
        let domain = identity_domain(&element.name.local_name);
        let identity = ForeignIdentity::new(domain.clone(), value.to_owned())
            .map_err(map_preservation_error)?;
        if !seen.insert(identity.clone()) {
            return Err(FdxError::AmbiguousIdentity);
        }
        definitions.push((
            element.name.local_name.clone(),
            identity,
            element.attributes.clone(),
        ));
        Ok(())
    })?;

    let mut source_order = 0u32;
    for (name, identity, attributes) in &definitions {
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
                // The encoded payload remains in the ordered source tree and
                // exact original snapshot; it is intentionally not decoded here.
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
        .map(|(_, identity, _)| identity.clone())
        .collect::<BTreeSet<_>>();
    let mut references = Vec::new();
    walk_elements(root, &mut |element, nearest_board| {
        let board = if element.name.local_name == "DisplayBoard" {
            identity_attribute(element)
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
                if let Some(target_value) = identity_attribute(element) {
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
            Ok(())
        } else {
            Ok(())
        }
    })?;
    board_graph.references = references;
    Ok(())
}

fn walk_elements<F>(element: &RawElement, visitor: &mut F) -> Result<(), FdxError>
where
    F: FnMut(&RawElement, Option<&ForeignIdentity>) -> Result<(), FdxError>,
{
    fn visit<F>(
        element: &RawElement,
        nearest_board: Option<&ForeignIdentity>,
        visitor: &mut F,
    ) -> Result<(), FdxError>
    where
        F: FnMut(&RawElement, Option<&ForeignIdentity>) -> Result<(), FdxError>,
    {
        let local_board = if element.name.local_name == "DisplayBoard" {
            identity_attribute(element)
                .map(|value| ForeignIdentity::new(ForeignIdDomain::Board, value.to_owned()))
                .transpose()
                .map_err(map_preservation_error)?
        } else {
            None
        };
        let active_board = local_board.as_ref().or(nearest_board);
        visitor(element, active_board)?;
        for child in &element.children {
            if let RawNode::Element(child) = child {
                visit(child, active_board, visitor)?;
            }
        }
        Ok(())
    }
    visit(element, None, visitor)
}

fn identity_domain(name: &str) -> ForeignIdDomain {
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

fn identity_attribute(element: &RawElement) -> Option<&str> {
    attribute_ci(element, "id")
}

fn identity_attribute_count(element: &RawElement) -> usize {
    element
        .attributes
        .iter()
        .filter(|attribute| {
            attribute.name.namespace_uri.is_none()
                && attribute.name.local_name.eq_ignore_ascii_case("id")
        })
        .count()
}

fn attribute_ci<'a>(element: &'a RawElement, name: &str) -> Option<&'a str> {
    let mut found = None;
    for attribute in &element.attributes {
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

fn raw_attribute_ci<'a>(attributes: &'a [RawAttribute], name: &str) -> Option<&'a str> {
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

fn validate_image_payload_budgets(root: &RawElement) -> Result<(), FdxError> {
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
    fn inspect(element: &RawElement, total_image_text: &mut usize) -> Result<(), FdxError> {
        if element.name.local_name == "Image" {
            let encoded = text_size(element)?;
            let per_image_encoded_limit = MAX_IMAGE_BYTES
                .checked_mul(4)
                .and_then(|value| value.checked_add(2))
                .map(|value| value / 3)
                .ok_or(FdxError::ResourceLimit)?;
            *total_image_text = total_image_text
                .checked_add(encoded)
                .ok_or(FdxError::ResourceLimit)?;
            let total_encoded_limit = MAX_TOTAL_IMAGE_BYTES
                .checked_mul(4)
                .and_then(|value| value.checked_add(2))
                .map(|value| value / 3)
                .ok_or(FdxError::ResourceLimit)?;
            if encoded > per_image_encoded_limit || *total_image_text > total_encoded_limit {
                return Err(FdxError::ResourceLimit);
            }
        }
        for child in &element.children {
            if let RawNode::Element(child) = child {
                inspect(child, total_image_text)?;
            }
        }
        Ok(())
    }
    let mut total_image_text = 0usize;
    inspect(root, &mut total_image_text)
}

fn map_model_error(error: ModelError) -> FdxError {
    match error {
        ModelError::EmptyId | ModelError::InvalidId | ModelError::DuplicateId => {
            FdxError::Model(ModelErrorKind::InvalidIdentifier)
        }
        ModelError::TextTooLong | ModelError::StringTooLong => {
            FdxError::Model(ModelErrorKind::InvalidText)
        }
        _ => FdxError::Model(ModelErrorKind::InvalidModel),
    }
}

fn map_preservation_error(error: PreservationError) -> FdxError {
    use PreservationError as Error;
    match error {
        Error::SourceTooLarge
        | Error::TooManyEvents
        | Error::ExcessiveDepth
        | Error::TooManyAttributes
        | Error::FieldTooLarge
        | Error::TextTooLarge
        | Error::ImageTooLarge
        | Error::TotalImageBytesTooLarge => {
            FdxError::Preservation(PreservationErrorKind::ResourceLimit)
        }
        Error::DuplicateIdentity => FdxError::AmbiguousIdentity,
        Error::DanglingReference | Error::MissingReferenceSource | Error::ImageLengthMismatch => {
            FdxError::DanglingReference
        }
        Error::InvalidAnchor
        | Error::DuplicateAnchor
        | Error::UnboundAnchor
        | Error::AnchorTargetMissing
        | Error::ManagedAnchorMissing
        | Error::ManagedAnchorAmbiguous => {
            FdxError::Preservation(PreservationErrorKind::InvalidAnchor)
        }
        _ => FdxError::Preservation(PreservationErrorKind::InvalidState),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SYNTHETIC: &[u8] = include_bytes!(
        "../../../.forge/initiatives/2026-10-31-document-model-refactor/evidence/wo001-preservation-shapes-synthetic.xml"
    );

    #[test]
    fn reads_directed_roles_runs_and_source_order() {
        let result = read_v6(SYNTHETIC);
        assert!(result.is_ok());
        let project = match result {
            Ok(result) => result.project,
            Err(_) => return,
        };
        assert_eq!(project.elements.len(), 17);
        assert_eq!(project.elements[0].role, Role::SceneHeading);
        assert_eq!(project.elements[3].text, "synthetic dialogue");
        assert_eq!(project.elements[3].runs.len(), 2);
        assert_eq!(project.elements[3].runs[0].text, "synthetic ");
        assert_eq!(project.elements[3].runs[1].text, "dialogue");
        assert_eq!(project.elements[3].runs[0].attributes[0].value, "Bold");
        assert_eq!(project.elements[16].role, Role::Summary);
    }

    #[test]
    fn retains_namespace_unknown_type_title_and_original_bytes() {
        let mut input = SYNTHETIC.to_vec();
        let insertion = b"  <TitlePage><Paragraph><Text>untouched</Text></Paragraph></TitlePage>\n";
        let marker = b"  <Content>";
        let index = input
            .windows(marker.len())
            .position(|window| window == marker)
            .unwrap_or(0);
        input.splice(index..index, insertion.iter().copied());
        let result = read_v6(&input);
        assert!(result.is_ok());
        if let Ok(result) = result {
            assert_eq!(
                result
                    .project
                    .preservation
                    .as_ref()
                    .map(|state| state.source.original_bytes.as_slice()),
                Some(input.as_slice())
            );
            assert_eq!(result.project.title_page.len(), 0);
            let state = result.project.preservation.as_ref();
            assert!(state.is_some_and(|state| state.board_graph.entities.len() == 3));
        }
    }

    #[test]
    fn rejects_dtd_entities_and_non_v6_without_exposing_input() {
        let doctype = b"<!DOCTYPE FinalDraft [<!ENTITY secret 'hidden'>]><FinalDraft DocumentType='Script' Version='6'><Content/></FinalDraft>";
        assert_eq!(read_v6(doctype), Err(FdxError::ForbiddenDoctype));
        let version = b"<FinalDraft DocumentType='Script' Version='7'><Content/></FinalDraft>";
        assert_eq!(read_v6(version), Err(FdxError::UnsupportedVersion));
        let entity = b"<FinalDraft DocumentType='Script' Version='6'><Content>&secret;</Content></FinalDraft>";
        assert_eq!(read_v6(entity), Err(FdxError::UnsupportedEntity));
    }

    #[test]
    fn preflight_distinguishes_versions_and_invalid_declarations() {
        let root = |version: &str| {
            format!("<FinalDraft DocumentType='Script' Version='{version}'><Content/></FinalDraft>")
        };
        assert_eq!(
            preflight_version(root("6").as_bytes()),
            Ok(FdxVersionStatus::Version6)
        );
        assert_eq!(
            preflight_version(root("0006").as_bytes()),
            Ok(FdxVersionStatus::Version6)
        );
        let leading_zero_version = root("0006");
        let leading_zero_document_result = read_v6(leading_zero_version.as_bytes());
        assert!(leading_zero_document_result.is_ok());
        let Ok(leading_zero_document) = leading_zero_document_result else {
            return;
        };
        let canonicalized_result = write_v6(&leading_zero_document.project);
        assert!(canonicalized_result.is_ok());
        let Ok(canonicalized) = canonicalized_result else {
            return;
        };
        assert!(
            std::str::from_utf8(&canonicalized).is_ok_and(|text| text.contains("Version=\"6\""))
        );
        assert_eq!(
            preflight_version(root("5").as_bytes()),
            Ok(FdxVersionStatus::ExplicitNon6 {
                declared: "5".to_owned(),
                direction: FdxVersionDirection::Older,
            })
        );
        assert_eq!(
            preflight_version(root("7").as_bytes()),
            Ok(FdxVersionStatus::ExplicitNon6 {
                declared: "7".to_owned(),
                direction: FdxVersionDirection::Newer,
            })
        );
        assert_eq!(
            preflight_version(b"<FinalDraft DocumentType='Script'><Content/></FinalDraft>"),
            Ok(FdxVersionStatus::Missing)
        );
        assert_eq!(
            preflight_version(root("unknown").as_bytes()),
            Ok(FdxVersionStatus::Malformed)
        );
        assert_eq!(
            preflight_version(
                b"<FinalDraft DocumentType='Script' Version='5' Version='6'><Content/></FinalDraft>"
            ),
            Ok(FdxVersionStatus::Duplicate)
        );
        assert_eq!(
            read_v6(b"<FinalDraft DocumentType='Script'><Content/></FinalDraft>"),
            Err(FdxError::MissingVersion)
        );
        assert_eq!(
            read_v6(root("latest").as_bytes()),
            Err(FdxError::InvalidVersion)
        );
        assert_eq!(
            read_v6(
                b"<FinalDraft DocumentType='Script' Version='5' Version='6'><Content/></FinalDraft>"
            ),
            Err(FdxError::DuplicateVersion)
        );
        assert_eq!(
            preflight_version(b"<FinalDraft DocumentType='Script' Version='6'><Content>"),
            Err(FdxError::MalformedXml)
        );
    }

    #[test]
    fn older_import_requires_consent_and_converts_only_title_and_content_atomically() {
        let input = b"<FinalDraft DocumentType='Script' Version='5'><DocumentRef/><TitlePage><Paragraph Type='Old Title Type'><Text>synthetic title</Text></Paragraph></TitlePage><Content><Paragraph Type='Old Body Type' id='old-1'><Text>synthetic old role</Text></Paragraph><Paragraph Type='Action' id='old-2'><Text>known role</Text></Paragraph></Content><DisplayBoards><DisplayBoard ID='board-1'/></DisplayBoards></FinalDraft>";
        let original = input.to_vec();
        let cancelled_input =
            b"<FinalDraft DocumentType='Script' Version='5'><TitlePage/></FinalDraft>";
        assert_eq!(
            import_fdx(cancelled_input, FdxMigrationConsent::Cancel),
            Ok(FdxImportOutcome::Cancelled {
                declared_version: "5".to_owned(),
            })
        );
        assert_eq!(
            cancelled_input,
            b"<FinalDraft DocumentType='Script' Version='5'><TitlePage/></FinalDraft>"
        );
        assert_eq!(input, original.as_slice());

        let outcome = import_fdx(input, FdxMigrationConsent::Continue);
        assert!(matches!(&outcome, Ok(FdxImportOutcome::Candidate(_))));
        let Ok(FdxImportOutcome::Candidate(candidate)) = outcome else {
            return;
        };
        assert_eq!(
            candidate.migration,
            Some(FdxMigrationKind::OlderLossy {
                declared: "5".to_owned(),
            })
        );
        assert_eq!(candidate.document.project.elements.len(), 2);
        assert_eq!(candidate.document.project.elements[0].role, Role::General);
        assert_eq!(
            candidate.document.project.elements[0].text,
            "synthetic old role"
        );
        assert_eq!(candidate.document.project.elements[1].role, Role::Action);
        assert_eq!(candidate.document.project.title_page.len(), 1);
        assert_eq!(candidate.document.project.title_page[0].role, Role::General);
        assert_eq!(input, original.as_slice());

        let emitted_result = write_v6(&candidate.document.project);
        assert!(emitted_result.is_ok());
        let Ok(emitted) = emitted_result else {
            return;
        };
        let emitted_root_result = parse_document(&emitted);
        assert!(emitted_root_result.is_ok());
        let Ok(emitted_root) = emitted_root_result else {
            return;
        };
        let root_result = emitted_root.iter().find_map(|node| match node {
            RawNode::Element(root) => Some(root),
            _ => None,
        });
        assert!(root_result.is_some());
        let Some(root) = root_result else { return };
        assert_eq!(attribute(root, "Version"), Some("6"));
        let sections = root
            .children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(element) => Some(element.name.local_name.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(sections
            .iter()
            .all(|name| *name == "TitlePage" || *name == "Content"));
        assert!(!sections
            .iter()
            .any(|name| *name == "DocumentRef" || *name == "DisplayBoards"));
    }

    #[test]
    fn older_import_rejects_missing_content_and_unconvertible_paragraphs() {
        let missing_content =
            b"<FinalDraft DocumentType='Script' Version='5'><TitlePage/></FinalDraft>";
        assert_eq!(
            import_fdx(missing_content, FdxMigrationConsent::Continue),
            Err(FdxError::MissingContent)
        );
        let unconvertible = b"<FinalDraft DocumentType='Script' Version='5'><Content><Paragraph Type='Action'><Text><Unknown/></Text></Paragraph></Content></FinalDraft>";
        assert_eq!(
            import_fdx(unconvertible, FdxMigrationConsent::Continue),
            Err(FdxError::UnsupportedConstruct)
        );
        let non_paragraph_content = b"<FinalDraft DocumentType='Script' Version='5'><Content><UnknownElement/></Content></FinalDraft>";
        assert_eq!(
            import_fdx(non_paragraph_content, FdxMigrationConsent::Continue),
            Err(FdxError::UnsupportedConstruct)
        );
    }

    #[test]
    fn newer_import_is_explicitly_best_effort_and_preserves_unknown_sections() {
        let input = b"<FinalDraft DocumentType='Script' Version='7'><Content><Paragraph Type='Action' id='p1'><Text>newer text</Text></Paragraph></Content><FutureSection><Data value='opaque'/></FutureSection></FinalDraft>";
        let outcome = import_fdx(input, FdxMigrationConsent::Continue);
        assert!(matches!(&outcome, Ok(FdxImportOutcome::Candidate(_))));
        let Ok(FdxImportOutcome::Candidate(candidate)) = outcome else {
            return;
        };
        assert_eq!(
            candidate.migration,
            Some(FdxMigrationKind::NewerBestEffort {
                declared: "7".to_owned(),
            })
        );
        assert_eq!(candidate.document.project.elements[0].text, "newer text");
        let emitted_result = write_v6(&candidate.document.project);
        assert!(emitted_result.is_ok());
        let Ok(emitted) = emitted_result else { return };
        assert!(std::str::from_utf8(&emitted).is_ok_and(|text| text.contains("FutureSection")));
        assert!(std::str::from_utf8(&emitted).is_ok_and(|text| text.contains("Version=\"6\"")));
    }

    #[test]
    fn rejects_duplicate_ids_and_invalid_utf8() {
        let duplicate = b"<FinalDraft DocumentType='Script' Version='6'><Content><Paragraph Type='Action' id='same'><Text>a</Text></Paragraph><Paragraph Type='Action' id='same'><Text>b</Text></Paragraph></Content></FinalDraft>";
        assert_eq!(read_v6(duplicate), Err(FdxError::AmbiguousIdentity));
        assert_eq!(read_v6(&[0xff, 0xfe]), Err(FdxError::InvalidEncoding));
        let mixed_case_duplicate = b"<FinalDraft DocumentType='Script' Version='6'><Content><Paragraph Type='Action' id='one' ID='two'><Text>x</Text></Paragraph></Content></FinalDraft>";
        assert_eq!(
            read_v6(mixed_case_duplicate),
            Err(FdxError::AmbiguousIdentity)
        );
    }

    #[test]
    fn rejects_configured_depth_and_byte_limits() {
        let oversized = vec![b' '; MAX_SOURCE_BYTES + 1];
        assert_eq!(read_v6(&oversized), Err(FdxError::SourceTooLarge));
        let mut nested = String::from("<FinalDraft DocumentType='Script' Version='6'><Content>");
        nested.push_str(&"<x>".repeat(MAX_SOURCE_DEPTH + 1));
        nested.push_str(&"</x>".repeat(MAX_SOURCE_DEPTH + 1));
        nested.push_str("</Content></FinalDraft>");
        assert_eq!(read_v6(nested.as_bytes()), Err(FdxError::ResourceLimit));

        let mut at_depth = String::from("<FinalDraft DocumentType='Script' Version='6'><Content>");
        at_depth.push_str(&"<x/>".repeat(MAX_SOURCE_DEPTH - 2));
        at_depth.push_str("</Content></FinalDraft>");
        assert!(read_v6(at_depth.as_bytes()).is_ok());
    }

    #[test]
    fn enforces_event_attribute_text_and_image_budgets() {
        let mut exact_events =
            String::from("<FinalDraft DocumentType='Script' Version='6'><Content>");
        exact_events.push_str(&"<x/>".repeat(MAX_SOURCE_EVENTS - 5));
        exact_events.push_str("</Content></FinalDraft>");
        assert!(read_v6(exact_events.as_bytes()).is_ok());
        let insertion = exact_events
            .find("</Content>")
            .unwrap_or(exact_events.len());
        exact_events.insert_str(insertion, "<x/>");
        assert_eq!(
            read_v6(exact_events.as_bytes()),
            Err(FdxError::ResourceLimit)
        );

        let attributes = (0..MAX_ATTRIBUTES_PER_ELEMENT)
            .map(|index| format!(" a{index}='v'"))
            .collect::<String>();
        let allowed = format!(
            "<FinalDraft DocumentType='Script' Version='6'><Content><x{attributes}/></Content></FinalDraft>"
        );
        assert!(read_v6(allowed.as_bytes()).is_ok());
        let rejected = format!(
            "<FinalDraft DocumentType='Script' Version='6'><Content><x{attributes} extra='v'/></Content></FinalDraft>"
        );
        assert_eq!(read_v6(rejected.as_bytes()), Err(FdxError::ResourceLimit));

        let attribute_value = "v".repeat(MAX_SOURCE_NAME_OR_VALUE_BYTES + 1);
        let oversized_field = format!(
            "<FinalDraft DocumentType='Script' Version='6'><Content><x value='{attribute_value}'/></Content></FinalDraft>"
        );
        assert_eq!(
            read_v6(oversized_field.as_bytes()),
            Err(FdxError::ResourceLimit)
        );

        let many_attributes = (0..MAX_ATTRIBUTES_PER_ELEMENT)
            .map(|index| format!(" a{index}='v'"))
            .collect::<String>();
        let under_aggregate_attribute_limit = format!(
            "<FinalDraft DocumentType='Script' Version='6'><Content>{}</Content></FinalDraft>",
            format!("<x{many_attributes}/>")
                .repeat(MAX_SOURCE_EVENTS / (MAX_ATTRIBUTES_PER_ELEMENT + 1))
        );
        assert!(read_v6(under_aggregate_attribute_limit.as_bytes()).is_ok());
        let over_aggregate_attribute_limit = format!(
            "<FinalDraft DocumentType='Script' Version='6'><Content>{}</Content></FinalDraft>",
            format!("<x{many_attributes}/>")
                .repeat(MAX_SOURCE_EVENTS / (MAX_ATTRIBUTES_PER_ELEMENT + 1) + 1)
        );
        assert_eq!(
            read_v6(over_aggregate_attribute_limit.as_bytes()),
            Err(FdxError::ResourceLimit)
        );

        let exact_text = format!(
            "<FinalDraft DocumentType='Script' Version='6'><Content>{}</Content></FinalDraft>",
            "x".repeat(MAX_SOURCE_TEXT_BYTES)
        );
        assert!(read_v6(exact_text.as_bytes()).is_ok());
        let over_text = format!(
            "<FinalDraft DocumentType='Script' Version='6'><Content>{}</Content></FinalDraft>",
            "x".repeat(MAX_SOURCE_TEXT_BYTES + 1)
        );
        assert_eq!(read_v6(over_text.as_bytes()), Err(FdxError::ResourceLimit));

        let per_image_limit = (MAX_IMAGE_BYTES * 4).div_ceil(3);
        let over_image = format!(
            "<FinalDraft DocumentType='Script' Version='6'><Content/><Images><Image ID='img'>{}</Image></Images></FinalDraft>",
            "A".repeat(per_image_limit + 1)
        );
        assert_eq!(read_v6(over_image.as_bytes()), Err(FdxError::ResourceLimit));
    }

    #[test]
    fn known_role_projection_does_not_coerce_unknown_types() {
        let xml = b"<FinalDraft DocumentType='Script' Version='6'><Content><Paragraph Type='Vendor Role' id='x'><Text>source-owned</Text></Paragraph><Paragraph Type='Action' id='y'><Text>Caf\xc3\xa9 \xf0\x9f\x99\x82 &amp; B</Text></Paragraph></Content></FinalDraft>";
        let result = read_v6(xml);
        assert!(result.is_ok());
        if let Ok(result) = result {
            assert_eq!(result.project.elements.len(), 1);
            assert_eq!(result.project.elements[0].text, "Café 🙂 & B");
            assert_eq!(result.project.elements[0].role, Role::Action);
        }
    }

    #[test]
    fn imports_explicit_hidden_and_page_start_role_settings() {
        let xml = b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Note'><FontSpec Style='Italic+HiddenText'/></ElementSettings><ElementSettings Type='New Act'><ParagraphSpec Type='New Act' StartsNewPage='Yes'/></ElementSettings><Content><Paragraph Type='Note' id='note'><Text>hidden</Text></Paragraph><Paragraph Type='New Act' id='act'><Text>act</Text></Paragraph></Content></FinalDraft>";
        let result = read_v6(xml);
        assert!(result.is_ok());
        if let Ok(result) = result {
            assert_eq!(
                result.project.page_profile.print_visibility(Role::Note),
                PrintVisibility::Hidden
            );
            assert!(result.project.page_profile.starts_new_page(Role::NewAct));
        }
    }

    #[test]
    fn retains_and_validates_internal_board_item_and_image_links() {
        let xml = b"<FinalDraft DocumentType='Script' Version='6'><Content/><ListItems><ListItem ID='item-1'/></ListItems><DisplayBoards><DisplayBoard ID='board-1'><Item id='item-1'/><Picture id='image-1'/></DisplayBoard></DisplayBoards><Images><Image ID='image-1' Length='0'/></Images></FinalDraft>";
        let result = read_v6(xml);
        assert!(result.is_ok());
        if let Ok(result) = result {
            let graph = result
                .project
                .preservation
                .as_ref()
                .map(|state| &state.board_graph);
            assert!(graph.is_some_and(|graph| graph.entities.len() == 3));
            assert!(graph.is_some_and(|graph| graph.references.len() == 2));
        }
        let dangling = b"<FinalDraft DocumentType='Script' Version='6'><Content/><DisplayBoards><DisplayBoard ID='board-1'><Item id='missing'/></DisplayBoard></DisplayBoards></FinalDraft>";
        assert_eq!(read_v6(dangling), Err(FdxError::DanglingReference));
    }

    #[test]
    fn writes_all_evidenced_roles_with_escaped_unicode_and_runs() {
        let mut project = Project::new("writer-project", "", "").unwrap_or_else(|_| unreachable!());
        project.elements.clear();
        for (index, role) in Role::ALL.into_iter().enumerate() {
            let text = if index == 0 {
                "INT. CAFÉ & <ROOM> 🙂".to_owned()
            } else {
                format!("role-{index}")
            };
            let attributes = if index == 0 {
                vec![RunAttribute {
                    name: "Style".into(),
                    value: "Bold+Italic".into(),
                }]
            } else {
                Vec::new()
            };
            let element = ScreenplayElement::with_runs(
                format!("element-{index}"),
                role,
                vec![TextRun { text, attributes }],
                None,
            )
            .unwrap_or_else(|_| unreachable!());
            project.elements.push(element);
        }
        project.title_page.push(
            ScreenplayElement::new("title-element", Role::Action, "Title & Subtitle")
                .unwrap_or_else(|_| unreachable!()),
        );

        let bytes = write_v6(&project).unwrap_or_else(|_| unreachable!());
        let xml = std::str::from_utf8(&bytes).unwrap_or("");
        assert!(xml.contains("CAFÉ &amp; &lt;ROOM&gt; 🙂"));
        assert!(xml.contains("Style=\"Bold+Italic\""));
        let reopened = read_v6(&bytes).unwrap_or_else(|_| unreachable!());
        assert!(same_semantic_elements(
            &project.elements,
            &reopened.project.elements
        ));
        assert!(same_semantic_elements(
            &project.title_page,
            &reopened.project.title_page
        ));
        assert_eq!(
            reopened
                .project
                .elements
                .iter()
                .map(|element| element.role)
                .collect::<Vec<_>>(),
            Role::ALL
        );
    }

    #[test]
    fn reconciles_managed_edits_and_retains_opaque_namespaced_source() {
        let mut project = read_v6(SYNTHETIC)
            .unwrap_or_else(|_| unreachable!())
            .project;
        let original = project
            .preservation
            .as_ref()
            .map(|state| state.source.original_bytes.clone())
            .unwrap_or_default();
        project.elements[3].runs[0].text = "edited < & unicode 🙂".into();
        project.elements[3].text = project.elements[3]
            .runs
            .iter()
            .map(|run| run.text.as_str())
            .collect();

        let bytes = write_v6(&project).unwrap_or_else(|_| unreachable!());
        assert_ne!(bytes, original);
        assert!(std::str::from_utf8(&bytes)
            .unwrap_or("")
            .contains("edited &lt; &amp; unicode 🙂"));
        let reopened = read_v6(&bytes).unwrap_or_else(|_| unreachable!());
        assert_eq!(
            reopened.project.elements[3].text,
            "edited < & unicode 🙂dialogue"
        );
        let original_state = project.preservation.as_ref();
        let reopened_state = reopened.project.preservation.as_ref();
        assert!(original_state.is_some_and(|state| {
            reopened_state.is_some_and(|reopened| {
                state.board_graph == reopened.board_graph
                    && state.identities == reopened.identities
                    && state.references == reopened.references
                    && state.source.original_bytes == original
            })
        }));
        assert!(bytes
            .windows(b"<test:Opaque".len())
            .any(|window| window == b"<test:Opaque"));
        assert!(bytes
            .windows(b"test:flag=\"keep\"".len())
            .any(|window| window == b"test:flag=\"keep\""));
    }

    #[test]
    fn retains_synthetic_image_payload_and_board_links_when_writing() {
        let xml = b"<FinalDraft DocumentType='Script' Version='6'><Content><Paragraph Type='Action' id='p1'><Text>preserve me</Text></Paragraph></Content><SmartType><Suggestion>synthetic</Suggestion></SmartType><ListItems><ListItem ID='item-1'/></ListItems><DisplayBoards><DisplayBoard ID='board-1'><Item id='item-1'/><Picture id='image-1'/></DisplayBoard></DisplayBoards><Images><Image ID='image-1' Length='5'>AAECAwQ=</Image></Images></FinalDraft>";
        let project = read_v6(xml).unwrap_or_else(|_| unreachable!()).project;
        let bytes = write_v6(&project).unwrap_or_else(|_| unreachable!());
        let reopened = read_v6(&bytes).unwrap_or_else(|_| unreachable!());
        let graph = reopened
            .project
            .preservation
            .as_ref()
            .map(|state| &state.board_graph);
        assert!(graph.is_some_and(|graph| {
            graph.entities.len() == 3
                && graph.references.len() == 2
                && graph.entities.iter().any(|entity| {
                    entity.kind == BoardEntityKind::Image
                        && entity.declared_image_length == Some(5)
                        && entity.image_payload.is_none()
                })
        }));
        assert!(std::str::from_utf8(&bytes)
            .unwrap_or("")
            .contains("<SmartType>"));
        assert!(std::str::from_utf8(&bytes)
            .unwrap_or("")
            .contains("AAECAwQ="));
    }

    #[test]
    fn preserves_ordered_title_page_formatting_and_header_footer() {
        let xml = b"<FinalDraft DocumentType='Script' Version='6'><Content><Paragraph Type='Scene Heading' id='body-1'><Text>INT. ROOM</Text></Paragraph></Content><TitlePage><HeaderAndFooter><Header>synthetic header</Header></HeaderAndFooter><Paragraph Type='Action' id='title-1' Alignment='Center'><Text Style='Underline+AllCaps'>TITLE</Text></Paragraph><Paragraph><Text>unmapped title line</Text></Paragraph></TitlePage></FinalDraft>";
        let project = read_v6(xml).unwrap_or_else(|_| unreachable!()).project;
        assert_eq!(project.title_page.len(), 1);
        let bytes = write_v6(&project).unwrap_or_else(|_| unreachable!());
        let reopened = read_v6(&bytes).unwrap_or_else(|_| unreachable!());
        assert_eq!(reopened.project.title_page[0].text, "TITLE");
        assert_eq!(
            reopened.project.title_page[0].runs[0].attributes[0].value,
            "Underline+AllCaps"
        );
        let output = std::str::from_utf8(&bytes).unwrap_or("");
        assert!(output.contains("synthetic header"));
        assert!(output.contains("unmapped title line"));
        assert!(output.contains("Alignment=\"Center\""));
    }

    #[test]
    fn rejects_unmapped_app_state_and_unsafe_source_reconciliation() {
        let mut metadata = read_v6(SYNTHETIC)
            .unwrap_or_else(|_| unreachable!())
            .project;
        metadata.metadata.title = "not mapped".into();
        assert_eq!(write_v6(&metadata), Err(FdxError::UnsupportedAppState));

        let mut inserted = read_v6(SYNTHETIC)
            .unwrap_or_else(|_| unreachable!())
            .project;
        inserted.elements.push(
            ScreenplayElement::new("unanchored-new", Role::Action, "new")
                .unwrap_or_else(|_| unreachable!()),
        );
        assert_eq!(write_v6(&inserted), Err(FdxError::UnsupportedConstruct));

        let mut invalid_text =
            Project::new("invalid-text", "", "").unwrap_or_else(|_| unreachable!());
        invalid_text.elements[0].text = "bad\u{1}".into();
        assert_eq!(write_v6(&invalid_text), Err(FdxError::InvalidValue));
    }
}

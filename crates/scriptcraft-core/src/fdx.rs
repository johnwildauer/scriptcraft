//! Bounded, read-only FDX Version 6 import.
//!
//! The result is a semantic editing projection plus the complete parsed source
//! tree and original bytes. ScriptCraft-only identity, schema, page-profile and
//! edit-policy mappings are not established by the inspected FDX evidence; this
//! reader therefore does not enable native save/reopen or claim interoperability.

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
use std::collections::BTreeSet;
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
    InvalidRoot,
    MissingContent,
    DuplicateSection,
    ResourceLimit,
    InvalidValue,
    AmbiguousIdentity,
    DanglingReference,
    UnsupportedConstruct,
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
    if attribute(root, "Version") != Some(FDX_VERSION) {
        return Err(FdxError::UnsupportedVersion);
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
                let element = make_element(&reader, &start)?;
                add_retained_budget(
                    &mut retained_count,
                    1usize
                        .saturating_add(element.attributes.len())
                        .saturating_add(element.namespace_bindings.len()),
                )?;
                stack.push(element);
            }
            Event::Empty(start) => {
                let element = make_element(&reader, &start)?;
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
) -> Result<RawElement, FdxError> {
    let (resolved, local) = reader.resolver().resolve_element(start.name());
    let (namespace_uri, local_name) = resolved_name(resolved, local.as_ref())?;
    if start.name().as_ref().len() > MAX_SOURCE_NAME_OR_VALUE_BYTES {
        return Err(FdxError::ResourceLimit);
    }
    let qualified_name = utf8(start.name().as_ref())?.to_owned();
    let mut attributes = Vec::new();
    let mut namespace_bindings = Vec::new();
    for item in start.attributes().with_checks(true) {
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
}

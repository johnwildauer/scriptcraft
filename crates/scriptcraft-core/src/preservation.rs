//! Bounded, source-owned FDX preservation state.
//!
//! The FDX codec retains the parsed source tree and original byte snapshot
//! separately. The ordered tree is the serialization authority for source-owned
//! content; byte identity and external-application interoperability are not
//! implied.

use crate::{
    Role, ScreenplayElement, SourceFormat, SourceLayoutSettings, SourceProvenance, SourceSection,
    StableId, TextRun,
};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_SOURCE_EVENTS: usize = 100_000;
pub const MAX_SOURCE_DEPTH: usize = 128;
pub const MAX_ATTRIBUTES_PER_ELEMENT: usize = 128;
pub const MAX_SOURCE_NAME_OR_VALUE_BYTES: usize = 64 * 1024;
pub const MAX_SOURCE_TEXT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_TOTAL_IMAGE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreservationError {
    SourceTooLarge,
    InvalidEncoding,
    TooManyEvents,
    ExcessiveDepth,
    TooManyAttributes,
    FieldTooLarge,
    TextTooLarge,
    ImageTooLarge,
    TotalImageBytesTooLarge,
    DuplicateSourceOrder,
    InvalidSourceName,
    InvalidAnchor,
    DuplicateAnchor,
    UnboundAnchor,
    AnchorTargetMissing,
    DuplicateIdentity,
    MissingReferenceSource,
    DanglingReference,
    ImageLengthMismatch,
    ManagedAnchorMissing,
    ManagedAnchorAmbiguous,
    ManagedTextMismatch,
    UnsupportedTextMapping,
    UnsupportedRoleMapping,
    StructuralAttachmentConflict,
    PotentialReferenceConflict,
}

/// The source's expanded name and, when available, original qualified spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceName {
    pub namespace_uri: Option<String>,
    pub local_name: String,
    pub qualified_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceBinding {
    pub prefix: String,
    pub namespace_uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreservedAttribute {
    pub name: SourceName,
    pub value: String,
}

/// Confidence is intentionally one-sided: neither variant asserts that an
/// unrecognized section is independent of other document data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyConfidence {
    Unknown,
    SomeReferencesMapped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceOwnership {
    Opaque,
    Managed(StableId),
    Attached(ForeignIdentity),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreservedElement {
    pub name: SourceName,
    /// Ordered, including attributes the semantic model does not understand.
    pub attributes: Vec<PreservedAttribute>,
    /// Namespace declarations are retained separately and in source order.
    pub namespace_bindings: Vec<NamespaceBinding>,
    /// Original role/type spelling, including unknown/custom values.
    pub source_type: Option<String>,
    pub ownership: SourceOwnership,
    pub dependency_confidence: DependencyConfidence,
    pub anchor: Option<SourceAnchorId>,
    /// Ordered mixed content; do not flatten into a text projection.
    pub children: Vec<PreservedNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreservedNode {
    Element(PreservedElement),
    Text(String),
    CData(String),
    Comment(String),
    ProcessingInstruction { target: String, data: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionKind {
    Content,
    TitlePage,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreservedSection {
    pub name: SourceName,
    pub kind: SectionKind,
    /// Original top-level order. Repeated/unknown sections are permitted.
    pub ordinal: u32,
    pub children: Vec<PreservedNode>,
}

/// Bytes are the untouched source snapshot, not proof that a future edited
/// serialization is byte-identical. The tree separately retains ordered source
/// structure for later reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSnapshot {
    pub format: SourceFormat,
    pub original_bytes: Vec<u8>,
    pub encoding: SourceEncoding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceEncoding {
    Utf8,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceAnchorId(String);

impl SourceAnchorId {
    pub fn new(value: impl Into<String>) -> Result<Self, PreservationError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 128
            || !value.bytes().all(|byte| byte.is_ascii_graphic())
        {
            return Err(PreservationError::InvalidAnchor);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ForeignIdDomain {
    Document,
    Paragraph,
    Scene,
    Beat,
    CastMember,
    Character,
    ListItem,
    Board,
    Image,
    Revision,
    Tag,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ForeignIdentity {
    pub domain: ForeignIdDomain,
    pub value: String,
}

impl ForeignIdentity {
    pub fn new(
        domain: ForeignIdDomain,
        value: impl Into<String>,
    ) -> Result<Self, PreservationError> {
        let value = value.into();
        if value.is_empty() {
            return Err(PreservationError::InvalidSourceName);
        }
        if value.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES {
            return Err(PreservationError::FieldTooLarge);
        }
        let identity = Self { domain, value };
        validate_identity(&identity)?;
        Ok(identity)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnchorTarget {
    ProjectElement(StableId),
    Foreign(ForeignIdentity),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorRelation {
    Represents,
    Before,
    After,
    Attached,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAnchor {
    pub id: SourceAnchorId,
    pub target: AnchorTarget,
    pub relation: AnchorRelation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferencePolicy {
    /// A known link within a preserved identity domain must resolve.
    RequiredInternal,
    /// The user permits script/board links to become orphaned; entities and
    /// links are still retained and no board entity is removed by this policy.
    CrossDomainMayOrphan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignReference {
    pub source: ForeignIdentity,
    pub target: ForeignIdentity,
    pub policy: ReferencePolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardEntityKind {
    ListItem,
    DisplayBoard,
    Image,
}

/// An authored board/item/image record. Its opaque XML remains in the ordered
/// section tree; an image payload is retained as bytes here. A path is data only
/// and must never be resolved or logged by consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardEntity {
    pub identity: ForeignIdentity,
    pub kind: BoardEntityKind,
    pub source_order: u32,
    pub anchor: Option<SourceAnchorId>,
    pub image_payload: Option<Vec<u8>>,
    pub declared_image_length: Option<u64>,
    pub source_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BoardGraph {
    /// Kept in source order; script/board independence does not permit dropping
    /// these authored entities or their own internal links.
    pub entities: Vec<BoardEntity>,
    pub references: Vec<ForeignReference>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreservationState {
    pub source: SourceSnapshot,
    /// Ephemeral projection of recognized source layout settings for runtime
    /// behavior. The complete source values in `sections` and
    /// `source.original_bytes` remain authoritative for serialization.
    pub(crate) source_layout_settings: SourceLayoutSettings,
    pub sections: Vec<PreservedSection>,
    pub anchors: Vec<SourceAnchor>,
    pub identities: Vec<ForeignIdentity>,
    pub references: Vec<ForeignReference>,
    pub board_graph: BoardGraph,
}

impl PreservationState {
    pub fn source_layout_settings(&self) -> &SourceLayoutSettings {
        &self.source_layout_settings
    }

    pub fn validate(&self) -> Result<(), PreservationError> {
        if self.source.original_bytes.len() > MAX_SOURCE_BYTES {
            return Err(PreservationError::SourceTooLarge);
        }
        if std::str::from_utf8(&self.source.original_bytes).is_err() {
            return Err(PreservationError::InvalidEncoding);
        }
        if self.sections.len() > MAX_SOURCE_EVENTS {
            return Err(PreservationError::TooManyEvents);
        }

        let mut event_count = self.sections.len();
        let mut text_bytes = 0usize;
        let mut anchor_uses = BTreeMap::<SourceAnchorId, usize>::new();
        let mut ownership_bindings = Vec::new();
        let mut previous_section_order = None;
        for section in &self.sections {
            if previous_section_order.is_some_and(|order| section.ordinal <= order) {
                return Err(PreservationError::DuplicateSourceOrder);
            }
            previous_section_order = Some(section.ordinal);
            validate_name(&section.name)?;
            let mut stack = section
                .children
                .iter()
                .rev()
                .map(|node| (node, 1usize))
                .collect::<Vec<_>>();
            while let Some((node, depth)) = stack.pop() {
                if depth > MAX_SOURCE_DEPTH {
                    return Err(PreservationError::ExcessiveDepth);
                }
                event_count = event_count.saturating_add(1);
                if event_count > MAX_SOURCE_EVENTS {
                    return Err(PreservationError::TooManyEvents);
                }
                match node {
                    PreservedNode::Element(element) => {
                        validate_name(&element.name)?;
                        if let Some(source_type) = &element.source_type {
                            validate_field(source_type)?;
                        }
                        if element.attributes.len() > MAX_ATTRIBUTES_PER_ELEMENT {
                            return Err(PreservationError::TooManyAttributes);
                        }
                        if element.namespace_bindings.len() > MAX_ATTRIBUTES_PER_ELEMENT {
                            return Err(PreservationError::TooManyAttributes);
                        }
                        event_count = event_count
                            .saturating_add(element.attributes.len())
                            .saturating_add(element.namespace_bindings.len());
                        if event_count > MAX_SOURCE_EVENTS {
                            return Err(PreservationError::TooManyEvents);
                        }
                        for attribute in &element.attributes {
                            validate_name(&attribute.name)?;
                            validate_field(&attribute.value)?;
                        }
                        for binding in &element.namespace_bindings {
                            validate_field(&binding.prefix)?;
                            validate_field(&binding.namespace_uri)?;
                        }
                        if let Some(anchor) = &element.anchor {
                            *anchor_uses.entry(anchor.clone()).or_default() += 1;
                        }
                        ownership_bindings
                            .push((element.anchor.clone(), element.ownership.clone()));
                        validate_ownership(&element.ownership)?;
                        stack.extend(
                            element
                                .children
                                .iter()
                                .rev()
                                .map(|child| (child, depth.saturating_add(1))),
                        );
                    }
                    PreservedNode::Text(text)
                    | PreservedNode::CData(text)
                    | PreservedNode::Comment(text) => {
                        text_bytes = text_bytes.saturating_add(text.len());
                        if text_bytes > MAX_SOURCE_TEXT_BYTES {
                            return Err(PreservationError::TextTooLarge);
                        }
                    }
                    PreservedNode::ProcessingInstruction { target, data } => {
                        if target.is_empty() {
                            return Err(PreservationError::InvalidSourceName);
                        }
                        validate_field(target)?;
                        text_bytes = text_bytes.saturating_add(data.len());
                        if text_bytes > MAX_SOURCE_TEXT_BYTES {
                            return Err(PreservationError::TextTooLarge);
                        }
                    }
                }
            }
        }

        let mut defined = BTreeSet::new();
        event_count = event_count
            .saturating_add(self.anchors.len())
            .saturating_add(self.identities.len())
            .saturating_add(self.references.len())
            .saturating_add(self.board_graph.entities.len())
            .saturating_add(self.board_graph.references.len());
        if event_count > MAX_SOURCE_EVENTS {
            return Err(PreservationError::TooManyEvents);
        }
        for identity in self.identities.iter().chain(
            self.board_graph
                .entities
                .iter()
                .map(|entity| &entity.identity),
        ) {
            validate_identity(identity)?;
            if !defined.insert(identity.clone()) {
                return Err(PreservationError::DuplicateIdentity);
            }
        }

        let mut anchor_ids = BTreeSet::new();
        for anchor in &self.anchors {
            if !anchor_ids.insert(anchor.id.clone()) {
                return Err(PreservationError::DuplicateAnchor);
            }
            if anchor_uses.get(&anchor.id) != Some(&1) {
                return Err(PreservationError::UnboundAnchor);
            }
            if let AnchorTarget::Foreign(identity) = &anchor.target {
                if !defined.contains(identity) {
                    return Err(PreservationError::DanglingReference);
                }
            }
        }
        let anchor_targets = self
            .anchors
            .iter()
            .map(|anchor| (anchor.id.clone(), (&anchor.target, anchor.relation)))
            .collect::<BTreeMap<_, _>>();
        for (anchor, ownership) in &ownership_bindings {
            match ownership {
                SourceOwnership::Opaque => {}
                SourceOwnership::Managed(id) => {
                    if !anchor.as_ref().is_some_and(|anchor| {
                        matches!(
                            anchor_targets.get(anchor),
                            Some((AnchorTarget::ProjectElement(target), AnchorRelation::Represents))
                                if target == id
                        )
                    }) {
                        return Err(PreservationError::InvalidAnchor);
                    }
                }
                SourceOwnership::Attached(identity) => {
                    if !defined.contains(identity)
                        || !anchor.as_ref().is_some_and(|anchor| {
                            matches!(
                                anchor_targets.get(anchor),
                                Some((AnchorTarget::Foreign(target), AnchorRelation::Attached))
                                    if target == identity
                            )
                        })
                    {
                        return Err(PreservationError::DanglingReference);
                    }
                }
            }
        }
        if anchor_uses
            .keys()
            .any(|anchor| !anchor_ids.contains(anchor))
        {
            return Err(PreservationError::UnboundAnchor);
        }

        for reference in self.references.iter().chain(&self.board_graph.references) {
            validate_identity(&reference.source)?;
            validate_identity(&reference.target)?;
            if !defined.contains(&reference.source) {
                return Err(PreservationError::MissingReferenceSource);
            }
            match reference.policy {
                ReferencePolicy::RequiredInternal => {
                    if !defined.contains(&reference.target) {
                        return Err(PreservationError::DanglingReference);
                    }
                }
                ReferencePolicy::CrossDomainMayOrphan => {
                    if is_board_domain(&reference.source.domain)
                        == is_board_domain(&reference.target.domain)
                    {
                        return Err(PreservationError::DanglingReference);
                    }
                }
            }
        }

        let mut image_bytes = 0usize;
        let mut previous_order = None;
        for entity in &self.board_graph.entities {
            if previous_order.is_some_and(|order| entity.source_order <= order) {
                return Err(PreservationError::DuplicateSourceOrder);
            }
            previous_order = Some(entity.source_order);
            if entity.kind == BoardEntityKind::Image {
                if let Some(payload) = &entity.image_payload {
                    if payload.len() > MAX_IMAGE_BYTES {
                        return Err(PreservationError::ImageTooLarge);
                    }
                    image_bytes = image_bytes.saturating_add(payload.len());
                    if image_bytes > MAX_TOTAL_IMAGE_BYTES {
                        return Err(PreservationError::TotalImageBytesTooLarge);
                    }
                    if entity
                        .declared_image_length
                        .is_some_and(|length| length != payload.len() as u64)
                    {
                        return Err(PreservationError::ImageLengthMismatch);
                    }
                }
            } else if entity.image_payload.is_some() || entity.declared_image_length.is_some() {
                return Err(PreservationError::ImageLengthMismatch);
            }
            if let Some(path) = &entity.source_path {
                validate_field(path)?;
            }
            if let Some(anchor) = &entity.anchor {
                if !anchor_ids.contains(anchor) {
                    return Err(PreservationError::UnboundAnchor);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn validate_project_elements(
        &self,
        project_element_ids: &BTreeSet<StableId>,
    ) -> Result<(), PreservationError> {
        self.validate()?;
        for anchor in &self.anchors {
            if let AnchorTarget::ProjectElement(id) = &anchor.target {
                if !project_element_ids.contains(id) {
                    return Err(PreservationError::AnchorTargetMissing);
                }
            }
        }
        let mut stack = self
            .sections
            .iter()
            .flat_map(|section| section.children.iter())
            .collect::<Vec<_>>();
        while let Some(node) = stack.pop() {
            if let PreservedNode::Element(element) = node {
                if let SourceOwnership::Managed(id) = &element.ownership {
                    if !project_element_ids.contains(id) {
                        return Err(PreservationError::AnchorTargetMissing);
                    }
                }
                stack.extend(element.children.iter());
            }
        }
        Ok(())
    }

    pub(crate) fn has_managed_element(&self, id: &StableId) -> bool {
        self.sections
            .iter()
            .any(|section| contains_managed_element(&section.children, id))
    }

    /// Applies a UTF-8 byte-range edit to the exact ordered text children of one
    /// managed paragraph. If semantic runs are present, each run must correspond
    /// one-to-one with a source text child; otherwise the source mapping is not
    /// sufficiently explicit to mutate safely. At a source-child boundary,
    /// insertion uses right affinity except at end-of-text, which uses left
    /// affinity; a replacement crossing an intervening opaque child is rejected.
    pub(crate) fn replace_managed_text(
        &mut self,
        id: &StableId,
        old_text: &str,
        start: usize,
        end: usize,
        inserted: &str,
        expected_runs: Option<&[String]>,
    ) -> Result<(), PreservationError> {
        if start > end
            || end > old_text.len()
            || !old_text.is_char_boundary(start)
            || !old_text.is_char_boundary(end)
        {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let element = self.managed_element_mut(id)?;
        if element.children.iter().any(
            |node| matches!(node, PreservedNode::Element(child) if is_text_element(child, element)),
        ) {
            return replace_text_element_children(
                element,
                old_text,
                start,
                end,
                inserted,
                expected_runs,
            );
        }
        let indexes = direct_text_child_indexes(element);
        if indexes.is_empty() {
            return replace_text_element_children(
                element,
                old_text,
                start,
                end,
                inserted,
                expected_runs,
            );
        }
        let source_segments = indexes
            .iter()
            .map(|index| match &element.children[*index] {
                PreservedNode::Text(text) | PreservedNode::CData(text) => text.clone(),
                _ => unreachable!(),
            })
            .collect::<Vec<_>>();
        if source_segments.concat() != old_text {
            return Err(PreservationError::ManagedTextMismatch);
        }
        if expected_runs.is_some_and(|runs| runs != source_segments.as_slice()) {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let mut boundary = 0usize;
        for pair in indexes.windows(2) {
            let previous = pair[0];
            let next = pair[1];
            if element.children[previous + 1..next]
                .iter()
                .any(|node| !matches!(node, PreservedNode::Text(_) | PreservedNode::CData(_)))
            {
                boundary = boundary.saturating_add(match &element.children[previous] {
                    PreservedNode::Text(text) | PreservedNode::CData(text) => text.len(),
                    _ => unreachable!(),
                });
                if start < boundary && end > boundary {
                    return Err(PreservationError::UnsupportedTextMapping);
                }
            } else {
                boundary = boundary.saturating_add(match &element.children[previous] {
                    PreservedNode::Text(text) | PreservedNode::CData(text) => text.len(),
                    _ => unreachable!(),
                });
            }
        }
        let updated = replace_segments(&source_segments, start, end, inserted)?;
        for (index, text) in indexes.into_iter().zip(updated) {
            match &mut element.children[index] {
                PreservedNode::Text(value) | PreservedNode::CData(value) => *value = text,
                _ => unreachable!(),
            }
        }
        Ok(())
    }

    /// Replaces the complete plain-text projection of a managed FDX paragraph
    /// whose text is represented by ordered `<Text>` elements. The caller must
    /// establish that those runs carry no formatting; opaque or mixed children
    /// are rejected rather than moved or discarded.
    pub(crate) fn replace_managed_text_elements(
        &mut self,
        id: &StableId,
        expected_runs: &[String],
        replacement: &str,
    ) -> Result<(), PreservationError> {
        if replacement.len() > crate::MAX_TEXT_BYTES {
            return Err(PreservationError::TextTooLarge);
        }
        let paragraph = self.managed_element_mut(id)?;
        let mut text_elements = Vec::new();
        for (index, node) in paragraph.children.iter().enumerate() {
            match node {
                PreservedNode::Element(element)
                    if element.name.local_name == "Text"
                        && element.name.namespace_uri == paragraph.name.namespace_uri =>
                {
                    text_elements.push(index);
                }
                PreservedNode::Text(text) | PreservedNode::CData(text)
                    if text.trim().is_empty() => {}
                _ => return Err(PreservationError::UnsupportedTextMapping),
            }
        }
        if text_elements.is_empty() || text_elements.len() != expected_runs.len() {
            return Err(PreservationError::UnsupportedTextMapping);
        }

        for (index, expected) in text_elements.iter().zip(expected_runs) {
            let PreservedNode::Element(text_element) = &paragraph.children[*index] else {
                return Err(PreservationError::UnsupportedTextMapping);
            };
            if !text_element.attributes.is_empty() {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            let mut actual = String::new();
            for child in &text_element.children {
                match child {
                    PreservedNode::Text(text) | PreservedNode::CData(text) => {
                        actual.push_str(text);
                    }
                    _ => return Err(PreservationError::UnsupportedTextMapping),
                }
            }
            if &actual != expected {
                return Err(PreservationError::ManagedTextMismatch);
            }
        }

        for (run_index, index) in text_elements.into_iter().enumerate() {
            let PreservedNode::Element(text_element) = &mut paragraph.children[index] else {
                return Err(PreservationError::UnsupportedTextMapping);
            };
            if let Some(first) = text_element.children.first_mut() {
                match first {
                    PreservedNode::Text(text) | PreservedNode::CData(text) => {
                        *text = if run_index == 0 {
                            replacement.to_owned()
                        } else {
                            String::new()
                        };
                    }
                    _ => return Err(PreservationError::UnsupportedTextMapping),
                }
                for child in text_element.children.iter_mut().skip(1) {
                    match child {
                        PreservedNode::Text(text) | PreservedNode::CData(text) => text.clear(),
                        _ => return Err(PreservationError::UnsupportedTextMapping),
                    }
                }
            } else if run_index == 0 {
                text_element
                    .children
                    .push(PreservedNode::Text(replacement.to_owned()));
            }
        }
        Ok(())
    }

    /// Keeps source-owned text synchronized for an explicit cue-casing edit.
    /// Casing is mapped per ordered source run so Unicode expansions remain
    /// attached to their original run boundaries.
    pub(crate) fn uppercase_managed_text(
        &mut self,
        id: &StableId,
        old_text: &str,
        expected_runs: Option<&[String]>,
        expected_text: &str,
    ) -> Result<(), PreservationError> {
        let element = self.managed_element_mut(id)?;
        let indexes = direct_text_child_indexes(element);
        if indexes.is_empty() {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let source_segments = indexes
            .iter()
            .map(|index| match &element.children[*index] {
                PreservedNode::Text(text) | PreservedNode::CData(text) => text.clone(),
                _ => unreachable!(),
            })
            .collect::<Vec<_>>();
        if source_segments.concat() != old_text {
            return Err(PreservationError::ManagedTextMismatch);
        }
        if expected_runs.is_some_and(|runs| runs != source_segments.as_slice()) {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let updated = source_segments
            .iter()
            .map(|text| text.to_uppercase())
            .collect::<Vec<_>>();
        if updated.concat() != expected_text {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        for (index, text) in indexes.into_iter().zip(updated) {
            match &mut element.children[index] {
                PreservedNode::Text(value) | PreservedNode::CData(value) => *value = text,
                _ => unreachable!(),
            }
        }
        Ok(())
    }

    /// Updates only a recognized paragraph Type mapping. All other source
    /// attributes and nested attachments remain byte/structure-owned by source.
    pub(crate) fn change_managed_role(
        &mut self,
        id: &StableId,
        old_role: &str,
        new_role: &str,
    ) -> Result<(), PreservationError> {
        let element = self.managed_element_mut(id)?;
        if element.source_type.as_deref() != Some(old_role) {
            return Err(PreservationError::UnsupportedRoleMapping);
        }
        if old_role != new_role
            && (old_role == "Scene Heading" || new_role == "Scene Heading")
            && (contains_attached_data(&element.children) || contains_unknown_nested_data(element))
        {
            return Err(PreservationError::StructuralAttachmentConflict);
        }
        let mut type_index = None;
        for (index, attribute) in element.attributes.iter().enumerate() {
            if attribute.name.local_name == "Type" {
                if type_index.is_some() || attribute.value != old_role {
                    return Err(PreservationError::UnsupportedRoleMapping);
                }
                type_index = Some(index);
            }
        }
        let Some(type_index) = type_index else {
            return Err(PreservationError::UnsupportedRoleMapping);
        };
        element.attributes[type_index].value = new_role.to_owned();
        element.source_type = Some(new_role.to_owned());
        Ok(())
    }

    /// Reconciles a body-structure edit in the same candidate as its semantic
    /// model. Existing foreign paragraph IDs stay with surviving elements;
    /// newly inserted paragraphs receive fresh foreign and source-anchor IDs.
    /// Opaque nested content and plausible non-board references are safe-stops.
    pub(crate) fn reconcile_body_structure(
        &mut self,
        before: &[ScreenplayElement],
        after: &mut [ScreenplayElement],
    ) -> Result<(), PreservationError> {
        let before_ids = before
            .iter()
            .map(|element| element.id.clone())
            .collect::<BTreeSet<_>>();
        let after_ids = after
            .iter()
            .map(|element| element.id.clone())
            .collect::<BTreeSet<_>>();
        let structural_change = before_ids != after_ids
            || before.iter().map(|item| &item.id).collect::<Vec<_>>()
                != after.iter().map(|item| &item.id).collect::<Vec<_>>();
        if !structural_change {
            return Ok(());
        }

        for element in before {
            if !after_ids.contains(&element.id) && self.has_managed_element(&element.id) {
                self.remove_managed_paragraph(&element.id, element.source_provenance.as_ref())?;
            }
        }

        // A retained identity may have acquired the left or joined text of a
        // structural operation. Rebuild only simple, explicitly mapped Text
        // runs; ordinary in-paragraph edits continue to use the finer-grained
        // source-child transform above.
        for element in after.iter() {
            let Some(previous) = before.iter().find(|item| item.id == element.id) else {
                continue;
            };
            if previous.text != element.text && self.has_managed_element(&element.id) {
                self.replace_managed_projection(element)?;
            }
        }

        let ordered_ids = after
            .iter()
            .map(|element| element.id.clone())
            .collect::<Vec<_>>();
        for (index, element) in after.iter_mut().enumerate() {
            if before_ids.contains(&element.id) || self.has_managed_element(&element.id) {
                continue;
            }
            let provenance = self.insert_managed_paragraph(element, index, &ordered_ids)?;
            element.source_provenance = Some(provenance);
        }
        Ok(())
    }

    fn insert_managed_paragraph(
        &mut self,
        semantic: &ScreenplayElement,
        index: usize,
        ordered: &[StableId],
    ) -> Result<SourceProvenance, PreservationError> {
        let ManagedParagraphTemplate {
            paragraph_name,
            paragraph_bindings,
            text_name,
            text_bindings,
            id_spelling,
        } = {
            let children = content_children_mut(&mut self.sections)
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            let template = find_any_managed_paragraph(children)
                .ok_or(PreservationError::UnsupportedTextMapping)?;
            paragraph_template(template)?
        };
        let foreign_id = self.fresh_foreign_paragraph_id()?;
        let anchor_id = self.fresh_anchor_id()?;
        let text_runs = semantic_text_runs(semantic);
        let mut paragraph = PreservedElement {
            name: paragraph_name,
            attributes: vec![
                PreservedAttribute {
                    name: plain_source_name("Type"),
                    value: fdx_role_name(semantic.role).to_owned(),
                },
                PreservedAttribute {
                    name: plain_source_name(&id_spelling),
                    value: foreign_id.clone(),
                },
            ],
            namespace_bindings: paragraph_bindings,
            source_type: Some(fdx_role_name(semantic.role).to_owned()),
            ownership: SourceOwnership::Managed(semantic.id.clone()),
            dependency_confidence: DependencyConfidence::Unknown,
            anchor: Some(anchor_id.clone()),
            children: Vec::new(),
        };
        for run in text_runs {
            paragraph
                .children
                .push(PreservedNode::Element(PreservedElement {
                    name: text_name.clone(),
                    attributes: run
                        .attributes
                        .iter()
                        .map(|attribute| PreservedAttribute {
                            name: source_name_from_qname(&attribute.name),
                            value: attribute.value.clone(),
                        })
                        .collect(),
                    namespace_bindings: text_bindings.clone(),
                    source_type: None,
                    ownership: SourceOwnership::Opaque,
                    dependency_confidence: DependencyConfidence::Unknown,
                    anchor: None,
                    children: vec![PreservedNode::Text(run.text)],
                }));
        }

        let children = content_children_mut(&mut self.sections)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let mut insert_at = None;
        for following in ordered.iter().skip(index + 1) {
            if let Some(position) = managed_child_index(children, following) {
                insert_at = Some(position);
                break;
            }
        }
        if insert_at.is_none() {
            for preceding in ordered[..index].iter().rev() {
                if let Some(position) = managed_child_index(children, preceding) {
                    insert_at = Some(position + 1);
                    break;
                }
            }
        }
        let position = insert_at.unwrap_or(children.len());
        children.insert(position, PreservedNode::Element(paragraph));
        self.anchors.push(SourceAnchor {
            id: anchor_id,
            target: AnchorTarget::ProjectElement(semantic.id.clone()),
            relation: AnchorRelation::Represents,
        });
        self.identities.push(ForeignIdentity::new(
            ForeignIdDomain::Paragraph,
            foreign_id.clone(),
        )?);
        Ok(SourceProvenance {
            format: SourceFormat::Fdx,
            section: SourceSection::Content,
            ordinal: u32::try_from(index).map_err(|_| PreservationError::TooManyEvents)?,
            foreign_id: Some(foreign_id),
            foreign_role: Some(fdx_role_name(semantic.role).to_owned()),
        })
    }

    fn remove_managed_paragraph(
        &mut self,
        id: &StableId,
        provenance: Option<&SourceProvenance>,
    ) -> Result<(), PreservationError> {
        let target = find_managed_element(&self.sections, id)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        if !is_plain_managed_paragraph(target) {
            return Err(PreservationError::StructuralAttachmentConflict);
        }
        let foreign_id = paragraph_foreign_id(target);
        if provenance.and_then(|item| item.foreign_id.as_deref()) != foreign_id {
            return Err(PreservationError::ManagedTextMismatch);
        }
        if let Some(value) = foreign_id {
            let referenced = self
                .references
                .iter()
                .chain(&self.board_graph.references)
                .any(|reference| {
                    reference.target.domain == ForeignIdDomain::Paragraph
                        && reference.target.value == value
                });
            if referenced || has_potential_reference(&self.sections, value, id) {
                return Err(PreservationError::PotentialReferenceConflict);
            }
            self.identities.retain(|identity| {
                !(identity.domain == ForeignIdDomain::Paragraph && identity.value == value)
            });
        }
        let removed = remove_managed_node_from_content(&mut self.sections, id);
        if !removed {
            return Err(PreservationError::ManagedAnchorMissing);
        }
        self.anchors.retain(|anchor| {
            !matches!(&anchor.target, AnchorTarget::ProjectElement(target) if target == id)
        });
        Ok(())
    }

    fn replace_managed_projection(
        &mut self,
        semantic: &ScreenplayElement,
    ) -> Result<(), PreservationError> {
        let paragraph = self.managed_element_mut(&semantic.id)?;
        if !is_plain_managed_paragraph(paragraph) {
            return Err(PreservationError::StructuralAttachmentConflict);
        }
        let templates = paragraph
            .children
            .iter()
            .filter_map(|node| match node {
                PreservedNode::Element(element) if is_text_element(element, paragraph) => {
                    Some(element.clone())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let runs = semantic_text_runs(semantic);
        let mut rebuilt = Vec::with_capacity(paragraph.children.len().saturating_add(runs.len()));
        let mut inserted = false;
        let mut template_cursor = 0usize;
        for child in &paragraph.children {
            match child {
                PreservedNode::Element(element) if is_text_element(element, paragraph) => {
                    if !inserted {
                        for run in &runs {
                            let template = templates
                                .iter()
                                .find(|candidate| text_element_matches_run(candidate, run))
                                .or_else(|| templates.get(template_cursor))
                                .ok_or(PreservationError::UnsupportedTextMapping)?;
                            template_cursor = template_cursor.saturating_add(1);
                            rebuilt
                                .push(PreservedNode::Element(text_element_for_run(template, run)));
                        }
                        inserted = true;
                    }
                }
                PreservedNode::Text(text) | PreservedNode::CData(text)
                    if text.trim().is_empty() =>
                {
                    rebuilt.push(child.clone())
                }
                _ => return Err(PreservationError::UnsupportedTextMapping),
            }
        }
        if !inserted && !runs.is_empty() {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        paragraph.children = rebuilt;
        Ok(())
    }

    fn fresh_foreign_paragraph_id(&self) -> Result<String, PreservationError> {
        let mut serial = 1u64;
        loop {
            let value = format!("scriptcraft-paragraph-{serial}");
            let collision = self
                .identities
                .iter()
                .any(|identity| identity.value == value)
                || self
                    .board_graph
                    .entities
                    .iter()
                    .any(|entity| entity.identity.value == value)
                || source_has_id_value(&self.sections, &value);
            if !collision {
                return Ok(value);
            }
            serial = serial
                .checked_add(1)
                .ok_or(PreservationError::TooManyEvents)?;
        }
    }

    fn fresh_anchor_id(&self) -> Result<SourceAnchorId, PreservationError> {
        let mut serial = 1u64;
        loop {
            let candidate = SourceAnchorId::new(format!("scriptcraft-anchor-{serial}"))?;
            if !self.anchors.iter().any(|anchor| anchor.id == candidate) {
                return Ok(candidate);
            }
            serial = serial
                .checked_add(1)
                .ok_or(PreservationError::TooManyEvents)?;
        }
    }

    fn managed_element_mut(
        &mut self,
        id: &StableId,
    ) -> Result<&mut PreservedElement, PreservationError> {
        let mut matches = Vec::new();
        for section in &mut self.sections {
            collect_managed_elements_mut(&mut section.children, id, &mut matches);
        }
        match matches.len() {
            0 => Err(PreservationError::ManagedAnchorMissing),
            1 => matches.pop().ok_or(PreservationError::ManagedAnchorMissing),
            _ => Err(PreservationError::ManagedAnchorAmbiguous),
        }
    }
}

fn plain_source_name(name: &str) -> SourceName {
    SourceName {
        namespace_uri: None,
        local_name: name.to_owned(),
        qualified_name: Some(name.to_owned()),
    }
}

fn source_name_from_qname(name: &str) -> SourceName {
    let local_name = name.rsplit(':').next().unwrap_or(name).to_owned();
    SourceName {
        namespace_uri: None,
        local_name,
        qualified_name: Some(name.to_owned()),
    }
}

fn fdx_role_name(role: Role) -> &'static str {
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

fn semantic_text_runs(element: &ScreenplayElement) -> Vec<TextRun> {
    if !element.runs.is_empty() {
        element.runs.clone()
    } else if element.text.is_empty() {
        Vec::new()
    } else {
        vec![TextRun {
            text: element.text.clone(),
            attributes: Vec::new(),
        }]
    }
}

fn content_children_mut(sections: &mut [PreservedSection]) -> Option<&mut Vec<PreservedNode>> {
    for section in sections {
        for node in &mut section.children {
            let PreservedNode::Element(root) = node else {
                continue;
            };
            if root.name.local_name != "FinalDraft" {
                continue;
            }
            let namespace = root.name.namespace_uri.clone();
            for child in &mut root.children {
                if let PreservedNode::Element(content) = child {
                    if content.name.local_name == "Content"
                        && content.name.namespace_uri == namespace
                    {
                        return Some(&mut content.children);
                    }
                }
            }
        }
    }
    None
}

fn find_any_managed_paragraph(nodes: &[PreservedNode]) -> Option<&PreservedElement> {
    for node in nodes {
        if let PreservedNode::Element(element) = node {
            if matches!(element.ownership, SourceOwnership::Managed(_)) {
                return Some(element);
            }
            if let Some(found) = find_any_managed_paragraph(&element.children) {
                return Some(found);
            }
        }
    }
    None
}

struct ManagedParagraphTemplate {
    paragraph_name: SourceName,
    paragraph_bindings: Vec<NamespaceBinding>,
    text_name: SourceName,
    text_bindings: Vec<NamespaceBinding>,
    id_spelling: String,
}

fn paragraph_template(
    paragraph: &PreservedElement,
) -> Result<ManagedParagraphTemplate, PreservationError> {
    if paragraph.name.local_name != "Paragraph" {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    let text = paragraph.children.iter().find_map(|node| match node {
        PreservedNode::Element(element) if is_text_element(element, paragraph) => Some(element),
        _ => None,
    });
    let (text_name, text_bindings) = if let Some(text) = text {
        (text.name.clone(), text.namespace_bindings.clone())
    } else {
        let prefix = paragraph
            .name
            .qualified_name
            .as_deref()
            .and_then(|name| name.split_once(':').map(|(prefix, _)| prefix));
        let qualified_name =
            prefix.map_or_else(|| "Text".to_owned(), |prefix| format!("{prefix}:Text"));
        (
            SourceName {
                namespace_uri: paragraph.name.namespace_uri.clone(),
                local_name: "Text".to_owned(),
                qualified_name: Some(qualified_name),
            },
            Vec::new(),
        )
    };
    let id_spelling = paragraph
        .attributes
        .iter()
        .find(|attribute| {
            attribute.name.namespace_uri.is_none()
                && attribute.name.local_name.eq_ignore_ascii_case("id")
        })
        .and_then(|attribute| attribute.name.qualified_name.clone())
        .unwrap_or_else(|| "ID".to_owned());
    Ok(ManagedParagraphTemplate {
        paragraph_name: paragraph.name.clone(),
        paragraph_bindings: paragraph.namespace_bindings.clone(),
        text_name,
        text_bindings,
        id_spelling,
    })
}

fn managed_child_index(nodes: &[PreservedNode], id: &StableId) -> Option<usize> {
    nodes.iter().position(|node| {
        matches!(node, PreservedNode::Element(element)
            if matches!(&element.ownership, SourceOwnership::Managed(target) if target == id))
    })
}

fn find_managed_element<'a>(
    sections: &'a [PreservedSection],
    id: &StableId,
) -> Option<&'a PreservedElement> {
    fn find<'a>(nodes: &'a [PreservedNode], id: &StableId) -> Option<&'a PreservedElement> {
        for node in nodes {
            if let PreservedNode::Element(element) = node {
                if matches!(&element.ownership, SourceOwnership::Managed(target) if target == id) {
                    return Some(element);
                }
                if let Some(found) = find(&element.children, id) {
                    return Some(found);
                }
            }
        }
        None
    }
    sections
        .iter()
        .find_map(|section| find(&section.children, id))
}

fn is_text_element(element: &PreservedElement, paragraph: &PreservedElement) -> bool {
    element.name.local_name == "Text" && element.name.namespace_uri == paragraph.name.namespace_uri
}

fn is_plain_managed_paragraph(paragraph: &PreservedElement) -> bool {
    paragraph.name.local_name == "Paragraph"
        && paragraph.children.iter().all(|node| match node {
            PreservedNode::Text(text) | PreservedNode::CData(text) => text.trim().is_empty(),
            PreservedNode::Element(element) if is_text_element(element, paragraph) => element
                .children
                .iter()
                .all(|child| matches!(child, PreservedNode::Text(_) | PreservedNode::CData(_))),
            _ => false,
        })
}

fn paragraph_foreign_id(paragraph: &PreservedElement) -> Option<&str> {
    let mut ids = paragraph.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none()
            && attribute.name.local_name.eq_ignore_ascii_case("id")
    });
    let id = ids.next()?.value.as_str();
    if ids.next().is_some() {
        None
    } else {
        Some(id)
    }
}

fn text_element_matches_run(element: &PreservedElement, run: &TextRun) -> bool {
    element.attributes.len() == run.attributes.len()
        && element
            .attributes
            .iter()
            .zip(&run.attributes)
            .all(|(source, model)| {
                source
                    .name
                    .qualified_name
                    .as_deref()
                    .unwrap_or(source.name.local_name.as_str())
                    == model.name
                    && source.value == model.value
            })
}

fn text_element_for_run(template: &PreservedElement, run: &TextRun) -> PreservedElement {
    let mut element = template.clone();
    element.attributes = run
        .attributes
        .iter()
        .map(|attribute| PreservedAttribute {
            name: source_name_from_qname(&attribute.name),
            value: attribute.value.clone(),
        })
        .collect();
    element.children = vec![PreservedNode::Text(run.text.clone())];
    element
}

fn remove_managed_node_from_content(sections: &mut [PreservedSection], id: &StableId) -> bool {
    let Some(children) = content_children_mut(sections) else {
        return false;
    };
    let Some(index) = managed_child_index(children, id) else {
        return false;
    };
    children.remove(index);
    true
}

fn source_has_id_value(sections: &[PreservedSection], value: &str) -> bool {
    fn contains(nodes: &[PreservedNode], value: &str) -> bool {
        nodes.iter().any(|node| match node {
            PreservedNode::Element(element) => {
                element.attributes.iter().any(|attribute| {
                    attribute.name.namespace_uri.is_none()
                        && attribute.name.local_name.eq_ignore_ascii_case("id")
                        && attribute.value == value
                }) || contains(&element.children, value)
            }
            _ => false,
        })
    }
    sections
        .iter()
        .any(|section| contains(&section.children, value))
}

fn has_potential_reference(
    sections: &[PreservedSection],
    value: &str,
    excluded_element: &StableId,
) -> bool {
    fn scan(nodes: &[PreservedNode], value: &str, excluded: &StableId) -> bool {
        nodes.iter().any(|node| {
            let PreservedNode::Element(element) = node else {
                return false;
            };
            if matches!(
                element.name.local_name.as_str(),
                "DisplayBoards" | "ListItems" | "Images" | "StoryMaps" | "Boards"
            ) {
                return false;
            }
            let is_excluded =
                matches!(&element.ownership, SourceOwnership::Managed(id) if id == excluded);
            element.attributes.iter().any(|attribute| {
                !(is_excluded
                    && attribute.name.namespace_uri.is_none()
                    && attribute.name.local_name.eq_ignore_ascii_case("id"))
                    && attribute.value == value
            }) || scan(&element.children, value, excluded)
        })
    }
    sections
        .iter()
        .any(|section| scan(&section.children, value, excluded_element))
}

fn contains_managed_element(nodes: &[PreservedNode], id: &StableId) -> bool {
    nodes.iter().any(|node| match node {
        PreservedNode::Element(element) => {
            matches!(&element.ownership, SourceOwnership::Managed(target) if target == id)
                || contains_managed_element(&element.children, id)
        }
        _ => false,
    })
}

fn contains_attached_data(nodes: &[PreservedNode]) -> bool {
    nodes.iter().any(|node| match node {
        PreservedNode::Element(element) => {
            matches!(element.ownership, SourceOwnership::Attached(_))
                || contains_attached_data(&element.children)
        }
        _ => false,
    })
}

fn contains_unknown_nested_data(paragraph: &PreservedElement) -> bool {
    paragraph.children.iter().any(|node| match node {
        PreservedNode::Element(element)
            if element.name.local_name == "Text"
                && element.name.namespace_uri == paragraph.name.namespace_uri =>
        {
            element
                .children
                .iter()
                .any(|child| !matches!(child, PreservedNode::Text(_) | PreservedNode::CData(_)))
        }
        PreservedNode::Text(text) | PreservedNode::CData(text) => !text.trim().is_empty(),
        _ => true,
    })
}

fn collect_managed_elements_mut<'a>(
    nodes: &'a mut [PreservedNode],
    id: &StableId,
    matches: &mut Vec<&'a mut PreservedElement>,
) {
    for node in nodes {
        if let PreservedNode::Element(element) = node {
            if matches!(&element.ownership, SourceOwnership::Managed(target) if target == id) {
                matches.push(element);
            } else {
                collect_managed_elements_mut(&mut element.children, id, matches);
            }
        }
    }
}

fn replace_text_element_children(
    paragraph: &mut PreservedElement,
    old_text: &str,
    start: usize,
    end: usize,
    inserted: &str,
    expected_runs: Option<&[String]>,
) -> Result<(), PreservationError> {
    if start > end
        || end > old_text.len()
        || !old_text.is_char_boundary(start)
        || !old_text.is_char_boundary(end)
    {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    let mut indexes = paragraph
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, node)| match node {
            PreservedNode::Element(element) if is_text_element(element, paragraph) => Some(index),
            _ => None,
        })
        .collect::<Vec<_>>();
    if indexes.is_empty() {
        if !old_text.is_empty() || start != 0 || end != 0 {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let (name, namespace_bindings) = paragraph_text_name(paragraph);
        paragraph
            .children
            .push(PreservedNode::Element(PreservedElement {
                name,
                attributes: Vec::new(),
                namespace_bindings,
                source_type: None,
                ownership: SourceOwnership::Opaque,
                dependency_confidence: DependencyConfidence::Unknown,
                anchor: None,
                children: vec![PreservedNode::Text(String::new())],
            }));
        indexes.push(paragraph.children.len().saturating_sub(1));
    }
    let source_segments = indexes
        .iter()
        .map(|index| {
            let PreservedNode::Element(text_element) = &paragraph.children[*index] else {
                return Err(PreservationError::UnsupportedTextMapping);
            };
            text_element
                .children
                .iter()
                .try_fold(String::new(), |mut value, child| match child {
                    PreservedNode::Text(text) | PreservedNode::CData(text) => {
                        value.push_str(text);
                        Ok(value)
                    }
                    _ => Err(PreservationError::UnsupportedTextMapping),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if source_segments.concat() != old_text {
        return Err(PreservationError::ManagedTextMismatch);
    }
    if expected_runs.is_some_and(|runs| runs != source_segments.as_slice()) {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    let mut boundary = 0usize;
    for pair in indexes.windows(2) {
        let previous = pair[0];
        let next = pair[1];
        boundary = boundary.saturating_add(
            source_segments[indexes
                .iter()
                .position(|index| *index == previous)
                .ok_or(PreservationError::UnsupportedTextMapping)?]
            .len(),
        );
        let has_opaque_gap = paragraph.children[previous + 1..next]
            .iter()
            .any(|node| match node {
                PreservedNode::Text(text) | PreservedNode::CData(text) => !text.trim().is_empty(),
                _ => true,
            });
        if has_opaque_gap && start < boundary && end > boundary {
            return Err(PreservationError::UnsupportedTextMapping);
        }
    }
    let updated = replace_segments(&source_segments, start, end, inserted)?;
    for (index, text) in indexes.into_iter().zip(updated) {
        let PreservedNode::Element(text_element) = &mut paragraph.children[index] else {
            return Err(PreservationError::UnsupportedTextMapping);
        };
        replace_text_element_content(text_element, &text)?;
    }
    Ok(())
}

fn paragraph_text_name(paragraph: &PreservedElement) -> (SourceName, Vec<NamespaceBinding>) {
    let prefix = paragraph
        .name
        .qualified_name
        .as_deref()
        .and_then(|name| name.split_once(':').map(|(prefix, _)| prefix));
    let qualified_name =
        prefix.map_or_else(|| "Text".to_owned(), |prefix| format!("{prefix}:Text"));
    (
        SourceName {
            namespace_uri: paragraph.name.namespace_uri.clone(),
            local_name: "Text".to_owned(),
            qualified_name: Some(qualified_name),
        },
        paragraph.namespace_bindings.clone(),
    )
}

fn replace_text_element_content(
    element: &mut PreservedElement,
    replacement: &str,
) -> Result<(), PreservationError> {
    if element
        .children
        .iter()
        .any(|child| !matches!(child, PreservedNode::Text(_) | PreservedNode::CData(_)))
    {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    if let Some(first) = element.children.first_mut() {
        match first {
            PreservedNode::Text(text) | PreservedNode::CData(text) => {
                *text = replacement.to_owned();
            }
            _ => return Err(PreservationError::UnsupportedTextMapping),
        }
        for child in element.children.iter_mut().skip(1) {
            match child {
                PreservedNode::Text(text) | PreservedNode::CData(text) => text.clear(),
                _ => return Err(PreservationError::UnsupportedTextMapping),
            }
        }
    } else {
        element
            .children
            .push(PreservedNode::Text(replacement.to_owned()));
    }
    Ok(())
}

fn direct_text_child_indexes(element: &PreservedElement) -> Vec<usize> {
    element
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            matches!(node, PreservedNode::Text(_) | PreservedNode::CData(_)).then_some(index)
        })
        .collect()
}

fn replace_segments(
    segments: &[String],
    start: usize,
    end: usize,
    inserted: &str,
) -> Result<Vec<String>, PreservationError> {
    let total = segments.iter().map(String::len).sum::<usize>();
    if start > end || end > total {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    let mut base = 0usize;
    let mut insertion_segment = None;
    for (index, segment) in segments.iter().enumerate() {
        let limit = base.saturating_add(segment.len());
        if start < limit || (start == total && limit == total) {
            insertion_segment = Some(index);
            break;
        }
        base = limit;
    }
    let insertion_segment = insertion_segment
        .or_else(|| segments.len().checked_sub(1))
        .ok_or(PreservationError::UnsupportedTextMapping)?;

    let mut base = 0usize;
    let mut updated = Vec::with_capacity(segments.len());
    for (index, segment) in segments.iter().enumerate() {
        let limit = base.saturating_add(segment.len());
        let before_len = start.saturating_sub(base).min(segment.len());
        let after_start = end.saturating_sub(base).min(segment.len());
        let mut value = segment[..before_len].to_owned();
        if index == insertion_segment {
            value.push_str(inserted);
        }
        if end < limit || end <= base {
            value.push_str(&segment[after_start..]);
        }
        updated.push(value);
        base = limit;
    }
    Ok(updated)
}

fn validate_name(name: &SourceName) -> Result<(), PreservationError> {
    if name.local_name.is_empty() {
        return Err(PreservationError::InvalidSourceName);
    }
    validate_field(&name.local_name)?;
    if let Some(uri) = &name.namespace_uri {
        validate_field(uri)?;
    }
    if let Some(qualified) = &name.qualified_name {
        validate_field(qualified)?;
    }
    Ok(())
}

fn validate_field(value: &str) -> Result<(), PreservationError> {
    if value.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES {
        return Err(PreservationError::FieldTooLarge);
    }
    Ok(())
}

fn validate_identity(identity: &ForeignIdentity) -> Result<(), PreservationError> {
    if let ForeignIdDomain::Other(domain) = &identity.domain {
        validate_field(domain)?;
    }
    if identity.value.is_empty() {
        return Err(PreservationError::InvalidSourceName);
    }
    validate_field(&identity.value)
}

fn is_board_domain(domain: &ForeignIdDomain) -> bool {
    matches!(
        domain,
        ForeignIdDomain::ListItem | ForeignIdDomain::Board | ForeignIdDomain::Image
    )
}

fn validate_ownership(ownership: &SourceOwnership) -> Result<(), PreservationError> {
    match ownership {
        SourceOwnership::Opaque | SourceOwnership::Managed(_) => Ok(()),
        SourceOwnership::Attached(identity) => validate_identity(identity),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Project, Role, ScreenplayElement};

    fn name(local_name: &str) -> SourceName {
        SourceName {
            namespace_uri: None,
            local_name: local_name.into(),
            qualified_name: None,
        }
    }

    fn snapshot() -> SourceSnapshot {
        SourceSnapshot {
            format: SourceFormat::Fdx,
            original_bytes: b"<Script/>".to_vec(),
            encoding: SourceEncoding::Utf8,
        }
    }

    fn project_with_preservation(state: PreservationState) -> Project {
        let element = ScreenplayElement::new("element", Role::SceneHeading, "INT. ROOM")
            .unwrap_or_else(|_| unreachable!());
        Project {
            title_page: Vec::new(),
            elements: vec![element],
            preservation: Some(state),
            legacy_json_state: None,
        }
    }

    fn base_state() -> PreservationState {
        let anchor = SourceAnchorId::new("anchor-scene").unwrap_or_else(|_| unreachable!());
        let paragraph = PreservedElement {
            name: name("Paragraph"),
            attributes: vec![PreservedAttribute {
                name: name("Type"),
                value: "Scene Heading".into(),
            }],
            namespace_bindings: Vec::new(),
            source_type: Some("Scene Heading".into()),
            ownership: SourceOwnership::Managed(
                StableId::new("element").unwrap_or_else(|_| unreachable!()),
            ),
            dependency_confidence: DependencyConfidence::SomeReferencesMapped,
            anchor: Some(anchor.clone()),
            children: vec![
                PreservedNode::Text("INT. ".into()),
                PreservedNode::CData("ROOM".into()),
            ],
        };
        PreservationState {
            source: snapshot(),
            source_layout_settings: SourceLayoutSettings::default(),
            sections: vec![PreservedSection {
                name: name("Content"),
                kind: SectionKind::Content,
                ordinal: 0,
                children: vec![PreservedNode::Element(paragraph)],
            }],
            anchors: vec![SourceAnchor {
                id: anchor,
                target: AnchorTarget::ProjectElement(
                    StableId::new("element").unwrap_or_else(|_| unreachable!()),
                ),
                relation: AnchorRelation::Represents,
            }],
            identities: Vec::new(),
            references: Vec::new(),
            board_graph: BoardGraph::default(),
        }
    }

    #[test]
    fn ordered_source_tree_and_snapshot_clone_and_compare_with_project() {
        let state = base_state();
        assert!(state.validate().is_ok());
        let project = project_with_preservation(state);
        assert!(project.validate().is_ok());
        let cloned = project.clone();
        assert_eq!(project, cloned);
        let serialized = project.to_json();
        assert_eq!(serialized, Err(crate::ModelError::UnsupportedJsonState));
    }

    #[test]
    fn identity_domains_and_board_relationships_have_deterministic_outcomes() {
        let mut state = base_state();
        let list = ForeignIdentity::new(ForeignIdDomain::ListItem, "list-1")
            .unwrap_or_else(|_| unreachable!());
        let board = ForeignIdentity::new(ForeignIdDomain::Board, "board-1")
            .unwrap_or_else(|_| unreachable!());
        let image = ForeignIdentity::new(ForeignIdDomain::Image, "image-1")
            .unwrap_or_else(|_| unreachable!());
        state.board_graph.entities = vec![
            BoardEntity {
                identity: list.clone(),
                kind: BoardEntityKind::ListItem,
                source_order: 1,
                anchor: None,
                image_payload: None,
                declared_image_length: None,
                source_path: None,
            },
            BoardEntity {
                identity: board.clone(),
                kind: BoardEntityKind::DisplayBoard,
                source_order: 2,
                anchor: None,
                image_payload: None,
                declared_image_length: None,
                source_path: None,
            },
            BoardEntity {
                identity: image.clone(),
                kind: BoardEntityKind::Image,
                source_order: 3,
                anchor: None,
                image_payload: Some(vec![0, 1, 2]),
                declared_image_length: Some(3),
                source_path: Some("not-opened/local/path.png".into()),
            },
        ];
        state.board_graph.references.push(ForeignReference {
            source: board.clone(),
            target: list.clone(),
            policy: ReferencePolicy::RequiredInternal,
        });
        state.board_graph.references.push(ForeignReference {
            source: board.clone(),
            target: image,
            policy: ReferencePolicy::RequiredInternal,
        });
        let screenplay_id = ForeignIdentity::new(ForeignIdDomain::Paragraph, "paragraph-1")
            .unwrap_or_else(|_| unreachable!());
        state.identities.push(screenplay_id.clone());
        state.references.push(ForeignReference {
            source: screenplay_id,
            target: ForeignIdentity::new(ForeignIdDomain::Board, "future-board")
                .unwrap_or_else(|_| unreachable!()),
            policy: ReferencePolicy::CrossDomainMayOrphan,
        });
        assert!(state.validate().is_ok());

        state.board_graph.references[0].target =
            ForeignIdentity::new(ForeignIdDomain::ListItem, "missing")
                .unwrap_or_else(|_| unreachable!());
        assert_eq!(state.validate(), Err(PreservationError::DanglingReference));
    }

    #[test]
    fn opaque_types_title_order_and_nested_scene_attachments_remain_source_owned() {
        let mut state = base_state();
        let scene = ForeignIdentity::new(ForeignIdDomain::Scene, "scene-source-1")
            .unwrap_or_else(|_| unreachable!());
        let attachment_anchor =
            SourceAnchorId::new("anchor-scene-attachment").unwrap_or_else(|_| unreachable!());
        let content = match &mut state.sections[0].children[0] {
            PreservedNode::Element(paragraph) => &mut paragraph.children,
            _ => unreachable!(),
        };
        content.push(PreservedNode::Element(PreservedElement {
            name: name("SceneProperties"),
            attributes: Vec::new(),
            namespace_bindings: Vec::new(),
            source_type: None,
            ownership: SourceOwnership::Attached(scene.clone()),
            dependency_confidence: DependencyConfidence::Unknown,
            anchor: Some(attachment_anchor.clone()),
            children: vec![PreservedNode::Element(PreservedElement {
                name: SourceName {
                    namespace_uri: Some("urn:example:opaque".into()),
                    local_name: "FutureBeatData".into(),
                    qualified_name: Some("future:FutureBeatData".into()),
                },
                attributes: vec![
                    PreservedAttribute {
                        name: name("first"),
                        value: "one".into(),
                    },
                    PreservedAttribute {
                        name: name("second"),
                        value: "two".into(),
                    },
                ],
                namespace_bindings: vec![NamespaceBinding {
                    prefix: "future".into(),
                    namespace_uri: "urn:example:opaque".into(),
                }],
                source_type: Some("VendorBeat".into()),
                ownership: SourceOwnership::Opaque,
                dependency_confidence: DependencyConfidence::Unknown,
                anchor: None,
                children: vec![PreservedNode::Text("opaque authored text".into())],
            })],
        }));
        state.anchors.push(SourceAnchor {
            id: attachment_anchor,
            target: AnchorTarget::Foreign(scene.clone()),
            relation: AnchorRelation::Attached,
        });
        state.identities.push(scene);
        state.sections.push(PreservedSection {
            name: name("TitlePage"),
            kind: SectionKind::TitlePage,
            ordinal: 1,
            children: vec![
                PreservedNode::Text(String::new()),
                PreservedNode::Element(PreservedElement {
                    name: name("Paragraph"),
                    attributes: vec![PreservedAttribute {
                        name: name("Alignment"),
                        value: "Center".into(),
                    }],
                    namespace_bindings: Vec::new(),
                    source_type: None,
                    ownership: SourceOwnership::Opaque,
                    dependency_confidence: DependencyConfidence::Unknown,
                    anchor: None,
                    children: vec![PreservedNode::Text("A title".into())],
                }),
            ],
        });

        assert!(state.validate().is_ok());
        assert_eq!(state.sections[0].kind, SectionKind::Content);
        assert_eq!(state.sections[1].kind, SectionKind::TitlePage);
        let project = project_with_preservation(state);
        assert!(project.validate().is_ok());
        assert!(project.clone().preservation.is_some());
    }

    #[test]
    fn ambiguous_ids_anchors_and_unsafe_limits_are_typed_errors() {
        let mut duplicate = base_state();
        let id =
            ForeignIdentity::new(ForeignIdDomain::Board, "same").unwrap_or_else(|_| unreachable!());
        duplicate.identities = vec![id.clone(), id];
        assert_eq!(
            duplicate.validate(),
            Err(PreservationError::DuplicateIdentity)
        );

        let mut dangling_anchor = base_state();
        let missing_element = StableId::new("not-in-project").unwrap_or_else(|_| unreachable!());
        dangling_anchor.anchors[0].target = AnchorTarget::ProjectElement(missing_element.clone());
        if let PreservedNode::Element(paragraph) = &mut dangling_anchor.sections[0].children[0] {
            paragraph.ownership = SourceOwnership::Managed(missing_element);
        }
        let project = project_with_preservation(dangling_anchor);
        assert_eq!(
            project.validate(),
            Err(crate::ModelError::InvalidPreservationState(
                PreservationError::AnchorTargetMissing
            ))
        );

        let mut deep = base_state();
        let mut node = PreservedNode::Text("leaf".into());
        for _ in 0..MAX_SOURCE_DEPTH {
            node = PreservedNode::Element(PreservedElement {
                name: name("nested"),
                attributes: Vec::new(),
                namespace_bindings: Vec::new(),
                source_type: None,
                ownership: SourceOwnership::Opaque,
                dependency_confidence: DependencyConfidence::Unknown,
                anchor: None,
                children: vec![node],
            });
        }
        deep.sections[0].children = vec![node];
        assert_eq!(deep.validate(), Err(PreservationError::ExcessiveDepth));

        let mut oversized = base_state();
        oversized.source.original_bytes = vec![0; MAX_SOURCE_BYTES + 1];
        assert_eq!(oversized.validate(), Err(PreservationError::SourceTooLarge));
    }
}

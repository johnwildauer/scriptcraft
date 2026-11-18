use super::identity::{ForeignIdentity, SourceAnchorId};
use super::shared::{SharedBytes, SharedText, SharedVec};
use crate::{SourceFormat, StableId};

/// The source's expanded name and, when available, original qualified spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceName {
    pub namespace_uri: Option<String>,
    pub local_name: String,
    pub qualified_name: Option<String>,
}

/// Identifies the one supported paragraph-owned opaque FDX attachment by
/// expanded name. The owner namespace is authoritative; prefix spelling is not.
pub fn is_user_paragraph_data_name(name: &SourceName, owner: &SourceName) -> bool {
    name.local_name == "UserParagraphData" && name.namespace_uri == owner.namespace_uri
}

pub fn is_user_paragraph_data_lookalike(name: &SourceName) -> bool {
    name.local_name == "UserParagraphData"
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
    /// Outer dual-dialogue wrapper ownership; never enters paragraph indexes.
    DualDialogueWrapper(StableId),
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
    pub children: SharedVec<PreservedNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
// PreservedNode is the public mixed-content tree representation. Keeping the
// Element payload inline avoids API-wide boxing churn in preservation traversal.
#[allow(clippy::large_enum_variant)]
pub enum PreservedNode {
    Element(PreservedElement),
    Text(SharedText),
    CData(SharedText),
    Comment(SharedText),
    ProcessingInstruction { target: SharedText, data: SharedText },
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
    pub children: SharedVec<PreservedNode>,
}

/// Bytes are the untouched source snapshot, not proof that a future edited
/// serialization is byte-identical. The tree separately retains ordered source
/// structure for later reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSnapshot {
    pub format: SourceFormat,
    pub original_bytes: SharedBytes,
    pub encoding: SourceEncoding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceEncoding {
    Utf8,
}

pub const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_SOURCE_EVENTS: usize = 100_000;
pub const MAX_SOURCE_DEPTH: usize = 128;
pub const MAX_ATTRIBUTES_PER_ELEMENT: usize = 128;
pub const MAX_SOURCE_NAME_OR_VALUE_BYTES: usize = 64 * 1024;
pub const MAX_SOURCE_TEXT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_TOTAL_IMAGE_BYTES: usize = 8 * 1024 * 1024;

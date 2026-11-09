//! Bounded, source-owned FDX preservation state.
//!
//! The FDX codec retains the parsed source tree and original byte snapshot
//! separately. The ordered tree is the serialization authority for source-owned
//! content; byte identity and external-application interoperability are not
//! implied.

use crate::paragraph_id::{ParagraphIdAllocationError, ParagraphIdAllocator};
use crate::{
    Role, ScreenplayElement, SourceFormat, SourceLayoutSettings, SourceProvenance, SourceSection,
    StableId, TextRun, TitlePageParagraph,
};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Value-free counters for immutable preservation snapshot sharing.
///
/// These process-wide counters report wrapper clones and copy-on-write
/// detachments only. They never contain document-derived values.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CopyOnWriteDiagnostics {
    pub shared_vector_clones: u64,
    pub detached_vectors: u64,
    pub shared_text_payload_clones: u64,
    pub detached_text_payloads: u64,
    pub shared_source_byte_clones: u64,
    pub detached_source_byte_payloads: u64,
    pub shared_image_payload_clones: u64,
    pub detached_image_payloads: u64,
}

static SHARED_VECTOR_CLONES: AtomicU64 = AtomicU64::new(0);
static DETACHED_VECTORS: AtomicU64 = AtomicU64::new(0);
static SHARED_TEXT_PAYLOAD_CLONES: AtomicU64 = AtomicU64::new(0);
static DETACHED_TEXT_PAYLOADS: AtomicU64 = AtomicU64::new(0);
static SHARED_SOURCE_BYTE_CLONES: AtomicU64 = AtomicU64::new(0);
static DETACHED_SOURCE_BYTE_PAYLOADS: AtomicU64 = AtomicU64::new(0);
static SHARED_IMAGE_PAYLOAD_CLONES: AtomicU64 = AtomicU64::new(0);
static DETACHED_IMAGE_PAYLOADS: AtomicU64 = AtomicU64::new(0);

fn increment_counter(counter: &AtomicU64) {
    let _ = counter.try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
        Some(value.saturating_add(1))
    });
}

fn add_counter(counter: &AtomicU64, amount: u64) {
    if amount == 0 {
        return;
    }
    let _ = counter.try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
        Some(value.saturating_add(amount))
    });
}

/// Returns aggregate, value-free copy-on-write counters.
pub fn copy_on_write_diagnostics() -> CopyOnWriteDiagnostics {
    CopyOnWriteDiagnostics {
        shared_vector_clones: SHARED_VECTOR_CLONES.load(Ordering::Relaxed),
        detached_vectors: DETACHED_VECTORS.load(Ordering::Relaxed),
        shared_text_payload_clones: SHARED_TEXT_PAYLOAD_CLONES.load(Ordering::Relaxed),
        detached_text_payloads: DETACHED_TEXT_PAYLOADS.load(Ordering::Relaxed),
        shared_source_byte_clones: SHARED_SOURCE_BYTE_CLONES.load(Ordering::Relaxed),
        detached_source_byte_payloads: DETACHED_SOURCE_BYTE_PAYLOADS.load(Ordering::Relaxed),
        shared_image_payload_clones: SHARED_IMAGE_PAYLOAD_CLONES.load(Ordering::Relaxed),
        detached_image_payloads: DETACHED_IMAGE_PAYLOADS.load(Ordering::Relaxed),
    }
}

/// Resets copy-on-write counters without touching any document or snapshot.
pub fn reset_copy_on_write_diagnostics() {
    SHARED_VECTOR_CLONES.store(0, Ordering::Relaxed);
    DETACHED_VECTORS.store(0, Ordering::Relaxed);
    SHARED_TEXT_PAYLOAD_CLONES.store(0, Ordering::Relaxed);
    DETACHED_TEXT_PAYLOADS.store(0, Ordering::Relaxed);
    SHARED_SOURCE_BYTE_CLONES.store(0, Ordering::Relaxed);
    DETACHED_SOURCE_BYTE_PAYLOADS.store(0, Ordering::Relaxed);
    SHARED_IMAGE_PAYLOAD_CLONES.store(0, Ordering::Relaxed);
    DETACHED_IMAGE_PAYLOADS.store(0, Ordering::Relaxed);
}

/// Value-free structural counters for managed paragraph indexing.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ManagedParagraphIndexDiagnostics {
    pub rebuilds: u64,
    pub source_nodes_indexed: u64,
    pub lookups: u64,
    pub indexed_entries_examined: u64,
    pub fallback_tree_nodes_examined: u64,
}

static MANAGED_INDEX_REBUILDS: AtomicU64 = AtomicU64::new(0);
static MANAGED_INDEX_SOURCE_NODES: AtomicU64 = AtomicU64::new(0);
static MANAGED_INDEX_LOOKUPS: AtomicU64 = AtomicU64::new(0);
static MANAGED_INDEX_ENTRIES: AtomicU64 = AtomicU64::new(0);
static MANAGED_INDEX_FALLBACK_NODES: AtomicU64 = AtomicU64::new(0);

/// Returns process-wide managed-paragraph index work counts.
pub fn managed_paragraph_index_diagnostics() -> ManagedParagraphIndexDiagnostics {
    ManagedParagraphIndexDiagnostics {
        rebuilds: MANAGED_INDEX_REBUILDS.load(Ordering::Relaxed),
        source_nodes_indexed: MANAGED_INDEX_SOURCE_NODES.load(Ordering::Relaxed),
        lookups: MANAGED_INDEX_LOOKUPS.load(Ordering::Relaxed),
        indexed_entries_examined: MANAGED_INDEX_ENTRIES.load(Ordering::Relaxed),
        fallback_tree_nodes_examined: MANAGED_INDEX_FALLBACK_NODES.load(Ordering::Relaxed),
    }
}

/// Resets index diagnostics without changing preserved data or indexes.
pub fn reset_managed_paragraph_index_diagnostics() {
    MANAGED_INDEX_REBUILDS.store(0, Ordering::Relaxed);
    MANAGED_INDEX_SOURCE_NODES.store(0, Ordering::Relaxed);
    MANAGED_INDEX_LOOKUPS.store(0, Ordering::Relaxed);
    MANAGED_INDEX_ENTRIES.store(0, Ordering::Relaxed);
    MANAGED_INDEX_FALLBACK_NODES.store(0, Ordering::Relaxed);
}

/// Value-free structural counters for the transient source identity-use index.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SourceIdentityUseIndexDiagnostics {
    pub rebuilds: u64,
    pub source_nodes_indexed: u64,
    pub identity_queries: u64,
    pub reserved_id_queries: u64,
}

static SOURCE_USE_INDEX_REBUILDS: AtomicU64 = AtomicU64::new(0);
static SOURCE_USE_INDEX_SOURCE_NODES: AtomicU64 = AtomicU64::new(0);
static SOURCE_USE_INDEX_IDENTITY_QUERIES: AtomicU64 = AtomicU64::new(0);
static SOURCE_USE_INDEX_RESERVED_QUERIES: AtomicU64 = AtomicU64::new(0);

/// Returns process-wide source identity-use work counts without document values.
pub fn source_identity_use_index_diagnostics() -> SourceIdentityUseIndexDiagnostics {
    SourceIdentityUseIndexDiagnostics {
        rebuilds: SOURCE_USE_INDEX_REBUILDS.load(Ordering::Relaxed),
        source_nodes_indexed: SOURCE_USE_INDEX_SOURCE_NODES.load(Ordering::Relaxed),
        identity_queries: SOURCE_USE_INDEX_IDENTITY_QUERIES.load(Ordering::Relaxed),
        reserved_id_queries: SOURCE_USE_INDEX_RESERVED_QUERIES.load(Ordering::Relaxed),
    }
}

/// Resets source identity-use work counts without changing preserved data.
pub fn reset_source_identity_use_index_diagnostics() {
    SOURCE_USE_INDEX_REBUILDS.store(0, Ordering::Relaxed);
    SOURCE_USE_INDEX_SOURCE_NODES.store(0, Ordering::Relaxed);
    SOURCE_USE_INDEX_IDENTITY_QUERIES.store(0, Ordering::Relaxed);
    SOURCE_USE_INDEX_RESERVED_QUERIES.store(0, Ordering::Relaxed);
}

/// A vector whose clones share immutable storage and detach on mutable access.
/// Its vector-like API keeps the preservation tree's ordered semantic surface.
#[derive(Debug)]
pub struct SharedVec<T>(Arc<Vec<T>>);

impl<T> SharedVec<T> {
    pub fn new(values: Vec<T>) -> Self {
        Self(Arc::new(values))
    }

    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Clone for SharedVec<T> {
    fn clone(&self) -> Self {
        increment_counter(&SHARED_VECTOR_CLONES);
        Self(Arc::clone(&self.0))
    }
}

impl<T> Default for SharedVec<T> {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl<T> From<Vec<T>> for SharedVec<T> {
    fn from(values: Vec<T>) -> Self {
        Self::new(values)
    }
}

impl<T> Deref for SharedVec<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Clone> DerefMut for SharedVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if Arc::strong_count(&self.0) > 1 {
            increment_counter(&DETACHED_VECTORS);
        }
        Arc::make_mut(&mut self.0)
    }
}

impl<T: PartialEq> PartialEq for SharedVec<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || **self == **other
    }
}

impl<T: Eq> Eq for SharedVec<T> {}

impl<T> AsRef<[T]> for SharedVec<T> {
    fn as_ref(&self) -> &[T] {
        self.0.as_slice()
    }
}

impl<T> FromIterator<T> for SharedVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::new(iter.into_iter().collect())
    }
}

impl<T: Clone> IntoIterator for SharedVec<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        match Arc::try_unwrap(self.0) {
            Ok(values) => values.into_iter(),
            Err(values) => values.as_ref().clone().into_iter(),
        }
    }
}

impl<'a, T> IntoIterator for &'a SharedVec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: Clone> IntoIterator for &'a mut SharedVec<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// An immutable UTF-8 payload shared by preserved text, CDATA, and comments.
#[derive(Debug)]
pub struct SharedText(Arc<String>);

impl SharedText {
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Clone for SharedText {
    fn clone(&self) -> Self {
        increment_counter(&SHARED_TEXT_PAYLOAD_CLONES);
        Self(Arc::clone(&self.0))
    }
}

impl From<String> for SharedText {
    fn from(value: String) -> Self {
        Self(Arc::new(value))
    }
}

impl From<&str> for SharedText {
    fn from(value: &str) -> Self {
        Self(Arc::new(value.to_owned()))
    }
}

impl Deref for SharedText {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SharedText {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if Arc::strong_count(&self.0) > 1 {
            increment_counter(&DETACHED_TEXT_PAYLOADS);
        }
        Arc::make_mut(&mut self.0)
    }
}

impl PartialEq for SharedText {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || **self == **other
    }
}

impl Eq for SharedText {}

impl PartialEq<String> for SharedText {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<str> for SharedText {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for SharedText {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<SharedText> for String {
    fn eq(&self, other: &SharedText) -> bool {
        self.as_str() == other.as_str()
    }
}

impl From<SharedText> for String {
    fn from(value: SharedText) -> Self {
        Arc::try_unwrap(value.0).unwrap_or_else(|shared| shared.as_ref().clone())
    }
}

/// Immutable byte storage used for original source and opaque image payloads.
#[derive(Debug)]
pub struct SharedBytes {
    bytes: Arc<Vec<u8>>,
    image_payload: bool,
}

impl SharedBytes {
    pub fn source(bytes: Vec<u8>) -> Self {
        Self {
            bytes: Arc::new(bytes),
            image_payload: false,
        }
    }

    pub fn image(bytes: Vec<u8>) -> Self {
        Self {
            bytes: Arc::new(bytes),
            image_payload: true,
        }
    }

    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.bytes, &other.bytes)
    }
}

impl Default for SharedBytes {
    fn default() -> Self {
        Self::source(Vec::new())
    }
}

impl Clone for SharedBytes {
    fn clone(&self) -> Self {
        if self.image_payload {
            increment_counter(&SHARED_IMAGE_PAYLOAD_CLONES);
        } else {
            increment_counter(&SHARED_SOURCE_BYTE_CLONES);
        }
        Self {
            bytes: Arc::clone(&self.bytes),
            image_payload: self.image_payload,
        }
    }
}

impl From<Vec<u8>> for SharedBytes {
    fn from(bytes: Vec<u8>) -> Self {
        Self::source(bytes)
    }
}

impl Deref for SharedBytes {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl DerefMut for SharedBytes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if Arc::strong_count(&self.bytes) > 1 {
            if self.image_payload {
                increment_counter(&DETACHED_IMAGE_PAYLOADS);
            } else {
                increment_counter(&DETACHED_SOURCE_BYTE_PAYLOADS);
            }
        }
        Arc::make_mut(&mut self.bytes)
    }
}

impl PartialEq for SharedBytes {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.bytes, &other.bytes) || **self == **other
    }
}

impl Eq for SharedBytes {}

impl PartialEq<Vec<u8>> for SharedBytes {
    fn eq(&self, other: &Vec<u8>) -> bool {
        **self == *other
    }
}

impl<const N: usize> PartialEq<&[u8; N]> for SharedBytes {
    fn eq(&self, other: &&[u8; N]) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<const N: usize> PartialEq<[u8; N]> for SharedBytes {
    fn eq(&self, other: &[u8; N]) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl PartialEq<SharedBytes> for Vec<u8> {
    fn eq(&self, other: &SharedBytes) -> bool {
        self.as_slice() == other.as_slice()
    }
}

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
    /// Value-free call-site tag for managed projection mismatches.
    ManagedTextMismatchAt(ManagedTextMismatchSite),
    UnsupportedTextMapping,
    UnsupportedRunTopology,
    AmbiguousDirectAttribute,
    UnsupportedRoleMapping,
    StructuralAttachmentConflict,
    PotentialReferenceConflict,
    ParagraphIdentityEntropyUnavailable,
    ParagraphIdentityCollisionLimit,
}

/// Stable, bounded call-site categories for managed-text synchronization failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedTextMismatchSite {
    ScopedEditValidation,
    TextReplacement,
    TextElementsReplacement,
    ParagraphRemoval,
}

impl ManagedTextMismatchSite {
    pub const fn diagnostic_name(self) -> &'static str {
        match self {
            Self::ScopedEditValidation => "scoped-edit-validation",
            Self::TextReplacement => "text-replacement",
            Self::TextElementsReplacement => "text-elements-replacement",
            Self::ParagraphRemoval => "paragraph-removal",
        }
    }
}

fn categorize_managed_text_mismatch(
    error: PreservationError,
    site: ManagedTextMismatchSite,
) -> PreservationError {
    match error {
        PreservationError::ManagedTextMismatch => PreservationError::ManagedTextMismatchAt(site),
        other => other,
    }
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
    ProcessingInstruction {
        target: SharedText,
        data: SharedText,
    },
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
    DualDialogueWrapper(StableId),
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
    pub image_payload: Option<SharedBytes>,
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

#[derive(Debug, Clone)]
pub struct PreservationState {
    pub source: SourceSnapshot,
    /// Ephemeral projection of recognized source layout settings for runtime
    /// behavior. The complete source values in `sections` and
    /// `source.original_bytes` remain authoritative for serialization.
    pub(crate) source_layout_settings: SourceLayoutSettings,
    pub sections: SharedVec<PreservedSection>,
    pub anchors: Vec<SourceAnchor>,
    pub identities: Vec<ForeignIdentity>,
    pub references: Vec<ForeignReference>,
    pub board_graph: BoardGraph,
    pub(crate) managed_paragraph_index: Option<BTreeMap<StableId, Vec<ManagedParagraphLocation>>>,
    pub(crate) source_identity_use_index: Option<SourceIdentityUseIndex>,
    pub(crate) source_text_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedParagraphLocation {
    section_index: usize,
    node_path: Vec<usize>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SourceIdentityUseIndex {
    paragraph_identity_counts: BTreeMap<String, usize>,
    paragraph_source_identity_counts: BTreeMap<String, usize>,
    referenced_paragraph_ids: BTreeSet<String>,
    anchored_paragraph_ids: BTreeSet<String>,
    externally_used_paragraph_ids: BTreeSet<String>,
    source_id_values: BTreeSet<String>,
}

#[derive(Debug, Default)]
struct SubstringAutomatonNode {
    transitions: BTreeMap<u8, usize>,
    failure: usize,
    output_link: Option<usize>,
    outputs: Vec<usize>,
}

#[derive(Debug, Default)]
struct SubstringAutomaton {
    patterns: Vec<String>,
    nodes: Vec<SubstringAutomatonNode>,
}

impl SubstringAutomaton {
    fn new(patterns: Vec<String>) -> Self {
        let mut automaton = Self {
            patterns,
            nodes: vec![SubstringAutomatonNode::default()],
        };
        for (pattern_index, pattern) in automaton.patterns.iter().enumerate() {
            let mut node_index = 0;
            for byte in pattern.bytes() {
                let next = if let Some(next) = automaton.nodes[node_index].transitions.get(&byte) {
                    *next
                } else {
                    let next = automaton.nodes.len();
                    automaton.nodes.push(SubstringAutomatonNode::default());
                    automaton.nodes[node_index].transitions.insert(byte, next);
                    next
                };
                node_index = next;
            }
            automaton.nodes[node_index].outputs.push(pattern_index);
        }

        let mut pending = std::collections::VecDeque::new();
        let root_children = automaton.nodes[0]
            .transitions
            .values()
            .copied()
            .collect::<Vec<_>>();
        pending.extend(root_children);
        while let Some(node_index) = pending.pop_front() {
            let edges = automaton.nodes[node_index]
                .transitions
                .iter()
                .map(|(byte, child)| (*byte, *child))
                .collect::<Vec<_>>();
            for (byte, child) in edges {
                let mut failure = automaton.nodes[node_index].failure;
                while failure != 0 && !automaton.nodes[failure].transitions.contains_key(&byte) {
                    failure = automaton.nodes[failure].failure;
                }
                let fallback = automaton.nodes[failure]
                    .transitions
                    .get(&byte)
                    .copied()
                    .unwrap_or(0);
                automaton.nodes[child].failure = fallback;
                automaton.nodes[child].output_link = if automaton.nodes[fallback].outputs.is_empty()
                {
                    automaton.nodes[fallback].output_link
                } else {
                    Some(fallback)
                };
                pending.push_back(child);
            }
        }
        automaton
    }

    fn scan(&self, text: &str, skip_exact: Option<&str>, hits: &mut BTreeSet<String>) {
        if self.patterns.is_empty() {
            return;
        }
        let mut state = 0;
        for byte in text.bytes() {
            while state != 0 && !self.nodes[state].transitions.contains_key(&byte) {
                state = self.nodes[state].failure;
            }
            state = self.nodes[state]
                .transitions
                .get(&byte)
                .copied()
                .unwrap_or(0);
            let mut output_state = Some(state);
            while let Some(output) = output_state {
                for pattern_index in &self.nodes[output].outputs {
                    let pattern = &self.patterns[*pattern_index];
                    if skip_exact != Some(pattern.as_str()) {
                        hits.insert(pattern.clone());
                    }
                }
                output_state = self.nodes[output].output_link;
            }
        }
    }
}

impl SourceIdentityUseIndex {
    fn build(
        sections: &[PreservedSection],
        identities: &[ForeignIdentity],
        references: &[ForeignReference],
        anchors: &[SourceAnchor],
        board_references: &[ForeignReference],
        managed_index: &mut BTreeMap<StableId, Vec<ManagedParagraphLocation>>,
        text_bytes: &mut usize,
    ) -> (Self, u64) {
        let mut index = Self::default();
        for identity in identities {
            if identity.domain == ForeignIdDomain::Paragraph {
                *index
                    .paragraph_identity_counts
                    .entry(identity.value.clone())
                    .or_default() += 1;
            }
        }
        for reference in references.iter().chain(board_references) {
            if reference.source.domain == ForeignIdDomain::Paragraph {
                index
                    .referenced_paragraph_ids
                    .insert(reference.source.value.clone());
            }
            if reference.target.domain == ForeignIdDomain::Paragraph {
                index
                    .referenced_paragraph_ids
                    .insert(reference.target.value.clone());
            }
        }
        for anchor in anchors {
            if let AnchorTarget::Foreign(identity) = &anchor.target {
                if identity.domain == ForeignIdDomain::Paragraph {
                    index.anchored_paragraph_ids.insert(identity.value.clone());
                }
            }
        }

        let patterns = index
            .paragraph_identity_counts
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let automaton = SubstringAutomaton::new(patterns);
        let mut visited = 0u64;
        // The recursive collector threads bounded counters and path state down
        // the source tree; a context object would add indirection to this walk.
        #[allow(clippy::too_many_arguments)]
        fn collect(
            nodes: &[PreservedNode],
            section_index: usize,
            path: &mut Vec<usize>,
            parent: Option<&PreservedElement>,
            grandparent: Option<&PreservedElement>,
            automaton: &SubstringAutomaton,
            index: &mut SourceIdentityUseIndex,
            managed_index: &mut BTreeMap<StableId, Vec<ManagedParagraphLocation>>,
            visited: &mut u64,
            text_bytes: &mut usize,
        ) {
            for (node_index, node) in nodes.iter().enumerate() {
                *visited = visited.saturating_add(1);
                match node {
                    PreservedNode::Element(element) => {
                        path.push(node_index);
                        let exact_alternate_copy = element.name.local_name == "Paragraph"
                            && parent.is_some_and(|parent| {
                                parent.name.local_name == "Alt"
                                    && parent.name.namespace_uri == element.name.namespace_uri
                            })
                            && grandparent.is_some_and(|grandparent| {
                                grandparent.name.local_name == "AltCollection"
                                    && grandparent.name.namespace_uri == element.name.namespace_uri
                            });
                        if let SourceOwnership::Managed(id) = &element.ownership {
                            managed_index.entry(id.clone()).or_default().push(
                                ManagedParagraphLocation {
                                    section_index,
                                    node_path: path.clone(),
                                },
                            );
                        }
                        let mut unqualified_ids = element.attributes.iter().filter(|attribute| {
                            attribute.name.namespace_uri.is_none()
                                && attribute.name.local_name.eq_ignore_ascii_case("id")
                        });
                        let unique_id = unqualified_ids
                            .next()
                            .map(|attribute| attribute.value.as_str())
                            .filter(|_| unqualified_ids.next().is_none());
                        if element.name.local_name == "Paragraph" && !exact_alternate_copy {
                            if let Some(value) = unique_id {
                                *index
                                    .paragraph_source_identity_counts
                                    .entry(value.to_owned())
                                    .or_default() += 1;
                            }
                        }
                        for attribute in &element.attributes {
                            if attribute.name.namespace_uri.is_none()
                                && attribute.name.local_name.eq_ignore_ascii_case("id")
                            {
                                index.source_id_values.insert(attribute.value.clone());
                            }
                            let exempt = ((exact_alternate_copy
                                || (element.name.local_name == "Paragraph"
                                    && matches!(&element.ownership, SourceOwnership::Managed(_))))
                                && attribute.name.namespace_uri.is_none()
                                && attribute.name.local_name.eq_ignore_ascii_case("id"))
                            .then_some(unique_id)
                            .flatten()
                            .filter(|value| *value == attribute.value.as_str());
                            automaton.scan(
                                &attribute.value,
                                exempt,
                                &mut index.externally_used_paragraph_ids,
                            );
                        }
                        collect(
                            &element.children,
                            section_index,
                            path,
                            Some(element),
                            parent,
                            automaton,
                            index,
                            managed_index,
                            visited,
                            text_bytes,
                        );
                        path.pop();
                    }
                    PreservedNode::Text(text)
                    | PreservedNode::CData(text)
                    | PreservedNode::Comment(text) => {
                        *text_bytes = text_bytes.saturating_add(text.len());
                        automaton.scan(text, None, &mut index.externally_used_paragraph_ids);
                    }
                    PreservedNode::ProcessingInstruction { target, data } => {
                        *text_bytes = text_bytes.saturating_add(data.len());
                        automaton.scan(target, None, &mut index.externally_used_paragraph_ids);
                        automaton.scan(data, None, &mut index.externally_used_paragraph_ids);
                    }
                }
            }
        }
        for (section_index, section) in sections.iter().enumerate() {
            collect(
                &section.children,
                section_index,
                &mut Vec::new(),
                None,
                None,
                &automaton,
                &mut index,
                managed_index,
                &mut visited,
                text_bytes,
            );
        }
        (index, visited)
    }

    fn paragraph_is_safely_removable(&self, id: &str) -> bool {
        self.paragraph_identity_counts.get(id) == Some(&1)
            && !self.referenced_paragraph_ids.contains(id)
            && !self.anchored_paragraph_ids.contains(id)
            && !self.externally_used_paragraph_ids.contains(id)
    }
}

impl PartialEq for PreservationState {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
            && self.source_layout_settings == other.source_layout_settings
            && self.sections == other.sections
            && self.anchors == other.anchors
            && self.identities == other.identities
            && self.references == other.references
            && self.board_graph == other.board_graph
    }
}

impl Eq for PreservationState {}

fn source_element(local_name: &str, namespace_uri: &Option<String>, prefix: Option<&str>) -> PreservedElement {
    let qualified_name = prefix.map_or_else(
        || local_name.to_owned(),
        |prefix| format!("{prefix}:{local_name}"),
    );
    PreservedElement {
        name: SourceName {
            namespace_uri: namespace_uri.clone(),
            local_name: local_name.to_owned(),
            qualified_name: Some(qualified_name),
        },
        attributes: Vec::new(),
        namespace_bindings: Vec::new(),
        source_type: None,
        ownership: SourceOwnership::Opaque,
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: None,
        children: SharedVec::default(),
    }
}

fn push_source_attribute(element: &mut PreservedElement, local_name: &str, value: &str) {
    element.attributes.push(PreservedAttribute {
        name: SourceName {
            namespace_uri: None,
            local_name: local_name.to_owned(),
            qualified_name: Some(local_name.to_owned()),
        },
        value: value.to_owned(),
    });
}

fn push_wrapper_overrides(
    element: &mut PreservedElement,
    overrides: crate::formatting::SourceParagraphOverrides,
) {
    if let Some(value) = overrides.paragraph.left_indent {
        push_source_attribute(element, "LeftIndent", &fixed_inches_string(value));
    }
    if let Some(value) = overrides.paragraph.right_indent {
        push_source_attribute(element, "RightIndent", &fixed_inches_string(value));
    }
    if let Some(value) = overrides.paragraph.space_before {
        push_source_attribute(element, "SpaceBefore", &value.decimal_string());
    }
    if let Some(value) = overrides.starts_new_page {
        push_source_attribute(element, "StartsNewPage", if value { "Yes" } else { "No" });
    }
}

fn fixed_inches_string(value: crate::FixedPoint) -> String {
    let raw = value.raw();
    const PER_INCH: u64 = 7_200;
    let magnitude = raw.unsigned_abs();
    let whole = magnitude / PER_INCH;
    let remainder = magnitude % PER_INCH;
    if remainder == 0 {
        return format!("{}{whole}", if raw < 0 { "-" } else { "" });
    }
    let mut fraction = format!("{remainder:06}");
    while fraction.ends_with('0') {
        fraction.pop();
    }
    format!("{}{whole}.{fraction}", if raw < 0 { "-" } else { "" })
}

impl PreservationState {
    pub fn source_layout_settings(&self) -> &SourceLayoutSettings {
        &self.source_layout_settings
    }

    /// Updates exactly one unqualified managed Paragraph attribute while
    /// retaining its original ordered slot and every unrelated source value.
    pub(crate) fn set_managed_paragraph_attribute(
        &mut self,
        id: &StableId,
        name: &str,
        value: Option<&str>,
    ) -> Result<(), PreservationError> {
        let paragraph = self.managed_element_mut_indexed(id)?;
        if paragraph.name.local_name != "Paragraph" {
            return Err(PreservationError::ManagedAnchorMissing);
        }
        let matching = paragraph
            .attributes
            .iter()
            .enumerate()
            .filter(|(_, attribute)| {
                attribute.name.namespace_uri.is_none() && attribute.name.local_name == name
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if matching.len() > 1 {
            return Err(PreservationError::AmbiguousDirectAttribute);
        }
        match (matching.first().copied(), value) {
            (Some(index), Some(value)) => paragraph.attributes[index].value = value.to_owned(),
            (Some(index), None) => {
                paragraph.attributes.remove(index);
            }
            (None, Some(value)) => paragraph.attributes.push(PreservedAttribute {
                name: SourceName {
                    namespace_uri: None,
                    local_name: name.to_owned(),
                    qualified_name: Some(name.to_owned()),
                },
                value: value.to_owned(),
            }),
            (None, None) => {}
        }
        self.rebuild_managed_paragraph_index();
        Ok(())
    }

    /// Body run editing has no semantic slot for an opaque child between
    /// source Text elements. Reject that shape instead of moving or merging
    /// styled text across the preserved child.
    pub(crate) fn ensure_run_range_is_contiguous(
        &self,
        id: &StableId,
        range: std::ops::Range<usize>,
    ) -> Result<(), PreservationError> {
        let paragraph = self
            .managed_element(id)?
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let mut text_indexes = Vec::new();
        for (index, child) in paragraph.children.iter().enumerate() {
            if let PreservedNode::Element(element) = child {
                if is_text_element(element, paragraph) {
                    if element.children.iter().any(|content| {
                        !matches!(content, PreservedNode::Text(_) | PreservedNode::CData(_))
                    }) {
                        return Err(PreservationError::UnsupportedRunTopology);
                    }
                    text_indexes.push(index);
                }
            }
        }
        let mut boundary = 0usize;
        for pair in text_indexes.windows(2) {
            let PreservedNode::Element(previous) = &paragraph.children[pair[0]] else {
                return Err(PreservationError::UnsupportedRunTopology);
            };
            boundary = boundary.saturating_add(previous.children.iter().filter_map(|node| match node {
                PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.len()),
                _ => None,
            }).sum::<usize>());
            let gap = &paragraph.children[pair[0] + 1..pair[1]];
            let has_unsupported_barrier = gap.iter().any(|node| match node {
                PreservedNode::Element(element) => !(element.name.local_name == "Alts"
                    && element.name.namespace_uri == paragraph.name.namespace_uri),
                PreservedNode::Text(text) | PreservedNode::CData(text) => !text.trim().is_empty(),
                _ => true,
            });
            if has_unsupported_barrier || (range.start < boundary && range.end > boundary
                && !gap.is_empty()) {
                return Err(PreservationError::UnsupportedRunTopology);
            }
        }
        Ok(())
    }

    /// Replaces the ordered source Text-node projection with an exact semantic
    /// run projection while retaining each node's expanded names, namespace
    /// bindings, and all run attributes. A non-Text child or source-text
    /// mismatch is a safe-stop; callers stage this against a detached project.
    pub(crate) fn sync_managed_text_runs(
        &mut self,
        id: &StableId,
        expected_text: &str,
        runs: &[TextRun],
    ) -> Result<(), PreservationError> {
        let paragraph = self.managed_element_mut(id)?;
        if expected_text.is_empty() && runs.is_empty() {
            let mut indexes = BTreeSet::new();
            for (index, node) in paragraph.children.iter().enumerate() {
                if let PreservedNode::Element(element) = node {
                    if is_text_element(element, paragraph) {
                        if element.children.iter().any(|child| {
                            !matches!(child, PreservedNode::Text(_) | PreservedNode::CData(_))
                        }) {
                            return Err(PreservationError::UnsupportedRunTopology);
                        }
                        indexes.insert(index);
                    }
                }
            }
            paragraph.children = paragraph.children.drain(..).enumerate()
                .filter_map(|(index, child)| (!indexes.contains(&index)).then_some(child)).collect();
            return Ok(());
        }
        let text_elements = paragraph.children.iter().enumerate().filter_map(|(index, node)| {
            match node {
                PreservedNode::Element(element) if is_text_element(element, paragraph) => Some((index, element)),
                _ => None,
            }
        }).collect::<Vec<_>>();
        let source_text = text_elements.iter().flat_map(|(_, element)| element.children.iter()).filter_map(|node| {
            match node {
                PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
                _ => None,
            }
        }).collect::<String>();
        if source_text != expected_text {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        if text_elements.iter().any(|(_, element)| element.children.iter().any(|node| {
            !matches!(node, PreservedNode::Text(_) | PreservedNode::CData(_))
        })) {
            return Err(PreservationError::UnsupportedRunTopology);
        }
        if text_elements.is_empty() {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let effective = if runs.is_empty() {
            vec![TextRun { text: expected_text.to_owned(), attributes: Vec::new() }]
        } else {
            runs.to_vec()
        };
        if effective.iter().map(|run| run.text.as_str()).collect::<String>() != expected_text {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let mut replacements = BTreeMap::<usize, Vec<PreservedNode>>::new();
        let mut text_offset = 0usize;
        for (index, template) in &text_elements {
            let segment_text = template.children.iter().filter_map(|node| match node {
                PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
                _ => None,
            }).collect::<String>();
            let end = text_offset.saturating_add(segment_text.len());
            let mut segment_runs = slice_text_runs(&effective, text_offset..end)?;
            if segment_runs.is_empty() && !segment_text.is_empty() {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            let mut replacement = Vec::with_capacity(segment_runs.len());
            for run in segment_runs.drain(..) {
                replacement.push(PreservedNode::Element(sync_text_element_attributes(template, &run)?));
            }
            replacements.insert(*index, replacement);
            text_offset = end;
        }
        if text_offset != expected_text.len() {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let mut rebuilt = Vec::with_capacity(paragraph.children.len().saturating_add(effective.len()));
        for (index, child) in paragraph.children.iter().enumerate() {
            if let Some(replacement) = replacements.remove(&index) {
                rebuilt.extend(replacement);
            } else {
                rebuilt.push(child.clone());
            }
        }
        paragraph.children = rebuilt.into();
        Ok(())
    }

    pub(crate) fn set_direct_paragraph_alignment(
        &mut self,
        id: &StableId,
        alignment: crate::ParagraphAlignment,
    ) {
        let overrides = self
            .source_layout_settings
            .direct_paragraph_overrides
            .entry(id.clone())
            .or_default();
        overrides.paragraph.alignment = Some(alignment);
    }

    /// Reconstructs all transient preservation indexes after direct edits to
    /// the publicly exposed source tree or identity/reference collections.
    /// Managed editing APIs maintain these indexes transactionally.
    pub fn rebuild_indexes(&mut self) {
        self.rebuild_managed_paragraph_index();
    }

    /// Rebuilds the transient source-tree location map from the authoritative
    /// ordered preservation tree. Call after any out-of-band structural update.
    pub(crate) fn rebuild_managed_paragraph_index(&mut self) {
        let mut managed_index = BTreeMap::new();
        let mut text_bytes = 0usize;
        let (source_index, visited) = SourceIdentityUseIndex::build(
            &self.sections,
            &self.identities,
            &self.references,
            &self.anchors,
            &self.board_graph.references,
            &mut managed_index,
            &mut text_bytes,
        );
        increment_counter(&MANAGED_INDEX_REBUILDS);
        add_counter(&MANAGED_INDEX_SOURCE_NODES, visited);
        self.managed_paragraph_index = Some(managed_index);
        self.source_text_bytes = text_bytes;
        increment_counter(&SOURCE_USE_INDEX_REBUILDS);
        add_counter(&SOURCE_USE_INDEX_SOURCE_NODES, visited);
        self.source_identity_use_index = Some(source_index);
    }

    /// Moves the exact existing managed body Paragraph nodes under a generated
    /// General/DualDialogue wrapper. This runs only on a detached transaction
    /// candidate; all indexes are rebuilt before the candidate can be published.
    pub(crate) fn wrap_dual_dialogue_members(
        &mut self,
        group_id: &StableId,
        member_ids: &[StableId],
        overrides: Option<crate::formatting::SourceParagraphOverrides>,
    ) -> Result<(String, SourceAnchorId, u32), PreservationError> {
        if self.source.format != SourceFormat::Fdx || member_ids.len() < 2 {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        self.ensure_managed_paragraph_index();
        self.ensure_source_identity_use_index();
        let mut locations = Vec::with_capacity(member_ids.len());
        for id in member_ids {
            let location = self
                .indexed_location(id)?
                .cloned()
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            if location.node_path.len() != 3 {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            locations.push(location);
        }
        let first = locations.first().ok_or(PreservationError::UnsupportedTextMapping)?;
        if locations.iter().any(|location| {
            location.section_index != first.section_index
                || location.node_path[..2] != first.node_path[..2]
        }) {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let root_index = first.node_path[0];
        let content_index = first.node_path[1];
        let child_indexes = locations
            .iter()
            .map(|location| location.node_path[2])
            .collect::<Vec<_>>();
        if child_indexes.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(PreservationError::ManagedAnchorMissing);
        }
        let section = self.sections.get(first.section_index)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let root = match section.children.get(root_index) {
            Some(PreservedNode::Element(root)) if root.name.local_name == "FinalDraft" => root,
            _ => return Err(PreservationError::UnsupportedTextMapping),
        };
        let content = match root.children.get(content_index) {
            Some(PreservedNode::Element(content))
                if content.name.local_name == "Content"
                    && content.name.namespace_uri == root.name.namespace_uri => content,
            _ => return Err(PreservationError::UnsupportedTextMapping),
        };
        for (id, child_index) in member_ids.iter().zip(&child_indexes) {
            if !matches!(content.children.get(*child_index), Some(PreservedNode::Element(paragraph))
                if paragraph.name.local_name == "Paragraph"
                    && matches!(&paragraph.ownership, SourceOwnership::Managed(owner) if owner == id))
            {
                return Err(PreservationError::ManagedAnchorMissing);
            }
        }
        for pair in child_indexes.windows(2) {
            if content.children[pair[0] + 1..pair[1]].iter().any(|node| {
                !matches!(node, PreservedNode::Text(text) if text.chars().all(char::is_whitespace))
            }) {
                return Err(PreservationError::UnsupportedTextMapping);
            }
        }
        let root_namespace = content.name.namespace_uri.clone();
        let prefix = content.name.qualified_name.as_deref()
            .and_then(|name| name.split_once(':').map(|(prefix, _)| prefix.to_owned()));
        let source_ordinal = u32::try_from(child_indexes[0])
            .map_err(|_| PreservationError::TooManyEvents)?;
        let foreign_id = self.fresh_foreign_paragraph_id()?;
        let anchor_id = self.fresh_anchor_id()?;
        let mut outer = source_element("Paragraph", &root_namespace, prefix.as_deref());
        outer.source_type = Some("General".to_owned());
        outer.ownership = SourceOwnership::DualDialogueWrapper(group_id.clone());
        outer.anchor = Some(anchor_id.clone());
        push_source_attribute(&mut outer, "Type", "General");
        push_source_attribute(&mut outer, "id", &foreign_id);
        if let Some(overrides) = overrides {
            push_wrapper_overrides(&mut outer, overrides);
        }
        let mut dual = source_element("DualDialogue", &root_namespace, prefix.as_deref());
        dual.ownership = SourceOwnership::Opaque;
        let mut spell = source_element("SpellCheckIgnoreLists", &root_namespace, prefix.as_deref());
        spell.ownership = SourceOwnership::Opaque;
        for name in ["IgnoredRanges", "IgnoredWords"] {
            let mut list = source_element(name, &root_namespace, prefix.as_deref());
            list.ownership = SourceOwnership::Opaque;
            spell.children.push(PreservedNode::Element(list));
        }
        dual.children.push(PreservedNode::Element(spell));

        let section = self.sections.get_mut(first.section_index)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let root = match section.children.get_mut(root_index) {
            Some(PreservedNode::Element(root)) => root,
            _ => return Err(PreservationError::ManagedAnchorMissing),
        };
        let content = match root.children.get_mut(content_index) {
            Some(PreservedNode::Element(content)) => content,
            _ => return Err(PreservationError::ManagedAnchorMissing),
        };
        let mut moved = Vec::with_capacity(child_indexes.len());
        for child_index in child_indexes.iter().rev() {
            moved.push(content.children.remove(*child_index));
        }
        moved.reverse();
        dual.children.extend(moved);
        outer.children.push(PreservedNode::Element(dual));
        content.children.insert(child_indexes[0], PreservedNode::Element(outer));
        self.identities.push(ForeignIdentity::new(
            ForeignIdDomain::Paragraph,
            foreign_id.clone(),
        )?);
        self.anchors.push(SourceAnchor {
            id: anchor_id.clone(),
            target: AnchorTarget::DualDialogueWrapper(group_id.clone()),
            relation: AnchorRelation::Represents,
        });
        self.invalidate_source_identity_use_index();
        self.rebuild_managed_paragraph_index();
        Ok((foreign_id, anchor_id, source_ordinal))
    }

    /// Removes only the resolved wrapper and its nested spell-check subtree,
    /// restoring the exact managed member Paragraph nodes at the wrapper slot.
    pub(crate) fn unwrap_dual_dialogue_members(
        &mut self,
        group_id: &StableId,
        member_ids: &[StableId],
        wrapper_anchor: &SourceAnchorId,
        foreign_id: Option<&str>,
    ) -> Result<(), PreservationError> {
        if self.source.format != SourceFormat::Fdx || member_ids.len() < 2 {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        self.ensure_managed_paragraph_index();
        self.ensure_source_identity_use_index();
        let mut wrapper_locations = Vec::new();
        for (section_index, section) in self.sections.iter().enumerate() {
            for (root_index, root_node) in section.children.iter().enumerate() {
                let PreservedNode::Element(root) = root_node else { continue };
                if root.name.local_name != "FinalDraft" { continue }
                for (content_index, content_node) in root.children.iter().enumerate() {
                    let PreservedNode::Element(content) = content_node else { continue };
                    if content.name.local_name != "Content" { continue }
                    for (outer_index, outer_node) in content.children.iter().enumerate() {
                        if matches!(outer_node, PreservedNode::Element(outer)
                            if matches!(&outer.ownership, SourceOwnership::DualDialogueWrapper(owner) if owner == group_id)
                                && outer.anchor.as_ref() == Some(wrapper_anchor))
                        {
                            wrapper_locations.push((section_index, root_index, content_index, outer_index));
                        }
                    }
                }
            }
        }
        let [(section_index, root_index, content_index, outer_index)] = wrapper_locations.as_slice()
            else { return Err(PreservationError::ManagedAnchorMissing) };
        if let Some(foreign_id) = foreign_id {
            if !self.paragraph_identity_safely_removable(foreign_id) {
                return Err(PreservationError::PotentialReferenceConflict);
            }
        }
        let section = self.sections.get(*section_index).ok_or(PreservationError::ManagedAnchorMissing)?;
        let PreservedNode::Element(root) = section.children.get(*root_index).ok_or(PreservationError::ManagedAnchorMissing)?
            else { return Err(PreservationError::ManagedAnchorMissing) };
        let PreservedNode::Element(content) = root.children.get(*content_index).ok_or(PreservationError::ManagedAnchorMissing)?
            else { return Err(PreservationError::ManagedAnchorMissing) };
        let PreservedNode::Element(outer) = content.children.get(*outer_index).ok_or(PreservationError::ManagedAnchorMissing)?
            else { return Err(PreservationError::ManagedAnchorMissing) };
        let duals = outer.children.iter().filter_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "DualDialogue" => Some(element),
            PreservedNode::Text(text) if text.chars().all(char::is_whitespace) => None,
            _ => None,
        }).collect::<Vec<_>>();
        if duals.len() != 1 || outer.children.iter().any(|node| match node {
            PreservedNode::Element(element) => element.name.local_name != "DualDialogue",
            PreservedNode::Text(text) => !text.chars().all(char::is_whitespace),
            _ => true,
        }) {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let dual = duals[0];
        let mut spell_count = 0usize;
        let mut found_members = Vec::with_capacity(member_ids.len());
        for node in dual.children.iter() {
            match node {
                PreservedNode::Element(element) if element.name.local_name == "SpellCheckIgnoreLists" => spell_count += 1,
                PreservedNode::Element(element) if element.name.local_name == "Paragraph" => {
                    if let SourceOwnership::Managed(id) = &element.ownership {
                        found_members.push(id.clone());
                    } else {
                        return Err(PreservationError::UnsupportedTextMapping);
                    }
                }
                PreservedNode::Text(text) if text.chars().all(char::is_whitespace) => {}
                _ => return Err(PreservationError::UnsupportedTextMapping),
            }
        }
        if spell_count != 1 || found_members != member_ids {
            return Err(PreservationError::ManagedAnchorMissing);
        }
        let member_nodes = dual.children.iter().filter_map(|node| match node {
            PreservedNode::Element(element) if matches!(&element.ownership, SourceOwnership::Managed(_)) => Some(node.clone()),
            _ => None,
        }).collect::<Vec<_>>();
        let section = self.sections.get_mut(*section_index).ok_or(PreservationError::ManagedAnchorMissing)?;
        let PreservedNode::Element(root) = section.children.get_mut(*root_index).ok_or(PreservationError::ManagedAnchorMissing)?
            else { return Err(PreservationError::ManagedAnchorMissing) };
        let PreservedNode::Element(content) = root.children.get_mut(*content_index).ok_or(PreservationError::ManagedAnchorMissing)?
            else { return Err(PreservationError::ManagedAnchorMissing) };
        content.children.remove(*outer_index);
        for (offset, member) in member_nodes.into_iter().enumerate() {
            content.children.insert(*outer_index + offset, member);
        }
        if let Some(foreign_id) = foreign_id {
            self.identities.retain(|identity| {
                !(identity.domain == ForeignIdDomain::Paragraph && identity.value == foreign_id)
            });
        }
        self.anchors.retain(|anchor| anchor.id != *wrapper_anchor);
        self.source_layout_settings.direct_paragraph_overrides.remove(group_id);
        self.invalidate_source_identity_use_index();
        self.rebuild_managed_paragraph_index();
        Ok(())
    }

    pub(crate) fn invalidate_source_identity_use_index(&mut self) {
        self.source_identity_use_index = None;
    }

    pub(crate) fn ensure_source_identity_use_index(&mut self) {
        if self.source_identity_use_index.is_none() {
            self.rebuild_managed_paragraph_index();
        }
    }

    pub(crate) fn has_source_identity_use_index(&self) -> bool {
        self.source_identity_use_index.is_some()
    }

    pub(crate) fn paragraph_identity_safely_removable(&self, id: &str) -> bool {
        increment_counter(&SOURCE_USE_INDEX_IDENTITY_QUERIES);
        self.source_identity_use_index
            .as_ref()
            .is_some_and(|index| index.paragraph_is_safely_removable(id))
    }

    pub(crate) fn paragraph_identity_has_unique_source_definition(&self, id: &str) -> bool {
        self.source_identity_use_index
            .as_ref()
            .is_some_and(|index| index.paragraph_source_identity_counts.get(id) == Some(&1))
    }

    pub(crate) fn indexed_source_id_values(&self) -> BTreeSet<String> {
        increment_counter(&SOURCE_USE_INDEX_RESERVED_QUERIES);
        self.source_identity_use_index
            .as_ref()
            .map(|index| index.source_id_values.clone())
            .unwrap_or_default()
    }

    pub(crate) fn ensure_managed_paragraph_index(&mut self) {
        if self.managed_paragraph_index.is_none() {
            self.rebuild_managed_paragraph_index();
        }
    }

    /// Looks up a managed element through the preservation-owned location index.
    /// A missing index retains the counted complete-search fallback for callers
    /// operating on externally constructed preservation states.
    pub(crate) fn managed_element(
        &self,
        id: &StableId,
    ) -> Result<Option<&PreservedElement>, PreservationError> {
        match self.indexed_location(id) {
            Ok(Some(location)) => {
                let element = self.managed_element_at(location);
                if element.is_some_and(|element| {
                    !matches!(&element.ownership, SourceOwnership::Managed(target) if target == id)
                }) {
                    return Err(PreservationError::ManagedAnchorMissing);
                }
                Ok(element)
            }
            Ok(None) if self.managed_paragraph_index.is_none() => {
                Ok(find_managed_element_counted(&self.sections, id))
            }
            Ok(None) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Gets the indexed source location's document depth and inherited
    /// `xml:space` state without walking unrelated source branches.
    pub(crate) fn managed_element_context(
        &self,
        id: &StableId,
    ) -> Result<Option<(usize, bool)>, PreservationError> {
        let location = match self.indexed_location(id) {
            Ok(Some(location)) => location,
            Ok(None) if self.managed_paragraph_index.is_none() => {
                return Ok(find_managed_context_counted(&self.sections, id));
            }
            Ok(None) => return Ok(None),
            Err(error) => return Err(error),
        };
        let section = self
            .sections
            .get(location.section_index)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        if !self.managed_element_at(location).is_some_and(|element| {
            matches!(&element.ownership, SourceOwnership::Managed(target) if target == id)
        }) {
            return Err(PreservationError::ManagedAnchorMissing);
        }
        indexed_element_context(&section.children, &location.node_path)
            .map(Some)
            .ok_or(PreservationError::ManagedAnchorMissing)
    }

    /// Mutable indexed access for a detached candidate. The source-use index is
    /// invalidated before mutation; callers rebuild both indexes before publish.
    pub(crate) fn managed_element_mut_indexed(
        &mut self,
        id: &StableId,
    ) -> Result<&mut PreservedElement, PreservationError> {
        self.ensure_managed_paragraph_index();
        self.invalidate_source_identity_use_index();
        let location = self
            .indexed_location(id)?
            .cloned()
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let section = self
            .sections
            .get_mut(location.section_index)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let element = element_at_path_mut(&mut section.children, &location.node_path)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        if !matches!(&element.ownership, SourceOwnership::Managed(target) if target == id) {
            return Err(PreservationError::ManagedAnchorMissing);
        }
        Ok(element)
    }

    fn indexed_location(
        &self,
        id: &StableId,
    ) -> Result<Option<&ManagedParagraphLocation>, PreservationError> {
        increment_counter(&MANAGED_INDEX_LOOKUPS);
        let Some(index) = &self.managed_paragraph_index else {
            return Ok(None);
        };
        let locations = index.get(id).map(Vec::as_slice).unwrap_or_default();
        add_counter(&MANAGED_INDEX_ENTRIES, locations.len() as u64);
        match locations {
            [] => Ok(None),
            [location] => Ok(Some(location)),
            _ => Err(PreservationError::ManagedAnchorAmbiguous),
        }
    }

    fn managed_element_at(&self, location: &ManagedParagraphLocation) -> Option<&PreservedElement> {
        element_at_path(
            self.sections.get(location.section_index)?.children.as_ref(),
            &location.node_path,
        )
    }

    fn is_supported_managed_body_location(
        &self,
        location: &ManagedParagraphLocation,
        id: &StableId,
    ) -> bool {
        let Some(section) = self.sections.get(location.section_index) else {
            return false;
        };
        let mut nodes = section.children.as_ref();
        let mut chain = Vec::with_capacity(location.node_path.len());
        for index in &location.node_path {
            let Some(PreservedNode::Element(element)) = nodes.get(*index) else {
                return false;
            };
            chain.push(element);
            nodes = &element.children;
        }
        let Some(root) = chain.first() else {
            return false;
        };
        if root.name.local_name != "FinalDraft" {
            return false;
        }
        let namespace = root.name.namespace_uri.as_deref();
        if chain
            .iter()
            .any(|element| element.name.namespace_uri.as_deref() != namespace)
        {
            return false;
        }
        match chain.as_slice() {
            [_, content, paragraph]
                if content.name.local_name == "Content"
                    && paragraph.name.local_name == "Paragraph"
                    && matches!(&paragraph.ownership, SourceOwnership::Managed(target) if target == id) =>
            {
                true
            }
            [_, content, wrapper, dual, paragraph]
                if content.name.local_name == "Content"
                    && wrapper.name.local_name == "Paragraph"
                    && wrapper.source_type.as_deref() == Some("General")
                    && matches!(&wrapper.ownership, SourceOwnership::DualDialogueWrapper(_))
                    && dual.name.local_name == "DualDialogue"
                    && paragraph.name.local_name == "Paragraph"
                    && matches!(&paragraph.ownership, SourceOwnership::Managed(target) if target == id) =>
            {
                let Some(dual_index) = location.node_path.get(3) else {
                    return false;
                };
                let Some(member_index) = location.node_path.get(4) else {
                    return false;
                };
                let Some(PreservedNode::Element(wrapper_dual)) = wrapper.children.get(*dual_index)
                else {
                    return false;
                };
                if wrapper_dual.name.local_name != "DualDialogue"
                    || wrapper_dual.children.get(*member_index).is_none_or(|node| {
                        !matches!(node, PreservedNode::Element(member) if matches!(&member.ownership, SourceOwnership::Managed(target) if target == id))
                    })
                {
                    return false;
                }
                let spell_blocks = wrapper_dual
                    .children
                    .iter()
                    .enumerate()
                    .filter_map(|(index, node)| match node {
                        PreservedNode::Element(element)
                            if element.name.local_name == "SpellCheckIgnoreLists" => Some(index),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                spell_blocks.len() == 1 && spell_blocks[0] < *member_index
            }
            _ => false,
        }
    }

    pub(crate) fn has_managed_paragraph_index(&self) -> bool {
        self.managed_paragraph_index.is_some()
    }

    pub(crate) fn scoped_global_invariants_match(&self, baseline: &PreservationState) -> bool {
        self.source == baseline.source
            && self.source_layout_settings == baseline.source_layout_settings
            && self.anchors == baseline.anchors
            && self.identities == baseline.identities
            && self.references == baseline.references
            && self.board_graph == baseline.board_graph
    }

    pub(crate) fn set_validated_source_text_bytes(&mut self, byte_count: usize) {
        self.source_text_bytes = byte_count;
    }

    /// Validates only a single managed paragraph against a complete valid
    /// baseline. It refuses any topology or identity change, leaving those
    /// cases for complete source-tree validation.
    pub(crate) fn validate_scoped_managed_edit(
        &self,
        baseline: &PreservationState,
        semantic: &ScreenplayElement,
    ) -> Result<usize, PreservationError> {
        let baseline_location = baseline
            .indexed_location(&semantic.id)?
            .cloned()
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let candidate_location = self
            .indexed_location(&semantic.id)?
            .cloned()
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        if baseline_location != candidate_location
            || !baseline.is_supported_managed_body_location(&baseline_location, &semantic.id)
            || !self.is_supported_managed_body_location(&candidate_location, &semantic.id)
        {
            return Err(PreservationError::ManagedAnchorMissing);
        }
        if !same_sections_except_managed_location(
            &baseline.sections,
            &self.sections,
            &candidate_location,
        ) {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let before = baseline
            .managed_element_at(&baseline_location)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let after = self
            .managed_element_at(&candidate_location)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        if !matches!(&before.ownership, SourceOwnership::Managed(id) if id == &semantic.id)
            || !matches!(&after.ownership, SourceOwnership::Managed(id) if id == &semantic.id)
            || !is_plain_managed_paragraph(after)
            || !same_managed_edit_shape(before, after, true)
            || after.name.local_name != "Paragraph"
        {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let role_name = fdx_role_name(semantic.role);
        let role_attribute_count = after
            .attributes
            .iter()
            .filter(|attribute| {
                attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Type"
            })
            .count();
        if after.source_type.as_deref() != Some(role_name)
            || role_attribute_count != 1
            || !after.attributes.iter().any(|attribute| {
                attribute.name.namespace_uri.is_none()
                    && attribute.name.local_name == "Type"
                    && attribute.value == role_name
            })
        {
            return Err(PreservationError::UnsupportedRoleMapping);
        }
        let mut source_projection = String::new();
        for child in &after.children {
            match child {
                PreservedNode::Text(text) | PreservedNode::CData(text)
                    if text.trim().is_empty() => {}
                PreservedNode::Text(text) | PreservedNode::CData(text) => {
                    source_projection.push_str(text)
                }
                PreservedNode::Element(text_element) if is_text_element(text_element, after) => {
                    for text_child in &text_element.children {
                        match text_child {
                            PreservedNode::Text(text) | PreservedNode::CData(text) => {
                                source_projection.push_str(text)
                            }
                            _ => return Err(PreservationError::UnsupportedTextMapping),
                        }
                    }
                }
                _ => return Err(PreservationError::UnsupportedTextMapping),
            }
        }
        if source_projection != semantic.text {
            return Err(PreservationError::ManagedTextMismatchAt(
                ManagedTextMismatchSite::ScopedEditValidation,
            ));
        }
        let old_bytes = source_text_bytes_in_element(before);
        let new_bytes = source_text_bytes_in_element(after);
        let new_total = self
            .source_text_bytes
            .saturating_sub(old_bytes)
            .saturating_add(new_bytes);
        if new_total > MAX_SOURCE_TEXT_BYTES {
            return Err(PreservationError::TextTooLarge);
        }
        Ok(new_total)
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
                SourceOwnership::DualDialogueWrapper(id) => {
                    if !anchor.as_ref().is_some_and(|anchor| {
                        matches!(
                            anchor_targets.get(anchor),
                            Some((AnchorTarget::DualDialogueWrapper(target), AnchorRelation::Represents))
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
        dual_dialogue_group_ids: &BTreeSet<StableId>,
    ) -> Result<(), PreservationError> {
        self.validate()?;
        for anchor in &self.anchors {
            match &anchor.target {
                AnchorTarget::ProjectElement(id) if !project_element_ids.contains(id) => {
                    return Err(PreservationError::AnchorTargetMissing);
                }
                AnchorTarget::DualDialogueWrapper(id) if !dual_dialogue_group_ids.contains(id) => {
                    return Err(PreservationError::AnchorTargetMissing);
                }
                _ => {}
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
                if let SourceOwnership::DualDialogueWrapper(id) = &element.ownership {
                    if !dual_dialogue_group_ids.contains(id) {
                        return Err(PreservationError::AnchorTargetMissing);
                    }
                }
                stack.extend(element.children.iter());
            }
        }
        Ok(())
    }

    pub(crate) fn dual_dialogue_wrapper_anchor(&self, id: &StableId) -> Option<SourceAnchorId> {
        let mut stack = self.sections.iter().flat_map(|section| section.children.iter()).collect::<Vec<_>>();
        while let Some(node) = stack.pop() {
            if let PreservedNode::Element(element) = node {
                if matches!(&element.ownership, SourceOwnership::DualDialogueWrapper(owner) if owner == id) {
                    return element.anchor.clone();
                }
                stack.extend(element.children.iter());
            }
        }
        None
    }

    pub(crate) fn has_managed_element(&self, id: &StableId) -> bool {
        increment_counter(&MANAGED_INDEX_LOOKUPS);
        if let Some(index) = &self.managed_paragraph_index {
            let locations = index.get(id).map(Vec::as_slice).unwrap_or_default();
            add_counter(&MANAGED_INDEX_ENTRIES, locations.len() as u64);
            return !locations.is_empty();
        }
        self.sections
            .iter()
            .any(|section| contains_managed_element_counted(&section.children, id))
    }

    /// Returns whether the exact managed Paragraph owns a direct inline Alts
    /// child. The codec has already validated its role, namespace, and
    /// multiplicity; callers use this only to guard role transitions.
    pub(crate) fn has_inline_alts(&self, id: &StableId) -> bool {
        let paragraph = match self.indexed_location(id) {
            Ok(Some(location)) => self.managed_element_at(location),
            Ok(None) if self.managed_paragraph_index.is_none() => {
                find_managed_element_counted(&self.sections, id)
            }
            _ => None,
        };
        paragraph.is_some_and(|paragraph| {
            paragraph.children.iter().any(|node| matches!(node,
                PreservedNode::Element(element)
                    if element.name.local_name == "Alts"
                        && element.name.namespace_uri == paragraph.name.namespace_uri
            ))
        })
    }

    /// Reconciles a join or cross-paragraph deletion while retaining the
    /// surviving owner's Alts in place. The left source is trimmed through its
    /// mapped Text runs, and only the retained suffix Text runs of the final
    /// removed Paragraph are copied; removed Alts and AltCollection are never
    /// transferred or pruned.
    fn reconcile_joined_text_nodes(
        &mut self,
        survivor_id: &StableId,
        original_survivor_text: &str,
        survivor_prefix_len: usize,
        final_removed_id: &StableId,
        final_removed_text: &str,
        final_suffix_start: usize,
        expected_result: &str,
        expected_runs: Option<&[String]>,
    ) -> Result<(), PreservationError> {
        let survivor_location = self.indexed_location(survivor_id)?
            .cloned().ok_or(PreservationError::ManagedAnchorMissing)?;
        let removed_location = self.indexed_location(final_removed_id)?
            .cloned().ok_or(PreservationError::ManagedAnchorMissing)?;
        let survivor = self.managed_element_at(&survivor_location)
            .filter(|element| matches!(&element.ownership, SourceOwnership::Managed(id) if id == survivor_id))
            .ok_or(PreservationError::ManagedAnchorMissing)?
            .clone();
        let removed = self.managed_element_at(&removed_location)
            .filter(|element| matches!(&element.ownership, SourceOwnership::Managed(id) if id == final_removed_id))
            .ok_or(PreservationError::ManagedAnchorMissing)?
            .clone();
        let survivor_runs = paragraph_text_nodes_with_direct_alts(&survivor)?;
        let removed_runs = paragraph_text_nodes_with_direct_alts(&removed)?;
        if text_of_nodes(&survivor_runs) != original_survivor_text
            || text_of_nodes(&removed_runs) != final_removed_text
            || survivor_prefix_len > original_survivor_text.len()
            || final_suffix_start > final_removed_text.len()
            || !original_survivor_text.is_char_boundary(survivor_prefix_len)
            || !final_removed_text.is_char_boundary(final_suffix_start)
        {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let suffix_runs = slice_source_text_nodes(&removed_runs, final_suffix_start)?;
        let suffix_text = text_of_nodes(&suffix_runs);
        if format!("{}{}", &original_survivor_text[..survivor_prefix_len], suffix_text)
            != expected_result
        {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        self.replace_managed_text(
            survivor_id,
            original_survivor_text,
            survivor_prefix_len,
            original_survivor_text.len(),
            "",
            expected_runs,
        )?;
        let target = self.managed_element_mut(survivor_id)?;
        target.children.extend(suffix_runs);
        Ok(())
    }

    /// Trims the right part of a split from the original Paragraph's mapped
    /// Text nodes. Opaque children, including Alts, stay on the original/left
    /// owner and are never copied to the new Paragraph.
    fn trim_split_text(
        &mut self,
        id: &StableId,
        original_text: &str,
        split_offset: usize,
        expected_runs: Option<&[String]>,
    ) -> Result<(), PreservationError> {
        self.replace_managed_text(
            id,
            original_text,
            split_offset,
            original_text.len(),
            "",
            expected_runs,
        )
    }

    pub(crate) fn has_direct_paragraph_formatting(&self, id: &StableId) -> bool {
        const DIRECT_FIELDS: [&str; 8] = [
            "Alignment",
            "FirstIndent",
            "Leading",
            "LeftIndent",
            "RightIndent",
            "SpaceBefore",
            "Spacing",
            "StartsNewPage",
        ];
        let paragraph = match self.indexed_location(id) {
            Ok(Some(location)) => self.managed_element_at(location),
            Ok(None) if self.managed_paragraph_index.is_none() => {
                find_managed_element_counted(&self.sections, id)
            }
            Err(_) => return true,
            _ => None,
        };
        paragraph.is_some_and(|paragraph| {
            paragraph.attributes.iter().any(|attribute| {
                attribute.name.namespace_uri.is_none()
                    && DIRECT_FIELDS.contains(&attribute.name.local_name.as_str())
            })
        })
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
            )
            .map_err(|error| {
                categorize_managed_text_mismatch(error, ManagedTextMismatchSite::TextReplacement)
            });
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
            )
            .map_err(|error| {
                categorize_managed_text_mismatch(error, ManagedTextMismatchSite::TextReplacement)
            });
        }
        let source_segments = indexes
            .iter()
            .map(|index| match &element.children[*index] {
                PreservedNode::Text(text) | PreservedNode::CData(text) => text.to_string(),
                _ => unreachable!(),
            })
            .collect::<Vec<_>>();
        if source_segments.concat() != old_text {
            return Err(PreservationError::ManagedTextMismatchAt(
                ManagedTextMismatchSite::TextReplacement,
            ));
        }
        if expected_runs.is_some_and(|runs| runs != source_segments.as_slice()) {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let mut boundary = 0usize;
        let clear_all_text = start == 0 && end == old_text.len() && inserted.is_empty();
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
                if start < boundary && end > boundary && !clear_all_text {
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
                PreservedNode::Text(value) | PreservedNode::CData(value) => *value = text.into(),
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
        let text_elements = validate_managed_text_elements_replacement(paragraph, expected_runs)?;
        if expected_runs.is_empty() {
            let indexes = text_elements.into_iter().collect::<BTreeSet<_>>();
            paragraph.children = paragraph
                .children
                .drain(..)
                .enumerate()
                .filter_map(|(index, child)| (!indexes.contains(&index)).then_some(child))
                .collect();
            if !replacement.is_empty() {
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
                        children: vec![PreservedNode::Text(replacement.to_owned().into())].into(),
                    }));
            }
            return Ok(());
        }

        if replacement.is_empty() {
            let indexes = text_elements.into_iter().collect::<BTreeSet<_>>();
            paragraph.children = paragraph
                .children
                .drain(..)
                .enumerate()
                .filter_map(|(index, child)| (!indexes.contains(&index)).then_some(child))
                .collect();
            return Ok(());
        }

        for (run_index, index) in text_elements.into_iter().enumerate() {
            let PreservedNode::Element(text_element) = &mut paragraph.children[index] else {
                return Err(PreservationError::UnsupportedTextMapping);
            };
            if let Some(first) = text_element.children.first_mut() {
                match first {
                    PreservedNode::Text(text) | PreservedNode::CData(text) => {
                        *text = if run_index == 0 {
                            replacement.to_owned().into()
                        } else {
                            String::new().into()
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
                    .push(PreservedNode::Text(replacement.to_owned().into()));
            }
        }
        Ok(())
    }

    /// Checks that a complete plain-text replacement has an exact, supported
    /// source mapping without mutating the retained tree. Derived correction
    /// uses this to defer an unsupported scene before reconciling its arcs.
    pub(crate) fn validate_managed_text_elements_replacement(
        &self,
        id: &StableId,
        expected_runs: &[String],
    ) -> Result<(), PreservationError> {
        let paragraph = if let Some(location) = self.indexed_location(id)? {
            self.managed_element_at(location)
        } else {
            find_managed_element_counted(&self.sections, id)
        }
        .ok_or(PreservationError::ManagedAnchorMissing)?;
        validate_managed_text_elements_replacement(paragraph, expected_runs).map(|_| ())
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
        generated_scene_heading_deletions: &BTreeSet<StableId>,
        groups: &[crate::dual_dialogue::DualDialogueGroup],
    ) -> Result<(), PreservationError> {
        self.ensure_managed_paragraph_index();
        self.ensure_source_identity_use_index();
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

        // Generated heading deletion is authorized only by explicit selection
        // deletion. Validate every selected heading subtree and every identity
        // it owns against the unmodified candidate before removing any source.
        for element in before {
            if !after_ids.contains(&element.id)
                && element.role == Role::SceneHeading
                && self.has_managed_element(&element.id)
                && generated_scene_heading_deletions.contains(&element.id)
            {
                self.preflight_generated_scene_heading_removal(
                    &element.id,
                    element.source_provenance.as_ref(),
                )?;
            }
        }

        let mut structurally_reconciled_text = BTreeSet::new();
        // Preserve retained owner attachments across joins and ranged deletion.
        // Only the survivor's left prefix and the final removed Paragraph's
        // right suffix may contribute canonical text; all removed Alts remain
        // attached to their deleted owners and are discarded with those nodes.
        for (index, previous) in before.iter().enumerate() {
            let end = (index + 1..before.len())
                .find(|next| after_ids.contains(&before[*next].id))
                .unwrap_or(before.len());
            if end == index + 1 {
                continue;
            }
            let removed = &before[index + 1..end];
            let Some(updated) = after.iter().find(|item| item.id == previous.id) else {
                continue;
            };
            let final_removed = &removed[removed.len() - 1];
            let has_inline_attachment = self.has_inline_alts(&previous.id)
                || removed.iter().any(|item| self.has_inline_alts(&item.id));
            if !has_inline_attachment
            {
                continue;
            }
            if (self.has_inline_alts(&previous.id) && previous.role != Role::Dialogue)
                || removed.iter().any(|item| {
                    self.has_inline_alts(&item.id) && item.role != Role::Dialogue
                })
            {
                return Err(PreservationError::StructuralAttachmentConflict);
            }
            let mut partitions = Vec::new();
            for (prefix_len, _) in previous.text.char_indices().chain(
                std::iter::once((previous.text.len(), '\0')),
            ) {
                if !updated.text.starts_with(&previous.text[..prefix_len]) {
                    continue;
                }
                let suffix_len = updated.text.len().saturating_sub(prefix_len);
                let Some(suffix_start) = final_removed.text.len().checked_sub(suffix_len) else {
                    continue;
                };
                if final_removed.text.is_char_boundary(suffix_start)
                    && updated.text[prefix_len..] == final_removed.text[suffix_start..]
                {
                    partitions.push((prefix_len, suffix_start));
                }
            }
            let [(prefix_len, suffix_start)] = partitions.as_slice() else {
                return Err(PreservationError::UnsupportedTextMapping);
            };
            let expected_runs = previous.runs.iter().map(|run| run.text.clone()).collect::<Vec<_>>();
            self.reconcile_joined_text_nodes(
                &previous.id,
                &previous.text,
                *prefix_len,
                &final_removed.id,
                &final_removed.text,
                *suffix_start,
                &updated.text,
                (!previous.runs.is_empty()).then_some(expected_runs.as_slice()),
            )?;
            structurally_reconciled_text.insert(previous.id.clone());
        }

        for element in before {
            if !after_ids.contains(&element.id) && self.has_managed_element(&element.id) {
                self.remove_managed_paragraph(
                    &element.id,
                    element.source_provenance.as_ref(),
                    element.role,
                    generated_scene_heading_deletions.contains(&element.id),
                )?;
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
            let provenance = self.insert_managed_paragraph(element, index, &ordered_ids, groups)?;
            element.source_provenance = Some(provenance);
            self.rebuild_managed_paragraph_index();
        }

        // A newly inserted right Paragraph after an existing owner is a split
        // only when its two canonical texts exactly partition the old text.
        // Trim the original owner's mapped Text nodes; its opaque Alts remains
        // attached there and the fresh Paragraph has no copied attachment.
        for index in 1..after.len() {
            let right = &after[index];
            let left = &after[index - 1];
            let Some(original) = before.iter().find(|item| item.id == left.id) else {
                continue;
            };
            if before_ids.contains(&right.id)
                || original.text != format!("{}{}", left.text, right.text)
                || !self.has_inline_alts(&original.id)
            {
                continue;
            }
            let expected = original.runs.iter().map(|run| run.text.clone()).collect::<Vec<_>>();
            self.trim_split_text(
                &original.id,
                &original.text,
                left.text.len(),
                (!original.runs.is_empty()).then_some(expected.as_slice()),
            )?;
            structurally_reconciled_text.insert(original.id.clone());
        }

        // Other structural text projections continue through the existing
        // strict path. In particular, arbitrary attachments and unsupported
        // run mappings are not rewritten to make a candidate pass.
        for element in after.iter() {
            let Some(previous) = before.iter().find(|item| item.id == element.id) else {
                continue;
            };
            if previous.text != element.text
                && self.has_managed_element(&element.id)
                && !structurally_reconciled_text.contains(&element.id)
            {
                self.replace_managed_projection(element)?;
            }
        }
        self.rebuild_managed_paragraph_index();
        Ok(())
    }

    /// Reconciles one supported title-page structural transaction against the
    /// exact TitlePage -> Content source shape. All callers operate on detached
    /// Projects, so every mapping, identity, and attachment check completes
    /// before session publication.
    pub(crate) fn reconcile_title_structure(
        &mut self,
        before: &[TitlePageParagraph],
        after: &mut [TitlePageParagraph],
    ) -> Result<(), PreservationError> {
        self.ensure_managed_paragraph_index();
        self.ensure_source_identity_use_index();

        let before_ids = before.iter().map(|paragraph| paragraph.id.clone()).collect::<Vec<_>>();
        let after_ids = after.iter().map(|paragraph| paragraph.id.clone()).collect::<Vec<_>>();
        if before_ids == after_ids {
            return Ok(());
        }
        let before_set = before_ids.iter().cloned().collect::<BTreeSet<_>>();
        let after_set = after_ids.iter().cloned().collect::<BTreeSet<_>>();
        if before_set.len() != before_ids.len() || after_set.len() != after_ids.len() {
            return Err(PreservationError::ManagedAnchorAmbiguous);
        }
        let added = after_ids.iter().filter(|id| !before_set.contains(*id)).cloned().collect::<Vec<_>>();
        let removed = before_ids.iter().filter(|id| !after_set.contains(*id)).cloned().collect::<Vec<_>>();

        if added.len() == 1 && removed.is_empty() && after.len() == before.len().saturating_add(1) {
            let new_index = after_ids.iter().position(|id| id == &added[0])
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            if new_index == 0 || after[new_index - 1].id != before_ids[new_index - 1]
                || after_ids.iter().enumerate().any(|(index, id)| {
                    index != new_index && id != &before_ids[if index < new_index { index } else { index - 1 }]
                })
            {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            let source_index = new_index - 1;
            let source = before.get(source_index).ok_or(PreservationError::ManagedAnchorMissing)?;
            let (left, right) = after.split_at_mut(new_index);
            if left[source_index].id != source.id
                || left[source_index].text.len().saturating_add(right[0].text.len()) != source.text.len()
                || format!("{}{}", left[source_index].text, right[0].text) != source.text {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            self.split_title_source_paragraph(source, &left[source_index], &mut right[0])?;
            return Ok(());
        }

        if added.is_empty() && !removed.is_empty() && after.len().saturating_add(removed.len()) == before.len() {
            let first_removed = before_ids.iter().position(|id| !after_set.contains(id))
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            if first_removed == 0 {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            let last_removed = before_ids.iter().rposition(|id| !after_set.contains(id))
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            if before_ids[first_removed..=last_removed].iter().any(|id| after_set.contains(id))
                || before_ids.iter().filter(|id| after_set.contains(*id)).collect::<Vec<_>>()
                    != after_ids.iter().collect::<Vec<_>>()
            {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            let recipient = &before[first_removed - 1];
            if !after_set.contains(&recipient.id) {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            let mut removed_semantics = Vec::with_capacity(removed.len());
            let mut source_templates = Vec::new();
            let recipient_source = self.validate_title_source(recipient)?;
            source_templates.extend(title_text_templates(&recipient_source)?);
            for id in &removed {
                let semantic = before.iter().find(|paragraph| &paragraph.id == id)
                    .ok_or(PreservationError::ManagedAnchorMissing)?;
                let source = self.validate_title_source(semantic)?;
                if !same_title_paragraph_shell(&recipient_source, &source) {
                    return Err(PreservationError::StructuralAttachmentConflict);
                }
                self.preflight_title_paragraph_removal(semantic, &source)?;
                source_templates.extend(title_text_templates(&source)?);
                removed_semantics.push((semantic.id.clone(), semantic.source_provenance.as_ref()
                    .and_then(|provenance| provenance.foreign_id.clone())));
            }
            let updated_recipient = after.iter().find(|paragraph| paragraph.id == recipient.id)
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            let replacement_children = title_text_children_from_templates(
                &recipient_source,
                &source_templates,
                updated_recipient,
            )?;
            {
                let target = self.managed_element_mut(&recipient.id)?;
                target.children = replacement_children.into();
            }
            self.rebuild_managed_paragraph_index();
            for (id, foreign_id) in removed_semantics {
                let location = self.indexed_location(&id)?.cloned()
                    .ok_or(PreservationError::ManagedAnchorMissing)?;
                if !remove_managed_at_location(&mut self.sections, &location) {
                    return Err(PreservationError::ManagedAnchorMissing);
                }
                if let Some(foreign_id) = foreign_id {
                    self.identities.retain(|identity| {
                        !(identity.domain == ForeignIdDomain::Paragraph && identity.value == foreign_id)
                    });
                }
                self.anchors.retain(|anchor| {
                    !matches!(&anchor.target, AnchorTarget::ProjectElement(target) if target == &id)
                });
                self.source_layout_settings.direct_paragraph_overrides.remove(&id);
                self.rebuild_managed_paragraph_index();
            }
            return Ok(());
        }

        Err(PreservationError::UnsupportedTextMapping)
    }

    fn split_title_source_paragraph(
        &mut self,
        original: &TitlePageParagraph,
        left_semantic: &TitlePageParagraph,
        right_semantic: &mut TitlePageParagraph,
    ) -> Result<(), PreservationError> {
        let location = self.title_source_location(original)?;
        let source = self.validate_title_source_at(original, &location)?;
        let (left_children, right_children) = partition_title_children(
            &source,
            original,
            left_semantic,
            right_semantic,
        )?;
        let foreign_id = self.fresh_foreign_paragraph_id()?;
        let anchor_id = self.fresh_anchor_id()?;
        let mut right_source = source.clone();
        set_paragraph_foreign_id(&mut right_source, &foreign_id)?;
        right_source.ownership = SourceOwnership::Managed(right_semantic.id.clone());
        right_source.anchor = Some(anchor_id.clone());
        right_source.children = right_children.into();
        {
            let left_source = self.managed_element_mut(&original.id)?;
            left_source.children = left_children.into();
        }
        self.rebuild_managed_paragraph_index();
        let parent = title_content_parent_mut(&mut self.sections, &location)?;
        let insert_at = location.node_path[2].checked_add(1)
            .ok_or(PreservationError::TooManyEvents)?;
        parent.children.insert(insert_at, PreservedNode::Element(right_source));
        self.anchors.push(SourceAnchor {
            id: anchor_id,
            target: AnchorTarget::ProjectElement(right_semantic.id.clone()),
            relation: AnchorRelation::Represents,
        });
        self.identities.push(ForeignIdentity::new(ForeignIdDomain::Paragraph, foreign_id.clone())?);
        right_semantic.source_provenance = Some(SourceProvenance {
            format: SourceFormat::Fdx,
            section: SourceSection::TitlePage,
            ordinal: original.source_provenance.as_ref()
                .map(|provenance| provenance.ordinal.saturating_add(1))
                .ok_or(PreservationError::ManagedAnchorMissing)?,
            foreign_id: Some(foreign_id),
            foreign_role: original.source_provenance.as_ref()
                .and_then(|provenance| provenance.foreign_role.clone()),
        });
        self.rebuild_managed_paragraph_index();
        Ok(())
    }

    fn title_source_location(
        &self,
        semantic: &TitlePageParagraph,
    ) -> Result<ManagedParagraphLocation, PreservationError> {
        let location = self.indexed_location(&semantic.id)?
            .cloned()
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        if !is_title_content_location(&self.sections, &location) {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        Ok(location)
    }

    fn validate_title_source(
        &self,
        semantic: &TitlePageParagraph,
    ) -> Result<PreservedElement, PreservationError> {
        let location = self.title_source_location(semantic)?;
        self.validate_title_source_at(semantic, &location)
    }

    fn validate_title_source_at(
        &self,
        semantic: &TitlePageParagraph,
        location: &ManagedParagraphLocation,
    ) -> Result<PreservedElement, PreservationError> {
        let source = self.managed_element_at(location)
            .filter(|element| matches!(&element.ownership, SourceOwnership::Managed(id) if id == &semantic.id))
            .ok_or(PreservationError::ManagedAnchorMissing)?
            .clone();
        let provenance = semantic.source_provenance.as_ref()
            .filter(|provenance| provenance.format == SourceFormat::Fdx && provenance.section == SourceSection::TitlePage)
            .ok_or(PreservationError::ManagedTextMismatchAt(ManagedTextMismatchSite::ParagraphRemoval))?;
        if paragraph_id_attribute_count(&source) > 1
            || paragraph_foreign_id(&source) != provenance.foreign_id.as_deref()
            || source.source_type != provenance.foreign_role
        {
            return Err(PreservationError::ManagedTextMismatchAt(ManagedTextMismatchSite::ParagraphRemoval));
        }
        let matching_anchors = self.anchors.iter().filter(|anchor| {
            matches!(&anchor.target, AnchorTarget::ProjectElement(id) if id == &semantic.id)
                && anchor.relation == AnchorRelation::Represents
        }).collect::<Vec<_>>();
        if matching_anchors.len() != 1 || source.anchor.as_ref() != Some(&matching_anchors[0].id) {
            return Err(PreservationError::ManagedAnchorAmbiguous);
        }
        validate_title_text_shape(&source, semantic)?;
        Ok(source)
    }

    fn preflight_title_paragraph_removal(
        &mut self,
        semantic: &TitlePageParagraph,
        source: &PreservedElement,
    ) -> Result<(), PreservationError> {
        if !is_plain_managed_paragraph(source) {
            return Err(PreservationError::StructuralAttachmentConflict);
        }
        if let Some(id) = semantic.source_provenance.as_ref().and_then(|provenance| provenance.foreign_id.as_deref()) {
            if !self.paragraph_identity_has_unique_source_definition(id)
                || !self.paragraph_identity_safely_removable(id)
                || has_potential_reference(&self.sections, id, &semantic.id)
            {
                return Err(PreservationError::PotentialReferenceConflict);
            }
        }
        Ok(())
    }

    fn preflight_generated_scene_heading_removal(
        &mut self,
        id: &StableId,
        provenance: Option<&SourceProvenance>,
    ) -> Result<(), PreservationError> {
        let location = self
            .indexed_location(id)?
            .cloned()
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let target = self
            .managed_element_at(&location)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let nested_ids = crate::scene_metadata::recognized_generated_scene_heading_subtree(target)
            .ok_or(PreservationError::StructuralAttachmentConflict)?;
        let heading_id = paragraph_foreign_id(target)
            .map(str::to_owned)
            .ok_or(PreservationError::StructuralAttachmentConflict)?;
        if provenance.and_then(|item| item.foreign_id.as_deref()) != Some(heading_id.as_str()) {
            return Err(PreservationError::ManagedTextMismatchAt(
                ManagedTextMismatchSite::ParagraphRemoval,
            ));
        }
        let mut identities = BTreeSet::new();
        identities.insert(heading_id);
        for nested_id in nested_ids {
            if !identities.insert(nested_id) {
                return Err(PreservationError::StructuralAttachmentConflict);
            }
        }
        self.ensure_source_identity_use_index();
        if identities.iter().any(|value| {
            !self.paragraph_identity_safely_removable(value)
                || !self.paragraph_identity_has_unique_source_definition(value)
        }) {
            return Err(PreservationError::PotentialReferenceConflict);
        }
        Ok(())
    }

    fn insert_managed_paragraph(
        &mut self,
        semantic: &ScreenplayElement,
        index: usize,
        ordered: &[StableId],
        groups: &[crate::dual_dialogue::DualDialogueGroup],
    ) -> Result<SourceProvenance, PreservationError> {
        if let Some(group) = groups.iter().find(|group| {
            group.member_ids().any(|member| member == &semantic.id)
        }) {
            return self.insert_nested_managed_paragraph(semantic, index, group);
        }
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
            children: SharedVec::default(),
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
                    children: vec![PreservedNode::Text(run.text.into())].into(),
                }));
        }

        self.invalidate_source_identity_use_index();
        let mut wrapped_member_slots = BTreeMap::new();
        for group in groups {
            for member in group.member_ids() {
                let Some(location) = self.indexed_location(member)?.cloned() else {
                    continue;
                };
                if is_exact_dual_dialogue_member_location(&self.sections, &location, &group.id) {
                    wrapped_member_slots.insert(member.clone(), location.node_path[2]);
                }
            }
        }
        let children = content_children_mut(&mut self.sections)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let mut insert_at = None;
        for following in ordered.iter().skip(index + 1) {
            if let Some(position) = managed_child_index(children, following) {
                insert_at = Some(position);
                break;
            }
            if let Some(position) = wrapped_member_slots.get(following) {
                insert_at = Some(*position);
                break;
            }
        }
        if insert_at.is_none() {
            for preceding in ordered[..index].iter().rev() {
                if let Some(position) = managed_child_index(children, preceding) {
                    insert_at = Some(position + 1);
                    break;
                }
                if let Some(position) = wrapped_member_slots.get(preceding) {
                    insert_at = Some(*position + 1);
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

    fn insert_nested_managed_paragraph(
        &mut self,
        semantic: &ScreenplayElement,
        index: usize,
        group: &crate::dual_dialogue::DualDialogueGroup,
    ) -> Result<SourceProvenance, PreservationError> {
        self.ensure_managed_paragraph_index();
        let member_ids = group.member_ids().cloned().collect::<Vec<_>>();
        let template_location = member_ids.iter()
            .filter(|id| *id != &semantic.id)
            .find_map(|id| self.indexed_location(id).ok().flatten().cloned())
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        if !is_exact_dual_dialogue_member_location(&self.sections, &template_location, &group.id) {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        for member_id in member_ids.iter().filter(|id| *id != &semantic.id) {
            let location = self.indexed_location(member_id)?
                .cloned()
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            if !is_exact_dual_dialogue_member_location(&self.sections, &location, &group.id)
                || location.section_index != template_location.section_index
                || location.node_path[..4] != template_location.node_path[..4]
            {
                return Err(PreservationError::UnsupportedTextMapping);
            }
        }
        let template = self.managed_element_at(&template_location)
            .ok_or(PreservationError::ManagedAnchorMissing)?
            .clone();
        let ManagedParagraphTemplate {
            paragraph_name,
            paragraph_bindings,
            text_name,
            text_bindings,
            id_spelling,
        } = paragraph_template(&template)?;
        let foreign_id = self.fresh_foreign_paragraph_id()?;
        let anchor_id = self.fresh_anchor_id()?;
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
            children: SharedVec::default(),
        };
        for run in semantic_text_runs(semantic) {
            paragraph.children.push(PreservedNode::Element(PreservedElement {
                name: text_name.clone(),
                attributes: run.attributes.iter().map(|attribute| PreservedAttribute {
                    name: source_name_from_qname(&attribute.name),
                    value: attribute.value.clone(),
                }).collect(),
                namespace_bindings: text_bindings.clone(),
                source_type: None,
                ownership: SourceOwnership::Opaque,
                dependency_confidence: DependencyConfidence::Unknown,
                anchor: None,
                children: vec![PreservedNode::Text(run.text.into())].into(),
            }));
        }

        let member_position = member_ids.iter().position(|id| id == &semantic.id)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let parent_path = &template_location.node_path[..4];
        let nodes = {
            let section = self.sections.get_mut(template_location.section_index)
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            let parent = element_at_path_mut(&mut section.children, parent_path)
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            &mut parent.children
        };
        let mut insert_at = None;
        for following in member_ids.iter().skip(member_position + 1) {
            if let Some(position) = managed_child_index(nodes, following) {
                insert_at = Some(position);
                break;
            }
        }
        if insert_at.is_none() {
            for preceding in member_ids[..member_position].iter().rev() {
                if let Some(position) = managed_child_index(nodes, preceding) {
                    insert_at = Some(position + 1);
                    break;
                }
            }
        }
        let position = insert_at.ok_or(PreservationError::ManagedAnchorMissing)?;
        nodes.insert(position, PreservedNode::Element(paragraph));
        self.anchors.push(SourceAnchor {
            id: anchor_id,
            target: AnchorTarget::ProjectElement(semantic.id.clone()),
            relation: AnchorRelation::Represents,
        });
        self.identities.push(ForeignIdentity::new(
            ForeignIdDomain::Paragraph,
            foreign_id.clone(),
        )?);
        self.invalidate_source_identity_use_index();
        self.rebuild_managed_paragraph_index();
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
        role: Role,
        allow_generated_scene_heading_deletion: bool,
    ) -> Result<(), PreservationError> {
        let location = self
            .indexed_location(id)?
            .cloned()
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let (foreign_id, nested_paragraph_ids, is_generated_scene_heading) = {
            let target = self
                .managed_element_at(&location)
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            if is_plain_managed_paragraph(target) {
                (
                    paragraph_foreign_id(target).map(str::to_owned),
                    Vec::new(),
                    false,
                )
            } else if role == Role::Dialogue && is_managed_paragraph_with_direct_alts(target) {
                // The complete owner node (including Alts) is removed as one
                // unit. AltCollection is outside this managed subtree and is
                // intentionally never pruned, even when this creates orphans.
                (
                    paragraph_foreign_id(target).map(str::to_owned),
                    Vec::new(),
                    false,
                )
            } else if role == Role::SceneHeading && allow_generated_scene_heading_deletion {
                let nested =
                    crate::scene_metadata::recognized_generated_scene_heading_subtree(target)
                        .ok_or(PreservationError::StructuralAttachmentConflict)?;
                (
                    paragraph_foreign_id(target).map(str::to_owned),
                    nested,
                    true,
                )
            } else {
                return Err(PreservationError::StructuralAttachmentConflict);
            }
        };
        if is_generated_scene_heading && foreign_id.is_none() {
            return Err(PreservationError::StructuralAttachmentConflict);
        }
        if provenance.and_then(|item| item.foreign_id.as_deref()) != foreign_id.as_deref() {
            return Err(PreservationError::ManagedTextMismatchAt(
                ManagedTextMismatchSite::ParagraphRemoval,
            ));
        }
        let mut removed_foreign_ids = BTreeSet::new();
        if let Some(value) = foreign_id.as_deref() {
            removed_foreign_ids.insert(value.to_owned());
        }
        for nested_id in nested_paragraph_ids {
            if !removed_foreign_ids.insert(nested_id) {
                return Err(PreservationError::StructuralAttachmentConflict);
            }
        }
        self.ensure_source_identity_use_index();
        if removed_foreign_ids.iter().any(|value| {
            !self.paragraph_identity_safely_removable(value)
                || !self.paragraph_identity_has_unique_source_definition(value)
        }) {
            return Err(PreservationError::PotentialReferenceConflict);
        }
        if is_generated_scene_heading && !removed_foreign_ids.is_empty() {
            self.ensure_source_identity_use_index();
            if removed_foreign_ids.iter().any(|value| {
                !self.paragraph_identity_safely_removable(value)
                    || !self.paragraph_identity_has_unique_source_definition(value)
            }) {
                return Err(PreservationError::PotentialReferenceConflict);
            }
        }
        if let Some(value) = foreign_id.as_deref() {
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
            if referenced || has_potential_reference(&self.sections, value, id) {
                return Err(PreservationError::PotentialReferenceConflict);
            }
        }
        let removed = remove_managed_at_location(&mut self.sections, &location);
        if !removed {
            return Err(PreservationError::ManagedAnchorMissing);
        }
        self.identities.retain(|identity| {
            !(identity.domain == ForeignIdDomain::Paragraph
                && removed_foreign_ids.contains(&identity.value))
        });
        self.source_layout_settings
            .direct_paragraph_overrides
            .remove(id);
        self.anchors.retain(|anchor| {
            !matches!(&anchor.target, AnchorTarget::ProjectElement(target) if target == id)
        });
        self.rebuild_managed_paragraph_index();
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
        paragraph.children = rebuilt.into();
        Ok(())
    }

    fn fresh_foreign_paragraph_id(&mut self) -> Result<String, PreservationError> {
        self.ensure_source_identity_use_index();
        let reserved = self
            .identities
            .iter()
            .map(|identity| identity.value.clone())
            .chain(
                self.board_graph
                    .entities
                    .iter()
                    .map(|entity| entity.identity.value.clone()),
            )
            .chain(self.references.iter().flat_map(|reference| {
                [
                    reference.source.value.clone(),
                    reference.target.value.clone(),
                ]
            }))
            .chain(self.board_graph.references.iter().flat_map(|reference| {
                [
                    reference.source.value.clone(),
                    reference.target.value.clone(),
                ]
            }))
            .chain(self.indexed_source_id_values());
        let mut allocator = ParagraphIdAllocator::new(reserved);
        allocator.allocate().map_err(|error| match error {
            ParagraphIdAllocationError::EntropyUnavailable => {
                PreservationError::ParagraphIdentityEntropyUnavailable
            }
            ParagraphIdAllocationError::CollisionLimit => {
                PreservationError::ParagraphIdentityCollisionLimit
            }
        })
    }

    fn fresh_anchor_id(&self) -> Result<SourceAnchorId, PreservationError> {
        for serial in 1..=u64::try_from(MAX_SOURCE_EVENTS).unwrap_or(u64::MAX) {
            let candidate = SourceAnchorId::new(format!("scriptcraft-anchor-{serial}"))?;
            if !self.anchors.iter().any(|anchor| anchor.id == candidate) {
                return Ok(candidate);
            }
        }
        Err(PreservationError::TooManyEvents)
    }

    fn managed_element_mut(
        &mut self,
        id: &StableId,
    ) -> Result<&mut PreservedElement, PreservationError> {
        self.ensure_managed_paragraph_index();
        self.invalidate_source_identity_use_index();
        let location = self
            .indexed_location(id)?
            .cloned()
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let indexed_target_matches = self
            .sections
            .get(location.section_index)
            .and_then(|section| element_at_path(&section.children, &location.node_path))
            .is_some_and(|element| {
                matches!(&element.ownership, SourceOwnership::Managed(target) if target == id)
            });
        let location = if indexed_target_matches {
            location
        } else {
            // Structural source reconciliation can invalidate a cached path.
            // Never let a stale location apply a text edit to a different
            // managed paragraph; rebuild from the authoritative tree and
            // resolve the requested identity again before returning it.
            self.rebuild_managed_paragraph_index();
            self.indexed_location(id)?
                .cloned()
                .ok_or(PreservationError::ManagedAnchorMissing)?
        };
        let section = self
            .sections
            .get_mut(location.section_index)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let element = element_at_path_mut(&mut section.children, &location.node_path)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        if !matches!(&element.ownership, SourceOwnership::Managed(target) if target == id) {
            return Err(PreservationError::ManagedAnchorMissing);
        }
        Ok(element)
    }
}

fn validate_managed_text_elements_replacement(
    paragraph: &PreservedElement,
    expected_runs: &[String],
) -> Result<Vec<usize>, PreservationError> {
    let mut text_elements = Vec::new();
    for (index, node) in paragraph.children.iter().enumerate() {
        match node {
            PreservedNode::Element(element)
                if element.name.local_name == "Text"
                    && element.name.namespace_uri == paragraph.name.namespace_uri =>
            {
                text_elements.push(index);
            }
            PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => {}
            _ => return Err(PreservationError::UnsupportedTextMapping),
        }
    }

    if expected_runs.is_empty() {
        for index in &text_elements {
            let PreservedNode::Element(text_element) = &paragraph.children[*index] else {
                return Err(PreservationError::UnsupportedTextMapping);
            };
            if !text_element.attributes.is_empty()
                || text_element
                    .children
                    .iter()
                    .any(|child| !matches!(child, PreservedNode::Text(text) if text.is_empty()))
            {
                return Err(PreservationError::UnsupportedTextMapping);
            }
        }
        return Ok(text_elements);
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
                PreservedNode::Text(text) | PreservedNode::CData(text) => actual.push_str(text),
                _ => return Err(PreservationError::UnsupportedTextMapping),
            }
        }
        if &actual != expected {
            return Err(PreservationError::ManagedTextMismatchAt(
                ManagedTextMismatchSite::TextElementsReplacement,
            ));
        }
    }
    Ok(text_elements)
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

fn find_managed_element_counted<'a>(
    sections: &'a [PreservedSection],
    id: &StableId,
) -> Option<&'a PreservedElement> {
    fn find<'a>(nodes: &'a [PreservedNode], id: &StableId) -> Option<&'a PreservedElement> {
        for node in nodes {
            increment_counter(&MANAGED_INDEX_FALLBACK_NODES);
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

fn element_at_path<'a>(nodes: &'a [PreservedNode], path: &[usize]) -> Option<&'a PreservedElement> {
    let (index, rest) = path.split_first()?;
    let PreservedNode::Element(element) = nodes.get(*index)? else {
        return None;
    };
    if rest.is_empty() {
        Some(element)
    } else {
        element_at_path(&element.children, rest)
    }
}

fn is_exact_dual_dialogue_member_location(
    sections: &[PreservedSection],
    location: &ManagedParagraphLocation,
    group_id: &StableId,
) -> bool {
    let [root_index, content_index, wrapper_index, dual_index, paragraph_index] =
        location.node_path.as_slice() else { return false };
    let Some(section) = sections.get(location.section_index) else { return false };
    let Some(PreservedNode::Element(root)) = section.children.get(*root_index) else { return false };
    let Some(PreservedNode::Element(content)) = root.children.get(*content_index) else { return false };
    let Some(PreservedNode::Element(wrapper)) = content.children.get(*wrapper_index) else { return false };
    let Some(PreservedNode::Element(dual)) = wrapper.children.get(*dual_index) else { return false };
    let Some(PreservedNode::Element(paragraph)) = dual.children.get(*paragraph_index) else { return false };
    let dual_count = wrapper.children.iter().filter(|node| matches!(node,
        PreservedNode::Element(element) if element.name.local_name == "DualDialogue"
            && element.name.namespace_uri == wrapper.name.namespace_uri
    )).count();
    root.name.local_name == "FinalDraft"
        && content.name.local_name == "Content"
        && root.name.namespace_uri == content.name.namespace_uri
        && wrapper.name.local_name == "Paragraph"
        && wrapper.name.namespace_uri == content.name.namespace_uri
        && matches!(&wrapper.ownership, SourceOwnership::DualDialogueWrapper(owner) if owner == group_id)
        && dual.name.local_name == "DualDialogue"
        && dual.name.namespace_uri == wrapper.name.namespace_uri
        && dual_count == 1
        && paragraph.name.local_name == "Paragraph"
        && paragraph.name.namespace_uri == dual.name.namespace_uri
        && matches!(&paragraph.ownership, SourceOwnership::Managed(_))
}

fn indexed_element_context(mut nodes: &[PreservedNode], path: &[usize]) -> Option<(usize, bool)> {
    let mut inherited_space_preserve = false;
    for index in path {
        let PreservedNode::Element(element) = nodes.get(*index)? else {
            return None;
        };
        inherited_space_preserve = effective_xml_space_preserve(element, inherited_space_preserve);
        nodes = &element.children;
    }
    (!path.is_empty()).then_some((path.len().saturating_sub(1), inherited_space_preserve))
}

fn effective_xml_space_preserve(element: &PreservedElement, inherited: bool) -> bool {
    element
        .attributes
        .iter()
        .find(|attribute| {
            attribute.name.local_name == "space"
                && (attribute.name.namespace_uri.as_deref()
                    == Some("http://www.w3.org/XML/1998/namespace")
                    || attribute.name.qualified_name.as_deref() == Some("xml:space"))
        })
        .map_or(inherited, |attribute| match attribute.value.as_str() {
            "default" => false,
            "preserve" => true,
            _ => true,
        })
}

fn find_managed_context_counted(
    sections: &[PreservedSection],
    id: &StableId,
) -> Option<(usize, bool)> {
    fn visit(
        nodes: &[PreservedNode],
        id: &StableId,
        depth: usize,
        inherited_space_preserve: bool,
    ) -> Option<(usize, bool)> {
        for node in nodes {
            increment_counter(&MANAGED_INDEX_FALLBACK_NODES);
            let PreservedNode::Element(element) = node else {
                continue;
            };
            let space_preserve = effective_xml_space_preserve(element, inherited_space_preserve);
            if matches!(&element.ownership, SourceOwnership::Managed(target) if target == id) {
                return Some((depth, space_preserve));
            }
            if let Some(found) = visit(
                &element.children,
                id,
                depth.saturating_add(1),
                space_preserve,
            ) {
                return Some(found);
            }
        }
        None
    }

    sections
        .iter()
        .find_map(|section| visit(&section.children, id, 0, false))
}

fn element_at_path_mut<'a>(
    nodes: &'a mut [PreservedNode],
    path: &[usize],
) -> Option<&'a mut PreservedElement> {
    let (index, rest) = path.split_first()?;
    let PreservedNode::Element(element) = nodes.get_mut(*index)? else {
        return None;
    };
    if rest.is_empty() {
        Some(element)
    } else {
        element_at_path_mut(&mut element.children, rest)
    }
}

fn same_sections_except_managed_location(
    before: &[PreservedSection],
    after: &[PreservedSection],
    location: &ManagedParagraphLocation,
) -> bool {
    before.len() == after.len()
        && before
            .iter()
            .zip(after)
            .enumerate()
            .all(|(index, (left, right))| {
                left.name == right.name
                    && left.kind == right.kind
                    && left.ordinal == right.ordinal
                    && if index == location.section_index {
                        same_nodes_except_path(&left.children, &right.children, &location.node_path)
                    } else {
                        left.children.shares_storage_with(&right.children)
                    }
            })
}

fn same_nodes_except_path(
    before: &[PreservedNode],
    after: &[PreservedNode],
    path: &[usize],
) -> bool {
    if before.len() != after.len() {
        return false;
    }
    let Some((target_index, remaining_path)) = path.split_first() else {
        return before == after;
    };
    if *target_index >= before.len() {
        return false;
    }
    before
        .iter()
        .zip(after)
        .enumerate()
        .all(|(index, (left, right))| {
            if index != *target_index {
                return same_preserved_node_shallow(left, right);
            }
            if remaining_path.is_empty() {
                return matches!(
                    (left, right),
                    (PreservedNode::Element(_), PreservedNode::Element(_))
                );
            }
            match (left, right) {
                (PreservedNode::Element(left), PreservedNode::Element(right)) => {
                    left.name == right.name
                        && left.attributes == right.attributes
                        && left.namespace_bindings == right.namespace_bindings
                        && left.source_type == right.source_type
                        && left.ownership == right.ownership
                        && left.dependency_confidence == right.dependency_confidence
                        && left.anchor == right.anchor
                        && same_nodes_except_path(&left.children, &right.children, remaining_path)
                }
                _ => false,
            }
        })
}

fn same_preserved_node_shallow(before: &PreservedNode, after: &PreservedNode) -> bool {
    match (before, after) {
        (PreservedNode::Text(left), PreservedNode::Text(right))
        | (PreservedNode::CData(left), PreservedNode::CData(right))
        | (PreservedNode::Comment(left), PreservedNode::Comment(right)) => {
            left.shares_storage_with(right)
        }
        (
            PreservedNode::ProcessingInstruction {
                target: left_target,
                data: left_data,
            },
            PreservedNode::ProcessingInstruction {
                target: right_target,
                data: right_data,
            },
        ) => {
            left_target.shares_storage_with(right_target)
                && left_data.shares_storage_with(right_data)
        }
        (PreservedNode::Element(left), PreservedNode::Element(right)) => {
            left.name == right.name
                && left.attributes == right.attributes
                && left.namespace_bindings == right.namespace_bindings
                && left.source_type == right.source_type
                && left.ownership == right.ownership
                && left.dependency_confidence == right.dependency_confidence
                && left.anchor == right.anchor
                && left.children.shares_storage_with(&right.children)
        }
        _ => false,
    }
}

fn same_managed_edit_shape(
    before: &PreservedElement,
    after: &PreservedElement,
    managed_root: bool,
) -> bool {
    let same_attributes = before.attributes.len() == after.attributes.len()
        && before
            .attributes
            .iter()
            .zip(&after.attributes)
            .all(|(left, right)| {
                left.name == right.name
                    && (left.value == right.value
                        || (managed_root
                            && left.name.namespace_uri.is_none()
                            && left.name.local_name == "Type"))
            });
    before.name == after.name
        && before.namespace_bindings == after.namespace_bindings
        && before.ownership == after.ownership
        && before.dependency_confidence == after.dependency_confidence
        && before.anchor == after.anchor
        && (before.source_type == after.source_type || managed_root)
        && same_attributes
        && before.children.len() == after.children.len()
        && before
            .children
            .iter()
            .zip(&after.children)
            .all(|(left, right)| match (left, right) {
                (PreservedNode::Text(_), PreservedNode::Text(_))
                | (PreservedNode::CData(_), PreservedNode::CData(_)) => true,
                (PreservedNode::Comment(left), PreservedNode::Comment(right)) => left == right,
                (
                    PreservedNode::ProcessingInstruction {
                        target: left_target,
                        data: left_data,
                    },
                    PreservedNode::ProcessingInstruction {
                        target: right_target,
                        data: right_data,
                    },
                ) => left_target == right_target && left_data == right_data,
                (PreservedNode::Element(left), PreservedNode::Element(right)) => {
                    same_managed_edit_shape(left, right, false)
                }
                _ => false,
            })
}

fn source_text_bytes_in_element(element: &PreservedElement) -> usize {
    let mut total = 0usize;
    let mut stack = vec![element];
    while let Some(current) = stack.pop() {
        for child in &current.children {
            match child {
                PreservedNode::Element(nested) => stack.push(nested),
                PreservedNode::Text(text)
                | PreservedNode::CData(text)
                | PreservedNode::Comment(text) => total = total.saturating_add(text.len()),
                PreservedNode::ProcessingInstruction { data, .. } => {
                    total = total.saturating_add(data.len())
                }
            }
        }
    }
    total
}

fn remove_managed_at_location(
    sections: &mut [PreservedSection],
    location: &ManagedParagraphLocation,
) -> bool {
    let Some((&target_index, parent_path)) = location.node_path.split_last() else {
        return false;
    };
    let Some(section) = sections.get_mut(location.section_index) else {
        return false;
    };
    let parent_nodes = if parent_path.is_empty() {
        &mut section.children
    } else {
        let Some(parent) = element_at_path_mut(&mut section.children, parent_path) else {
            return false;
        };
        &mut parent.children
    };
    if target_index >= parent_nodes.len() {
        return false;
    }
    parent_nodes.remove(target_index);
    true
}

fn contains_managed_element_counted(nodes: &[PreservedNode], id: &StableId) -> bool {
    for node in nodes {
        increment_counter(&MANAGED_INDEX_FALLBACK_NODES);
        let PreservedNode::Element(element) = node else {
            continue;
        };
        if matches!(&element.ownership, SourceOwnership::Managed(target) if target == id)
            || contains_managed_element_counted(&element.children, id)
        {
            return true;
        }
    }
    false
}

fn is_text_element(element: &PreservedElement, paragraph: &PreservedElement) -> bool {
    element.name.local_name == "Text" && element.name.namespace_uri == paragraph.name.namespace_uri
}

fn is_title_content_location(
    sections: &[PreservedSection],
    location: &ManagedParagraphLocation,
) -> bool {
    let [title_index, content_index, paragraph_index] = location.node_path.as_slice() else {
        return false;
    };
    if sections.get(location.section_index).is_none() {
        return false;
    }
    let Some(PreservedNode::Element(title)) = sections[location.section_index].children.get(*title_index) else {
        return false;
    };
    let Some(PreservedNode::Element(content)) = title.children.get(*content_index) else {
        return false;
    };
    let Some(PreservedNode::Element(paragraph)) = content.children.get(*paragraph_index) else {
        return false;
    };
    title.name.local_name == "TitlePage"
        && content.name.local_name == "Content"
        && paragraph.name.local_name == "Paragraph"
        && title.name.namespace_uri == content.name.namespace_uri
        && content.name.namespace_uri == paragraph.name.namespace_uri
}

fn title_content_parent_mut<'a>(
    sections: &'a mut [PreservedSection],
    location: &ManagedParagraphLocation,
) -> Result<&'a mut PreservedElement, PreservationError> {
    if location.node_path.len() != 3 {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    let section = sections.get_mut(location.section_index)
        .ok_or(PreservationError::ManagedAnchorMissing)?;
    let PreservedNode::Element(title) = section.children.get_mut(location.node_path[0])
        .ok_or(PreservationError::ManagedAnchorMissing)? else {
        return Err(PreservationError::UnsupportedTextMapping);
    };
    let PreservedNode::Element(content) = title.children.get_mut(location.node_path[1])
        .ok_or(PreservationError::ManagedAnchorMissing)? else {
        return Err(PreservationError::UnsupportedTextMapping);
    };
    if title.name.local_name != "TitlePage"
        || content.name.local_name != "Content"
        || title.name.namespace_uri != content.name.namespace_uri
        || !matches!(content.children.get(location.node_path[2]), Some(PreservedNode::Element(paragraph)) if paragraph.name.local_name == "Paragraph" && paragraph.name.namespace_uri == content.name.namespace_uri)
    {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    Ok(content)
}

fn paragraph_id_attribute_count(paragraph: &PreservedElement) -> usize {
    paragraph.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none()
            && attribute.name.local_name.eq_ignore_ascii_case("id")
    }).count()
}

fn paragraph_attributes_without_id(paragraph: &PreservedElement) -> Vec<&PreservedAttribute> {
    paragraph.attributes.iter().filter(|attribute| {
        !(attribute.name.namespace_uri.is_none()
            && attribute.name.local_name.eq_ignore_ascii_case("id"))
    }).collect()
}

fn same_title_paragraph_shell(left: &PreservedElement, right: &PreservedElement) -> bool {
    left.name == right.name
        && left.namespace_bindings == right.namespace_bindings
        && left.source_type == right.source_type
        && paragraph_attributes_without_id(left) == paragraph_attributes_without_id(right)
}

fn source_attribute_matches_run(attribute: &PreservedAttribute, run: &crate::RunAttribute) -> bool {
    attribute.name.qualified_name.as_deref()
        .unwrap_or(attribute.name.local_name.as_str()) == run.name
        && attribute.value == run.value
}

fn title_effective_runs(semantic: &TitlePageParagraph) -> Vec<TextRun> {
    if semantic.runs.is_empty() {
        if semantic.text.is_empty() {
            Vec::new()
        } else {
            vec![TextRun { text: semantic.text.clone(), attributes: Vec::new() }]
        }
    } else {
        semantic.runs.clone()
    }
}

fn title_text_templates(
    paragraph: &PreservedElement,
) -> Result<Vec<PreservedElement>, PreservationError> {
    let mut templates = Vec::new();
    for child in &paragraph.children {
        match child {
            PreservedNode::Element(element) if is_text_element(element, paragraph) => {
                if element.children.iter().any(|content| {
                    !matches!(content, PreservedNode::Text(_) | PreservedNode::CData(_))
                }) {
                    return Err(PreservationError::UnsupportedRunTopology);
                }
                templates.push(element.clone());
            }
            PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => {}
            _ => return Err(PreservationError::StructuralAttachmentConflict),
        }
    }
    Ok(templates)
}

fn validate_title_text_shape(
    source: &PreservedElement,
    semantic: &TitlePageParagraph,
) -> Result<(), PreservationError> {
    if !is_plain_managed_paragraph(source) {
        return Err(PreservationError::StructuralAttachmentConflict);
    }
    let templates = title_text_templates(source)?;
    let source_text = templates.iter().flat_map(|template| template.children.iter()).filter_map(|child| {
        match child {
            PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
            _ => None,
        }
    }).collect::<String>();
    if source_text != semantic.text {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    let runs = title_effective_runs(semantic);
    if runs.iter().map(|run| run.text.as_str()).collect::<String>() != semantic.text {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    if semantic.text.is_empty() {
        if !runs.is_empty() {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        if !templates.is_empty() {
            // Empty source Text elements can carry authored run attributes or
            // namespace state with no semantic run slot to transfer safely.
            return Err(PreservationError::UnsupportedRunTopology);
        }
        return Ok(());
    }
    if templates.len() != runs.len() {
        return Err(PreservationError::UnsupportedRunTopology);
    }
    for (template, run) in templates.iter().zip(&runs) {
        let text = template.children.iter().filter_map(|child| match child {
            PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
            _ => None,
        }).collect::<String>();
        if text != run.text
            || template.attributes.len() != run.attributes.len()
            || !template.attributes.iter().zip(&run.attributes)
                .all(|(source, semantic)| source_attribute_matches_run(source, semantic))
        {
            return Err(PreservationError::UnsupportedTextMapping);
        }
    }
    Ok(())
}

fn partition_title_children(
    source: &PreservedElement,
    original: &TitlePageParagraph,
    left: &TitlePageParagraph,
    right: &TitlePageParagraph,
) -> Result<(Vec<PreservedNode>, Vec<PreservedNode>), PreservationError> {
    if format!("{}{}", left.text, right.text) != original.text
        || left.text.len() > original.text.len()
        || !original.text.is_char_boundary(left.text.len())
        || left.runs.iter().map(|run| run.text.as_str()).collect::<String>() != left.text
        || right.runs.iter().map(|run| run.text.as_str()).collect::<String>() != right.text
    {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    validate_title_text_shape(source, original)?;
    let source_runs = title_effective_runs(original);
    let mut left_children = Vec::new();
    let mut right_children = Vec::new();
    let mut cursor = 0usize;
    let mut run_index = 0usize;
    for child in &source.children {
        match child {
            PreservedNode::Element(element) if is_text_element(element, source) => {
                let content = element.children.iter().filter_map(|node| match node {
                    PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
                    _ => None,
                }).collect::<String>();
                if content.is_empty() {
                    left_children.push(child.clone());
                    continue;
                }
                let run = source_runs.get(run_index).ok_or(PreservationError::UnsupportedTextMapping)?;
                if run.text != content {
                    return Err(PreservationError::UnsupportedTextMapping);
                }
                run_index += 1;
                let end = cursor.saturating_add(content.len());
                if end <= left.text.len() {
                    left_children.push(child.clone());
                } else if cursor >= left.text.len() {
                    right_children.push(child.clone());
                } else {
                    let split = left.text.len().saturating_sub(cursor);
                    let (left_text, right_text) = split_title_text_element(element, split)?;
                    if let Some(element) = left_text {
                        left_children.push(PreservedNode::Element(element));
                    }
                    if let Some(element) = right_text {
                        right_children.push(PreservedNode::Element(element));
                    }
                }
                cursor = end;
            }
            PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => {
                // Formatting whitespace remains at its original source position
                // with the retained left occurrence; no opaque payload is moved.
                left_children.push(child.clone());
            }
            _ => return Err(PreservationError::StructuralAttachmentConflict),
        }
    }
    if cursor != original.text.len() || run_index != source_runs.len() {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    let emitted_left = left_children.iter().flat_map(|node| match node {
        PreservedNode::Element(element) if is_text_element(element, source) => element.children.iter().collect::<Vec<_>>(),
        _ => Vec::new(),
    }).filter_map(|node| match node {
        PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
        _ => None,
    }).collect::<String>();
    let emitted_right = right_children.iter().flat_map(|node| match node {
        PreservedNode::Element(element) if is_text_element(element, source) => element.children.iter().collect::<Vec<_>>(),
        _ => Vec::new(),
    }).filter_map(|node| match node {
        PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
        _ => None,
    }).collect::<String>();
    if emitted_left != left.text || emitted_right != right.text {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    Ok((left_children, right_children))
}

fn split_title_text_element(
    source: &PreservedElement,
    split: usize,
) -> Result<(Option<PreservedElement>, Option<PreservedElement>), PreservationError> {
    let total = source.children.iter().map(|node| match node {
        PreservedNode::Text(text) | PreservedNode::CData(text) => text.len(),
        _ => 0,
    }).sum::<usize>();
    if split == 0 {
        return Ok((None, Some(source.clone())));
    }
    if split == total {
        return Ok((Some(source.clone()), None));
    }
    if split > total || !source.children.iter().all(|node| matches!(node, PreservedNode::Text(_) | PreservedNode::CData(_))) {
        return Err(PreservationError::UnsupportedRunTopology);
    }
    let mut left = source.clone();
    let mut right = source.clone();
    let mut left_nodes = Vec::new();
    let mut right_nodes = Vec::new();
    let mut cursor = 0usize;
    for node in &source.children {
        let (text, cdata) = match node {
            PreservedNode::Text(text) => (text.as_str(), false),
            PreservedNode::CData(text) => (text.as_str(), true),
            _ => unreachable!(),
        };
        if text.is_empty() {
            return Err(PreservationError::UnsupportedRunTopology);
        }
        let end = cursor.saturating_add(text.len());
        if end <= split {
            left_nodes.push(node.clone());
        } else if cursor >= split {
            right_nodes.push(node.clone());
        } else {
            let local = split.saturating_sub(cursor);
            let (left_text, right_text) = text.split_at(local);
            if cdata {
                left_nodes.push(PreservedNode::CData(left_text.to_owned().into()));
                right_nodes.push(PreservedNode::CData(right_text.to_owned().into()));
            } else {
                left_nodes.push(PreservedNode::Text(left_text.to_owned().into()));
                right_nodes.push(PreservedNode::Text(right_text.to_owned().into()));
            }
        }
        cursor = end;
    }
    left.children = left_nodes.into();
    right.children = right_nodes.into();
    Ok((Some(left), Some(right)))
}

fn set_paragraph_foreign_id(
    paragraph: &mut PreservedElement,
    value: &str,
) -> Result<(), PreservationError> {
    let indexes = paragraph.attributes.iter().enumerate().filter(|(_, attribute)| {
        attribute.name.namespace_uri.is_none()
            && attribute.name.local_name.eq_ignore_ascii_case("id")
    }).map(|(index, _)| index).collect::<Vec<_>>();
    match indexes.as_slice() {
        [index] => paragraph.attributes[*index].value = value.to_owned(),
        [] => paragraph.attributes.push(PreservedAttribute {
            name: SourceName {
                namespace_uri: None,
                local_name: "ID".to_owned(),
                qualified_name: Some("ID".to_owned()),
            },
            value: value.to_owned(),
        }),
        _ => return Err(PreservationError::AmbiguousDirectAttribute),
    }
    Ok(())
}

fn title_text_children_from_templates(
    recipient_source: &PreservedElement,
    templates: &[PreservedElement],
    semantic: &TitlePageParagraph,
) -> Result<Vec<PreservedNode>, PreservationError> {
    let runs = title_effective_runs(semantic);
    let mut rebuilt = Vec::with_capacity(recipient_source.children.len().saturating_add(runs.len()));
    let mut inserted = false;
    for child in &recipient_source.children {
        match child {
            PreservedNode::Element(element) if is_text_element(element, recipient_source) => {
                if !inserted {
                    for run in &runs {
                        let template = templates.iter().find(|template| {
                            template.attributes.len() == run.attributes.len()
                                && template.attributes.iter().zip(&run.attributes)
                                    .all(|(source, semantic)| source_attribute_matches_run(source, semantic))
                        }).ok_or(PreservationError::UnsupportedTextMapping)?;
                        let mut text_element = template.clone();
                        if text_element.children.iter().any(|node| {
                            !matches!(node, PreservedNode::Text(_) | PreservedNode::CData(_))
                        }) {
                            return Err(PreservationError::UnsupportedRunTopology);
                        }
                        if let Some(first) = text_element.children.first_mut() {
                            match first {
                                PreservedNode::Text(text) | PreservedNode::CData(text) => {
                                    **text = run.text.clone();
                                }
                                _ => unreachable!(),
                            }
                            for child in text_element.children.iter_mut().skip(1) {
                                match child {
                                    PreservedNode::Text(text) | PreservedNode::CData(text) => text.clear(),
                                    _ => unreachable!(),
                                }
                            }
                        } else {
                            text_element.children.push(PreservedNode::Text(run.text.clone().into()));
                        }
                        rebuilt.push(PreservedNode::Element(text_element));
                    }
                    inserted = true;
                }
            }
            PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => {
                rebuilt.push(child.clone());
            }
            _ => return Err(PreservationError::StructuralAttachmentConflict),
        }
    }
    if !inserted {
        for run in &runs {
            let template = templates.iter().find(|template| {
                template.attributes.len() == run.attributes.len()
                    && template.attributes.iter().zip(&run.attributes)
                        .all(|(source, semantic)| source_attribute_matches_run(source, semantic))
            }).ok_or(PreservationError::UnsupportedTextMapping)?;
            let mut text_element = template.clone();
            if text_element.children.iter().any(|node| {
                !matches!(node, PreservedNode::Text(_) | PreservedNode::CData(_))
            }) {
                return Err(PreservationError::UnsupportedRunTopology);
            }
            if let Some(first) = text_element.children.first_mut() {
                match first {
                    PreservedNode::Text(text) | PreservedNode::CData(text) => {
                        **text = run.text.clone();
                    }
                    _ => unreachable!(),
                }
                for child in text_element.children.iter_mut().skip(1) {
                    match child {
                        PreservedNode::Text(text) | PreservedNode::CData(text) => text.clear(),
                        _ => unreachable!(),
                    }
                }
            } else {
                text_element.children.push(PreservedNode::Text(run.text.clone().into()));
            }
            rebuilt.push(PreservedNode::Element(text_element));
        }
    }
    if runs.iter().map(|run| run.text.as_str()).collect::<String>() != semantic.text {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    Ok(rebuilt)
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

fn paragraph_text_nodes_with_direct_alts(
    paragraph: &PreservedElement,
) -> Result<Vec<PreservedNode>, PreservationError> {
    let mut alts_count = 0usize;
    let mut text_nodes = Vec::new();
    for node in &paragraph.children {
        match node {
            PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => {}
            PreservedNode::Element(element)
                if element.name.local_name == "Text"
                    && element.name.namespace_uri == paragraph.name.namespace_uri
                    && element.children.iter().all(|child| matches!(
                        child,
                        PreservedNode::Text(_) | PreservedNode::CData(_)
                    )) => text_nodes.push(node.clone()),
            PreservedNode::Element(element)
                if element.name.local_name == "Alts"
                    && element.name.namespace_uri == paragraph.name.namespace_uri =>
            {
                alts_count = alts_count.saturating_add(1);
                if alts_count > 1 {
                    return Err(PreservationError::StructuralAttachmentConflict);
                }
            }
            _ => return Err(PreservationError::StructuralAttachmentConflict),
        }
    }
    Ok(text_nodes)
}

fn text_of_nodes(nodes: &[PreservedNode]) -> String {
    nodes.iter().filter_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "Text" => Some(
            element.children.iter().filter_map(|child| match child {
                PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
                _ => None,
            }).collect::<String>(),
        ),
        _ => None,
    }).fold(String::new(), |mut value, text| {
        value.push_str(&text);
        value
    })
}

fn slice_source_text_nodes(
    nodes: &[PreservedNode],
    start: usize,
) -> Result<Vec<PreservedNode>, PreservationError> {
    let total = text_of_nodes(nodes);
    if start > total.len() || !total.is_char_boundary(start) {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    let mut offset = 0usize;
    let mut result = Vec::new();
    for node in nodes {
        let PreservedNode::Element(original) = node else { continue };
        let segment = original.children.iter().filter_map(|child| match child {
            PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
            _ => None,
        }).collect::<String>();
        let segment_end = offset.saturating_add(segment.len());
        if segment_end > start {
            let local_start = start.saturating_sub(offset);
            if !segment.is_char_boundary(local_start) {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            let mut clone = original.clone();
            let mut child_offset = 0usize;
            let mut children = Vec::new();
            for child in &original.children {
                match child {
                    PreservedNode::Text(text) | PreservedNode::CData(text) => {
                        let child_end = child_offset.saturating_add(text.len());
                        let keep_from = local_start.max(child_offset);
                        if child_end > keep_from {
                            let local_from = keep_from - child_offset;
                            if !text.is_char_boundary(local_from) {
                                return Err(PreservationError::UnsupportedTextMapping);
                            }
                            let sliced = &text[local_from..];
                            children.push(match child {
                                PreservedNode::Text(_) => PreservedNode::Text(sliced.to_owned().into()),
                                PreservedNode::CData(_) => PreservedNode::CData(sliced.to_owned().into()),
                                _ => return Err(PreservationError::UnsupportedTextMapping),
                            });
                        }
                        child_offset = child_end;
                    }
                    _ => return Err(PreservationError::UnsupportedTextMapping),
                }
            }
            let retained_text = children.iter().filter_map(|child| match child {
                PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
                _ => None,
            }).collect::<String>();
            clone.children = children.into();
            if !retained_text.is_empty() {
                result.push(PreservedNode::Element(clone));
            }
        }
        offset = segment_end;
    }
    Ok(result)
}

fn is_managed_paragraph_with_direct_alts(paragraph: &PreservedElement) -> bool {
    if paragraph.name.local_name != "Paragraph" {
        return false;
    }
    let mut alts_count = 0usize;
    for node in &paragraph.children {
        match node {
            PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => {}
            PreservedNode::Element(element)
                if element.name.local_name == "Text"
                    && element.name.namespace_uri == paragraph.name.namespace_uri
                    && element.children.iter().all(|child| matches!(
                        child,
                        PreservedNode::Text(_) | PreservedNode::CData(_)
                    )) => {}
            PreservedNode::Element(element)
                if element.name.local_name == "Alts"
                    && element.name.namespace_uri == paragraph.name.namespace_uri =>
            {
                alts_count = alts_count.saturating_add(1);
            }
            _ => return false,
        }
    }
    alts_count == 1
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

fn slice_text_runs(
    runs: &[TextRun],
    range: std::ops::Range<usize>,
) -> Result<Vec<TextRun>, PreservationError> {
    let total = runs.iter().map(|run| run.text.len()).sum::<usize>();
    if range.start > range.end || range.end > total {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    let mut result = Vec::new();
    let mut offset = 0usize;
    for run in runs {
        let end = offset.saturating_add(run.text.len());
        let start = range.start.max(offset);
        let finish = range.end.min(end);
        if start < finish {
            let from = start - offset;
            let to = finish - offset;
            if !run.text.is_char_boundary(from) || !run.text.is_char_boundary(to) {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            result.push(TextRun {
                text: run.text[from..to].to_owned(),
                attributes: run.attributes.clone(),
            });
        }
        offset = end;
    }
    if range.is_empty() && total == 0 {
        if let Some(run) = runs.first() {
            result.push(run.clone());
        }
    }
    Ok(result)
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
    let existing_text = element.children.iter().filter_map(|child| match child {
        PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
        _ => None,
    }).collect::<String>();
    if existing_text != run.text {
        element.children = vec![PreservedNode::Text(run.text.clone().into())].into();
    }
    element
}

fn sync_text_element_attributes(
    template: &PreservedElement,
    run: &TextRun,
) -> Result<PreservedElement, PreservationError> {
    let mut element = template.clone();
    for source in &mut element.attributes {
        let source_name = source.name.qualified_name.as_deref()
            .unwrap_or(source.name.local_name.as_str());
        let Some(attribute) = run.attributes.iter().find(|attribute| attribute.name == source_name) else {
            return Err(PreservationError::UnsupportedTextMapping);
        };
        source.value.clone_from(&attribute.value);
    }
    for attribute in &run.attributes {
        let found = element.attributes.iter().any(|source| {
            source.name.qualified_name.as_deref()
                .unwrap_or(source.name.local_name.as_str()) == attribute.name
        });
        if !found {
            if attribute.name != "Style" {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            element.attributes.push(PreservedAttribute {
                name: plain_source_name("Style"),
                value: attribute.value.clone(),
            });
        }
    }
    element.children = vec![PreservedNode::Text(run.text.clone().into())].into();
    Ok(element)
}

fn has_potential_reference(
    sections: &[PreservedSection],
    value: &str,
    excluded_element: &StableId,
) -> bool {
    fn scan(
        nodes: &[PreservedNode],
        value: &str,
        excluded: &StableId,
        parent: Option<&PreservedElement>,
        grandparent: Option<&PreservedElement>,
    ) -> bool {
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
            let exact_alternate_copy = element.name.local_name == "Paragraph"
                && parent.is_some_and(|parent| {
                    parent.name.local_name == "Alt"
                        && parent.name.namespace_uri == element.name.namespace_uri
                })
                && grandparent.is_some_and(|grandparent| {
                    grandparent.name.local_name == "AltCollection"
                        && grandparent.name.namespace_uri == element.name.namespace_uri
                });
            element.attributes.iter().any(|attribute| {
                !((is_excluded || exact_alternate_copy)
                    && attribute.name.namespace_uri.is_none()
                    && attribute.name.local_name.eq_ignore_ascii_case("id"))
                    && attribute.value == value
            }) || scan(&element.children, value, excluded, Some(element), parent)
        })
    }
    sections
        .iter()
        .any(|section| scan(&section.children, value, excluded_element, None, None))
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
        if inserted.is_empty() {
            return Ok(());
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
                children: SharedVec::default(),
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
    if updated.iter().all(String::is_empty) {
        if indexes.iter().any(|index| {
            matches!(&paragraph.children[*index], PreservedNode::Element(element)
                if element.children.iter().any(|child| matches!(child, PreservedNode::CData(_))))
        }) {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let indexes = indexes.into_iter().collect::<BTreeSet<_>>();
        paragraph.children = paragraph
            .children
            .drain(..)
            .enumerate()
            .filter_map(|(index, child)| (!indexes.contains(&index)).then_some(child))
            .collect();
        return Ok(());
    }
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
                *text = replacement.to_owned().into();
            }
            _ => return Err(PreservationError::UnsupportedTextMapping),
        }
        for child in element.children.iter_mut().skip(1) {
            match child {
                PreservedNode::Text(text) | PreservedNode::CData(text) => text.clear(),
                _ => return Err(PreservationError::UnsupportedTextMapping),
            }
        }
    } else if !replacement.is_empty() {
        element
            .children
            .push(PreservedNode::Text(replacement.to_owned().into()));
    }
    Ok(())
}

fn direct_text_child_indexes(element: &PreservedElement) -> Vec<usize> {
    element
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            // Whitespace-only direct text around generated scene containers is
            // element-only XML formatting, not part of the screenplay text
            // projection. Keep those nodes in place, but do not compare or edit
            // them as semantic text runs.
            matches!(node, PreservedNode::Text(text) | PreservedNode::CData(text) if !text.trim().is_empty())
                .then_some(index)
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
        SourceOwnership::Opaque | SourceOwnership::Managed(_) | SourceOwnership::DualDialogueWrapper(_) => Ok(()),
        SourceOwnership::Attached(identity) => validate_identity(identity),
    }
}

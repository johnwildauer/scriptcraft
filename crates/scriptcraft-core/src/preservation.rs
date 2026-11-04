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
use crate::paragraph_id::{ParagraphIdAllocationError, ParagraphIdAllocator};
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
    let _ = counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
        Some(value.saturating_add(1))
    });
}

fn add_counter(counter: &AtomicU64, amount: u64) {
    if amount == 0 {
        return;
    }
    let _ = counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
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
        Self { bytes: Arc::new(bytes), image_payload: false }
    }

    pub fn image(bytes: Vec<u8>) -> Self {
        Self { bytes: Arc::new(bytes), image_payload: true }
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
        Self { bytes: Arc::clone(&self.bytes), image_payload: self.image_payload }
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
        PreservationError::ManagedTextMismatch => {
            PreservationError::ManagedTextMismatchAt(site)
        }
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
                automaton.nodes[child].output_link = if automaton.nodes[fallback].outputs.is_empty() {
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

        let patterns = index.paragraph_identity_counts.keys().cloned().collect::<Vec<_>>();
        let automaton = SubstringAutomaton::new(patterns);
        let mut visited = 0u64;
        fn collect(
            nodes: &[PreservedNode],
            section_index: usize,
            path: &mut Vec<usize>,
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
                        if let SourceOwnership::Managed(id) = &element.ownership {
                            managed_index
                                .entry(id.clone())
                                .or_default()
                                .push(ManagedParagraphLocation {
                                    section_index,
                                    node_path: path.clone(),
                                });
                        }
                        let mut unqualified_ids = element.attributes.iter().filter(|attribute| {
                            attribute.name.namespace_uri.is_none()
                                && attribute.name.local_name.eq_ignore_ascii_case("id")
                        });
                        let unique_id = unqualified_ids.next().map(|attribute| attribute.value.as_str())
                            .filter(|_| unqualified_ids.next().is_none());
                        for attribute in &element.attributes {
                            if attribute.name.namespace_uri.is_none()
                                && attribute.name.local_name.eq_ignore_ascii_case("id")
                            {
                                index.source_id_values.insert(attribute.value.clone());
                            }
                            let exempt = (element.name.local_name == "Paragraph"
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
                        automaton.scan(
                            text,
                            None,
                            &mut index.externally_used_paragraph_ids,
                        );
                    }
                    PreservedNode::ProcessingInstruction { target, data } => {
                        *text_bytes = text_bytes.saturating_add(data.len());
                        automaton.scan(
                            target,
                            None,
                            &mut index.externally_used_paragraph_ids,
                        );
                        automaton.scan(
                            data,
                            None,
                            &mut index.externally_used_paragraph_ids,
                        );
                    }
                }
            }
        }
        for (section_index, section) in sections.iter().enumerate() {
            collect(
                &section.children,
                section_index,
                &mut Vec::new(),
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

impl PreservationState {
    pub fn source_layout_settings(&self) -> &SourceLayoutSettings {
        &self.source_layout_settings
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
        let section = self.sections.get(location.section_index)
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
        let location = self.indexed_location(id)?
            .cloned()
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let section = self.sections.get_mut(location.section_index)
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
        if baseline_location != candidate_location {
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
        let role_attribute_count = after.attributes.iter().filter(|attribute| {
            attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Type"
        }).count();
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
                PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => {}
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
        increment_counter(&MANAGED_INDEX_LOOKUPS);
        if let Some(index) = &self.managed_paragraph_index {
            let locations = index.get(id).map(Vec::as_slice).unwrap_or_default();
            add_counter(&MANAGED_INDEX_ENTRIES, locations.len() as u64);
            return !locations.is_empty();
        }
        self.sections.iter().any(|section| {
            contains_managed_element_counted(&section.children, id)
        })
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
            .map_err(|error| categorize_managed_text_mismatch(
                error,
                ManagedTextMismatchSite::TextReplacement,
            ));
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
            .map_err(|error| categorize_managed_text_mismatch(
                error,
                ManagedTextMismatchSite::TextReplacement,
            ));
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
            self.rebuild_managed_paragraph_index();
        }
        self.rebuild_managed_paragraph_index();
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
        let location = self
            .indexed_location(id)?
            .cloned()
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let foreign_id = {
            let target = self
            .managed_element_at(&location)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
            if !is_plain_managed_paragraph(target) {
                return Err(PreservationError::StructuralAttachmentConflict);
            }
            paragraph_foreign_id(target).map(str::to_owned)
        };
        if provenance.and_then(|item| item.foreign_id.as_deref()) != foreign_id.as_deref() {
            return Err(PreservationError::ManagedTextMismatchAt(
                ManagedTextMismatchSite::ParagraphRemoval,
            ));
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
            self.identities.retain(|identity| {
                !(identity.domain == ForeignIdDomain::Paragraph && identity.value == value)
            });
        }
        let removed = remove_managed_at_location(&mut self.sections, &location);
        if !removed {
            return Err(PreservationError::ManagedAnchorMissing);
        }
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
        element_at_path_mut(&mut section.children, &location.node_path)
            .ok_or(PreservationError::ManagedAnchorMissing)
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
                || text_element.children.iter().any(|child| {
                    !matches!(child, PreservedNode::Text(text) if text.is_empty())
                })
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

fn element_at_path<'a>(
    nodes: &'a [PreservedNode],
    path: &[usize],
) -> Option<&'a PreservedElement> {
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

fn indexed_element_context(
    mut nodes: &[PreservedNode],
    path: &[usize],
) -> Option<(usize, bool)> {
    let mut inherited_space_preserve = false;
    for index in path {
        let PreservedNode::Element(element) = nodes.get(*index)? else {
            return None;
        };
        inherited_space_preserve = effective_xml_space_preserve(
            element,
            inherited_space_preserve,
        );
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

    sections.iter().find_map(|section| visit(&section.children, id, 0, false))
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
        && before.iter().zip(after).enumerate().all(|(index, (left, right))| {
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
                return matches!((left, right), (PreservedNode::Element(_), PreservedNode::Element(_)));
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
                        && same_nodes_except_path(
                            &left.children,
                            &right.children,
                            remaining_path,
                        )
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
        && before.attributes.iter().zip(&after.attributes).all(|(left, right)| {
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
        && before.children.iter().zip(&after.children).all(|(left, right)| match (left, right) {
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
    element.children = vec![PreservedNode::Text(run.text.clone().into())].into();
    element
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
            original_bytes: SharedBytes::source(b"<Script/>".to_vec()),
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
            layout_identity: std::sync::Arc::new(()),
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
            ].into(),
        };
        PreservationState {
            source: snapshot(),
            source_layout_settings: SourceLayoutSettings::default(),
            sections: vec![PreservedSection {
                name: name("Content"),
                kind: SectionKind::Content,
                ordinal: 0,
                children: vec![PreservedNode::Element(paragraph)].into(),
            }].into(),
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
            managed_paragraph_index: None,
            source_identity_use_index: None,
            source_text_bytes: 0,
        }
    }

    #[test]
    fn managed_mismatch_sites_are_distinct_and_value_free() {
        let id = StableId::new("element").unwrap_or_else(|_| unreachable!());

        let mut text_replacement = base_state();
        let text_replacement_error = text_replacement
            .replace_managed_text(
                &id,
                "INT. OTHER",
                0,
                0,
                "",
                None,
            )
            .expect_err("semantic/source text disagreement must reject replacement");
        assert_eq!(
            text_replacement_error,
            PreservationError::ManagedTextMismatchAt(ManagedTextMismatchSite::TextReplacement),
        );

        let mut text_elements = base_state();
        let paragraph = text_elements
            .managed_element_mut(&id)
            .unwrap_or_else(|_| unreachable!());
        paragraph.children = vec![PreservedNode::Element(PreservedElement {
            name: name("Text"),
            attributes: Vec::new(),
            namespace_bindings: Vec::new(),
            source_type: None,
            ownership: SourceOwnership::Opaque,
            dependency_confidence: DependencyConfidence::Unknown,
            anchor: None,
            children: vec![PreservedNode::Text("SOURCE-ONLY-VALUE".into())].into(),
        })]
        .into();
        let text_elements_error = text_elements
            .replace_managed_text_elements(
                &id,
                &["EXPECTED-VALUE".to_owned()],
                "replacement",
            )
            .expect_err("source text runs must match the expected semantic runs");
        assert_eq!(
            text_elements_error,
            PreservationError::ManagedTextMismatchAt(
                ManagedTextMismatchSite::TextElementsReplacement,
            ),
        );

        let mut paragraph_removal = base_state();
        let provenance = crate::SourceProvenance {
            format: crate::SourceFormat::Fdx,
            section: crate::SourceSection::Content,
            ordinal: 0,
            foreign_id: Some("PRIVATE-FOREIGN-ID".to_owned()),
            foreign_role: None,
        };
        let removal_error = paragraph_removal
            .remove_managed_paragraph(&id, Some(&provenance))
            .expect_err("mismatched provenance must reject removal");
        assert_eq!(
            removal_error,
            PreservationError::ManagedTextMismatchAt(
                ManagedTextMismatchSite::ParagraphRemoval,
            ),
        );

        let baseline = base_state();
        let mut scoped_edit = baseline.clone();
        let paragraph = scoped_edit
            .managed_element_mut(&id)
            .unwrap_or_else(|_| unreachable!());
        paragraph.children[1] = PreservedNode::CData("SOURCE-ONLY-VALUE".into());
        let semantic = ScreenplayElement::new("element", Role::SceneHeading, "INT. ROOM")
            .unwrap_or_else(|_| unreachable!());
        let scoped_error = scoped_edit
            .validate_scoped_managed_edit(&baseline, &semantic)
            .expect_err("scoped validation must reject source projection disagreement");
        assert_eq!(
            scoped_error,
            PreservationError::ManagedTextMismatchAt(
                ManagedTextMismatchSite::ScopedEditValidation,
            ),
        );

        let value_free = format!(
            "{text_replacement_error:?} {text_elements_error:?} {removal_error:?} {scoped_error:?}"
        );
        assert!(!value_free.contains("SOURCE-ONLY-VALUE"));
        assert!(!value_free.contains("PRIVATE-FOREIGN-ID"));
        assert_eq!(
            ManagedTextMismatchSite::TextElementsReplacement.diagnostic_name(),
            "text-elements-replacement",
        );
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
    fn shared_storage_equality_keeps_deep_fallback_after_manual_restore() {
        let saved = SharedVec::from(vec![1_u8, 2, 3]);
        let mut candidate = saved.clone();
        assert_eq!(saved, candidate);

        candidate[1] = 9;
        assert_ne!(saved, candidate);
        candidate[1] = 2;

        assert!(!saved.shares_storage_with(&candidate));
        assert_eq!(saved, candidate);
    }

    #[test]
    fn copy_on_write_text_edit_shares_source_and_unrelated_subtrees() {
        let mut original = base_state();
        original.sections[0].children.push(PreservedNode::Element(PreservedElement {
            name: name("VendorPayload"),
            attributes: Vec::new(),
            namespace_bindings: Vec::new(),
            source_type: None,
            ownership: SourceOwnership::Opaque,
            dependency_confidence: DependencyConfidence::Unknown,
            anchor: None,
            children: vec![PreservedNode::Comment("opaque retained comment".into())].into(),
        }));
        let mut candidate = original.clone();
        let before_counters = copy_on_write_diagnostics();

        {
            assert!(original.source.original_bytes.shares_storage_with(&candidate.source.original_bytes));
            assert!(original.sections.shares_storage_with(&candidate.sections));
            assert!(original.sections[0].children.shares_storage_with(&candidate.sections[0].children));
            let PreservedNode::Element(original_paragraph) = &original.sections[0].children[0] else {
                unreachable!();
            };
            let PreservedNode::Element(candidate_paragraph) = &candidate.sections[0].children[0] else {
                unreachable!();
            };
            assert!(original_paragraph.children.shares_storage_with(&candidate_paragraph.children));
            let (PreservedNode::Text(original_text), PreservedNode::Text(candidate_text)) =
                (&original_paragraph.children[0], &candidate_paragraph.children[0])
            else {
                unreachable!();
            };
            assert!(original_text.shares_storage_with(candidate_text));
        }

        let id = StableId::new("element").unwrap_or_else(|_| unreachable!());
        candidate
            .replace_managed_text(
                &id,
                "INT. ROOM",
                0,
                5,
                "EXT.",
                Some(&["INT. ".to_owned(), "ROOM".to_owned()]),
            )
            .unwrap_or_else(|_| unreachable!());

        assert_eq!(original.source.original_bytes, b"<Script/>".to_vec());
        assert!(original.source.original_bytes.shares_storage_with(&candidate.source.original_bytes));
        assert_eq!(
            match &original.sections[0].children[0] {
                PreservedNode::Element(paragraph) => match &paragraph.children[0] {
                    PreservedNode::Text(text) => text.as_str(),
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            },
            "INT. "
        );
        let (PreservedNode::Element(original_vendor), PreservedNode::Element(candidate_vendor)) =
            (&original.sections[0].children[1], &candidate.sections[0].children[1])
        else {
            unreachable!();
        };
        assert!(original_vendor.children.shares_storage_with(&candidate_vendor.children));
        let PreservedNode::Element(original_paragraph) = &original.sections[0].children[0] else {
            unreachable!();
        };
        let PreservedNode::Element(candidate_paragraph) = &candidate.sections[0].children[0] else {
            unreachable!();
        };
        let (PreservedNode::CData(original_cdata), PreservedNode::CData(candidate_cdata)) =
            (&original_paragraph.children[1], &candidate_paragraph.children[1])
        else {
            unreachable!();
        };
        assert!(original_cdata.shares_storage_with(candidate_cdata));
        let after_counters = copy_on_write_diagnostics();
        assert!(after_counters.shared_vector_clones > before_counters.shared_vector_clones);
        assert!(after_counters.detached_vectors > before_counters.detached_vectors);
        assert!(after_counters.shared_text_payload_clones > before_counters.shared_text_payload_clones);
    }

    #[test]
    fn copy_on_write_source_and_image_payload_mutation_is_isolated() {
        let mut original = base_state();
        original.board_graph.entities.push(BoardEntity {
            identity: ForeignIdentity::new(ForeignIdDomain::Image, "image-cow")
                .unwrap_or_else(|_| unreachable!()),
            kind: BoardEntityKind::Image,
            source_order: 1,
            anchor: None,
            image_payload: Some(SharedBytes::image(vec![1, 2, 3, 4])),
            declared_image_length: Some(4),
            source_path: None,
        });
        let before_counters = copy_on_write_diagnostics();
        let mut candidate = original.clone();
        assert!(original.source.original_bytes.shares_storage_with(&candidate.source.original_bytes));
        let original_image = original.board_graph.entities[0]
            .image_payload.as_ref().unwrap_or_else(|| unreachable!());
        let candidate_image = candidate.board_graph.entities[0]
            .image_payload.as_ref().unwrap_or_else(|| unreachable!());
        assert!(original_image.shares_storage_with(candidate_image));
        let shared_counters = copy_on_write_diagnostics();
        assert!(shared_counters.shared_source_byte_clones > before_counters.shared_source_byte_clones);
        assert!(shared_counters.shared_image_payload_clones > before_counters.shared_image_payload_clones);

        candidate.source.original_bytes[0] = b'X';
        candidate.board_graph.entities[0]
            .image_payload
            .as_mut()
            .unwrap_or_else(|| unreachable!())[0] = 9;

        assert_eq!(original.source.original_bytes.as_slice(), b"<Script/>");
        assert_eq!(candidate.source.original_bytes[0], b'X');
        assert_eq!(
            original.board_graph.entities[0]
                .image_payload
                .as_ref()
                .unwrap_or_else(|| unreachable!())
                .as_slice(),
            &[1, 2, 3, 4]
        );
        assert_eq!(
            candidate.board_graph.entities[0]
                .image_payload
                .as_ref()
                .unwrap_or_else(|| unreachable!())
                .as_slice(),
            &[9, 2, 3, 4]
        );
        let detached_counters = copy_on_write_diagnostics();
        assert!(detached_counters.detached_source_byte_payloads > shared_counters.detached_source_byte_payloads);
        assert!(detached_counters.detached_image_payloads > shared_counters.detached_image_payloads);
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
                image_payload: Some(SharedBytes::image(vec![0, 1, 2])),
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
                children: vec![PreservedNode::Text("opaque authored text".into())].into(),
            })].into(),
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
                PreservedNode::Text(String::new().into()),
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
                    children: vec![PreservedNode::Text("A title".into())].into(),
                }),
            ].into(),
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
                children: vec![node].into(),
            });
        }
        deep.sections[0].children = vec![node].into();
        assert_eq!(deep.validate(), Err(PreservationError::ExcessiveDepth));

        let mut oversized = base_state();
        oversized.source.original_bytes = SharedBytes::source(vec![0; MAX_SOURCE_BYTES + 1]);
        assert_eq!(oversized.validate(), Err(PreservationError::SourceTooLarge));
    }

    #[test]
    fn source_identity_index_matches_legacy_safety_and_reservation_scans() {
        fn legacy_external_use(sections: &[PreservedSection], id: &str) -> bool {
            fn scan(nodes: &[PreservedNode], id: &str) -> bool {
                nodes.iter().any(|node| match node {
                    PreservedNode::Element(element) => {
                        let mut ids = element.attributes.iter().filter(|attribute| {
                            attribute.name.namespace_uri.is_none()
                                && attribute.name.local_name.eq_ignore_ascii_case("id")
                        });
                        let target = (element.name.local_name == "Paragraph")
                            .then(|| ids.next().map(|attribute| attribute.value.as_str()))
                            .flatten()
                            .filter(|_| ids.next().is_none())
                            == Some(id);
                        element.attributes.iter().any(|attribute| {
                            attribute.value.contains(id)
                                && !(target
                                    && attribute.name.namespace_uri.is_none()
                                    && attribute.name.local_name.eq_ignore_ascii_case("id"))
                        }) || scan(&element.children, id)
                    }
                    PreservedNode::Text(text)
                    | PreservedNode::CData(text)
                    | PreservedNode::Comment(text) => text.contains(id),
                    PreservedNode::ProcessingInstruction { target, data } => {
                        target.contains(id) || data.contains(id)
                    }
                })
            }
            sections.iter().any(|section| scan(&section.children, id))
        }

        fn legacy_source_ids(sections: &[PreservedSection]) -> BTreeSet<String> {
            fn scan(nodes: &[PreservedNode], ids: &mut BTreeSet<String>) {
                for node in nodes {
                    if let PreservedNode::Element(element) = node {
                        for attribute in &element.attributes {
                            if attribute.name.namespace_uri.is_none()
                                && attribute.name.local_name.eq_ignore_ascii_case("id")
                            {
                                ids.insert(attribute.value.clone());
                            }
                        }
                        scan(&element.children, ids);
                    }
                }
            }
            let mut ids = BTreeSet::new();
            for section in sections {
                scan(&section.children, &mut ids);
            }
            ids
        }

        fn paragraph_identity(value: &str) -> ForeignIdentity {
            ForeignIdentity::new(ForeignIdDomain::Paragraph, value)
                .unwrap_or_else(|_| unreachable!())
        }

        let mut state = base_state();
        let values = [
            "SAFE_ID",
            "ID_ATTR",
            "ID_TEXT",
            "ID_CDATA",
            "ID_COMMENT",
            "ID_PI_TARGET",
            "ID_PI_DATA",
            "ID_CHILD_ATTR",
            "DUPLICATE_ID",
            "REFERENCED_ID",
            "ANCHORED_ID",
        ];
        state.identities = values.iter().map(|value| paragraph_identity(value)).collect();
        state.identities.push(paragraph_identity("DUPLICATE_ID"));
        state.references.push(ForeignReference {
            source: ForeignIdentity::new(ForeignIdDomain::Board, "board-ref")
                .unwrap_or_else(|_| unreachable!()),
            target: paragraph_identity("REFERENCED_ID"),
            policy: ReferencePolicy::CrossDomainMayOrphan,
        });
        let foreign_anchor = SourceAnchorId::new("foreign-anchor")
            .unwrap_or_else(|_| unreachable!());
        state.anchors.push(SourceAnchor {
            id: foreign_anchor,
            target: AnchorTarget::Foreign(paragraph_identity("ANCHORED_ID")),
            relation: AnchorRelation::Attached,
        });
        let PreservedNode::Element(managed) = &mut state.sections[0].children[0] else {
            unreachable!();
        };
        managed.attributes.push(PreservedAttribute {
            name: name("id"),
            value: "SAFE_ID".into(),
        });
        let opaque = PreservedElement {
            name: name("VendorPayload"),
            attributes: vec![
                PreservedAttribute {
                    name: name("ref"),
                    value: "prefix-ID_ATTR-suffix".into(),
                },
                PreservedAttribute {
                    name: name("ID"),
                    value: "SOURCE_RESERVED".into(),
                },
                PreservedAttribute {
                    name: SourceName {
                        namespace_uri: Some("urn:vendor".into()),
                        local_name: "id".into(),
                        qualified_name: Some("vendor:id".into()),
                    },
                    value: "NAMESPACE_ONLY".into(),
                },
            ],
            namespace_bindings: Vec::new(),
            source_type: None,
            ownership: SourceOwnership::Opaque,
            dependency_confidence: DependencyConfidence::Unknown,
            anchor: None,
            children: vec![
                PreservedNode::Text("before-ID_TEXT-after".into()),
                PreservedNode::CData("before-ID_CDATA-after".into()),
                PreservedNode::Comment("before-ID_COMMENT-after".into()),
                PreservedNode::ProcessingInstruction {
                    target: "ID_PI_TARGET".into(),
                    data: "before-ID_PI_DATA-after".into(),
                },
                PreservedNode::Element(PreservedElement {
                    name: name("NestedVendor"),
                    attributes: vec![PreservedAttribute {
                        name: name("data"),
                        value: "x-ID_CHILD_ATTR-y".into(),
                    }],
                    namespace_bindings: Vec::new(),
                    source_type: None,
                    ownership: SourceOwnership::Opaque,
                    dependency_confidence: DependencyConfidence::Unknown,
                    anchor: None,
                    children: Vec::new().into(),
                }),
            ]
            .into(),
        };
        state.sections[0].children.push(PreservedNode::Element(opaque));
        state.rebuild_managed_paragraph_index();

        let legacy_ids = legacy_source_ids(&state.sections);
        assert_eq!(state.indexed_source_id_values(), legacy_ids);
        for identity in state
            .identities
            .iter()
            .filter(|identity| identity.domain == ForeignIdDomain::Paragraph)
        {
            let legacy = state.identities.iter().filter(|other| **other == *identity).count() == 1
                && !state.references.iter().chain(&state.board_graph.references).any(|reference| {
                    reference.source == *identity || reference.target == *identity
                })
                && !state.anchors.iter().any(|anchor| {
                    matches!(&anchor.target, AnchorTarget::Foreign(target) if target == identity)
                })
                && !legacy_external_use(&state.sections, &identity.value);
            assert_eq!(
                state.paragraph_identity_safely_removable(&identity.value),
                legacy,
                "differential mismatch for an indexed Paragraph identity",
            );
        }
        assert!(state.paragraph_identity_safely_removable("SAFE_ID"));
        assert!(state.indexed_source_id_values().contains("SOURCE_RESERVED"));
        assert!(!state.indexed_source_id_values().contains("NAMESPACE_ONLY"));
    }

    #[test]
    fn managed_source_mutation_invalidates_then_rebuilds_identity_use_index() {
        let mut state = base_state();
        state.identities.push(
            ForeignIdentity::new(ForeignIdDomain::Paragraph, "LATE_USE")
                .unwrap_or_else(|_| unreachable!()),
        );
        state.rebuild_managed_paragraph_index();
        assert!(state.paragraph_identity_safely_removable("LATE_USE"));
        let managed_id = StableId::new("element").unwrap_or_else(|_| unreachable!());
        state
            .replace_managed_text(&managed_id, "INT. ROOM", 9, 9, " LATE_USE", None)
            .unwrap_or_else(|_| unreachable!());
        assert!(state.source_identity_use_index.is_none());
        state.ensure_source_identity_use_index();
        assert!(!state.paragraph_identity_safely_removable("LATE_USE"));

        let mut clone = state.clone();
        clone.invalidate_source_identity_use_index();
        assert_eq!(clone, state, "transient indexes stay outside semantic equality");
    }

    #[test]
    fn derived_publication_rebuilds_both_transient_preservation_indexes() {
        let mut state = base_state();
        state.rebuild_managed_paragraph_index();
        let mut project = project_with_preservation(state);
        assert!(crate::scene_metadata::reconcile_project(&mut project)
            .unwrap_or_else(|_| unreachable!()));
        let state = project.preservation.as_ref().unwrap_or_else(|| unreachable!());
        assert!(state.has_managed_paragraph_index());
        assert!(state.has_source_identity_use_index());
    }
}

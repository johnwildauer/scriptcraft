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

/// Value-free operation counts for preservation reconciliation.
///
/// Reconciliation counters record attempted operations immediately before a
/// fallible operation begins. They are process-wide and intentionally contain
/// no document-derived values.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PreservationReconciliationDiagnostics {
    pub body_survivors_visited: u64,
    pub body_unchanged_survivors_skipped: u64,
    pub body_owners_projected: u64,
    pub body_owners_removed: u64,
    pub body_owners_inserted: u64,
    pub title_survivors_visited: u64,
    pub title_unchanged_survivors_skipped: u64,
    pub title_owners_projected: u64,
    pub title_owners_removed: u64,
    pub title_owners_inserted: u64,
    pub source_text_byte_delta_updates: u64,
}

static BODY_SURVIVORS_VISITED: AtomicU64 = AtomicU64::new(0);
static BODY_UNCHANGED_SURVIVORS_SKIPPED: AtomicU64 = AtomicU64::new(0);
static BODY_OWNERS_PROJECTED: AtomicU64 = AtomicU64::new(0);
static BODY_OWNERS_REMOVED: AtomicU64 = AtomicU64::new(0);
static BODY_OWNERS_INSERTED: AtomicU64 = AtomicU64::new(0);
static TITLE_SURVIVORS_VISITED: AtomicU64 = AtomicU64::new(0);
static TITLE_UNCHANGED_SURVIVORS_SKIPPED: AtomicU64 = AtomicU64::new(0);
static TITLE_OWNERS_PROJECTED: AtomicU64 = AtomicU64::new(0);
static TITLE_OWNERS_REMOVED: AtomicU64 = AtomicU64::new(0);
static TITLE_OWNERS_INSERTED: AtomicU64 = AtomicU64::new(0);
static SOURCE_TEXT_BYTE_DELTA_UPDATES: AtomicU64 = AtomicU64::new(0);

/// Returns process-wide preservation reconciliation work counts.
pub fn preservation_reconciliation_diagnostics() -> PreservationReconciliationDiagnostics {
    PreservationReconciliationDiagnostics {
        body_survivors_visited: BODY_SURVIVORS_VISITED.load(Ordering::Relaxed),
        body_unchanged_survivors_skipped: BODY_UNCHANGED_SURVIVORS_SKIPPED
            .load(Ordering::Relaxed),
        body_owners_projected: BODY_OWNERS_PROJECTED.load(Ordering::Relaxed),
        body_owners_removed: BODY_OWNERS_REMOVED.load(Ordering::Relaxed),
        body_owners_inserted: BODY_OWNERS_INSERTED.load(Ordering::Relaxed),
        title_survivors_visited: TITLE_SURVIVORS_VISITED.load(Ordering::Relaxed),
        title_unchanged_survivors_skipped: TITLE_UNCHANGED_SURVIVORS_SKIPPED
            .load(Ordering::Relaxed),
        title_owners_projected: TITLE_OWNERS_PROJECTED.load(Ordering::Relaxed),
        title_owners_removed: TITLE_OWNERS_REMOVED.load(Ordering::Relaxed),
        title_owners_inserted: TITLE_OWNERS_INSERTED.load(Ordering::Relaxed),
        source_text_byte_delta_updates: SOURCE_TEXT_BYTE_DELTA_UPDATES.load(Ordering::Relaxed),
    }
}

/// Resets reconciliation counters without changing preserved data or indexes.
pub fn reset_preservation_reconciliation_diagnostics() {
    BODY_SURVIVORS_VISITED.store(0, Ordering::Relaxed);
    BODY_UNCHANGED_SURVIVORS_SKIPPED.store(0, Ordering::Relaxed);
    BODY_OWNERS_PROJECTED.store(0, Ordering::Relaxed);
    BODY_OWNERS_REMOVED.store(0, Ordering::Relaxed);
    BODY_OWNERS_INSERTED.store(0, Ordering::Relaxed);
    TITLE_SURVIVORS_VISITED.store(0, Ordering::Relaxed);
    TITLE_UNCHANGED_SURVIVORS_SKIPPED.store(0, Ordering::Relaxed);
    TITLE_OWNERS_PROJECTED.store(0, Ordering::Relaxed);
    TITLE_OWNERS_REMOVED.store(0, Ordering::Relaxed);
    TITLE_OWNERS_INSERTED.store(0, Ordering::Relaxed);
    SOURCE_TEXT_BYTE_DELTA_UPDATES.store(0, Ordering::Relaxed);
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

/// Identifies the one supported paragraph-owned opaque FDX attachment by
/// expanded name. The owner namespace is authoritative; prefix spelling is not.
pub(crate) fn is_user_paragraph_data_name(name: &SourceName, owner: &SourceName) -> bool {
    name.local_name == "UserParagraphData" && name.namespace_uri == owner.namespace_uri
}

pub(crate) fn is_user_paragraph_data_lookalike(name: &SourceName) -> bool {
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
        fn reserve_payload_ids(element: &PreservedElement, index: &mut SourceIdentityUseIndex) {
            for attribute in &element.attributes {
                if attribute.name.namespace_uri.is_none()
                    && attribute.name.local_name.eq_ignore_ascii_case("id")
                {
                    index.source_id_values.insert(attribute.value.clone());
                }
            }
            for child in &element.children {
                if let PreservedNode::Element(child) = child {
                    reserve_payload_ids(child, index);
                }
            }
        }
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
                        if parent.is_some_and(|owner| {
                            owner.name.local_name == "Paragraph"
                                && is_user_paragraph_data_name(&element.name, &owner.name)
                        }) {
                            // Payload IDs still reserve lexical source values for
                            // collision-safe allocation, but have no typed identity
                            // or reference meaning in ScriptCraft.
                            reserve_payload_ids(element, index);
                            path.pop();
                            continue;
                        }
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
                            // A unique managed/alternate Paragraph ID attribute is
                            // a definition, not a lexical use of any shorter ID it
                            // happens to contain (for example, 10 is not a use of 1).
                            if exempt.is_none() {
                                automaton.scan(
                                    &attribute.value,
                                    None,
                                    &mut index.externally_used_paragraph_ids,
                                );
                            }
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

}

/// Builds only managed-owner locations and source text-byte accounting. This
/// deliberately avoids constructing the identity-use automaton and its scans.
fn build_managed_paragraph_index(
    sections: &[PreservedSection],
) -> (BTreeMap<StableId, Vec<ManagedParagraphLocation>>, usize, u64) {
    fn collect(
        nodes: &[PreservedNode],
        section_index: usize,
        parent: Option<&PreservedElement>,
        path: &mut Vec<usize>,
        managed_index: &mut BTreeMap<StableId, Vec<ManagedParagraphLocation>>,
        text_bytes: &mut usize,
        visited: &mut u64,
    ) {
        for (node_index, node) in nodes.iter().enumerate() {
            *visited = visited.saturating_add(1);
            match node {
                PreservedNode::Element(element) => {
                    path.push(node_index);
                    if parent.is_some_and(|owner| {
                        owner.name.local_name == "Paragraph"
                            && is_user_paragraph_data_name(&element.name, &owner.name)
                    }) {
                        path.pop();
                        continue;
                    }
                    if let SourceOwnership::Managed(id) = &element.ownership {
                        managed_index.entry(id.clone()).or_default().push(
                            ManagedParagraphLocation {
                                section_index,
                                node_path: path.clone(),
                            },
                        );
                    }
                    collect(
                        &element.children,
                        section_index,
                        Some(element),
                        path,
                        managed_index,
                        text_bytes,
                        visited,
                    );
                    path.pop();
                }
                PreservedNode::Text(text)
                | PreservedNode::CData(text)
                | PreservedNode::Comment(text) => {
                    *text_bytes = text_bytes.saturating_add(text.len());
                }
                PreservedNode::ProcessingInstruction { data, .. } => {
                    *text_bytes = text_bytes.saturating_add(data.len());
                }
            }
        }
    }

    let mut managed_index = BTreeMap::new();
    let mut text_bytes = 0usize;
    let mut visited = 0u64;
    for (section_index, section) in sections.iter().enumerate() {
        collect(
            &section.children,
            section_index,
            None,
            &mut Vec::new(),
            &mut managed_index,
            &mut text_bytes,
            &mut visited,
        );
    }
    (managed_index, text_bytes, visited)
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

pub(crate) enum ScopedManagedEditResult {
    Validated { source_text_bytes: usize },
    CompleteValidationRequired,
}

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
        let mut matching = paragraph.attributes.iter().enumerate().filter(|(_, attribute)| {
            attribute.name.namespace_uri.is_none() && attribute.name.local_name == name
        }).map(|(index, _)| index).collect::<Vec<_>>();
        if let Some(first) = matching.first().copied() {
            if let Some(value) = value {
                paragraph.attributes[first].value = value.to_owned();
                for index in matching.drain(1..).rev() {
                    paragraph.attributes.remove(index);
                }
            } else {
                for index in matching.into_iter().rev() {
                    paragraph.attributes.remove(index);
                }
            }
        } else if let Some(value) = value {
            paragraph.attributes.push(PreservedAttribute {
                name: SourceName {
                    namespace_uri: None,
                    local_name: name.to_owned(),
                    qualified_name: Some(name.to_owned()),
                },
                value: value.to_owned(),
            });
        }
        Ok(())
    }

    /// Kept as a compatibility query for callers that only need to validate a
    /// canonical range. Opaque source boundaries do not constrain semantic edits.
    pub(crate) fn ensure_run_range_is_contiguous(
        &self,
        _id: &StableId,
        _range: std::ops::Range<usize>,
    ) -> Result<(), PreservationError> {
        Ok(())
    }

    /// Projects canonical text and ordered runs to the current owner. Reuses an
    /// unchanged, unambiguous Text template; changed Text subtrees are replaced,
    /// so opaque inline descendants are discarded only with the affected run.
    /// Non-Text Paragraph attachments and siblings are retained in source order.
    /// In-place projection keeps the managed location and updates retained text
    /// byte accounting from this owner's before/after contribution.
    pub(crate) fn project_managed_text_runs(
        &mut self,
        id: &StableId,
        original_text: &str,
        text: &str,
        runs: &[TextRun],
        provenance: Option<&SourceProvenance>,
        role_name: Option<&str>,
    ) -> Result<Option<SourceProvenance>, PreservationError> {
        if text.len() > crate::MAX_TEXT_BYTES {
            return Err(PreservationError::TextTooLarge);
        }
        if runs.iter().map(|run| run.text.as_str()).collect::<String>()
            != if runs.is_empty() { "" } else { text }
        {
            // Empty legacy run lists project as one unstyled run for nonempty text.
            if !runs.is_empty() {
                return Err(PreservationError::UnsupportedTextMapping);
            }
        }
        self.ensure_managed_paragraph_index();
        let recovered = if self.indexed_location(id)?.is_none() {
            match provenance {
                Some(provenance) => Some(self.recover_managed_paragraph(id, provenance, role_name)?),
                None => return Ok(None),
            }
        } else {
            None
        };
        let effective_runs = if text.is_empty() {
            Vec::new()
        } else if runs.is_empty() {
            vec![TextRun { text: text.to_owned(), attributes: Vec::new() }]
        } else {
            runs.to_vec()
        };
        if effective_runs.iter().map(|run| run.text.as_str()).collect::<String>() != text {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let old_bytes = self
            .managed_element(id)?
            .map(source_text_bytes_in_element)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let paragraph = self.managed_element_mut_indexed(id)?;
        let text_slots = paragraph.children.iter().enumerate().filter_map(|(index, node)| {
            match node {
                PreservedNode::Element(element) if is_text_element(element, paragraph) => {
                    Some((index, element.clone()))
                }
                _ => None,
            }
        }).collect::<Vec<_>>();
        let source_text = text_slots.iter().map(|(_, template)| text_element_plain_content(template))
            .collect::<String>();
        let has_exact_old_source = source_text == original_text;
        let (old_change, new_change) = if has_exact_old_source {
            common_text_change(original_text, text)
        } else {
            (0..original_text.len(), 0..text.len())
        };
        let delta = i64::try_from(new_change.len()).unwrap_or(i64::MAX)
            - i64::try_from(old_change.len()).unwrap_or(i64::MAX);
        let mut replacements = BTreeMap::<usize, Vec<PreservedNode>>::new();
        let mut mapped_ranges = Vec::with_capacity(text_slots.len());
        let mut old_offset = 0usize;
        for (source_index, template) in &text_slots {
            let segment = text_element_plain_content(template);
            let old_start = old_offset;
            let old_end = old_start.saturating_add(segment.len());
            old_offset = old_end;
            let mapped = if !has_exact_old_source || text.is_empty() {
                0..0
            } else if old_end <= old_change.start {
                old_start..old_end
            } else if old_start >= old_change.end {
                shift_text_offset(old_start, delta)..shift_text_offset(old_end, delta)
            } else {
                let map_boundary = |offset| {
                    if offset <= old_change.start {
                        offset
                    } else if offset >= old_change.end {
                        shift_text_offset(offset, delta)
                    } else {
                        new_change.start
                    }
                };
                let start = map_boundary(old_start);
                let end = map_boundary(old_end);
                start..end
            };
            mapped_ranges.push((*source_index, mapped.clone()));
            let segment_runs = slice_text_runs(&effective_runs, mapped.clone())?;
            let unchanged_template = has_exact_old_source
                && segment == text.get(mapped.clone()).unwrap_or_default()
                && segment_runs.len() == 1
                && segment_runs.first().is_some_and(|run| {
                    run.text == segment && text_element_matches_run(template, run)
                });
            let projected = if unchanged_template {
                vec![PreservedNode::Element(template.clone())]
            } else {
                segment_runs.iter().map(|run| {
                    PreservedNode::Element(new_text_element_for_run(paragraph, run))
                }).collect()
            };
            replacements.insert(*source_index, projected);
        }
        let mut covered = 0usize;
        let mut gaps = Vec::new();
        for (_, range) in &mapped_ranges {
            if range.start > covered {
                gaps.push(covered..range.start);
            }
            covered = covered.max(range.end);
        }
        if covered < text.len() {
            gaps.push(covered..text.len());
        }
        if text_slots.is_empty() && !text.is_empty() {
            let projected = effective_runs.iter().map(|run| {
                PreservedNode::Element(new_text_element_for_run(paragraph, run))
            }).collect::<Vec<_>>();
            let insertion_index = paragraph.children.iter().position(|node| !matches!(node,
                PreservedNode::Text(value) | PreservedNode::CData(value) if value.trim().is_empty()
            )).unwrap_or(paragraph.children.len());
            replacements.insert(insertion_index, projected);
        } else {
            for gap in gaps {
                let gap_runs = slice_text_runs(&effective_runs, gap.clone())?;
                let generated = gap_runs.iter().map(|run| {
                    PreservedNode::Element(new_text_element_for_run(paragraph, run))
                }).collect::<Vec<_>>();
                if generated.is_empty() { continue; }
                let prior = mapped_ranges.iter().rev().find(|(_, range)| range.end <= gap.start);
                let next = mapped_ranges.iter().find(|(_, range)| range.start >= gap.end);
                if let Some((index, _)) = prior {
                    replacements.entry(*index).or_default().extend(generated);
                } else if let Some((index, _)) = next {
                    let nodes = replacements.entry(*index).or_default();
                    let mut joined = generated;
                    joined.append(nodes);
                    *nodes = joined;
                } else {
                    let insertion_index = text_slots.first().map_or_else(|| paragraph.children.len(), |(index, _)| *index);
                    replacements.entry(insertion_index).or_default().extend(generated);
                }
            }
        }
        let first_text = text_slots.first().map(|(index, _)| *index).unwrap_or_else(|| {
            paragraph.children.iter().position(|node| !matches!(node,
                PreservedNode::Text(value) | PreservedNode::CData(value) if value.trim().is_empty()
            )).unwrap_or(paragraph.children.len())
        });
        let mut rebuilt = Vec::with_capacity(paragraph.children.len().saturating_add(effective_runs.len()));
        let mut inserted = false;
        for (index, child) in paragraph.children.iter().enumerate() {
            if let Some(replacement) = replacements.remove(&index) {
                rebuilt.extend(replacement);
                if text_slots.is_empty() { inserted = true; }
                continue;
            }
            if index == first_text && text_slots.is_empty() && !inserted {
                inserted = true;
            }
            match child {
                PreservedNode::Element(element) if is_text_element(element, paragraph) => {}
                PreservedNode::Text(value) | PreservedNode::CData(value) if !value.trim().is_empty() => {}
                _ => rebuilt.push(child.clone()),
            }
        }
        if !inserted && text_slots.is_empty() {
            rebuilt.extend(replacements.remove(&first_text).unwrap_or_default());
        }
        paragraph.children = rebuilt.into();
        let new_bytes = source_text_bytes_in_element(paragraph);
        let new_total = self
            .source_text_bytes
            .saturating_sub(old_bytes)
            .saturating_add(new_bytes);
        if new_total > MAX_SOURCE_TEXT_BYTES {
            return Err(PreservationError::TextTooLarge);
        }
        self.source_text_bytes = new_total;
        increment_counter(&SOURCE_TEXT_BYTE_DELTA_UPDATES);
        Ok(recovered)
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
        self.rebuild_managed_and_source_identity_use_indexes();
    }

    /// Rebuilds the transient source-tree location map from the authoritative
    /// ordered preservation tree. Call after any out-of-band structural update.
    pub(crate) fn rebuild_managed_paragraph_index(&mut self) {
        let (managed_index, text_bytes, visited) =
            build_managed_paragraph_index(&self.sections);
        increment_counter(&MANAGED_INDEX_REBUILDS);
        add_counter(&MANAGED_INDEX_SOURCE_NODES, visited);
        self.managed_paragraph_index = Some(managed_index);
        self.source_text_bytes = text_bytes;
    }

    /// Rebuilds both related derived views in one traversal when identity data
    /// is requested and either the identity-use view or managed locations are
    /// missing. Managed-only lookup must not pay for the identity automaton.
    fn rebuild_managed_and_source_identity_use_indexes(&mut self) {
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
        members: &[ScreenplayElement],
        overrides: Option<crate::formatting::SourceParagraphOverrides>,
    ) -> Result<(String, SourceAnchorId, u32), PreservationError> {
        if self.source.format != SourceFormat::Fdx || member_ids.len() < 2
            || member_ids.len() != members.len()
            || member_ids.iter().zip(members).any(|(id, member)| id != &member.id)
        {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let content = source_content(&self.sections, SourceSection::Content)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let root_namespace = content.name.namespace_uri.clone();
        let namespace_bindings = content.namespace_bindings.clone();
        let prefix = content.name.qualified_name.as_deref()
            .and_then(|name| name.split_once(':').map(|(prefix, _)| prefix.to_owned()));
        let foreign_id = self.fresh_foreign_paragraph_id()?;
        let anchor_id = self.fresh_anchor_id()?;
        let mut generated_member_ids = Vec::new();
        for id in member_ids {
            let already_projected = self.sections.iter().any(|section| {
                contains_managed_owner(&section.children, id)
            });
            if !already_projected {
                generated_member_ids.push((id.clone(), self.fresh_foreign_paragraph_id()?, self.fresh_anchor_id()?));
            }
        }

        let member_set = member_ids.iter().cloned().collect::<BTreeSet<_>>();
        let source_position = source_content(&self.sections, SourceSection::Content)
            .and_then(|content| content.children.iter().position(|node| {
                node_contains_managed_owner(node, &member_set)
            }))
            .unwrap_or_else(|| source_content(&self.sections, SourceSection::Content)
                .map_or(0, |content| content.children.len()));
        let source_ordinal = u32::try_from(source_position)
            .map_err(|_| PreservationError::TooManyEvents)?;
        let mut existing_members = BTreeMap::new();
        for section in &mut self.sections {
            take_managed_owner_nodes(&mut section.children, &member_set, &mut existing_members);
        }
        let content_children = source_content_mut(&mut self.sections, SourceSection::Content)
            .map(|content| &mut content.children)
            .ok_or(PreservationError::ManagedAnchorMissing)?;

        let mut outer = source_element("Paragraph", &root_namespace, prefix.as_deref());
        outer.source_type = Some("General".to_owned());
        outer.ownership = SourceOwnership::DualDialogueWrapper(group_id.clone());
        outer.anchor = Some(anchor_id.clone());
        outer.namespace_bindings = namespace_bindings.clone();
        push_source_attribute(&mut outer, "Type", "General");
        push_source_attribute(&mut outer, "id", &foreign_id);
        if let Some(overrides) = overrides {
            push_wrapper_overrides(&mut outer, overrides);
        }
        let mut dual = source_element("DualDialogue", &root_namespace, prefix.as_deref());
        dual.ownership = SourceOwnership::Opaque;
        dual.namespace_bindings = namespace_bindings.clone();
        dual.children.push(PreservedNode::Element(generated_spell_check_block(
            &root_namespace,
            prefix.as_deref(),
            &namespace_bindings,
        )));
        for (id, member) in member_ids.iter().zip(members) {
            let node = if let Some(existing) = existing_members.remove(id) {
                existing
            } else {
                let (_, member_foreign_id, member_anchor) = generated_member_ids.iter()
                    .find(|(missing_id, _, _)| missing_id == id)
                    .ok_or(PreservationError::ManagedAnchorMissing)?;
                PreservedNode::Element(generated_managed_paragraph(
                    member,
                    &root_namespace,
                    prefix.as_deref(),
                    &namespace_bindings,
                    member_foreign_id,
                    member_anchor,
                ))
            };
            dual.children.push(node);
        }
        outer.children.push(PreservedNode::Element(dual));
        content_children.insert(source_position, PreservedNode::Element(outer));
        self.identities.push(ForeignIdentity::new(
            ForeignIdDomain::Paragraph,
            foreign_id.clone(),
        )?);
        self.anchors.push(SourceAnchor {
            id: anchor_id.clone(),
            target: AnchorTarget::DualDialogueWrapper(group_id.clone()),
            relation: AnchorRelation::Represents,
        });
        for (member_id, member_foreign_id, member_anchor) in generated_member_ids {
            self.identities.push(ForeignIdentity::new(
                ForeignIdDomain::Paragraph,
                member_foreign_id,
            )?);
            self.anchors.push(SourceAnchor {
                id: member_anchor,
                target: AnchorTarget::ProjectElement(member_id),
                relation: AnchorRelation::Represents,
            });
        }
        self.invalidate_source_identity_use_index();
        self.rebuild_managed_paragraph_index();
        Ok((foreign_id, anchor_id, source_ordinal))
    }

    /// Removes wrapper-owned source and restores exact managed member nodes at
    /// the wrapper slot; opaque wrapper payload is discarded, never transferred.
    pub(crate) fn unwrap_dual_dialogue_members(
        &mut self,
        group_id: &StableId,
        member_ids: &[StableId],
        body_order: &[StableId],
        wrapper_anchor: Option<&SourceAnchorId>,
        foreign_id: Option<&str>,
    ) -> Result<(), PreservationError> {
        if self.source.format != SourceFormat::Fdx || member_ids.len() < 2 {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let mut members = BTreeMap::new();
        let member_set = member_ids.iter().cloned().collect::<BTreeSet<_>>();
        for section in &mut self.sections {
            take_managed_owner_nodes(&mut section.children, &member_set, &mut members);
        }

        let mut removed_foreign_ids = BTreeSet::new();
        let mut removed_project_ids = BTreeSet::new();
        let mut removed_anchor_ids = BTreeSet::new();
        let mut wrapper_found = false;
        for section in &mut self.sections {
            remove_dual_wrapper_nodes(
                &mut section.children,
                group_id,
                &mut wrapper_found,
                &mut removed_foreign_ids,
                &mut removed_project_ids,
                &mut removed_anchor_ids,
            );
        }
        if let Some(foreign_id) = foreign_id {
            removed_foreign_ids.insert(foreign_id.to_owned());
        }
        if let Some(wrapper_anchor) = wrapper_anchor {
            removed_anchor_ids.insert(wrapper_anchor.clone());
        }
        // Member ownership is semantic and can occur at any depth in the old
        // wrapper subtree. Its registrations survive even though the wrapper is
        // discarded and the exact owner nodes are restored into flat body order.
        for member in members.values() {
            if let PreservedNode::Element(element) = member {
                let mut member_foreign_ids = BTreeSet::new();
                let mut member_project_ids = BTreeSet::new();
                let mut member_anchor_ids = BTreeSet::new();
                collect_owner_registrations(element, &mut member_foreign_ids, &mut member_project_ids, &mut member_anchor_ids);
                removed_foreign_ids.retain(|id| !member_foreign_ids.contains(id));
                removed_project_ids.retain(|id| !member_project_ids.contains(id));
                removed_anchor_ids.retain(|id| !member_anchor_ids.contains(id));
            }
        }

        let content_children = source_content_mut(&mut self.sections, SourceSection::Content)
            .map(|content| &mut content.children)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let member_positions = body_order.iter().enumerate()
            .filter_map(|(index, id)| member_set.contains(id).then_some((index, id)))
            .collect::<Vec<_>>();
        let first_member_order = member_positions.first().map(|(index, _)| *index)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let mut insert_at = None;
        for following in body_order.iter().skip(first_member_order + member_positions.len()) {
            if !member_set.contains(following) {
                if let Some(position) = managed_child_index(content_children, following) {
                    insert_at = Some(position);
                    break;
                }
            }
        }
        if insert_at.is_none() {
            for preceding in body_order[..first_member_order].iter().rev() {
                if let Some(position) = managed_child_index(content_children, preceding) {
                    insert_at = Some(position + 1);
                    break;
                }
            }
        }
        let mut insertion = insert_at.unwrap_or(content_children.len());
        for member_id in member_ids {
            if let Some(member) = members.remove(member_id) {
                content_children.insert(insertion, member);
                insertion += 1;
            }
        }
        self.ensure_source_identity_use_index();
        removed_foreign_ids.retain(|value| {
            self.source_identity_use_index.as_ref().is_some_and(|index| {
                index.paragraph_source_identity_counts.get(value) == Some(&1)
            })
        });
        self.remove_wrapper_registrations(
            &removed_foreign_ids,
            &removed_project_ids,
            &removed_anchor_ids,
            group_id,
        );
        Ok(())
    }

    pub(crate) fn invalidate_source_identity_use_index(&mut self) {
        self.source_identity_use_index = None;
    }

    pub(crate) fn ensure_source_identity_use_index(&mut self) {
        if self.source_identity_use_index.is_none() {
            self.rebuild_managed_and_source_identity_use_indexes();
        } else {
            self.ensure_managed_paragraph_index();
        }
    }

    pub(crate) fn has_source_identity_use_index(&self) -> bool {
        self.source_identity_use_index.is_some()
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
    /// invalidated before mutation; structural callers maintain managed locations
    /// when their edits actually change a path, while identity data rebuilds on demand.
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

    fn is_current_ordinary_body_location(&self, location: &ManagedParagraphLocation) -> bool {
        let Some(section) = self.sections.get(location.section_index) else {
            return false;
        };
        if location.node_path.is_empty() {
            return false;
        }
        let mut nodes = section.children.as_ref();
        let mut inside_content = false;
        for (depth, index) in location.node_path.iter().enumerate() {
            let Some(PreservedNode::Element(element)) = nodes.get(*index) else {
                return false;
            };
            let is_owner = depth + 1 == location.node_path.len();
            if is_owner {
                return element.name.local_name == "Paragraph"
                    && inside_content
                    && matches!(&element.ownership, SourceOwnership::Managed(_));
            }
            if matches!(
                &element.ownership,
                SourceOwnership::DualDialogueWrapper(_) | SourceOwnership::Managed(_)
            )
                || element.name.local_name == "TitlePage"
            {
                return false;
            }
            inside_content |= element.name.local_name == "Content";
            nodes = &element.children;
        }
        false
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

    /// Proves a text/run-only change to one current Body owner against a
    /// complete valid baseline. Scope uncertainty requests complete validation;
    /// a proven current inconsistency remains an error.
    pub(crate) fn validate_scoped_managed_edit(
        &self,
        baseline: &PreservationState,
        semantic: &ScreenplayElement,
    ) -> Result<ScopedManagedEditResult, PreservationError> {
        let baseline_location = match baseline.indexed_location(&semantic.id) {
            Ok(Some(location)) => location.clone(),
            Ok(None) | Err(PreservationError::ManagedAnchorAmbiguous) => {
                return Ok(ScopedManagedEditResult::CompleteValidationRequired)
            }
            Err(error) => return Err(error),
        };
        let candidate_location = match self.indexed_location(&semantic.id) {
            Ok(Some(location)) => location.clone(),
            Ok(None) | Err(PreservationError::ManagedAnchorAmbiguous) => {
                return Ok(ScopedManagedEditResult::CompleteValidationRequired)
            }
            Err(error) => return Err(error),
        };
        if baseline_location != candidate_location
            || !baseline.is_current_ordinary_body_location(&baseline_location)
            || !self.is_current_ordinary_body_location(&candidate_location)
        {
            return Ok(ScopedManagedEditResult::CompleteValidationRequired);
        }
        if !same_sections_except_managed_location(
            &baseline.sections,
            &self.sections,
            &candidate_location,
        ) {
            return Ok(ScopedManagedEditResult::CompleteValidationRequired);
        }
        let before = baseline
            .managed_element_at(&baseline_location)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let after = self
            .managed_element_at(&candidate_location)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        if !matches!(&before.ownership, SourceOwnership::Managed(id) if id == &semantic.id) {
            return Ok(ScopedManagedEditResult::CompleteValidationRequired);
        }
        if !matches!(&after.ownership, SourceOwnership::Managed(id) if id == &semantic.id)
            || after.name.local_name != "Paragraph"
            || !same_managed_paragraph_shell(before, after)
            || !same_managed_owner_attributes_except_type(before, after)
        {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        if !same_direct_owner_attachments(before, after) {
            return Err(PreservationError::StructuralAttachmentConflict);
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
        validate_scoped_owner_fields(after, candidate_location.node_path.len())?;

        let expected_runs = merge_adjacent_equivalent_runs(semantic_text_runs(semantic));
        let mut text_elements = Vec::new();
        for child in &after.children {
            match child {
                PreservedNode::Text(text) | PreservedNode::CData(text)
                    if text.trim().is_empty() => {}
                PreservedNode::Text(text) | PreservedNode::CData(text) => {
                    return Err(PreservationError::ManagedTextMismatchAt(
                        ManagedTextMismatchSite::ScopedEditValidation,
                    ));
                }
                PreservedNode::Element(element) if is_text_element(element, after) => {
                    text_elements.push(element);
                }
                PreservedNode::Element(_) | PreservedNode::Comment(_)
                | PreservedNode::ProcessingInstruction { .. } => {}
            }
        }
        let projected_runs = merge_adjacent_equivalent_runs(
            text_elements.iter().map(|text| TextRun {
                text: text_element_plain_content(text),
                attributes: text.attributes.iter().map(|attribute| crate::RunAttribute {
                    name: attribute.name.qualified_name.as_deref()
                        .unwrap_or(attribute.name.local_name.as_str()).to_owned(),
                    value: attribute.value.clone(),
                }).collect(),
            }).collect(),
        );
        if projected_runs != expected_runs {
            return Err(PreservationError::ManagedTextMismatchAt(
                ManagedTextMismatchSite::ScopedEditValidation,
            ));
        }

        let old_bytes = source_text_bytes_in_element(before);
        let new_bytes = source_text_bytes_in_element(after);
        let new_total = baseline
            .source_text_bytes
            .saturating_sub(old_bytes)
            .saturating_add(new_bytes);
        if new_total > MAX_SOURCE_TEXT_BYTES {
            return Err(PreservationError::TextTooLarge);
        }
        if self.source_text_bytes != new_total {
            return Err(PreservationError::ManagedTextMismatchAt(
                ManagedTextMismatchSite::ScopedEditValidation,
            ));
        }

        // The complete baseline is known valid and the copy-on-write proof above
        // establishes that no other source subtree changed. If this owner adds
        // source events, remaining document-wide event capacity is unknown, so
        // defer that candidate to complete validation rather than guessing.
        if source_event_count_in_element(after) > source_event_count_in_element(before) {
            return Ok(ScopedManagedEditResult::CompleteValidationRequired);
        }
        if !same_owner_anchor_bindings(before, after) {
            return Err(PreservationError::InvalidAnchor);
        }
        Ok(ScopedManagedEditResult::Validated {
            source_text_bytes: new_total,
        })
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

    /// Checks the retained source graph as current projection data and verifies
    /// that every managed registration targets a live semantic owner. This is
    /// deliberately independent of the graph present in `source.original_bytes`.
    pub(crate) fn validate_current_projection(
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

    fn has_owner_opaque_data(&self, id: &StableId) -> bool {
        let paragraph = match self.indexed_location(id) {
            Ok(Some(location)) => self.managed_element_at(location),
            Ok(None) if self.managed_paragraph_index.is_none() => {
                find_managed_element_counted(&self.sections, id)
            }
            _ => None,
        };
        paragraph.is_some_and(|paragraph| {
            paragraph.attributes.iter().any(|attribute| {
                !is_paragraph_id_attribute(attribute) && !is_managed_paragraph_attribute(attribute)
            }) || paragraph.children.iter().any(|node| match node {
                PreservedNode::Element(element) => !is_text_element(element, paragraph)
                    || element.children.iter().any(|child| !matches!(child,
                        PreservedNode::Text(_) | PreservedNode::CData(_))),
                PreservedNode::Text(text) | PreservedNode::CData(text) => !text.trim().is_empty(),
                PreservedNode::Comment(_) | PreservedNode::ProcessingInstruction { .. } => true,
            })
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
        let survivor_runs = paragraph_text_nodes_with_supported_attachments(&survivor)?;
        let removed_runs = paragraph_text_nodes_with_supported_attachments(&removed)?;
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
        self.ensure_managed_paragraph_index();
        let title_source = self.indexed_location(id)?
            .is_some_and(|location| is_title_content_location(&self.sections, location));
        let element = self.managed_element_mut(id)?;
        if title_source && !is_plain_managed_paragraph(element) {
            return Err(PreservationError::StructuralAttachmentConflict);
        }
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
                let text_node = PreservedNode::Element(PreservedElement {
                        name,
                        attributes: Vec::new(),
                        namespace_bindings,
                        source_type: None,
                        ownership: SourceOwnership::Opaque,
                        dependency_confidence: DependencyConfidence::Unknown,
                        anchor: None,
                        children: vec![PreservedNode::Text(replacement.to_owned().into())].into(),
                    });
                let insertion_index = paragraph.children.iter().position(|node| !matches!(node,
                    PreservedNode::Element(element) if is_text_element(element, paragraph)
                )).unwrap_or(paragraph.children.len());
                paragraph.children.insert(insertion_index, text_node);
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
        new_role: &str,
        provenance: Option<&SourceProvenance>,
        ordinal: u32,
    ) -> Result<Option<SourceProvenance>, PreservationError> {
        self.ensure_managed_paragraph_index();
        let recovered = if self.indexed_location(id)?.is_none() {
            let Some(provenance) = provenance else { return Ok(None) };
            let mut provenance = provenance.clone();
            provenance.ordinal = ordinal;
            Some(self.recover_managed_paragraph(id, &provenance, Some(new_role))?)
        } else {
            None
        };
        let element = self.managed_element_mut_indexed(id)?;
        let mut type_indexes = element.attributes.iter().enumerate().filter(|(_, attribute)| {
            attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Type"
        }).map(|(index, _)| index).collect::<Vec<_>>();
        if let Some(first) = type_indexes.first().copied() {
            element.attributes[first].value = new_role.to_owned();
            for index in type_indexes.drain(1..).rev() {
                element.attributes.remove(index);
            }
        } else {
            element.attributes.push(PreservedAttribute {
                name: plain_source_name("Type"),
                value: new_role.to_owned(),
            });
        }
        element.source_type = Some(new_role.to_owned());
        Ok(recovered)
    }

    fn recover_managed_paragraph(
        &mut self,
        id: &StableId,
        provenance: &SourceProvenance,
        role_name: Option<&str>,
    ) -> Result<SourceProvenance, PreservationError> {
        if provenance.format != SourceFormat::Fdx {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let foreign_id = self.fresh_foreign_paragraph_id()?;
        let anchor_id = self.fresh_anchor_id()?;
        let overrides = self.source_layout_settings.direct_paragraph_overrides.get(id).copied();
        let source_role = role_name.or(provenance.foreign_role.as_deref()).unwrap_or("Action");
        {
            let parent = source_content_mut(&mut self.sections, provenance.section)
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            let prefix = parent.name.qualified_name.as_deref()
                .and_then(|name| name.split_once(':').map(|(prefix, _)| prefix));
            let mut paragraph = source_element("Paragraph", &parent.name.namespace_uri, prefix);
            paragraph.source_type = Some(source_role.to_owned());
            paragraph.ownership = SourceOwnership::Managed(id.clone());
            paragraph.anchor = Some(anchor_id.clone());
            push_source_attribute(&mut paragraph, "Type", source_role);
            push_source_attribute(&mut paragraph, "id", &foreign_id);
            if let Some(overrides) = overrides {
                append_recovered_direct_attributes(&mut paragraph, overrides);
            }
            let position = usize::try_from(provenance.ordinal).unwrap_or(usize::MAX)
                .min(parent.children.len());
            parent.children.insert(position, PreservedNode::Element(paragraph));
        }
        self.anchors.push(SourceAnchor {
            id: anchor_id,
            target: AnchorTarget::ProjectElement(id.clone()),
            relation: AnchorRelation::Represents,
        });
        self.identities.push(ForeignIdentity::new(ForeignIdDomain::Paragraph, foreign_id.clone())?);
        self.invalidate_source_identity_use_index();
        self.rebuild_managed_paragraph_index();
        Ok(SourceProvenance {
            format: SourceFormat::Fdx,
            section: provenance.section,
            ordinal: provenance.ordinal,
            foreign_id: Some(foreign_id),
            foreign_role: Some(source_role.to_owned()),
        })
    }

    /// Reconciles a body-structure edit in the same candidate as its semantic
    /// model. Existing foreign paragraph IDs stay with surviving elements;
    /// newly inserted paragraphs receive fresh foreign and source-anchor IDs.
    /// Explicitly removed owners are destructively discarded after semantic
    /// structure has authorized the transaction; surviving owners keep their
    /// exact opaque source and mapped text behavior.
    pub(crate) fn reconcile_body_structure(
        &mut self,
        before: &[ScreenplayElement],
        after: &mut [ScreenplayElement],
        _generated_scene_heading_deletions: &BTreeSet<StableId>,
        groups: &mut [crate::dual_dialogue::DualDialogueGroup],
    ) -> Result<(), PreservationError> {
        self.ensure_managed_paragraph_index();
        let before_by_id = before
            .iter()
            .map(|element| (element.id.clone(), element))
            .collect::<BTreeMap<_, _>>();
        let after_ids = after
            .iter()
            .map(|element| element.id.clone())
            .collect::<BTreeSet<_>>();
        let removed = before
            .iter()
            .filter(|element| !after_ids.contains(&element.id))
            .collect::<Vec<_>>();
        for element in &removed {
            increment_counter(&BODY_OWNERS_REMOVED);
            if removed.len() == 1 {
                self.remove_managed_paragraph(
                    &element.id,
                    element.source_provenance.as_ref(),
                )?;
            } else {
                self.remove_managed_paragraph_without_rebuild(
                    &element.id,
                    element.source_provenance.as_ref(),
                )?;
            }
        }
        // Removals change paths, but deleting many owners is one structural
        // boundary: publish their combined location map before owner lookups.
        if removed.len() > 1 {
            self.rebuild_managed_paragraph_index();
        }

        let ordered_ids = after
            .iter()
            .map(|element| element.id.clone())
            .collect::<Vec<_>>();
        let canonical_after = after.to_vec();
        let mut generated_member_provenance = Vec::new();
        let mut preexisting_source_owners = BTreeSet::new();
        let mut source_owners = BTreeSet::new();
        for element in after.iter() {
            if self.indexed_location(&element.id)?.is_some() {
                preexisting_source_owners.insert(element.id.clone());
                source_owners.insert(element.id.clone());
            }
        }
        let mut inserted_any = false;
        for (index, element) in after.iter_mut().enumerate() {
            if source_owners.contains(&element.id) {
                continue;
            }
            increment_counter(&BODY_OWNERS_INSERTED);
            let provenance = self.insert_managed_paragraph(
                element,
                index,
                &ordered_ids,
                groups,
                &canonical_after,
                &mut generated_member_provenance,
            )?;
            element.source_provenance = Some(provenance);
            source_owners.insert(element.id.clone());
            source_owners.extend(
                generated_member_provenance
                    .iter()
                    .map(|(id, _)| id.clone()),
            );
            inserted_any = true;
        }
        for element in after.iter_mut() {
            if let Some((_, provenance)) = generated_member_provenance
                .iter()
                .find(|(id, _)| id == &element.id)
            {
                element.source_provenance = Some(provenance.clone());
            }
        }
        // Insertions may shift paths for every later owner. Publish one map
        // after the batch; role and Text projection below are location-neutral.
        if inserted_any {
            self.invalidate_source_identity_use_index();
            self.rebuild_managed_paragraph_index();
        }
        // Stable semantic owners, not prior source partitions, define the
        // resulting Text projection. Newly inserted and split-right owners use
        // the generated shell above; existing owners keep their own Paragraph
        // attachments while their Text descendants are projected from runs.
        for (position, element) in after.iter_mut().enumerate() {
            let old = before_by_id.get(&element.id).copied();
            if old.is_some() {
                increment_counter(&BODY_SURVIVORS_VISITED);
            }
            let role_changed = old.is_some_and(|previous| previous.role != element.role);
            let text_or_runs_changed = old.is_some_and(|previous| {
                previous.text != element.text
                    || !semantic_text_runs_equal(previous, element)
            });
            if old.is_some()
                && preexisting_source_owners.contains(&element.id)
                && !role_changed
                && !text_or_runs_changed
            {
                increment_counter(&BODY_UNCHANGED_SURVIVORS_SKIPPED);
                continue;
            }
            let original_text = old.map_or("", |previous| previous.text.as_str());
            if role_changed {
                let recovered = self.change_managed_role(
                    &element.id,
                    fdx_role_name(element.role),
                    element.source_provenance.as_ref(),
                    u32::try_from(position).map_err(|_| PreservationError::TooManyEvents)?,
                )?;
                if let Some(provenance) = recovered {
                    element.source_provenance = Some(provenance);
                }
            }
            if source_owners.contains(&element.id) {
                increment_counter(&BODY_OWNERS_PROJECTED);
                if let Some(recovered) = self.project_managed_text_runs(
                    &element.id,
                    original_text,
                    &element.text,
                    &semantic_text_runs(element),
                    element.source_provenance.as_ref(),
                    Some(fdx_role_name(element.role)),
                )? {
                    element.source_provenance = Some(recovered);
                }
            }
        }
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
        let before_by_id = before
            .iter()
            .map(|paragraph| (paragraph.id.clone(), paragraph))
            .collect::<BTreeMap<_, _>>();
        let after_ids = after
            .iter()
            .map(|paragraph| paragraph.id.clone())
            .collect::<BTreeSet<_>>();
        let removed = before
            .iter()
            .filter(|paragraph| !after_ids.contains(&paragraph.id))
            .collect::<Vec<_>>();
        let mut removed_source_owner = false;
        for paragraph in &removed {
            increment_counter(&TITLE_OWNERS_REMOVED);
            removed_source_owner |= self.indexed_location(&paragraph.id)?.is_some();
            self.remove_managed_paragraph_without_rebuild(
                &paragraph.id,
                paragraph.source_provenance.as_ref(),
            )?;
        }
        // Batched removals shift later source paths. Publish their combined map
        // once, before deciding which surviving owners need recovery.
        if removed_source_owner {
            self.rebuild_managed_paragraph_index();
        }
        let ordered = after.iter().map(|paragraph| paragraph.id.clone()).collect::<Vec<_>>();
        let mut preexisting_source_owners = BTreeSet::new();
        for paragraph in after.iter() {
            if self.indexed_location(&paragraph.id)?.is_some() {
                preexisting_source_owners.insert(paragraph.id.clone());
            }
        }
        let mut inserted_any = false;
        for (index, paragraph) in after.iter_mut().enumerate() {
            if preexisting_source_owners.contains(&paragraph.id) {
                continue;
            }
            increment_counter(&TITLE_OWNERS_INSERTED);
            paragraph.source_provenance = Some(self.insert_title_managed_paragraph(
                paragraph,
                index,
                &ordered,
            )?);
            inserted_any = true;
        }
        // Insertion shifts paths, but role and Text projection are location-
        // neutral. Rebuild once after the complete insertion batch.
        if inserted_any {
            self.rebuild_managed_paragraph_index();
        }
        for paragraph in after.iter_mut() {
            let original = before_by_id.get(&paragraph.id).copied();
            if original.is_some() {
                increment_counter(&TITLE_SURVIVORS_VISITED);
            }
            let role_changed = original.is_some_and(|old| old.role != paragraph.role);
            let text_or_runs_changed = original.is_some_and(|old| {
                old.text != paragraph.text
                    || title_effective_runs(old) != title_effective_runs(paragraph)
            });
            if original.is_some()
                && preexisting_source_owners.contains(&paragraph.id)
                && !role_changed
                && !text_or_runs_changed
            {
                increment_counter(&TITLE_UNCHANGED_SURVIVORS_SKIPPED);
                continue;
            }
            let old_text = original.map_or("", |old| old.text.as_str());
            if role_changed {
                self.set_managed_role_projection(&paragraph.id, paragraph.role.map(fdx_role_name))?;
            }
            increment_counter(&TITLE_OWNERS_PROJECTED);
            if let Some(provenance) = self.project_managed_text_runs(
                &paragraph.id,
                old_text,
                &paragraph.text,
                &title_effective_runs(paragraph),
                paragraph.source_provenance.as_ref(),
                paragraph.role.map(fdx_role_name),
            )? {
                paragraph.source_provenance = Some(provenance);
            }
        }
        Ok(())
    }

    fn insert_title_managed_paragraph(
        &mut self,
        semantic: &TitlePageParagraph,
        index: usize,
        ordered: &[StableId],
    ) -> Result<SourceProvenance, PreservationError> {
        let (namespace, bindings, prefix) = {
            let content = source_content_mut(&mut self.sections, SourceSection::TitlePage)
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            let prefix = content.name.qualified_name.as_deref()
                .and_then(|name| name.split_once(':').map(|(prefix, _)| prefix.to_owned()));
            (content.name.namespace_uri.clone(), content.namespace_bindings.clone(), prefix)
        };
        let foreign_id = self.fresh_foreign_paragraph_id()?;
        let anchor_id = self.fresh_anchor_id()?;
        let mut paragraph = source_element("Paragraph", &namespace, prefix.as_deref());
        paragraph.namespace_bindings = bindings.clone();
        paragraph.ownership = SourceOwnership::Managed(semantic.id.clone());
        paragraph.anchor = Some(anchor_id.clone());
        if let Some(role) = semantic.role {
            paragraph.source_type = Some(fdx_role_name(role).to_owned());
            push_source_attribute(&mut paragraph, "Type", fdx_role_name(role));
        }
        push_source_attribute(&mut paragraph, "id", &foreign_id);
        for run in title_effective_runs(semantic) {
            paragraph.children.push(PreservedNode::Element(PreservedElement {
                name: source_element("Text", &namespace, prefix.as_deref()).name,
                attributes: run.attributes.iter().map(|attribute| PreservedAttribute {
                    name: source_name_from_qname(&attribute.name),
                    value: attribute.value.clone(),
                }).collect(),
                namespace_bindings: bindings.clone(),
                source_type: None,
                ownership: SourceOwnership::Opaque,
                dependency_confidence: DependencyConfidence::Unknown,
                anchor: None,
                children: vec![PreservedNode::Text(run.text.into())].into(),
            }));
        }
        let content = source_content_mut(&mut self.sections, SourceSection::TitlePage)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let mut position = None;
        for following in ordered.iter().skip(index + 1) {
            if let Some(found) = managed_child_index(&content.children, following) {
                position = Some(found);
                break;
            }
        }
        if position.is_none() {
            for preceding in ordered[..index].iter().rev() {
                if let Some(found) = managed_child_index(&content.children, preceding) {
                    position = Some(found + 1);
                    break;
                }
            }
        }
        let insert_at = position.unwrap_or(content.children.len());
        content.children.insert(insert_at, PreservedNode::Element(paragraph));
        self.anchors.push(SourceAnchor {
            id: anchor_id,
            target: AnchorTarget::ProjectElement(semantic.id.clone()),
            relation: AnchorRelation::Represents,
        });
        self.identities.push(ForeignIdentity::new(ForeignIdDomain::Paragraph, foreign_id.clone())?);
        self.invalidate_source_identity_use_index();
        Ok(SourceProvenance {
            format: SourceFormat::Fdx,
            section: SourceSection::TitlePage,
            ordinal: u32::try_from(index).map_err(|_| PreservationError::TooManyEvents)?,
            foreign_id: Some(foreign_id),
            foreign_role: semantic.role.map(|role| fdx_role_name(role).to_owned()),
        })
    }

    fn set_managed_role_projection(
        &mut self,
        id: &StableId,
        role: Option<&str>,
    ) -> Result<(), PreservationError> {
        let paragraph = self.managed_element_mut_indexed(id)?;
        paragraph.attributes.retain(|attribute| {
            !(attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Type")
        });
        paragraph.source_type = role.map(str::to_owned);
        if let Some(role) = role {
            push_source_attribute(paragraph, "Type", role);
        }
        Ok(())
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
        right_source.attributes.retain(|attribute| {
            is_managed_paragraph_attribute(attribute) || is_paragraph_id_attribute(attribute)
        });
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

    fn insert_managed_paragraph(
        &mut self,
        semantic: &ScreenplayElement,
        index: usize,
        ordered: &[StableId],
        groups: &mut [crate::dual_dialogue::DualDialogueGroup],
        canonical_members: &[ScreenplayElement],
        generated_member_provenance: &mut Vec<(StableId, SourceProvenance)>,
    ) -> Result<SourceProvenance, PreservationError> {
        if let Some(group_index) = groups.iter().position(|group| {
            group.member_ids().any(|member| member == &semantic.id)
        }) {
            return self.insert_nested_managed_paragraph(
                semantic,
                index,
                &mut groups[group_index],
                ordered,
                canonical_members,
                generated_member_provenance,
            );
        }
        let ManagedParagraphTemplate {
            paragraph_name,
            paragraph_bindings,
            text_name,
            text_bindings,
            id_spelling,
        } = {
            let content = source_content_mut(&mut self.sections, SourceSection::Content)
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            if let Some(template) = find_any_managed_paragraph(&content.children) {
                paragraph_template(template)?
            } else {
                let prefix = content.name.qualified_name.as_deref()
                    .and_then(|name| name.split_once(':').map(|(prefix, _)| prefix));
                let paragraph = source_element("Paragraph", &content.name.namespace_uri, prefix);
                let text = source_element("Text", &content.name.namespace_uri, prefix);
                ManagedParagraphTemplate {
                    paragraph_name: paragraph.name,
                    paragraph_bindings: content.namespace_bindings.clone(),
                    text_name: text.name,
                    text_bindings: content.namespace_bindings.clone(),
                    id_spelling: "id".to_owned(),
                }
            }
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

        let mut wrapped_member_slots = BTreeMap::new();
        if let Some(content) = source_content(&self.sections, SourceSection::Content) {
            for group in groups.iter() {
                let group_members = group.member_ids().cloned().collect::<BTreeSet<_>>();
                for (position, node) in content.children.iter().enumerate() {
                    let PreservedNode::Element(wrapper) = node else {
                        continue;
                    };
                    if wrapper.name.local_name != "Paragraph"
                        || wrapper.name.namespace_uri != content.name.namespace_uri
                        || !matches!(&wrapper.ownership, SourceOwnership::DualDialogueWrapper(owner) if owner == &group.id)
                    {
                        continue;
                    }
                    let duals = wrapper
                        .children
                        .iter()
                        .filter_map(|child| match child {
                            PreservedNode::Element(dual)
                                if dual.name.local_name == "DualDialogue"
                                    && dual.name.namespace_uri == wrapper.name.namespace_uri =>
                            {
                                Some(dual)
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    if duals.len() != 1 {
                        continue;
                    }
                    for child in &duals[0].children {
                        if let PreservedNode::Element(paragraph) = child {
                            if paragraph.name.local_name == "Paragraph"
                                && paragraph.name.namespace_uri == duals[0].name.namespace_uri
                            {
                                if let SourceOwnership::Managed(member) = &paragraph.ownership {
                                    if group_members.contains(member) {
                                        wrapped_member_slots.insert(member.clone(), position);
                                    }
                                }
                            }
                        }
                    }
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
        group: &mut crate::dual_dialogue::DualDialogueGroup,
        ordered: &[StableId],
        canonical_members: &[ScreenplayElement],
        generated_member_provenance: &mut Vec<(StableId, SourceProvenance)>,
    ) -> Result<SourceProvenance, PreservationError> {
        let content = source_content(&self.sections, SourceSection::Content)
            .ok_or(PreservationError::ManagedAnchorMissing)?;
        let namespace = content.name.namespace_uri.clone();
        let prefix = content.name.qualified_name.as_deref()
            .and_then(|name| name.split_once(':').map(|(prefix, _)| prefix.to_owned()));
        let namespace_bindings = content.namespace_bindings.clone();
        let member_ids = group.member_ids().cloned().collect::<Vec<_>>();
        let member_set = member_ids.iter().cloned().collect::<BTreeSet<_>>();
        let wrapper_snapshot = self.sections.iter().find_map(|section| {
            find_dual_wrapper_ref(&section.children, &group.id)
        }).map(|wrapper| (
            paragraph_foreign_id(wrapper).map(str::to_owned),
            wrapper.anchor.clone(),
        ));
        let wrapper_was_present = wrapper_snapshot.is_some();
        let old_wrapper_position = content.children.iter().position(|node| {
            node_contains_dual_wrapper(node, &group.id)
                || node_contains_managed_owner(node, &member_set)
        });

        // Stable semantic member identities are the only extraction key. This
        // preserves exact surviving Paragraph owners even when they are outside
        // the wrapper or nested below an obsolete source envelope.
        let mut existing_members = BTreeMap::new();
        for section in &mut self.sections {
            take_managed_owner_nodes(&mut section.children, &member_set, &mut existing_members);
        }

        let mut member_provenance = BTreeMap::new();
        for member_id in &member_ids {
            if existing_members.contains_key(member_id) {
                continue;
            }
            let member = canonical_members.iter().find(|member| &member.id == member_id)
                .ok_or(PreservationError::UnsupportedTextMapping)?;
            let foreign_id = self.fresh_foreign_paragraph_id()?;
            let anchor_id = self.fresh_anchor_id()?;
            existing_members.insert(member_id.clone(), PreservedNode::Element(generated_managed_paragraph(
                member,
                &namespace,
                prefix.as_deref(),
                &namespace_bindings,
                &foreign_id,
                &anchor_id,
            )));
            member_provenance.insert(member_id.clone(), (foreign_id, anchor_id));
        }

        if !wrapper_was_present {
            let old_foreign_ids = group.wrapper.outer_foreign_id.iter().cloned().collect();
            let old_anchors = group.wrapper.preservation_anchor.iter().cloned().collect();
            self.remove_wrapper_registrations(
                &old_foreign_ids,
                &BTreeSet::new(),
                &old_anchors,
                &group.id,
            );
        }

        let mut wrapper_foreign_id = wrapper_snapshot.as_ref()
            .and_then(|(foreign_id, _)| foreign_id.clone());
        if wrapper_foreign_id.is_none() {
            wrapper_foreign_id = Some(self.fresh_foreign_paragraph_id()?);
        }
        let wrapper_foreign_id = wrapper_foreign_id.expect("wrapper identity was allocated");
        let mut wrapper_anchor = wrapper_snapshot.as_ref().and_then(|(_, anchor)| {
            anchor.as_ref().filter(|anchor_id| self.anchors.iter().any(|anchor| {
                anchor.id == **anchor_id
                    && matches!(&anchor.target, AnchorTarget::DualDialogueWrapper(owner) if owner == &group.id)
                    && anchor.relation == AnchorRelation::Represents
            })).cloned()
        });
        if wrapper_anchor.is_none() {
            wrapper_anchor = Some(self.fresh_anchor_id()?);
        }
        let wrapper_anchor = wrapper_anchor.expect("wrapper anchor was allocated");

        // A present wrapper can still have stale semantic provenance. Remove
        // only the obsolete wrapper registrations before installing the current
        // source identity; member registrations remain owner-local.
        if wrapper_was_present {
            let stale_wrapper_ids = group.wrapper.outer_foreign_id.iter()
                .filter(|old| old.as_str() != wrapper_foreign_id)
                .cloned().collect::<BTreeSet<_>>();
            let stale_wrapper_anchors = group.wrapper.preservation_anchor.iter()
                .filter(|old| *old != &wrapper_anchor)
                .cloned().collect::<BTreeSet<_>>();
            if !stale_wrapper_ids.is_empty() || !stale_wrapper_anchors.is_empty() {
                self.remove_wrapper_registrations(
                    &stale_wrapper_ids,
                    &BTreeSet::new(),
                    &stale_wrapper_anchors,
                    &group.id,
                );
            }
        }

        let wrapper_position = if wrapper_was_present {
            old_wrapper_position.unwrap_or(index)
        } else {
            let mut generated = source_element("Paragraph", &namespace, prefix.as_deref());
            generated.source_type = Some("General".to_owned());
            generated.ownership = SourceOwnership::DualDialogueWrapper(group.id.clone());
            generated.anchor = Some(wrapper_anchor.clone());
            generated.namespace_bindings = namespace_bindings.clone();
            push_source_attribute(&mut generated, "Type", "General");
            push_source_attribute(&mut generated, "id", &wrapper_foreign_id);
            let mut dual = source_element("DualDialogue", &namespace, prefix.as_deref());
            dual.namespace_bindings = namespace_bindings.clone();
            dual.children.push(PreservedNode::Element(generated_spell_check_block(
                &namespace,
                prefix.as_deref(),
                &namespace_bindings,
            )));
            generated.children.push(PreservedNode::Element(dual));

            let content = source_content_mut(&mut self.sections, SourceSection::Content)
                .ok_or(PreservationError::ManagedAnchorMissing)?;
            let position = old_wrapper_position.or_else(|| {
                ordered.iter().skip(index + 1).find_map(|following| {
                    let following_set = BTreeSet::from([following.clone()]);
                    content.children.iter().position(|node| node_contains_managed_owner(node, &following_set))
                })
            }).or_else(|| {
                ordered[..index].iter().rev().find_map(|preceding| {
                    let preceding_set = BTreeSet::from([preceding.clone()]);
                    content.children.iter().rposition(|node| node_contains_managed_owner(node, &preceding_set))
                        .map(|position| position + 1)
                })
            }).unwrap_or(content.children.len());
            content.children.insert(position, PreservedNode::Element(generated));
            position
        };

        if wrapper_was_present {
            let wrapper = self.sections.iter_mut().find_map(|section| {
                find_dual_wrapper_mut(&mut section.children, &group.id)
            }).ok_or(PreservationError::ManagedAnchorMissing)?;
            if paragraph_foreign_id(wrapper).is_none() {
                wrapper.attributes.retain(|attribute| {
                    attribute.name.local_name != "id" || attribute.name.namespace_uri.is_some()
                });
                push_source_attribute(wrapper, "id", &wrapper_foreign_id);
            }
            if !wrapper.attributes.iter().any(|attribute| {
                attribute.name.local_name == "Type" && attribute.name.namespace_uri.is_none()
            }) {
                push_source_attribute(wrapper, "Type", "General");
            }
            wrapper.source_type = Some("General".to_owned());
            if wrapper.anchor.is_none() {
                wrapper.anchor = Some(wrapper_anchor.clone());
            }
        }

        // Repair stale wrapper-only registrations, then register the current
        // owner identities. Existing member registrations are left untouched.
        if !self.identities.iter().any(|identity| {
            identity.domain == ForeignIdDomain::Paragraph && identity.value == wrapper_foreign_id
        }) {
            self.identities.push(ForeignIdentity::new(
                ForeignIdDomain::Paragraph,
                wrapper_foreign_id.clone(),
            )?);
        }
        if !self.anchors.iter().any(|anchor| anchor.id == wrapper_anchor) {
            self.anchors.push(SourceAnchor {
                id: wrapper_anchor.clone(),
                target: AnchorTarget::DualDialogueWrapper(group.id.clone()),
                relation: AnchorRelation::Represents,
            });
        }
        group.wrapper.outer_foreign_id = Some(wrapper_foreign_id.clone());
        group.wrapper.preservation_anchor = Some(wrapper_anchor.clone());
        group.wrapper.source = Some(SourceProvenance {
            format: SourceFormat::Fdx,
            section: SourceSection::Content,
            ordinal: u32::try_from(wrapper_position).map_err(|_| PreservationError::TooManyEvents)?,
            foreign_id: Some(wrapper_foreign_id),
            foreign_role: Some("General".to_owned()),
        });

        // Reparent exact extant member owners into the wrapper's canonical
        // DualDialogue list, preserving every member shell and opaque attachment.
        {
            let wrapper = self.sections.iter_mut().find_map(|section| {
                find_dual_wrapper_mut(&mut section.children, &group.id)
            }).ok_or(PreservationError::ManagedAnchorMissing)?;
            let mut dual = find_dual_dialogue_mut(&mut wrapper.children, &namespace);
            if dual.is_none() {
                let mut generated = source_element("DualDialogue", &namespace, prefix.as_deref());
                generated.namespace_bindings = namespace_bindings.clone();
                let spell = generated_spell_check_block(&namespace, prefix.as_deref(), &namespace_bindings);
                generated.children.push(PreservedNode::Element(spell));
                wrapper.children.push(PreservedNode::Element(generated));
                dual = wrapper.children.last_mut().and_then(|node| match node {
                    PreservedNode::Element(element) => Some(element),
                    _ => None,
                });
            }
            let dual = dual.ok_or(PreservationError::ManagedAnchorMissing)?;
            let spell_index = dual.children.iter().position(|node| matches!(node,
                PreservedNode::Element(element) if element.name.local_name == "SpellCheckIgnoreLists"
                    && element.name.namespace_uri == namespace
            ));
            let mut insert_at = spell_index.map_or(0, |position| position + 1);
            if spell_index.is_none() {
                dual.children.insert(0, PreservedNode::Element(generated_spell_check_block(
                    &namespace,
                    prefix.as_deref(),
                    &namespace_bindings,
                )));
                insert_at = 1;
            }
            for member_id in &member_ids {
                if let Some(node) = existing_members.remove(member_id) {
                    dual.children.insert(insert_at, node);
                    insert_at += 1;
                }
            }
        }
        for (member_id, (foreign_id, anchor_id)) in member_provenance {
            self.anchors.push(SourceAnchor {
                id: anchor_id.clone(),
                target: AnchorTarget::ProjectElement(member_id.clone()),
                relation: AnchorRelation::Represents,
            });
            self.identities.push(ForeignIdentity::new(
                ForeignIdDomain::Paragraph,
                foreign_id.clone(),
            )?);
            let member = canonical_members.iter().find(|member| member.id == member_id)
                .ok_or(PreservationError::UnsupportedTextMapping)?;
            let provenance = SourceProvenance {
                format: SourceFormat::Fdx,
                section: SourceSection::Content,
                ordinal: u32::try_from(ordered.iter().position(|id| id == &member_id).unwrap_or(index))
                    .map_err(|_| PreservationError::TooManyEvents)?,
                foreign_id: Some(foreign_id),
                foreign_role: Some(fdx_role_name(member.role).to_owned()),
            };
            generated_member_provenance.push((member_id, provenance));
        }
        generated_member_provenance.iter().find(|(id, _)| id == &semantic.id)
            .map(|(_, provenance)| provenance.clone())
            .ok_or(PreservationError::ManagedAnchorMissing)
    }

    fn remove_managed_paragraph(
        &mut self,
        id: &StableId,
        _provenance: Option<&SourceProvenance>,
    ) -> Result<(), PreservationError> {
        self.remove_managed_paragraph_without_rebuild(id, _provenance)?;
        self.rebuild_managed_paragraph_index();
        Ok(())
    }

    fn remove_managed_paragraph_without_rebuild(
        &mut self,
        id: &StableId,
        _provenance: Option<&SourceProvenance>,
    ) -> Result<(), PreservationError> {
        let mut removed_foreign_ids = BTreeSet::new();
        let mut removed_project_ids = BTreeSet::from([id.clone()]);
        let mut removed_anchor_ids = BTreeSet::new();
        let mut removed_any = false;
        for section in &mut self.sections {
            removed_any |= remove_managed_owner_nodes(
                &mut section.children,
                id,
                &mut removed_foreign_ids,
                &mut removed_project_ids,
                &mut removed_anchor_ids,
            );
        }
        // A missing source occurrence is still a valid semantic deletion. Raw
        // surviving XML uses may become orphaned; they are not an edit-time veto.
        if !removed_any {
            removed_project_ids.insert(id.clone());
        }
        self.cleanup_removed_owner_bindings(&removed_foreign_ids, &removed_project_ids, &removed_anchor_ids);
        if removed_any {
            self.managed_paragraph_index = None;
        }
        Ok(())
    }

    fn cleanup_removed_owner_bindings(
        &mut self,
        removed_foreign_ids: &BTreeSet<String>,
        removed_project_ids: &BTreeSet<StableId>,
        removed_anchor_ids: &BTreeSet<SourceAnchorId>,
    ) {
        let stale_anchors = self.anchors.iter().filter(|anchor| {
            removed_anchor_ids.contains(&anchor.id)
                || match &anchor.target {
                    AnchorTarget::ProjectElement(id) => removed_project_ids.contains(id),
                    AnchorTarget::Foreign(identity) => {
                        identity.domain == ForeignIdDomain::Paragraph
                            && removed_foreign_ids.contains(&identity.value)
                    }
                    AnchorTarget::DualDialogueWrapper(id) => removed_project_ids.contains(id),
                }
        }).map(|anchor| anchor.id.clone()).collect::<BTreeSet<_>>();
        if !stale_anchors.is_empty() {
            for section in &mut self.sections {
                clear_stale_anchor_bindings(&mut section.children, &stale_anchors, removed_foreign_ids);
            }
        }
        self.identities.retain(|identity| {
            !(identity.domain == ForeignIdDomain::Paragraph
                && removed_foreign_ids.contains(&identity.value))
        });
        self.references.retain(|reference| {
            !paragraph_reference_touches(reference, removed_foreign_ids)
        });
        self.board_graph.references.retain(|reference| {
            !paragraph_reference_touches(reference, removed_foreign_ids)
        });
        for id in removed_project_ids {
            self.source_layout_settings.direct_paragraph_overrides.remove(id);
        }
        self.anchors.retain(|anchor| !stale_anchors.contains(&anchor.id));
        self.invalidate_source_identity_use_index();
    }

    fn remove_wrapper_registrations(
        &mut self,
        removed_foreign_ids: &BTreeSet<String>,
        removed_project_ids: &BTreeSet<StableId>,
        removed_anchor_ids: &BTreeSet<SourceAnchorId>,
        group_id: &StableId,
    ) {
        let mut removed_project_ids = removed_project_ids.clone();
        removed_project_ids.insert(group_id.clone());
        self.cleanup_removed_owner_bindings(
            removed_foreign_ids,
            &removed_project_ids,
            removed_anchor_ids,
        );
        self.source_layout_settings.direct_paragraph_overrides.remove(group_id);
        self.rebuild_managed_paragraph_index();
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
        if templates.iter().any(|template| template.children.iter().any(|node| {
            !matches!(node, PreservedNode::Text(_) | PreservedNode::CData(_))
        })) {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        let text_indexes = paragraph.children.iter().enumerate().filter_map(|(index, node)| {
            matches!(node, PreservedNode::Element(element) if is_text_element(element, paragraph))
                .then_some(index)
        }).collect::<Vec<_>>();
        if text_indexes.windows(2).any(|pair| {
            paragraph.children[pair[0] + 1..pair[1]].iter().any(|node| match node {
                PreservedNode::Text(text) | PreservedNode::CData(text) => !text.trim().is_empty(),
                _ => true,
            })
        }) {
            return Err(PreservationError::UnsupportedTextMapping);
        }
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
                PreservedNode::Element(element) if !is_text_element(element, paragraph) => {
                    rebuilt.push(child.clone())
                }
                PreservedNode::Comment(_) | PreservedNode::ProcessingInstruction { .. } => {
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
            PreservedNode::Element(_) | PreservedNode::Comment(_)
            | PreservedNode::ProcessingInstruction { .. } => {}
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

fn merge_adjacent_equivalent_runs(mut runs: Vec<TextRun>) -> Vec<TextRun> {
    let mut merged = Vec::<TextRun>::with_capacity(runs.len());
    for run in runs.drain(..) {
        match merged.last_mut() {
            Some(previous) if previous.attributes == run.attributes => {
                previous.text.push_str(&run.text);
            }
            _ => merged.push(run),
        }
    }
    merged
}

fn semantic_text_runs_equal(left: &ScreenplayElement, right: &ScreenplayElement) -> bool {
    match (left.runs.is_empty(), right.runs.is_empty()) {
        (false, false) => left.runs == right.runs,
        (true, true) => true,
        (true, false) => {
            !left.text.is_empty()
                && right.runs.len() == 1
                && right.runs[0].text == left.text
                && right.runs[0].attributes.is_empty()
        }
        (false, true) => {
            !right.text.is_empty()
                && left.runs.len() == 1
                && left.runs[0].text == right.text
                && left.runs[0].attributes.is_empty()
        }
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

fn source_content_mut(
    sections: &mut [PreservedSection],
    section: SourceSection,
) -> Option<&mut PreservedElement> {
    let (outer_name, content_name) = match section {
        SourceSection::Content => ("FinalDraft", "Content"),
        SourceSection::TitlePage => ("TitlePage", "Content"),
    };
    for source_section in sections {
        for node in &mut source_section.children {
            let PreservedNode::Element(outer) = node else { continue };
            if outer.name.local_name != outer_name { continue }
            let namespace = outer.name.namespace_uri.clone();
            for child in &mut outer.children {
                if let PreservedNode::Element(content) = child {
                    if content.name.local_name == content_name
                        && content.name.namespace_uri == namespace
                    {
                        return Some(content);
                    }
                }
            }
        }
    }
    None
}

fn source_content(
    sections: &[PreservedSection],
    section: SourceSection,
) -> Option<&PreservedElement> {
    let (outer_name, content_name) = match section {
        SourceSection::Content => ("FinalDraft", "Content"),
        SourceSection::TitlePage => ("TitlePage", "Content"),
    };
    for source_section in sections {
        for node in &source_section.children {
            let PreservedNode::Element(outer) = node else { continue };
            if outer.name.local_name != outer_name { continue }
            let namespace = outer.name.namespace_uri.as_deref();
            for child in &outer.children {
                if let PreservedNode::Element(content) = child {
                    if content.name.local_name == content_name
                        && content.name.namespace_uri.as_deref() == namespace
                    {
                        return Some(content);
                    }
                }
            }
        }
    }
    None
}

fn generated_managed_paragraph(
    semantic: &ScreenplayElement,
    namespace: &Option<String>,
    prefix: Option<&str>,
    namespace_bindings: &[NamespaceBinding],
    foreign_id: &str,
    anchor_id: &SourceAnchorId,
) -> PreservedElement {
    let mut paragraph = source_element("Paragraph", namespace, prefix);
    paragraph.attributes = vec![
        PreservedAttribute {
            name: plain_source_name("Type"),
            value: fdx_role_name(semantic.role).to_owned(),
        },
        PreservedAttribute {
            name: plain_source_name("id"),
            value: foreign_id.to_owned(),
        },
    ];
    paragraph.namespace_bindings = namespace_bindings.to_vec();
    paragraph.source_type = Some(fdx_role_name(semantic.role).to_owned());
    paragraph.ownership = SourceOwnership::Managed(semantic.id.clone());
    paragraph.anchor = Some(anchor_id.clone());
    for run in semantic_text_runs(semantic) {
        let mut text = source_element("Text", namespace, prefix);
        text.attributes = run.attributes.iter().map(|attribute| PreservedAttribute {
            name: source_name_from_qname(&attribute.name),
            value: attribute.value.clone(),
        }).collect();
        text.namespace_bindings = namespace_bindings.to_vec();
        text.children.push(PreservedNode::Text(run.text.into()));
        paragraph.children.push(PreservedNode::Element(text));
    }
    paragraph
}

fn generated_spell_check_block(
    namespace: &Option<String>,
    prefix: Option<&str>,
    namespace_bindings: &[NamespaceBinding],
) -> PreservedElement {
    let mut spell = source_element("SpellCheckIgnoreLists", namespace, prefix);
    spell.namespace_bindings = namespace_bindings.to_vec();
    for name in ["IgnoredRanges", "IgnoredWords"] {
        let mut list = source_element(name, namespace, prefix);
        list.namespace_bindings = namespace_bindings.to_vec();
        spell.children.push(PreservedNode::Element(list));
    }
    spell
}

fn find_dual_wrapper_mut<'a>(
    nodes: &'a mut [PreservedNode],
    group_id: &StableId,
) -> Option<&'a mut PreservedElement> {
    for node in nodes {
        let PreservedNode::Element(element) = node else { continue };
        if matches!(&element.ownership, SourceOwnership::DualDialogueWrapper(owner) if owner == group_id) {
            return Some(element);
        }
        if let Some(found) = find_dual_wrapper_mut(&mut element.children, group_id) {
            return Some(found);
        }
    }
    None
}

fn find_dual_wrapper_ref<'a>(
    nodes: &'a [PreservedNode],
    group_id: &StableId,
) -> Option<&'a PreservedElement> {
    for node in nodes {
        let PreservedNode::Element(element) = node else { continue };
        if matches!(&element.ownership, SourceOwnership::DualDialogueWrapper(owner) if owner == group_id) {
            return Some(element);
        }
        if let Some(found) = find_dual_wrapper_ref(&element.children, group_id) {
            return Some(found);
        }
    }
    None
}

fn node_contains_dual_wrapper(node: &PreservedNode, group_id: &StableId) -> bool {
    match node {
        PreservedNode::Element(element) => {
            matches!(&element.ownership, SourceOwnership::DualDialogueWrapper(owner) if owner == group_id)
                || element.children.iter().any(|child| node_contains_dual_wrapper(child, group_id))
        }
        _ => false,
    }
}

fn find_dual_dialogue_mut<'a>(
    nodes: &'a mut [PreservedNode],
    namespace: &Option<String>,
) -> Option<&'a mut PreservedElement> {
    for node in nodes {
        let PreservedNode::Element(element) = node else { continue };
        if element.name.local_name == "DualDialogue" && &element.name.namespace_uri == namespace {
            return Some(element);
        }
        if let Some(found) = find_dual_dialogue_mut(&mut element.children, namespace) {
            return Some(found);
        }
    }
    None
}

fn append_recovered_direct_attributes(
    paragraph: &mut PreservedElement,
    overrides: crate::formatting::SourceParagraphOverrides,
) {
    let spec = overrides.paragraph;
    if let Some(value) = spec.alignment {
        push_source_attribute(paragraph, "Alignment", value.fdx_value());
    }
    for (name, value) in [
        ("FirstIndent", spec.first_indent),
        ("LeftIndent", spec.left_indent),
        ("RightIndent", spec.right_indent),
    ] {
        if let Some(value) = value {
            push_source_attribute(paragraph, name, &fixed_inches_string(value));
        }
    }
    if spec.leading_regular == Some(true) {
        push_source_attribute(paragraph, "Leading", "Regular");
    }
    if let Some(value) = spec.space_before {
        push_source_attribute(paragraph, "SpaceBefore", &value.decimal_string());
    }
    if spec.spacing_single == Some(true) {
        push_source_attribute(paragraph, "Spacing", "1");
    }
    if let Some(value) = overrides.starts_new_page {
        push_source_attribute(paragraph, "StartsNewPage", if value { "Yes" } else { "No" });
    }
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

fn take_managed_owner_nodes(
    nodes: &mut Vec<PreservedNode>,
    wanted: &BTreeSet<StableId>,
    found: &mut BTreeMap<StableId, PreservedNode>,
) {
    let mut index = 0;
    while index < nodes.len() {
        let owner = match nodes.get(index) {
            Some(PreservedNode::Element(element)) => match &element.ownership {
                SourceOwnership::Managed(id) if wanted.contains(id) => Some(id.clone()),
                _ => None,
            },
            _ => None,
        };
        if let Some(owner) = owner {
            let node = nodes.remove(index);
            found.entry(owner).or_insert(node);
            continue;
        }
        if let Some(PreservedNode::Element(element)) = nodes.get_mut(index) {
            take_managed_owner_nodes(&mut element.children, wanted, found);
        }
        index += 1;
    }
}

fn contains_managed_owner(nodes: &[PreservedNode], id: &StableId) -> bool {
    nodes.iter().any(|node| match node {
        PreservedNode::Element(element) => {
            matches!(&element.ownership, SourceOwnership::Managed(owner) if owner == id)
                || contains_managed_owner(&element.children, id)
        }
        _ => false,
    })
}

fn node_contains_managed_owner(node: &PreservedNode, ids: &BTreeSet<StableId>) -> bool {
    match node {
        PreservedNode::Element(element) => {
            matches!(&element.ownership, SourceOwnership::Managed(id) if ids.contains(id))
                || element.children.iter().any(|child| node_contains_managed_owner(child, ids))
        }
        _ => false,
    }
}

fn remove_dual_wrapper_nodes(
    nodes: &mut Vec<PreservedNode>,
    group_id: &StableId,
    found_wrapper: &mut bool,
    removed_foreign_ids: &mut BTreeSet<String>,
    removed_project_ids: &mut BTreeSet<StableId>,
    removed_anchor_ids: &mut BTreeSet<SourceAnchorId>,
) {
    let mut index = 0;
    while index < nodes.len() {
        let is_target = matches!(nodes.get(index), Some(PreservedNode::Element(element))
            if matches!(&element.ownership, SourceOwnership::DualDialogueWrapper(owner) if owner == group_id));
        if is_target {
            if let Some(PreservedNode::Element(wrapper)) = nodes.get(index) {
                collect_owner_registrations(
                    wrapper,
                    removed_foreign_ids,
                    removed_project_ids,
                    removed_anchor_ids,
                );
            }
            *found_wrapper = true;
            nodes.remove(index);
            continue;
        }
        if let Some(PreservedNode::Element(element)) = nodes.get_mut(index) {
            remove_dual_wrapper_nodes(
                &mut element.children,
                group_id,
                found_wrapper,
                removed_foreign_ids,
                removed_project_ids,
                removed_anchor_ids,
            );
        }
        index += 1;
    }
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

fn same_managed_paragraph_shell(
    before: &PreservedElement,
    after: &PreservedElement,
) -> bool {
    let opaque_attributes = |element: &PreservedElement| element.attributes.iter().filter(|attribute| {
        !is_paragraph_id_attribute(attribute) && !is_managed_paragraph_attribute(attribute)
    }).cloned().collect::<Vec<_>>();
    let id_attributes = |element: &PreservedElement| element.attributes.iter()
        .filter(|attribute| is_paragraph_id_attribute(attribute)).cloned().collect::<Vec<_>>();
    before.name == after.name
        && before.namespace_bindings == after.namespace_bindings
        && before.ownership == after.ownership
        && before.dependency_confidence == after.dependency_confidence
        && before.anchor == after.anchor
        && (before.source_type == after.source_type || before.name.local_name == "Paragraph")
        && opaque_attributes(before) == opaque_attributes(after)
        && id_attributes(before) == id_attributes(after)
}

fn same_managed_owner_attributes_except_type(
    before: &PreservedElement,
    after: &PreservedElement,
) -> bool {
    let unchanged = |attribute: &&PreservedAttribute| {
        is_managed_paragraph_attribute(attribute)
            && !(attribute.name.namespace_uri.is_none()
                && attribute.name.local_name == "Type")
    };
    before.attributes.iter().filter(unchanged).eq(after.attributes.iter().filter(unchanged))
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

fn source_event_count_in_element(element: &PreservedElement) -> usize {
    let mut total = 0usize;
    let mut stack = vec![element];
    while let Some(current) = stack.pop() {
        total = total
            .saturating_add(1)
            .saturating_add(current.attributes.len())
            .saturating_add(current.namespace_bindings.len());
        stack.extend(current.children.iter().filter_map(|child| match child {
            PreservedNode::Element(nested) => Some(nested),
            _ => None,
        }));
        total = total.saturating_add(current.children.len().saturating_sub(
            current.children.iter().filter(|child| matches!(child, PreservedNode::Element(_))).count(),
        ));
    }
    total
}

fn validate_scoped_owner_fields(
    owner: &PreservedElement,
    owner_depth: usize,
) -> Result<(), PreservationError> {
    let mut stack = vec![(owner, owner_depth)];
    while let Some((element, depth)) = stack.pop() {
        if depth > MAX_SOURCE_DEPTH {
            return Err(PreservationError::ExcessiveDepth);
        }
        validate_name(&element.name)?;
        if let Some(source_type) = &element.source_type {
            validate_field(source_type)?;
        }
        if element.attributes.len() > MAX_ATTRIBUTES_PER_ELEMENT
            || element.namespace_bindings.len() > MAX_ATTRIBUTES_PER_ELEMENT
        {
            return Err(PreservationError::TooManyAttributes);
        }
        for attribute in &element.attributes {
            validate_name(&attribute.name)?;
            validate_field(&attribute.value)?;
        }
        for binding in &element.namespace_bindings {
            validate_field(&binding.prefix)?;
            validate_field(&binding.namespace_uri)?;
        }
        validate_ownership(&element.ownership)?;
        if !matches!(&element.ownership, SourceOwnership::Opaque) && element.anchor.is_none() {
            return Err(PreservationError::InvalidAnchor);
        }
        for child in &element.children {
            match child {
                PreservedNode::Element(nested) => {
                    stack.push((nested, depth.saturating_add(1)));
                }
                PreservedNode::ProcessingInstruction { target, .. } => {
                    if target.is_empty() {
                        return Err(PreservationError::InvalidSourceName);
                    }
                    validate_field(target)?;
                }
                PreservedNode::Text(_)
                | PreservedNode::CData(_)
                | PreservedNode::Comment(_) => {}
            }
        }
    }
    Ok(())
}

fn same_direct_owner_attachments(before: &PreservedElement, after: &PreservedElement) -> bool {
    let unchanged = |owner: &PreservedElement, node: &&PreservedNode| {
        !matches!(node, PreservedNode::Element(element) if is_text_element(element, owner))
    };
    before.children.iter().filter(|node| unchanged(before, node))
        .eq(after.children.iter().filter(|node| unchanged(after, node)))
}

fn is_plain_managed_paragraph(paragraph: &PreservedElement) -> bool {
    paragraph.name.local_name == "Paragraph"
        && paragraph.children.iter().all(|node| match node {
            PreservedNode::Text(text) | PreservedNode::CData(text) => text.trim().is_empty(),
            PreservedNode::Element(element) if is_text_element(element, paragraph) => true,
            _ => true,
        })
}

fn owner_anchor_bindings(
    owner: &PreservedElement,
) -> Result<BTreeMap<SourceAnchorId, SourceOwnership>, PreservationError> {
    let mut bindings = BTreeMap::new();
    let mut stack = vec![owner];
    while let Some(element) = stack.pop() {
        if let Some(anchor) = &element.anchor {
            if bindings.insert(anchor.clone(), element.ownership.clone()).is_some() {
                return Err(PreservationError::DuplicateAnchor);
            }
        } else if !matches!(&element.ownership, SourceOwnership::Opaque) {
            return Err(PreservationError::InvalidAnchor);
        }
        stack.extend(element.children.iter().filter_map(|child| match child {
            PreservedNode::Element(nested) => Some(nested),
            _ => None,
        }));
    }
    Ok(bindings)
}

fn same_owner_anchor_bindings(
    before: &PreservedElement,
    after: &PreservedElement,
) -> bool {
    match (owner_anchor_bindings(before), owner_anchor_bindings(after)) {
        (Ok(before), Ok(after)) => before == after,
        _ => false,
    }
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

fn remove_managed_owner_nodes(
    nodes: &mut Vec<PreservedNode>,
    id: &StableId,
    foreign_ids: &mut BTreeSet<String>,
    project_ids: &mut BTreeSet<StableId>,
    anchor_ids: &mut BTreeSet<SourceAnchorId>,
) -> bool {
    let mut found = false;
    let mut retained = Vec::with_capacity(nodes.len());
    for mut node in nodes.drain(..) {
        if let PreservedNode::Element(element) = &mut node {
            if matches!(&element.ownership, SourceOwnership::Managed(owner) if owner == id) {
                collect_owner_registrations(element, foreign_ids, project_ids, anchor_ids);
                found = true;
                continue;
            }
            found |= remove_managed_owner_nodes(
                &mut element.children,
                id,
                foreign_ids,
                project_ids,
                anchor_ids,
            );
        }
        retained.push(node);
    }
    *nodes = retained;
    found
}

fn collect_owner_registrations(
    root: &PreservedElement,
    foreign_ids: &mut BTreeSet<String>,
    project_ids: &mut BTreeSet<StableId>,
    anchor_ids: &mut BTreeSet<SourceAnchorId>,
) {
    fn collect(
        element: &PreservedElement,
        parent: Option<&PreservedElement>,
        grandparent: Option<&PreservedElement>,
        in_user_data: bool,
        foreign_ids: &mut BTreeSet<String>,
        project_ids: &mut BTreeSet<StableId>,
        anchor_ids: &mut BTreeSet<SourceAnchorId>,
    ) {
        if let SourceOwnership::Managed(id) = &element.ownership {
            project_ids.insert(id.clone());
        }
        if let Some(anchor) = &element.anchor {
            anchor_ids.insert(anchor.clone());
        }
        let is_user_data = parent.is_some_and(|owner| {
            owner.name.local_name == "Paragraph"
                && is_user_paragraph_data_name(&element.name, &owner.name)
        });
        let is_alternate_copy = element.name.local_name == "Paragraph"
            && parent.is_some_and(|owner| {
                owner.name.local_name == "Alt"
                    && owner.name.namespace_uri == element.name.namespace_uri
            })
            && grandparent.is_some_and(|owner| {
                owner.name.local_name == "AltCollection"
                    && owner.name.namespace_uri == element.name.namespace_uri
            });
        if !in_user_data && !is_alternate_copy && element.name.local_name == "Paragraph" {
            if let Some(id) = paragraph_foreign_id(element) {
                foreign_ids.insert(id.to_owned());
            }
        }
        for child in &element.children {
            if let PreservedNode::Element(child) = child {
                collect(
                    child,
                    Some(element),
                    parent,
                    in_user_data || is_user_data,
                    foreign_ids,
                    project_ids,
                    anchor_ids,
                );
            }
        }
    }
    collect(root, None, None, false, foreign_ids, project_ids, anchor_ids);
}

fn clear_stale_anchor_bindings(
    nodes: &mut [PreservedNode],
    stale_anchors: &BTreeSet<SourceAnchorId>,
    removed_foreign_ids: &BTreeSet<String>,
) {
    for node in nodes {
        let PreservedNode::Element(element) = node else { continue };
        if element.anchor.as_ref().is_some_and(|anchor| stale_anchors.contains(anchor)) {
            element.anchor = None;
        }
        if matches!(&element.ownership, SourceOwnership::Attached(identity)
            if identity.domain == ForeignIdDomain::Paragraph
                && removed_foreign_ids.contains(&identity.value))
        {
            element.ownership = SourceOwnership::Opaque;
            element.anchor = None;
        }
        clear_stale_anchor_bindings(&mut element.children, stale_anchors, removed_foreign_ids);
    }
}

fn paragraph_reference_touches(
    reference: &ForeignReference,
    removed_foreign_ids: &BTreeSet<String>,
) -> bool {
    [&reference.source, &reference.target].into_iter().any(|identity| {
        identity.domain == ForeignIdDomain::Paragraph
            && removed_foreign_ids.contains(&identity.value)
    })
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

fn same_title_paragraph_shell(left: &PreservedElement, right: &PreservedElement) -> bool {
    left.name == right.name
        && left.source_type == right.source_type
        && managed_paragraph_attributes(left) == managed_paragraph_attributes(right)
}

fn is_paragraph_id_attribute(attribute: &PreservedAttribute) -> bool {
    attribute.name.namespace_uri.is_none()
        && attribute.name.local_name.eq_ignore_ascii_case("id")
}

fn is_managed_paragraph_attribute(attribute: &PreservedAttribute) -> bool {
    attribute.name.namespace_uri.is_none()
        && matches!(attribute.name.local_name.as_str(),
            "Type" | "Alignment" | "FirstIndent" | "Leading" | "LeftIndent"
            | "RightIndent" | "SpaceBefore" | "Spacing" | "StartsNewPage")
}

fn managed_paragraph_attributes(paragraph: &PreservedElement) -> Vec<&PreservedAttribute> {
    paragraph.attributes.iter().filter(|attribute| is_managed_paragraph_attribute(attribute)).collect()
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
                templates.push(element.clone());
            }
            PreservedNode::Element(element) if !is_text_element(element, paragraph) => {}
            PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => {}
            PreservedNode::Comment(_) | PreservedNode::ProcessingInstruction { .. } => {}
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
            PreservedNode::Element(element) if !is_text_element(element, &source) => {
                // Split ownership is left-biased: opaque source data remains
                // attached only to the original title paragraph.
                left_children.push(child.clone());
            }
            PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => {
                // Formatting whitespace remains at its original source position
                // with the retained left occurrence; no opaque payload is moved.
                left_children.push(child.clone());
            }
            PreservedNode::Comment(_) | PreservedNode::ProcessingInstruction { .. } => {
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
                        let source_text = text_element.children.iter().filter_map(|node| match node {
                            PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
                            _ => None,
                        }).collect::<String>();
                        if source_text != run.text {
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
                        }
                        rebuilt.push(PreservedNode::Element(text_element));
                    }
                    inserted = true;
                }
            }
            PreservedNode::Element(element) if !is_text_element(element, recipient_source) => {
                rebuilt.push(child.clone());
            }
            PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => {
                rebuilt.push(child.clone());
            }
            PreservedNode::Comment(_) | PreservedNode::ProcessingInstruction { .. } => {
                rebuilt.push(child.clone());
            }
            _ => return Err(PreservationError::StructuralAttachmentConflict),
        }
    }
    if !inserted {
        let mut generated = Vec::with_capacity(runs.len());
        for run in &runs {
            let template = templates.iter().find(|template| {
                template.attributes.len() == run.attributes.len()
                    && template.attributes.iter().zip(&run.attributes)
                        .all(|(source, semantic)| source_attribute_matches_run(source, semantic))
            }).ok_or(PreservationError::UnsupportedTextMapping)?;
            let mut text_element = template.clone();
            let source_text = text_element.children.iter().filter_map(|node| match node {
                PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
                _ => None,
            }).collect::<String>();
            if source_text != run.text {
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
            }
            generated.push(PreservedNode::Element(text_element));
        }
        let insert_at = rebuilt.iter().position(|node| matches!(node,
            PreservedNode::Element(element) if !is_text_element(element, recipient_source)
        )).unwrap_or(rebuilt.len());
        rebuilt.splice(insert_at..insert_at, generated);
    }
    if runs.iter().map(|run| run.text.as_str()).collect::<String>() != semantic.text {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    Ok(rebuilt)
}

fn paragraph_text_nodes_with_supported_attachments(
    paragraph: &PreservedElement,
) -> Result<Vec<PreservedNode>, PreservationError> {
    let mut alts_count = 0usize;
    let mut user_data_count = 0usize;
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
                if element.name.local_name == "Text"
                    && element.name.namespace_uri == paragraph.name.namespace_uri =>
            {
                return Err(PreservationError::UnsupportedRunTopology);
            }
            PreservedNode::Element(element)
                if element.name.local_name == "Alts"
                    && element.name.namespace_uri == paragraph.name.namespace_uri =>
            {
                alts_count = alts_count.saturating_add(1);
                if alts_count > 1 {
                    return Err(PreservationError::StructuralAttachmentConflict);
                }
            }
            PreservedNode::Element(element)
                if is_user_paragraph_data_name(&element.name, &paragraph.name) =>
            {
                user_data_count = user_data_count.saturating_add(1);
                if user_data_count > 1 {
                    return Err(PreservationError::StructuralAttachmentConflict);
                }
            }
            // Unknown direct siblings belong to their paragraph owner. They are
            // intentionally omitted from the returned managed Text slot list.
            _ => {}
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

fn text_element_plain_content(element: &PreservedElement) -> String {
    element.children.iter().filter_map(|child| match child {
        PreservedNode::Text(text) | PreservedNode::CData(text) => Some(text.as_str()),
        _ => None,
    }).collect()
}

fn new_text_element_for_run(paragraph: &PreservedElement, run: &TextRun) -> PreservedElement {
    let (name, namespace_bindings) = paragraph_text_name(paragraph);
    PreservedElement {
        name,
        attributes: run.attributes.iter().map(|attribute| PreservedAttribute {
            name: source_name_from_qname(&attribute.name),
            value: attribute.value.clone(),
        }).collect(),
        namespace_bindings,
        source_type: None,
        ownership: SourceOwnership::Opaque,
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: None,
        children: vec![PreservedNode::Text(run.text.clone().into())].into(),
    }
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

fn common_text_change(before: &str, after: &str) -> (std::ops::Range<usize>, std::ops::Range<usize>) {
    let mut prefix = 0usize;
    for (left, right) in before.chars().zip(after.chars()) {
        if left != right { break; }
        prefix = prefix.saturating_add(left.len_utf8());
    }
    let before_tail = &before[prefix..];
    let after_tail = &after[prefix..];
    let mut suffix = 0usize;
    for (left, right) in before_tail.chars().rev().zip(after_tail.chars().rev()) {
        if left != right { break; }
        suffix = suffix.saturating_add(left.len_utf8());
    }
    (prefix..before.len().saturating_sub(suffix), prefix..after.len().saturating_sub(suffix))
}

fn shift_text_offset(offset: usize, delta: i64) -> usize {
    if delta >= 0 {
        offset.saturating_add(usize::try_from(delta).unwrap_or(usize::MAX))
    } else {
        offset.saturating_sub(usize::try_from(delta.unsigned_abs()).unwrap_or(usize::MAX))
    }
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
        let text_node = PreservedNode::Element(PreservedElement {
                name,
                attributes: Vec::new(),
                namespace_bindings,
                source_type: None,
                ownership: SourceOwnership::Opaque,
                dependency_confidence: DependencyConfidence::Unknown,
                anchor: None,
                children: SharedVec::default(),
            });
        let insertion_index = paragraph.children.iter().position(|node| !matches!(node,
            PreservedNode::Element(element) if is_text_element(element, paragraph)
        )).unwrap_or(paragraph.children.len());
        paragraph.children.insert(insertion_index, text_node);
        indexes.push(insertion_index);
    }
    let mut mixed_segments = Vec::with_capacity(indexes.len());
    let mut segment_offsets = Vec::with_capacity(indexes.len());
    let mut running_offset = 0usize;
    let source_segments = indexes.iter().map(|index| {
        let PreservedNode::Element(text_element) = &paragraph.children[*index] else {
            return Err(PreservationError::UnsupportedTextMapping);
        };
        let mut text = String::new();
        let mut mixed = false;
        for child in &text_element.children {
            match child {
                PreservedNode::Text(value) | PreservedNode::CData(value) => text.push_str(value),
                _ => mixed = true,
            }
        }
        segment_offsets.push(running_offset);
        running_offset = running_offset.saturating_add(text.len());
        mixed_segments.push(mixed);
        Ok(text)
    }).collect::<Result<Vec<_>, _>>()?;
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
        let clear_all_text = start == 0 && end == old_text.len() && inserted.is_empty();
        if has_opaque_gap && !clear_all_text
            && ((start < boundary && end > boundary) || (start == end && start == boundary))
        {
            return Err(PreservationError::UnsupportedTextMapping);
        }
    }
    let updated = replace_segments(&source_segments, start, end, inserted)?;
    let clear_all_text = start == 0 && end == old_text.len() && inserted.is_empty();
    let mut remove_indexes = BTreeSet::new();
    for (run_index, (index, text)) in indexes.into_iter().zip(updated).enumerate() {
        let PreservedNode::Element(text_element) = &mut paragraph.children[index] else {
            return Err(PreservationError::UnsupportedTextMapping);
        };
        if mixed_segments[run_index] && text != source_segments[run_index] {
            if clear_all_text {
                set_direct_text_content_preserving_opaque(text_element, "")?;
                continue;
            }
            let run_start = segment_offsets[run_index];
            let run_end = run_start.saturating_add(source_segments[run_index].len());
            if start < run_start || end > run_end {
                return Err(PreservationError::UnsupportedTextMapping);
            }
            replace_mixed_text_run(text_element, start - run_start, end - run_start, inserted)?;
            continue;
        }
        let removable = text.is_empty()
            && text_element.attributes.is_empty()
            && text_element.namespace_bindings.is_empty()
            && text_element.children.iter().all(|child| matches!(child,
                PreservedNode::Text(_) | PreservedNode::CData(_)
            ));
        if removable {
            remove_indexes.insert(index);
        } else if text != source_segments[run_index] || clear_all_text {
            set_direct_text_content_preserving_opaque(text_element, &text)?;
        }
    }
    if !remove_indexes.is_empty() {
        paragraph.children = paragraph.children.drain(..).enumerate()
            .filter_map(|(index, child)| (!remove_indexes.contains(&index)).then_some(child))
            .collect();
    }
    Ok(())
}

fn replace_mixed_text_run(
    element: &mut PreservedElement,
    start: usize,
    end: usize,
    inserted: &str,
) -> Result<(), PreservationError> {
    let mut segments = Vec::new();
    let mut opaque_boundaries = Vec::new();
    let mut offset = 0usize;
    let mut saw_text = false;
    let mut previous_was_text = false;
    for child in &element.children {
        match child {
            PreservedNode::Text(text) | PreservedNode::CData(text) => {
                segments.push(text.to_string());
                offset = offset.saturating_add(text.len());
                saw_text = true;
                previous_was_text = true;
            }
            _ => {
                if previous_was_text || !saw_text {
                    opaque_boundaries.push(offset);
                }
                previous_was_text = false;
            }
        }
    }
    let clear_all = start == 0 && end == offset && inserted.is_empty();
    if !clear_all && opaque_boundaries.iter().any(|boundary| {
        (start < *boundary && end > *boundary)
            || (start == end && start == *boundary)
    }) {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    let updated = replace_segments(&segments, start, end, inserted)?;
    let mut update_index = 0usize;
    for child in &mut element.children {
        match child {
            PreservedNode::Text(text) | PreservedNode::CData(text) => {
                *text = updated.get(update_index)
                    .ok_or(PreservationError::UnsupportedTextMapping)?
                    .clone().into();
                update_index += 1;
            }
            _ => {}
        }
    }
    if update_index != updated.len() {
        return Err(PreservationError::UnsupportedTextMapping);
    }
    Ok(())
}

fn set_direct_text_content_preserving_opaque(
    element: &mut PreservedElement,
    replacement: &str,
) -> Result<(), PreservationError> {
    let mut first = true;
    for child in &mut element.children {
        match child {
            PreservedNode::Text(text) | PreservedNode::CData(text) => {
                if first {
                    *text = if replacement.is_empty() { String::new().into() } else { replacement.to_owned().into() };
                    first = false;
                } else {
                    text.clear();
                }
            }
            _ => {}
        }
    }
    if first && !replacement.is_empty() {
        if element.children.iter().any(|child| !matches!(child,
            PreservedNode::Text(_) | PreservedNode::CData(_)
        )) {
            return Err(PreservationError::UnsupportedTextMapping);
        }
        element.children.push(PreservedNode::Text(replacement.to_owned().into()));
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

#[cfg(test)]
mod opaque_identity_reservation_tests {
    use super::*;

    #[test]
    fn opaque_raw_id_values_are_reserved_without_typed_registration() {
        let imported = crate::fdx::read_v6(include_bytes!("../tests/fixtures/broad_compatibility.fdx"))
            .expect("bounded compatibility fixture imports");
        let mut preservation = imported.project.preservation.expect("source tree is retained");
        assert!(!preservation
            .identities
            .iter()
            .any(|identity| identity.value == "opaque-repeat"));
        preservation.ensure_source_identity_use_index();
        let reserved = preservation.indexed_source_id_values();
        assert!(reserved.contains("opaque-repeat"));
        assert!(reserved.contains("shared-note"));
        assert!(reserved.contains("opaque-image-id"));
    }

    #[test]
    fn old_arc_reference_accounting_no_longer_authorizes_derived_removal() {
        let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="1"><Text>A</Text></Paragraph><Paragraph Type="Action" id="10"><Text>B</Text></Paragraph></Content></FinalDraft>"#;
        let imported = crate::fdx::read_v6(source).expect("numeric identity fixture imports");
        let mut preservation = imported.project.preservation.expect("source tree is retained");
        preservation.ensure_source_identity_use_index();
        assert!(preservation.indexed_source_id_values().contains("1"));
        assert!(preservation.indexed_source_id_values().contains("10"));

        let external_use_source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="1"><Text>A</Text></Paragraph><Paragraph Type="Action" id="10"><Text>B</Text></Paragraph><Paragraph Type="Action" id="other"><Text ref="1">C</Text></Paragraph></Content></FinalDraft>"#;
        let imported = crate::fdx::read_v6(external_use_source).expect("raw-use fixture imports");
        let mut preservation = imported.project.preservation.expect("source tree is retained");
        preservation.ensure_source_identity_use_index();
        assert!(preservation.indexed_source_id_values().contains("1"));
    }
}

#[cfg(test)]
mod dual_structural_projection_tests {
    use super::*;

    fn nested_legacy_wrapper(
        preservation: &mut PreservationState,
        group_id: &StableId,
        member_ids: &[StableId],
    ) {
        let content = source_content_mut(&mut preservation.sections, SourceSection::Content)
            .expect("dual fixture has a Content container");
        let wrapper_index = content.children.iter().position(|node| matches!(node,
            PreservedNode::Element(element)
                if matches!(&element.ownership, SourceOwnership::DualDialogueWrapper(owner) if owner == group_id)
        )).expect("fixture contains the semantic wrapper");
        let PreservedNode::Element(mut wrapper) = content.children.remove(wrapper_index)
            else { unreachable!("matched source node is an element") };
        wrapper.name.local_name = "LegacyDualContainer".to_owned();
        let namespace = wrapper.name.namespace_uri.clone();
        let mut legacy_parent = source_element("LegacyEnvelope", &namespace, None);
        legacy_parent.children.push(PreservedNode::Element(wrapper));
        content.children.insert(wrapper_index, PreservedNode::Element(legacy_parent));

        let wrapper = find_dual_wrapper_mut(&mut preservation.sections[0].children, group_id)
            .expect("wrapper ownership is found recursively");
        let member_set = member_ids.iter().cloned().collect::<BTreeSet<_>>();
        let mut members = BTreeMap::new();
        take_managed_owner_nodes(&mut wrapper.children, &member_set, &mut members);
        let dual = find_dual_dialogue_mut(&mut wrapper.children, &namespace)
            .expect("wrapper retains a DualDialogue container");
        let mut legacy_members = source_element("LegacyMemberEnvelope", &namespace, None);
        for member_id in member_ids {
            if let Some(member) = members.remove(member_id) {
                legacy_members.children.push(member);
            }
        }
        dual.children.push(PreservedNode::Element(legacy_members));
    }

    #[test]
    fn dual_unwrap_finds_nested_legacy_wrapper_and_restores_semantic_members_flat() {
        let imported = crate::fdx::read_v6(include_bytes!("../tests/fixtures/dual_dialogue_basic.fdx"))
            .expect("dual fixture imports");
        let mut project = imported.project;
        let group = project.dual_dialogue_groups[0].clone();
        let members = group.member_ids().cloned().collect::<Vec<_>>();
        let body_order = project.elements.iter().map(|element| element.id.clone()).collect::<Vec<_>>();
        let preservation = project.preservation.as_mut().expect("fixture retains source");
        nested_legacy_wrapper(preservation, &group.id, &members);

        preservation.unwrap_dual_dialogue_members(
            &group.id,
            &members,
            &body_order,
            None,
            group.wrapper.outer_foreign_id.as_deref(),
        ).expect("semantic unwrap ignores legacy wrapper ancestry and member nesting");

        let content = source_content(&preservation.sections, SourceSection::Content)
            .expect("Content remains available");
        let member_positions = members.iter().map(|id| {
            managed_child_index(&content.children, id).expect("restored member is a direct body child")
        }).collect::<Vec<_>>();
        assert!(member_positions.windows(2).all(|pair| pair[1] == pair[0] + 1));
        assert!(preservation.sections.iter().all(|section| {
            !contains_dual_wrapper(&section.children, &group.id)
        }));
    }

    #[test]
    fn nested_dual_insertion_uses_canonical_runs_without_member_template_or_old_ancestry() {
        let imported = crate::fdx::read_v6(include_bytes!("../tests/fixtures/dual_dialogue_basic.fdx"))
            .expect("dual fixture imports");
        let mut project = imported.project;
        let mut group = project.dual_dialogue_groups[0].clone();
        let new_id = StableId::new("dual-generated-member").expect("test identity is valid");
        let semantic = ScreenplayElement::new(new_id.as_str(), crate::Role::Dialogue, "Canonical insertion")
            .expect("new canonical member is valid");
        group.right.paragraphs.push(new_id.clone());
        let member_ids = group.member_ids().cloned().collect::<Vec<_>>();
        let mut canonical_members = project.elements.clone();
        canonical_members.push(semantic.clone());
        let mut ordered = project.elements.iter().map(|element| element.id.clone()).collect::<Vec<_>>();
        ordered.push(new_id.clone());
        let mut generated_member_provenance = Vec::new();
        let preservation = project.preservation.as_mut().expect("fixture retains source");
        nested_legacy_wrapper(preservation, &group.id, &member_ids[..member_ids.len() - 1]);

        preservation.insert_nested_managed_paragraph(
            &semantic,
            project.elements.len() - 1,
            &mut group,
            &ordered,
            &canonical_members,
            &mut generated_member_provenance,
        ).expect("nested insertion does not require old member ancestry or a template");

        let namespace = source_content(&preservation.sections, SourceSection::Content)
            .map(|content| content.name.namespace_uri.clone())
            .expect("Content remains available");
        let wrapper = preservation.sections.iter_mut().find_map(|section| {
            find_dual_wrapper_mut(&mut section.children, &group.id)
        }).expect("semantic wrapper remains");
        let dual = find_dual_dialogue_mut(&mut wrapper.children, &namespace)
            .expect("canonical DualDialogue container remains");
        let direct_members = dual.children.iter().filter_map(|node| match node {
            PreservedNode::Element(element) => match &element.ownership {
                SourceOwnership::Managed(id) => Some(id.clone()),
                _ => None,
            },
            _ => None,
        }).collect::<Vec<_>>();
        assert_eq!(direct_members, member_ids);
        let generated = find_managed_element_counted(&preservation.sections, &new_id)
            .expect("new source owner is indexed by semantic identity");
        assert_eq!(generated.source_type.as_deref(), Some("Dialogue"));
        assert!(generated.children.iter().any(|node| matches!(node,
                PreservedNode::Element(text) if text.name.local_name == "Text"
                && text.children.iter().any(|child| matches!(child,
                    PreservedNode::Text(value) if value.as_str() == "Canonical insertion"))
        )));
    }

    #[test]
    fn nested_insertion_recreates_missing_wrapper_and_member_from_current_semantics() {
        let imported = crate::fdx::read_v6(include_bytes!("../tests/fixtures/dual_dialogue_basic.fdx"))
            .expect("dual fixture imports");
        let mut project = imported.project;
        let mut groups = project.dual_dialogue_groups.clone();
        let group = &mut groups[0];
        let existing_member_ids = group.member_ids().cloned().collect::<Vec<_>>();
        let new_id = StableId::new("dual-missing-wrapper-insertion").expect("test identity is valid");
        let semantic = ScreenplayElement::new(new_id.as_str(), crate::Role::Dialogue, "Generated member text")
            .expect("new canonical member is valid");
        group.right.paragraphs.push(new_id.clone());
        let member_ids = group.member_ids().cloned().collect::<Vec<_>>();
        let missing_member_id = existing_member_ids[0].clone();

        let preservation = project.preservation.as_mut().expect("fixture retains source");
        let member_set = existing_member_ids.iter().cloned().collect::<BTreeSet<_>>();
        let mut extracted_members = BTreeMap::new();
        for section in &mut preservation.sections {
            take_managed_owner_nodes(&mut section.children, &member_set, &mut extracted_members);
        }
        let surviving_owner_snapshots = extracted_members.iter()
            .filter(|(id, _)| *id != &missing_member_id)
            .map(|(id, node)| (id.clone(), node.clone()))
            .collect::<BTreeMap<_, _>>();

        let mut removed_foreign_ids = BTreeSet::new();
        let mut removed_project_ids = BTreeSet::new();
        let mut removed_anchor_ids = BTreeSet::new();
        for section in &mut preservation.sections {
            let mut wrapper_found = false;
            remove_dual_wrapper_nodes(
                &mut section.children,
                &group.id,
                &mut wrapper_found,
                &mut removed_foreign_ids,
                &mut removed_project_ids,
                &mut removed_anchor_ids,
            );
        }
        preservation.remove_wrapper_registrations(
            &removed_foreign_ids,
            &removed_project_ids,
            &removed_anchor_ids,
            &group.id,
        );
        extracted_members.remove(&missing_member_id);
        preservation.remove_managed_paragraph(&missing_member_id, None)
            .expect("missing member source can be cleaned without its wrapper");
        let content = source_content_mut(&mut preservation.sections, SourceSection::Content)
            .expect("Content remains available");
        for member_id in &existing_member_ids {
            if let Some(owner) = extracted_members.remove(member_id) {
                content.children.push(owner);
            }
        }
        assert!(!preservation.sections.iter().any(|section| {
            contains_dual_wrapper(&section.children, &group.id)
        }));

        let before = project.elements.clone();
        let mut after = before.clone();
        after.push(semantic.clone());
        let canonical_members = after.clone();
        preservation.reconcile_body_structure(
            &before,
            &mut after,
            &BTreeSet::new(),
            &mut groups,
        ).expect("nested insertion generates the absent wrapper from semantic group state");

        let group = &groups[0];
        assert!(group.wrapper.outer_foreign_id.is_some());
        assert!(group.wrapper.preservation_anchor.is_some());
        assert_eq!(group.wrapper.source.as_ref().and_then(|source| source.foreign_role.as_deref()), Some("General"));
        let namespace = source_content(&preservation.sections, SourceSection::Content)
            .map(|content| content.name.namespace_uri.clone())
            .expect("Content remains available");
        let wrapper = preservation.sections.iter_mut().find_map(|section| {
            find_dual_wrapper_mut(&mut section.children, &group.id)
        }).expect("wrapper is generated for the current semantic group");
        assert_eq!(wrapper.name.namespace_uri, namespace);
        let dual = find_dual_dialogue_mut(&mut wrapper.children, &namespace)
            .expect("generated wrapper contains a DualDialogue container");
        let actual_member_ids = dual.children.iter().filter_map(|node| match node {
            PreservedNode::Element(element) => match &element.ownership {
                SourceOwnership::Managed(id) => Some(id.clone()),
                _ => None,
            },
            _ => None,
        }).collect::<Vec<_>>();
        assert_eq!(actual_member_ids, member_ids);
        for (id, expected_owner) in surviving_owner_snapshots {
            if let PreservedNode::Element(expected) = expected_owner {
                assert_eq!(find_managed_element_counted(&preservation.sections, &id), Some(&expected));
            } else {
                panic!("captured member owner is an element");
            }
        }
        for generated_id in [missing_member_id, new_id] {
            let generated = find_managed_element_counted(&preservation.sections, &generated_id)
                .expect("each missing current owner receives source data");
            let canonical = canonical_members.iter().find(|member| member.id == generated_id)
                .expect("generated owner has canonical semantic data");
            assert_eq!(generated.source_type.as_deref(), Some(fdx_role_name(canonical.role)));
            assert!(generated.children.iter().any(|node| matches!(node,
                PreservedNode::Element(text) if text.name.local_name == "Text"
                    && text.children.iter().any(|child| matches!(child,
                        PreservedNode::Text(value) if value.as_str() == canonical.text))
            )));
        }
    }

    fn contains_dual_wrapper(nodes: &[PreservedNode], group_id: &StableId) -> bool {
        nodes.iter().any(|node| match node {
            PreservedNode::Element(element) => {
                matches!(&element.ownership, SourceOwnership::DualDialogueWrapper(owner) if owner == group_id)
                    || contains_dual_wrapper(&element.children, group_id)
            }
            _ => false,
        })
    }
}

#[cfg(test)]
mod preservation_reconciliation_diagnostics_tests {
    use super::*;

    fn import_with_title_and_body() -> crate::Project {
        let source = br#"<FinalDraft DocumentType="Script" Version="6"><TitlePage><Content><Paragraph Type="Action" id="title-1"><Text>Title</Text></Paragraph></Content></TitlePage><Content><Paragraph Type="Action" id="body-1"><Text>Body</Text></Paragraph></Content></FinalDraft>"#;
        crate::fdx::read_v6(source)
            .expect("synthetic title/body fixture imports")
            .project
    }

    fn import_body_with_owner_count(owner_count: usize) -> crate::Project {
        let mut source = String::from(
            r#"<FinalDraft DocumentType="Script" Version="6"><Content>"#,
        );
        for index in 0..owner_count {
            source.push_str(&format!(
                "<Paragraph Type=\"Action\" id=\"body-{index}\"><Text>Owner {index}</Text></Paragraph>"
            ));
        }
        source.push_str("</Content></FinalDraft>");
        crate::fdx::read_v6(source.as_bytes())
            .expect("synthetic body fixture imports")
            .project
    }

    fn import_title_with_owner_count(owner_count: usize, empty_index: usize) -> crate::Project {
        let mut source = String::from(
            r#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="title-benchmark-body"><Text>Body</Text></Paragraph></Content><TitlePage><Content>"#,
        );
        for index in 0..owner_count {
            let text = if index == empty_index {
                String::new()
            } else {
                format!("synthetic title paragraph {index}")
            };
            source.push_str(&format!(
                "<Paragraph Type=\"Title\" id=\"synthetic-title-{index}\"><Text>{text}</Text></Paragraph>"
            ));
        }
        source.push_str("</Content></TitlePage></FinalDraft>");
        crate::fdx::read_v6(source.as_bytes())
            .expect("synthetic title fixture imports")
            .project
    }

    fn full_source_text_bytes(sections: &[PreservedSection]) -> usize {
        fn nodes_byte_count(nodes: &[PreservedNode]) -> usize {
            nodes
                .iter()
                .map(|node| match node {
                    PreservedNode::Element(element) => nodes_byte_count(&element.children),
                    PreservedNode::Text(text)
                    | PreservedNode::CData(text)
                    | PreservedNode::Comment(text) => text.len(),
                    PreservedNode::ProcessingInstruction { data, .. } => data.len(),
                })
                .sum()
        }
        sections
            .iter()
            .map(|section| nodes_byte_count(&section.children))
            .sum()
    }

    #[test]
    fn reconciliation_diagnostics_getter_and_reset_cover_every_field_independently() {
        reset_preservation_reconciliation_diagnostics();
        for counter in [
            &BODY_SURVIVORS_VISITED,
            &BODY_UNCHANGED_SURVIVORS_SKIPPED,
            &BODY_OWNERS_PROJECTED,
            &BODY_OWNERS_REMOVED,
            &BODY_OWNERS_INSERTED,
            &TITLE_SURVIVORS_VISITED,
            &TITLE_UNCHANGED_SURVIVORS_SKIPPED,
            &TITLE_OWNERS_PROJECTED,
            &TITLE_OWNERS_REMOVED,
            &TITLE_OWNERS_INSERTED,
            &SOURCE_TEXT_BYTE_DELTA_UPDATES,
        ] {
            increment_counter(counter);
        }
        assert_eq!(
            preservation_reconciliation_diagnostics(),
            PreservationReconciliationDiagnostics {
                body_survivors_visited: 1,
                body_unchanged_survivors_skipped: 1,
                body_owners_projected: 1,
                body_owners_removed: 1,
                body_owners_inserted: 1,
                title_survivors_visited: 1,
                title_unchanged_survivors_skipped: 1,
                title_owners_projected: 1,
                title_owners_removed: 1,
                title_owners_inserted: 1,
                source_text_byte_delta_updates: 1,
            }
        );

        reset_managed_paragraph_index_diagnostics();
        reset_copy_on_write_diagnostics();
        increment_counter(&MANAGED_INDEX_REBUILDS);
        increment_counter(&SHARED_VECTOR_CLONES);
        reset_preservation_reconciliation_diagnostics();
        assert_eq!(preservation_reconciliation_diagnostics(), Default::default());
        assert_eq!(managed_paragraph_index_diagnostics().rebuilds, 1);
        assert_eq!(copy_on_write_diagnostics().shared_vector_clones, 1);
        reset_managed_paragraph_index_diagnostics();
        reset_copy_on_write_diagnostics();
    }

    #[test]
    fn body_structural_reconciliation_counts_current_survivors_and_insertion() {
        let mut project = crate::fdx::read_v6(include_bytes!("../tests/fixtures/dual_dialogue_basic.fdx"))
            .expect("body reconciliation fixture imports")
            .project;
        let before = project.elements.clone();
        let mut after = before.clone();
        after.push(
            ScreenplayElement::new("counter-body-insert", crate::Role::Action, "Inserted")
                .expect("inserted semantic owner is valid"),
        );
        let mut groups = project.dual_dialogue_groups.clone();
        let preservation = project.preservation.as_mut().expect("fixture retains source");
        reset_preservation_reconciliation_diagnostics();

        preservation
            .reconcile_body_structure(&before, &mut after, &BTreeSet::new(), &mut groups)
            .expect("body structure reconciles");

        let diagnostics = preservation_reconciliation_diagnostics();
        assert_eq!(diagnostics.body_survivors_visited, before.len() as u64);
        assert_eq!(diagnostics.body_unchanged_survivors_skipped, before.len() as u64);
        assert_eq!(diagnostics.body_owners_projected, 1);
        assert_eq!(diagnostics.body_owners_removed, 0);
        assert_eq!(diagnostics.body_owners_inserted, 1);
        assert_eq!(diagnostics.title_survivors_visited, 0);
        assert_eq!(diagnostics.title_owners_projected, 0);
        assert_eq!(diagnostics.source_text_byte_delta_updates, 1);
    }

    #[test]
    fn large_body_deletion_batches_location_rebuild_and_skips_every_unchanged_survivor() {
        const OWNER_COUNT: usize = 96;
        let mut project = import_body_with_owner_count(OWNER_COUNT);
        let before = project.elements.clone();
        let deleted_id = before[OWNER_COUNT / 2].id.clone();
        let mut after = before.clone();
        after.retain(|element| element.id != deleted_id);
        let mut groups = project.dual_dialogue_groups.clone();
        let preservation = project.preservation.as_mut().expect("fixture retains source");
        reset_managed_paragraph_index_diagnostics();
        reset_source_identity_use_index_diagnostics();
        reset_preservation_reconciliation_diagnostics();

        preservation
            .reconcile_body_structure(&before, &mut after, &BTreeSet::new(), &mut groups)
            .expect("body deletion reconciles");

        let reconciliation = preservation_reconciliation_diagnostics();
        assert_eq!(reconciliation.body_owners_removed, 1);
        assert_eq!(reconciliation.body_survivors_visited, (OWNER_COUNT - 1) as u64);
        assert_eq!(
            reconciliation.body_unchanged_survivors_skipped,
            (OWNER_COUNT - 1) as u64,
        );
        assert_eq!(reconciliation.body_owners_projected, 0);
        assert_eq!(reconciliation.body_owners_inserted, 0);
        assert_eq!(reconciliation.source_text_byte_delta_updates, 0);
        assert_eq!(managed_paragraph_index_diagnostics().rebuilds, 1);
        assert_eq!(source_identity_use_index_diagnostics().rebuilds, 0);
        assert_eq!(
            preservation.source_text_bytes,
            full_source_text_bytes(&preservation.sections),
        );
    }

    #[test]
    fn body_reconciliation_projects_only_changed_survivors_without_rebuilding_locations() {
        const OWNER_COUNT: usize = 24;
        let mut project = import_body_with_owner_count(OWNER_COUNT);
        let before = project.elements.clone();
        let mut after = before.clone();
        let changed = OWNER_COUNT / 2;
        after[changed].text.push_str(" changed");
        after[changed + 1].role = crate::Role::Dialogue;
        let styled_text = after[changed + 2].text.clone();
        after[changed + 2].runs = vec![TextRun {
            text: styled_text,
            attributes: vec![crate::RunAttribute {
                name: "Style".to_owned(),
                value: "Bold".to_owned(),
            }],
        }];
        let mut groups = project.dual_dialogue_groups.clone();
        let preservation = project.preservation.as_mut().expect("fixture retains source");
        reset_managed_paragraph_index_diagnostics();
        reset_source_identity_use_index_diagnostics();
        reset_preservation_reconciliation_diagnostics();

        preservation
            .reconcile_body_structure(&before, &mut after, &BTreeSet::new(), &mut groups)
            .expect("mixed body projections reconcile");

        let reconciliation = preservation_reconciliation_diagnostics();
        assert_eq!(reconciliation.body_survivors_visited, OWNER_COUNT as u64);
        assert_eq!(
            reconciliation.body_unchanged_survivors_skipped,
            (OWNER_COUNT - 3) as u64,
        );
        assert_eq!(reconciliation.body_owners_projected, 3);
        assert_eq!(reconciliation.source_text_byte_delta_updates, 3);
        assert_eq!(managed_paragraph_index_diagnostics().rebuilds, 0);
        assert_eq!(source_identity_use_index_diagnostics().rebuilds, 0);
        assert_eq!(
            preservation.source_text_bytes,
            full_source_text_bytes(&preservation.sections),
        );
    }

    #[test]
    fn missing_body_source_is_recreated_without_deleted_payload_or_reused_foreign_id() {
        let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="missing-owner" Bookmark="deleted-bookmark" vendor-owner="deleted"><Text>Owner text</Text><UserParagraphData><OpaquePayload/></UserParagraphData></Paragraph><Paragraph Type="Action" id="survivor" vendor-ref="missing-owner"><Text>Survivor text</Text></Paragraph></Content></FinalDraft>"#;
        let mut project = crate::fdx::read_v6(source)
            .expect("missing-owner fixture imports")
            .project;
        let before = project.elements.clone();
        let mut after = before.clone();
        let missing_id = before[0].id.clone();
        let old_foreign_id = before[0]
            .source_provenance
            .as_ref()
            .and_then(|provenance| provenance.foreign_id.clone())
            .expect("imported body owner has a foreign identity");
        let mut groups = project.dual_dialogue_groups.clone();
        let preservation = project.preservation.as_mut().expect("fixture retains source");
        let mut removed_foreign_ids = BTreeSet::new();
        let mut removed_project_ids = BTreeSet::new();
        let mut removed_anchor_ids = BTreeSet::new();
        let mut removed_any = false;
        for section in &mut preservation.sections {
            removed_any |= remove_managed_owner_nodes(
                &mut section.children,
                &missing_id,
                &mut removed_foreign_ids,
                &mut removed_project_ids,
                &mut removed_anchor_ids,
            );
        }
        assert!(removed_any);
        preservation.cleanup_removed_owner_bindings(
            &removed_foreign_ids,
            &removed_project_ids,
            &removed_anchor_ids,
        );
        preservation.rebuild_managed_paragraph_index();
        reset_preservation_reconciliation_diagnostics();

        preservation
            .reconcile_body_structure(&before, &mut after, &BTreeSet::new(), &mut groups)
            .expect("missing current source occurrence is recreated");

        let recovered_foreign_id = after[0]
            .source_provenance
            .as_ref()
            .and_then(|provenance| provenance.foreign_id.as_deref())
            .expect("recreated owner has a new foreign identity");
        assert_ne!(recovered_foreign_id, old_foreign_id);
        let recovered = preservation
            .managed_element(&missing_id)
            .expect("recreated owner lookup succeeds")
            .expect("recreated owner is present");
        assert!(!recovered.attributes.iter().any(|attribute| {
            attribute.name.local_name == "Bookmark"
                || attribute.name.local_name == "vendor-owner"
        }));
        assert!(!recovered.children.iter().any(|node| matches!(node,
            PreservedNode::Element(element) if element.name.local_name == "UserParagraphData"
        )));
        let survivor = preservation
            .managed_element(&before[1].id)
            .expect("unchanged survivor lookup succeeds")
            .expect("unchanged survivor remains present");
        assert!(survivor.attributes.iter().any(|attribute| {
            attribute.name.local_name == "vendor-ref" && attribute.value == old_foreign_id
        }));
        let diagnostics = preservation_reconciliation_diagnostics();
        assert_eq!(diagnostics.body_unchanged_survivors_skipped, 1);
        assert_eq!(diagnostics.body_owners_projected, 1);
        assert_eq!(diagnostics.body_owners_inserted, 1);
    }

    #[test]
    fn title_structural_reconciliation_counts_current_survivors_and_insertion() {
        let mut project = import_with_title_and_body();
        let before = project.title_page.clone();
        let mut after = before.clone();
        after.push(
            TitlePageParagraph::new(
                "counter-title-insert",
                "Inserted title",
                crate::TitlePageParagraphFormatting::default(),
            )
            .expect("inserted title owner is valid"),
        );
        let preservation = project.preservation.as_mut().expect("fixture retains source");
        preservation.ensure_source_identity_use_index();
        reset_managed_paragraph_index_diagnostics();
        reset_source_identity_use_index_diagnostics();
        reset_preservation_reconciliation_diagnostics();

        preservation
            .reconcile_title_structure(&before, &mut after)
            .expect("title structure reconciles");

        let diagnostics = preservation_reconciliation_diagnostics();
        assert_eq!(diagnostics.title_survivors_visited, before.len() as u64);
        assert_eq!(diagnostics.title_unchanged_survivors_skipped, before.len() as u64);
        assert_eq!(diagnostics.title_owners_projected, 1);
        assert_eq!(diagnostics.title_owners_removed, 0);
        assert_eq!(diagnostics.title_owners_inserted, 1);
        assert_eq!(diagnostics.body_survivors_visited, 0);
        assert_eq!(diagnostics.body_owners_projected, 0);
        assert_eq!(diagnostics.source_text_byte_delta_updates, 1);
        assert_eq!(managed_paragraph_index_diagnostics().rebuilds, 1);
        assert_eq!(source_identity_use_index_diagnostics().rebuilds, 0);
    }

    #[test]
    fn large_title_deletion_skips_unchanged_owners_and_projects_only_changed_title_survivors() {
        const OWNER_COUNT: usize = 96;
        let mut project = import_title_with_owner_count(OWNER_COUNT, OWNER_COUNT / 2);
        let before = project.title_page.clone();
        let deleted_id = before[OWNER_COUNT / 2].id.clone();
        let mut after = before.clone();
        after.retain(|paragraph| paragraph.id != deleted_id);
        let first_changed = 8;
        let role_changed = 24;
        let runs_changed = 72;
        after[first_changed].text.push_str(" changed");
        after[role_changed].role = None;
        let run_text = after[runs_changed].text.clone();
        after[runs_changed].runs = vec![TextRun {
            text: run_text,
            attributes: vec![crate::RunAttribute {
                name: "Style".to_owned(),
                value: "Bold".to_owned(),
            }],
        }];
        let preservation = project.preservation.as_mut().expect("source is retained");
        reset_managed_paragraph_index_diagnostics();
        reset_source_identity_use_index_diagnostics();
        reset_preservation_reconciliation_diagnostics();

        preservation
            .reconcile_title_structure(&before, &mut after)
            .expect("title deletion and mixed owner projections reconcile");

        let reconciliation = preservation_reconciliation_diagnostics();
        let survivors = OWNER_COUNT - 1;
        assert_eq!(reconciliation.title_owners_removed, 1);
        assert_eq!(reconciliation.title_survivors_visited, survivors as u64);
        assert_eq!(
            reconciliation.title_unchanged_survivors_skipped,
            (survivors - 3) as u64,
        );
        assert_eq!(reconciliation.title_owners_projected, 3);
        assert_eq!(reconciliation.title_owners_inserted, 0);
        assert_eq!(reconciliation.source_text_byte_delta_updates, 3);
        assert_eq!(managed_paragraph_index_diagnostics().rebuilds, 1);
        assert_eq!(source_identity_use_index_diagnostics().rebuilds, 0);
        assert_eq!(
            preservation.source_text_bytes,
            full_source_text_bytes(&preservation.sections),
        );
        assert_eq!(after[role_changed - usize::from(role_changed > OWNER_COUNT / 2)].role, None);
    }

    #[test]
    fn title_text_only_reconciliation_is_location_index_neutral() {
        const OWNER_COUNT: usize = 24;
        let mut project = import_title_with_owner_count(OWNER_COUNT, OWNER_COUNT);
        let before = project.title_page.clone();
        let mut after = before.clone();
        after[OWNER_COUNT / 2].text.push_str(" edited");
        let preservation = project.preservation.as_mut().expect("source is retained");
        reset_managed_paragraph_index_diagnostics();
        reset_source_identity_use_index_diagnostics();
        reset_preservation_reconciliation_diagnostics();

        preservation
            .reconcile_title_structure(&before, &mut after)
            .expect("title text-only projection reconciles");

        let reconciliation = preservation_reconciliation_diagnostics();
        assert_eq!(reconciliation.title_survivors_visited, OWNER_COUNT as u64);
        assert_eq!(reconciliation.title_unchanged_survivors_skipped, (OWNER_COUNT - 1) as u64);
        assert_eq!(reconciliation.title_owners_projected, 1);
        assert_eq!(reconciliation.title_owners_removed, 0);
        assert_eq!(reconciliation.title_owners_inserted, 0);
        assert_eq!(managed_paragraph_index_diagnostics().rebuilds, 0);
        assert_eq!(source_identity_use_index_diagnostics().rebuilds, 0);
        assert_eq!(
            preservation.source_text_bytes,
            full_source_text_bytes(&preservation.sections),
        );
    }

    #[test]
    fn failed_body_projection_counts_the_attempt_before_the_fallible_call() {
        let mut project = crate::fdx::read_v6(include_bytes!("../tests/fixtures/dual_dialogue_basic.fdx"))
            .expect("body failure fixture imports")
            .project;
        let before = project.elements.clone();
        let mut after = before.clone();
        after[0].runs = vec![TextRun {
            text: "inconsistent run".to_owned(),
            attributes: Vec::new(),
        }];
        let mut groups = project.dual_dialogue_groups.clone();
        let preservation = project.preservation.as_mut().expect("fixture retains source");
        reset_preservation_reconciliation_diagnostics();

        assert_eq!(
            preservation.reconcile_body_structure(&before, &mut after, &BTreeSet::new(), &mut groups),
            Err(PreservationError::UnsupportedTextMapping),
        );

        let diagnostics = preservation_reconciliation_diagnostics();
        assert_eq!(diagnostics.body_survivors_visited, 1);
        assert_eq!(diagnostics.body_owners_projected, 1);
        assert_eq!(diagnostics.body_unchanged_survivors_skipped, 0);
        assert_eq!(diagnostics.source_text_byte_delta_updates, 0);
    }
}

#[cfg(test)]
mod lazy_index_lifecycle_tests {
    use super::*;

    fn imported_project() -> crate::Project {
        crate::fdx::read_v6(
            br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="lazy-101"><Text>Alpha</Text></Paragraph><Paragraph Type="Action" id="lazy-102"><Text ref="lazy-101">Beta</Text></Paragraph></Content></FinalDraft>"#,
        )
        .expect("small lazy-index fixture imports")
        .project
    }

    fn full_text_byte_count(sections: &[PreservedSection]) -> usize {
        fn nodes_byte_count(nodes: &[PreservedNode]) -> usize {
            nodes.iter().map(|node| match node {
                PreservedNode::Element(element) => nodes_byte_count(&element.children),
                PreservedNode::Text(text)
                | PreservedNode::CData(text)
                | PreservedNode::Comment(text) => text.len(),
                PreservedNode::ProcessingInstruction { data, .. } => data.len(),
            }).sum()
        }
        sections.iter().map(|section| nodes_byte_count(&section.children)).sum()
    }

    #[test]
    fn managed_location_ensure_does_not_build_source_identity_uses() {
        let mut project = imported_project();
        let id = project.elements[0].id.clone();
        let preservation = project.preservation.as_mut().expect("source is retained");
        reset_source_identity_use_index_diagnostics();
        reset_managed_paragraph_index_diagnostics();

        preservation.ensure_managed_paragraph_index();

        assert!(preservation.has_managed_paragraph_index());
        assert!(!preservation.has_source_identity_use_index());
        assert!(preservation.managed_element(&id).expect("managed lookup succeeds").is_some());
        assert_eq!(managed_paragraph_index_diagnostics().rebuilds, 1);
        assert_eq!(source_identity_use_index_diagnostics(), Default::default());
    }

    #[test]
    fn identity_use_rebuild_after_invalidation_matches_complete_results_and_is_reused() {
        let mut expected = imported_project().preservation.expect("source is retained");
        expected.ensure_source_identity_use_index();
        let mut actual = imported_project().preservation.expect("source is retained");
        actual.ensure_managed_paragraph_index();
        actual.invalidate_source_identity_use_index();
        reset_source_identity_use_index_diagnostics();

        actual.ensure_source_identity_use_index();
        let expected_index = expected.source_identity_use_index.as_ref().expect("complete index exists");
        let actual_index = actual.source_identity_use_index.as_ref().expect("lazy index was built");
        assert_eq!(actual_index.paragraph_identity_counts, expected_index.paragraph_identity_counts);
        assert_eq!(actual_index.paragraph_source_identity_counts, expected_index.paragraph_source_identity_counts);
        assert_eq!(actual_index.referenced_paragraph_ids, expected_index.referenced_paragraph_ids);
        assert_eq!(actual_index.anchored_paragraph_ids, expected_index.anchored_paragraph_ids);
        assert_eq!(actual_index.externally_used_paragraph_ids, expected_index.externally_used_paragraph_ids);
        assert_eq!(actual.indexed_source_id_values(), expected.indexed_source_id_values());
        assert!(actual.has_managed_paragraph_index());
        assert_eq!(source_identity_use_index_diagnostics().rebuilds, 1);

        let rebuilt = source_identity_use_index_diagnostics();
        actual.ensure_source_identity_use_index();
        assert_eq!(source_identity_use_index_diagnostics(), rebuilt);
    }

    #[test]
    fn full_rebuild_sets_source_text_bytes_to_a_complete_tree_walk() {
        let mut preservation = imported_project().preservation.expect("source is retained");
        let expected = full_text_byte_count(&preservation.sections);

        preservation.rebuild_indexes();

        assert_eq!(preservation.source_text_bytes, expected);
    }

    #[test]
    fn text_projection_keeps_locations_and_applies_owner_local_byte_deltas() {
        let mut project = imported_project();
        let id = project.elements[1].id.clone();
        let provenance = project.elements[1]
            .source_provenance
            .clone()
            .expect("managed source owner has provenance");
        let preservation = project.preservation.as_mut().expect("source is retained");
        preservation.ensure_managed_paragraph_index();
        preservation.ensure_source_identity_use_index();
        let locations = preservation.managed_paragraph_index.clone();

        let mut old_text = "Beta".to_owned();
        for text in ["Beta!", "Bet", "", "retyped", "é界", "lazy-102"] {
            let runs = if text.is_empty() {
                Vec::new()
            } else {
                vec![TextRun {
                    text: text.to_owned(),
                    attributes: Vec::new(),
                }]
            };
            preservation
                .project_managed_text_runs(
                    &id,
                    &old_text,
                    text,
                    &runs,
                    Some(&provenance),
                    Some("Action"),
                )
                .expect("owner text projection succeeds");
            old_text = text.to_owned();
            assert_eq!(
                preservation.source_text_bytes,
                full_text_byte_count(&preservation.sections),
                "owner-local accounting matches a full retained-tree traversal after {text:?}",
            );
        }

        assert_eq!(preservation.managed_paragraph_index, locations);
        assert!(!preservation.has_source_identity_use_index());

        preservation.ensure_source_identity_use_index();
        assert!(preservation
            .source_identity_use_index
            .as_ref()
            .expect("identity-use index is rebuilt on demand")
            .externally_used_paragraph_ids
            .contains("lazy-102"));

        preservation
            .project_managed_text_runs(
                &id,
                &old_text,
                "retyped",
                &[TextRun {
                    text: "retyped".to_owned(),
                    attributes: Vec::new(),
                }],
                Some(&provenance),
                Some("Action"),
            )
            .expect("identity-like text can be removed by projection");
        assert!(!preservation.has_source_identity_use_index());
        assert_eq!(
            preservation.source_text_bytes,
            full_text_byte_count(&preservation.sections),
        );
        preservation.ensure_source_identity_use_index();
        assert!(!preservation
            .source_identity_use_index
            .as_ref()
            .expect("identity-use index is rebuilt after invalidation")
            .externally_used_paragraph_ids
            .contains("lazy-102"));

        preservation
            .project_managed_text_runs(
                &id,
                "retyped",
                "retyped",
                &[TextRun {
                    text: "retyped".to_owned(),
                    attributes: vec![crate::RunAttribute {
                        name: "Style".to_owned(),
                        value: "Bold".to_owned(),
                    }],
                }],
                Some(&provenance),
                Some("Action"),
            )
            .expect("run-style-only projection succeeds");
        assert!(!preservation.has_source_identity_use_index());
        assert_eq!(
            preservation.source_text_bytes,
            full_text_byte_count(&preservation.sections),
        );
        assert_eq!(preservation.managed_paragraph_index, locations);
    }

    #[test]
    fn role_and_attribute_projection_do_not_rebuild_managed_locations() {
        let mut project = imported_project();
        let id = project.elements[1].id.clone();
        let provenance = project.elements[1]
            .source_provenance
            .clone()
            .expect("managed source owner has provenance");
        let preservation = project.preservation.as_mut().expect("source is retained");
        preservation.ensure_managed_paragraph_index();
        preservation.ensure_source_identity_use_index();
        let locations = preservation.managed_paragraph_index.clone();
        let byte_count = full_text_byte_count(&preservation.sections);

        preservation
            .change_managed_role(&id, "Dialogue", Some(&provenance), 1)
            .expect("role projection succeeds");
        preservation
            .set_managed_paragraph_attribute(&id, "Bookmark", Some("role-stable"))
            .expect("owner attribute projection succeeds");
        preservation
            .set_managed_role_projection(&id, None)
            .expect("title-role projection updates attributes in place");

        assert_eq!(preservation.managed_paragraph_index, locations);
        assert!(!preservation.has_source_identity_use_index());
        assert_eq!(preservation.source_text_bytes, byte_count);
        assert_eq!(
            preservation.source_text_bytes,
            full_text_byte_count(&preservation.sections),
        );
    }

    #[test]
    fn managed_location_rebuild_preserves_missing_and_ambiguous_owner_errors() {
        let mut missing = imported_project();
        let missing_id = missing.elements[0].id.clone();
        let preservation = missing.preservation.as_mut().expect("source is retained");
        source_content_mut(&mut preservation.sections, SourceSection::Content)
            .expect("Content is retained")
            .children
            .retain(|node| !matches!(node,
                PreservedNode::Element(element)
                    if matches!(&element.ownership, SourceOwnership::Managed(id) if id == &missing_id)
            ));
        preservation.rebuild_managed_paragraph_index();
        assert_eq!(
            preservation.managed_element_mut_indexed(&missing_id).unwrap_err(),
            PreservationError::ManagedAnchorMissing,
        );

        let mut ambiguous = imported_project();
        let ambiguous_id = ambiguous.elements[0].id.clone();
        let preservation = ambiguous.preservation.as_mut().expect("source is retained");
        let content = source_content_mut(&mut preservation.sections, SourceSection::Content)
            .expect("Content is retained");
        let duplicate = content.children.iter().find_map(|node| match node {
            PreservedNode::Element(element)
                if matches!(&element.ownership, SourceOwnership::Managed(id) if id == &ambiguous_id) =>
            {
                Some(PreservedNode::Element(element.clone()))
            }
            _ => None,
        }).expect("the first managed owner is present");
        content.children.push(duplicate);
        preservation.rebuild_managed_paragraph_index();
        assert_eq!(
            preservation.managed_element(&ambiguous_id).unwrap_err(),
            PreservationError::ManagedAnchorAmbiguous,
        );
    }

    #[test]
    fn current_tree_validation_does_not_rebuild_a_dirty_source_use_index() {
        let mut project = imported_project();
        let preservation = project.preservation.as_mut().expect("source is retained");
        preservation.source_identity_use_index = None;
        preservation.managed_paragraph_index = None;
        reset_source_identity_use_index_diagnostics();

        assert!(project.validate_current_state().is_ok());

        assert!(!project.preservation.as_ref().unwrap().has_source_identity_use_index());
        assert_eq!(source_identity_use_index_diagnostics(), Default::default());
    }
}

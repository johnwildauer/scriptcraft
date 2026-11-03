//! Bounded FDX Version 6 codec for the evidenced element subset.
//!
//! The reader returns a semantic editing projection plus the complete parsed
//! source tree and original bytes. The writer reconciles supported element edits
//! with that tree, or emits a minimal new document for the evidenced role labels.
//! ScriptCraft-only persistent identity, metadata and geometry are not part of the
//! semantic model. Runtime inputs are not serialized; imported FDX source remains
//! authoritative for source settings. These APIs do not claim Final Draft
//! interoperability.

use crate::preservation::{
    BoardEntity, BoardEntityKind, BoardGraph, DependencyConfidence, ForeignIdDomain,
    ForeignIdentity, ForeignReference, NamespaceBinding, PreservationError, PreservationState,
    PreservedAttribute, PreservedElement, PreservedNode, PreservedSection, ReferencePolicy,
    SectionKind, SourceAnchor, SourceAnchorId, SourceEncoding, SourceName, SourceOwnership,
    SourceSnapshot,
};
use crate::paragraph_id::{ParagraphIdAllocationError, ParagraphIdAllocator};
use crate::formatting::{formatting_for_role, formatting_in_native_order, role_index};
use crate::text_state::{self};
pub use crate::text_state::TextState;
use crate::{
    ModelError, PrintVisibility, Project, Role, RunAttribute, ScreenplayElement, SourceFormat,
    SourceLayoutSettings, SourceProvenance, SourceSection, TextRun,
};
use quick_xml::escape::unescape;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::{Namespace, ResolveResult};
use quick_xml::reader::NsReader;
use quick_xml::XmlVersion;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use time::OffsetDateTime;

pub use crate::preservation::{
    MAX_ATTRIBUTES_PER_ELEMENT, MAX_IMAGE_BYTES, MAX_SOURCE_BYTES, MAX_SOURCE_DEPTH,
    MAX_SOURCE_EVENTS, MAX_SOURCE_NAME_OR_VALUE_BYTES, MAX_SOURCE_TEXT_BYTES,
    MAX_TOTAL_IMAGE_BYTES,
};

const UTF8_BOM: &[u8] = b"\xef\xbb\xbf";
const FDX_ROOT: &str = "FinalDraft";
const FDX_VERSION: &str = "6";
const XML_DECLARATION_WITHOUT_TRAILING_LF: &str =
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\" ?>";
const XML_DECLARATION: &str =
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\" ?>\n";

/// Bounded, privacy-safe reader failures. Variants intentionally carry no source
/// text, identifiers, file paths, or image payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Save-path UnsupportedConstruct failure with a stable call-site code.
    SaveDiagnosticUnsupportedConstruct(u16),
    /// Code 30 with bounded, value-free structural detail for a normalized tree mismatch.
    SaveDiagnosticSourceTreeMismatch(String),
    ParagraphIdentityEntropyUnavailable,
    ParagraphIdentityCollisionLimit,
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
            Self::SaveDiagnosticUnsupportedConstruct(code) => {
                return write!(
                    formatter,
                    "{code}: FDX contains a construct that cannot be projected safely"
                );
            }
            Self::SaveDiagnosticSourceTreeMismatch(detail) => {
                return write!(
                    formatter,
                    "30: FDX contains a construct that cannot be projected safely; {detail}"
                );
            }
            Self::ParagraphIdentityEntropyUnavailable => {
                "secure randomness for a new FDX paragraph identity is unavailable"
            }
            Self::ParagraphIdentityCollisionLimit => {
                "a unique FDX paragraph identity could not be allocated within the retry limit"
            }
            Self::Model(_) => "FDX semantic projection failed model validation",
            Self::Preservation(_) => "FDX preservation state failed validation",
        })
    }
}

impl std::error::Error for FdxError {}

/// Stable Save/Save As diagnostics. Values are persistent diagnostic identifiers,
/// not source-line numbers. Keep this map centralized and append new values rather
/// than reassigning existing ones. The corresponding human-readable map is also
/// recorded in `.forge/blueprints/technical-context.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub(crate) enum SaveDiagnosticCode {
    WriteSourceFormat = 1,
    WriteOriginalRereadPropagation = 2,
    WriteOriginalPreservationMissing = 3,
    WriteOriginalIdentityGraphMismatch = 4,
    ReconcileProjectManagedOrderMismatch = 5,
    ReconcileNodeDuplicateManagedIdentity = 6,
    ReconcileNodeManagedOutsideSection = 7,
    ReconcileNodeSemanticElementMissing = 8,
    ReconcileProjectNodePropagation = 9,
    ReconcileParagraphRoleMismatch = 10,
    ReconcileParagraphTypeAttributeMismatch = 11,
    ReconcileParagraphEmptyRunsMismatch = 12,
    ReconcileParagraphCdataInEmptyText = 13,
    ReconcileParagraphUnsupportedEmptyChild = 14,
    ReconcileParagraphUnexpectedEmptyText = 15,
    ReconcileParagraphRunCountMismatch = 16,
    ReconcileParagraphRunProjectionMismatch = 17,
    ReconcileParagraphUnexpectedFirstChild = 18,
    ReconcileParagraphUnexpectedLaterChild = 19,
    NewDocumentHeadingNodeMismatch = 20,
    NewDocumentScenePropertiesUnsupported = 21,
    NewParagraphEmptyRunsMismatch = 22,
    NewParagraphNamespacedRunAttribute = 23,
    WriteElementNamespacePrefixMissing = 24,
    WriteOutputRereadPropagation = 25,
    WriteSemanticRereadMismatch = 26,
    WriteOutputPreservationMissing = 27,
    WriteCandidatePreservationMissing = 28,
    WriteIdentitySetMismatch = 29,
    WriteNormalizedSourceTreeMismatch = 30,
    StorageRereadPropagation = 31,
    StorageSemanticProjectionMismatch = 32,
    ReconcileProjectChildPropagation = 33,
    ReconcileManagedParagraphPropagation = 34,
    LineageAmbiguousDocumentRef = 35,
    LineageAmbiguousDocumentRefDateTime = 36,
    LineageIdentityEntropyUnavailable = 37,
    LineageIdentityCollisionLimit = 38,
}

impl SaveDiagnosticCode {
    const fn number(self) -> u16 {
        self as u16
    }

    pub(crate) const fn error(self) -> FdxError {
        FdxError::SaveDiagnosticUnsupportedConstruct(self.number())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriterStage {
    ProjectValidation,
    PreservationValidation,
    OriginalSourceGraphComparison,
    ManagedSourceReconciliation,
    NewDocumentGeneration,
    XmlWriting,
    OutputReread,
    SemanticComparison,
    IdentityComparison,
    SourceTreeComparison,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WriterFailure {
    error: FdxError,
    stage: WriterStage,
    structural_difference: Option<SourceTreeDifference>,
    identity_difference: Option<IdentitySetDifference>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdentityDomainSummary {
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
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IdentitySetDifference {
    domain: IdentityDomainSummary,
    missing: bool,
    unexpected: bool,
}

/// A Version 6 reader result. `project` is an editing projection only; runtime
/// layout uses deterministic application defaults where the FDX samples do not
/// establish reversible geometry mappings. The source-owned preservation state
/// retains the actual FDX settings and values independently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FdxDocument {
    pub project: Project,
    /// Valid root DocumentRef DateTime observed during deserialization.
    /// Malformed or ambiguous values remain source-preserved but are not typed.
    pub opened_document_ref_datetime: Option<String>,
    /// Valid direct-root disposable native geometry. Invalid or ambiguous source
    /// state remains preserved but is not projected.
    pub window_state: Option<WindowState>,
    /// Direct-root disposable zoom and body-editor selection, with safe defaults.
    pub text_state: TextState,
}

/// Supported native-window geometry in OS logical coordinates. FDX V6 currently
/// emits only Normal mode; unknown imported modes safely project as Normal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowState {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

impl WindowState {
    pub const DEFAULT: Self = Self {
        left: 0,
        top: 95,
        width: 870,
        height: 854,
    };

    pub fn new(left: i32, top: i32, width: i32, height: i32) -> Option<Self> {
        (width > 0 && height > 0).then_some(Self { left, top, width, height })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SaveOperation {
    DirectSerialization,
    Save,
    SaveAs,
}

/// Save-scoped writer authority. Direct serialization has no lineage side
/// effects; transactional saves capture one timestamp before candidate work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SaveContext {
    pub(crate) operation: SaveOperation,
    pub(crate) timestamp: String,
    pub(crate) opened_timestamp: Option<String>,
    pub(crate) window_state: WindowState,
    pub(crate) text_state: TextState,
    injected_lineage_ids: Vec<String>,
    next_injected_lineage_id: usize,
    #[cfg(test)]
    fail_lineage_entropy: bool,
}

impl SaveContext {
    pub(crate) fn new(operation: SaveOperation) -> Self {
        Self {
            operation,
            timestamp: current_save_timestamp(),
            opened_timestamp: None,
            window_state: WindowState::DEFAULT,
            text_state: TextState::DEFAULT,
            injected_lineage_ids: Vec::new(),
            next_injected_lineage_id: 0,
            #[cfg(test)]
            fail_lineage_entropy: false,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_timestamp(operation: SaveOperation, timestamp: String) -> Self {
        Self {
            operation,
            timestamp,
            opened_timestamp: None,
            window_state: WindowState::DEFAULT,
            text_state: TextState::DEFAULT,
            injected_lineage_ids: Vec::new(),
            next_injected_lineage_id: 0,
            #[cfg(test)]
            fail_lineage_entropy: false,
        }
    }

    pub(crate) fn with_opened_timestamp(mut self, timestamp: Option<String>) -> Self {
        self.opened_timestamp = timestamp;
        self
    }

    pub(crate) fn with_window_state(mut self, window_state: WindowState) -> Self {
        self.window_state = window_state;
        self
    }

    pub(crate) fn with_text_state(mut self, text_state: TextState) -> Self {
        self.text_state = text_state;
        self
    }

    #[cfg(test)]
    pub(crate) fn with_test_lineage_ids(mut self, ids: Vec<String>) -> Self {
        self.injected_lineage_ids = ids;
        self
    }

    #[cfg(test)]
    pub(crate) fn with_test_lineage_entropy_failure(mut self) -> Self {
        self.fail_lineage_entropy = true;
        self
    }

    fn allocate_lineage_id(
        &mut self,
        reserved: &mut BTreeSet<String>,
    ) -> Result<String, FdxError> {
        const MAX_ATTEMPTS: usize = 16;
        for _ in 0..MAX_ATTEMPTS {
            let candidate = if let Some(candidate) = self
                .injected_lineage_ids
                .get(self.next_injected_lineage_id)
            {
                self.next_injected_lineage_id += 1;
                candidate.clone()
            } else {
                #[cfg(test)]
                if self.fail_lineage_entropy {
                    return Err(SaveDiagnosticCode::LineageIdentityEntropyUnavailable.error());
                }
                let mut bytes = [0; 16];
                getrandom::fill(&mut bytes)
                    .map_err(|_| SaveDiagnosticCode::LineageIdentityEntropyUnavailable.error())?;
                bytes[6] = (bytes[6] & 0x0f) | 0x40;
                bytes[8] = (bytes[8] & 0x3f) | 0x80;
                uuid::Uuid::from_bytes(bytes).hyphenated().to_string()
            };
            if is_lowercase_uuid_v4(&candidate) && reserved.insert(candidate.clone()) {
                return Ok(candidate);
            }
        }
        Err(SaveDiagnosticCode::LineageIdentityCollisionLimit.error())
    }
}

fn is_lowercase_uuid_v4(value: &str) -> bool {
    if value.len() != 36 || value.bytes().any(|byte| byte.is_ascii_uppercase()) {
        return false;
    }
    uuid::Uuid::parse_str(value).is_ok_and(|uuid| {
        uuid.hyphenated().to_string() == value
            && uuid.get_version() == Some(uuid::Version::Random)
            && uuid.get_variant() == uuid::Variant::RFC4122
    })
}

pub(crate) fn valid_document_ref_datetime(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 15
        || bytes[8] != b'T'
        || !bytes[..8].iter().all(u8::is_ascii_digit)
        || !bytes[9..].iter().all(u8::is_ascii_digit)
    {
        return false;
    }
    let number = |range: std::ops::Range<usize>| value.get(range)?.parse::<u32>().ok();
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        number(0..4),
        number(4..6),
        number(6..8),
        number(9..11),
        number(11..13),
        number(13..15),
    ) else {
        return false;
    };
    if year == 0
        || !(1..=12).contains(&month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return false;
    }
    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        2 if leap_year => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=days_in_month).contains(&day)
}

fn current_save_timestamp() -> String {
    let local = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}",
        local.year(),
        u8::from(local.month()),
        local.day(),
        local.hour(),
        local.minute(),
        local.second()
    )
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
    match version_status(root)? {
        FdxVersionStatus::Version6 => {}
        FdxVersionStatus::ExplicitNon6 { .. } => return Err(FdxError::UnsupportedVersion),
        FdxVersionStatus::Missing => return Err(FdxError::MissingVersion),
        FdxVersionStatus::Malformed => return Err(FdxError::InvalidVersion),
        FdxVersionStatus::Duplicate => return Err(FdxError::DuplicateVersion),
    }
    let opened_document_ref_datetime = raw_opened_document_ref_datetime(root);
    let window_state = raw_root_window_state(root);

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
        title_page: Vec::new(),
        elements: Vec::new(),
        preservation: None,
        legacy_json_state: None,
    };
    let mut source_layout_settings = read_source_layout_settings(root, root_namespace)?;
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
    let preservation = PreservationState {
        source: SourceSnapshot {
            format: SourceFormat::Fdx,
            original_bytes: input.to_vec(),
            encoding: SourceEncoding::Utf8,
        },
        source_layout_settings,
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
    let text_state = raw_root_text_state(&root_for_text_state, &project);
    Ok(FdxDocument {
        project,
        opened_document_ref_datetime,
        window_state,
        text_state,
    })
}

/// Serialize the 17 evidenced screenplay roles to UTF-8 Version 6 XML.
///
/// For imported documents, managed paragraphs are reconciled into the retained
/// ordered source tree; unknown and source-owned nodes are re-emitted in place.
/// For a new document, semantic title/body content and the shared native role
/// defaults are emitted in a minimal supported FDX structure. ScriptCraft identity, legacy adapter fields,
/// runtime geometry inputs, and editing policies are not serialized. Output is
/// structurally re-read through the bounded reader before it is returned. This
/// does not establish desktop save/reopen guarantees or Final Draft application
/// interoperability. The reference-compatible UTF-8 standalone declaration is
/// emitted once for every serialization; imported XML is otherwise retained
/// semantically, not lexically: entity spelling, quote style and exact attribute
/// interleaving with namespace declarations are not guaranteed to be
/// byte-identical. UTF-8 is emitted; supported tree order, expanded names,
/// attributes, text, comments, processing instructions and opaque children are
/// checked against the re-read output.
pub fn write_v6(project: &Project) -> Result<Vec<u8>, FdxError> {
    write_v6_with_diagnostics(project).map_err(|failure| failure.error)
}

pub(crate) fn write_v6_for_save(
    project: &Project,
    context: &mut SaveContext,
) -> Result<Vec<u8>, FdxError> {
    write_v6_with_context(project, context).map_err(|failure| failure.error)
}

fn write_v6_with_diagnostics(project: &Project) -> Result<Vec<u8>, WriterFailure> {
    let mut context = SaveContext::new(SaveOperation::DirectSerialization);
    write_v6_with_context(project, &mut context)
}

fn write_v6_with_context(
    project: &Project,
    context: &mut SaveContext,
) -> Result<Vec<u8>, WriterFailure> {
    let mut stage = WriterStage::ProjectValidation;
    let mut structural_difference = None;
    let mut identity_difference = None;
    write_v6_inner(
        project,
        context,
        &mut stage,
        &mut structural_difference,
        &mut identity_difference,
    )
    .map_err(|error| {
        WriterFailure {
            error,
            stage,
            structural_difference,
            identity_difference,
        }
    })
}

fn write_v6_inner(
    project: &Project,
    context: &mut SaveContext,
    stage: &mut WriterStage,
    structural_difference: &mut Option<SourceTreeDifference>,
    identity_difference: &mut Option<IdentitySetDifference>,
) -> Result<Vec<u8>, FdxError> {
    project.validate().map_err(map_model_error)?;
    if !valid_document_ref_datetime(&context.timestamp) {
        return Err(FdxError::InvalidValue);
    }
    if WindowState::new(
        context.window_state.left,
        context.window_state.top,
        context.window_state.width,
        context.window_state.height,
    ) != Some(context.window_state)
    {
        return Err(FdxError::InvalidValue);
    }
    validate_project_output_budgets(project)?;

    let (nodes, expected_preserved_sections) = if let Some(state) = &project.preservation {
        *stage = WriterStage::PreservationValidation;
        state.validate().map_err(map_preservation_error)?;
        if state.source.format != SourceFormat::Fdx || state.source.encoding != SourceEncoding::Utf8
        {
            return Err(SaveDiagnosticCode::WriteSourceFormat.error());
        }
        *stage = WriterStage::OriginalSourceGraphComparison;
        let original = read_v6(&state.source.original_bytes).map_err(|error| {
            save_diagnostic_at(error, SaveDiagnosticCode::WriteOriginalRereadPropagation)
        })?;
        let original_state = original
            .project
            .preservation
            .as_ref()
            .ok_or_else(|| SaveDiagnosticCode::WriteOriginalPreservationMissing.error())?;
        if !same_non_paragraph_identities(&state.identities, &original_state.identities)
            || state.references != original_state.references
            || state.board_graph != original_state.board_graph
        {
            return Err(SaveDiagnosticCode::WriteOriginalIdentityGraphMismatch.error());
        }

        let mut sections = state.sections.clone();
        let mut effective_identities = state.identities.clone();
        set_preserved_root_version(&mut sections)?;
        if context.operation != SaveOperation::DirectSerialization {
            effective_identities.extend(reconcile_document_lineage(
                &mut sections,
                &state.identities,
                context,
            )?);
            reconcile_root_window_state(&mut sections, context.window_state)?;
            reconcile_root_text_state(&mut sections, &project, context.text_state)?;
        }
        *stage = WriterStage::ManagedSourceReconciliation;
        reconcile_project_elements(&mut sections, project)?;
        let original_paragraph_ids = original_state
            .identities
            .iter()
            .filter(|identity| identity.domain == ForeignIdDomain::Paragraph)
            .map(|identity| identity.value.clone())
            .collect::<BTreeSet<_>>();
        let inserted_paragraph_ids = collect_inserted_managed_paragraph_ids(
            &sections,
            &original_paragraph_ids,
        );
        format_inserted_paragraph_whitespace(&mut sections, &inserted_paragraph_ids);
        let expected_sections = sections.clone();
        let nodes = sections
            .into_iter()
            .flat_map(|section| section.children)
            .collect::<Vec<_>>();
        (nodes, Some((expected_sections, effective_identities)))
    } else {
        *stage = WriterStage::NewDocumentGeneration;
        let mut nodes = make_new_document_nodes(project, context)?;
        format_generated_structure(&mut nodes);
        (nodes, None)
    };

    *stage = WriterStage::XmlWriting;
    let declaration = if project.preservation.is_some() {
        // Source-backed output assigns declaration-to-root separation solely to
        // the retained top-level nodes emitted below.
        XML_DECLARATION_WITHOUT_TRAILING_LF
    } else {
        // Preserve the canonical declaration-plus-LF convention for new files.
        XML_DECLARATION
    };
    let mut output = declaration.to_owned();
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
    *stage = WriterStage::OutputReread;
    let written = read_v6(&bytes).map_err(|error| {
        save_diagnostic_at(error, SaveDiagnosticCode::WriteOutputRereadPropagation)
    })?;
    *stage = WriterStage::SemanticComparison;
    if !same_semantic_elements(&project.title_page, &written.project.title_page)
        || !same_semantic_elements(&project.elements, &written.project.elements)
    {
        return Err(SaveDiagnosticCode::WriteSemanticRereadMismatch.error());
    }
    if let Some((expected_sections, expected_identities)) = expected_preserved_sections {
        *stage = WriterStage::IdentityComparison;
        let actual_state = written
            .project
            .preservation
            .as_ref()
            .ok_or_else(|| SaveDiagnosticCode::WriteOutputPreservationMissing.error())?;
        let actual_identity_set = identity_set(&actual_state.identities);
        let expected_identity_set = identity_set(&expected_identities);
        if actual_identity_set != expected_identity_set {
            *identity_difference = first_identity_set_difference(
                &expected_identity_set,
                &actual_identity_set,
            );
            return Err(SaveDiagnosticCode::WriteIdentitySetMismatch.error());
        }
        let actual_sections = &actual_state.sections;
        if normalize_sections(actual_sections) != normalize_sections(&expected_sections) {
            *stage = WriterStage::SourceTreeComparison;
            let expected = normalize_sections(&expected_sections);
            let actual = normalize_sections(actual_sections);
            let difference = first_source_tree_difference(&expected, &actual)
                .expect("unequal normalized source trees must have a structural difference");
            let public_detail = difference.public_detail();
            *structural_difference = Some(difference);
            return Err(FdxError::SaveDiagnosticSourceTreeMismatch(public_detail));
        }
    }
    Ok(bytes)
}

pub(crate) fn save_diagnostic_at(error: FdxError, code: SaveDiagnosticCode) -> FdxError {
    if error == FdxError::UnsupportedConstruct {
        code.error()
    } else {
        error
    }
}

fn same_non_paragraph_identities(left: &[ForeignIdentity], right: &[ForeignIdentity]) -> bool {
    left.iter()
        .filter(|identity| identity.domain != ForeignIdDomain::Paragraph)
        .eq(right
            .iter()
            .filter(|identity| identity.domain != ForeignIdDomain::Paragraph))
}

fn identity_set(identities: &[ForeignIdentity]) -> BTreeSet<ForeignIdentity> {
    identities.iter().cloned().collect()
}

fn first_identity_set_difference(
    expected: &BTreeSet<ForeignIdentity>,
    actual: &BTreeSet<ForeignIdentity>,
) -> Option<IdentitySetDifference> {
    if let Some(identity) = expected.difference(actual).next() {
        return Some(IdentitySetDifference {
            domain: identity_domain_summary(&identity.domain),
            missing: true,
            unexpected: false,
        });
    }
    actual.difference(expected).next().map(|identity| IdentitySetDifference {
        domain: identity_domain_summary(&identity.domain),
        missing: false,
        unexpected: true,
    })
}

fn identity_domain_summary(domain: &ForeignIdDomain) -> IdentityDomainSummary {
    match domain {
        ForeignIdDomain::Document => IdentityDomainSummary::Document,
        ForeignIdDomain::Paragraph => IdentityDomainSummary::Paragraph,
        ForeignIdDomain::Scene => IdentityDomainSummary::Scene,
        ForeignIdDomain::Beat => IdentityDomainSummary::Beat,
        ForeignIdDomain::CastMember => IdentityDomainSummary::CastMember,
        ForeignIdDomain::Character => IdentityDomainSummary::Character,
        ForeignIdDomain::ListItem => IdentityDomainSummary::ListItem,
        ForeignIdDomain::Board => IdentityDomainSummary::Board,
        ForeignIdDomain::Image => IdentityDomainSummary::Image,
        ForeignIdDomain::Revision => IdentityDomainSummary::Revision,
        ForeignIdDomain::Tag => IdentityDomainSummary::Tag,
        ForeignIdDomain::Other(_) => IdentityDomainSummary::Other,
    }
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

fn reconcile_document_lineage(
    sections: &mut [PreservedSection],
    existing_identities: &[ForeignIdentity],
    context: &mut SaveContext,
) -> Result<Vec<ForeignIdentity>, FdxError> {
    if !valid_document_ref_datetime(&context.timestamp) {
        return Err(FdxError::InvalidValue);
    }
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

    let mut reserved = existing_identities
        .iter()
        .map(|identity| identity.value.clone())
        .collect::<BTreeSet<_>>();
    collect_preserved_id_attributes(&root.children, &mut reserved);

    let document_ref_indices = root
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, node)| match node {
            PreservedNode::Element(element) if element.name.local_name == "DocumentRef" => {
                Some(index)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if document_ref_indices.len() > 1 {
        return Err(SaveDiagnosticCode::LineageAmbiguousDocumentRef.error());
    }

    let mut additions = Vec::new();
    let document_ref_index = if let Some(index) = document_ref_indices.first().copied() {
        let PreservedNode::Element(document_ref) = &root.children[index] else {
            return Err(SaveDiagnosticCode::LineageAmbiguousDocumentRef.error());
        };
        if document_ref.name.namespace_uri != root.name.namespace_uri {
            return Err(SaveDiagnosticCode::LineageAmbiguousDocumentRef.error());
        }
        index
    } else {
        let document_ref_id = context.allocate_lineage_id(&mut reserved)?;
        additions.push(ForeignIdentity::new(
            ForeignIdDomain::Document,
            document_ref_id.clone(),
        ).map_err(map_preservation_error)?);
        let document_ref = new_lineage_element(
            &root.name,
            "DocumentRef",
            vec![
                ("DateTime".to_owned(), context.timestamp.clone()),
                ("id".to_owned(), document_ref_id),
            ],
        );
        let content_index = root
            .children
            .iter()
            .position(|node| matches!(node,
                PreservedNode::Element(element)
                    if element.name.local_name == "Content"
                        && element.name.namespace_uri == root.name.namespace_uri
            ))
            .ok_or(FdxError::MissingContent)?;
        root.children
            .insert(content_index, PreservedNode::Element(document_ref));
        content_index
    };

    let PreservedNode::Element(document_ref) = &mut root.children[document_ref_index] else {
        return Err(SaveDiagnosticCode::LineageAmbiguousDocumentRef.error());
    };
    let datetime_indices = document_ref
        .attributes
        .iter()
        .enumerate()
        .filter_map(|(index, attribute)| {
            (attribute.name.namespace_uri.is_none() && attribute.name.local_name == "DateTime")
                .then_some(index)
        })
        .collect::<Vec<_>>();
    if datetime_indices.len() > 1 {
        return Err(SaveDiagnosticCode::LineageAmbiguousDocumentRefDateTime.error());
    }
    let previous_datetime = datetime_indices
        .first()
        .and_then(|index| document_ref.attributes.get(*index))
        .map(|attribute| attribute.value.as_str())
        .filter(|value| valid_document_ref_datetime(value))
        .map(str::to_owned);
    let xref_datetime = context
        .opened_timestamp
        .as_deref()
        .filter(|value| valid_document_ref_datetime(value))
        .or(previous_datetime.as_deref())
        .unwrap_or(context.timestamp.as_str())
        .to_owned();

    if let Some(index) = datetime_indices.first().copied() {
        document_ref.attributes[index].value = context.timestamp.clone();
    } else {
        let insert_at = document_ref
            .attributes
            .iter()
            .position(|attribute| {
                attribute.name.namespace_uri.is_none()
                    && attribute.name.local_name.eq_ignore_ascii_case("id")
            })
            .unwrap_or(0);
        document_ref.attributes.insert(
            insert_at,
            PreservedAttribute {
                name: plain_source_name("DateTime"),
                value: context.timestamp.clone(),
            },
        );
    }

    if context.operation == SaveOperation::SaveAs {
        let xref_id = context.allocate_lineage_id(&mut reserved)?;
        let xref_identity = ForeignIdentity::new(identity_domain("XRef"), xref_id.clone())
            .map_err(map_preservation_error)?;
        let xref = new_lineage_element(
            &document_ref.name,
            "XRef",
            vec![
                ("DateTime".to_owned(), xref_datetime),
                ("id".to_owned(), xref_id),
            ],
        );
        append_lineage_xref(root, document_ref_index, xref)?;
        additions.push(xref_identity);
    }
    Ok(additions)
}

fn collect_preserved_id_attributes(nodes: &[PreservedNode], values: &mut BTreeSet<String>) {
    for node in nodes {
        let PreservedNode::Element(element) = node else {
            continue;
        };
        values.extend(element.attributes.iter().filter_map(|attribute| {
            (attribute.name.namespace_uri.is_none()
                && attribute.name.local_name.eq_ignore_ascii_case("id"))
                .then(|| attribute.value.clone())
        }));
        collect_preserved_id_attributes(&element.children, values);
    }
}

fn new_lineage_element(
    parent: &SourceName,
    local_name: &str,
    attributes: Vec<(String, String)>,
) -> PreservedElement {
    let prefix = parent
        .qualified_name
        .as_deref()
        .and_then(|qualified| qualified.rsplit_once(':').map(|(prefix, _)| prefix));
    let qualified_name = prefix.map_or_else(
        || local_name.to_owned(),
        |prefix| format!("{prefix}:{local_name}"),
    );
    PreservedElement {
        name: SourceName {
            namespace_uri: parent.namespace_uri.clone(),
            local_name: local_name.to_owned(),
            qualified_name: Some(qualified_name),
        },
        attributes: attributes
            .into_iter()
            .map(|(name, value)| PreservedAttribute {
                name: plain_source_name(&name),
                value,
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

fn plain_source_name(local_name: &str) -> SourceName {
    SourceName {
        namespace_uri: None,
        local_name: local_name.to_owned(),
        qualified_name: Some(local_name.to_owned()),
    }
}

fn append_lineage_xref(
    root: &mut PreservedElement,
    document_ref_index: usize,
    xref: PreservedElement,
) -> Result<(), FdxError> {
    let root_preserves_space = root.attributes.iter().any(|attribute| {
        attribute.name.namespace_uri.as_deref() == Some("http://www.w3.org/XML/1998/namespace")
            && attribute.name.local_name == "space"
            && attribute.value == "preserve"
    });
    let PreservedNode::Element(document_ref) = root
        .children
        .get_mut(document_ref_index)
        .ok_or(SaveDiagnosticCode::LineageAmbiguousDocumentRef.error())?
    else {
        return Err(SaveDiagnosticCode::LineageAmbiguousDocumentRef.error());
    };
    let document_ref_preserves_space = document_ref
        .attributes
        .iter()
        .any(|attribute| {
            attribute.name.namespace_uri.as_deref() == Some("http://www.w3.org/XML/1998/namespace")
                && attribute.name.local_name == "space"
                && attribute.value == "preserve"
        });
    if root_preserves_space || document_ref_preserves_space {
        return append_xref_without_formatting(document_ref, xref);
    }
    let format_safe = document_ref.children.iter().all(|node| match node {
        PreservedNode::Element(element) => element.name.namespace_uri == document_ref.name.namespace_uri,
        PreservedNode::Text(text) => text.chars().all(char::is_whitespace),
        _ => false,
    });
    if !format_safe {
        return append_xref_without_formatting(document_ref, xref);
    }
    append_xref_with_formatting(document_ref, xref)
}

fn append_xref_without_formatting(
    document_ref: &mut PreservedElement,
    xref: PreservedElement,
) -> Result<(), FdxError> {
    let index = lineage_xref_insert_index(document_ref)?;
    document_ref
        .children
        .insert(index, PreservedNode::Element(xref));
    Ok(())
}

fn append_xref_with_formatting(
    document_ref: &mut PreservedElement,
    xref: PreservedElement,
) -> Result<(), FdxError> {
    let index = lineage_xref_insert_index(document_ref)?;
    let has_following_node = index < document_ref.children.len();
    let following_is_whitespace = matches!(
        document_ref.children.get(index),
        Some(PreservedNode::Text(text)) if text.chars().all(char::is_whitespace)
    );
    document_ref
        .children
        .insert(index, PreservedNode::Text("\n    ".to_owned()));
    document_ref
        .children
        .insert(index + 1, PreservedNode::Element(xref));
    if has_following_node && !following_is_whitespace {
        document_ref
            .children
            .insert(index + 2, PreservedNode::Text("\n    ".to_owned()));
    } else if !has_following_node {
        document_ref
            .children
            .push(PreservedNode::Text("\n  ".to_owned()));
    }
    Ok(())
}

fn lineage_xref_insert_index(document_ref: &PreservedElement) -> Result<usize, FdxError> {
    let mut last_xref = None;
    for (index, node) in document_ref.children.iter().enumerate() {
        if let PreservedNode::Element(element) = node {
            if element.name.local_name == "XRef" {
                if element.name.namespace_uri != document_ref.name.namespace_uri {
                    return Err(SaveDiagnosticCode::LineageAmbiguousDocumentRef.error());
                }
                last_xref = Some(index);
            }
        }
    }
    Ok(last_xref.map_or_else(
        || {
            document_ref
                .children
                .iter()
                .rposition(|node| !matches!(node, PreservedNode::Text(text) if text.chars().all(char::is_whitespace)))
                .map_or(0, |index| index + 1)
        },
        |index| index + 1,
    ))
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
            section.children = normalize_source_tree(&section.children);
            section
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StructuralNodeKind {
    Element,
    Text,
    CData,
    Comment,
    ProcessingInstruction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StructuralNodeSummary {
    kind: StructuralNodeKind,
    namespace_uri: Option<String>,
    local_name: Option<String>,
    child_count: usize,
    attribute_count: usize,
}

/// Private, value-free first-difference metadata for unit tests. The path is a
/// sequence of ordered child indexes prefixed by the source-section index.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceTreeDifference {
    node_path: Vec<usize>,
    field: &'static str,
    expected_detail: Option<String>,
    actual_detail: Option<String>,
    expected: Option<StructuralNodeSummary>,
    actual: Option<StructuralNodeSummary>,
}

impl SourceTreeDifference {
    fn public_detail(&self) -> String {
        const MAX_DETAIL_BYTES: usize = 240;
        const MAX_PATH_DEPTH: usize = 12;
        let mut path = String::from("section[");
        path.push_str(&self.node_path.first().copied().unwrap_or(0).to_string());
        path.push(']');
        for (depth, index) in self.node_path.iter().skip(1).enumerate() {
            if depth >= MAX_PATH_DEPTH {
                path.push_str("/…");
                break;
            }
            path.push_str("/node[");
            path.push_str(&index.to_string());
            path.push(']');
        }
        let expected = self.expected_detail.as_deref().unwrap_or("absent");
        let actual = self.actual_detail.as_deref().unwrap_or("absent");
        let detail = format!("path={path}; field={}; expected={expected}; actual={actual}", self.field);
        if detail.len() <= MAX_DETAIL_BYTES {
            detail
        } else {
            let mut bounded = detail;
            bounded.truncate(MAX_DETAIL_BYTES.saturating_sub(3));
            bounded.push_str("...");
            bounded
        }
    }
}

fn difference(
    node_path: Vec<usize>,
    field: &'static str,
    expected_detail: Option<String>,
    actual_detail: Option<String>,
    expected: Option<StructuralNodeSummary>,
    actual: Option<StructuralNodeSummary>,
) -> SourceTreeDifference {
    SourceTreeDifference {
        node_path,
        field,
        expected_detail,
        actual_detail,
        expected,
        actual,
    }
}

fn safe_xml_name(name: &str) -> String {
    const MAX_NAME_BYTES: usize = 36;
    let mut safe = String::new();
    for character in name.chars() {
        if safe.len() >= MAX_NAME_BYTES {
            safe.push_str("...");
            break;
        }
        safe.push(if character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.') {
            character
        } else {
            '_'
        });
    }
    safe
}

fn safe_name_detail(value: Option<&str>) -> Option<String> {
    value.map(|value| safe_xml_name(value))
}

fn source_name_difference(
    expected: &SourceName,
    actual: &SourceName,
) -> Option<(&'static str, Option<String>, Option<String>)> {
    if expected.local_name != actual.local_name {
        return Some((
            "element.local-name",
            safe_name_detail(Some(&expected.local_name)),
            safe_name_detail(Some(&actual.local_name)),
        ));
    }
    if expected.namespace_uri != actual.namespace_uri {
        return Some((
            "element.namespace",
            Some(if expected.namespace_uri.is_some() { "present" } else { "absent" }.into()),
            Some(if actual.namespace_uri.is_some() { "present" } else { "absent" }.into()),
        ));
    }
    if expected.qualified_name != actual.qualified_name {
        return Some((
            "element.qualified-name",
            Some(if expected.qualified_name.is_some() { "present" } else { "absent" }.into()),
            Some(if actual.qualified_name.is_some() { "present" } else { "absent" }.into()),
        ));
    }
    None
}

fn ownership_category(ownership: &SourceOwnership) -> &'static str {
    match ownership {
        SourceOwnership::Opaque => "opaque",
        SourceOwnership::Managed(_) => "managed",
        SourceOwnership::Attached(_) => "attached",
    }
}

fn anchor_category(anchor: &Option<SourceAnchorId>) -> &'static str {
    if anchor.is_some() { "present" } else { "absent" }
}

fn text_classification(text: &str) -> String {
    let class = if text.is_empty() {
        "empty"
    } else if text.chars().all(char::is_whitespace) {
        "whitespace-only"
    } else {
        "contains-non-whitespace"
    };
    format!("length={},{}", text.chars().count(), class)
}

fn node_description(node: &PreservedNode) -> String {
    match node {
        PreservedNode::Element(element) => format!(
            "element:{};children={};attributes={}",
            safe_xml_name(&element.name.local_name),
            element.children.len(),
            element.attributes.len()
        ),
        PreservedNode::Text(text) => format!("text:{}", text_classification(text)),
        PreservedNode::CData(text) => format!("cdata:{}", text_classification(text)),
        PreservedNode::Comment(text) => format!("comment:{}", text_classification(text)),
        PreservedNode::ProcessingInstruction { .. } => "processing-instruction".into(),
    }
}

fn structural_node_summary(node: &PreservedNode) -> StructuralNodeSummary {
    match node {
        PreservedNode::Element(element) => StructuralNodeSummary {
            kind: StructuralNodeKind::Element,
            namespace_uri: element.name.namespace_uri.clone(),
            local_name: Some(element.name.local_name.clone()),
            child_count: element.children.len(),
            attribute_count: element.attributes.len(),
        },
        PreservedNode::Text(_) => StructuralNodeSummary {
            kind: StructuralNodeKind::Text,
            namespace_uri: None,
            local_name: None,
            child_count: 0,
            attribute_count: 0,
        },
        PreservedNode::CData(_) => StructuralNodeSummary {
            kind: StructuralNodeKind::CData,
            namespace_uri: None,
            local_name: None,
            child_count: 0,
            attribute_count: 0,
        },
        PreservedNode::Comment(_) => StructuralNodeSummary {
            kind: StructuralNodeKind::Comment,
            namespace_uri: None,
            local_name: None,
            child_count: 0,
            attribute_count: 0,
        },
        PreservedNode::ProcessingInstruction { .. } => StructuralNodeSummary {
            kind: StructuralNodeKind::ProcessingInstruction,
            namespace_uri: None,
            local_name: None,
            child_count: 0,
            attribute_count: 0,
        },
    }
}

fn first_source_tree_difference(
    expected: &[PreservedSection],
    actual: &[PreservedSection],
) -> Option<SourceTreeDifference> {
    let section_count = expected.len().max(actual.len());
    for section_index in 0..section_count {
        let expected_section = expected.get(section_index);
        let actual_section = actual.get(section_index);
        let Some(expected_section) = expected_section else {
            return Some(difference(vec![section_index], "section.presence", None,
                actual_section.map(|section| format!("kind={:?},ordinal={}", section.kind, section.ordinal)),
                None, actual_section.and_then(|section| section.children.first()).map(structural_node_summary)));
        };
        let Some(actual_section) = actual_section else {
            return Some(difference(vec![section_index], "section.presence",
                Some(format!("kind={:?},ordinal={}", expected_section.kind, expected_section.ordinal)), None,
                expected_section.children.first().map(structural_node_summary), None));
        };
        let section_path = vec![section_index];
        if let Some((field, left, right)) = source_name_difference(&expected_section.name, &actual_section.name) {
            let field = match field {
                "element.local-name" => "section.local-name",
                "element.namespace" => "section.namespace",
                _ => "section.qualified-name",
            };
            return Some(difference(section_path, field, left, right,
                expected_section.children.first().map(structural_node_summary), actual_section.children.first().map(structural_node_summary)));
        }
        if expected_section.kind != actual_section.kind {
            return Some(difference(section_path, "section.kind", Some(format!("{:?}", expected_section.kind)),
                Some(format!("{:?}", actual_section.kind)), expected_section.children.first().map(structural_node_summary),
                actual_section.children.first().map(structural_node_summary)));
        }
        if expected_section.ordinal != actual_section.ordinal {
            return Some(difference(section_path, "section.ordinal", Some(expected_section.ordinal.to_string()),
                Some(actual_section.ordinal.to_string()), expected_section.children.first().map(structural_node_summary),
                actual_section.children.first().map(structural_node_summary)));
        }
        if let Some(difference) = first_node_sequence_difference(
            &expected_section.children,
            &actual_section.children,
            vec![section_index],
        ) {
            return Some(difference);
        }
    }
    None
}

fn first_node_sequence_difference(
    expected: &[PreservedNode],
    actual: &[PreservedNode],
    path: Vec<usize>,
) -> Option<SourceTreeDifference> {
    let node_count = expected.len().max(actual.len());
    for child_index in 0..node_count {
        let mut child_path = path.clone();
        child_path.push(child_index);
        let expected_node = expected.get(child_index);
        let actual_node = actual.get(child_index);
        let (Some(expected_node), Some(actual_node)) = (expected_node, actual_node) else {
            let expected_detail = expected_node.map_or_else(
                || format!("absent;sibling-count={}", expected.len()),
                |node| format!("{};sibling-count={}", node_description(node), expected.len()),
            );
            let actual_detail = actual_node.map_or_else(
                || format!("absent;sibling-count={}", actual.len()),
                |node| format!("{};sibling-count={}", node_description(node), actual.len()),
            );
            return Some(difference(child_path, "child.presence", Some(expected_detail),
                Some(actual_detail), expected_node.map(structural_node_summary), actual_node.map(structural_node_summary)));
        };
        if expected_node != actual_node
            && (actual[child_index + 1..].contains(expected_node)
                || expected[child_index + 1..].contains(actual_node))
        {
            return Some(difference(
                child_path,
                "child.order",
                Some(node_description(expected_node)),
                Some(node_description(actual_node)),
                Some(structural_node_summary(expected_node)),
                Some(structural_node_summary(actual_node)),
            ));
        }
        match (expected_node, actual_node) {
            (PreservedNode::Element(expected_element), PreservedNode::Element(actual_element)) => {
                let expected_summary = Some(structural_node_summary(expected_node));
                let actual_summary = Some(structural_node_summary(actual_node));
                if let Some((field, left, right)) = source_name_difference(&expected_element.name, &actual_element.name) {
                    return Some(difference(child_path, field, left, right, expected_summary, actual_summary));
                }
                let common_attributes = expected_element.attributes.len().min(actual_element.attributes.len());
                for attribute_index in 0..common_attributes {
                    let left = &expected_element.attributes[attribute_index];
                    let right = &actual_element.attributes[attribute_index];
                    if left.name != right.name {
                        return Some(difference(child_path, "attribute.name-or-order",
                            safe_name_detail(Some(&left.name.local_name)), safe_name_detail(Some(&right.name.local_name)),
                            expected_summary, actual_summary));
                    }
                    if left.value != right.value {
                        return Some(difference(child_path, "attribute.value", safe_name_detail(Some(&left.name.local_name)),
                            safe_name_detail(Some(&right.name.local_name)), expected_summary, actual_summary));
                    }
                }
                if expected_element.attributes.len() != actual_element.attributes.len() {
                    let left = expected_element.attributes.get(common_attributes).map(|attribute| safe_xml_name(&attribute.name.local_name));
                    let right = actual_element.attributes.get(common_attributes).map(|attribute| safe_xml_name(&attribute.name.local_name));
                    return Some(difference(child_path, "attribute.count-or-presence", left, right, expected_summary, actual_summary));
                }
                let common_bindings = expected_element.namespace_bindings.len().min(actual_element.namespace_bindings.len());
                for binding_index in 0..common_bindings {
                    let left = &expected_element.namespace_bindings[binding_index];
                    let right = &actual_element.namespace_bindings[binding_index];
                    if left.prefix != right.prefix {
                        return Some(difference(child_path, "namespace-binding.prefix-or-order",
                            Some(if left.prefix.is_empty() { "default".into() } else { safe_xml_name(&left.prefix) }),
                            Some(if right.prefix.is_empty() { "default".into() } else { safe_xml_name(&right.prefix) }),
                            expected_summary, actual_summary));
                    }
                    if left.namespace_uri != right.namespace_uri {
                        return Some(difference(child_path, "namespace-binding.namespace",
                            Some("present".into()), Some("present".into()), expected_summary, actual_summary));
                    }
                }
                if expected_element.namespace_bindings.len() != actual_element.namespace_bindings.len() {
                    return Some(difference(child_path, "namespace-binding.count-or-presence",
                        expected_element.namespace_bindings.get(common_bindings).map(|binding| safe_xml_name(&binding.prefix)),
                        actual_element.namespace_bindings.get(common_bindings).map(|binding| safe_xml_name(&binding.prefix)),
                        expected_summary, actual_summary));
                }
                if expected_element.source_type != actual_element.source_type {
                    return Some(difference(child_path, "source-type", Some(if expected_element.source_type.is_some() { "present".into() } else { "absent".into() }),
                        Some(if actual_element.source_type.is_some() { "present".into() } else { "absent".into() }), expected_summary, actual_summary));
                }
                if expected_element.ownership != actual_element.ownership {
                    return Some(difference(child_path, "ownership", Some(ownership_category(&expected_element.ownership).into()),
                        Some(ownership_category(&actual_element.ownership).into()), expected_summary, actual_summary));
                }
                if expected_element.dependency_confidence != actual_element.dependency_confidence {
                    return Some(difference(child_path, "dependency-confidence", Some(format!("{:?}", expected_element.dependency_confidence)),
                        Some(format!("{:?}", actual_element.dependency_confidence)), expected_summary, actual_summary));
                }
                if expected_element.anchor != actual_element.anchor {
                    return Some(difference(child_path, "anchor", Some(anchor_category(&expected_element.anchor).into()),
                        Some(anchor_category(&actual_element.anchor).into()), expected_summary, actual_summary));
                }
                if let Some(difference) = first_node_sequence_difference(
                    &expected_element.children,
                    &actual_element.children,
                    child_path,
                ) {
                    return Some(difference);
                }
            }
            (PreservedNode::Text(expected_text), PreservedNode::Text(actual_text)) if expected_text == actual_text => {}
            (PreservedNode::Text(expected_text), PreservedNode::Text(actual_text)) => {
                return Some(difference(child_path, "text.content", Some(text_classification(expected_text)),
                    Some(text_classification(actual_text)), Some(structural_node_summary(expected_node)), Some(structural_node_summary(actual_node))));
            }
            (PreservedNode::CData(expected_text), PreservedNode::CData(actual_text)) if expected_text == actual_text => {}
            (PreservedNode::CData(expected_text), PreservedNode::CData(actual_text)) => {
                return Some(difference(child_path, "cdata.content", Some(text_classification(expected_text)),
                    Some(text_classification(actual_text)), Some(structural_node_summary(expected_node)), Some(structural_node_summary(actual_node))));
            }
            (PreservedNode::Comment(expected_text), PreservedNode::Comment(actual_text)) if expected_text == actual_text => {}
            (PreservedNode::Comment(expected_text), PreservedNode::Comment(actual_text)) => {
                return Some(difference(child_path, "comment.content", Some(text_classification(expected_text)),
                    Some(text_classification(actual_text)), Some(structural_node_summary(expected_node)), Some(structural_node_summary(actual_node))));
            }
            (
                PreservedNode::ProcessingInstruction {
                    target: expected_target,
                    data: expected_data,
                },
                PreservedNode::ProcessingInstruction {
                    target: actual_target,
                    data: actual_data,
                },
            ) if expected_target == actual_target && expected_data == actual_data => {}
            (PreservedNode::ProcessingInstruction { target: expected_target, data: expected_data },
             PreservedNode::ProcessingInstruction { target: actual_target, data: actual_data }) => {
                let (field, left, right) = if expected_target != actual_target {
                    ("processing-instruction.target", Some("present".into()), Some("present".into()))
                } else {
                    ("processing-instruction.data", Some(text_classification(expected_data)), Some(text_classification(actual_data)))
                };
                return Some(difference(child_path, field, left, right, Some(structural_node_summary(expected_node)),
                    Some(structural_node_summary(actual_node))));
            }
            _ => {
                return Some(difference(child_path, "node.kind-or-order", Some(node_description(expected_node)),
                    Some(node_description(actual_node)), Some(structural_node_summary(expected_node)), Some(structural_node_summary(actual_node))));
            }
        }
    }
    None
}

fn normalize_source_tree(nodes: &[PreservedNode]) -> Vec<PreservedNode> {
    let mut normalized = normalize_text_nodes(nodes);
    let mut stack = normalized.iter_mut().collect::<Vec<_>>();
    while let Some(node) = stack.pop() {
        if let PreservedNode::Element(element) = node {
            if matches!(element.ownership, SourceOwnership::Managed(_)) {
                // Semantic element IDs and source-anchor IDs are session-local;
                // they are not serialized FDX fields and may be reallocated on
                // reread. Compare the source XML structure, not these keys.
                element.ownership = SourceOwnership::Opaque;
                element.anchor = None;
            }
            stack.extend(element.children.iter_mut());
        }
    }
    normalized
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
        if matches!(&normalized_node, PreservedNode::Text(text) if text.is_empty()) {
            // The XML writer emits no bytes for an empty text node, so a reader
            // cannot reconstruct it. Omit it only at this comparison boundary.
            continue;
        }
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

fn format_generated_structure(nodes: &mut [PreservedNode]) {
    for node in nodes {
        if let PreservedNode::Element(element) = node {
            format_generated_element(element, 0, false);
        }
    }
}

fn format_generated_element(
    element: &mut PreservedElement,
    depth: usize,
    inherited_space_preserve: bool,
) {
    let space_preserve = xml_space_preserve(element, inherited_space_preserve);
    if space_preserve {
        return;
    }

    // Generated elements are formatted only when their children are known to
    // be element-only. Character data, comments, PIs and CDATA are never
    // surrounded with serializer whitespace.
    if element.children.is_empty()
        || !element
            .children
            .iter()
            .all(|child| matches!(child, PreservedNode::Element(_)))
    {
        return;
    }

    for child in &mut element.children {
        if let PreservedNode::Element(child) = child {
            format_generated_element(child, depth.saturating_add(1), space_preserve);
        }
    }

    let root_children = depth == 0 && element.name.local_name == FDX_ROOT;
    let child_indent = "  ".repeat(depth.saturating_add(1));
    let closing_indent = "  ".repeat(depth);
    let mut formatted = Vec::with_capacity(element.children.len().saturating_mul(2) + 1);
    for (index, child) in std::mem::take(&mut element.children).into_iter().enumerate() {
        if index == 0 {
            formatted.push(PreservedNode::Text(if root_children {
                format!("\n\n{child_indent}")
            } else {
                format!("\n{child_indent}")
            }));
        } else {
            formatted.push(PreservedNode::Text(if root_children {
                format!("\n\n{child_indent}")
            } else {
                format!("\n{child_indent}")
            }));
        }
        formatted.push(child);
    }
    formatted.push(PreservedNode::Text(format!("\n{closing_indent}")));
    element.children = formatted;
}

fn xml_space_preserve(element: &PreservedElement, inherited: bool) -> bool {
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

fn collect_inserted_managed_paragraph_ids(
    sections: &[PreservedSection],
    original_ids: &BTreeSet<String>,
) -> BTreeSet<String> {
    fn visit(nodes: &[PreservedNode], original_ids: &BTreeSet<String>, found: &mut BTreeSet<String>) {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if element.name.local_name == "Paragraph"
                && matches!(element.ownership, SourceOwnership::Managed(_))
            {
                if let Some(id) = element
                    .attributes
                    .iter()
                    .find(|attribute| attribute.name.local_name.eq_ignore_ascii_case("id"))
                    .map(|attribute| attribute.value.as_str())
                {
                    if !original_ids.contains(id) {
                        found.insert(id.to_owned());
                    }
                }
            }
            visit(&element.children, original_ids, found);
        }
    }

    let mut found = BTreeSet::new();
    for section in sections {
        visit(&section.children, original_ids, &mut found);
    }
    found
}

fn format_inserted_paragraph_whitespace(
    sections: &mut [PreservedSection],
    inserted_ids: &BTreeSet<String>,
) {
    if inserted_ids.is_empty() {
        return;
    }

    fn visit(
        nodes: &mut [PreservedNode],
        inherited_space_preserve: bool,
        inserted_ids: &BTreeSet<String>,
    ) {
        for node in nodes {
            let PreservedNode::Element(root) = node else {
                continue;
            };
            let root_preserve = xml_space_preserve(root, inherited_space_preserve);
            if root.name.local_name == FDX_ROOT {
                for child in &mut root.children {
                    let PreservedNode::Element(content) = child else {
                        continue;
                    };
                    if content.name.local_name == "Content" {
                        let content_preserve = xml_space_preserve(content, root_preserve);
                        if !content_preserve {
                            format_content_insertions(&mut content.children, inserted_ids);
                        }
                    }
                    visit(
                        std::slice::from_mut(child),
                        root_preserve,
                        inserted_ids,
                    );
                }
            } else {
                visit(&mut root.children, root_preserve, inserted_ids);
            }
        }
    }

    for section in sections {
        visit(&mut section.children, false, inserted_ids);
    }
}

fn format_content_insertions(
    children: &mut Vec<PreservedNode>,
    inserted_ids: &BTreeSet<String>,
) {
    // Only the unambiguous element-only Content shape is eligible. Comments,
    // PIs, CDATA, non-whitespace text and non-Paragraph children make the
    // context mixed/opaque, so source order and all whitespace remain intact.
    if children.iter().any(|child| match child {
        PreservedNode::Text(text) => !text.chars().all(char::is_whitespace),
        PreservedNode::Element(element) => element.name.local_name != "Paragraph",
        _ => true,
    }) {
        return;
    }
    let has_inserted = children.iter().any(|child| match child {
        PreservedNode::Element(element) if element.name.local_name == "Paragraph" => element
                .attributes
                .iter()
                .find(|attribute| attribute.name.local_name.eq_ignore_ascii_case("id"))
            .is_some_and(|attribute| inserted_ids.contains(&attribute.value)),
        _ => false,
    });
    if !has_inserted {
        return;
    }

    let mut formatted = Vec::with_capacity(children.len().saturating_mul(2) + 1);
    let mut pending_whitespace = Vec::new();
    let mut previous_was_inserted = false;
    let mut saw_paragraph = false;
    for child in std::mem::take(children) {
        if matches!(&child, PreservedNode::Text(text) if text.chars().all(char::is_whitespace)) {
            pending_whitespace.push(child);
            continue;
        }
        let PreservedNode::Element(element) = &child else {
            // Already excluded above; keep defensive fallback lossless.
            formatted.append(&mut pending_whitespace);
            formatted.push(child);
            continue;
        };
        let is_inserted = element
            .attributes
            .iter()
            .find(|attribute| attribute.name.local_name.eq_ignore_ascii_case("id"))
            .is_some_and(|attribute| inserted_ids.contains(&attribute.value));
        if !saw_paragraph && is_inserted {
            formatted.push(PreservedNode::Text("\n    ".to_owned()));
        } else if saw_paragraph && (previous_was_inserted || is_inserted) {
            formatted.push(PreservedNode::Text("\n    ".to_owned()));
        } else {
            formatted.append(&mut pending_whitespace);
        }
        pending_whitespace.clear();
        formatted.push(child);
        saw_paragraph = true;
        previous_was_inserted = is_inserted;
    }
    if previous_was_inserted {
        formatted.push(PreservedNode::Text("\n  ".to_owned()));
    } else {
        formatted.append(&mut pending_whitespace);
    }
    *children = formatted;
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
            )
            .map_err(|error| {
                save_diagnostic_at(error, SaveDiagnosticCode::ReconcileProjectNodePropagation)
            })?;
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
        return Err(SaveDiagnosticCode::ReconcileProjectManagedOrderMismatch.error());
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
            return Err(SaveDiagnosticCode::ReconcileNodeDuplicateManagedIdentity.error());
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
            SectionKind::Other => {
                return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error())
            }
        }
        .copied()
        .ok_or_else(|| SaveDiagnosticCode::ReconcileNodeSemanticElementMissing.error())?;
        reconcile_managed_paragraph(element, semantic).map_err(|error| {
            save_diagnostic_at(error, SaveDiagnosticCode::ReconcileManagedParagraphPropagation)
        })?;
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
        )
        .map_err(|error| {
            save_diagnostic_at(error, SaveDiagnosticCode::ReconcileProjectChildPropagation)
        })?;
    }
    Ok(())
}

fn reconcile_managed_paragraph(
    paragraph: &mut PreservedElement,
    semantic: &ScreenplayElement,
) -> Result<(), FdxError> {
    let role_type = role_to_fdx(semantic.role);
    if paragraph.source_type.as_deref() != Some(role_type) {
        return Err(SaveDiagnosticCode::ReconcileParagraphRoleMismatch.error());
    }
    let type_attributes = paragraph
        .attributes
        .iter()
        .filter(|attribute| {
            attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Type"
        })
        .collect::<Vec<_>>();
    if type_attributes.len() != 1 || type_attributes[0].value != role_type {
        return Err(SaveDiagnosticCode::ReconcileParagraphTypeAttributeMismatch.error());
    }

    if semantic.text.is_empty() {
        if !semantic.runs.is_empty() {
            return Err(SaveDiagnosticCode::ReconcileParagraphEmptyRunsMismatch.error());
        }
        for child in &paragraph.children {
            let PreservedNode::Element(text_element) = child else {
                continue;
            };
            if text_element.name.local_name != "Text"
                || text_element.name.namespace_uri != paragraph.name.namespace_uri
            {
                continue;
            }
            let mut text = String::new();
            for inline in &text_element.children {
                match inline {
                    PreservedNode::Text(value) => text.push_str(value),
                    // CDATA has explicit serialized syntax and is not an
                    // interchangeable serializer-invisible text leaf.
                    PreservedNode::CData(_) => {
                        return Err(SaveDiagnosticCode::ReconcileParagraphCdataInEmptyText.error())
                    }
                    _ => {
                        return Err(SaveDiagnosticCode::ReconcileParagraphUnsupportedEmptyChild.error())
                    }
                }
            }
            if !text.is_empty() {
                return Err(SaveDiagnosticCode::ReconcileParagraphUnexpectedEmptyText.error());
            }
        }
        paragraph.children.retain(|child| {
            !matches!(child,
                PreservedNode::Element(element)
                    if element.name.local_name == "Text"
                        && element.name.namespace_uri == paragraph.name.namespace_uri)
        });
        return Ok(());
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
    let plain_run;
    let semantic_runs = if semantic.runs.is_empty() && !semantic.text.is_empty() {
        plain_run = vec![TextRun {
            text: semantic.text.clone(),
            attributes: Vec::new(),
        }];
        plain_run.as_slice()
    } else {
        semantic.runs.as_slice()
    };
    if text_elements.len() != semantic_runs.len() {
        if text_elements.is_empty() && semantic_runs.is_empty() && semantic.text.is_empty() {
            return Ok(());
        }
        return Err(SaveDiagnosticCode::ReconcileParagraphRunCountMismatch.error());
    }
    for (text_element, run) in text_elements.iter_mut().zip(semantic_runs) {
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
            return Err(SaveDiagnosticCode::ReconcileParagraphRunProjectionMismatch.error());
        }
        if let Some(first) = text_element.children.first_mut() {
            match first {
                PreservedNode::Text(value) | PreservedNode::CData(value) => {
                    *value = run.text.clone();
                }
                _ => {
                    return Err(SaveDiagnosticCode::ReconcileParagraphUnexpectedFirstChild.error())
                }
            }
            for child in text_element.children.iter_mut().skip(1) {
                match child {
                    PreservedNode::Text(value) | PreservedNode::CData(value) => value.clear(),
                    _ => {
                        return Err(SaveDiagnosticCode::ReconcileParagraphUnexpectedLaterChild.error())
                    }
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

fn make_new_document_nodes(
    project: &Project,
    context: &mut SaveContext,
) -> Result<Vec<PreservedNode>, FdxError> {
    let mut root = preserved_element(
        "FinalDraft",
        vec![("DocumentType", "Script"), ("Version", FDX_VERSION)],
    );
    let mut lineage_ids = BTreeSet::new();
    let document_ref_id = context.allocate_lineage_id(&mut lineage_ids)?;
    let xref_id = if context.operation == SaveOperation::SaveAs {
        Some(context.allocate_lineage_id(&mut lineage_ids)?)
    } else {
        None
    };
    let mut document_ref = preserved_element(
        "DocumentRef",
        vec![("DateTime", context.timestamp.as_str()), ("id", &document_ref_id)],
    );
    if let Some(xref_id) = xref_id {
        document_ref.children.push(PreservedNode::Element(preserved_element(
            "XRef",
            vec![("DateTime", context.timestamp.as_str()), ("id", &xref_id)],
        )));
    }
    root.children
        .push(PreservedNode::Element(document_ref));
    let mut content = preserved_element("Content", Vec::new());
    let mut paragraph_ids = ParagraphIdAllocator::new(lineage_ids.into_iter());
    for (index, element) in project.elements.iter().enumerate() {
        let mut paragraph = new_paragraph(element, &mut paragraph_ids)?;
        if element.role == Role::SceneHeading {
            let scene_end = project.elements[index + 1..]
                .iter()
                .position(|candidate| candidate.role == Role::SceneHeading)
                .map_or(project.elements.len(), |offset| index + 1 + offset);
            let names = crate::scene_metadata::derived_character_projection(
                &project.elements[index + 1..scene_end],
            )
            .into_iter()
            .map(|(_, name)| name)
            .collect::<Vec<_>>();
            let PreservedNode::Element(heading) = &mut paragraph else {
                return Err(SaveDiagnosticCode::NewDocumentHeadingNodeMismatch.error());
            };
            let properties = crate::scene_metadata::new_scene_properties(
                &names,
                &mut paragraph_ids,
            )
            .map_err(|error| match error {
                crate::scene_metadata::SceneMetadataError::ParagraphIdentityEntropyUnavailable => {
                    FdxError::ParagraphIdentityEntropyUnavailable
                }
                crate::scene_metadata::SceneMetadataError::ParagraphIdentityCollisionLimit => {
                    FdxError::ParagraphIdentityCollisionLimit
                }
                _ => SaveDiagnosticCode::NewDocumentScenePropertiesUnsupported.error(),
            })?;
            let text_index = heading
                .children
                .iter()
                .position(|node| matches!(node, PreservedNode::Element(child) if child.name.local_name == "Text"))
                .unwrap_or(heading.children.len());
            heading
                .children
                .insert(text_index, PreservedNode::Element(properties));
        }
        content.children.push(paragraph);
    }
    root.children.push(PreservedNode::Element(content));
    root.children.push(PreservedNode::Element(window_state_element(
        &root.name,
        context.window_state,
    )));
    root.children.push(PreservedNode::Element(text_state_element(
        &root.name,
        project,
        context.text_state,
    )?));
    for formatting in formatting_in_native_order() {
        root.children
            .push(PreservedNode::Element(new_element_settings(formatting)));
    }
    if !project.title_page.is_empty() {
        let mut title_page = preserved_element("TitlePage", Vec::new());
        for element in &project.title_page {
            title_page
                .children
                .push(new_paragraph(element, &mut paragraph_ids)?);
        }
        root.children.push(PreservedNode::Element(title_page));
    }
    Ok(vec![PreservedNode::Element(root)])
}

fn window_state_element(parent: &SourceName, state: WindowState) -> PreservedElement {
    new_lineage_element(
        parent,
        "WindowState",
        vec![
            ("Height".to_owned(), state.height.to_string()),
            ("Left".to_owned(), state.left.to_string()),
            ("Mode".to_owned(), "Normal".to_owned()),
            ("Top".to_owned(), state.top.to_string()),
            ("Width".to_owned(), state.width.to_string()),
        ],
    )
}

fn text_state_element(
    parent: &SourceName,
    project: &Project,
    state: TextState,
) -> Result<PreservedElement, FdxError> {
    let (start, end) = text_state::selection_to_flat_offsets(project, state.selection)
        .ok_or(FdxError::InvalidValue)?;
    Ok(new_lineage_element(
        parent,
        "TextState",
        vec![
            ("Scaling".to_owned(), state.scaling.clamp(
                text_state::MIN_TEXT_SCALING,
                text_state::MAX_TEXT_SCALING,
            ).to_string()),
            ("Selection".to_owned(), format!("{start},{end}")),
            ("ShowInvisibles".to_owned(), "No".to_owned()),
        ],
    ))
}

fn reconcile_root_text_state(
    sections: &mut [PreservedSection],
    project: &Project,
    state: TextState,
) -> Result<(), FdxError> {
    let mut roots = sections.iter_mut().flat_map(|section| &mut section.children)
        .filter_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == FDX_ROOT => Some(element),
            _ => None,
        });
    let root = roots.next().ok_or(FdxError::InvalidRoot)?;
    if roots.next().is_some() {
        return Err(FdxError::InvalidRoot);
    }
    let indices = root.children.iter().enumerate().filter_map(|(index, node)| match node {
        PreservedNode::Element(element)
            if element.name.local_name == "TextState"
                && element.name.namespace_uri == root.name.namespace_uri => Some(index),
        _ => None,
    }).collect::<Vec<_>>();
    let canonical = text_state_element(&root.name, project, state)?;
    if let Some(first) = indices.first().copied() {
        root.children[first] = PreservedNode::Element(canonical);
        for index in indices.into_iter().skip(1).rev() {
            root.children.remove(index);
        }
        return Ok(());
    }

    let window = root.children.iter().position(|node| matches!(node,
        PreservedNode::Element(element)
            if element.name.local_name == "WindowState"
                && element.name.namespace_uri == root.name.namespace_uri
    ));
    let page_layout = root.children.iter().position(|node| matches!(node,
        PreservedNode::Element(element)
            if element.name.local_name == "PageLayout"
                && element.name.namespace_uri == root.name.namespace_uri
    ));
    let settings = root.children.iter().position(|node| matches!(node,
        PreservedNode::Element(element)
            if element.name.local_name == "ElementSettings"
                && element.name.namespace_uri == root.name.namespace_uri
    ));
    let mut insertion = window.map(|index| index + 1)
        .or_else(|| page_layout.map(|index| index + 1))
        .or(settings)
        .unwrap_or(root.children.len());
    while insertion < root.children.len()
        && matches!(root.children.get(insertion), Some(PreservedNode::Text(text)) if text.trim().is_empty())
    {
        insertion += 1;
    }
    let needs_leading_space = insertion == 0
        || matches!(insertion.checked_sub(1).and_then(|index| root.children.get(index)), Some(PreservedNode::Element(_)));
    if needs_leading_space {
        root.children.insert(insertion, PreservedNode::Text("\n\n  ".to_owned()));
        insertion += 1;
    }
    root.children.insert(insertion, PreservedNode::Element(canonical));
    let after = insertion + 1;
    if after == root.children.len()
        || matches!(root.children.get(after), Some(PreservedNode::Element(_)))
    {
        root.children.insert(after, PreservedNode::Text("\n\n  ".to_owned()));
    }
    Ok(())
}

fn reconcile_root_window_state(
    sections: &mut [PreservedSection],
    state: WindowState,
) -> Result<(), FdxError> {
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
    let window_indices = root
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, node)| match node {
            PreservedNode::Element(element)
                if element.name.local_name == "WindowState"
                    && element.name.namespace_uri == root.name.namespace_uri =>
            {
                Some(index)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let canonical = window_state_element(&root.name, state);
    if let Some(first) = window_indices.first().copied() {
        root.children[first] = PreservedNode::Element(canonical);
        for index in window_indices.into_iter().skip(1).rev() {
            root.children.remove(index);
        }
        return Ok(());
    }

    let page_layout = root.children.iter().position(|node| matches!(node,
        PreservedNode::Element(element)
            if element.name.local_name == "PageLayout"
                && element.name.namespace_uri == root.name.namespace_uri
    ));
    let settings = root.children.iter().position(|node| matches!(node,
        PreservedNode::Element(element)
            if element.name.local_name == "ElementSettings"
                && element.name.namespace_uri == root.name.namespace_uri
    ));
    let anchor = page_layout.map(|index| index + 1).or(settings).unwrap_or(root.children.len());
    // Keep existing inter-element whitespace on the leading side; where the
    // source is compact, add only the generated block's local separator.
    let mut insertion = anchor;
    if page_layout.is_some() {
        while insertion < root.children.len()
            && matches!(root.children.get(insertion), Some(PreservedNode::Text(text)) if text.trim().is_empty())
        {
            insertion += 1;
        }
    } else if settings.is_some() {
        while insertion > 0
            && matches!(root.children.get(insertion - 1), Some(PreservedNode::Text(text)) if text.trim().is_empty())
        {
            insertion -= 1;
        }
    }
    let needs_leading_space = insertion == 0
        || matches!(insertion.checked_sub(1).and_then(|index| root.children.get(index)), Some(PreservedNode::Element(_)));
    if needs_leading_space {
        root.children
            .insert(insertion, PreservedNode::Text("\n\n  ".to_owned()));
        insertion += 1;
    }
    root.children
        .insert(insertion, PreservedNode::Element(canonical));
    let after = insertion + 1;
    if after == root.children.len()
        || matches!(root.children.get(after), Some(PreservedNode::Element(_)))
    {
        root.children
            .insert(after, PreservedNode::Text("\n\n  ".to_owned()));
    }
    Ok(())
}

fn new_element_settings(
    formatting: &crate::formatting::RoleFormatting,
) -> PreservedElement {
    let mut settings = preserved_element(
        "ElementSettings",
        vec![("Type", formatting.fdx_label)],
    );
    let font = formatting.font;
    let mut font_attributes = vec![("AdornmentStyle".to_owned(), font.adornment_style.to_string())];
    if let Some(color) = font.color {
        font_attributes.push(("Color".to_owned(), color.to_owned()));
    }
    font_attributes.extend([
        ("Font".to_owned(), font.family.to_owned()),
        ("RevisionID".to_owned(), font.revision_id.to_string()),
        ("Size".to_owned(), font.size.to_string()),
        ("Style".to_owned(), font.style_tokens.join("+")),
    ]);
    settings
        .children
        .push(PreservedNode::Element(preserved_element_owned(
            "FontSpec",
            font_attributes,
        )));

    let paragraph = formatting.paragraph;
    settings
        .children
        .push(PreservedNode::Element(preserved_element_owned(
            "ParagraphSpec",
            vec![
                ("Alignment".to_owned(), paragraph.alignment.to_owned()),
                ("FirstIndent".to_owned(), paragraph.first_indent.to_owned()),
                ("Leading".to_owned(), paragraph.leading.to_owned()),
                ("LeftIndent".to_owned(), paragraph.left_indent.to_owned()),
                ("RightIndent".to_owned(), paragraph.right_indent.to_owned()),
                ("SpaceBefore".to_owned(), paragraph.space_before.to_string()),
                ("Spacing".to_owned(), paragraph.spacing.to_string()),
                (
                    "StartsNewPage".to_owned(),
                    if paragraph.starts_new_page { "Yes" } else { "No" }.to_owned(),
                ),
                ("Type".to_owned(), formatting.fdx_label.to_owned()),
            ],
        )));

    let behavior = formatting.behavior;
    settings
        .children
        .push(PreservedNode::Element(preserved_element_owned(
            "Behavior",
            vec![
                ("PaginateAs".to_owned(), behavior.paginate_as.to_owned()),
                ("ReturnKey".to_owned(), behavior.return_key.to_owned()),
                ("Shortcut".to_owned(), behavior.shortcut.to_owned()),
                ("TabKey".to_owned(), behavior.tab_key.to_owned()),
                ("WinShortcut".to_owned(), behavior.win_shortcut.to_owned()),
            ],
        )));

    settings.children.push(PreservedNode::Element(
        preserved_element_owned(
            "Outline",
            vec![
                (
                    "CanHide".to_owned(),
                    if formatting.outline.can_hide { "Yes" } else { "No" }.to_owned(),
                ),
                ("Level".to_owned(), formatting.outline.level.to_string()),
            ],
        ),
    ));
    settings
}

fn preserved_element_owned(name: &str, attributes: Vec<(String, String)>) -> PreservedElement {
    let mut element = preserved_element(name, Vec::new());
    element.attributes = attributes
        .into_iter()
        .map(|(name, value)| PreservedAttribute {
            name: SourceName {
                namespace_uri: None,
                local_name: name.clone(),
                qualified_name: Some(name),
            },
            value,
        })
        .collect();
    element
}

fn new_paragraph(
    semantic: &ScreenplayElement,
    paragraph_ids: &mut ParagraphIdAllocator,
) -> Result<PreservedNode, FdxError> {
    let mut paragraph = preserved_element(
        "Paragraph",
        vec![("Type", role_to_fdx(semantic.role)), ("id", "")],
    );
    paragraph.attributes[1].value = paragraph_ids.allocate().map_err(map_paragraph_id_error)?;
    if semantic.text.is_empty() {
        if !semantic.runs.is_empty() {
            return Err(SaveDiagnosticCode::NewParagraphEmptyRunsMismatch.error());
        }
        return Ok(PreservedNode::Element(paragraph));
    }
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
                return Err(SaveDiagnosticCode::NewParagraphNamespacedRunAttribute.error());
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

fn map_paragraph_id_error(error: ParagraphIdAllocationError) -> FdxError {
    match error {
        ParagraphIdAllocationError::EntropyUnavailable => {
            FdxError::ParagraphIdentityEntropyUnavailable
        }
        ParagraphIdAllocationError::CollisionLimit => FdxError::ParagraphIdentityCollisionLimit,
    }
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
    formatting_for_role(role).fdx_label
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
        return Err(SaveDiagnosticCode::WriteElementNamespacePrefixMissing.error());
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

fn raw_opened_document_ref_datetime(root: &RawElement) -> Option<String> {
    let root_namespace = root.name.namespace_uri.as_deref();
    let mut document_refs = root.children.iter().filter_map(|node| match node {
        RawNode::Element(element) if element.name.local_name == "DocumentRef" => Some(element),
        _ => None,
    });
    let document_ref = document_refs.next()?;
    if document_refs.next().is_some()
        || document_ref.name.namespace_uri.as_deref() != root_namespace
    {
        return None;
    }
    let mut datetimes = document_ref.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == "DateTime"
    });
    let value = datetimes.next()?.value.as_str();
    if datetimes.next().is_some() || !valid_document_ref_datetime(value) {
        return None;
    }
    Some(value.to_owned())
}

fn raw_root_window_state(root: &RawElement) -> Option<WindowState> {
    let root_namespace = root.name.namespace_uri.as_deref();
    let mut windows = root.children.iter().filter_map(|node| match node {
        RawNode::Element(element) if is_fdx_element(element, "WindowState", root_namespace) => {
            Some(element)
        }
        _ => None,
    });
    let window = windows.next()?;
    if windows.next().is_some() {
        return None;
    }

    let mut parsed = BTreeMap::new();
    for attribute in &window.attributes {
        if attribute.name.namespace_uri.is_none()
            && matches!(attribute.name.local_name.as_str(), "Left" | "Top" | "Width" | "Height")
            && parsed
                .insert(attribute.name.local_name.as_str(), attribute.value.as_str())
                .is_some()
        {
            return None;
        }
    }
    let parse = |name: &'static str| parsed.get(name)?.parse::<i32>().ok();
    WindowState::new(
        parse("Left")?,
        parse("Top")?,
        parse("Width")?,
        parse("Height")?,
    )
}

fn raw_root_text_state(root: &RawElement, project: &Project) -> TextState {
    let namespace = root.name.namespace_uri.as_deref();
    let mut blocks = root.children.iter().filter_map(|node| match node {
        RawNode::Element(element) if is_fdx_element(element, "TextState", namespace) => Some(element),
        _ => None,
    });
    let Some(block) = blocks.next() else { return TextState::DEFAULT };
    if blocks.next().is_some() {
        return TextState::DEFAULT;
    }
    let mut scaling = None;
    let mut selection = None;
    let mut scaling_ambiguous = false;
    let mut selection_ambiguous = false;
    for attribute in &block.attributes {
        if attribute.name.namespace_uri.is_some() { continue; }
        match attribute.name.local_name.as_str() {
            "Scaling" => {
                if scaling.replace(attribute.value.as_str()).is_some() { scaling_ambiguous = true; }
            }
            "Selection" => {
                if selection.replace(attribute.value.as_str()).is_some() { selection_ambiguous = true; }
            }
            _ => {}
        }
    }
    let scale = if scaling_ambiguous { 100 } else {
        scaling.and_then(|value| value.parse::<u64>().ok())
            .and_then(TextState::normalized_scaling)
            .unwrap_or(100)
    };
    let mapped = if selection_ambiguous { None } else {
        selection.and_then(|value| text_state::parse_flat_selection(project, value))
    };
    TextState { scaling: scale, selection: mapped.unwrap_or(TextState::DEFAULT.selection) }
}

pub(crate) fn opened_document_ref_datetime_from_project(project: &Project) -> Option<String> {
    let preservation = project.preservation.as_ref()?;
    let mut roots = preservation
        .sections
        .iter()
        .flat_map(|section| &section.children)
        .filter_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == FDX_ROOT => Some(element),
            _ => None,
        });
    let root = roots.next()?;
    if roots.next().is_some() {
        return None;
    }
    let mut document_refs = root.children.iter().filter_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "DocumentRef" => Some(element),
        _ => None,
    });
    let document_ref = document_refs.next()?;
    if document_refs.next().is_some()
        || document_ref.name.namespace_uri != root.name.namespace_uri
    {
        return None;
    }
    let mut datetimes = document_ref.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == "DateTime"
    });
    let value = datetimes.next()?.value.as_str();
    if datetimes.next().is_some() || !valid_document_ref_datetime(value) {
        return None;
    }
    Some(value.to_owned())
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
    source_layout_settings: &mut SourceLayoutSettings,
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
        children,
    };
    if let (Some(semantic), Some(anchor_id)) = (managed, anchor_id) {
        if current_section == SectionKind::Content {
            if let Some(overrides) = direct_paragraph_overrides(element) {
                source_layout_settings
                    .direct_paragraph_overrides
                    .insert(semantic.id.clone(), overrides);
            }
        }
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

fn direct_paragraph_overrides(
    element: &RawElement,
) -> Option<crate::formatting::SourceParagraphOverrides> {
    let mut overrides = crate::formatting::SourceParagraphOverrides::default();
    let paragraph = &mut overrides.paragraph;
    paragraph.alignment = attribute(element, "Alignment").and_then(|alignment| match alignment {
        "Right" => Some(crate::ParagraphAlignment::Right),
        "Center" => Some(crate::ParagraphAlignment::Center),
        "Left" => Some(crate::ParagraphAlignment::Left),
        _ => None,
    });
    paragraph.first_indent = attribute(element, "FirstIndent")
        .and_then(|value| bounded_inches(value, true));
    paragraph.left_indent = attribute(element, "LeftIndent")
        .and_then(|value| bounded_inches(value, false));
    paragraph.right_indent = attribute(element, "RightIndent")
        .and_then(|value| bounded_inches(value, false));
    paragraph.leading_regular = (attribute(element, "Leading") == Some("Regular")).then_some(true);
    paragraph.space_before = attribute(element, "SpaceBefore")
        .and_then(crate::FixedPoint::parse_points)
        .filter(|value| (0..=79_200).contains(&value.raw()));
    paragraph.spacing_single = (attribute(element, "Spacing") == Some("1")).then_some(true);
    overrides.starts_new_page = attribute(element, "StartsNewPage").and_then(|value| match value {
        "Yes" => Some(true),
        "No" => Some(false),
        _ => None,
    });

    (paragraph.alignment.is_some()
        || paragraph.first_indent.is_some()
        || paragraph.left_indent.is_some()
        || paragraph.right_indent.is_some()
        || paragraph.leading_regular.is_some()
        || paragraph.space_before.is_some()
        || paragraph.spacing_single.is_some()
        || overrides.starts_new_page.is_some())
    .then_some(overrides)
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
    if runs.iter().all(|run| run.text.is_empty()) {
        // Empty Text elements have no canonical semantic contribution, regardless
        // of their formatting attributes. The preservation tree retains them
        // until managed writer reconciliation.
        runs.clear();
    }
    ScreenplayElement::with_runs(id, role, runs, Some(source_provenance)).map_err(map_model_error)
}

/// Apply only explicit source settings that have direct counterparts in the
/// model. Unknown settings remain source-owned in the preservation tree.
fn read_source_layout_settings(
    root: &RawElement,
    root_ns: Option<&str>,
) -> Result<SourceLayoutSettings, FdxError> {
    let mut settings = SourceLayoutSettings::default();
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
        let font_specs = setting
            .children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(child)
                    if child.name.namespace_uri.as_deref() == root_ns
                        && child.name.local_name == "FontSpec" =>
                {
                    Some(child)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        // Duplicate FontSpec children are ambiguous as a whole. Leave every
        // projected font field at its role default without modifying source.
        if let [font] = font_specs.as_slice() {
            let mut projection = crate::formatting::SourceFontSpec::default();
            projection.family = attribute(font, "Font")
                .filter(|value| !value.is_empty() && value.len() <= 128)
                .map(str::to_owned);
            projection.size = attribute(font, "Size").and_then(parse_bounded_font_size);
            projection.color = attribute(font, "Color")
                .filter(|value| value.len() <= 128)
                .map(str::to_owned);
            if let Some(style) = attribute(font, "Style") {
                let tokens = style.split('+').collect::<BTreeSet<_>>();
                projection.bold = Some(tokens.contains("Bold"));
                projection.italic = Some(tokens.contains("Italic"));
                projection.underline = Some(tokens.contains("Underline"));
                projection.all_caps = Some(tokens.contains("AllCaps"));
                projection.hidden_text = Some(tokens.contains("HiddenText"));
                settings.role_print_visibility[role_index(role)] = Some(
                    if projection.hidden_text == Some(true) {
                        PrintVisibility::Hidden
                    } else {
                        PrintVisibility::Visible
                    },
                );
            }
            settings.role_font_specs[role_index(role)] = projection;
        }
        let paragraph_specs = setting
            .children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(child)
                    if child.name.namespace_uri.as_deref() == root_ns
                        && child.name.local_name == "ParagraphSpec" =>
                {
                    Some(child)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        if paragraph_specs.len() > 1 {
            return Err(FdxError::DuplicateSection);
        }
        let Some(paragraph) = paragraph_specs.first() else {
            continue;
        };
        // A conflicting child Type makes the entire specification ambiguous;
        // the preserved XML remains untouched and shared role defaults apply.
        if attribute(paragraph, "Type").is_some_and(|paragraph_role| paragraph_role != source_role) {
            continue;
        }

        let index = role_index(role);
        let mut projection = crate::SourceParagraphSpec::default();
        projection.alignment = attribute(paragraph, "Alignment").map(|alignment| match alignment {
            "Right" => crate::ParagraphAlignment::Right,
            "Center" => crate::ParagraphAlignment::Center,
            "Left" => crate::ParagraphAlignment::Left,
            _ => crate::ParagraphAlignment::Left,
        });
        projection.first_indent = attribute(paragraph, "FirstIndent")
            .and_then(|value| bounded_inches(value, true));
        projection.left_indent = attribute(paragraph, "LeftIndent")
            .and_then(|value| bounded_inches(value, false));
        projection.right_indent = attribute(paragraph, "RightIndent")
            .and_then(|value| bounded_inches(value, false));
        projection.leading_regular =
            (attribute(paragraph, "Leading") == Some("Regular")).then_some(true);
        projection.space_before = attribute(paragraph, "SpaceBefore")
            .and_then(crate::FixedPoint::parse_points)
            .filter(|value| (0..=79_200).contains(&value.raw()));
        projection.spacing_single =
            (attribute(paragraph, "Spacing") == Some("1")).then_some(true);
        settings.role_paragraph_specs[index] = projection;
        if let Some(starts_new_page) = attribute(paragraph, "StartsNewPage") {
            settings.role_starts_new_page[index] = match starts_new_page {
                "Yes" => Some(true),
                "No" => Some(false),
                _ => None,
            };
        }
    }
    Ok(settings)
}

fn parse_bounded_font_size(value: &str) -> Option<u8> {
    if value.is_empty() || value.len() > 2 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let size = value.parse::<u8>().ok()?;
    (1..=72).contains(&size).then_some(size)
}

fn bounded_inches(value: &str, signed: bool) -> Option<crate::FixedPoint> {
    let fixed = crate::FixedPoint::parse_inches(value)?;
    // The current runtime page is bounded to US Letter. Values outside that
    // model budget remain source-owned but are not effective projections.
    if fixed.raw().unsigned_abs() > 61_200 || (!signed && fixed.raw() < 0) {
        return None;
    }
    Some(fixed)
}

fn role_from_fdx(value: &str) -> Option<Role> {
    formatting_in_native_order()
        .iter()
        .find(|formatting| formatting.fdx_label == value)
        .map(|formatting| formatting.role)
}

fn collect_identities(
    root: &RawElement,
    identities: &mut Vec<ForeignIdentity>,
    board_graph: &mut BoardGraph,
) -> Result<(), FdxError> {
    let mut seen = BTreeSet::new();
    let mut scoped_title_page_paragraphs = BTreeMap::new();
    let mut definitions = Vec::new();
    walk_elements(root, &mut |element, nearest_board, in_title_page, depth| {
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
        // Paragraphs in TitlePage source content are opaque unless they are
        // direct, recognized title-page roles managed by preserve_node. Count
        // those raw ID occurrences separately so only repetitions that have no
        // global consumer are omitted from the global Paragraph definitions.
        let opaque_title_page_paragraph = in_title_page
            && element.name.local_name == "Paragraph"
            && !(depth == 2 && attribute(element, "Type").and_then(role_from_fdx).is_some());
        if opaque_title_page_paragraph {
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

    // Repeated scoped occurrences are deliberately not global definitions, but
    // they may not shadow an identity that is globally registered elsewhere.
    // A single opaque title-page Paragraph retains its existing global identity.
    for (identity, count) in &scoped_title_page_paragraphs {
        if seen.contains(identity) {
            return Err(FdxError::AmbiguousIdentity);
        }
        if *count == 1 {
            seen.insert(identity.clone());
        }
    }

    let mut source_order = 0u32;
    for (name, identity, attributes, scoped_title_page) in &definitions {
        let repeated_scoped_title_page = *scoped_title_page
            && scoped_title_page_paragraphs
                .get(identity)
                .is_some_and(|count| *count > 1);
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
            if !repeated_scoped_title_page {
                identities.push(identity.clone());
            }
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
    walk_elements(root, &mut |element, nearest_board, _in_title_page, _depth| {
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
    F: FnMut(&RawElement, Option<&ForeignIdentity>, bool, usize) -> Result<(), FdxError>,
{
    fn visit<F>(
        element: &RawElement,
        nearest_board: Option<&ForeignIdentity>,
        in_title_page: bool,
        depth: usize,
        root_namespace: Option<&str>,
        visitor: &mut F,
    ) -> Result<(), FdxError>
    where
        F: FnMut(&RawElement, Option<&ForeignIdentity>, bool, usize) -> Result<(), FdxError>,
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
        visitor(element, active_board, in_title_page, depth)?;
        for child in &element.children {
            if let RawNode::Element(child) = child {
                let child_in_title_page = in_title_page
                    || (depth == 0 && is_fdx_element(child, "TitlePage", root_namespace));
                visit(
                    child,
                    active_board,
                    child_in_title_page,
                    depth.saturating_add(1),
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
        false,
        0,
        element.name.namespace_uri.as_deref(),
        visitor,
    )
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
        Error::ParagraphIdentityEntropyUnavailable => {
            FdxError::ParagraphIdentityEntropyUnavailable
        }
        Error::ParagraphIdentityCollisionLimit => FdxError::ParagraphIdentityCollisionLimit,
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
        let xml = b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Note'><FontSpec Style='Italic+HiddenText'/></ElementSettings><ElementSettings Type='New Act'><ParagraphSpec Type='New Act' StartsNewPage='Yes'/></ElementSettings><Content><Paragraph Type='Action' id='before'><Text>before</Text></Paragraph><Paragraph Type='Note' id='note'><Text>hidden</Text></Paragraph><Paragraph Type='New Act' id='act'><Text>act</Text></Paragraph></Content></FinalDraft>";
        let result = read_v6(xml);
        assert!(result.is_ok());
        if let Ok(result) = result {
            let settings = &result
                .project
                .preservation
                .as_ref()
                .unwrap_or_else(|| unreachable!())
                .source_layout_settings;
            assert_eq!(
                settings.print_visibility(Role::Note),
                PrintVisibility::Hidden
            );
            assert!(settings.starts_new_page(Role::NewAct));
            let layout = crate::layout::ScreenplayLayout::build(&result.project)
                .unwrap_or_else(|_| unreachable!());
            assert_eq!(layout.pages.len(), 2);
            assert_eq!(layout.pages[0].lines.len(), 1);
            assert_eq!(layout.pages[0].lines[0].text, "before");
            assert_eq!(layout.pages[1].lines[0].text, "act");
            let preserved = result
                .project
                .preservation
                .as_ref()
                .unwrap_or_else(|| unreachable!());
            assert_eq!(preserved.source.original_bytes, xml);
            let written = write_v6(&result.project).unwrap_or_else(|_| unreachable!());
            let written = String::from_utf8(written).unwrap_or_else(|_| unreachable!());
            assert!(written.contains("Style=\"Italic+HiddenText\""));
            assert!(written.contains("StartsNewPage=\"Yes\""));
        }
    }

    #[test]
    fn imported_paragraph_fields_project_independently_and_keep_source_authoritative() {
        let xml = b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Action'><ParagraphSpec Type='Action' Alignment='Diagonal' FirstIndent='-0.10' LeftIndent='0.88' RightIndent='7.50' Leading='Future' SpaceBefore='14.25' Spacing='2' StartsNewPage='Maybe'/></ElementSettings><ElementSettings Type='Parenthetical'><ParagraphSpec Type='Dialogue' Alignment='Right' LeftIndent='3.00' RightIndent='5.50' StartsNewPage='Yes'/></ElementSettings><Content><Paragraph Type='Action' id='a'><Text>one</Text></Paragraph></Content></FinalDraft>";
        let project = read_v6(xml).unwrap_or_else(|_| unreachable!()).project;
        let settings = project
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!())
            .source_layout_settings();
        let action = settings.paragraph_spec(Role::Action);
        assert_eq!(action.alignment, Some(crate::ParagraphAlignment::Left));
        assert_eq!(action.first_indent.map(crate::FixedPoint::raw), Some(-720));
        assert_eq!(action.left_indent.map(crate::FixedPoint::raw), Some(6_336));
        assert_eq!(action.right_indent.map(crate::FixedPoint::raw), Some(54_000));
        assert_eq!(action.leading_regular, None);
        assert_eq!(action.space_before.map(crate::FixedPoint::raw), Some(1_425));
        assert_eq!(action.spacing_single, None);
        assert!(!settings.starts_new_page(Role::Action));

        // The conflicting child Type excludes the full ParagraphSpec projection.
        assert_eq!(
            settings.paragraph_spec(Role::Parenthetical),
            crate::SourceParagraphSpec::default()
        );
        assert!(!settings.starts_new_page(Role::Parenthetical));

        let bytes = write_v6(&project).unwrap_or_else(|_| unreachable!());
        let saved = String::from_utf8(bytes).unwrap_or_else(|_| unreachable!());
        assert!(saved.contains("Alignment=\"Diagonal\""));
        assert!(saved.contains("Leading=\"Future\""));
        assert!(saved.contains("StartsNewPage=\"Maybe\""));
        assert!(saved.contains("Type=\"Dialogue\" Alignment=\"Right\""));
    }

    #[test]
    fn duplicate_role_paragraph_specs_remain_structural_errors() {
        let xml = b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Action'><ParagraphSpec Type='Action'/><ParagraphSpec Type='Action'/></ElementSettings><Content><Paragraph Type='Action' id='a'><Text>one</Text></Paragraph></Content></FinalDraft>";
        assert!(matches!(read_v6(xml), Err(FdxError::DuplicateSection)));
    }

    #[test]
    fn source_settings_remain_authoritative_when_runtime_projection_changes() {
        let xml = b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Note'><FontSpec Style='Italic+HiddenText'/></ElementSettings><Content><Paragraph Type='Note' id='source-note'><Text>synthetic hidden note</Text></Paragraph></Content></FinalDraft>";
        let mut project = read_v6(xml).unwrap_or_else(|_| unreachable!()).project;
        let state = project
            .preservation
            .as_mut()
            .unwrap_or_else(|| unreachable!());
        state.source_layout_settings = SourceLayoutSettings::default();

        let bytes = write_v6(&project).unwrap_or_else(|_| unreachable!());
        let output = std::str::from_utf8(&bytes).unwrap_or("");
        assert!(output.contains("Style=\"Italic+HiddenText\""));
        let reopened = read_v6(&bytes).unwrap_or_else(|_| unreachable!());
        let settings = reopened
            .project
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!())
            .source_layout_settings();
        assert_eq!(
            settings.print_visibility(Role::Note),
            PrintVisibility::Hidden
        );
    }

    #[test]
    fn new_native_documents_emit_complete_ordered_role_defaults() {
        let project = Project::new().unwrap_or_else(|_| unreachable!());
        let bytes = write_v6(&project).unwrap_or_else(|_| unreachable!());
        let document = read_v6(&bytes).unwrap_or_else(|_| unreachable!());
        let state = document
            .project
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!());
        let root = state.sections[0]
            .children
            .iter()
            .find_map(|node| match node {
                PreservedNode::Element(element) if element.name.local_name == FDX_ROOT => {
                    Some(element)
                }
                _ => None,
            })
            .unwrap_or_else(|| unreachable!());
        let settings = root
            .children
            .iter()
            .filter_map(|node| match node {
                PreservedNode::Element(element)
                    if element.name.local_name == "ElementSettings" =>
                {
                    Some(element)
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(settings.len(), 17);
        for (setting, formatting) in settings.iter().zip(formatting_in_native_order()) {
            assert_eq!(attribute_preserved(setting, "Type"), Some(formatting.fdx_label));
            let children = setting
                .children
                .iter()
                .filter_map(|node| match node {
                    PreservedNode::Element(element) => Some(element),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                children
                    .iter()
                    .map(|element| element.name.local_name.as_str())
                    .collect::<Vec<_>>(),
                ["FontSpec", "ParagraphSpec", "Behavior", "Outline"]
            );
            assert_eq!(attribute_preserved(&children[1], "Type"), Some(formatting.fdx_label));
        }

        let new_act = settings
            .iter()
            .find(|setting| attribute_preserved(setting, "Type") == Some("New Act"))
            .unwrap_or_else(|| unreachable!());
        let new_act_paragraph = new_act.children.iter().find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "ParagraphSpec" => {
                Some(element)
            }
            _ => None,
        });
        assert_eq!(
            new_act_paragraph.and_then(|element| attribute_preserved(element, "StartsNewPage")),
            Some("Yes")
        );
        let summary = settings
            .iter()
            .find(|setting| attribute_preserved(setting, "Type") == Some("Summary"))
            .unwrap_or_else(|| unreachable!());
        let summary_font = summary.children.iter().find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "FontSpec" => {
                Some(element)
            }
            _ => None,
        });
        assert_eq!(
            summary_font.and_then(|element| attribute_preserved(element, "Color")),
            Some("#87878F8FE0E0")
        );

        let blank_reference = read_v6(include_bytes!("../../../.reference/reference.fdx"))
            .unwrap_or_else(|_| unreachable!());
        let populated_reference = read_v6(include_bytes!("../../../.reference/reference2.fdx"))
            .unwrap_or_else(|_| unreachable!());
        let blank_root = preserved_root(&blank_reference.project);
        let populated_root = preserved_root(&populated_reference.project);
        assert_eq!(element_settings_shape(root), element_settings_shape(blank_root));
        assert_eq!(
            element_settings_shape(root),
            element_settings_shape(populated_root)
        );
    }

    #[test]
    fn reference_declaration_and_generated_section_order_survive_reopened_saves() {
        for include_title_page in [false, true] {
            let mut project = Project::new().unwrap_or_else(|_| unreachable!());
            if include_title_page {
                project.title_page.push(
                    ScreenplayElement::new("title-element", Role::Action, "Title")
                        .unwrap_or_else(|_| unreachable!()),
                );
            }

            let mut expected_order = vec!["Content".to_owned()];
            expected_order.extend((0..17).map(|_| "ElementSettings".to_owned()));
            if include_title_page {
                expected_order.push("TitlePage".to_owned());
            }

            for _ in 0..3 {
                let bytes = write_v6(&project).unwrap_or_else(|_| unreachable!());
                let output = std::str::from_utf8(&bytes).unwrap_or("");
                assert!(output.starts_with(XML_DECLARATION));
                assert_eq!(output.matches("<?xml").count(), 1);

                let reopened = read_v6(&bytes).unwrap_or_else(|_| unreachable!());
                assert_eq!(
                    top_level_element_names(preserved_root(&reopened.project)),
                    expected_order
                );
                project = reopened.project;
            }
        }
    }

    #[test]
    fn imported_top_level_source_order_is_not_canonicalized() {
        let source = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\" ?><FinalDraft DocumentType=\"Script\" Version=\"6\"><VendorBefore/><ElementSettings Type=\"Action\"/><Content/><VendorAfter/></FinalDraft>";
        let project = read_v6(source)
            .unwrap_or_else(|_| unreachable!())
            .project;

        let bytes = write_v6(&project).unwrap_or_else(|_| unreachable!());
        let output = std::str::from_utf8(&bytes).unwrap_or("");
        assert!(output.starts_with(XML_DECLARATION_WITHOUT_TRAILING_LF));
        assert_eq!(output.matches("<?xml").count(), 1);
        let reopened = read_v6(&bytes).unwrap_or_else(|_| unreachable!());
        assert_eq!(
            top_level_element_names(preserved_root(&reopened.project)),
            vec![
                "VendorBefore".to_owned(),
                "ElementSettings".to_owned(),
                "Content".to_owned(),
                "VendorAfter".to_owned(),
            ]
        );
    }

    #[test]
    fn new_document_layout_defaults_match_after_native_save_and_reopen() {
        let mut project = Project::new().unwrap_or_else(|_| unreachable!());
        project.elements = [
            ("action", Role::Action, "before"),
            ("note", Role::Note, "private note"),
            ("summary", Role::Summary, "private summary"),
            ("outline", Role::Outline1, "private outline"),
            ("act", Role::NewAct, "ACT TWO"),
            ("after", Role::Action, "after"),
        ]
        .into_iter()
        .map(|(id, role, text)| {
            ScreenplayElement::new(id, role, text).unwrap_or_else(|_| unreachable!())
        })
        .collect();

        let before = crate::layout::ScreenplayLayout::build(&project)
            .unwrap_or_else(|_| unreachable!());
        let bytes = write_v6(&project).unwrap_or_else(|_| unreachable!());
        let reopened = read_v6(&bytes).unwrap_or_else(|_| unreachable!());
        let after = crate::layout::ScreenplayLayout::build(&reopened.project)
            .unwrap_or_else(|_| unreachable!());

        let rendered_projection = |layout: &crate::layout::ScreenplayLayout| {
            layout
                .pages
                .iter()
                .map(|page| {
                    page.lines
                        .iter()
                        .map(|line| (line.element, line.role, line.text.clone()))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(rendered_projection(&before), rendered_projection(&after));
        assert_eq!(before.pages.len(), 2);
        assert_eq!(before.pages[0].lines.len(), 1);
        assert_eq!(before.pages[0].lines[0].role, Role::Action);
        assert!(before.pages[1]
            .lines
            .iter()
            .any(|line| line.role == Role::NewAct));
        assert_eq!(before.pages[1].lines.last().map(|line| line.text.as_str()), Some("after"));
    }

    #[test]
    fn imported_custom_element_settings_are_not_filled_or_rewritten() {
        let xml = b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Action' VendorSetting='keep'><FontSpec Style='' VendorFontFlag='custom'/><ParagraphSpec StartsNewPage='Yes' VendorParagraphFlag='custom'/></ElementSettings><Content><Paragraph Type='Action' id='source-action'><Text>source action</Text></Paragraph></Content><VendorSection Marker='untouched'/></FinalDraft>";
        let project = read_v6(xml).unwrap_or_else(|_| unreachable!()).project;
        let output = write_v6(&project).unwrap_or_else(|_| unreachable!());
        let output = std::str::from_utf8(&output).unwrap_or("");
        assert_eq!(output.matches("<ElementSettings ").count(), 1);
        assert!(output.contains("Type=\"Action\" VendorSetting=\"keep\""));
        assert!(output.contains("VendorFontFlag=\"custom\""));
        assert!(output.contains("StartsNewPage=\"Yes\" VendorParagraphFlag=\"custom\""));
        assert!(output.contains("<VendorSection Marker=\"untouched\"/>"));
    }

    fn attribute_preserved<'a>(element: &'a PreservedElement, name: &str) -> Option<&'a str> {
        element
            .attributes
            .iter()
            .find(|attribute| {
                attribute.name.namespace_uri.is_none() && attribute.name.local_name == name
            })
            .map(|attribute| attribute.value.as_str())
    }

    fn preserved_root(project: &Project) -> &PreservedElement {
        project
            .preservation
            .as_ref()
            .and_then(|state| state.sections.first())
            .and_then(|section| {
                section.children.iter().find_map(|node| match node {
                    PreservedNode::Element(element) if element.name.local_name == FDX_ROOT => {
                        Some(element)
                    }
                    _ => None,
                })
            })
            .unwrap_or_else(|| unreachable!())
    }

    fn top_level_element_names(root: &PreservedElement) -> Vec<String> {
        root.children
            .iter()
            .filter_map(|node| match node {
                PreservedNode::Element(element) => Some(element.name.local_name.clone()),
                _ => None,
            })
            .collect()
    }

    fn element_settings_shape(
        root: &PreservedElement,
    ) -> Vec<Vec<(String, Vec<(String, String)>)>> {
        root.children
            .iter()
            .filter_map(|node| match node {
                PreservedNode::Element(element)
                    if element.name.local_name == "ElementSettings" =>
                {
                    Some(element)
                }
                _ => None,
            })
            .map(|setting| {
                std::iter::once(setting)
                    .chain(setting.children.iter().filter_map(|node| match node {
                        PreservedNode::Element(element) => Some(element),
                        _ => None,
                    }))
                    .map(|element| {
                        (
                            element.name.local_name.clone(),
                            element
                                .attributes
                                .iter()
                                .map(|attribute| {
                                    (
                                        attribute.name.local_name.clone(),
                                        attribute.value.clone(),
                                    )
                                })
                                .collect(),
                        )
                    })
                    .collect()
            })
            .collect()
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
        let mut project = Project::new().unwrap_or_else(|_| unreachable!());
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
    fn retains_foreign_metadata_without_project_metadata_projection() {
        let xml = b"<FinalDraft DocumentType='Script' Version='6' VendorRoot='keep'><DocumentRef Date='2026-10-31' XRef='foreign-ref'/><Content><Paragraph Type='Action' id='p1'><Text>body</Text></Paragraph></Content><VendorMetadata key='opaque'>foreign title and date</VendorMetadata></FinalDraft>";
        let project = read_v6(xml).unwrap_or_else(|_| unreachable!()).project;
        let bytes = write_v6(&project).unwrap_or_else(|_| unreachable!());
        let output = std::str::from_utf8(&bytes).unwrap_or("");
        assert!(output.contains("VendorRoot=\"keep\""));
        assert!(output.contains("Date=\"2026-10-31\""));
        assert!(output.contains("XRef=\"foreign-ref\""));
        assert!(output.contains("foreign title and date"));
    }

    #[test]
    fn serializes_legacy_adapter_content_without_persisting_retired_app_state() {
        let legacy_json = include_str!("../tests/fixtures/legacy_project.json")
            .replace("\"margin_top\":72", "\"margin_top\":73");
        let project = Project::from_json(&legacy_json).unwrap_or_else(|_| unreachable!());
        let bytes = write_v6(&project).unwrap_or_else(|_| unreachable!());
        let output = std::str::from_utf8(&bytes).unwrap_or("");
        assert!(output.contains("Version=\"6\""));
        let generated = read_v6(&bytes)
            .ok()
            .and_then(|document| {
                document.project.preservation.and_then(|state| {
                    state
                        .identities
                        .into_iter()
                        .find(|identity| identity.domain == ForeignIdDomain::Paragraph)
                        .map(|identity| identity.value)
                })
            })
            .unwrap_or_default();
        let parsed = uuid::Uuid::parse_str(&generated).ok();
        assert_eq!(generated.len(), 36);
        assert_eq!(
            parsed.as_ref().and_then(|value| value.get_version()),
            Some(uuid::Version::Random)
        );
        assert_eq!(
            parsed.as_ref().map(|value| value.get_variant()),
            Some(uuid::Variant::RFC4122)
        );
        assert_eq!(
            parsed.map(|value| value.hyphenated().to_string()).as_deref(),
            Some(generated.as_str())
        );
        assert!(!output.contains(project.elements[0].id.as_str()));
        assert!(!output.contains("legacy-project-1"));
        assert!(!output.contains("Legacy title"));
        assert!(!output.contains("Legacy author"));
        assert!(!output.contains("contact@example.invalid"));
        assert!(!output.contains("2026-10-31T00:00:00Z"));
        assert!(!output.contains("margin_top"));
        assert!(!output.contains("Courier Prime"));
        assert!(!output.contains("uppercase_cues"));
        assert!(!output.contains("auto_parenthetical"));
        assert!(!output.contains("strict_transition_boundaries"));
        assert!(!output.contains("ScriptCraft"));
        let reopened = read_v6(&bytes).unwrap_or_else(|_| unreachable!());
        assert_eq!(reopened.project.elements[0].text, project.elements[0].text);
        assert_eq!(reopened.project.elements[0].role, project.elements[0].role);
        assert!(reopened.project.legacy_json_state.is_none());
    }

    #[test]
    fn rejects_unsafe_source_reconciliation() {
        let mut inserted = read_v6(SYNTHETIC)
            .unwrap_or_else(|_| unreachable!())
            .project;
        inserted.elements.push(
            ScreenplayElement::new("unanchored-new", Role::Action, "new")
                .unwrap_or_else(|_| unreachable!()),
        );
        assert_eq!(
            write_v6(&inserted),
            Err(SaveDiagnosticCode::ReconcileProjectManagedOrderMismatch.error())
        );

        let mut invalid_text = Project::new().unwrap_or_else(|_| unreachable!());
        invalid_text.elements[0].text = "bad\u{1}".into();
        assert_eq!(write_v6(&invalid_text), Err(FdxError::InvalidValue));
    }

    #[test]
    fn v4_wo003_codes_are_unique_stable_and_prefix_the_underlying_error() {
        let codes = [
            (SaveDiagnosticCode::WriteSourceFormat, 1),
            (SaveDiagnosticCode::WriteOriginalRereadPropagation, 2),
            (SaveDiagnosticCode::WriteOriginalPreservationMissing, 3),
            (SaveDiagnosticCode::WriteOriginalIdentityGraphMismatch, 4),
            (SaveDiagnosticCode::ReconcileProjectManagedOrderMismatch, 5),
            (SaveDiagnosticCode::ReconcileNodeDuplicateManagedIdentity, 6),
            (SaveDiagnosticCode::ReconcileNodeManagedOutsideSection, 7),
            (SaveDiagnosticCode::ReconcileNodeSemanticElementMissing, 8),
            (SaveDiagnosticCode::ReconcileProjectNodePropagation, 9),
            (SaveDiagnosticCode::ReconcileParagraphRoleMismatch, 10),
            (SaveDiagnosticCode::ReconcileParagraphTypeAttributeMismatch, 11),
            (SaveDiagnosticCode::ReconcileParagraphEmptyRunsMismatch, 12),
            (SaveDiagnosticCode::ReconcileParagraphCdataInEmptyText, 13),
            (SaveDiagnosticCode::ReconcileParagraphUnsupportedEmptyChild, 14),
            (SaveDiagnosticCode::ReconcileParagraphUnexpectedEmptyText, 15),
            (SaveDiagnosticCode::ReconcileParagraphRunCountMismatch, 16),
            (SaveDiagnosticCode::ReconcileParagraphRunProjectionMismatch, 17),
            (SaveDiagnosticCode::ReconcileParagraphUnexpectedFirstChild, 18),
            (SaveDiagnosticCode::ReconcileParagraphUnexpectedLaterChild, 19),
            (SaveDiagnosticCode::NewDocumentHeadingNodeMismatch, 20),
            (SaveDiagnosticCode::NewDocumentScenePropertiesUnsupported, 21),
            (SaveDiagnosticCode::NewParagraphEmptyRunsMismatch, 22),
            (SaveDiagnosticCode::NewParagraphNamespacedRunAttribute, 23),
            (SaveDiagnosticCode::WriteElementNamespacePrefixMissing, 24),
            (SaveDiagnosticCode::WriteOutputRereadPropagation, 25),
            (SaveDiagnosticCode::WriteSemanticRereadMismatch, 26),
            (SaveDiagnosticCode::WriteOutputPreservationMissing, 27),
            (SaveDiagnosticCode::WriteCandidatePreservationMissing, 28),
            (SaveDiagnosticCode::WriteIdentitySetMismatch, 29),
            (SaveDiagnosticCode::WriteNormalizedSourceTreeMismatch, 30),
            (SaveDiagnosticCode::StorageRereadPropagation, 31),
            (SaveDiagnosticCode::StorageSemanticProjectionMismatch, 32),
            (SaveDiagnosticCode::ReconcileProjectChildPropagation, 33),
            (SaveDiagnosticCode::ReconcileManagedParagraphPropagation, 34),
        ];
        for (code, expected_number) in &codes {
            assert_eq!(code.number(), *expected_number);
        }
        let numbers = codes
            .iter()
            .map(|(code, _)| code.number())
            .collect::<BTreeSet<_>>();
        assert_eq!(numbers.len(), codes.len());
        assert_eq!(numbers, (1..=34).collect());
        assert_eq!(
            SaveDiagnosticCode::NewParagraphNamespacedRunAttribute.error().to_string(),
            "23: FDX contains a construct that cannot be projected safely"
        );
        assert_eq!(
            SaveDiagnosticCode::StorageSemanticProjectionMismatch.number(),
            32
        );
    }

    #[test]
    fn v4_wo003_writer_failure_exposes_private_stage_and_numeric_prefix() {
        let mut project = Project::new().unwrap_or_else(|_| unreachable!());
        project.elements[0].text = "safe text".to_owned();
        project.elements[0].runs = vec![TextRun {
            text: "safe text".to_owned(),
            attributes: vec![RunAttribute {
                name: "vendor:style".to_owned(),
                value: "private value is never included in the diagnostic".to_owned(),
            }],
        }];

        let failure = match write_v6_with_diagnostics(&project) {
            Ok(_) => unreachable!("namespaced generated run attributes are unsupported"),
            Err(failure) => failure,
        };
        assert_eq!(failure.stage, WriterStage::NewDocumentGeneration);
        assert_eq!(
            failure.error,
            SaveDiagnosticCode::NewParagraphNamespacedRunAttribute.error()
        );
        assert!(failure.error.to_string().starts_with("23:"));
        assert!(failure.structural_difference.is_none());
        assert!(failure.identity_difference.is_none());
    }

    #[test]
    fn v4_wo003_structural_difference_is_ordered_and_value_free() {
        let mut expected_root = preserved_element("Root", Vec::new());
        expected_root.children.push(PreservedNode::Element(preserved_element(
            "ExpectedChild",
            Vec::new(),
        )));
        let mut actual_root = preserved_element("Root", Vec::new());
        actual_root.children.push(PreservedNode::Element(preserved_element(
            "ActualChild",
            Vec::new(),
        )));
        let section = |root| PreservedSection {
            name: SourceName {
                namespace_uri: None,
                local_name: "Section".to_owned(),
                qualified_name: Some("Section".to_owned()),
            },
            kind: SectionKind::Other,
            ordinal: 0,
            children: vec![PreservedNode::Element(root)],
        };
        let difference = first_source_tree_difference(
            &[section(expected_root)],
            &[section(actual_root)],
        )
        .unwrap_or_else(|| unreachable!("different expanded element names have a first difference"));

        assert_eq!(difference.node_path, vec![0, 0, 0]);
        assert_eq!(difference.field, "element.local-name");
        assert_eq!(
            difference.expected.as_ref().map(|summary| summary.kind),
            Some(StructuralNodeKind::Element)
        );
        assert_eq!(
            difference.expected.as_ref().and_then(|summary| summary.local_name.as_deref()),
            Some("ExpectedChild")
        );
        assert_eq!(
            difference.actual.as_ref().and_then(|summary| summary.local_name.as_deref()),
            Some("ActualChild")
        );
        assert_eq!(
            difference.expected.as_ref().map(|summary| summary.attribute_count),
            Some(0)
        );
        assert!(difference.public_detail().contains("expected=ExpectedChild"));
        assert!(difference.public_detail().contains("field=element.local-name"));
    }

    #[test]
    fn v5_wo001_classifies_internal_xml_and_text_difference_fields() {
        fn section() -> PreservedSection {
            PreservedSection {
                name: SourceName {
                    namespace_uri: None,
                    local_name: "Section".into(),
                    qualified_name: Some("Section".into()),
                },
                kind: SectionKind::Other,
                ordinal: 0,
                children: vec![PreservedNode::Element(preserved_element("Root", Vec::new()))],
            }
        }
        fn diff(expected: &PreservedSection, actual: &PreservedSection) -> SourceTreeDifference {
            first_source_tree_difference(std::slice::from_ref(expected), std::slice::from_ref(actual))
                .unwrap_or_else(|| unreachable!("deliberately unequal trees have a categorized difference"))
        }
        fn assert_category(expected: PreservedSection, actual: PreservedSection, field: &str) {
            let result = diff(&expected, &actual);
            assert_eq!(result.field, field);
            let detail = result.public_detail();
            assert!(detail.starts_with("path=section[0]"));
            assert!(detail.len() <= 240);
            assert!(!detail.contains("secret"));
            assert!(!detail.contains("uuid"));
        }

        let expected = section();
        let mut actual = expected.clone();
        actual.name.local_name = "OtherSection".into();
        assert_category(expected, actual, "section.local-name");

        let expected = section();
        let mut actual = expected.clone();
        actual.kind = SectionKind::Content;
        assert_category(expected, actual, "section.kind");

        let expected = section();
        let mut actual = expected.clone();
        actual.ordinal = 1;
        assert_category(expected, actual, "section.ordinal");

        let expected = section();
        let mut actual = expected.clone();
        let PreservedNode::Element(root) = &mut actual.children[0] else { unreachable!() };
        root.name.namespace_uri = Some("urn:private-namespace".into());
        assert_category(expected, actual, "element.namespace");

        let expected = section();
        let mut actual = expected.clone();
        let PreservedNode::Element(root) = &mut actual.children[0] else { unreachable!() };
        root.name.qualified_name = Some("prefix:Root".into());
        assert_category(expected, actual, "element.qualified-name");

        let expected = section();
        let mut actual = expected.clone();
        let PreservedNode::Element(root) = &mut actual.children[0] else { unreachable!() };
        root.attributes.push(PreservedAttribute {
            name: SourceName { namespace_uri: None, local_name: "id".into(), qualified_name: Some("id".into()) },
            value: "uuid-secret".into(),
        });
        assert_category(expected, actual, "attribute.count-or-presence");

        let mut expected = section();
        let mut actual = section();
        let PreservedNode::Element(expected_root) = &mut expected.children[0] else { unreachable!() };
        let PreservedNode::Element(actual_root) = &mut actual.children[0] else { unreachable!() };
        expected_root.attributes.push(PreservedAttribute {
            name: SourceName { namespace_uri: None, local_name: "id".into(), qualified_name: Some("id".into()) },
            value: "uuid-secret-one".into(),
        });
        actual_root.attributes.push(PreservedAttribute {
            name: SourceName { namespace_uri: None, local_name: "id".into(), qualified_name: Some("id".into()) },
            value: "uuid-secret-two".into(),
        });
        assert_category(expected, actual, "attribute.value");

        let mut expected = section();
        let mut actual = section();
        let PreservedNode::Element(expected_root) = &mut expected.children[0] else { unreachable!() };
        let PreservedNode::Element(actual_root) = &mut actual.children[0] else { unreachable!() };
        for root in [expected_root, actual_root] {
            root.attributes = vec![
                PreservedAttribute {
                    name: SourceName { namespace_uri: None, local_name: "id".into(), qualified_name: Some("id".into()) },
                    value: "first-secret".into(),
                },
                PreservedAttribute {
                    name: SourceName { namespace_uri: None, local_name: "Length".into(), qualified_name: Some("Length".into()) },
                    value: "second-secret".into(),
                },
            ];
        }
        let PreservedNode::Element(actual_root) = &mut actual.children[0] else { unreachable!() };
        actual_root.attributes.swap(0, 1);
        assert_category(expected, actual, "attribute.name-or-order");

        let mut expected = section();
        let mut actual = section();
        let PreservedNode::Element(expected_root) = &mut expected.children[0] else { unreachable!() };
        let PreservedNode::Element(actual_root) = &mut actual.children[0] else { unreachable!() };
        expected_root.attributes.push(PreservedAttribute {
            name: SourceName { namespace_uri: None, local_name: "id".into(), qualified_name: Some("id".into()) },
            value: "secret-one".into(),
        });
        actual_root.attributes.push(PreservedAttribute {
            name: SourceName { namespace_uri: None, local_name: "other".into(), qualified_name: Some("other".into()) },
            value: "secret-two".into(),
        });
        assert_category(expected, actual, "attribute.name-or-order");

        let expected = section();
        let mut actual = expected.clone();
        let PreservedNode::Element(root) = &mut actual.children[0] else { unreachable!() };
        root.namespace_bindings.push(NamespaceBinding { prefix: "p".into(), namespace_uri: "urn:secret".into() });
        assert_category(expected, actual, "namespace-binding.count-or-presence");

        let mut expected = section();
        let mut actual = section();
        let PreservedNode::Element(expected_root) = &mut expected.children[0] else { unreachable!() };
        let PreservedNode::Element(actual_root) = &mut actual.children[0] else { unreachable!() };
        expected_root.namespace_bindings.push(NamespaceBinding { prefix: "p".into(), namespace_uri: "urn:one".into() });
        actual_root.namespace_bindings.push(NamespaceBinding { prefix: "p".into(), namespace_uri: "urn:two".into() });
        assert_category(expected, actual, "namespace-binding.namespace");

        let mut expected = section();
        let mut actual = section();
        let PreservedNode::Element(expected_root) = &mut expected.children[0] else { unreachable!() };
        let PreservedNode::Element(actual_root) = &mut actual.children[0] else { unreachable!() };
        expected_root.namespace_bindings.push(NamespaceBinding { prefix: "p".into(), namespace_uri: "urn:same".into() });
        actual_root.namespace_bindings.push(NamespaceBinding { prefix: "q".into(), namespace_uri: "urn:same".into() });
        assert_category(expected, actual, "namespace-binding.prefix-or-order");

        let expected = section();
        let mut actual = expected.clone();
        let PreservedNode::Element(root) = &mut actual.children[0] else { unreachable!() };
        root.source_type = Some("secret-source-type".into());
        assert_category(expected, actual, "source-type");

        let expected = section();
        let mut actual = expected.clone();
        let PreservedNode::Element(root) = &mut actual.children[0] else { unreachable!() };
        root.ownership = SourceOwnership::Attached(ForeignIdentity {
            domain: ForeignIdDomain::Paragraph,
            value: "uuid-secret".into(),
        });
        assert_category(expected, actual, "ownership");

        let expected = section();
        let mut actual = expected.clone();
        let PreservedNode::Element(root) = &mut actual.children[0] else { unreachable!() };
        root.dependency_confidence = DependencyConfidence::SomeReferencesMapped;
        assert_category(expected, actual, "dependency-confidence");

        let expected = section();
        let mut actual = expected.clone();
        let PreservedNode::Element(root) = &mut actual.children[0] else { unreachable!() };
        root.anchor = Some(SourceAnchorId::new("anchor-secret").unwrap_or_else(|_| unreachable!()));
        assert_category(expected, actual, "anchor");

        for (left, right, field) in [
            (PreservedNode::Text("secret-left".into()), PreservedNode::Text("secret-right".into()), "text.content"),
            (PreservedNode::CData("secret-left".into()), PreservedNode::CData("secret-right".into()), "cdata.content"),
            (PreservedNode::Comment("secret-left".into()), PreservedNode::Comment("secret-right".into()), "comment.content"),
            (PreservedNode::ProcessingInstruction { target: "left".into(), data: "secret".into() },
             PreservedNode::ProcessingInstruction { target: "right".into(), data: "secret".into() }, "processing-instruction.target"),
            (PreservedNode::ProcessingInstruction { target: "target".into(), data: "secret-left".into() },
             PreservedNode::ProcessingInstruction { target: "target".into(), data: "secret-right".into() }, "processing-instruction.data"),
        ] {
            let mut expected = section();
            let mut actual = section();
            let PreservedNode::Element(expected_root) = &mut expected.children[0] else { unreachable!() };
            let PreservedNode::Element(actual_root) = &mut actual.children[0] else { unreachable!() };
            expected_root.children.push(left);
            actual_root.children.push(right);
            assert_category(expected, actual, field);
        }

        let mut expected = section();
        let mut actual = section();
        let PreservedNode::Element(expected_root) = &mut expected.children[0] else { unreachable!() };
        let PreservedNode::Element(actual_root) = &mut actual.children[0] else { unreachable!() };
        expected_root.children.push(PreservedNode::Text("secret".into()));
        actual_root.children.push(PreservedNode::CData("secret".into()));
        assert_category(expected, actual, "node.kind-or-order");

        let mut expected = section();
        let actual = section();
        let PreservedNode::Element(root) = &mut expected.children[0] else { unreachable!() };
        root.children.push(PreservedNode::Text("secret".into()));
        assert_category(expected, actual, "child.presence");

        let mut expected = section();
        let mut actual = section();
        let PreservedNode::Element(expected_root) = &mut expected.children[0] else { unreachable!() };
        let PreservedNode::Element(actual_root) = &mut actual.children[0] else { unreachable!() };
        expected_root.children = vec![
            PreservedNode::Element(preserved_element("First", Vec::new())),
            PreservedNode::Element(preserved_element("Second", Vec::new())),
        ];
        actual_root.children = vec![
            PreservedNode::Element(preserved_element("Second", Vec::new())),
            PreservedNode::Element(preserved_element("First", Vec::new())),
        ];
        assert_category(expected, actual, "child.order");

        let mut expected = section();
        let mut actual = section();
        let PreservedNode::Element(expected_root) = &mut expected.children[0] else { unreachable!() };
        let PreservedNode::Element(actual_root) = &mut actual.children[0] else { unreachable!() };
        expected_root.children.push(PreservedNode::Text(String::new()));
        actual_root.children.push(PreservedNode::Text("   ".into()));
        let expected = normalize_sections(&[expected]);
        let actual = normalize_sections(&[actual]);
        let whitespace_difference = first_source_tree_difference(&expected, &actual)
            .unwrap_or_else(|| unreachable!("normalized empty and whitespace-only text differ"));
        assert_eq!(whitespace_difference.field, "child.presence");
        assert!(whitespace_difference.public_detail().contains("length=3,whitespace-only"));
        assert!(!whitespace_difference.public_detail().contains("   "));

        let equal = section();
        assert!(first_source_tree_difference(&[equal.clone()], &[equal]).is_none());
    }

    #[test]
    fn v4_wo003_identity_difference_reports_only_domain_and_direction() {
        let required = ForeignIdentity {
            domain: ForeignIdDomain::Paragraph,
            value: "sensitive-id".to_owned(),
        };
        let expected = BTreeSet::from([required]);
        let actual = BTreeSet::new();
        assert_eq!(
            first_identity_set_difference(&expected, &actual),
            Some(IdentitySetDifference {
                domain: IdentityDomainSummary::Paragraph,
                missing: true,
                unexpected: false,
            })
        );
    }

    #[test]
    fn v6_wo0011_documentref_datetime_validation_checks_shape_and_calendar() {
        for valid in [
            "20261001T120000",
            "20000229T000000",
            "20261231T235959",
        ] {
            assert!(valid_document_ref_datetime(valid));
        }
        for invalid in [
            "20260229T120000",
            "20261301T120000",
            "20261131T120000",
            "20261001T240000",
            "20261001T126000",
            "20261001T120060",
            "00000101T000000",
            "20261001T12000",
            "20261001T120000Z",
            "2026-10-01T120000",
        ] {
            assert!(!valid_document_ref_datetime(invalid));
        }
    }

    #[test]
    fn v6_wo0011_original_identity_gate_precedes_authorized_lineage_additions() {
        let source = br#"<FinalDraft DocumentType="Script" Version="6"><DocumentRef DateTime="20261001T120000" id="document-original"/><Content><Paragraph Type="Action" id="p1"><Text>identity gate</Text></Paragraph></Content></FinalDraft>"#;
        let mut project = read_v6(source)
            .unwrap_or_else(|_| unreachable!("lineage identity fixture is valid"))
            .project;
        project
            .preservation
            .as_mut()
            .unwrap_or_else(|| unreachable!("imported FDX has preservation"))
            .identities
            .push(
                ForeignIdentity::new(ForeignIdDomain::Other("Injected".to_owned()), "unrelated")
                    .unwrap_or_else(|_| unreachable!("injected identity is valid")),
            );
        let mut context = SaveContext::with_timestamp(
            SaveOperation::SaveAs,
            "20261103T101112".to_owned(),
        )
        .with_opened_timestamp(Some("20261001T120000".to_owned()))
        .with_test_lineage_ids(vec!["b8e1f570-3e16-4a90-8d6a-7e7f0d0a51aa".to_owned()]);

        assert_eq!(
            write_v6_for_save(&project, &mut context),
            Err(FdxError::SaveDiagnosticUnsupportedConstruct(4))
        );
    }

    #[test]
    fn v6_wo0011_ambiguous_documentref_structures_refuse_lineage_selection() {
        let duplicate_refs = br#"<FinalDraft DocumentType="Script" Version="6"><DocumentRef DateTime="20261001T120000" id="document-1"/><DocumentRef DateTime="20261002T120000" id="document-2"/><Content><Paragraph Type="Action" id="p1"><Text>ambiguity</Text></Paragraph></Content></FinalDraft>"#;
        let project = read_v6(duplicate_refs)
            .unwrap_or_else(|_| unreachable!("distinct DocumentRef identities are readable"))
            .project;
        let mut context = SaveContext::with_timestamp(
            SaveOperation::Save,
            "20261103T101112".to_owned(),
        );
        assert_eq!(
            write_v6_for_save(&project, &mut context),
            Err(FdxError::SaveDiagnosticUnsupportedConstruct(35))
        );

        let single_ref = br#"<FinalDraft DocumentType="Script" Version="6"><DocumentRef DateTime="20261001T120000" id="document-1"/><Content><Paragraph Type="Action" id="p1"><Text>ambiguity</Text></Paragraph></Content></FinalDraft>"#;
        let mut project = read_v6(single_ref)
            .unwrap_or_else(|_| unreachable!("single DocumentRef fixture is valid"))
            .project;
        let state = project
            .preservation
            .as_mut()
            .unwrap_or_else(|| unreachable!("imported FDX has preservation"));
        let document_ref = state
            .sections
            .iter_mut()
            .flat_map(|section| &mut section.children)
            .find_map(|node| match node {
                PreservedNode::Element(root) if root.name.local_name == FDX_ROOT => {
                    root.children.iter_mut().find_map(|child| match child {
                        PreservedNode::Element(element)
                            if element.name.local_name == "DocumentRef" => Some(element),
                        _ => None,
                    })
                }
                _ => None,
            })
            .unwrap_or_else(|| unreachable!("DocumentRef is in the retained root"));
        document_ref.attributes.push(PreservedAttribute {
            name: plain_source_name("DateTime"),
            value: "20261002T120000".to_owned(),
        });
        let mut context = SaveContext::with_timestamp(
            SaveOperation::SaveAs,
            "20261103T101112".to_owned(),
        )
        .with_opened_timestamp(Some("20261001T120000".to_owned()))
        .with_test_lineage_ids(vec!["b8e1f570-3e16-4a90-8d6a-7e7f0d0a51aa".to_owned()]);
        let error = write_v6_for_save(&project, &mut context)
            .err()
            .unwrap_or_else(|| unreachable!("duplicate DateTime must stop lineage editing"));
        assert_eq!(error, FdxError::SaveDiagnosticUnsupportedConstruct(36));
        let message = error.to_string();
        assert!(!message.contains("20261001T120000"));
        assert!(!message.contains("20261002T120000"));
    }
}

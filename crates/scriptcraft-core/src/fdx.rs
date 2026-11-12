//! Bounded FDX Version 6 codec for the evidenced element subset.
//!
//! The reader returns a semantic editing projection plus the complete parsed
//! source tree and original bytes. The writer reconciles supported element edits
//! with that tree, or emits a minimal new document for the evidenced role labels.
//! ScriptCraft-only persistent identity, metadata and geometry are not part of the
//! semantic model. Runtime inputs are not serialized; imported FDX source remains
//! authoritative for source settings. These APIs do not claim Final Draft
//! interoperability.

use crate::formatting::{formatting_for_role, formatting_in_native_order, role_index};
use crate::paragraph_id::{ParagraphIdAllocationError, ParagraphIdAllocator};
use crate::preservation::{
    BoardEntity, BoardEntityKind, BoardGraph, DependencyConfidence, ForeignIdDomain,
    ForeignIdentity, ForeignReference, NamespaceBinding, PreservationError, PreservationState,
    PreservedAttribute, PreservedElement, PreservedNode, PreservedSection, ReferencePolicy,
    SectionKind, SharedBytes, SharedVec, SourceAnchor, SourceAnchorId, SourceEncoding, SourceName,
    SourceOwnership, SourceSnapshot, is_user_paragraph_data_lookalike,
    is_user_paragraph_data_name,
};
pub use crate::text_state::TextState;
use crate::text_state::{self};
use crate::{
    ModelError, PrintVisibility, Project, Role, RunAttribute, ScreenplayElement, SourceFormat,
    SourceLayoutSettings, SourceProvenance, SourceSection, TextRun, TitlePageParagraph,
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
const XML_DECLARATION: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\" ?>\n";

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
    // Reserved diagnostic slot; candidate-preservation failure currently maps to
    // the output-preservation check, but this identifier must not be recycled.
    #[allow(dead_code)]
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
    /// Valid root DocumentRef DateTime observed during deserialization. This is
    /// source metadata, not the ephemeral session-open timestamp used for XRefs.
    /// Malformed or ambiguous values remain source-preserved but are not typed.
    pub document_ref_datetime: Option<String>,
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
        top: 75,
        width: 870,
        height: 874,
    };

    pub fn new(left: i32, top: i32, width: i32, height: i32) -> Option<Self> {
        (width > 0 && height > 0).then_some(Self {
            left,
            top,
            width,
            height,
        })
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
    pub(crate) session_open_timestamp: Option<String>,
    pub(crate) window_state: WindowState,
    pub(crate) text_state: TextState,
}

impl SaveContext {
    pub(crate) fn new(operation: SaveOperation) -> Self {
        Self {
            operation,
            timestamp: current_timestamp(),
            session_open_timestamp: None,
            window_state: WindowState::DEFAULT,
            text_state: TextState::DEFAULT,
        }
    }

    pub(crate) fn with_session_open_timestamp(mut self, timestamp: String) -> Self {
        self.session_open_timestamp = Some(timestamp);
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

    fn allocate_lineage_id(&mut self, reserved: &mut BTreeSet<String>) -> Result<String, FdxError> {
        const MAX_ATTEMPTS: usize = 16;
        for _ in 0..MAX_ATTEMPTS {
            let mut bytes = [0; 16];
            getrandom::fill(&mut bytes)
                .map_err(|_| SaveDiagnosticCode::LineageIdentityEntropyUnavailable.error())?;
            bytes[6] = (bytes[6] & 0x0f) | 0x40;
            bytes[8] = (bytes[8] & 0x3f) | 0x80;
            let candidate = uuid::Uuid::from_bytes(bytes).hyphenated().to_string();
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
    if year == 0 || !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
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

pub(crate) fn current_timestamp() -> String {
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
    let document_ref_datetime = raw_document_ref_datetime(root);
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
        dual_dialogue_groups: Vec::new(),
        dual_dialogue_paragraph_overrides: std::collections::BTreeMap::new(),
        direct_paragraph_overrides: std::collections::BTreeMap::new(),
        preservation: None,
        legacy_json_state: None,
        layout_identity: std::sync::Arc::new(()),
    };
    let mut source_layout_settings = read_source_layout_settings(root, root_namespace)?;
    let mut identities = Vec::new();
    let mut board_graph = BoardGraph::default();
    validate_image_payload_budgets(root)?;
    validate_user_paragraph_data_locations(root, root_namespace)?;
    collect_identities(root, &mut identities, &mut board_graph)?;
    let mut anchors = Vec::new();
    let mut next_id = 1usize;
    let mut next_anchor = 1usize;
    let mut root_owned = RawNode::Element(root.clone());
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
    let mut preservation = PreservationState {
        source: SourceSnapshot {
            format: SourceFormat::Fdx,
            original_bytes: SharedBytes::source(input.to_vec()),
            encoding: SourceEncoding::Utf8,
        },
        source_layout_settings,
        sections: vec![PreservedSection {
            name: document_name,
            kind: SectionKind::Other,
            ordinal: 0,
            children: std::mem::take(&mut preserved_children).into(),
        }]
        .into(),
        anchors,
        identities,
        references: Vec::new(),
        board_graph,
        managed_paragraph_index: None,
        source_identity_use_index: None,
        source_text_bytes: 0,
    };
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
    preservation.rebuild_managed_paragraph_index();
    project.preservation = Some(preservation);
    project.validate().map_err(map_model_error)?;
    let text_state = raw_root_text_state(&root_for_text_state, &project);
    Ok(FdxDocument {
        project,
        document_ref_datetime,
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

// WriterFailure intentionally carries bounded structural/identity differences
// to the save-diagnostics boundary; boxing it would churn the internal protocol.
#[allow(clippy::result_large_err)]
fn write_v6_with_diagnostics(project: &Project) -> Result<Vec<u8>, WriterFailure> {
    let mut context = SaveContext::new(SaveOperation::DirectSerialization);
    write_v6_with_context(project, &mut context)
}

#[allow(clippy::result_large_err)]
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
    .map_err(|error| WriterFailure {
        error,
        stage,
        structural_difference,
        identity_difference,
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
    if context.operation != SaveOperation::DirectSerialization
        && context
            .session_open_timestamp
            .as_deref()
            .is_none_or(|timestamp| !valid_document_ref_datetime(timestamp))
    {
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
            reconcile_root_text_state(&mut sections, project, context.text_state)?;
        }
        *stage = WriterStage::ManagedSourceReconciliation;
        reconcile_project_elements(&mut sections, project)?;
        let original_paragraph_ids = original_state
            .identities
            .iter()
            .filter(|identity| identity.domain == ForeignIdDomain::Paragraph)
            .map(|identity| identity.value.clone())
            .collect::<BTreeSet<_>>();
        let inserted_paragraph_ids =
            collect_inserted_managed_paragraph_ids(&sections, &original_paragraph_ids);
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
    if !same_semantic_title_paragraphs(&project.title_page, &written.project.title_page)
        || !same_semantic_elements(&project.elements, &written.project.elements)
        || !same_dual_dialogue_projection(project, &written.project)
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
            *identity_difference =
                first_identity_set_difference(&expected_identity_set, &actual_identity_set);
            return Err(SaveDiagnosticCode::WriteIdentitySetMismatch.error());
        }
        let actual_sections = &actual_state.sections;
        if normalize_sections(actual_sections) != normalize_sections(&expected_sections) {
            *stage = WriterStage::SourceTreeComparison;
            let expected = normalize_sections(&expected_sections);
            let actual = normalize_sections(actual_sections);
            let Some(difference) = first_source_tree_difference(&expected, &actual) else {
                return Err(SaveDiagnosticCode::WriteNormalizedSourceTreeMismatch.error());
            };
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
    actual
        .difference(expected)
        .next()
        .map(|identity| IdentitySetDifference {
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

pub(crate) fn same_dual_dialogue_projection(left: &Project, right: &Project) -> bool {
    fn member_positions(project: &Project, group: &crate::dual_dialogue::DualDialogueGroup) -> Option<Vec<usize>> {
        group
            .member_ids()
            .map(|id| project.elements.iter().position(|element| &element.id == id))
            .collect()
    }

    if left.dual_dialogue_groups.len() != right.dual_dialogue_groups.len() {
        return false;
    }
    for (left_group, right_group) in left
        .dual_dialogue_groups
        .iter()
        .zip(&right.dual_dialogue_groups)
    {
        if left_group.left.paragraphs.len() != right_group.left.paragraphs.len()
            || left_group.right.paragraphs.len() != right_group.right.paragraphs.len()
            || member_positions(left, left_group) != member_positions(right, right_group)
        {
            return false;
        }
        if left.preservation.is_some()
            && left_group.wrapper.outer_foreign_id != right_group.wrapper.outer_foreign_id
        {
            return false;
        }
        if left_group.wrapper.outer_foreign_id.is_some()
            && !left_group.wrapper.source.as_ref().zip(right_group.wrapper.source.as_ref())
                .is_some_and(|(expected, actual)| {
                    expected.format == actual.format
                        && expected.section == actual.section
                        && expected.foreign_id == actual.foreign_id
                        && expected.foreign_role == actual.foreign_role
                })
        {
            return false;
        }
        let left_overrides = left.dual_dialogue_paragraph_overrides.get(&left_group.id);
        let right_overrides = right.dual_dialogue_paragraph_overrides.get(&right_group.id);
        if left_overrides != right_overrides {
            return false;
        }
        if let Some(positions) = member_positions(left, left_group) {
            for position in positions {
                let Some(left_element) = left.elements.get(position) else {
                    return false;
                };
                let Some(right_element) = right.elements.get(position) else {
                    return false;
                };
                let expected_source = left_element.source_provenance.as_ref();
                let actual_source = right_element.source_provenance.as_ref();
                if let Some(expected_source) = expected_source {
                    if let Some(expected_foreign_id) = expected_source.foreign_id.as_deref() {
                        if !actual_source.is_some_and(|actual| {
                            actual.foreign_id.as_deref() == Some(expected_foreign_id)
                                && actual.section == expected_source.section
                                && actual.foreign_role == expected_source.foreign_role
                        }) {
                            return false;
                        }
                    }
                }
                if left.direct_paragraph_overrides.get(&left_element.id)
                    != right.direct_paragraph_overrides.get(&right_element.id)
                {
                    return false;
                }
            }
        }
    }
    left.elements.len() == right.elements.len()
}

fn same_semantic_title_paragraphs(
    left: &[TitlePageParagraph],
    right: &[TitlePageParagraph],
) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.role == right.role
                && left.text == right.text
                && left.formatting == right.formatting
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
        additions.push(
            ForeignIdentity::new(ForeignIdDomain::Document, document_ref_id.clone())
                .map_err(map_preservation_error)?,
        );
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
            .position(|node| {
                matches!(node,
                    PreservedNode::Element(element)
                        if element.name.local_name == "Content"
                            && element.name.namespace_uri == root.name.namespace_uri
                )
            })
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
    let xref_datetime = context
        .session_open_timestamp
        .as_deref()
        .ok_or(FdxError::InvalidValue)?
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

    let has_direct_xref = document_ref.children.iter().any(|node| {
        matches!(node,
            PreservedNode::Element(element)
                if element.name.local_name == "XRef"
                    && element.name.namespace_uri == document_ref.name.namespace_uri
        )
    });
    let should_append_xref = match context.operation {
        SaveOperation::DirectSerialization => false,
        SaveOperation::Save => !has_direct_xref,
        SaveOperation::SaveAs => true,
    };
    if should_append_xref {
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
        values.extend(
            element
                .attributes
                .iter()
                .filter(|&attribute| {
                    attribute.name.namespace_uri.is_none()
                        && attribute.name.local_name.eq_ignore_ascii_case("id")
                })
                .map(|attribute| attribute.value.clone()),
        );
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
        children: SharedVec::default(),
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
    let document_ref_preserves_space = document_ref.attributes.iter().any(|attribute| {
        attribute.name.namespace_uri.as_deref() == Some("http://www.w3.org/XML/1998/namespace")
            && attribute.name.local_name == "space"
            && attribute.value == "preserve"
    });
    if root_preserves_space || document_ref_preserves_space {
        return append_xref_without_formatting(document_ref, xref);
    }
    let format_safe = document_ref.children.iter().all(|node| match node {
        PreservedNode::Element(element) => {
            element.name.namespace_uri == document_ref.name.namespace_uri
        }
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
        .insert(index, PreservedNode::Text("\n    ".into()));
    document_ref
        .children
        .insert(index + 1, PreservedNode::Element(xref));
    if has_following_node && !following_is_whitespace {
        document_ref
            .children
            .insert(index + 2, PreservedNode::Text("\n    ".into()));
    } else if !has_following_node {
        document_ref
            .children
            .push(PreservedNode::Text("\n  ".into()));
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
    for (text, runs) in project
        .title_page
        .iter()
        .map(|element| (element.text.as_str(), element.runs.as_slice()))
        .chain(
            project
                .elements
                .iter()
                .map(|element| (element.text.as_str(), element.runs.as_slice())),
        )
    {
        event_count = event_count.saturating_add(1);
        text_bytes = text_bytes.saturating_add(text.len());
        let run_count = runs.len().max(usize::from(runs.is_empty()));
        event_count = event_count.saturating_add(run_count);
        if runs.is_empty() && !text.is_empty() {
            event_count = event_count.saturating_add(1);
        }
        for run in runs {
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
            section.children = normalize_source_tree(&section.children).into();
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
        let detail = format!(
            "path={path}; field={}; expected={expected}; actual={actual}",
            self.field
        );
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
        safe.push(
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.') {
                character
            } else {
                '_'
            },
        );
    }
    safe
}

fn safe_name_detail(value: Option<&str>) -> Option<String> {
    value.map(safe_xml_name)
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
            Some(
                if expected.namespace_uri.is_some() {
                    "present"
                } else {
                    "absent"
                }
                .into(),
            ),
            Some(
                if actual.namespace_uri.is_some() {
                    "present"
                } else {
                    "absent"
                }
                .into(),
            ),
        ));
    }
    if expected.qualified_name != actual.qualified_name {
        return Some((
            "element.qualified-name",
            Some(
                if expected.qualified_name.is_some() {
                    "present"
                } else {
                    "absent"
                }
                .into(),
            ),
            Some(
                if actual.qualified_name.is_some() {
                    "present"
                } else {
                    "absent"
                }
                .into(),
            ),
        ));
    }
    None
}

fn ownership_category(ownership: &SourceOwnership) -> &'static str {
    match ownership {
        SourceOwnership::Opaque => "opaque",
        SourceOwnership::Managed(_) => "managed",
        SourceOwnership::DualDialogueWrapper(_) => "dual_dialogue_wrapper",
        SourceOwnership::Attached(_) => "attached",
    }
}

fn anchor_category(anchor: &Option<SourceAnchorId>) -> &'static str {
    if anchor.is_some() {
        "present"
    } else {
        "absent"
    }
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
            return Some(difference(
                vec![section_index],
                "section.presence",
                None,
                actual_section
                    .map(|section| format!("kind={:?},ordinal={}", section.kind, section.ordinal)),
                None,
                actual_section
                    .and_then(|section| section.children.first())
                    .map(structural_node_summary),
            ));
        };
        let Some(actual_section) = actual_section else {
            return Some(difference(
                vec![section_index],
                "section.presence",
                Some(format!(
                    "kind={:?},ordinal={}",
                    expected_section.kind, expected_section.ordinal
                )),
                None,
                expected_section
                    .children
                    .first()
                    .map(structural_node_summary),
                None,
            ));
        };
        let section_path = vec![section_index];
        if let Some((field, left, right)) =
            source_name_difference(&expected_section.name, &actual_section.name)
        {
            let field = match field {
                "element.local-name" => "section.local-name",
                "element.namespace" => "section.namespace",
                _ => "section.qualified-name",
            };
            return Some(difference(
                section_path,
                field,
                left,
                right,
                expected_section
                    .children
                    .first()
                    .map(structural_node_summary),
                actual_section.children.first().map(structural_node_summary),
            ));
        }
        if expected_section.kind != actual_section.kind {
            return Some(difference(
                section_path,
                "section.kind",
                Some(format!("{:?}", expected_section.kind)),
                Some(format!("{:?}", actual_section.kind)),
                expected_section
                    .children
                    .first()
                    .map(structural_node_summary),
                actual_section.children.first().map(structural_node_summary),
            ));
        }
        if expected_section.ordinal != actual_section.ordinal {
            return Some(difference(
                section_path,
                "section.ordinal",
                Some(expected_section.ordinal.to_string()),
                Some(actual_section.ordinal.to_string()),
                expected_section
                    .children
                    .first()
                    .map(structural_node_summary),
                actual_section.children.first().map(structural_node_summary),
            ));
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
                |node| {
                    format!(
                        "{};sibling-count={}",
                        node_description(node),
                        expected.len()
                    )
                },
            );
            let actual_detail = actual_node.map_or_else(
                || format!("absent;sibling-count={}", actual.len()),
                |node| format!("{};sibling-count={}", node_description(node), actual.len()),
            );
            return Some(difference(
                child_path,
                "child.presence",
                Some(expected_detail),
                Some(actual_detail),
                expected_node.map(structural_node_summary),
                actual_node.map(structural_node_summary),
            ));
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
                if let Some((field, left, right)) =
                    source_name_difference(&expected_element.name, &actual_element.name)
                {
                    return Some(difference(
                        child_path,
                        field,
                        left,
                        right,
                        expected_summary,
                        actual_summary,
                    ));
                }
                let common_attributes = expected_element
                    .attributes
                    .len()
                    .min(actual_element.attributes.len());
                for attribute_index in 0..common_attributes {
                    let left = &expected_element.attributes[attribute_index];
                    let right = &actual_element.attributes[attribute_index];
                    if left.name != right.name {
                        return Some(difference(
                            child_path,
                            "attribute.name-or-order",
                            safe_name_detail(Some(&left.name.local_name)),
                            safe_name_detail(Some(&right.name.local_name)),
                            expected_summary,
                            actual_summary,
                        ));
                    }
                    if left.value != right.value {
                        return Some(difference(
                            child_path,
                            "attribute.value",
                            safe_name_detail(Some(&left.name.local_name)),
                            safe_name_detail(Some(&right.name.local_name)),
                            expected_summary,
                            actual_summary,
                        ));
                    }
                }
                if expected_element.attributes.len() != actual_element.attributes.len() {
                    let left = expected_element
                        .attributes
                        .get(common_attributes)
                        .map(|attribute| safe_xml_name(&attribute.name.local_name));
                    let right = actual_element
                        .attributes
                        .get(common_attributes)
                        .map(|attribute| safe_xml_name(&attribute.name.local_name));
                    return Some(difference(
                        child_path,
                        "attribute.count-or-presence",
                        left,
                        right,
                        expected_summary,
                        actual_summary,
                    ));
                }
                let common_bindings = expected_element
                    .namespace_bindings
                    .len()
                    .min(actual_element.namespace_bindings.len());
                for binding_index in 0..common_bindings {
                    let left = &expected_element.namespace_bindings[binding_index];
                    let right = &actual_element.namespace_bindings[binding_index];
                    if left.prefix != right.prefix {
                        return Some(difference(
                            child_path,
                            "namespace-binding.prefix-or-order",
                            Some(if left.prefix.is_empty() {
                                "default".into()
                            } else {
                                safe_xml_name(&left.prefix)
                            }),
                            Some(if right.prefix.is_empty() {
                                "default".into()
                            } else {
                                safe_xml_name(&right.prefix)
                            }),
                            expected_summary,
                            actual_summary,
                        ));
                    }
                    if left.namespace_uri != right.namespace_uri {
                        return Some(difference(
                            child_path,
                            "namespace-binding.namespace",
                            Some("present".into()),
                            Some("present".into()),
                            expected_summary,
                            actual_summary,
                        ));
                    }
                }
                if expected_element.namespace_bindings.len()
                    != actual_element.namespace_bindings.len()
                {
                    return Some(difference(
                        child_path,
                        "namespace-binding.count-or-presence",
                        expected_element
                            .namespace_bindings
                            .get(common_bindings)
                            .map(|binding| safe_xml_name(&binding.prefix)),
                        actual_element
                            .namespace_bindings
                            .get(common_bindings)
                            .map(|binding| safe_xml_name(&binding.prefix)),
                        expected_summary,
                        actual_summary,
                    ));
                }
                if expected_element.source_type != actual_element.source_type {
                    return Some(difference(
                        child_path,
                        "source-type",
                        Some(if expected_element.source_type.is_some() {
                            "present".into()
                        } else {
                            "absent".into()
                        }),
                        Some(if actual_element.source_type.is_some() {
                            "present".into()
                        } else {
                            "absent".into()
                        }),
                        expected_summary,
                        actual_summary,
                    ));
                }
                if expected_element.ownership != actual_element.ownership {
                    return Some(difference(
                        child_path,
                        "ownership",
                        Some(ownership_category(&expected_element.ownership).into()),
                        Some(ownership_category(&actual_element.ownership).into()),
                        expected_summary,
                        actual_summary,
                    ));
                }
                if expected_element.dependency_confidence != actual_element.dependency_confidence {
                    return Some(difference(
                        child_path,
                        "dependency-confidence",
                        Some(format!("{:?}", expected_element.dependency_confidence)),
                        Some(format!("{:?}", actual_element.dependency_confidence)),
                        expected_summary,
                        actual_summary,
                    ));
                }
                if expected_element.anchor != actual_element.anchor {
                    return Some(difference(
                        child_path,
                        "anchor",
                        Some(anchor_category(&expected_element.anchor).into()),
                        Some(anchor_category(&actual_element.anchor).into()),
                        expected_summary,
                        actual_summary,
                    ));
                }
                if let Some(difference) = first_node_sequence_difference(
                    &expected_element.children,
                    &actual_element.children,
                    child_path,
                ) {
                    return Some(difference);
                }
            }
            (PreservedNode::Text(expected_text), PreservedNode::Text(actual_text))
                if expected_text == actual_text => {}
            (PreservedNode::Text(expected_text), PreservedNode::Text(actual_text)) => {
                return Some(difference(
                    child_path,
                    "text.content",
                    Some(text_classification(expected_text)),
                    Some(text_classification(actual_text)),
                    Some(structural_node_summary(expected_node)),
                    Some(structural_node_summary(actual_node)),
                ));
            }
            (PreservedNode::CData(expected_text), PreservedNode::CData(actual_text))
                if expected_text == actual_text => {}
            (PreservedNode::CData(expected_text), PreservedNode::CData(actual_text)) => {
                return Some(difference(
                    child_path,
                    "cdata.content",
                    Some(text_classification(expected_text)),
                    Some(text_classification(actual_text)),
                    Some(structural_node_summary(expected_node)),
                    Some(structural_node_summary(actual_node)),
                ));
            }
            (PreservedNode::Comment(expected_text), PreservedNode::Comment(actual_text))
                if expected_text == actual_text => {}
            (PreservedNode::Comment(expected_text), PreservedNode::Comment(actual_text)) => {
                return Some(difference(
                    child_path,
                    "comment.content",
                    Some(text_classification(expected_text)),
                    Some(text_classification(actual_text)),
                    Some(structural_node_summary(expected_node)),
                    Some(structural_node_summary(actual_node)),
                ));
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
            (
                PreservedNode::ProcessingInstruction {
                    target: expected_target,
                    data: expected_data,
                },
                PreservedNode::ProcessingInstruction {
                    target: actual_target,
                    data: actual_data,
                },
            ) => {
                let (field, left, right) = if expected_target != actual_target {
                    (
                        "processing-instruction.target",
                        Some("present".into()),
                        Some("present".into()),
                    )
                } else {
                    (
                        "processing-instruction.data",
                        Some(text_classification(expected_data)),
                        Some(text_classification(actual_data)),
                    )
                };
                return Some(difference(
                    child_path,
                    field,
                    left,
                    right,
                    Some(structural_node_summary(expected_node)),
                    Some(structural_node_summary(actual_node)),
                ));
            }
            _ => {
                return Some(difference(
                    child_path,
                    "node.kind-or-order",
                    Some(node_description(expected_node)),
                    Some(node_description(actual_node)),
                    Some(structural_node_summary(expected_node)),
                    Some(structural_node_summary(actual_node)),
                ));
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
                element.children = normalize_text_nodes(&element.children).into();
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
    for child in std::mem::take(&mut element.children) {
        formatted.push(PreservedNode::Text(if root_children {
            format!("\n\n{child_indent}").into()
        } else {
            format!("\n{child_indent}").into()
        }));
        formatted.push(child);
    }
    formatted.push(PreservedNode::Text(format!("\n{closing_indent}").into()));
    element.children = formatted.into();
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
    fn visit(
        nodes: &[PreservedNode],
        original_ids: &BTreeSet<String>,
        found: &mut BTreeSet<String>,
    ) {
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
                    visit(std::slice::from_mut(child), root_preserve, inserted_ids);
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

fn format_content_insertions(children: &mut Vec<PreservedNode>, inserted_ids: &BTreeSet<String>) {
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
        if is_inserted || (saw_paragraph && previous_was_inserted) {
            formatted.push(PreservedNode::Text("\n    ".into()));
        } else {
            formatted.append(&mut pending_whitespace);
        }
        pending_whitespace.clear();
        formatted.push(child);
        saw_paragraph = true;
        previous_was_inserted = is_inserted;
    }
    if previous_was_inserted {
        formatted.push(PreservedNode::Text("\n  ".into()));
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
    let groups = project
        .dual_dialogue_groups
        .iter()
        .map(|group| (group.id.clone(), group))
        .collect::<BTreeMap<_, _>>();
    let mut body_order = Vec::new();
    let mut title_order = Vec::new();
    let mut group_order = Vec::new();
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
                &groups,
                false,
                &mut body_order,
                &mut title_order,
                &mut group_order,
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
        || group_order
            != project
                .dual_dialogue_groups
                .iter()
                .map(|group| group.id.clone())
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
    title: &BTreeMap<crate::StableId, &TitlePageParagraph>,
    groups: &BTreeMap<crate::StableId, &crate::dual_dialogue::DualDialogueGroup>,
    in_title_content: bool,
    body_order: &mut Vec<crate::StableId>,
    title_order: &mut Vec<crate::StableId>,
    group_order: &mut Vec<crate::StableId>,
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
    let current_title_content = current_section == SectionKind::TitlePage
        && (in_title_content
            || (depth == 2 && is_fdx_element && element.name.local_name == "Content"));
    if let SourceOwnership::DualDialogueWrapper(id) = element.ownership.clone() {
        if current_section != SectionKind::Content
            || depth != 2
            || !is_fdx_element
            || element.name.local_name != "Paragraph"
            || !seen.insert(id.clone())
        {
            return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error());
        }
        let group = groups
            .get(&id)
            .copied()
            .ok_or_else(|| SaveDiagnosticCode::ReconcileNodeSemanticElementMissing.error())?;
        reconcile_dual_dialogue_wrapper(
            element,
            group,
            root_namespace,
            body,
            body_order,
            seen,
        )?;
        group_order.push(id);
        return Ok(());
    }
    if let SourceOwnership::Managed(id) = element.ownership.clone() {
        if !seen.insert(id.clone()) {
            return Err(SaveDiagnosticCode::ReconcileNodeDuplicateManagedIdentity.error());
        }
        match current_section {
            SectionKind::Content => {
                if depth != 2 || !is_fdx_element || element.name.local_name != "Paragraph" {
                    return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error());
                }
                body_order.push(id.clone());
                let semantic = body
                    .get(&id)
                    .copied()
                    .ok_or_else(|| SaveDiagnosticCode::ReconcileNodeSemanticElementMissing.error())?;
                reconcile_managed_paragraph(element, semantic).map_err(|error| {
                    save_diagnostic_at(
                        error,
                        SaveDiagnosticCode::ReconcileManagedParagraphPropagation,
                    )
                })?;
            }
            SectionKind::TitlePage => {
                let exact_title_paragraph = is_fdx_element
                    && element.name.local_name == "Paragraph"
                    && ((depth == 2 && !in_title_content)
                        || (depth == 3 && in_title_content));
                if !exact_title_paragraph {
                    return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error());
                }
                title_order.push(id.clone());
                let semantic = title
                    .get(&id)
                    .copied()
                    .ok_or_else(|| SaveDiagnosticCode::ReconcileNodeSemanticElementMissing.error())?;
                reconcile_managed_title_paragraph(element, semantic).map_err(|error| {
                    save_diagnostic_at(
                        error,
                        SaveDiagnosticCode::ReconcileManagedParagraphPropagation,
                    )
                })?;
            }
            SectionKind::Other => {
                return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error())
            }
        }
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
            groups,
            current_title_content,
            body_order,
            title_order,
            group_order,
            seen,
            roots,
        )
        .map_err(|error| {
            save_diagnostic_at(error, SaveDiagnosticCode::ReconcileProjectChildPropagation)
        })?;
    }
    Ok(())
}

fn reconcile_dual_dialogue_wrapper(
    outer: &mut PreservedElement,
    group: &crate::dual_dialogue::DualDialogueGroup,
    root_namespace: Option<&str>,
    body: &BTreeMap<crate::StableId, &ScreenplayElement>,
    body_order: &mut Vec<crate::StableId>,
    seen: &mut BTreeSet<crate::StableId>,
) -> Result<(), FdxError> {
    if outer.source_type.as_deref() != Some("General")
        || outer.attributes.iter().filter(|attribute| {
            attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Type"
        }).count() != 1
        || !outer.attributes.iter().any(|attribute| {
            attribute.name.namespace_uri.is_none()
                && attribute.name.local_name == "Type"
                && attribute.value == "General"
        })
    {
        return Err(SaveDiagnosticCode::ReconcileParagraphTypeAttributeMismatch.error());
    }
    let outer_ids = outer.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none()
            && attribute.name.local_name.eq_ignore_ascii_case("id")
    }).collect::<Vec<_>>();
    if outer_ids.len() != 1
        || group.wrapper.outer_foreign_id.as_deref() != Some(outer_ids[0].value.as_str())
    {
        return Err(SaveDiagnosticCode::ReconcileNodeSemanticElementMissing.error());
    }
    let mut dual_index = None;
    let mut outer_attachment = false;
    for (index, child) in outer.children.iter().enumerate() {
        match child {
            PreservedNode::Element(element)
                if element.name.local_name == "DualDialogue"
                    && element.name.namespace_uri.as_deref() == root_namespace =>
            {
                if dual_index.replace(index).is_some() {
                    return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error());
                }
            }
            PreservedNode::Element(element)
                if is_user_paragraph_data_name(&element.name, &outer.name) && !outer_attachment =>
            {
                outer_attachment = true;
            }
            PreservedNode::Text(text) if text.chars().all(char::is_whitespace) => {}
            _ => return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error()),
        }
    }
    let dual = match dual_index.and_then(|index| outer.children.get_mut(index)) {
        Some(PreservedNode::Element(element)) => element,
        _ => return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error()),
    };
    let expected_members = group.member_ids().cloned().collect::<Vec<_>>();
    let mut actual_members = Vec::with_capacity(expected_members.len());
    let mut saw_spell_block = false;
    for child in &mut dual.children {
        match child {
            PreservedNode::Text(text) if text.chars().all(char::is_whitespace) => {}
            PreservedNode::Element(element)
                if element.name.local_name == "SpellCheckIgnoreLists"
                    && element.name.namespace_uri.as_deref() == root_namespace
                    && !saw_spell_block
                    && actual_members.is_empty() =>
            {
                if !is_supported_preserved_spell_block(element, root_namespace) {
                    return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error());
                }
                saw_spell_block = true;
            }
            PreservedNode::Element(element)
                if saw_spell_block
                    && element.name.local_name == "Paragraph"
                    && element.name.namespace_uri.as_deref() == root_namespace =>
            {
                let SourceOwnership::Managed(id) = element.ownership.clone() else {
                    return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error());
                };
                if expected_members.get(actual_members.len()) != Some(&id)
                    || !seen.insert(id.clone())
                {
                    return Err(SaveDiagnosticCode::ReconcileNodeDuplicateManagedIdentity.error());
                }
                let semantic = body
                    .get(&id)
                    .copied()
                    .ok_or_else(|| SaveDiagnosticCode::ReconcileNodeSemanticElementMissing.error())?;
                reconcile_managed_paragraph(element, semantic).map_err(|error| {
                    save_diagnostic_at(
                        error,
                        SaveDiagnosticCode::ReconcileManagedParagraphPropagation,
                    )
                })?;
                actual_members.push(id.clone());
                body_order.push(id.clone());
            }
            _ => return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error()),
        }
    }
    if !saw_spell_block || actual_members != expected_members {
        return Err(SaveDiagnosticCode::ReconcileProjectManagedOrderMismatch.error());
    }
    Ok(())
}

fn is_supported_preserved_spell_block(
    block: &PreservedElement,
    root_namespace: Option<&str>,
) -> bool {
    let mut lists = Vec::new();
    for child in &block.children {
        match child {
            PreservedNode::Text(text) if text.chars().all(char::is_whitespace) => {}
            PreservedNode::Element(element)
                if element.name.namespace_uri.as_deref() == root_namespace
                    && element.ownership == SourceOwnership::Opaque
                    && element.name.local_name == "IgnoredRanges" => lists.push("IgnoredRanges"),
            PreservedNode::Element(element)
                if element.name.namespace_uri.as_deref() == root_namespace
                    && element.ownership == SourceOwnership::Opaque
                    && element.name.local_name == "IgnoredWords" => lists.push("IgnoredWords"),
            _ => return false,
        }
    }
    lists == ["IgnoredRanges", "IgnoredWords"]
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

    reconcile_managed_text_runs(paragraph, &semantic.text, &semantic.runs)
}

fn reconcile_managed_title_paragraph(
    paragraph: &mut PreservedElement,
    semantic: &TitlePageParagraph,
) -> Result<(), FdxError> {
    let type_attributes = paragraph
        .attributes
        .iter()
        .filter(|attribute| {
            attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Type"
        })
        .collect::<Vec<_>>();
    match (semantic.role, paragraph.source_type.as_deref()) {
        (None, None) if type_attributes.is_empty() => {}
        (Some(role), Some(source_type))
            if type_attributes.len() == 1
                && type_attributes[0].value == source_type
                && role_from_fdx(source_type) == Some(role) => {}
        (None, _) => {
            return Err(SaveDiagnosticCode::ReconcileParagraphTypeAttributeMismatch.error())
        }
        (Some(_), _) => {
            return Err(SaveDiagnosticCode::ReconcileParagraphRoleMismatch.error())
        }
    }

    reconcile_managed_text_runs(paragraph, &semantic.text, &semantic.runs)
}

fn reconcile_managed_text_runs(
    paragraph: &mut PreservedElement,
    canonical_text: &str,
    semantic_runs: &[TextRun],
) -> Result<(), FdxError> {
    if canonical_text.is_empty() {
        if !semantic_runs.is_empty() {
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
            if text_element
                .children
                .iter()
                .any(|child| !matches!(child, PreservedNode::Text(_) | PreservedNode::CData(_)))
            {
                return Err(
                    SaveDiagnosticCode::ReconcileParagraphUnsupportedEmptyChild.error(),
                );
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

    let mut runs = semantic_runs
        .iter()
        .filter(|run| !run.text.is_empty())
        .cloned()
        .collect::<Vec<_>>();
    if runs.is_empty() {
        runs.push(TextRun {
            text: canonical_text.to_owned(),
            attributes: Vec::new(),
        });
    }
    if runs.iter().map(|run| run.text.as_str()).collect::<String>() != canonical_text {
        return Err(SaveDiagnosticCode::ReconcileParagraphRunProjectionMismatch.error());
    }

    let source_indexes = paragraph
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, child)| match child {
            PreservedNode::Element(element)
                if element.name.local_name == "Text"
                    && element.name.namespace_uri == paragraph.name.namespace_uri =>
            {
                Some(index)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut templates = Vec::with_capacity(source_indexes.len());
    for index in &source_indexes {
        let PreservedNode::Element(element) = &paragraph.children[*index] else {
            return Err(SaveDiagnosticCode::ReconcileParagraphRunProjectionMismatch.error());
        };
        if element
            .children
            .iter()
            .any(|child| !matches!(child, PreservedNode::Text(_) | PreservedNode::CData(_)))
        {
            return Err(SaveDiagnosticCode::ReconcileParagraphRunProjectionMismatch.error());
        }
        templates.push(element.clone());
    }

    let mut reconciled = Vec::with_capacity(runs.len());
    for (index, run) in runs.iter().enumerate() {
        validate_xml_chars(&run.text)?;
        let template = templates
            .get(index)
            .or_else(|| templates.last())
            .cloned()
            .unwrap_or_else(|| new_text_element_for_paragraph(paragraph));
        reconciled.push(PreservedNode::Element(reconcile_text_element(
            template, run,
        )?));
    }

    if source_indexes.is_empty() {
        paragraph.children.extend(reconciled);
        return Ok(());
    }

    let insertion_after = source_indexes.last().copied().unwrap_or_default() + 1;
    let mut replacements = reconciled.into_iter();
    let mut next_source = source_indexes.iter().copied().peekable();
    let mut children = Vec::with_capacity(
        paragraph
            .children
            .len()
            .saturating_add(runs.len().saturating_sub(source_indexes.len())),
    );
    for (index, child) in std::mem::take(&mut paragraph.children)
        .into_iter()
        .enumerate()
    {
        if next_source.peek().copied() == Some(index) {
            next_source.next();
            if let Some(replacement) = replacements.next() {
                children.push(replacement);
            }
        } else {
            children.push(child);
        }
        if index + 1 == insertion_after {
            children.extend(replacements.by_ref());
        }
    }
    paragraph.children = children.into();
    Ok(())
}

fn reconcile_text_element(
    mut element: PreservedElement,
    run: &TextRun,
) -> Result<PreservedElement, FdxError> {
    let mut attributes = Vec::with_capacity(run.attributes.len());
    for attribute in &run.attributes {
        validate_qname(&attribute.name)?;
        validate_xml_chars(&attribute.value)?;
        let name = if let Some(existing) = element.attributes.iter().find(|candidate| {
            candidate
                .name
                .qualified_name
                .as_deref()
                .unwrap_or(candidate.name.local_name.as_str())
                == attribute.name
        }) {
            existing.name.clone()
        } else if attribute.name.contains(':') {
            return Err(SaveDiagnosticCode::NewParagraphNamespacedRunAttribute.error());
        } else {
            SourceName {
                namespace_uri: None,
                local_name: attribute.name.clone(),
                qualified_name: Some(attribute.name.clone()),
            }
        };
        attributes.push(PreservedAttribute {
            name,
            value: attribute.value.clone(),
        });
    }
    element.attributes = attributes;

    let mut text_children = Vec::new();
    if let Some(first) = element.children.first() {
        match first {
            PreservedNode::CData(_) if run.text.contains("]]> ") => {
                return Err(FdxError::UnsupportedConstruct)
            }
            PreservedNode::CData(_) if run.text.contains("]]>") => {
                return Err(FdxError::UnsupportedConstruct)
            }
            PreservedNode::CData(_) => {
                text_children.push(PreservedNode::CData(run.text.clone().into()))
            }
            PreservedNode::Text(_) => text_children.push(PreservedNode::Text(run.text.clone().into())),
            _ => return Err(SaveDiagnosticCode::ReconcileParagraphUnexpectedFirstChild.error()),
        }
        for child in element.children.iter().skip(1) {
            match child {
                PreservedNode::CData(_) => text_children.push(PreservedNode::CData("".into())),
                PreservedNode::Text(_) => text_children.push(PreservedNode::Text("".into())),
                _ => return Err(SaveDiagnosticCode::ReconcileParagraphUnexpectedLaterChild.error()),
            }
        }
    } else if !run.text.is_empty() {
        text_children.push(PreservedNode::Text(run.text.clone().into()));
    }
    element.children = text_children.into();
    Ok(element)
}

fn new_text_element_for_paragraph(paragraph: &PreservedElement) -> PreservedElement {
    let prefix = paragraph
        .name
        .qualified_name
        .as_deref()
        .and_then(|name| name.split_once(':').map(|(prefix, _)| prefix));
    let qualified_name = prefix.map_or_else(|| "Text".to_owned(), |prefix| format!("{prefix}:Text"));
    PreservedElement {
        name: SourceName {
            namespace_uri: paragraph.name.namespace_uri.clone(),
            local_name: "Text".to_owned(),
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
    let xref_id = if matches!(
        context.operation,
        SaveOperation::Save | SaveOperation::SaveAs
    ) {
        Some(context.allocate_lineage_id(&mut lineage_ids)?)
    } else {
        None
    };
    let mut document_ref = preserved_element(
        "DocumentRef",
        vec![
            ("DateTime", context.timestamp.as_str()),
            ("id", &document_ref_id),
        ],
    );
    if let Some(xref_id) = xref_id {
        let xref_datetime = match context.operation {
            SaveOperation::DirectSerialization => context.timestamp.as_str(),
            SaveOperation::Save | SaveOperation::SaveAs => context
                .session_open_timestamp
                .as_deref()
                .ok_or(FdxError::InvalidValue)?,
        };
        document_ref
            .children
            .push(PreservedNode::Element(preserved_element(
                "XRef",
                vec![("DateTime", xref_datetime), ("id", &xref_id)],
            )));
    }
    root.children.push(PreservedNode::Element(document_ref));
    let mut content = preserved_element("Content", Vec::new());
    let mut reserved_paragraph_ids = lineage_ids;
    reserved_paragraph_ids.extend(project_source_paragraph_ids(project));
    let mut paragraph_ids = ParagraphIdAllocator::new(reserved_paragraph_ids);
    let mut index = 0usize;
    while let Some(element) = project.elements.get(index) {
        if let Some(group) = project
            .dual_dialogue_groups
            .iter()
            .find(|group| group.first_member() == &element.id)
        {
            let mut outer = preserved_element("Paragraph", vec![("Type", "General"), ("id", "")]);
            let outer_id = paragraph_ids.allocate().map_err(map_paragraph_id_error)?;
            set_unqualified_attribute(&mut outer, "id", &outer_id)?;
            if let Some(overrides) = project.dual_dialogue_paragraph_overrides.get(&group.id) {
                add_wrapper_direct_attributes(&mut outer, *overrides)?;
            }
            let mut dual = preserved_element("DualDialogue", Vec::new());
            let mut spell = preserved_element("SpellCheckIgnoreLists", Vec::new());
            spell.children.push(PreservedNode::Element(preserved_element(
                "IgnoredRanges",
                Vec::new(),
            )));
            spell.children.push(PreservedNode::Element(preserved_element(
                "IgnoredWords",
                Vec::new(),
            )));
            dual.children.push(PreservedNode::Element(spell));
            let members = group.member_ids().cloned().collect::<Vec<_>>();
            if members.is_empty() {
                return Err(SaveDiagnosticCode::NewDocumentHeadingNodeMismatch.error());
            }
            for (offset, member_id) in members.iter().enumerate() {
                let member = project
                    .elements
                    .get(index.saturating_add(offset))
                    .filter(|candidate| &candidate.id == member_id)
                    .ok_or_else(|| SaveDiagnosticCode::NewDocumentHeadingNodeMismatch.error())?;
                let overrides = project.direct_paragraph_overrides.get(member_id).copied();
                dual.children
                    .push(new_paragraph(member, overrides, &mut paragraph_ids)?);
            }
            outer.children.push(PreservedNode::Element(dual));
            content.children.push(PreservedNode::Element(outer));
            index = index
                .checked_add(members.len())
                .ok_or(FdxError::ResourceLimit)?;
            continue;
        }

        let direct_overrides = project.direct_paragraph_overrides.get(&element.id).copied();
        let mut paragraph = new_paragraph(element, direct_overrides, &mut paragraph_ids)?;
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
        index = index.checked_add(1).ok_or(FdxError::ResourceLimit)?;
    }
    root.children.push(PreservedNode::Element(content));
    root.children
        .push(PreservedNode::Element(window_state_element(
            &root.name,
            context.window_state,
        )));
    root.children
        .push(PreservedNode::Element(text_state_element(
            &root.name,
            project,
            context.text_state,
        )?));
    for formatting in formatting_in_native_order() {
        root.children
            .push(PreservedNode::Element(new_element_settings(formatting)));
    }
    let mut title_page = preserved_element("TitlePage", Vec::new());
    title_page.children.push(PreservedNode::Element(
        new_title_page_header_and_footer(&mut paragraph_ids)?,
    ));
    let mut title_content = preserved_element("Content", Vec::new());
    for element in &project.title_page {
        title_content
            .children
            .push(new_title_page_paragraph(element, &mut paragraph_ids)?);
    }
    title_page
        .children
        .push(PreservedNode::Element(title_content));
    title_page.children.push(PreservedNode::Element(preserved_element(
        "TextState",
        vec![("Scaling", "100"), ("Selection", "0,0"), ("ShowInvisibles", "No")],
    )));
    root.children.push(PreservedNode::Element(title_page));
    Ok(vec![PreservedNode::Element(root)])
}

fn project_source_paragraph_ids(project: &Project) -> BTreeSet<String> {
    fn collect_source_attributes(nodes: &[PreservedNode], reserved: &mut BTreeSet<String>) {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if element.name.local_name == "Paragraph" {
                if let Some(id) = element
                    .attributes
                    .iter()
                    .find(|attribute| attribute.name.local_name.eq_ignore_ascii_case("id"))
                {
                    reserved.insert(id.value.clone());
                }
            }
            collect_source_attributes(&element.children, reserved);
        }
    }

    let mut reserved = BTreeSet::new();
    for provenance in project
        .elements
        .iter()
        .filter_map(|element| element.source_provenance.as_ref())
        .chain(
            project
                .title_page
                .iter()
                .filter_map(|element| element.source_provenance.as_ref()),
        )
    {
        if let Some(id) = &provenance.foreign_id {
            reserved.insert(id.clone());
        }
    }
    reserved.extend(
        project
            .dual_dialogue_groups
            .iter()
            .filter_map(|group| group.wrapper.outer_foreign_id.clone()),
    );
    if let Some(preservation) = &project.preservation {
        reserved.extend(
            preservation
                .identities
                .iter()
                .filter(|identity| identity.domain == ForeignIdDomain::Paragraph)
                .map(|identity| identity.value.clone()),
        );
        for section in preservation.sections.iter() {
            collect_source_attributes(&section.children, &mut reserved);
        }
    }
    reserved
}

fn new_title_page_header_and_footer(
    paragraph_ids: &mut ParagraphIdAllocator,
) -> Result<PreservedElement, FdxError> {
    let mut header_and_footer = preserved_element(
        "HeaderAndFooter",
        vec![
            ("FooterFirstPage", "Yes"),
            ("FooterVisible", "No"),
            ("HeaderFirstPage", "No"),
            ("HeaderVisible", "Yes"),
            ("StartingPage", "1"),
        ],
    );

    let mut header = preserved_element("Header", Vec::new());
    let header_id = paragraph_ids.allocate().map_err(map_paragraph_id_error)?;
    let mut header_paragraph = preserved_element(
        "Paragraph",
        vec![
            ("Alignment", "Right"),
            ("FirstIndent", "0.00"),
            ("Leading", "Regular"),
            ("LeftIndent", "1.25"),
            ("OutlineLevel", "1"),
            ("RightIndent", "-1.25"),
            ("SpaceBefore", "0"),
            ("Spacing", "1"),
            ("StartsNewPage", "No"),
            ("id", &header_id),
        ],
    );
    header_paragraph
        .children
        .push(PreservedNode::Element(preserved_element(
            "DynamicLabel",
            vec![
                ("AdornmentStyle", "0"),
                ("Font", "Courier Final Draft"),
                ("RevisionID", "0"),
                ("Size", "12"),
                ("Style", ""),
                ("Type", "Page #"),
            ],
        )));
    let mut header_text = preserved_element(
        "Text",
        vec![
            ("AdornmentStyle", "0"),
            ("Font", "Courier Final Draft"),
            ("RevisionID", "0"),
            ("Size", "12"),
            ("Style", ""),
        ],
    );
    header_text.children.push(PreservedNode::Text(".".into()));
    header_paragraph
        .children
        .push(PreservedNode::Element(header_text));
    header
        .children
        .push(PreservedNode::Element(header_paragraph));

    let mut footer = preserved_element("Footer", Vec::new());
    let footer_id = paragraph_ids.allocate().map_err(map_paragraph_id_error)?;
    footer.children.push(PreservedNode::Element(preserved_element(
        "Paragraph",
        vec![
            ("Alignment", "Right"),
            ("FirstIndent", "0.00"),
            ("Leading", "Regular"),
            ("LeftIndent", "1.25"),
            ("OutlineLevel", "1"),
            ("RightIndent", "-1.25"),
            ("SpaceBefore", "0"),
            ("Spacing", "1"),
            ("StartsNewPage", "No"),
            ("id", &footer_id),
        ],
    )));
    header_and_footer
        .children
        .push(PreservedNode::Element(header));
    header_and_footer
        .children
        .push(PreservedNode::Element(footer));
    Ok(header_and_footer)
}

fn new_title_page_paragraph(
    semantic: &TitlePageParagraph,
    paragraph_ids: &mut ParagraphIdAllocator,
) -> Result<PreservedNode, FdxError> {
    let formatting = semantic.formatting;
    if formatting.leading_regular == Some(false) || formatting.spacing_single == Some(false) {
        return Err(FdxError::UnsupportedConstruct);
    }
    let mut attributes = Vec::with_capacity(12);
    if let Some(alignment) = formatting.alignment {
        let value = match alignment {
            crate::TitlePageAlignment::Left => "Left",
            crate::TitlePageAlignment::Right => "Right",
            crate::TitlePageAlignment::Center => "Center",
            crate::TitlePageAlignment::Justified => "Full",
        };
        attributes.push(("Alignment", value.to_owned()));
    }
    if let Some(value) = formatting.first_indent {
        attributes.push(("FirstIndent", format_inches_attribute(value)));
    }
    if formatting.leading_regular == Some(true) {
        attributes.push(("Leading", "Regular".to_owned()));
    }
    if let Some(value) = formatting.left_indent {
        attributes.push(("LeftIndent", format_inches_attribute(value)));
    }
    attributes.push(("OutlineLevel", "1".to_owned()));
    if let Some(value) = formatting.right_indent {
        attributes.push(("RightIndent", format_inches_attribute(value)));
    }
    if let Some(value) = formatting.space_before {
        attributes.push(("SpaceBefore", value.decimal_string()));
    }
    if formatting.spacing_single == Some(true) {
        attributes.push(("Spacing", "1".to_owned()));
    }
    if let Some(value) = formatting.starts_new_page {
        attributes.push((
            "StartsNewPage",
            if value { "Yes" } else { "No" }.to_owned(),
        ));
    }
    if let Some(role) = semantic.role {
        attributes.push(("Type", role_to_fdx(role).to_owned()));
    }
    validate_xml_chars(&semantic.text)?;
    let paragraph_id = paragraph_ids.allocate().map_err(map_paragraph_id_error)?;
    attributes.push(("id", paragraph_id));
    let mut paragraph = preserved_element_owned(
        "Paragraph",
        attributes
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    );
    let runs = if semantic.runs.is_empty() && !semantic.text.is_empty() {
        vec![TextRun {
            text: semantic.text.clone(),
            attributes: Vec::new(),
        }]
    } else {
        semantic.runs.clone()
    };
    if semantic.text.is_empty() {
        if !runs.is_empty() {
            return Err(SaveDiagnosticCode::NewParagraphEmptyRunsMismatch.error());
        }
        return Ok(PreservedNode::Element(paragraph));
    }
    if runs.iter().map(|run| run.text.as_str()).collect::<String>() != semantic.text {
        return Err(SaveDiagnosticCode::ReconcileParagraphRunProjectionMismatch.error());
    }
    for run in runs.into_iter().filter(|run| !run.text.is_empty()) {
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
        text.children.push(PreservedNode::Text(run.text.into()));
        paragraph.children.push(PreservedNode::Element(text));
    }
    Ok(PreservedNode::Element(paragraph))
}

fn format_inches_attribute(value: crate::FixedPoint) -> String {
    let hundredths = value.raw().div_euclid(72);
    let negative = hundredths < 0;
    let magnitude = hundredths.unsigned_abs();
    format!(
        "{}{}.{}",
        if negative { "-" } else { "" },
        magnitude / 100,
        magnitude % 100
    )
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
            (
                "Scaling".to_owned(),
                state
                    .scaling
                    .clamp(text_state::MIN_TEXT_SCALING, text_state::MAX_TEXT_SCALING)
                    .to_string(),
            ),
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
    let indices = root
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, node)| match node {
            PreservedNode::Element(element)
                if element.name.local_name == "TextState"
                    && element.name.namespace_uri == root.name.namespace_uri =>
            {
                Some(index)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let canonical = text_state_element(&root.name, project, state)?;
    if let Some(first) = indices.first().copied() {
        root.children[first] = PreservedNode::Element(canonical);
        for index in indices.into_iter().skip(1).rev() {
            root.children.remove(index);
        }
        return Ok(());
    }

    let window = root.children.iter().position(|node| {
        matches!(node,
            PreservedNode::Element(element)
                if element.name.local_name == "WindowState"
                    && element.name.namespace_uri == root.name.namespace_uri
        )
    });
    let page_layout = root.children.iter().position(|node| {
        matches!(node,
            PreservedNode::Element(element)
                if element.name.local_name == "PageLayout"
                    && element.name.namespace_uri == root.name.namespace_uri
        )
    });
    let settings = root.children.iter().position(|node| {
        matches!(node,
            PreservedNode::Element(element)
                if element.name.local_name == "ElementSettings"
                    && element.name.namespace_uri == root.name.namespace_uri
        )
    });
    let mut insertion = window
        .map(|index| index + 1)
        .or_else(|| page_layout.map(|index| index + 1))
        .or(settings)
        .unwrap_or(root.children.len());
    while insertion < root.children.len()
        && matches!(root.children.get(insertion), Some(PreservedNode::Text(text)) if text.trim().is_empty())
    {
        insertion += 1;
    }
    let needs_leading_space = insertion == 0
        || matches!(
            insertion
                .checked_sub(1)
                .and_then(|index| root.children.get(index)),
            Some(PreservedNode::Element(_))
        );
    if needs_leading_space {
        root.children
            .insert(insertion, PreservedNode::Text("\n\n  ".into()));
        insertion += 1;
    }
    root.children
        .insert(insertion, PreservedNode::Element(canonical));
    let after = insertion + 1;
    if after == root.children.len()
        || matches!(root.children.get(after), Some(PreservedNode::Element(_)))
    {
        root.children
            .insert(after, PreservedNode::Text("\n\n  ".into()));
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

    let page_layout = root.children.iter().position(|node| {
        matches!(node,
            PreservedNode::Element(element)
                if element.name.local_name == "PageLayout"
                    && element.name.namespace_uri == root.name.namespace_uri
        )
    });
    let settings = root.children.iter().position(|node| {
        matches!(node,
            PreservedNode::Element(element)
                if element.name.local_name == "ElementSettings"
                    && element.name.namespace_uri == root.name.namespace_uri
        )
    });
    let anchor = page_layout
        .map(|index| index + 1)
        .or(settings)
        .unwrap_or(root.children.len());
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
        || matches!(
            insertion
                .checked_sub(1)
                .and_then(|index| root.children.get(index)),
            Some(PreservedNode::Element(_))
        );
    if needs_leading_space {
        root.children
            .insert(insertion, PreservedNode::Text("\n\n  ".into()));
        insertion += 1;
    }
    root.children
        .insert(insertion, PreservedNode::Element(canonical));
    let after = insertion + 1;
    if after == root.children.len()
        || matches!(root.children.get(after), Some(PreservedNode::Element(_)))
    {
        root.children
            .insert(after, PreservedNode::Text("\n\n  ".into()));
    }
    Ok(())
}

fn new_element_settings(formatting: &crate::formatting::RoleFormatting) -> PreservedElement {
    let mut settings = preserved_element("ElementSettings", vec![("Type", formatting.fdx_label)]);
    let font = formatting.font;
    let mut font_attributes = vec![(
        "AdornmentStyle".to_owned(),
        font.adornment_style.to_string(),
    )];
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
                    if paragraph.starts_new_page {
                        "Yes"
                    } else {
                        "No"
                    }
                    .to_owned(),
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

    settings
        .children
        .push(PreservedNode::Element(preserved_element_owned(
            "Outline",
            vec![
                (
                    "CanHide".to_owned(),
                    if formatting.outline.can_hide {
                        "Yes"
                    } else {
                        "No"
                    }
                    .to_owned(),
                ),
                ("Level".to_owned(), formatting.outline.level.to_string()),
            ],
        )));
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
    direct_overrides: Option<crate::formatting::SourceParagraphOverrides>,
    paragraph_ids: &mut ParagraphIdAllocator,
) -> Result<PreservedNode, FdxError> {
    let mut paragraph = preserved_element("Paragraph", vec![("Type", role_to_fdx(semantic.role))]);
    if let Some(overrides) = direct_overrides {
        add_member_direct_attributes(&mut paragraph, overrides)?;
    }
    paragraph.attributes.push(PreservedAttribute {
        name: SourceName {
            namespace_uri: None,
            local_name: "id".to_owned(),
            qualified_name: Some("id".to_owned()),
        },
        value: String::new(),
    });
    let id_attribute = paragraph
        .attributes
        .iter_mut()
        .find(|attribute| attribute.name.namespace_uri.is_none() && attribute.name.local_name == "id")
        .ok_or(FdxError::UnsupportedConstruct)?;
    id_attribute.value = paragraph_ids.allocate().map_err(map_paragraph_id_error)?;
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
            text.children.push(PreservedNode::Text(run.text.into()));
        }
        paragraph.children.push(PreservedNode::Element(text));
    }
    Ok(PreservedNode::Element(paragraph))
}

fn add_member_direct_attributes(
    paragraph: &mut PreservedElement,
    overrides: crate::formatting::SourceParagraphOverrides,
) -> Result<(), FdxError> {
    let spec = overrides.paragraph;
    if let Some(alignment) = spec.alignment {
        push_unqualified_attribute(paragraph, "Alignment", alignment.fdx_value());
    }
    for (name, value) in [
        ("FirstIndent", spec.first_indent),
        ("LeftIndent", spec.left_indent),
        ("RightIndent", spec.right_indent),
    ] {
        if let Some(value) = value {
            push_unqualified_attribute(paragraph, name, &fixed_inches_string(value));
        }
    }
    if spec.leading_regular == Some(true) {
        push_unqualified_attribute(paragraph, "Leading", "Regular");
    }
    if let Some(value) = spec.space_before {
        push_unqualified_attribute(paragraph, "SpaceBefore", &value.decimal_string());
    }
    if spec.spacing_single == Some(true) {
        push_unqualified_attribute(paragraph, "Spacing", "1");
    }
    if let Some(value) = overrides.starts_new_page {
        push_unqualified_attribute(
            paragraph,
            "StartsNewPage",
            if value { "Yes" } else { "No" },
        );
    }
    Ok(())
}

fn add_wrapper_direct_attributes(
    paragraph: &mut PreservedElement,
    overrides: crate::formatting::SourceParagraphOverrides,
) -> Result<(), FdxError> {
    if let Some(value) = overrides.paragraph.left_indent {
        push_unqualified_attribute(paragraph, "LeftIndent", &fixed_inches_string(value));
    }
    if let Some(value) = overrides.paragraph.right_indent {
        push_unqualified_attribute(paragraph, "RightIndent", &fixed_inches_string(value));
    }
    if let Some(value) = overrides.paragraph.space_before {
        push_unqualified_attribute(paragraph, "SpaceBefore", &value.decimal_string());
    }
    if let Some(value) = overrides.starts_new_page {
        push_unqualified_attribute(
            paragraph,
            "StartsNewPage",
            if value { "Yes" } else { "No" },
        );
    }
    Ok(())
}

fn push_unqualified_attribute(element: &mut PreservedElement, name: &str, value: &str) {
    element.attributes.push(PreservedAttribute {
        name: SourceName {
            namespace_uri: None,
            local_name: name.to_owned(),
            qualified_name: Some(name.to_owned()),
        },
        value: value.to_owned(),
    });
}

fn set_unqualified_attribute(
    element: &mut PreservedElement,
    name: &str,
    value: &str,
) -> Result<(), FdxError> {
    let matching = element.attributes.iter_mut().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == name
    });
    let mut matches = matching;
    let Some(attribute) = matches.next() else {
        return Err(FdxError::UnsupportedConstruct);
    };
    if matches.next().is_some() {
        return Err(FdxError::UnsupportedConstruct);
    }
    attribute.value = value.to_owned();
    Ok(())
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
        children: SharedVec::default(),
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

fn raw_document_ref_datetime(root: &RawElement) -> Option<String> {
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
            && matches!(
                attribute.name.local_name.as_str(),
                "Left" | "Top" | "Width" | "Height"
            )
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
        RawNode::Element(element) if is_fdx_element(element, "TextState", namespace) => {
            Some(element)
        }
        _ => None,
    });
    let Some(block) = blocks.next() else {
        return TextState::DEFAULT;
    };
    if blocks.next().is_some() {
        return TextState::DEFAULT;
    }
    let mut scaling = None;
    let mut selection = None;
    let mut scaling_ambiguous = false;
    let mut selection_ambiguous = false;
    for attribute in &block.attributes {
        if attribute.name.namespace_uri.is_some() {
            continue;
        }
        match attribute.name.local_name.as_str() {
            "Scaling" => {
                if scaling.replace(attribute.value.as_str()).is_some() {
                    scaling_ambiguous = true;
                }
            }
            "Selection" if selection.replace(attribute.value.as_str()).is_some() => {
                selection_ambiguous = true;
            }
            _ => {}
        }
    }
    let scale = if scaling_ambiguous {
        100
    } else {
        scaling
            .and_then(|value| value.parse::<u64>().ok())
            .and_then(TextState::normalized_scaling)
            .unwrap_or(100)
    };
    let mapped = if selection_ambiguous {
        None
    } else {
        selection.and_then(|value| text_state::parse_flat_selection(project, value))
    };
    TextState {
        scaling: scale,
        selection: mapped.unwrap_or(TextState::DEFAULT.selection),
    }
}

fn validate_user_paragraph_data_locations(
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
            if !parent_accepts_attachment
                || !parent.is_some_and(|owner| {
                    is_user_paragraph_data_name(&element.name, &owner.name)
                })
            {
                return Err(FdxError::UnsupportedConstruct);
            }
            // The complete third-party subtree is opaque, including any
            // FDX-looking names nested within it.
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
            && (attribute(element, "Type").and_then(role_from_fdx).is_some()
                || (attribute(element, "Type") == Some("General")
                    && element.children.iter().any(|child| {
                        matches!(child, RawNode::Element(child)
                            if is_fdx_element(child, "DualDialogue", root_namespace))
                    })));
        let title_owner = current_section == SectionKind::TitlePage
            && is_paragraph
            && ((title_content && depth == 3)
                || (depth == 2 && attribute(element, "Type").and_then(role_from_fdx).is_some()));
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
            && matches!(
                attribute(element, "Type").and_then(role_from_fdx),
                Some(Role::Character | Role::Dialogue | Role::Parenthetical)
            );
        let accepts_attachment = body_owner || title_owner || dual_member_owner;

        let attachment_count = element
            .children
            .iter()
            .filter(|child| {
                matches!(child, RawNode::Element(child)
                    if is_user_paragraph_data_lookalike(&child.name))
            })
            .count();
        if attachment_count > 1 {
            return Err(FdxError::UnsupportedConstruct);
        }

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

fn preserve_misc_node(node: RawNode) -> Result<PreservedNode, FdxError> {
    match node {
        RawNode::Element(element) => preserve_opaque_element(element),
        RawNode::Text(text) => Ok(PreservedNode::Text(text.into())),
        RawNode::CData(text) => Ok(PreservedNode::CData(text.into())),
        RawNode::Comment(text) => Ok(PreservedNode::Comment(text.into())),
        RawNode::ProcessingInstruction { target, data } => {
            Ok(PreservedNode::ProcessingInstruction {
                target: target.into(),
                data: data.into(),
            })
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
        children: children.into(),
    }))
}

#[allow(clippy::too_many_arguments)]
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
    let title_content_paragraph = current_section == SectionKind::TitlePage
        && in_title_content
        && depth == 3
        && paragraph;
    let legacy_title_paragraph = current_section == SectionKind::TitlePage
        && depth == 2
        && paragraph
        && attribute(element, "Type").and_then(role_from_fdx).is_some();
    let body_paragraph = current_section == SectionKind::Content && depth == 2 && paragraph;

    // A DualDialogue container is only understood through the exact direct-body
    // General Paragraph shape below. Never let an unmatched occurrence drift
    // through generic opaque recursion or be projected as an ordinary General.
    if element.name.local_name == "DualDialogue" {
        return Err(FdxError::UnsupportedConstruct);
    }
    let has_dual_dialogue = element.children.iter().any(|child| {
        matches!(child, RawNode::Element(child) if child.name.local_name == "DualDialogue")
    });
    if has_dual_dialogue {
        if !body_paragraph || attribute(element, "Type") != Some("General") {
            return Err(FdxError::UnsupportedConstruct);
        }
        let preserved = preserve_dual_dialogue_wrapper(
            element,
            root_ns,
            project,
            source_layout_settings,
            anchors,
            next_id,
            next_anchor,
        )?;
        return Ok(PreservedNode::Element(preserved));
    }

    let managed = if title_content_paragraph || legacy_title_paragraph {
        Some(ImportedSemanticParagraph::Title(make_title_page_paragraph(
            element,
            u32::try_from(project.title_page.len()).map_err(|_| FdxError::ResourceLimit)?,
            next_id,
        )?))
    } else if body_paragraph {
        if let Some(role) = attribute(element, "Type").and_then(role_from_fdx) {
            Some(ImportedSemanticParagraph::Body(make_semantic_element(
                element,
                role,
                current_section,
                next_id,
            )?))
        } else {
            None
        }
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
            if let Some(overrides) = direct_paragraph_overrides(element) {
                source_layout_settings
                    .direct_paragraph_overrides
                    .insert(body.id.clone(), overrides);
            }
        }
        anchors.push(SourceAnchor {
            id: anchor_id,
            target: crate::preservation::AnchorTarget::ProjectElement(semantic.id().clone()),
            relation: crate::preservation::AnchorRelation::Represents,
        });
        match semantic {
            ImportedSemanticParagraph::Body(body) => project.elements.push(body),
            ImportedSemanticParagraph::Title(title) => project.title_page.push(title),
        }
    }
    Ok(PreservedNode::Element(preserved))
}

#[allow(clippy::too_many_arguments)]
fn preserve_dual_dialogue_wrapper(
    outer: &RawElement,
    root_ns: Option<&str>,
    project: &mut Project,
    source_layout_settings: &mut SourceLayoutSettings,
    anchors: &mut Vec<SourceAnchor>,
    next_id: &mut usize,
    next_anchor: &mut usize,
) -> Result<PreservedElement, FdxError> {
    let wrapper_ordinal = next_id.checked_sub(1).ok_or(FdxError::ResourceLimit)?;
    let member_start = project.elements.len();
    let mut staged_members = Vec::new();
    let mut staged_anchors = Vec::new();
    let mut staged_member_overrides = Vec::new();
    let duals = outer
        .children
        .iter()
        .filter_map(|child| match child {
            RawNode::Element(child) if child.name.local_name == "DualDialogue" => Some(child),
            _ => None,
        })
        .collect::<Vec<_>>();
    let [dual] = duals.as_slice() else {
        return Err(FdxError::UnsupportedConstruct);
    };
    let dual = *dual;
    if !is_fdx_element(outer, "Paragraph", root_ns)
        || !is_fdx_element(dual, "DualDialogue", root_ns)
        || attribute(outer, "Type") != Some("General")
    {
        return Err(FdxError::UnsupportedConstruct);
    }

    // The wrapper's direct mixed content is deliberately stricter than generic
    // preservation: only indentation whitespace, one opaque attachment, and its
    // one DualDialogue child are accepted.
    let mut outer_attachment = false;
    for child in &outer.children {
        match child {
            RawNode::Element(child) if child.name.local_name == "DualDialogue" => {}
            RawNode::Element(child)
                if is_user_paragraph_data_name(&child.name, &outer.name) && !outer_attachment =>
            {
                outer_attachment = true;
            }
            RawNode::Text(value) if value.chars().all(char::is_whitespace) => {}
            _ => return Err(FdxError::UnsupportedConstruct),
        }
    }

    let mut spell_blocks = 0usize;
    let mut seen_member = false;
    let mut roles = Vec::new();
    let mut dual_children = Vec::with_capacity(dual.children.len());
    for child in &dual.children {
        match child {
            RawNode::Text(value) if value.chars().all(char::is_whitespace) => {
                dual_children.push(preserve_misc_node(child.clone())?);
            }
            RawNode::Element(element) if element.name.local_name == "SpellCheckIgnoreLists" => {
                if seen_member || spell_blocks != 0 {
                    return Err(FdxError::UnsupportedConstruct);
                }
                validate_nested_spell_check(element, root_ns)?;
                spell_blocks += 1;
                dual_children.push(preserve_opaque_element(element.clone())?);
            }
            RawNode::Element(element) if element.name.local_name == "Paragraph" => {
                seen_member = true;
                if !is_fdx_element(element, "Paragraph", root_ns) {
                    return Err(FdxError::UnsupportedConstruct);
                }
                let role = attribute(element, "Type")
                    .and_then(role_from_fdx)
                    .ok_or(FdxError::UnsupportedConstruct)?;
                if !matches!(
                    role,
                    Role::Character | Role::Dialogue | Role::Parenthetical
                ) {
                    return Err(FdxError::UnsupportedConstruct);
                }
                validate_nested_member_topology(element, root_ns, role)?;
                let semantic = make_semantic_element(
                    element,
                    role,
                    SectionKind::Content,
                    next_id,
                )?;
                let member_id = semantic.id.clone();
                let anchor_id = next_source_anchor(next_anchor)?;
                let member_overrides = direct_paragraph_overrides(element);
                if let Some(overrides) = member_overrides {
                    staged_member_overrides.push((member_id.clone(), overrides));
                }
                staged_anchors.push(SourceAnchor {
                    id: anchor_id.clone(),
                    target: crate::preservation::AnchorTarget::ProjectElement(
                        member_id.clone(),
                    ),
                    relation: crate::preservation::AnchorRelation::Represents,
                });
                roles.push(role);
                staged_members.push(semantic);
                dual_children.push(PreservedNode::Element(
                    preserved_managed_nested_paragraph(element, member_id, anchor_id)?,
                ));
            }
            _ => return Err(FdxError::UnsupportedConstruct),
        }
    }
    if spell_blocks != 1 {
        return Err(FdxError::UnsupportedConstruct);
    }
    let right_cue = roles
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, role)| (*role == Role::Character).then_some(index))
        .ok_or(FdxError::UnsupportedConstruct)?;
    if roles.first() != Some(&Role::Character)
        || roles[1..right_cue]
            .iter()
            .any(|role| !matches!(role, Role::Dialogue | Role::Parenthetical))
        || roles[right_cue + 1..]
            .iter()
            .any(|role| !matches!(role, Role::Dialogue | Role::Parenthetical))
        || roles[right_cue + 1..]
            .iter()
            .any(|role| *role == Role::Character)
    {
        return Err(FdxError::UnsupportedConstruct);
    }

    // The semantic members were appended in source order. Their IDs are also
    // the group membership authority; the outer General remains invisible.
    if staged_members.len() != roles.len() {
        return Err(FdxError::UnsupportedConstruct);
    }
    let members = staged_members.as_slice();
    let group_id = crate::StableId::new(format!(
        "fdx-dual-group-{}",
        project.dual_dialogue_groups.len().checked_add(1).ok_or(FdxError::ResourceLimit)?
    ))
    .map_err(map_model_error)?;
    let left = crate::dual_dialogue::DualDialogueSide {
        character: members[0].id.clone(),
        paragraphs: members[1..right_cue]
            .iter()
            .map(|member| member.id.clone())
            .collect(),
    };
    let right = crate::dual_dialogue::DualDialogueSide {
        character: members[right_cue].id.clone(),
        paragraphs: members[right_cue + 1..]
            .iter()
            .map(|member| member.id.clone())
            .collect(),
    };

    let wrapper_anchor = next_source_anchor(next_anchor)?;
    let outer_id = Some(
        identity_attribute(outer)
            .ok_or(FdxError::UnsupportedConstruct)?
            .to_owned(),
    );
    let source = SourceProvenance {
        format: SourceFormat::Fdx,
        section: SourceSection::Content,
        ordinal: u32::try_from(wrapper_ordinal).map_err(|_| FdxError::ResourceLimit)?,
        foreign_id: outer_id.clone(),
        foreign_role: attribute(outer, "Type").map(str::to_owned),
    };
    let wrapper_overrides = dual_wrapper_direct_overrides(outer)?;
    if let Some(overrides) = &wrapper_overrides {
        staged_member_overrides.push((group_id.clone(), overrides.clone()));
    }
    staged_anchors.push(SourceAnchor {
        id: wrapper_anchor.clone(),
        target: crate::preservation::AnchorTarget::DualDialogueWrapper(group_id.clone()),
        relation: crate::preservation::AnchorRelation::Represents,
    });
    let group = crate::dual_dialogue::DualDialogueGroup {
        id: group_id.clone(),
        left,
        right,
        wrapper: crate::dual_dialogue::DualDialogueWrapperProvenance {
            outer_foreign_id: outer_id,
            source: Some(source),
            preservation_anchor: Some(wrapper_anchor.clone()),
        },
    };

    let dual_preserved = PreservedElement {
        name: dual.name.clone(),
        attributes: preserved_attributes(dual),
        namespace_bindings: dual.namespace_bindings.clone(),
        source_type: attribute(dual, "Type").map(str::to_owned),
        ownership: SourceOwnership::Opaque,
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: None,
        children: dual_children.into(),
    };
    let wrapper_children = outer
        .children
        .iter()
        .map(|child| match child {
            RawNode::Element(child) if child.name.local_name == "DualDialogue" => {
                Ok(PreservedNode::Element(dual_preserved.clone()))
            }
            RawNode::Element(child) if is_user_paragraph_data_name(&child.name, &outer.name) => {
                Ok(preserve_opaque_element(child.clone())?)
            }
            RawNode::Text(value) if value.chars().all(char::is_whitespace) => {
                preserve_misc_node(child.clone())
            }
            _ => Err(FdxError::UnsupportedConstruct),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let preserved_outer = PreservedElement {
        name: outer.name.clone(),
        attributes: preserved_attributes(outer),
        namespace_bindings: outer.namespace_bindings.clone(),
        source_type: attribute(outer, "Type").map(str::to_owned),
        ownership: SourceOwnership::DualDialogueWrapper(group_id.clone()),
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: Some(wrapper_anchor),
        children: wrapper_children.into(),
    };
    project.elements.extend(staged_members);
    project.dual_dialogue_groups.push(group);
    for (id, overrides) in staged_member_overrides {
        source_layout_settings
            .direct_paragraph_overrides
            .insert(id, overrides);
    }
    anchors.extend(staged_anchors);
    Ok(preserved_outer)
}

fn next_source_anchor(next_anchor: &mut usize) -> Result<SourceAnchorId, FdxError> {
    let value = format!("fdx-anchor-{}", *next_anchor);
    *next_anchor = next_anchor.checked_add(1).ok_or(FdxError::ResourceLimit)?;
    SourceAnchorId::new(value).map_err(|_| FdxError::InvalidValue)
}

fn preserved_attributes(element: &RawElement) -> Vec<PreservedAttribute> {
    element
        .attributes
        .iter()
        .map(|attribute| PreservedAttribute {
            name: attribute.name.clone(),
            value: attribute.value.clone(),
        })
        .collect()
}

fn preserved_managed_nested_paragraph(
    element: &RawElement,
    id: crate::StableId,
    anchor: SourceAnchorId,
) -> Result<PreservedElement, FdxError> {
    Ok(PreservedElement {
        name: element.name.clone(),
        attributes: preserved_attributes(element),
        namespace_bindings: element.namespace_bindings.clone(),
        source_type: attribute(element, "Type").map(str::to_owned),
        ownership: SourceOwnership::Managed(id),
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: Some(anchor),
        children: element
            .children
            .iter()
            .cloned()
            .map(preserve_misc_node)
            .collect::<Result<Vec<_>, _>>()?
            .into(),
    })
}

fn validate_nested_spell_check(element: &RawElement, root_ns: Option<&str>) -> Result<(), FdxError> {
    if !is_fdx_element(element, "SpellCheckIgnoreLists", root_ns) {
        return Err(FdxError::UnsupportedConstruct);
    }
    reject_nested_dual_dialogue(element)?;
    let mut ranges = 0usize;
    let mut words = 0usize;
    for child in &element.children {
        match child {
            RawNode::Text(value) if value.chars().all(char::is_whitespace) => {}
            RawNode::Element(child) if is_fdx_element(child, "IgnoredRanges", root_ns) => {
                ranges += 1;
            }
            RawNode::Element(child) if is_fdx_element(child, "IgnoredWords", root_ns) => {
                words += 1;
            }
            _ => return Err(FdxError::UnsupportedConstruct),
        }
    }
    if ranges != 1 || words != 1 {
        return Err(FdxError::UnsupportedConstruct);
    }
    Ok(())
}

fn reject_nested_dual_dialogue(element: &RawElement) -> Result<(), FdxError> {
    for child in &element.children {
        if let RawNode::Element(child) = child {
            if child.name.local_name == "DualDialogue" {
                return Err(FdxError::UnsupportedConstruct);
            }
            reject_nested_dual_dialogue(child)?;
        }
    }
    Ok(())
}

fn validate_nested_member_topology(
    element: &RawElement,
    root_ns: Option<&str>,
    role: Role,
) -> Result<(), FdxError> {
    let mut saw_alts = false;
    let mut saw_attachment = false;
    for child in &element.children {
        match child {
            RawNode::Text(value) if value.chars().all(char::is_whitespace) => {}
            RawNode::Element(child)
                if is_fdx_element(child, "Text", root_ns)
                    && child.children.iter().all(|inline| {
                        matches!(inline, RawNode::Text(_) | RawNode::CData(_))
                    }) => {}
            RawNode::Element(child)
                if role == Role::SceneHeading && is_fdx_element(child, "SceneProperties", root_ns) => {}
            RawNode::Element(child)
                if role == Role::Dialogue && is_fdx_element(child, "Alts", root_ns) =>
            {
                if saw_alts {
                    return Err(FdxError::UnsupportedConstruct);
                }
                saw_alts = true;
            }
            RawNode::Element(child)
                if is_user_paragraph_data_name(&child.name, &element.name) && !saw_attachment =>
            {
                saw_attachment = true;
            }
            _ => return Err(FdxError::UnsupportedConstruct),
        }
    }
    Ok(())
}

fn dual_wrapper_direct_overrides(
    element: &RawElement,
) -> Result<Option<crate::formatting::SourceParagraphOverrides>, FdxError> {
    let mut overrides = crate::formatting::SourceParagraphOverrides::default();
    let paragraph = &mut overrides.paragraph;
    if let Some(value) = attribute(element, "LeftIndent") {
        paragraph.left_indent = Some(bounded_inches(value, false).ok_or(FdxError::InvalidValue)?);
    }
    if let Some(value) = attribute(element, "RightIndent") {
        paragraph.right_indent = Some(bounded_inches(value, false).ok_or(FdxError::InvalidValue)?);
    }
    if let Some(value) = attribute(element, "SpaceBefore") {
        paragraph.space_before = Some(
            crate::FixedPoint::parse_points(value)
                .filter(|value| (0..=79_200).contains(&value.raw()))
                .ok_or(FdxError::InvalidValue)?,
        );
    }
    if let Some(value) = attribute(element, "StartsNewPage") {
        overrides.starts_new_page = Some(match value {
            "Yes" => true,
            "No" => false,
            _ => return Err(FdxError::InvalidValue),
        });
    }
    Ok((paragraph.left_indent.is_some()
        || paragraph.right_indent.is_some()
        || paragraph.space_before.is_some()
        || overrides.starts_new_page.is_some())
    .then_some(overrides))
}

enum ImportedSemanticParagraph {
    Body(ScreenplayElement),
    Title(TitlePageParagraph),
}

impl ImportedSemanticParagraph {
    fn id(&self) -> &crate::StableId {
        match self {
            Self::Body(element) => &element.id,
            Self::Title(paragraph) => &paragraph.id,
        }
    }
}

fn direct_paragraph_overrides(
    element: &RawElement,
) -> Option<crate::formatting::SourceParagraphOverrides> {
    let mut overrides = crate::formatting::SourceParagraphOverrides::default();
    let paragraph = &mut overrides.paragraph;
    paragraph.alignment = attribute(element, "Alignment")
        .and_then(crate::ParagraphAlignment::from_fdx);
    paragraph.first_indent =
        attribute(element, "FirstIndent").and_then(|value| bounded_inches(value, true));
    paragraph.left_indent =
        attribute(element, "LeftIndent").and_then(|value| bounded_inches(value, false));
    paragraph.right_indent =
        attribute(element, "RightIndent").and_then(|value| bounded_inches(value, false));
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
    let runs = semantic_text_runs(
        element,
        section == SectionKind::Content && role == Role::SceneHeading,
        section == SectionKind::Content && role == Role::Dialogue,
    )?;
    ScreenplayElement::with_runs(id, role, runs, Some(source_provenance)).map_err(map_model_error)
}

fn make_title_page_paragraph(
    element: &RawElement,
    ordinal: u32,
    next_id: &mut usize,
) -> Result<TitlePageParagraph, FdxError> {
    let id = format!("fdx-element-{}", *next_id);
    *next_id = next_id.checked_add(1).ok_or(FdxError::ResourceLimit)?;
    let source_provenance = SourceProvenance {
        format: SourceFormat::Fdx,
        section: SourceSection::TitlePage,
        ordinal,
        foreign_id: identity_attribute(element).map(str::to_owned),
        foreign_role: attribute(element, "Type").map(str::to_owned),
    };
    let role = attribute(element, "Type").and_then(role_from_fdx);
    TitlePageParagraph::with_runs(
        id,
        role,
        semantic_text_runs(element, false, false)?,
        Some(source_provenance),
        direct_title_page_formatting(element),
    )
    .map_err(map_model_error)
}

fn direct_title_page_formatting(element: &RawElement) -> crate::TitlePageParagraphFormatting {
    use crate::{TitlePageAlignment, TitlePageParagraphFormatting};

    TitlePageParagraphFormatting {
        alignment: attribute(element, "Alignment").and_then(|alignment| match alignment {
            "Left" => Some(TitlePageAlignment::Left),
            "Center" => Some(TitlePageAlignment::Center),
            "Right" => Some(TitlePageAlignment::Right),
            "Full" => Some(TitlePageAlignment::Justified),
            _ => None,
        }),
        first_indent: attribute(element, "FirstIndent")
            .and_then(|value| bounded_inches(value, true)),
        left_indent: attribute(element, "LeftIndent")
            .and_then(|value| bounded_inches(value, false)),
        right_indent: attribute(element, "RightIndent")
            .and_then(|value| bounded_inches(value, false)),
        leading_regular: (attribute(element, "Leading") == Some("Regular")).then_some(true),
        space_before: attribute(element, "SpaceBefore")
            .and_then(crate::FixedPoint::parse_points)
            .filter(|value| (0..=79_200).contains(&value.raw())),
        spacing_single: (attribute(element, "Spacing") == Some("1")).then_some(true),
        starts_new_page: attribute(element, "StartsNewPage").and_then(|value| match value {
            "Yes" => Some(true),
            "No" => Some(false),
            _ => None,
        }),
    }
}

fn semantic_text_runs(
    element: &RawElement,
    allow_scene_properties: bool,
    allow_inline_alts: bool,
) -> Result<Vec<TextRun>, FdxError> {
    let mut runs = Vec::new();
    let mut saw_alts = false;
    let mut saw_attachment = false;
    for child in &element.children {
        let RawNode::Element(text_element) = child else {
            continue;
        };
        if is_fdx_element(text_element, "Alts", element.name.namespace_uri.as_deref()) {
            if !allow_inline_alts || saw_alts {
                return Err(FdxError::UnsupportedConstruct);
            }
            saw_alts = true;
            continue;
        }
        if allow_scene_properties
            && is_fdx_element(
                text_element,
                "SceneProperties",
                element.name.namespace_uri.as_deref(),
            )
        {
            // Scene metadata is source-owned and retained in the preservation
            // tree. It is not inline text, but is a valid direct child of a
            // Scene Heading paragraph emitted by the new-document writer.
            continue;
        }
        if is_user_paragraph_data_name(&text_element.name, &element.name) {
            if saw_attachment {
                return Err(FdxError::UnsupportedConstruct);
            }
            saw_attachment = true;
            continue;
        }
        if text_element.name.local_name != "Text"
            || text_element.name.namespace_uri != element.name.namespace_uri
        {
            return Err(FdxError::UnsupportedConstruct);
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
        if text.is_empty() {
            continue;
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
    Ok(runs)
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
            let style =
                attribute(font, "Style").map(|style| style.split('+').collect::<BTreeSet<_>>());
            let projection = crate::formatting::SourceFontSpec {
                family: attribute(font, "Font")
                    .filter(|value| !value.is_empty() && value.len() <= 128)
                    .map(str::to_owned),
                size: attribute(font, "Size").and_then(parse_bounded_font_size),
                color: attribute(font, "Color")
                    .filter(|value| value.len() <= 128)
                    .map(str::to_owned),
                bold: style.as_ref().map(|tokens| tokens.contains("Bold")),
                italic: style.as_ref().map(|tokens| tokens.contains("Italic")),
                underline: style.as_ref().map(|tokens| tokens.contains("Underline")),
                all_caps: style.as_ref().map(|tokens| tokens.contains("AllCaps")),
                hidden_text: style.as_ref().map(|tokens| tokens.contains("HiddenText")),
            };
            if let Some(hidden_text) = projection.hidden_text {
                settings.role_print_visibility[role_index(role)] = Some(if hidden_text {
                    PrintVisibility::Hidden
                } else {
                    PrintVisibility::Visible
                });
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
        if attribute(paragraph, "Type").is_some_and(|paragraph_role| paragraph_role != source_role)
        {
            continue;
        }

        let index = role_index(role);
        let projection = crate::SourceParagraphSpec {
            alignment: attribute(paragraph, "Alignment").map(|alignment| match alignment {
                "Right" => crate::ParagraphAlignment::Right,
                "Center" => crate::ParagraphAlignment::Center,
                "Left" => crate::ParagraphAlignment::Left,
                _ => crate::ParagraphAlignment::Left,
            }),
            first_indent: attribute(paragraph, "FirstIndent")
                .and_then(|value| bounded_inches(value, true)),
            left_indent: attribute(paragraph, "LeftIndent")
                .and_then(|value| bounded_inches(value, false)),
            right_indent: attribute(paragraph, "RightIndent")
                .and_then(|value| bounded_inches(value, false)),
            leading_regular: (attribute(paragraph, "Leading") == Some("Regular")).then_some(true),
            space_before: attribute(paragraph, "SpaceBefore")
                .and_then(crate::FixedPoint::parse_points)
                .filter(|value| (0..=79_200).contains(&value.raw())),
            spacing_single: (attribute(paragraph, "Spacing") == Some("1")).then_some(true),
        };
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
    walk_elements(
        root,
        &mut |element, nearest_board, in_title_page, in_title_content, depth, is_alternate_copy| {
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
        if is_alternate_copy {
            // Exact AltCollection copies are retained as opaque source, but
            // never registered as competing global Paragraph definitions.
            return Ok(());
        }
        let domain = identity_domain(&element.name.local_name);
        let identity = ForeignIdentity::new(domain.clone(), value.to_owned())
            .map_err(map_preservation_error)?;
        // Exact TitlePage -> Content Paragraph occurrences are scoped identities.
        // Repeated source IDs among those editable occurrences stay distinct via
        // transient semantic IDs; they cannot shadow any globally defined ID.
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
        },
    )?;

    // Repeated scoped occurrences are not usable as global definitions, but one
    // registry entry keeps their foreign ID in the source-use scanner. This
    // prevents an external attribute/text consumer from being mistaken for an
    // unreferenced ID during later preservation edits. They may not shadow an
    // identity that is globally registered elsewhere.
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
        .filter(|(_, identity, _, scoped_title_page)| {
            !(*scoped_title_page
                && scoped_title_page_paragraphs
                    .get(identity)
                    .is_some_and(|count| *count > 1))
        })
        .map(|(_, identity, _, _)| identity.clone())
        .collect::<BTreeSet<_>>();
    let mut references = Vec::new();
    walk_elements(root, &mut |element,
                               nearest_board,
                               _in_title_page,
                               _in_title_content,
                               _depth,
                               _is_alternate_copy| {
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
    F: FnMut(&RawElement, Option<&ForeignIdentity>, bool, bool, usize, bool) -> Result<(), FdxError>,
{
    fn visit<F>(
        element: &RawElement,
        parent: Option<&RawElement>,
        grandparent: Option<&RawElement>,
        nearest_board: Option<&ForeignIdentity>,
        in_title_page: bool,
        in_title_content: bool,
        depth: usize,
        root_namespace: Option<&str>,
        visitor: &mut F,
    ) -> Result<(), FdxError>
    where
        F: FnMut(&RawElement, Option<&ForeignIdentity>, bool, bool, usize, bool) -> Result<(), FdxError>,
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
        visitor(
            element,
            active_board,
            in_title_page,
            in_title_content,
            depth,
            exact_alternate_copy,
        )?;
        for child in &element.children {
            if let RawNode::Element(child) = child {
                if is_user_paragraph_data_name(&child.name, &element.name) {
                    continue;
                }
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
                if !is_user_paragraph_data_lookalike(&child.name) {
                    inspect(child, total_image_text)?;
                }
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
        Error::ParagraphIdentityEntropyUnavailable => FdxError::ParagraphIdentityEntropyUnavailable,
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

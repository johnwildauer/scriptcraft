use crate::text_state::TextState;
use scriptcraft_model::Project;

/// A Version 6 reader result. `project` owns the effective document page layout;
/// its preservation state independently retains the original FDX source graph
/// and bytes, including unprojected settings and unsupported spellings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FdxDocument {
    pub project: Project,
    /// Bounded, value-free diagnostics produced while interpreting the source.
    pub import_diagnostics: FdxImportDiagnostics,
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

/// A privacy-safe import diagnostic. Settings ordinals identify source position
/// only; source labels and setting values are deliberately excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FdxImportDiagnostic {
    DuplicateElementSettings { settings_ordinal: usize },
}

/// Bounded diagnostics retained on every successful Version 6 reader result.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FdxImportDiagnostics {
    pub entries: Vec<FdxImportDiagnostic>,
    /// Number of additional duplicate entries suppressed after the cap.
    pub suppressed_duplicate_count: u32,
}

pub const MAX_FDX_IMPORT_DIAGNOSTICS: usize = 32;

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
    /// Older-version import may lose unsupported root sections/settings, but
    /// paragraph Type spelling and presence are not among those migration losses.
    OlderLossy {
        declared: String,
    },
    NewerBestEffort {
        declared: String,
    },
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

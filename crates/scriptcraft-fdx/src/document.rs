use crate::text_state::TextState;
use scriptcraft_model::{BeatPoint, BoardFixed, Project, MAX_BOARD_DIMENSION};

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
    /// Disposable FDX ScriptPanel display mode; never screenplay semantics.
    pub display_mode: ScriptPanelDisplayMode,
    /// Independent disposable visibility and marker color for the editor surfaces.
    pub invisibles: InvisiblesState,
    /// Disposable Beat viewport, independent of Project and editing history.
    pub beat_board_view_state: BeatBoardViewState,
    /// Disposable Story Map strip and horizontal timeline view, independent of Project.
    pub story_map_view_state: StoryMapViewState,
}

/// Disposable Story Map height, unresolved/known viewport width, and timeline view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoryMapViewState {
    height: u32,
    width: Option<BoardFixed>,
    scroll_origin: BoardFixed,
    zoom_milli_percent: u32,
}

impl StoryMapViewState {
    pub fn new(
        height: u32,
        width: Option<BoardFixed>,
        scroll_origin: BoardFixed,
        zoom_milli_percent: u32,
    ) -> Option<Self> {
        (height >= 90
            && height <= 1_000_000
            && width.is_none_or(|value| value.raw() > 0 && value <= MAX_BOARD_DIMENSION)
            && scroll_origin.raw() >= 0
            && scroll_origin <= MAX_BOARD_DIMENSION
            && (1..=120_000_000).contains(&zoom_milli_percent))
            .then_some(Self {
                height,
                width,
                scroll_origin,
                zoom_milli_percent,
            })
    }

    pub const fn height(self) -> u32 { self.height }
    pub const fn width(self) -> Option<BoardFixed> { self.width }
    pub const fn scroll_origin(self) -> BoardFixed { self.scroll_origin }
    pub const fn zoom_milli_percent(self) -> u32 { self.zoom_milli_percent }
}

impl Default for StoryMapViewState {
    fn default() -> Self {
        Self {
            height: 90,
            width: None,
            scroll_origin: BoardFixed::from_milli_pixels(0),
            zoom_milli_percent: 100_000,
        }
    }
}

/// Exact disposable Beat viewport coordinates and thousandths of a percent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeatBoardViewState {
    scroll_origin: BeatPoint,
    zoom_milli_percent: u32,
}

impl BeatBoardViewState {
    pub fn new(scroll_origin: BeatPoint, zoom_milli_percent: u32) -> Option<Self> {
        (scroll_origin.x.raw() >= 0
            && scroll_origin.y.raw() >= 0
            && scroll_origin.x <= MAX_BOARD_DIMENSION
            && scroll_origin.y <= MAX_BOARD_DIMENSION
            && (5_000..=300_000).contains(&zoom_milli_percent))
            .then_some(Self { scroll_origin, zoom_milli_percent })
    }

    pub const fn scroll_origin(self) -> BeatPoint {
        self.scroll_origin
    }

    pub const fn zoom_milli_percent(self) -> u32 {
        self.zoom_milli_percent
    }
}

impl Default for BeatBoardViewState {
    fn default() -> Self {
        Self { scroll_origin: BeatPoint::default(), zoom_milli_percent: 100_000 }
    }
}

/// Disposable FDX ScriptPanel presentation mode for the document body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScriptPanelDisplayMode {
    Normal,
    #[default]
    Page,
    Speed,
}

impl ScriptPanelDisplayMode {
    /// Return the exact FDX spelling for this display mode.
    pub const fn as_fdx_str(self) -> &'static str {
        match self {
            Self::Page => "Page",
            Self::Normal => "Normal",
            Self::Speed => "Speed",
        }
    }

    pub(crate) fn from_fdx_str(value: &str) -> Option<Self> {
        match value {
            "Page" => Some(Self::Page),
            "Normal" => Some(Self::Normal),
            "Speed" => Some(Self::Speed),
            _ => None,
        }
    }
}

/// RGB8 marker color projected from the root PageLayout InvisiblesColor setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvisiblesColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Default for InvisiblesColor {
    fn default() -> Self {
        Self {
            red: 128,
            green: 128,
            blue: 128,
        }
    }
}

/// Disposable, per-document editor view state; never screenplay semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvisiblesState {
    pub show_body: bool,
    pub show_title_page: bool,
    pub color: InvisiblesColor,
}

impl Default for InvisiblesState {
    fn default() -> Self {
        Self {
            show_body: false,
            show_title_page: false,
            color: InvisiblesColor::default(),
        }
    }
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

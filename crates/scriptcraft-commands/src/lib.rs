//! UI-independent command vocabulary, versioned wire envelope, and headless editor executor.
//!
//! The crate contains no desktop state. Its semantic editor executor operates
//! on the real core EditingSession; other commands remain descriptions for host
//! adapters. Serde's data model is the serialization boundary so hosts may
//! select a concrete transport format without coupling this crate to a codec.

#![forbid(unsafe_code)]

use std::{fmt, path::Path, str::FromStr};

use scriptcraft_core::editing::{Position as CorePosition, Selection as CoreSelection};
use scriptcraft_core::{ParagraphAlignment, Project, Role, RunStyle};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

pub mod executor;
pub use executor::{
    execute_editor_command, execute_editor_command_on_surface, CommandOutcome, CommandRejection,
    UnavailableReason,
};

/// Current version of the command-envelope contract.
pub const COMMAND_WIRE_VERSION: u16 = 1;

/// Stable identifier for a command. Identifiers are intentionally independent
/// of Rust variant names and use the dotted IDs in the public command catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandId {
    FileNew,
    FileOpenRequest,
    FileOpen,
    FileClose,
    FileSave,
    FileSaveAsRequest,
    FileSaveAs,
    FileSaveAsPdf,
    FileImportResolve,
    EditUndo,
    EditRedo,
    EditCut,
    EditCopy,
    EditPaste,
    EditDelete,
    EditSelectAll,
    EditorTextInsert,
    EditorTextPaste,
    EditorDeleteBackward,
    EditorParagraphEnter,
    EditorParagraphTab,
    EditorElementChange,
    EditorSelectionSet,
    EditorSelectionPointer,
    EditorNavigationHorizontal,
    EditorNavigationVertical,
    EditorNavigationLineEdge,
    EditorFocus,
    WindowTitlePageOpen,
    WindowTitlePageClose,
    EditorStyleToggle,
    EditorAlignmentSet,
    PdfIncludeTitlePageResolve,
    ViewZoomSet,
    ViewZoomPinch,
    ViewAppearanceSet,
    ViewFocusModeToggle,
    ToolsSprintTimerTogglePopup,
    ToolsSprintTimerClosePopup,
    ToolsSprintTimerSetMode,
    ToolsSprintTimerSetMinutes,
    ToolsSprintTimerStart,
    ToolsSprintTimerPause,
    ToolsSprintTimerResume,
    ToolsSprintTimerReset,
    ToolsSprintTimerEnd,
    UiStatusSet,
}

impl CommandId {
    /// All IDs in descriptor-catalog order.
    pub const ALL: [Self; 47] = [
        Self::FileNew,
        Self::FileOpenRequest,
        Self::FileOpen,
        Self::FileClose,
        Self::FileSave,
        Self::FileSaveAsRequest,
        Self::FileSaveAs,
        Self::FileSaveAsPdf,
        Self::FileImportResolve,
        Self::EditUndo,
        Self::EditRedo,
        Self::EditCut,
        Self::EditCopy,
        Self::EditPaste,
        Self::EditDelete,
        Self::EditSelectAll,
        Self::EditorTextInsert,
        Self::EditorTextPaste,
        Self::EditorDeleteBackward,
        Self::EditorParagraphEnter,
        Self::EditorParagraphTab,
        Self::EditorElementChange,
        Self::EditorSelectionSet,
        Self::EditorSelectionPointer,
        Self::EditorNavigationHorizontal,
        Self::EditorNavigationVertical,
        Self::EditorNavigationLineEdge,
        Self::EditorFocus,
        Self::WindowTitlePageOpen,
        Self::WindowTitlePageClose,
        Self::EditorStyleToggle,
        Self::EditorAlignmentSet,
        Self::PdfIncludeTitlePageResolve,
        Self::ViewZoomSet,
        Self::ViewZoomPinch,
        Self::ViewAppearanceSet,
        Self::ViewFocusModeToggle,
        Self::ToolsSprintTimerTogglePopup,
        Self::ToolsSprintTimerClosePopup,
        Self::ToolsSprintTimerSetMode,
        Self::ToolsSprintTimerSetMinutes,
        Self::ToolsSprintTimerStart,
        Self::ToolsSprintTimerPause,
        Self::ToolsSprintTimerResume,
        Self::ToolsSprintTimerReset,
        Self::ToolsSprintTimerEnd,
        Self::UiStatusSet,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FileNew => "file.new",
            Self::FileOpenRequest => "file.open.request",
            Self::FileOpen => "file.open",
            Self::FileClose => "file.close",
            Self::FileSave => "file.save",
            Self::FileSaveAsRequest => "file.save-as.request",
            Self::FileSaveAs => "file.save-as",
            Self::FileSaveAsPdf => "file.save-as-pdf",
            Self::FileImportResolve => "file.import.resolve",
            Self::EditUndo => "edit.undo",
            Self::EditRedo => "edit.redo",
            Self::EditCut => "edit.cut",
            Self::EditCopy => "edit.copy",
            Self::EditPaste => "edit.paste",
            Self::EditDelete => "edit.delete",
            Self::EditSelectAll => "edit.select-all",
            Self::EditorTextInsert => "editor.text.insert",
            Self::EditorTextPaste => "editor.text.paste",
            Self::EditorDeleteBackward => "editor.delete.backward",
            Self::EditorParagraphEnter => "editor.paragraph.enter",
            Self::EditorParagraphTab => "editor.paragraph.tab",
            Self::EditorElementChange => "editor.element.change",
            Self::EditorSelectionSet => "editor.selection.set",
            Self::EditorSelectionPointer => "editor.selection.pointer",
            Self::EditorNavigationHorizontal => "editor.navigation.horizontal",
            Self::EditorNavigationVertical => "editor.navigation.vertical",
            Self::EditorNavigationLineEdge => "editor.navigation.line-edge",
            Self::EditorFocus => "editor.focus",
            Self::WindowTitlePageOpen => "window.title-page.open",
            Self::WindowTitlePageClose => "window.title-page.close",
            Self::EditorStyleToggle => "editor.style.toggle",
            Self::EditorAlignmentSet => "editor.alignment.set",
            Self::PdfIncludeTitlePageResolve => "pdf.include-title-page.resolve",
            Self::ViewZoomSet => "view.zoom.set",
            Self::ViewZoomPinch => "view.zoom.pinch",
            Self::ViewAppearanceSet => "view.appearance.set",
            Self::ViewFocusModeToggle => "view.focus-mode.toggle",
            Self::ToolsSprintTimerTogglePopup => "tools.sprint-timer.toggle-popup",
            Self::ToolsSprintTimerClosePopup => "tools.sprint-timer.close-popup",
            Self::ToolsSprintTimerSetMode => "tools.sprint-timer.set-mode",
            Self::ToolsSprintTimerSetMinutes => "tools.sprint-timer.set-minutes",
            Self::ToolsSprintTimerStart => "tools.sprint-timer.start",
            Self::ToolsSprintTimerPause => "tools.sprint-timer.pause",
            Self::ToolsSprintTimerResume => "tools.sprint-timer.resume",
            Self::ToolsSprintTimerReset => "tools.sprint-timer.reset",
            Self::ToolsSprintTimerEnd => "tools.sprint-timer.end",
            Self::UiStatusSet => "ui.status.set",
        }
    }
}

impl fmt::Display for CommandId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for CommandId {
    type Err = UnknownCommandId;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|id| id.as_str() == value)
            .ok_or_else(|| UnknownCommandId(value.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownCommandId(pub String);

impl fmt::Display for UnknownCommandId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown command ID: {}", self.0)
    }
}

impl std::error::Error for UnknownCommandId {}

/// Diagnostic/routing source. Origins never change command semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandOrigin {
    Menu,
    Toolbar,
    NativeMenu,
    Shortcut,
    Keyboard,
    Pointer,
    ImeCommit,
    InternalFollowUp,
    Cli,
    Mcp,
    Test,
}

/// Screenplay element role serialized using the stable core role spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleValue(Role);

impl RoleValue {
    pub const fn new(role: Role) -> Self {
        Self(role)
    }

    pub const fn as_core(self) -> Role {
        self.0
    }

    pub const fn as_str(self) -> &'static str {
        self.0.as_str()
    }
}

impl Serialize for RoleValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RoleValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Role::parse(&value)
            .map(Self::new)
            .ok_or_else(|| de::Error::custom(format!("unknown screenplay role: {value}")))
    }
}

/// Semantic editing surface. Body is the compatibility default for payloads
/// written before surface-aware commands were introduced.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EditorSurface {
    #[default]
    Body,
    TitlePage,
}

impl<'de> Deserialize<'de> for EditorSurface {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match String::deserialize(deserializer)?.as_str() {
            "body" => Ok(Self::Body),
            "title_page" => Ok(Self::TitlePage),
            value => Err(de::Error::custom(format!("unknown editor surface: {value}"))),
        }
    }
}

impl EditorSurface {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Body => "body",
            Self::TitlePage => "title_page",
        }
    }
}

/// Bounded, typed inline Text style accepted by title and body commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TitleTextStyle {
    Bold,
    Italics,
    Underline,
    AllCaps,
    Strikethrough,
}

impl<'de> Deserialize<'de> for TitleTextStyle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match String::deserialize(deserializer)?.as_str() {
            "bold" => Ok(Self::Bold),
            "italics" => Ok(Self::Italics),
            "underline" => Ok(Self::Underline),
            "all_caps" => Ok(Self::AllCaps),
            "strikethrough" => Ok(Self::Strikethrough),
            value => Err(de::Error::custom(format!("unknown title text style: {value}"))),
        }
    }
}

impl TitleTextStyle {
    pub const fn as_core(self) -> RunStyle {
        match self {
            Self::Bold => RunStyle::Bold,
            Self::Italics => RunStyle::Italics,
            Self::Underline => RunStyle::Underline,
            Self::AllCaps => RunStyle::AllCaps,
            Self::Strikethrough => RunStyle::Strikethrough,
        }
    }
}

/// Bounded direct paragraph alignment value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParagraphAlignmentValue {
    Left,
    Right,
    Center,
    Justified,
}

impl<'de> Deserialize<'de> for ParagraphAlignmentValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match String::deserialize(deserializer)?.as_str() {
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            "center" => Ok(Self::Center),
            "justified" => Ok(Self::Justified),
            value => Err(de::Error::custom(format!("unknown paragraph alignment: {value}"))),
        }
    }
}

impl ParagraphAlignmentValue {
    pub const fn as_core(self) -> ParagraphAlignment {
        match self {
            Self::Left => ParagraphAlignment::Left,
            Self::Right => ParagraphAlignment::Right,
            Self::Center => ParagraphAlignment::Center,
            Self::Justified => ParagraphAlignment::Justified,
        }
    }
}

/// Transient PDF request options; never serialized into a native document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PdfExportOptions {
    pub include_title_page: bool,
}

impl Default for PdfExportOptions {
    fn default() -> Self {
        Self { include_title_page: true }
    }
}

/// Descriptions for effects that require the desktop host. The semantic
/// executor does not perform any of these operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostEffectDescription {
    OpenTitlePageWindow,
    FocusTitlePageWindow,
    CloseTitlePageWindow,
    FocusActiveEditor,
    OpenPdfDestinationPicker,
    ToggleFocusModeViewport,
    ResolvePdfIncludeTitlePage(PdfExportOptions),
}

/// UTF-8 byte position in the semantic screenplay model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Position {
    pub element: u32,
    pub offset: u32,
}

impl Position {
    pub fn from_core(position: CorePosition) -> Result<Self, PositionError> {
        Ok(Self {
            element: u32::try_from(position.element).map_err(|_| PositionError::IndexOutOfRange)?,
            offset: u32::try_from(position.offset).map_err(|_| PositionError::IndexOutOfRange)?,
        })
    }

    pub fn to_core(self) -> Result<CorePosition, PositionError> {
        Ok(CorePosition {
            element: usize::try_from(self.element).map_err(|_| PositionError::IndexOutOfRange)?,
            offset: usize::try_from(self.offset).map_err(|_| PositionError::IndexOutOfRange)?,
        })
    }
}

/// Selection expressed as element indexes and UTF-8 byte offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub anchor: Position,
    pub focus: Position,
}

impl Selection {
    pub fn from_core(selection: CoreSelection) -> Result<Self, PositionError> {
        Ok(Self {
            anchor: Position::from_core(selection.anchor)?,
            focus: Position::from_core(selection.focus)?,
        })
    }

    pub fn to_core(self) -> Result<CoreSelection, PositionError> {
        Ok(CoreSelection {
            anchor: self.anchor.to_core()?,
            focus: self.focus.to_core()?,
        })
    }

    /// Validates element indexes and UTF-8 byte boundaries against the active
    /// semantic project before a host passes this selection to an executor.
    pub fn validate_for(self, project: &Project) -> Result<(), SelectionError> {
        self.validate_for_surface(project, EditorSurface::Body)
    }

    pub fn validate_for_surface(
        self,
        project: &Project,
        surface: EditorSurface,
    ) -> Result<(), SelectionError> {
        validate_position(self.anchor, project, surface)?;
        validate_position(self.focus, project, surface)
    }
}

fn validate_position(
    position: Position,
    project: &Project,
    surface: EditorSurface,
) -> Result<(), SelectionError> {
    let index = usize::try_from(position.element).map_err(|_| SelectionError::ElementOutOfRange)?;
    let text = match surface {
        EditorSurface::Body => project.elements.get(index).map(|element| element.text.as_str()),
        EditorSurface::TitlePage => project.title_page.get(index).map(|paragraph| paragraph.text.as_str()),
    }
    .ok_or(SelectionError::ElementOutOfRange)?;
    let offset = usize::try_from(position.offset).map_err(|_| SelectionError::OffsetOutOfRange)?;
    if offset > text.len() {
        return Err(SelectionError::OffsetOutOfRange);
    }
    if !text.is_char_boundary(offset) {
        return Err(SelectionError::InvalidUtf8Boundary);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionError {
    ElementOutOfRange,
    OffsetOutOfRange,
    InvalidUtf8Boundary,
}

impl fmt::Display for SelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ElementOutOfRange => formatter.write_str("selection element is out of range"),
            Self::OffsetOutOfRange => formatter.write_str("selection byte offset is out of range"),
            Self::InvalidUtf8Boundary => formatter.write_str("selection is not on a UTF-8 boundary"),
        }
    }
}

impl std::error::Error for SelectionError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionError {
    IndexOutOfRange,
}

impl fmt::Display for PositionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IndexOutOfRange => formatter.write_str("selection index is out of range"),
        }
    }
}

impl std::error::Error for PositionError {}

/// A portable, nonempty UTF-8 path payload; native handles never cross the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentPath(String);

impl DocumentPath {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidDocumentPath> {
        let value = value.into();
        if value.trim().is_empty() || value.contains('\0') {
            return Err(InvalidDocumentPath);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl Serialize for DocumentPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DocumentPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidDocumentPath;

impl fmt::Display for InvalidDocumentPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("document path must be nonempty UTF-8 without NUL")
    }
}

impl std::error::Error for InvalidDocumentPath {}

/// Maximum UTF-8 byte length for a single diagnostic status line.
pub const MAX_DIAGNOSTIC_MESSAGE_BYTES: usize = 512;

/// A bounded, single-line diagnostic message suitable for the UI status channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticMessage(String);

impl DiagnosticMessage {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidDiagnosticMessage> {
        let value = value.into();
        if value.trim().is_empty()
            || value.len() > MAX_DIAGNOSTIC_MESSAGE_BYTES
            || value.chars().any(|character| {
                character.is_control() || matches!(character, '\u{2028}' | '\u{2029}')
            })
        {
            return Err(InvalidDiagnosticMessage);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for DiagnosticMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DiagnosticMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidDiagnosticMessage;

impl fmt::Display for InvalidDiagnosticMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "diagnostic message must be nonempty, single-line, and no longer than {MAX_DIAGNOSTIC_MESSAGE_BYTES} UTF-8 bytes"
        )
    }
}

impl std::error::Error for InvalidDiagnosticMessage {}

/// Whole-number zoom percentage constrained to the current desktop range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ZoomPercent(u16);

impl ZoomPercent {
    pub const MIN: u16 = 75;
    pub const MAX: u16 = 300;

    pub fn new(value: u16) -> Result<Self, InvalidZoomPercent> {
        (Self::MIN..=Self::MAX)
            .contains(&value)
            .then_some(Self(value))
            .ok_or(InvalidZoomPercent)
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

impl<'de> Deserialize<'de> for ZoomPercent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(u16::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidZoomPercent;

impl fmt::Display for InvalidZoomPercent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "zoom percentage must be in {}..={}", ZoomPercent::MIN, ZoomPercent::MAX)
    }
}

impl std::error::Error for InvalidZoomPercent {}

/// Positive finite multiplicative zoom input.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ZoomFactor(f32);

impl ZoomFactor {
    pub fn new(value: f32) -> Result<Self, InvalidZoomFactor> {
        (value.is_finite() && value > 0.0)
            .then_some(Self(value))
            .ok_or(InvalidZoomFactor)
    }

    pub const fn get(self) -> f32 {
        self.0
    }
}

impl<'de> Deserialize<'de> for ZoomFactor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(f32::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidZoomFactor;

impl fmt::Display for InvalidZoomFactor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("zoom factor must be finite and greater than zero")
    }
}

impl std::error::Error for InvalidZoomFactor {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportResolution {
    Continue,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PointerPhase {
    Press,
    Drag,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HorizontalDirection {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerticalDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineEdge {
    Start,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Appearance {
    System,
    Day,
    Night,
    Midnight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SprintTimerMode {
    Countdown,
    Stopwatch,
}

/// Typed command payload. Adjacent tagging fixes the wire keys to `id` and
/// `params`; the dotted IDs are explicit serde names, not inferred variant
/// spellings. Unknown command names and unknown fields are rejected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "id", content = "params", deny_unknown_fields)]
pub enum Command {
    #[serde(rename = "file.new")]
    FileNew,
    #[serde(rename = "file.open.request")]
    FileOpenRequest,
    #[serde(rename = "file.open")]
    FileOpen { path: DocumentPath },
    #[serde(rename = "file.close")]
    FileClose,
    #[serde(rename = "file.save")]
    FileSave,
    #[serde(rename = "file.save-as.request")]
    FileSaveAsRequest,
    #[serde(rename = "file.save-as")]
    FileSaveAs {
        path: DocumentPath,
        #[serde(default)]
        overwrite: bool,
    },
    #[serde(rename = "file.save-as-pdf")]
    FileSaveAsPdf,
    #[serde(rename = "file.import.resolve")]
    FileImportResolve { resolution: ImportResolution },
    #[serde(rename = "edit.undo")]
    EditUndo,
    #[serde(rename = "edit.redo")]
    EditRedo,
    #[serde(rename = "edit.cut")]
    EditCut,
    #[serde(rename = "edit.copy")]
    EditCopy,
    #[serde(rename = "edit.paste")]
    EditPaste,
    #[serde(rename = "edit.delete")]
    EditDelete,
    #[serde(rename = "edit.select-all")]
    EditSelectAll,
    #[serde(rename = "editor.text.insert")]
    EditorTextInsert { text: String, #[serde(default)] surface: EditorSurface },
    #[serde(rename = "editor.text.paste")]
    EditorTextPaste { text: String, #[serde(default)] surface: EditorSurface },
    #[serde(rename = "editor.delete.backward")]
    EditorDeleteBackward,
    #[serde(rename = "editor.paragraph.enter")]
    EditorParagraphEnter,
    #[serde(rename = "editor.paragraph.tab")]
    EditorParagraphTab,
    #[serde(rename = "editor.element.change")]
    EditorElementChange { role: RoleValue, #[serde(default)] surface: EditorSurface },
    #[serde(rename = "editor.selection.set")]
    EditorSelectionSet { selection: Selection, #[serde(default)] surface: EditorSurface },
    #[serde(rename = "editor.selection.pointer")]
    EditorSelectionPointer { position: Position, phase: PointerPhase, #[serde(default)] surface: EditorSurface },
    #[serde(rename = "editor.navigation.horizontal")]
    EditorNavigationHorizontal { direction: HorizontalDirection, extend: bool, #[serde(default)] surface: EditorSurface },
    #[serde(rename = "editor.navigation.vertical")]
    EditorNavigationVertical { direction: VerticalDirection, extend: bool, #[serde(default)] surface: EditorSurface },
    #[serde(rename = "editor.navigation.line-edge")]
    EditorNavigationLineEdge { edge: LineEdge, extend: bool, #[serde(default)] surface: EditorSurface },
    #[serde(rename = "editor.focus")]
    EditorFocus,
    #[serde(rename = "window.title-page.open")]
    WindowTitlePageOpen,
    #[serde(rename = "window.title-page.close")]
    WindowTitlePageClose,
    #[serde(rename = "editor.style.toggle")]
    EditorStyleToggle { surface: EditorSurface, style: TitleTextStyle },
    #[serde(rename = "editor.alignment.set")]
    EditorAlignmentSet { surface: EditorSurface, alignment: ParagraphAlignmentValue },
    #[serde(rename = "pdf.include-title-page.resolve")]
    PdfIncludeTitlePageResolve { options: PdfExportOptions },
    #[serde(rename = "view.zoom.set")]
    ViewZoomSet { percent: ZoomPercent },
    #[serde(rename = "view.zoom.pinch")]
    ViewZoomPinch { factor: ZoomFactor },
    #[serde(rename = "view.appearance.set")]
    ViewAppearanceSet { appearance: Appearance },
    #[serde(rename = "view.focus-mode.toggle")]
    ViewFocusModeToggle,
    #[serde(rename = "tools.sprint-timer.toggle-popup")]
    ToolsSprintTimerTogglePopup,
    #[serde(rename = "tools.sprint-timer.close-popup")]
    ToolsSprintTimerClosePopup,
    #[serde(rename = "tools.sprint-timer.set-mode")]
    ToolsSprintTimerSetMode { mode: SprintTimerMode },
    #[serde(rename = "tools.sprint-timer.set-minutes")]
    ToolsSprintTimerSetMinutes { text: String },
    #[serde(rename = "tools.sprint-timer.start")]
    ToolsSprintTimerStart,
    #[serde(rename = "tools.sprint-timer.pause")]
    ToolsSprintTimerPause,
    #[serde(rename = "tools.sprint-timer.resume")]
    ToolsSprintTimerResume,
    #[serde(rename = "tools.sprint-timer.reset")]
    ToolsSprintTimerReset,
    #[serde(rename = "tools.sprint-timer.end")]
    ToolsSprintTimerEnd,
    #[serde(rename = "ui.status.set")]
    UiStatusSet { message: Option<DiagnosticMessage> },
}

impl Command {
    pub const fn host_effect_description(&self) -> Option<HostEffectDescription> {
        match self {
            Self::FileSaveAsPdf => Some(HostEffectDescription::OpenPdfDestinationPicker),
            Self::ViewFocusModeToggle => Some(HostEffectDescription::ToggleFocusModeViewport),
            Self::EditorFocus => Some(HostEffectDescription::FocusActiveEditor),
            Self::WindowTitlePageOpen => Some(HostEffectDescription::OpenTitlePageWindow),
            Self::WindowTitlePageClose => Some(HostEffectDescription::CloseTitlePageWindow),
            Self::PdfIncludeTitlePageResolve { options } => {
                Some(HostEffectDescription::ResolvePdfIncludeTitlePage(*options))
            }
            _ => None,
        }
    }

    pub const fn id(&self) -> CommandId {
        match self {
            Self::FileNew => CommandId::FileNew,
            Self::FileOpenRequest => CommandId::FileOpenRequest,
            Self::FileOpen { .. } => CommandId::FileOpen,
            Self::FileClose => CommandId::FileClose,
            Self::FileSave => CommandId::FileSave,
            Self::FileSaveAsRequest => CommandId::FileSaveAsRequest,
            Self::FileSaveAs { .. } => CommandId::FileSaveAs,
            Self::FileSaveAsPdf => CommandId::FileSaveAsPdf,
            Self::FileImportResolve { .. } => CommandId::FileImportResolve,
            Self::EditUndo => CommandId::EditUndo,
            Self::EditRedo => CommandId::EditRedo,
            Self::EditCut => CommandId::EditCut,
            Self::EditCopy => CommandId::EditCopy,
            Self::EditPaste => CommandId::EditPaste,
            Self::EditDelete => CommandId::EditDelete,
            Self::EditSelectAll => CommandId::EditSelectAll,
            Self::EditorTextInsert { .. } => CommandId::EditorTextInsert,
            Self::EditorTextPaste { .. } => CommandId::EditorTextPaste,
            Self::EditorDeleteBackward => CommandId::EditorDeleteBackward,
            Self::EditorParagraphEnter => CommandId::EditorParagraphEnter,
            Self::EditorParagraphTab => CommandId::EditorParagraphTab,
            Self::EditorElementChange { .. } => CommandId::EditorElementChange,
            Self::EditorSelectionSet { .. } => CommandId::EditorSelectionSet,
            Self::EditorSelectionPointer { .. } => CommandId::EditorSelectionPointer,
            Self::EditorNavigationHorizontal { .. } => CommandId::EditorNavigationHorizontal,
            Self::EditorNavigationVertical { .. } => CommandId::EditorNavigationVertical,
            Self::EditorNavigationLineEdge { .. } => CommandId::EditorNavigationLineEdge,
            Self::EditorFocus => CommandId::EditorFocus,
            Self::WindowTitlePageOpen => CommandId::WindowTitlePageOpen,
            Self::WindowTitlePageClose => CommandId::WindowTitlePageClose,
            Self::EditorStyleToggle { .. } => CommandId::EditorStyleToggle,
            Self::EditorAlignmentSet { .. } => CommandId::EditorAlignmentSet,
            Self::PdfIncludeTitlePageResolve { .. } => CommandId::PdfIncludeTitlePageResolve,
            Self::ViewZoomSet { .. } => CommandId::ViewZoomSet,
            Self::ViewZoomPinch { .. } => CommandId::ViewZoomPinch,
            Self::ViewAppearanceSet { .. } => CommandId::ViewAppearanceSet,
            Self::ViewFocusModeToggle => CommandId::ViewFocusModeToggle,
            Self::ToolsSprintTimerTogglePopup => CommandId::ToolsSprintTimerTogglePopup,
            Self::ToolsSprintTimerClosePopup => CommandId::ToolsSprintTimerClosePopup,
            Self::ToolsSprintTimerSetMode { .. } => CommandId::ToolsSprintTimerSetMode,
            Self::ToolsSprintTimerSetMinutes { .. } => CommandId::ToolsSprintTimerSetMinutes,
            Self::ToolsSprintTimerStart => CommandId::ToolsSprintTimerStart,
            Self::ToolsSprintTimerPause => CommandId::ToolsSprintTimerPause,
            Self::ToolsSprintTimerResume => CommandId::ToolsSprintTimerResume,
            Self::ToolsSprintTimerReset => CommandId::ToolsSprintTimerReset,
            Self::ToolsSprintTimerEnd => CommandId::ToolsSprintTimerEnd,
            Self::UiStatusSet { .. } => CommandId::UiStatusSet,
        }
    }
}

/// Serializable command plus diagnostic origin and wire version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandEnvelope {
    pub version: u16,
    pub origin: CommandOrigin,
    pub command: Command,
}

impl CommandEnvelope {
    pub const fn new(origin: CommandOrigin, command: Command) -> Self {
        Self {
            version: COMMAND_WIRE_VERSION,
            origin,
            command,
        }
    }

    pub fn validate(&self) -> Result<(), EnvelopeError> {
        if self.version != COMMAND_WIRE_VERSION {
            return Err(EnvelopeError::UnsupportedVersion(self.version));
        }
        validate_descriptor_catalog().map_err(EnvelopeError::InvalidCatalog)?;
        Ok(())
    }

    /// Performs document-dependent validation before command execution.
    pub fn validate_for_project(&self, project: &Project) -> Result<(), EnvelopeError> {
        self.validate()?;
        match &self.command {
            Command::EditorSelectionSet { selection, surface } => selection
                .validate_for_surface(project, *surface)
                .map_err(EnvelopeError::InvalidSelection),
            Command::EditorSelectionPointer { position, surface, .. } => Selection {
                anchor: *position,
                focus: *position,
            }
            .validate_for_surface(project, *surface)
            .map_err(EnvelopeError::InvalidSelection),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeError {
    UnsupportedVersion(u16),
    InvalidCatalog(CatalogError),
    InvalidSelection(SelectionError),
}

impl fmt::Display for EnvelopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported command wire version: {version}")
            }
            Self::InvalidCatalog(error) => write!(formatter, "invalid command catalog: {error}"),
            Self::InvalidSelection(error) => write!(formatter, "invalid command selection: {error}"),
        }
    }
}

impl std::error::Error for EnvelopeError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandDescriptor {
    pub id: CommandId,
    pub family: &'static str,
    /// Human-readable field summary; typed validation is provided by `Command`.
    pub parameters: &'static str,
    /// Availability is intentionally evaluated by the executing host.
    pub availability: AvailabilityPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvailabilityPolicy {
    RecheckAtExecution,
}

const fn descriptor(id: CommandId, family: &'static str, parameters: &'static str) -> CommandDescriptor {
    CommandDescriptor {
        id,
        family,
        parameters,
        availability: AvailabilityPolicy::RecheckAtExecution,
    }
}

/// Authoritative discovery catalog; disabled placeholders are deliberately absent.
pub static COMMAND_DESCRIPTORS: [CommandDescriptor; 47] = [
    descriptor(CommandId::FileNew, "file", "none"),
    descriptor(CommandId::FileOpenRequest, "file", "none"),
    descriptor(CommandId::FileOpen, "file", "path: document_path"),
    descriptor(CommandId::FileClose, "file", "none"),
    descriptor(CommandId::FileSave, "file", "none"),
    descriptor(CommandId::FileSaveAsRequest, "file", "none"),
    descriptor(
        CommandId::FileSaveAs,
        "file",
        "path: document_path, overwrite: bool (default false)",
    ),
    descriptor(CommandId::FileSaveAsPdf, "file", "none"),
    descriptor(CommandId::FileImportResolve, "file", "resolution: import_resolution"),
    descriptor(CommandId::EditUndo, "edit", "none"),
    descriptor(CommandId::EditRedo, "edit", "none"),
    descriptor(CommandId::EditCut, "edit", "none"),
    descriptor(CommandId::EditCopy, "edit", "none"),
    descriptor(CommandId::EditPaste, "edit", "none"),
    descriptor(CommandId::EditDelete, "edit", "none"),
    descriptor(CommandId::EditSelectAll, "edit", "none"),
    descriptor(CommandId::EditorTextInsert, "editor", "text: string, surface: editor_surface (default body)"),
    descriptor(CommandId::EditorTextPaste, "editor", "text: string, surface: editor_surface (default body)"),
    descriptor(CommandId::EditorDeleteBackward, "editor", "none; host active surface"),
    descriptor(CommandId::EditorParagraphEnter, "editor", "none; host active surface"),
    descriptor(CommandId::EditorParagraphTab, "editor", "none; host active surface"),
    descriptor(CommandId::EditorElementChange, "editor", "role: screenplay_role, surface: editor_surface (default body)"),
    descriptor(CommandId::EditorSelectionSet, "editor", "selection: utf8_byte_positions, surface: editor_surface (default body)"),
    descriptor(CommandId::EditorSelectionPointer, "editor", "position, phase, surface: editor_surface (default body)"),
    descriptor(CommandId::EditorNavigationHorizontal, "editor", "direction, extend, surface: editor_surface (default body)"),
    descriptor(CommandId::EditorNavigationVertical, "editor", "direction, extend, surface: editor_surface (default body)"),
    descriptor(CommandId::EditorNavigationLineEdge, "editor", "edge, extend, surface: editor_surface (default body)"),
    descriptor(CommandId::EditorFocus, "editor", "none"),
    descriptor(CommandId::WindowTitlePageOpen, "window", "none; host viewport effect"),
    descriptor(CommandId::WindowTitlePageClose, "window", "none; host viewport effect"),
    descriptor(CommandId::EditorStyleToggle, "editor", "surface, style: bounded_text_style"),
    descriptor(CommandId::EditorAlignmentSet, "editor", "surface, alignment: bounded_alignment"),
    descriptor(CommandId::PdfIncludeTitlePageResolve, "pdf", "options: pdf_export_options; host dialog effect"),
    descriptor(CommandId::ViewZoomSet, "view", "percent: 75..=300"),
    descriptor(CommandId::ViewZoomPinch, "view", "factor: finite_positive_f32"),
    descriptor(CommandId::ViewAppearanceSet, "view", "appearance: appearance"),
    descriptor(CommandId::ViewFocusModeToggle, "view", "none"),
    descriptor(CommandId::ToolsSprintTimerTogglePopup, "tools", "none"),
    descriptor(CommandId::ToolsSprintTimerClosePopup, "tools", "none"),
    descriptor(CommandId::ToolsSprintTimerSetMode, "tools", "mode: sprint_timer_mode"),
    descriptor(CommandId::ToolsSprintTimerSetMinutes, "tools", "text: string"),
    descriptor(CommandId::ToolsSprintTimerStart, "tools", "none"),
    descriptor(CommandId::ToolsSprintTimerPause, "tools", "none"),
    descriptor(CommandId::ToolsSprintTimerResume, "tools", "none"),
    descriptor(CommandId::ToolsSprintTimerReset, "tools", "none"),
    descriptor(CommandId::ToolsSprintTimerEnd, "tools", "none"),
    descriptor(CommandId::UiStatusSet, "ui", "message: optional diagnostic_message (<=512 UTF-8 bytes, single-line)"),
];

pub fn descriptor_for(id: CommandId) -> Option<&'static CommandDescriptor> {
    COMMAND_DESCRIPTORS
        .iter()
        .find(|descriptor| descriptor.id == id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogError {
    DuplicateId(CommandId),
    MissingDescriptor(CommandId),
    UnexpectedDescriptor(CommandId),
}

impl fmt::Display for CatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(formatter, "duplicate descriptor for {id}"),
            Self::MissingDescriptor(id) => write!(formatter, "missing descriptor for {id}"),
            Self::UnexpectedDescriptor(id) => write!(formatter, "unexpected descriptor for {id}"),
        }
    }
}

impl std::error::Error for CatalogError {}

/// Checks catalog uniqueness and exact coverage of the typed ID vocabulary.
pub fn validate_descriptor_catalog() -> Result<(), CatalogError> {
    for (index, descriptor) in COMMAND_DESCRIPTORS.iter().enumerate() {
        if COMMAND_DESCRIPTORS[..index]
            .iter()
            .any(|earlier| earlier.id == descriptor.id)
        {
            return Err(CatalogError::DuplicateId(descriptor.id));
        }
        if !CommandId::ALL.contains(&descriptor.id) {
            return Err(CatalogError::UnexpectedDescriptor(descriptor.id));
        }
    }
    for id in CommandId::ALL {
        if !COMMAND_DESCRIPTORS.iter().any(|descriptor| descriptor.id == id) {
            return Err(CatalogError::MissingDescriptor(id));
        }
    }
    Ok(())
}

#[cfg(test)]
mod save_as_wire_compatibility_tests {
    use super::{Command, CommandId, EditorSurface};
    use serde::de::value::{Error as ValueError, MapDeserializer};
    use serde::de::{IntoDeserializer, Visitor};
    use serde::{Deserialize, Deserializer};

    enum WireValue {
        String(&'static str),
        Bool(bool),
        Map(Vec<(&'static str, WireValue)>),
    }

    impl<'de> IntoDeserializer<'de, ValueError> for WireValue {
        type Deserializer = Self;

        fn into_deserializer(self) -> Self::Deserializer {
            self
        }
    }

    impl<'de> Deserializer<'de> for WireValue {
        type Error = ValueError;

        fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            match self {
                Self::String(value) => visitor.visit_string(value.to_owned()),
                Self::Bool(value) => visitor.visit_bool(value),
                Self::Map(entries) => visitor.visit_map(MapDeserializer::new(
                    entries
                        .into_iter()
                        .map(|(key, value)| (Self::String(key), value)),
                )),
            }
        }

        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
            map struct enum identifier ignored_any
        }
    }

    fn save_as_wire(params: Vec<(&'static str, WireValue)>) -> WireValue {
        WireValue::Map(vec![
            ("id", WireValue::String("file.save-as")),
            ("params", WireValue::Map(params)),
        ])
    }

    fn insert_wire(params: Vec<(&'static str, WireValue)>) -> WireValue {
        WireValue::Map(vec![
            ("id", WireValue::String("editor.text.insert")),
            ("params", WireValue::Map(params)),
        ])
    }

    fn command_wire(id: &'static str, params: Vec<(&'static str, WireValue)>) -> WireValue {
        WireValue::Map(vec![
            ("id", WireValue::String(id)),
            ("params", WireValue::Map(params)),
        ])
    }

    #[test]
    fn omitted_overwrite_consent_deserializes_as_false_and_preserves_command_id() {
        let command = Command::deserialize(save_as_wire(vec![(
            "path",
            WireValue::String("/tmp/legacy-command.fdx"),
        )]))
        .unwrap_or_else(|_| unreachable!("legacy Save As payload remains decodable"));

        assert_eq!(command.id(), CommandId::FileSaveAs);
        assert!(matches!(
            command,
            Command::FileSaveAs {
                overwrite: false,
                ..
            }
        ));
    }

    #[test]
    fn explicit_overwrite_consent_round_trips_as_true() {
        let command = Command::deserialize(save_as_wire(vec![
            ("path", WireValue::String("/tmp/confirmed-command.fdx")),
            ("overwrite", WireValue::Bool(true)),
        ]))
        .unwrap_or_else(|_| unreachable!("explicit Save As payload is decodable"));

        assert!(matches!(
            command,
            Command::FileSaveAs {
                overwrite: true,
                ..
            }
        ));
    }

    #[test]
    fn legacy_text_insert_payload_defaults_to_body_and_explicit_surface_is_typed() {
        let legacy = Command::deserialize(insert_wire(vec![("text", WireValue::String("old"))]))
            .unwrap_or_else(|_| unreachable!("legacy body editor payload remains decodable"));
        assert_eq!(legacy.id().as_str(), "editor.text.insert");
        assert!(matches!(legacy, Command::EditorTextInsert { surface: EditorSurface::Body, .. }));

        let title = Command::deserialize(insert_wire(vec![
            ("text", WireValue::String("title")),
            ("surface", WireValue::String("title_page")),
        ]))
        .unwrap_or_else(|_| unreachable!("explicit title-page surface is a stable value"));
        assert!(matches!(title, Command::EditorTextInsert { surface: EditorSurface::TitlePage, .. }));
    }

    #[test]
    fn unknown_surface_and_payload_fields_are_rejected() {
        assert!(Command::deserialize(insert_wire(vec![
            ("text", WireValue::String("bad")),
            ("surface", WireValue::String("canvas")),
        ]))
        .is_err());
        assert!(Command::deserialize(insert_wire(vec![
            ("text", WireValue::String("bad")),
            ("mystery", WireValue::Bool(true)),
        ]))
        .is_err());
    }

    #[test]
    fn formatting_and_pdf_option_payloads_decode_to_bounded_typed_values() {
        let style = Command::deserialize(command_wire(
            "editor.style.toggle",
            vec![
                ("surface", WireValue::String("title_page")),
                ("style", WireValue::String("strikethrough")),
            ],
        ))
        .unwrap_or_else(|_| unreachable!("bounded style command payload is valid"));
        assert_eq!(style.id(), CommandId::EditorStyleToggle);

        let alignment = Command::deserialize(command_wire(
            "editor.alignment.set",
            vec![
                ("surface", WireValue::String("body")),
                ("alignment", WireValue::String("justified")),
            ],
        ))
        .unwrap_or_else(|_| unreachable!("bounded alignment command payload is valid"));
        assert_eq!(alignment.id(), CommandId::EditorAlignmentSet);

        let pdf = Command::deserialize(command_wire(
            "pdf.include-title-page.resolve",
            vec![("options", WireValue::Map(vec![("include_title_page", WireValue::Bool(false))]))],
        ))
        .unwrap_or_else(|_| unreachable!("bounded PDF options payload is valid"));
        assert!(matches!(pdf, Command::PdfIncludeTitlePageResolve { options: super::PdfExportOptions { include_title_page: false } }));

        assert!(Command::deserialize(command_wire(
            "editor.alignment.set",
            vec![
                ("surface", WireValue::String("body")),
                ("alignment", WireValue::String("diagonal")),
            ],
        ))
        .is_err());
    }
}

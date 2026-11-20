use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use super::{
    Appearance, BuiltinTemplateValue, DiagnosticMessage, DocumentPath, EditorSurface,
    ElementTypeValue, HorizontalDirection, ImportResolution, LineEdge, ParagraphAlignmentValue,
    PdfExportOptions, PointerPhase, Position, ScriptViewMode, Selection, SprintTimerMode,
    TitleTextStyle, VerticalDirection, ZoomFactor, ZoomPercent,
};

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
    FilePrint,
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
    EditorDualDialogueToggle,
    EditorStyleClear,
    ApplicationQuit,
    ViewScriptViewSet,
    FormatElementsOpen,
    FormatElementsClose,
    FormatTemplateSelectorOpen,
    FormatTemplateSelectorCancel,
    FormatTemplateApply,
    FileNewFromTemplateOpen,
    FileNewFromTemplate,
}

impl CommandId {
    /// All IDs in descriptor-catalog order.
    pub const ALL: [Self; 59] = [
        Self::FileNew,
        Self::FileOpenRequest,
        Self::FileOpen,
        Self::FileClose,
        Self::FileSave,
        Self::FileSaveAsRequest,
        Self::FileSaveAs,
        Self::FileSaveAsPdf,
        Self::FilePrint,
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
        Self::EditorDualDialogueToggle,
        Self::EditorStyleClear,
        Self::ApplicationQuit,
        Self::ViewScriptViewSet,
        Self::FormatElementsOpen,
        Self::FormatElementsClose,
        Self::FormatTemplateSelectorOpen,
        Self::FormatTemplateSelectorCancel,
        Self::FormatTemplateApply,
        Self::FileNewFromTemplateOpen,
        Self::FileNewFromTemplate,
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
            Self::FilePrint => "file.print",
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
            Self::EditorDualDialogueToggle => "editor.dual-dialogue.toggle",
            Self::EditorStyleClear => "editor.style.clear",
            Self::ApplicationQuit => "application.quit",
            Self::ViewScriptViewSet => "view.script-view.set",
            Self::FormatElementsOpen => "format.elements.open",
            Self::FormatElementsClose => "format.elements.close",
            Self::FormatTemplateSelectorOpen => "format.template-selector.open",
            Self::FormatTemplateSelectorCancel => "format.template-selector.cancel",
            Self::FormatTemplateApply => "format.template.apply",
            Self::FileNewFromTemplateOpen => "file.new-from-template.open",
            Self::FileNewFromTemplate => "file.new-from-template",
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

/// Descriptions for effects that require the desktop host. The semantic
/// executor does not perform any of these operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostEffectDescription {
    OpenTitlePageWindow,
    FocusTitlePageWindow,
    CloseTitlePageWindow,
    FocusActiveEditor,
    OpenPdfDestinationPicker,
    OpenPrintPanel,
    ToggleFocusModeViewport,
    ResolvePdfIncludeTitlePage(PdfExportOptions),
    OpenElementsWindow,
    CloseElementsWindow,
    OpenTemplateSelector,
    CancelTemplateSelector,
    OpenNewTemplateSelector,
    LaunchIndependentTemplateChild(BuiltinTemplateValue),
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
    #[serde(rename = "file.print")]
    FilePrint,
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
    EditorTextInsert {
        text: String,
        #[serde(default)]
        surface: EditorSurface,
    },
    #[serde(rename = "editor.text.paste")]
    EditorTextPaste {
        text: String,
        #[serde(default)]
        surface: EditorSurface,
    },
    #[serde(rename = "editor.delete.backward")]
    EditorDeleteBackward,
    #[serde(rename = "editor.paragraph.enter")]
    EditorParagraphEnter,
    #[serde(rename = "editor.paragraph.tab")]
    EditorParagraphTab,
    #[serde(rename = "editor.element.change")]
    EditorElementChange {
        element_type: ElementTypeValue,
        #[serde(default)]
        surface: EditorSurface,
    },
    #[serde(rename = "editor.selection.set")]
    EditorSelectionSet {
        selection: Selection,
        #[serde(default)]
        surface: EditorSurface,
    },
    #[serde(rename = "editor.selection.pointer")]
    EditorSelectionPointer {
        position: Position,
        phase: PointerPhase,
        #[serde(default)]
        surface: EditorSurface,
    },
    #[serde(rename = "editor.navigation.horizontal")]
    EditorNavigationHorizontal {
        direction: HorizontalDirection,
        extend: bool,
        #[serde(default)]
        surface: EditorSurface,
    },
    #[serde(rename = "editor.navigation.vertical")]
    EditorNavigationVertical {
        direction: VerticalDirection,
        extend: bool,
        #[serde(default)]
        surface: EditorSurface,
    },
    #[serde(rename = "editor.navigation.line-edge")]
    EditorNavigationLineEdge {
        edge: LineEdge,
        extend: bool,
        #[serde(default)]
        surface: EditorSurface,
    },
    #[serde(rename = "editor.focus")]
    EditorFocus,
    #[serde(rename = "window.title-page.open")]
    WindowTitlePageOpen,
    #[serde(rename = "window.title-page.close")]
    WindowTitlePageClose,
    #[serde(rename = "editor.style.toggle")]
    EditorStyleToggle {
        surface: EditorSurface,
        style: TitleTextStyle,
    },
    #[serde(rename = "editor.alignment.set")]
    EditorAlignmentSet {
        surface: EditorSurface,
        alignment: ParagraphAlignmentValue,
    },
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
    #[serde(rename = "editor.dual-dialogue.toggle")]
    EditorDualDialogueToggle,
    #[serde(rename = "editor.style.clear")]
    EditorStyleClear { surface: EditorSurface },
    #[serde(rename = "application.quit")]
    ApplicationQuit,
    #[serde(rename = "view.script-view.set")]
    ViewScriptViewSet { mode: ScriptViewMode },
    #[serde(rename = "format.elements.open")]
    FormatElementsOpen,
    #[serde(rename = "format.elements.close")]
    FormatElementsClose,
    #[serde(rename = "format.template-selector.open")]
    FormatTemplateSelectorOpen,
    #[serde(rename = "format.template-selector.cancel")]
    FormatTemplateSelectorCancel,
    #[serde(rename = "format.template.apply")]
    FormatTemplateApply { template: BuiltinTemplateValue },
    #[serde(rename = "file.new-from-template.open")]
    FileNewFromTemplateOpen,
    #[serde(rename = "file.new-from-template")]
    FileNewFromTemplate { template: BuiltinTemplateValue },
}

impl Command {
    pub const fn host_effect_description(&self) -> Option<HostEffectDescription> {
        match self {
            Self::FileSaveAsPdf => Some(HostEffectDescription::OpenPdfDestinationPicker),
            Self::FilePrint => Some(HostEffectDescription::OpenPrintPanel),
            Self::ViewFocusModeToggle => Some(HostEffectDescription::ToggleFocusModeViewport),
            Self::EditorFocus => Some(HostEffectDescription::FocusActiveEditor),
            Self::WindowTitlePageOpen => Some(HostEffectDescription::OpenTitlePageWindow),
            Self::WindowTitlePageClose => Some(HostEffectDescription::CloseTitlePageWindow),
            Self::PdfIncludeTitlePageResolve { options } => {
                Some(HostEffectDescription::ResolvePdfIncludeTitlePage(*options))
            }
            Self::FormatElementsOpen => Some(HostEffectDescription::OpenElementsWindow),
            Self::FormatElementsClose => Some(HostEffectDescription::CloseElementsWindow),
            Self::FormatTemplateSelectorOpen => Some(HostEffectDescription::OpenTemplateSelector),
            Self::FormatTemplateSelectorCancel => {
                Some(HostEffectDescription::CancelTemplateSelector)
            }
            Self::FileNewFromTemplateOpen => Some(HostEffectDescription::OpenNewTemplateSelector),
            Self::FileNewFromTemplate { template } => Some(
                HostEffectDescription::LaunchIndependentTemplateChild(*template),
            ),
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
            Self::FilePrint => CommandId::FilePrint,
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
            Self::EditorDualDialogueToggle => CommandId::EditorDualDialogueToggle,
            Self::EditorStyleClear { .. } => CommandId::EditorStyleClear,
            Self::ApplicationQuit => CommandId::ApplicationQuit,
            Self::ViewScriptViewSet { .. } => CommandId::ViewScriptViewSet,
            Self::FormatElementsOpen => CommandId::FormatElementsOpen,
            Self::FormatElementsClose => CommandId::FormatElementsClose,
            Self::FormatTemplateSelectorOpen => CommandId::FormatTemplateSelectorOpen,
            Self::FormatTemplateSelectorCancel => CommandId::FormatTemplateSelectorCancel,
            Self::FormatTemplateApply { .. } => CommandId::FormatTemplateApply,
            Self::FileNewFromTemplateOpen => CommandId::FileNewFromTemplateOpen,
            Self::FileNewFromTemplate { .. } => CommandId::FileNewFromTemplate,
        }
    }
}

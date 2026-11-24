use std::fmt;

use super::CommandId;

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

const fn descriptor(
    id: CommandId,
    family: &'static str,
    parameters: &'static str,
) -> CommandDescriptor {
    CommandDescriptor {
        id,
        family,
        parameters,
        availability: AvailabilityPolicy::RecheckAtExecution,
    }
}

/// Authoritative discovery catalog; disabled placeholders are deliberately absent.
pub static COMMAND_DESCRIPTORS: [CommandDescriptor; 75] = [
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
    descriptor(
        CommandId::FilePrint,
        "file",
        "none; native print host effect (macOS panel or Windows print session)",
    ),
    descriptor(
        CommandId::FileImportResolve,
        "file",
        "resolution: import_resolution",
    ),
    descriptor(CommandId::EditUndo, "edit", "none"),
    descriptor(CommandId::EditRedo, "edit", "none"),
    descriptor(CommandId::EditCut, "edit", "none"),
    descriptor(CommandId::EditCopy, "edit", "none"),
    descriptor(CommandId::EditPaste, "edit", "none"),
    descriptor(CommandId::EditDelete, "edit", "none"),
    descriptor(CommandId::EditSelectAll, "edit", "none"),
    descriptor(
        CommandId::EditorTextInsert,
        "editor",
        "text: string, surface: editor_surface (default body)",
    ),
    descriptor(
        CommandId::EditorTextPaste,
        "editor",
        "text: string, surface: editor_surface (default body)",
    ),
    descriptor(
        CommandId::EditorDeleteBackward,
        "editor",
        "none; host active surface",
    ),
    descriptor(
        CommandId::EditorParagraphEnter,
        "editor",
        "none; host active surface",
    ),
    descriptor(
        CommandId::EditorParagraphTab,
        "editor",
        "none; host active surface",
    ),
    descriptor(
        CommandId::EditorElementChange,
        "editor",
        "element_type: element_type_key (exact open label), surface: editor_surface (default body); v1 role envelope compatibility",
    ),
    descriptor(
        CommandId::EditorSelectionSet,
        "editor",
        "selection: utf8_byte_positions, surface: editor_surface (default body)",
    ),
    descriptor(
        CommandId::EditorSelectionPointer,
        "editor",
        "position, phase, surface: editor_surface (default body)",
    ),
    descriptor(
        CommandId::EditorNavigationHorizontal,
        "editor",
        "direction, extend, surface: editor_surface (default body)",
    ),
    descriptor(
        CommandId::EditorNavigationVertical,
        "editor",
        "direction, extend, surface: editor_surface (default body)",
    ),
    descriptor(
        CommandId::EditorNavigationLineEdge,
        "editor",
        "edge, extend, surface: editor_surface (default body)",
    ),
    descriptor(CommandId::EditorFocus, "editor", "none"),
    descriptor(
        CommandId::WindowTitlePageOpen,
        "window",
        "none; host viewport effect",
    ),
    descriptor(
        CommandId::WindowTitlePageClose,
        "window",
        "none; host viewport effect",
    ),
    descriptor(
        CommandId::EditorStyleToggle,
        "editor",
        "surface, style: bounded_text_style",
    ),
    descriptor(
        CommandId::EditorAlignmentSet,
        "editor",
        "surface, alignment: bounded_alignment",
    ),
    descriptor(
        CommandId::EditorRunFormattingSet,
        "editor",
        "surface: body, patch: bounded_font_size_color_patch",
    ),
    descriptor(
        CommandId::EditorParagraphFormattingSet,
        "editor",
        "surface: body, patch: bounded_spacing_space_before_leading_patch",
    ),
    descriptor(
        CommandId::PdfIncludeTitlePageResolve,
        "pdf",
        "options: pdf_export_options; host dialog effect",
    ),
    descriptor(CommandId::ViewZoomSet, "view", "percent: 5..=300; active target enforces Script/Title 75..=300 and Beat 5..=300"),
    descriptor(
        CommandId::ViewZoomPinch,
        "view",
        "factor: finite_positive_f32",
    ),
    descriptor(
        CommandId::ViewAppearanceSet,
        "view",
        "appearance: appearance",
    ),
    descriptor(CommandId::ViewFocusModeToggle, "view", "none"),
    descriptor(CommandId::ToolsSprintTimerTogglePopup, "tools", "none"),
    descriptor(CommandId::ToolsSprintTimerClosePopup, "tools", "none"),
    descriptor(
        CommandId::ToolsSprintTimerSetMode,
        "tools",
        "mode: sprint_timer_mode",
    ),
    descriptor(
        CommandId::ToolsSprintTimerSetMinutes,
        "tools",
        "text: string",
    ),
    descriptor(CommandId::ToolsSprintTimerStart, "tools", "none"),
    descriptor(CommandId::ToolsSprintTimerPause, "tools", "none"),
    descriptor(CommandId::ToolsSprintTimerResume, "tools", "none"),
    descriptor(CommandId::ToolsSprintTimerReset, "tools", "none"),
    descriptor(CommandId::ToolsSprintTimerEnd, "tools", "none"),
    descriptor(
        CommandId::UiStatusSet,
        "ui",
        "message: optional diagnostic_message (<=512 UTF-8 bytes, single-line)",
    ),
    descriptor(
        CommandId::EditorDualDialogueToggle,
        "editor",
        "none; Body only; host active surface",
    ),
    descriptor(
        CommandId::EditorStyleClear,
        "editor",
        "surface: editor_surface; clear supported run styles",
    ),
    descriptor(
        CommandId::ApplicationQuit,
        "application",
        "none; requests guarded root Close",
    ),
    descriptor(
        CommandId::ViewScriptViewSet,
        "view",
        "mode: script_view_mode (normal, page, speed)",
    ),
    descriptor(CommandId::FormatElementsOpen, "format", "none; host effect"),
    descriptor(CommandId::FormatElementsClose, "format", "none; host effect"),
    descriptor(
        CommandId::FormatTemplateSelectorOpen,
        "format",
        "none; host effect",
    ),
    descriptor(
        CommandId::FormatTemplateSelectorCancel,
        "format",
        "none; host effect",
    ),
    descriptor(
        CommandId::FormatTemplateApply,
        "format",
        "template: builtin_template_value",
    ),
    descriptor(
        CommandId::FileNewFromTemplateOpen,
        "file",
        "none; host effect opens New-document selector",
    ),
    descriptor(
        CommandId::FileNewFromTemplate,
        "file",
        "template: builtin_template_value; host effect launches independent child",
    ),
    descriptor(
        CommandId::ViewInvisiblesToggle,
        "view",
        "none; toggle active editor-surface presentation",
    ),
    descriptor(
        CommandId::ViewRulerToggle,
        "view",
        "none; desktop-only global presentation/settings effect",
    ),
    descriptor(CommandId::ViewBeatBoardShow, "view", "none; desktop-only root view effect"),
    descriptor(CommandId::BeatBoardCardCreate, "beat-board", "position: milli-pixel point"),
    descriptor(CommandId::BeatBoardCardActivate, "beat-board", "card: stable card ID"),
    descriptor(CommandId::BeatBoardCardGeometrySet, "beat-board", "card, rect: milli-pixel rectangle, activate: bool (default false)"),
    descriptor(CommandId::BeatBoardSelectionSet, "beat-board", "selection: none, card, or validated title/body text range"),
    descriptor(CommandId::BeatBoardTextEdit, "beat-board", "operation: insert, paste, delete_backward, delete_forward, or enter; active Beat text selection"),
    descriptor(CommandId::BeatBoardNavigation, "beat-board", "target: horizontal direction or logical paragraph edge; extend: bool"),
    descriptor(CommandId::BeatBoardStyleToggle, "beat-board", "style: bold, italics, underline, all_caps, or strikethrough; active Body selection/caret"),
    descriptor(CommandId::BeatBoardStyleClear, "beat-board", "none; clear supported Body run styles"),
    descriptor(CommandId::BeatBoardRunFormattingSet, "beat-board", "patch: bounded Font/Size/Color/Background; active Body selection/caret"),
    descriptor(CommandId::BeatBoardCardDelete, "beat-board", "card: stable card ID"),
    descriptor(CommandId::FileExportFountain, "file", "none; host effect opens Fountain destination picker"),
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
        if !COMMAND_DESCRIPTORS
            .iter()
            .any(|descriptor| descriptor.id == id)
        {
            return Err(CatalogError::MissingDescriptor(id));
        }
    }
    Ok(())
}

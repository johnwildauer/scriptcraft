//! Declarative, presentation-independent desktop menu definitions.
//!
//! Menu IDs are stable integration identities. Labels/accessibility names,
//! semantic icon hints, display-only shortcuts, and behavior identity live in
//! one catalog. Availability remains a fresh application-state projection for
//! real commands; placeholders are intrinsically disabled and never dispatch.

use super::shortcuts::{display_shortcut, primary_shortcut};
pub(super) use super::shortcuts::{LogicalShortcut, ShortcutKey};
use scriptcraft_commands::CommandId;
use scriptcraft_commands::{
    Appearance, Command, EditorSurface, InlineColorValue, InlineFontValue, InlineSizeValue,
    ParagraphAlignmentValue, ParagraphFormattingPatchValue, ParagraphLeadingValue,
    ParagraphSpacingValue, RunFormattingPatchValue, ScriptViewMode, SpaceBeforeLines,
    TitleTextStyle, ZoomPercent,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MenuSpec {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) accessibility_name: &'static str,
    pub(crate) entries: &'static [MenuEntrySpec],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuEntrySpec {
    Item(MenuItemSpec),
    Separator,
    Submenu(&'static MenuSpec),
    DynamicSubmenu(DynamicSubmenuKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DynamicSubmenuKind {
    ChangeElementTo,
    Zoom,
}

impl DynamicSubmenuKind {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::ChangeElementTo => "format.change-element",
            Self::Zoom => "view.zoom",
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::ChangeElementTo => "Change Element To",
            Self::Zoom => "Zoom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuItemBehavior {
    Command(CommandId),
    MenuCommand(MenuAction),
    DisabledPlaceholder(PlaceholderKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuAction {
    ToggleStyle(TitleTextStyle),
    ClearStyles,
    SetAlignment(ParagraphAlignmentValue),
    SetScriptView(ScriptViewMode),
    SetInlineFont(InlineFontValue),
    SetInlineSize(InlineSizeValue),
    ClearInlineSize,
    SetInlineColor(InlineColorValue),
    SetInlineBackground(InlineColorValue),
    SetParagraphSpacing(ParagraphSpacingValue),
    SetSpaceBefore(SpaceBeforeLines),
    SetParagraphLeading(ParagraphLeadingValue),
    SetAppearance(Appearance),
    SetZoom(u16),
    ZoomStep(ZoomStepDirection),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZoomStepDirection {
    In,
    Out,
}

#[derive(Debug, Clone)]
pub(crate) enum MenuActivation {
    Command(Command),
    ZoomStep(ZoomStepDirection),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ViewMenuProjection {
    pub(crate) zoom_percent: u16,
    pub(crate) appearance_mode: super::theme::AppearanceMode,
    pub(crate) beat_board: bool,
}

impl Default for ViewMenuProjection {
    fn default() -> Self {
        Self {
            zoom_percent: 100,
            appearance_mode: super::theme::AppearanceMode::System,
            beat_board: false,
        }
    }
}

pub(crate) fn activation_for_action(action: MenuAction, surface: EditorSurface) -> MenuActivation {
    match action {
        MenuAction::ZoomStep(direction) => MenuActivation::ZoomStep(direction),
        action => MenuActivation::Command(command_for_action(action, surface)),
    }
}

pub(crate) fn command_for_action(action: MenuAction, surface: EditorSurface) -> Command {
    match action {
        MenuAction::ToggleStyle(style) => Command::EditorStyleToggle { surface, style },
        MenuAction::ClearStyles => Command::EditorStyleClear { surface },
        MenuAction::SetAlignment(alignment) => Command::EditorAlignmentSet { surface, alignment },
        MenuAction::SetScriptView(mode) => Command::ViewScriptViewSet { mode },
        MenuAction::SetAppearance(appearance) => Command::ViewAppearanceSet { appearance },
        MenuAction::SetZoom(percent) => Command::ViewZoomSet {
            percent: ZoomPercent::new(percent).expect("menu zoom preset is bounded"),
        },
        MenuAction::ZoomStep(_) => unreachable!("zoom steps are deferred until activation"),
        MenuAction::SetInlineFont(font) => Command::EditorRunFormattingSet {
            surface,
            patch: RunFormattingPatchValue { font: Some(font), ..RunFormattingPatchValue::default() },
        },
        MenuAction::SetInlineSize(size) => Command::EditorRunFormattingSet {
            surface,
            patch: RunFormattingPatchValue { size: Some(size), ..RunFormattingPatchValue::default() },
        },
        MenuAction::ClearInlineSize => Command::EditorRunFormattingSet {
            surface,
            patch: RunFormattingPatchValue { clear_size: true, ..RunFormattingPatchValue::default() },
        },
        MenuAction::SetInlineColor(color) => Command::EditorRunFormattingSet {
            surface,
            patch: RunFormattingPatchValue { color: Some(color), ..RunFormattingPatchValue::default() },
        },
        MenuAction::SetInlineBackground(background) => Command::EditorRunFormattingSet {
            surface,
            patch: RunFormattingPatchValue { background: Some(background), ..RunFormattingPatchValue::default() },
        },
        MenuAction::SetParagraphSpacing(spacing) => Command::EditorParagraphFormattingSet {
            surface,
            patch: ParagraphFormattingPatchValue { spacing: Some(spacing), ..ParagraphFormattingPatchValue::default() },
        },
        MenuAction::SetSpaceBefore(lines) => Command::EditorParagraphFormattingSet {
            surface,
            patch: ParagraphFormattingPatchValue { space_before_lines: Some(lines), ..ParagraphFormattingPatchValue::default() },
        },
        MenuAction::SetParagraphLeading(leading) => Command::EditorParagraphFormattingSet {
            surface,
            patch: ParagraphFormattingPatchValue { leading: Some(leading), ..ParagraphFormattingPatchValue::default() },
        },
    }
}

/// Projects the checked appearance for a menu item from current application
/// state while retaining the catalog's unrelated static checked placeholder.
pub(crate) fn menu_item_is_checked(
    item: &MenuItemSpec,
    script_view_mode: ScriptViewMode,
    formatting: &super::formatting_menu::FormattingMenuProjection,
    view: ViewMenuProjection,
) -> bool {
    match item.behavior {
        MenuItemBehavior::MenuCommand(MenuAction::SetScriptView(mode)) => !view.beat_board && mode == script_view_mode,
        MenuItemBehavior::Command(CommandId::ViewBeatBoardShow) => view.beat_board,
        MenuItemBehavior::MenuCommand(MenuAction::SetAppearance(appearance)) => {
            appearance_mode_for_choice(appearance) == view.appearance_mode
        }
        MenuItemBehavior::MenuCommand(MenuAction::SetZoom(percent)) => {
            percent == view.zoom_percent
        }
        MenuItemBehavior::MenuCommand(action) => formatting.is_checked(action),
        MenuItemBehavior::DisabledPlaceholder(PlaceholderKind::Checked) => true,
        _ => false,
    }
}

/// Whether this catalog item can display a check in any presentation state.
/// This is deliberately separate from the current-state projection above so a
/// popup's leading slot does not move as selection state changes.
pub(crate) fn menu_item_can_display_check(item: &MenuItemSpec) -> bool {
    match item.behavior {
        MenuItemBehavior::Command(CommandId::ViewBeatBoardShow) => true,
        MenuItemBehavior::DisabledPlaceholder(PlaceholderKind::Checked) => true,
        MenuItemBehavior::MenuCommand(
            MenuAction::SetScriptView(_)
            | MenuAction::SetAppearance(_)
            | MenuAction::SetZoom(_)
            | MenuAction::ToggleStyle(_)
            | MenuAction::ClearStyles
            | MenuAction::SetAlignment(_)
            | MenuAction::SetInlineFont(_)
            | MenuAction::SetInlineSize(_)
            | MenuAction::ClearInlineSize
            | MenuAction::SetInlineColor(_)
            | MenuAction::SetInlineBackground(_)
            | MenuAction::SetParagraphSpacing(_)
            | MenuAction::SetSpaceBefore(_)
            | MenuAction::SetParagraphLeading(_),
        ) => true,
        MenuItemBehavior::Command(_) | MenuItemBehavior::MenuCommand(MenuAction::ZoomStep(_))
        | MenuItemBehavior::DisabledPlaceholder(_) => false,
    }
}

fn appearance_mode_for_choice(appearance: Appearance) -> super::theme::AppearanceMode {
    match appearance {
        Appearance::System => super::theme::AppearanceMode::System,
        Appearance::Day => super::theme::AppearanceMode::Day,
        Appearance::Night => super::theme::AppearanceMode::Night,
        Appearance::Midnight => super::theme::AppearanceMode::Midnight,
    }
}

pub(crate) fn menu_item_label(
    item: &MenuItemSpec,
    say_undual: bool,
    invisibles_visible: bool,
    ruler_visible: bool,
) -> &'static str {
    match item.behavior {
        MenuItemBehavior::Command(CommandId::EditorDualDialogueToggle) if say_undual => {
            "Undual Dialogue"
        }
        MenuItemBehavior::Command(CommandId::ViewInvisiblesToggle) if invisibles_visible => {
            "Hide Invisibles"
        }
        MenuItemBehavior::Command(CommandId::ViewInvisiblesToggle) => "Show Invisibles",
        MenuItemBehavior::Command(CommandId::ViewRulerToggle) if ruler_visible => "Hide Ruler",
        MenuItemBehavior::Command(CommandId::ViewRulerToggle) => "Show Ruler",
        _ => item.label,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlaceholderKind {
    Item,
    Disclosure,
    Checked,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MenuItemSpec {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) accessibility_name: &'static str,
    pub(crate) icon: Option<SemanticIcon>,
    pub(crate) swatch: Option<MenuSwatch>,
    pub(crate) shortcut: Option<LogicalShortcut>,
    /// Platform-specific alternate chord where the primary shortcut differs.
    pub(crate) windows_alias: Option<LogicalShortcut>,
    pub(crate) behavior: MenuItemBehavior,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuSwatch {
    Palette([u8; 3]),
    NoColor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SemanticIcon {
    DocumentNew,
    FolderOpen,
    WindowClose,
    Save,
    SaveAs,
    PdfExport,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Delete,
    SelectAll,
}

const fn command_item(
    id: &'static str,
    label: &'static str,
    accessibility_name: &'static str,
    icon: Option<SemanticIcon>,
    shortcut: Option<LogicalShortcut>,
    windows_alias: Option<LogicalShortcut>,
    command: CommandId,
) -> MenuEntrySpec {
    MenuEntrySpec::Item(MenuItemSpec {
        id,
        label,
        accessibility_name,
        icon,
        swatch: None,
        shortcut,
        windows_alias,
        behavior: MenuItemBehavior::Command(command),
    })
}

const fn placeholder(
    id: &'static str,
    label: &'static str,
    accessibility_name: &'static str,
    kind: PlaceholderKind,
    shortcut: Option<LogicalShortcut>,
) -> MenuEntrySpec {
    MenuEntrySpec::Item(MenuItemSpec {
        id,
        label,
        accessibility_name,
        icon: None,
        swatch: None,
        shortcut,
        windows_alias: None,
        behavior: MenuItemBehavior::DisabledPlaceholder(kind),
    })
}

const fn menu_command_item(
    id: &'static str,
    label: &'static str,
    shortcut: Option<LogicalShortcut>,
    action: MenuAction,
) -> MenuEntrySpec {
    MenuEntrySpec::Item(MenuItemSpec {
        id,
        label,
        accessibility_name: label,
        icon: None,
        swatch: None,
        shortcut,
        windows_alias: None,
        behavior: MenuItemBehavior::MenuCommand(action),
    })
}

const SEP: MenuEntrySpec = MenuEntrySpec::Separator;
const ITEM: PlaceholderKind = PlaceholderKind::Item;
const DISCLOSURE: PlaceholderKind = PlaceholderKind::Disclosure;
const CHECKED: PlaceholderKind = PlaceholderKind::Checked;
const SEARCH: PlaceholderKind = PlaceholderKind::Search;

pub(crate) const ZOOM_IN_ITEM: MenuItemSpec = MenuItemSpec {
    id: "view.zoom.in",
    label: "Zoom In",
    accessibility_name: "Zoom In",
    icon: None,
    swatch: None,
    shortcut: Some(primary_shortcut(ShortcutKey::Plus, true)),
    windows_alias: Some(primary_shortcut(ShortcutKey::Equals, false)),
    behavior: MenuItemBehavior::MenuCommand(MenuAction::ZoomStep(ZoomStepDirection::In)),
};

pub(crate) const ZOOM_OUT_ITEM: MenuItemSpec = MenuItemSpec {
    id: "view.zoom.out",
    label: "Zoom Out",
    accessibility_name: "Zoom Out",
    icon: None,
    swatch: None,
    shortcut: Some(primary_shortcut(ShortcutKey::Minus, false)),
    windows_alias: None,
    behavior: MenuItemBehavior::MenuCommand(MenuAction::ZoomStep(ZoomStepDirection::Out)),
};

pub(crate) const ZOOM_STEP_ITEMS: [MenuItemSpec; 2] = [ZOOM_IN_ITEM, ZOOM_OUT_ITEM];

const APPEARANCE_ENTRIES: &[MenuEntrySpec] = &[
    menu_command_item(
        "view.appearance.system",
        "System",
        None,
        MenuAction::SetAppearance(Appearance::System),
    ),
    menu_command_item(
        "view.appearance.day",
        "Day",
        None,
        MenuAction::SetAppearance(Appearance::Day),
    ),
    menu_command_item(
        "view.appearance.night",
        "Night",
        None,
        MenuAction::SetAppearance(Appearance::Night),
    ),
    menu_command_item(
        "view.appearance.midnight",
        "Midnight",
        None,
        MenuAction::SetAppearance(Appearance::Midnight),
    ),
];

const APPEARANCE_MENU: MenuSpec = MenuSpec {
    id: "view.appearance",
    label: "Appearance",
    accessibility_name: "Appearance",
    entries: APPEARANCE_ENTRIES,
};

#[cfg(any(target_os = "macos", target_os = "windows"))]
const PRINT_ENTRY: MenuEntrySpec = command_item(
    "file.print",
    "Print...",
    "Print screenplay",
    None,
    Some(primary_shortcut(ShortcutKey::P, false)),
    None,
    CommandId::FilePrint,
);

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const PRINT_ENTRY: MenuEntrySpec = placeholder(
    "file.print",
    "Print...",
    "Print (disabled placeholder)",
    ITEM,
    Some(primary_shortcut(ShortcutKey::P, false)),
);

const EXPORT_ENTRIES: &[MenuEntrySpec] = &[command_item(
    "file.export-fountain",
    "Fountain Text...",
    "Export Fountain text",
    None,
    None,
    None,
    CommandId::FileExportFountain,
)];

const EXPORT_MENU: MenuSpec = MenuSpec {
    id: "file.export",
    label: "Export",
    accessibility_name: "Export",
    entries: EXPORT_ENTRIES,
};

const FILE_ENTRIES: &[MenuEntrySpec] = &[
    command_item(
        "file.new",
        "New",
        "New document",
        Some(SemanticIcon::DocumentNew),
        Some(primary_shortcut(ShortcutKey::N, false)),
        None,
        CommandId::FileNew,
    ),
    command_item(
        "file.new-from-template",
        "New from Template...",
        "New document from a built-in template",
        None,
        Some(primary_shortcut(ShortcutKey::N, true)),
        None,
        CommandId::FileNewFromTemplateOpen,
    ),
    command_item(
        "file.open",
        "Open...",
        "Open document",
        Some(SemanticIcon::FolderOpen),
        Some(primary_shortcut(ShortcutKey::O, false)),
        None,
        CommandId::FileOpenRequest,
    ),
    placeholder(
        "file.open-recent",
        "Open Recent",
        "Open Recent submenu placeholder",
        DISCLOSURE,
        None,
    ),
    command_item(
        "file.close",
        "Close",
        "Close window",
        Some(SemanticIcon::WindowClose),
        Some(primary_shortcut(ShortcutKey::W, false)),
        None,
        CommandId::FileClose,
    ),
    SEP,
    command_item(
        "file.save",
        "Save",
        "Save document",
        Some(SemanticIcon::Save),
        Some(primary_shortcut(ShortcutKey::S, false)),
        None,
        CommandId::FileSave,
    ),
    command_item(
        "file.save-as",
        "Save As...",
        "Save document as",
        Some(SemanticIcon::SaveAs),
        Some(primary_shortcut(ShortcutKey::S, true)),
        None,
        CommandId::FileSaveAsRequest,
    ),
    command_item(
        "file.save-as-pdf",
        "Save As PDF...",
        "Save document as PDF",
        Some(SemanticIcon::PdfExport),
        Some(primary_shortcut(ShortcutKey::P, true)),
        None,
        CommandId::FileSaveAsPdf,
    ),
    MenuEntrySpec::Submenu(&EXPORT_MENU),
    placeholder(
        "file.backup",
        "Backup...",
        "Backup (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "file.revert",
        "Revert",
        "Revert (disabled placeholder)",
        ITEM,
        None,
    ),
    SEP,
    placeholder(
        "file.save-as-template",
        "Save as Template...",
        "Save as template (disabled placeholder)",
        ITEM,
        None,
    ),
    SEP,
    placeholder(
        "file.print-preview",
        "Print Preview",
        "Print Preview (disabled placeholder)",
        ITEM,
        None,
    ),
    PRINT_ENTRY,
    SEP,
    placeholder(
        "file.register-script",
        "Register Script",
        "Register Script (disabled placeholder)",
        ITEM,
        None,
    ),
];

const EDIT_ENTRIES: &[MenuEntrySpec] = &[
    command_item(
        "edit.undo",
        "Undo",
        "Undo last action",
        Some(SemanticIcon::Undo),
        Some(primary_shortcut(ShortcutKey::Z, false)),
        None,
        CommandId::EditUndo,
    ),
    command_item(
        "edit.redo",
        "Redo",
        "Redo last action",
        Some(SemanticIcon::Redo),
        Some(primary_shortcut(ShortcutKey::Z, true)),
        Some(primary_shortcut(ShortcutKey::Y, false)),
        CommandId::EditRedo,
    ),
    SEP,
    command_item(
        "edit.cut",
        "Cut",
        "Cut selected text",
        Some(SemanticIcon::Cut),
        Some(primary_shortcut(ShortcutKey::X, false)),
        None,
        CommandId::EditCut,
    ),
    command_item(
        "edit.copy",
        "Copy",
        "Copy selected text",
        Some(SemanticIcon::Copy),
        Some(primary_shortcut(ShortcutKey::C, false)),
        None,
        CommandId::EditCopy,
    ),
    command_item(
        "edit.paste",
        "Paste",
        "Paste clipboard text",
        Some(SemanticIcon::Paste),
        Some(primary_shortcut(ShortcutKey::V, false)),
        None,
        CommandId::EditPaste,
    ),
    SEP,
    command_item(
        "edit.delete",
        "Delete",
        "Delete forward",
        Some(SemanticIcon::Delete),
        None,
        None,
        CommandId::EditDelete,
    ),
    command_item(
        "edit.select-all",
        "Select All",
        "Select all screenplay text",
        Some(SemanticIcon::SelectAll),
        Some(primary_shortcut(ShortcutKey::A, false)),
        None,
        CommandId::EditSelectAll,
    ),
    placeholder(
        "edit.select-scene",
        "Select Scene",
        "Select Scene (disabled placeholder)",
        ITEM,
        Some(display_shortcut(ShortcutKey::A, false, false, false, true)),
    ),
    SEP,
    placeholder(
        "edit.track-changes",
        "Track Changes",
        "Track Changes submenu placeholder",
        DISCLOSURE,
        None,
    ),
    SEP,
    placeholder(
        "edit.find",
        "Find...",
        "Find (disabled placeholder)",
        ITEM,
        Some(primary_shortcut(ShortcutKey::F, false)),
    ),
    placeholder(
        "edit.go-to",
        "Go To...",
        "Go To (disabled placeholder)",
        ITEM,
        Some(primary_shortcut(ShortcutKey::G, false)),
    ),
    placeholder(
        "edit.find-selection",
        "Find Selection",
        "Find Selection (disabled placeholder)",
        ITEM,
        Some(primary_shortcut(ShortcutKey::F, true)),
    ),
    placeholder(
        "edit.rename-character",
        "Rename Character...",
        "Rename Character (disabled placeholder)",
        ITEM,
        None,
    ),
    SEP,
    placeholder(
        "edit.writing-tools",
        "Writing Tools",
        "Writing Tools submenu placeholder",
        DISCLOSURE,
        None,
    ),
    placeholder(
        "edit.autofill",
        "AutoFill",
        "AutoFill submenu placeholder",
        DISCLOSURE,
        None,
    ),
    placeholder(
        "edit.start-dictation",
        "Start Dictation",
        "Start Dictation (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "edit.emoji-symbols",
        "Emoji & Symbols",
        "Emoji & Symbols (disabled placeholder)",
        ITEM,
        None,
    ),
];

const VIEW_ENTRIES: &[MenuEntrySpec] = &[
    placeholder(
        "view.split",
        "Split",
        "Split submenu placeholder",
        DISCLOSURE,
        None,
    ),
    SEP,
    menu_command_item(
        "view.script-normal",
        "Script - Normal View",
        None,
        MenuAction::SetScriptView(ScriptViewMode::Normal),
    ),
    menu_command_item(
        "view.script-page",
        "Script - Page View",
        None,
        MenuAction::SetScriptView(ScriptViewMode::Page),
    ),
    menu_command_item(
        "view.script-speed",
        "Script - Speed View",
        None,
        MenuAction::SetScriptView(ScriptViewMode::Speed),
    ),
    SEP,
    command_item(
        "view.beat-board",
        "Beat Board",
        "Beat Board",
        None,
        None,
        None,
        CommandId::ViewBeatBoardShow,
    ),
    SEP,
    placeholder(
        "view.scene-view",
        "Scene View",
        "Scene View (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "view.scene-view-visibility",
        "Show/Hide Scene View",
        "Show/Hide Scene View submenu placeholder",
        DISCLOSURE,
        None,
    ),
    SEP,
    placeholder(
        "view.index-cards-script",
        "Index Cards - Script",
        "Index Cards - Script (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "view.index-cards-summary",
        "Index Cards - Summary",
        "Index Cards - Summary (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "view.cards-across",
        "Cards Across",
        "Cards Across submenu placeholder",
        DISCLOSURE,
        None,
    ),
    SEP,
    placeholder(
        "view.outline-editor",
        "Show Outline Editor",
        "Show Outline Editor (disabled placeholder)",
        ITEM,
        Some(primary_shortcut(ShortcutKey::M, true)),
    ),
    command_item(
        "view.ruler",
        "Show Ruler",
        "Show Ruler",
        None,
        None,
        None,
        CommandId::ViewRulerToggle,
    ),
    SEP,
    MenuEntrySpec::Submenu(&VIEW_DOCUMENT_MENU),
    SEP,
    MenuEntrySpec::DynamicSubmenu(DynamicSubmenuKind::Zoom),
    placeholder(
        "view.zoom-outline-editor",
        "Zoom Outline Editor",
        "Zoom Outline Editor submenu placeholder",
        DISCLOSURE,
        None,
    ),
    SEP,
    placeholder(
        "view.typewriter",
        "Typewriter",
        "Typewriter submenu placeholder",
        DISCLOSURE,
        None,
    ),
    SEP,
    MenuEntrySpec::Submenu(&APPEARANCE_MENU),
    SEP,
    placeholder(
        "view.tab-bar",
        "Show Tab Bar",
        "Show Tab Bar (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "view.all-tabs",
        "Show All Tabs",
        "Show All Tabs (disabled placeholder)",
        ITEM,
        Some(display_shortcut(
            ShortcutKey::Backslash,
            true,
            false,
            false,
            true,
        )),
    ),
    SEP,
    placeholder(
        "view.focus-mode",
        "Focus Mode",
        "Focus Mode (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "view.full-screen",
        "Enter Full Screen",
        "Enter Full Screen (disabled placeholder)",
        ITEM,
        Some(display_shortcut(ShortcutKey::F, true, true, false, false)),
    ),
    SEP,
    placeholder(
        "view.hide-toolbar",
        "Hide Toolbar",
        "Hide Toolbar (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "view.customize-toolbar",
        "Customize Toolbar...",
        "Customize Toolbar (disabled placeholder)",
        ITEM,
        None,
    ),
];

const VIEW_DOCUMENT_ENTRIES: &[MenuEntrySpec] = &[command_item(
    "view.document.invisibles",
    "Show Invisibles",
    "Show Invisibles",
    None,
    Some(primary_shortcut(ShortcutKey::J, false)),
    None,
    CommandId::ViewInvisiblesToggle,
)];

const VIEW_DOCUMENT_MENU: MenuSpec = MenuSpec {
    id: "view.document",
    label: "Document",
    accessibility_name: "Document",
    entries: VIEW_DOCUMENT_ENTRIES,
};

const FORMAT_ENTRIES: &[MenuEntrySpec] = &[
    command_item(
        "format.elements",
        "Elements...",
        "Elements",
        None,
        Some(primary_shortcut(ShortcutKey::E, false)),
        None,
        CommandId::FormatElementsOpen,
    ),
    MenuEntrySpec::DynamicSubmenu(DynamicSubmenuKind::ChangeElementTo),
    placeholder(
        "format.cast-list-options",
        "Cast List Element Options...",
        "Cast List Element Options (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "format.highlight-characters",
        "Highlight Characters...",
        "Highlight Characters (disabled placeholder)",
        ITEM,
        None,
    ),
    SEP,
    placeholder(
        "format.set-font",
        "Set Font...",
        "Set Font (disabled placeholder)",
        ITEM,
        None,
    ),
    MenuEntrySpec::Submenu(&FONT_MENU),
    MenuEntrySpec::Submenu(&SIZE_MENU),
    MenuEntrySpec::Submenu(&STYLE_MENU),
    MenuEntrySpec::Submenu(&COLOR_MENU),
    MenuEntrySpec::Submenu(&HIGHLIGHT_MENU),
    placeholder(
        "format.toggle-case",
        "tOGGLE cASE",
        "Toggle Case (disabled placeholder)",
        ITEM,
        None,
    ),
    SEP,
    MenuEntrySpec::Submenu(&ALIGNMENT_MENU),
    MenuEntrySpec::Submenu(&SPACING_MENU),
    MenuEntrySpec::Submenu(&SPACE_BEFORE_MENU),
    MenuEntrySpec::Submenu(&LEADING_MENU),
    SEP,
    placeholder(
        "format.beat-board",
        "Beat Board",
        "Beat Board submenu placeholder",
        DISCLOSURE,
        None,
    ),
    SEP,
    command_item(
        "format.dual-dialogue",
        "Dual Dialogue",
        "Toggle Dual Dialogue",
        None,
        Some(primary_shortcut(ShortcutKey::D, false)),
        Some(display_shortcut(ShortcutKey::D, true, false, true, false)),
        CommandId::EditorDualDialogueToggle,
    ),
];

const STYLE_ENTRIES: &[MenuEntrySpec] = &[
    menu_command_item(
        "format.style.bold",
        "Bold",
        Some(primary_shortcut(ShortcutKey::B, false)),
        MenuAction::ToggleStyle(TitleTextStyle::Bold),
    ),
    menu_command_item(
        "format.style.italics",
        "Italics",
        Some(primary_shortcut(ShortcutKey::I, false)),
        MenuAction::ToggleStyle(TitleTextStyle::Italics),
    ),
    menu_command_item(
        "format.style.underline",
        "Underline",
        Some(primary_shortcut(ShortcutKey::U, false)),
        MenuAction::ToggleStyle(TitleTextStyle::Underline),
    ),
    menu_command_item(
        "format.style.all-caps",
        "All Caps",
        Some(primary_shortcut(ShortcutKey::K, false)),
        MenuAction::ToggleStyle(TitleTextStyle::AllCaps),
    ),
    menu_command_item(
        "format.style.strikethrough",
        "Strikethrough",
        Some(primary_shortcut(ShortcutKey::X, true)),
        MenuAction::ToggleStyle(TitleTextStyle::Strikethrough),
    ),
    SEP,
    menu_command_item(
        "format.style.plain-text",
        "Plain Text",
        Some(primary_shortcut(ShortcutKey::T, false)),
        MenuAction::ClearStyles,
    ),
];

const ALIGNMENT_ENTRIES: &[MenuEntrySpec] = &[
    menu_command_item(
        "format.alignment.left",
        "Left",
        None,
        MenuAction::SetAlignment(ParagraphAlignmentValue::Left),
    ),
    menu_command_item(
        "format.alignment.center",
        "Center",
        None,
        MenuAction::SetAlignment(ParagraphAlignmentValue::Center),
    ),
    menu_command_item(
        "format.alignment.right",
        "Right",
        None,
        MenuAction::SetAlignment(ParagraphAlignmentValue::Right),
    ),
    menu_command_item(
        "format.alignment.justified",
        "Justified",
        None,
        MenuAction::SetAlignment(ParagraphAlignmentValue::Justified),
    ),
];

const FONT_ENTRIES: &[MenuEntrySpec] = &[
    menu_command_item("format.font.default", "Element Default", None, MenuAction::SetInlineFont(InlineFontValue::ElementDefault)),
    menu_command_item("format.font.courier-final-draft", "Courier Final Draft", None, MenuAction::SetInlineFont(InlineFontValue::CourierFinalDraft)),
    menu_command_item("format.font.courier-prime", "Courier Prime", None, MenuAction::SetInlineFont(InlineFontValue::CourierPrime)),
    menu_command_item("format.font.arial", "Arial", None, MenuAction::SetInlineFont(InlineFontValue::Arial)),
    menu_command_item("format.font.times-new-roman", "Times New Roman", None, MenuAction::SetInlineFont(InlineFontValue::TimesNewRoman)),
];

const SIZE_ENTRIES: &[MenuEntrySpec] = &[
    menu_command_item("format.size.default", "Element Default", None, MenuAction::ClearInlineSize),
    menu_command_item("format.size.7", "7", None, MenuAction::SetInlineSize(InlineSizeValue::Seven)),
    menu_command_item("format.size.8", "8", None, MenuAction::SetInlineSize(InlineSizeValue::Eight)),
    menu_command_item("format.size.9", "9", None, MenuAction::SetInlineSize(InlineSizeValue::Nine)),
    menu_command_item("format.size.10", "10", None, MenuAction::SetInlineSize(InlineSizeValue::Ten)),
    menu_command_item("format.size.12", "12", None, MenuAction::SetInlineSize(InlineSizeValue::Twelve)),
    menu_command_item("format.size.14", "14", None, MenuAction::SetInlineSize(InlineSizeValue::Fourteen)),
    menu_command_item("format.size.18", "18", None, MenuAction::SetInlineSize(InlineSizeValue::Eighteen)),
    menu_command_item("format.size.24", "24", None, MenuAction::SetInlineSize(InlineSizeValue::TwentyFour)),
];

const fn palette_item_id(background: bool, index: usize) -> &'static str {
    match (background, index) {
        (false, 0) => "format.color.none",
        (false, 1) => "format.color.pink",
        (false, 2) => "format.color.red",
        (false, 3) => "format.color.orange",
        (false, 4) => "format.color.yellow",
        (false, 5) => "format.color.olive",
        (false, 6) => "format.color.green",
        (false, 7) => "format.color.mint",
        (false, 8) => "format.color.blue",
        (false, 9) => "format.color.violet",
        (false, 10) => "format.color.purple",
        (false, 11) => "format.color.brown",
        (false, 12) => "format.color.gray",
        (true, 0) => "format.highlight.none",
        (true, 1) => "format.highlight.pink",
        (true, 2) => "format.highlight.red",
        (true, 3) => "format.highlight.orange",
        (true, 4) => "format.highlight.yellow",
        (true, 5) => "format.highlight.olive",
        (true, 6) => "format.highlight.green",
        (true, 7) => "format.highlight.mint",
        (true, 8) => "format.highlight.blue",
        (true, 9) => "format.highlight.violet",
        (true, 10) => "format.highlight.purple",
        (true, 11) => "format.highlight.brown",
        (true, 12) => "format.highlight.gray",
        _ => "format.color.invalid",
    }
}

const fn palette_item(background: bool, index: usize) -> MenuEntrySpec {
    use scriptcraft_model::formatted_text::InlineColor;

    if index == 0 {
        let value = InlineColorValue::ElementDefault;
        return menu_color_command_item(
            palette_item_id(background, index),
            "None",
            value,
            background,
            MenuSwatch::NoColor,
        );
    }
    let color = InlineColor::MENU_PALETTE[index - 1];
    menu_color_command_item(
        palette_item_id(background, index),
        color.label(),
        InlineColorValue::from_core(color),
        background,
        MenuSwatch::Palette(color.rgb8()),
    )
}

const fn menu_color_command_item(
    id: &'static str,
    label: &'static str,
    value: InlineColorValue,
    background: bool,
    swatch: MenuSwatch,
) -> MenuEntrySpec {
    MenuEntrySpec::Item(MenuItemSpec {
        id,
        label,
        accessibility_name: label,
        icon: None,
        swatch: Some(swatch),
        shortcut: None,
        windows_alias: None,
        behavior: MenuItemBehavior::MenuCommand(if background {
            MenuAction::SetInlineBackground(value)
        } else {
            MenuAction::SetInlineColor(value)
        }),
    })
}

const fn color_menu_entries(background: bool) -> [MenuEntrySpec; 13] {
    let mut entries = [MenuEntrySpec::Separator; 13];
    let mut index = 0;
    while index < entries.len() {
        entries[index] = palette_item(background, index);
        index += 1;
    }
    entries
}

const COLOR_ENTRIES: [MenuEntrySpec; 13] = color_menu_entries(false);
const HIGHLIGHT_ENTRIES: [MenuEntrySpec; 13] = color_menu_entries(true);

const SPACING_ENTRIES: &[MenuEntrySpec] = &[
    menu_command_item("format.spacing.single", "Single", None, MenuAction::SetParagraphSpacing(ParagraphSpacingValue::Single)),
    menu_command_item("format.spacing.one-and-half", "1.5 Lines", None, MenuAction::SetParagraphSpacing(ParagraphSpacingValue::OneAndHalf)),
    menu_command_item("format.spacing.double", "Double", None, MenuAction::SetParagraphSpacing(ParagraphSpacingValue::Double)),
];

const SPACE_BEFORE_ENTRIES: &[MenuEntrySpec] = &[
    menu_command_item("format.space-before.none", "None", None, MenuAction::SetSpaceBefore(SpaceBeforeLines::new(0).unwrap())),
    menu_command_item("format.space-before.one-line", "1 Line", None, MenuAction::SetSpaceBefore(SpaceBeforeLines::new(1).unwrap())),
    menu_command_item("format.space-before.two-lines", "2 Lines", None, MenuAction::SetSpaceBefore(SpaceBeforeLines::new(2).unwrap())),
    menu_command_item("format.space-before.three-lines", "3 Lines", None, MenuAction::SetSpaceBefore(SpaceBeforeLines::new(3).unwrap())),
];

const LEADING_ENTRIES: &[MenuEntrySpec] = &[
    menu_command_item("format.leading.very-tight", "Very Tight", None, MenuAction::SetParagraphLeading(ParagraphLeadingValue::VeryTight)),
    menu_command_item("format.leading.tight", "Tight", None, MenuAction::SetParagraphLeading(ParagraphLeadingValue::Tight)),
    menu_command_item("format.leading.regular", "Regular", None, MenuAction::SetParagraphLeading(ParagraphLeadingValue::Regular)),
    menu_command_item("format.leading.loose", "Loose", None, MenuAction::SetParagraphLeading(ParagraphLeadingValue::Loose)),
];

const STYLE_MENU: MenuSpec = MenuSpec {
    id: "format.style",
    label: "Style",
    accessibility_name: "Style",
    entries: STYLE_ENTRIES,
};
const ALIGNMENT_MENU: MenuSpec = MenuSpec {
    id: "format.alignment",
    label: "Alignment",
    accessibility_name: "Alignment",
    entries: ALIGNMENT_ENTRIES,
};
const FONT_MENU: MenuSpec = MenuSpec { id: "format.font", label: "Font", accessibility_name: "Font", entries: FONT_ENTRIES };
const SIZE_MENU: MenuSpec = MenuSpec { id: "format.size", label: "Size", accessibility_name: "Size", entries: SIZE_ENTRIES };
const COLOR_MENU: MenuSpec = MenuSpec { id: "format.color", label: "Color", accessibility_name: "Color", entries: &COLOR_ENTRIES };
const HIGHLIGHT_MENU: MenuSpec = MenuSpec { id: "format.highlight", label: "Highlight", accessibility_name: "Highlight", entries: &HIGHLIGHT_ENTRIES };
const SPACING_MENU: MenuSpec = MenuSpec { id: "format.spacing", label: "Spacing", accessibility_name: "Spacing", entries: SPACING_ENTRIES };
const SPACE_BEFORE_MENU: MenuSpec = MenuSpec { id: "format.space-before", label: "Space Before", accessibility_name: "Space Before", entries: SPACE_BEFORE_ENTRIES };
const LEADING_MENU: MenuSpec = MenuSpec { id: "format.leading", label: "Leading", accessibility_name: "Leading", entries: LEADING_ENTRIES };

const INSERT_ENTRIES: &[MenuEntrySpec] = &[
    placeholder(
        "insert.add-alt",
        "Add Alt",
        "Add Alt (disabled placeholder)",
        ITEM,
        Some(display_shortcut(
            ShortcutKey::Period,
            true,
            false,
            false,
            true,
        )),
    ),
    placeholder(
        "insert.remove-alt",
        "Remove Alt",
        "Remove Alt (disabled placeholder)",
        ITEM,
        Some(display_shortcut(
            ShortcutKey::Comma,
            true,
            false,
            false,
            true,
        )),
    ),
    SEP,
    placeholder(
        "insert.new-beat",
        "New Beat",
        "New Beat (disabled placeholder)",
        ITEM,
        Some(primary_shortcut(ShortcutKey::Enter, false)),
    ),
    SEP,
    placeholder(
        "insert.page-break",
        "Page Break",
        "Page Break (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "insert.new-scene",
        "New Scene",
        "New Scene (disabled placeholder)",
        ITEM,
        Some(display_shortcut(
            ShortcutKey::Pipe,
            true,
            false,
            false,
            true,
        )),
    ),
    SEP,
    placeholder(
        "insert.bookmark",
        "Bookmark",
        "Bookmark (disabled placeholder)",
        ITEM,
        Some(display_shortcut(
            ShortcutKey::Backslash,
            true,
            false,
            false,
            false,
        )),
    ),
    placeholder(
        "insert.script-note",
        "ScriptNote",
        "ScriptNote (disabled placeholder)",
        ITEM,
        Some(primary_shortcut(ShortcutKey::K, true)),
    ),
    placeholder(
        "insert.label",
        "Label...",
        "Label (disabled placeholder)",
        ITEM,
        Some(primary_shortcut(ShortcutKey::L, true)),
    ),
    placeholder(
        "insert.image",
        "Image...",
        "Image (disabled placeholder)",
        ITEM,
        None,
    ),
    SEP,
    placeholder(
        "insert.non-speaking-character",
        "Non-Speaking Character",
        "Non-Speaking Character (disabled placeholder)",
        ITEM,
        None,
    ),
    SEP,
    placeholder(
        "insert.send-to-script",
        "Send to Script",
        "Send to Script (disabled placeholder)",
        ITEM,
        Some(primary_shortcut(ShortcutKey::C, true)),
    ),
];

const DOCUMENT_ENTRIES: &[MenuEntrySpec] = &[
    placeholder(
        "document.page-layout",
        "Page Layout...",
        "Page Layout (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "document.header-footer",
        "Header and Footer...",
        "Header and Footer (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "document.mores-continueds",
        "Mores and Continueds...",
        "Mores and Continueds (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "document.watermark",
        "Watermark...",
        "Watermark (disabled placeholder)",
        ITEM,
        None,
    ),
    SEP,
    command_item(
        "document.title-page",
        "Title Page",
        "Title Page",
        None,
        None,
        None,
        CommandId::WindowTitlePageOpen,
    ),
    placeholder(
        "document.smarttype",
        "SmartType...",
        "SmartType (disabled placeholder)",
        ITEM,
        Some(primary_shortcut(ShortcutKey::L, false)),
    ),
    placeholder(
        "document.macros",
        "Macros...",
        "Macros (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "document.bookmarks",
        "Bookmarks...",
        "Bookmarks (disabled placeholder)",
        ITEM,
        None,
    ),
    SEP,
    placeholder(
        "document.target-script-length",
        "Change Target Script Length...",
        "Change Target Script Length (disabled placeholder)",
        ITEM,
        None,
    ),
];

const TOOLS_ENTRIES: &[MenuEntrySpec] = &[
    placeholder(
        "tools.spelling",
        "Spelling...",
        "Spelling (disabled placeholder)",
        ITEM,
        Some(display_shortcut(
            ShortcutKey::Comma,
            true,
            false,
            false,
            false,
        )),
    ),
    placeholder(
        "tools.thesaurus",
        "Thesaurus",
        "Thesaurus (disabled placeholder)",
        ITEM,
        Some(display_shortcut(
            ShortcutKey::Semicolon,
            true,
            false,
            false,
            false,
        )),
    ),
    SEP,
    placeholder(
        "tools.speech-control",
        "Speech Control...",
        "Speech Control (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "tools.assign-voices",
        "Assign Voices...",
        "Assign Voices (disabled placeholder)",
        ITEM,
        None,
    ),
    SEP,
    placeholder(
        "tools.reports",
        "Reports",
        "Reports submenu placeholder",
        DISCLOSURE,
        None,
    ),
    placeholder(
        "tools.combine-files",
        "Combine Files...",
        "Combine Files (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "tools.names-database",
        "Names Database...",
        "Names Database (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "tools.reformat",
        "Reformat...",
        "Reformat (disabled placeholder)",
        ITEM,
        Some(primary_shortcut(ShortcutKey::R, false)),
    ),
    placeholder(
        "tools.format-assistant",
        "Format Assistant...",
        "Format Assistant (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "tools.script-compare",
        "ScriptCompare...",
        "ScriptCompare (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "tools.navigator",
        "Show Navigator",
        "Show Navigator (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "tools.collaboration",
        "Collaboration...",
        "Collaboration (disabled placeholder)",
        ITEM,
        None,
    ),
    SEP,
    placeholder(
        "tools.sprint-timer",
        "Sprint Timer...",
        "Sprint Timer (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "tools.writing-stats",
        "Writing Stats...",
        "Writing Stats (disabled placeholder)",
        ITEM,
        None,
    ),
];

const PRODUCTION_ENTRIES: &[MenuEntrySpec] = &[
    placeholder(
        "production.scene-numbers",
        "Scene Numbers...",
        "Scene Numbers (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "production.edit-scene-number",
        "Edit Scene Number...",
        "Edit Scene Number (disabled placeholder)",
        ITEM,
        Some(primary_shortcut(ShortcutKey::J, true)),
    ),
    SEP,
    placeholder(
        "production.omit-scene",
        "Omit Scene",
        "Omit Scene (disabled placeholder)",
        ITEM,
        None,
    ),
    SEP,
    placeholder(
        "production.revisions",
        "Revisions...",
        "Revisions (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "production.revision-mode",
        "Revision Mode",
        "Revision Mode (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "production.mark-revised",
        "Mark Revised",
        "Mark Revised (disabled placeholder)",
        ITEM,
        Some(display_shortcut(
            ShortcutKey::BracketRight,
            true,
            false,
            false,
            false,
        )),
    ),
    placeholder(
        "production.clear-revised",
        "Clear Revised",
        "Clear Revised (disabled placeholder)",
        ITEM,
        Some(display_shortcut(
            ShortcutKey::BracketLeft,
            true,
            false,
            false,
            false,
        )),
    ),
    SEP,
    placeholder(
        "production.lock-pages",
        "Lock Pages",
        "Lock Pages (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "production.unlock-a-pages",
        "Unlock \"A\" Pages",
        "Unlock A Pages (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "production.unlock-all-pages",
        "Unlock All Pages",
        "Unlock All Pages (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "production.locking-tools",
        "Locking Tools",
        "Locking Tools submenu placeholder",
        DISCLOSURE,
        None,
    ),
    SEP,
    placeholder(
        "production.tags-mode",
        "Tags Mode",
        "Tags Mode (disabled placeholder)",
        ITEM,
        Some(primary_shortcut(ShortcutKey::T, true)),
    ),
    placeholder(
        "production.tag-all-characters",
        "Tag All Characters",
        "Tag All Characters (disabled placeholder)",
        ITEM,
        None,
    ),
];

const WINDOW_ENTRIES: &[MenuEntrySpec] = &[
    placeholder(
        "window.minimize",
        "Minimize",
        "Minimize (disabled placeholder)",
        ITEM,
        Some(primary_shortcut(ShortcutKey::M, false)),
    ),
    placeholder(
        "window.zoom",
        "Zoom",
        "Zoom (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "window.fill",
        "Fill",
        "Fill (disabled placeholder)",
        ITEM,
        Some(display_shortcut(ShortcutKey::F, true, false, true, false)),
    ),
    placeholder(
        "window.center",
        "Center",
        "Center (disabled placeholder)",
        ITEM,
        Some(display_shortcut(ShortcutKey::C, true, false, true, false)),
    ),
    SEP,
    placeholder(
        "window.move-resize",
        "Move & Resize",
        "Move & Resize submenu placeholder",
        DISCLOSURE,
        None,
    ),
    placeholder(
        "window.full-screen-tile",
        "Full Screen Tile",
        "Full Screen Tile submenu placeholder",
        DISCLOSURE,
        None,
    ),
    SEP,
    placeholder(
        "window.remove-from-set",
        "Remove Window from Set",
        "Remove Window from Set (disabled placeholder)",
        ITEM,
        None,
    ),
    SEP,
    placeholder(
        "window.bring-all-front",
        "Bring All to Front",
        "Bring All to Front (disabled placeholder)",
        ITEM,
        None,
    ),
    SEP,
    placeholder(
        "window.previous-tab",
        "Show Previous Tab",
        "Show Previous Tab (disabled placeholder)",
        ITEM,
        Some(display_shortcut(
            ShortcutKey::ArrowRight,
            false,
            true,
            false,
            true,
        )),
    ),
    placeholder(
        "window.next-tab",
        "Show Next Tab",
        "Show Next Tab (disabled placeholder)",
        ITEM,
        Some(display_shortcut(
            ShortcutKey::ArrowRight,
            false,
            true,
            false,
            false,
        )),
    ),
    placeholder(
        "window.move-tab-new-window",
        "Move Tab to New Window",
        "Move Tab to New Window (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "window.merge-all",
        "Merge All Windows",
        "Merge All Windows (disabled placeholder)",
        ITEM,
        None,
    ),
    SEP,
    placeholder(
        "window.blank-document",
        "blank document.fdx",
        "blank document.fdx (checked, disabled placeholder)",
        CHECKED,
        None,
    ),
];

const HELP_ENTRIES: &[MenuEntrySpec] = &[
    placeholder(
        "help.search",
        "Search",
        "Help menu Search placeholder (disabled and noninteractive)",
        SEARCH,
        None,
    ),
    SEP,
    placeholder(
        "help.get-support",
        "Get Support",
        "Get Support (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "help.submit-feedback",
        "Submit Feedback",
        "Submit Feedback (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "help.learn",
        "Learn",
        "Learn (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "help.user-guide",
        "Final Draft User Guide",
        "Final Draft User Guide (disabled placeholder)",
        ITEM,
        None,
    ),
    SEP,
    placeholder(
        "help.welcome",
        "Welcome to Final Draft",
        "Welcome to Final Draft (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "help.whats-new",
        "What's New",
        "What's New (disabled placeholder)",
        ITEM,
        None,
    ),
    SEP,
    placeholder(
        "help.tutorials",
        "Idea to Draft Tutorials",
        "Idea to Draft Tutorials submenu placeholder",
        DISCLOSURE,
        None,
    ),
    placeholder(
        "help.sample-scripts",
        "Sample Scripts",
        "Sample Scripts submenu placeholder",
        DISCLOSURE,
        None,
    ),
    SEP,
    placeholder(
        "help.check-updates",
        "Check for Updates",
        "Check for Updates (disabled placeholder)",
        ITEM,
        None,
    ),
    SEP,
    placeholder(
        "help.manage-account",
        "Manage Account",
        "Manage Account (disabled placeholder)",
        ITEM,
        None,
    ),
    placeholder(
        "help.sign-out",
        "Sign Out...",
        "Sign Out (disabled placeholder)",
        ITEM,
        None,
    ),
];

/// The complete ordered catalog shared by the in-window and native renderers.
/// macOS's standard Application menu is deliberately constructed separately.
pub(crate) const MENU_CATALOG: &[MenuSpec] = &[
    MenuSpec {
        id: "file",
        label: "File",
        accessibility_name: "File",
        entries: FILE_ENTRIES,
    },
    MenuSpec {
        id: "edit",
        label: "Edit",
        accessibility_name: "Edit",
        entries: EDIT_ENTRIES,
    },
    MenuSpec {
        id: "view",
        label: "View",
        accessibility_name: "View",
        entries: VIEW_ENTRIES,
    },
    MenuSpec {
        id: "format",
        label: "Format",
        accessibility_name: "Format",
        entries: FORMAT_ENTRIES,
    },
    MenuSpec {
        id: "insert",
        label: "Insert",
        accessibility_name: "Insert",
        entries: INSERT_ENTRIES,
    },
    MenuSpec {
        id: "document",
        label: "Document",
        accessibility_name: "Document",
        entries: DOCUMENT_ENTRIES,
    },
    MenuSpec {
        id: "tools",
        label: "Tools",
        accessibility_name: "Tools",
        entries: TOOLS_ENTRIES,
    },
    MenuSpec {
        id: "production",
        label: "Production",
        accessibility_name: "Production",
        entries: PRODUCTION_ENTRIES,
    },
    MenuSpec {
        id: "window",
        label: "Window",
        accessibility_name: "Window",
        entries: WINDOW_ENTRIES,
    },
    MenuSpec {
        id: "help",
        label: "Help",
        accessibility_name: "Help",
        entries: HELP_ENTRIES,
    },
];

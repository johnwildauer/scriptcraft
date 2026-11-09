//! Declarative, presentation-independent desktop menu definitions.
//!
//! Menu IDs are stable integration identities. Labels/accessibility names,
//! semantic icon hints, display-only shortcuts, and behavior identity live in
//! one catalog. Availability remains a fresh application-state projection for
//! real commands; placeholders are intrinsically disabled and never dispatch.

use scriptcraft_commands::CommandId;
use super::shortcuts::{display_shortcut, primary_shortcut};
pub(super) use super::shortcuts::{LogicalShortcut, ShortcutKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MenuSpec {
    pub(super) id: &'static str,
    pub(super) label: &'static str,
    pub(super) accessibility_name: &'static str,
    pub(super) entries: &'static [MenuEntrySpec],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MenuEntrySpec {
    Item(MenuItemSpec),
    Separator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MenuItemBehavior {
    Command(CommandId),
    DisabledPlaceholder(PlaceholderKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PlaceholderKind {
    Item,
    Disclosure,
    Checked,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MenuItemSpec {
    pub(super) id: &'static str,
    pub(super) label: &'static str,
    pub(super) accessibility_name: &'static str,
    pub(super) icon: Option<SemanticIcon>,
    pub(super) shortcut: Option<LogicalShortcut>,
    /// Platform-specific alternate chord where the primary shortcut differs.
    pub(super) windows_alias: Option<LogicalShortcut>,
    pub(super) behavior: MenuItemBehavior,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SemanticIcon {
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
        id, label, accessibility_name, icon, shortcut, windows_alias,
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
        id, label, accessibility_name, icon: None, shortcut, windows_alias: None,
        behavior: MenuItemBehavior::DisabledPlaceholder(kind),
    })
}

const SEP: MenuEntrySpec = MenuEntrySpec::Separator;
const ITEM: PlaceholderKind = PlaceholderKind::Item;
const DISCLOSURE: PlaceholderKind = PlaceholderKind::Disclosure;
const CHECKED: PlaceholderKind = PlaceholderKind::Checked;
const SEARCH: PlaceholderKind = PlaceholderKind::Search;

const FILE_ENTRIES: &[MenuEntrySpec] = &[
    command_item("file.new", "New", "New document", Some(SemanticIcon::DocumentNew), Some(primary_shortcut(ShortcutKey::N, false)), None, CommandId::FileNew),
    placeholder("file.new-from-template", "New from Template...", "New from template (disabled placeholder)", ITEM, Some(primary_shortcut(ShortcutKey::N, true))),
    command_item("file.open", "Open...", "Open document", Some(SemanticIcon::FolderOpen), Some(primary_shortcut(ShortcutKey::O, false)), None, CommandId::FileOpenRequest),
    placeholder("file.open-recent", "Open Recent", "Open Recent submenu placeholder", DISCLOSURE, None),
    command_item("file.close", "Close", "Close window", Some(SemanticIcon::WindowClose), Some(primary_shortcut(ShortcutKey::W, false)), None, CommandId::FileClose),
    SEP,
    command_item("file.save", "Save", "Save document", Some(SemanticIcon::Save), Some(primary_shortcut(ShortcutKey::S, false)), None, CommandId::FileSave),
    command_item("file.save-as", "Save As...", "Save document as", Some(SemanticIcon::SaveAs), Some(primary_shortcut(ShortcutKey::S, true)), None, CommandId::FileSaveAsRequest),
    command_item("file.save-as-pdf", "Save As PDF...", "Save document as PDF", Some(SemanticIcon::PdfExport), Some(primary_shortcut(ShortcutKey::P, true)), None, CommandId::FileSaveAsPdf),
    placeholder("file.export", "Export", "Export submenu placeholder", DISCLOSURE, None),
    placeholder("file.backup", "Backup...", "Backup (disabled placeholder)", ITEM, None),
    placeholder("file.revert", "Revert", "Revert (disabled placeholder)", ITEM, None),
    SEP,
    placeholder("file.save-as-template", "Save as Template...", "Save as template (disabled placeholder)", ITEM, None),
    SEP,
    placeholder("file.print-preview", "Print Preview", "Print Preview (disabled placeholder)", ITEM, None),
    placeholder("file.print", "Print...", "Print (disabled placeholder)", ITEM, Some(primary_shortcut(ShortcutKey::P, false))),
    SEP,
    placeholder("file.register-script", "Register Script", "Register Script (disabled placeholder)", ITEM, None),
];

const EDIT_ENTRIES: &[MenuEntrySpec] = &[
    command_item("edit.undo", "Undo", "Undo last action", Some(SemanticIcon::Undo), Some(primary_shortcut(ShortcutKey::Z, false)), None, CommandId::EditUndo),
    command_item("edit.redo", "Redo", "Redo last action", Some(SemanticIcon::Redo), Some(primary_shortcut(ShortcutKey::Z, true)), Some(primary_shortcut(ShortcutKey::Y, false)), CommandId::EditRedo),
    SEP,
    command_item("edit.cut", "Cut", "Cut selected text", Some(SemanticIcon::Cut), Some(primary_shortcut(ShortcutKey::X, false)), None, CommandId::EditCut),
    command_item("edit.copy", "Copy", "Copy selected text", Some(SemanticIcon::Copy), Some(primary_shortcut(ShortcutKey::C, false)), None, CommandId::EditCopy),
    command_item("edit.paste", "Paste", "Paste clipboard text", Some(SemanticIcon::Paste), Some(primary_shortcut(ShortcutKey::V, false)), None, CommandId::EditPaste),
    SEP,
    command_item("edit.delete", "Delete", "Delete forward", Some(SemanticIcon::Delete), None, None, CommandId::EditDelete),
    command_item("edit.select-all", "Select All", "Select all screenplay text", Some(SemanticIcon::SelectAll), Some(primary_shortcut(ShortcutKey::A, false)), None, CommandId::EditSelectAll),
    placeholder("edit.select-scene", "Select Scene", "Select Scene (disabled placeholder)", ITEM, Some(display_shortcut(ShortcutKey::A, false, false, false, true))),
    SEP,
    placeholder("edit.track-changes", "Track Changes", "Track Changes submenu placeholder", DISCLOSURE, None),
    SEP,
    placeholder("edit.find", "Find...", "Find (disabled placeholder)", ITEM, Some(primary_shortcut(ShortcutKey::F, false))),
    placeholder("edit.go-to", "Go To...", "Go To (disabled placeholder)", ITEM, Some(primary_shortcut(ShortcutKey::G, false))),
    placeholder("edit.find-selection", "Find Selection", "Find Selection (disabled placeholder)", ITEM, Some(primary_shortcut(ShortcutKey::F, true))),
    placeholder("edit.rename-character", "Rename Character...", "Rename Character (disabled placeholder)", ITEM, None),
    SEP,
    placeholder("edit.writing-tools", "Writing Tools", "Writing Tools submenu placeholder", DISCLOSURE, None),
    placeholder("edit.autofill", "AutoFill", "AutoFill submenu placeholder", DISCLOSURE, None),
    placeholder("edit.start-dictation", "Start Dictation", "Start Dictation (disabled placeholder)", ITEM, None),
    placeholder("edit.emoji-symbols", "Emoji & Symbols", "Emoji & Symbols (disabled placeholder)", ITEM, None),
];

const VIEW_ENTRIES: &[MenuEntrySpec] = &[
    placeholder("view.split", "Split", "Split submenu placeholder", DISCLOSURE, None), SEP,
    placeholder("view.script-normal", "Script - Normal View", "Script - Normal View (disabled placeholder)", ITEM, None),
    placeholder("view.script-page", "Script - Page View", "Script - Page View (checked, disabled placeholder)", CHECKED, None),
    placeholder("view.script-speed", "Script - Speed View", "Script - Speed View (disabled placeholder)", ITEM, None), SEP,
    placeholder("view.beat-board", "Beat Board", "Beat Board (disabled placeholder)", ITEM, None), SEP,
    placeholder("view.scene-view", "Scene View", "Scene View (disabled placeholder)", ITEM, None),
    placeholder("view.scene-view-visibility", "Show/Hide Scene View", "Show/Hide Scene View submenu placeholder", DISCLOSURE, None), SEP,
    placeholder("view.index-cards-script", "Index Cards - Script", "Index Cards - Script (disabled placeholder)", ITEM, None),
    placeholder("view.index-cards-summary", "Index Cards - Summary", "Index Cards - Summary (disabled placeholder)", ITEM, None),
    placeholder("view.cards-across", "Cards Across", "Cards Across submenu placeholder", DISCLOSURE, None), SEP,
    placeholder("view.outline-editor", "Show Outline Editor", "Show Outline Editor (disabled placeholder)", ITEM, Some(primary_shortcut(ShortcutKey::M, true))),
    placeholder("view.ruler", "Show Ruler", "Show Ruler (disabled placeholder)", ITEM, None), SEP,
    placeholder("view.document", "Document", "Document submenu placeholder", DISCLOSURE, None), SEP,
    placeholder("view.zoom", "Zoom", "Zoom submenu placeholder", DISCLOSURE, None),
    placeholder("view.zoom-outline-editor", "Zoom Outline Editor", "Zoom Outline Editor submenu placeholder", DISCLOSURE, None), SEP,
    placeholder("view.typewriter", "Typewriter", "Typewriter submenu placeholder", DISCLOSURE, None), SEP,
    placeholder("view.appearance", "Appearance", "Appearance submenu placeholder", DISCLOSURE, None), SEP,
    placeholder("view.tab-bar", "Show Tab Bar", "Show Tab Bar (disabled placeholder)", ITEM, None),
    placeholder("view.all-tabs", "Show All Tabs", "Show All Tabs (disabled placeholder)", ITEM, Some(display_shortcut(ShortcutKey::Backslash, true, false, false, true))), SEP,
    placeholder("view.focus-mode", "Focus Mode", "Focus Mode (disabled placeholder)", ITEM, None),
    placeholder("view.full-screen", "Enter Full Screen", "Enter Full Screen (disabled placeholder)", ITEM, Some(display_shortcut(ShortcutKey::F, true, true, false, false))), SEP,
    placeholder("view.hide-toolbar", "Hide Toolbar", "Hide Toolbar (disabled placeholder)", ITEM, None),
    placeholder("view.customize-toolbar", "Customize Toolbar...", "Customize Toolbar (disabled placeholder)", ITEM, None),
];

const FORMAT_ENTRIES: &[MenuEntrySpec] = &[
    placeholder("format.elements", "Elements...", "Elements (disabled placeholder)", ITEM, Some(primary_shortcut(ShortcutKey::E, false))),
    placeholder("format.change-element", "Change Element To", "Change Element To submenu placeholder", DISCLOSURE, None),
    placeholder("format.cast-list-options", "Cast List Element Options...", "Cast List Element Options (disabled placeholder)", ITEM, None),
    placeholder("format.highlight-characters", "Highlight Characters...", "Highlight Characters (disabled placeholder)", ITEM, None), SEP,
    placeholder("format.set-font", "Set Font...", "Set Font (disabled placeholder)", ITEM, None),
    placeholder("format.font", "Font", "Font submenu placeholder", DISCLOSURE, None),
    placeholder("format.size", "Size", "Size submenu placeholder", DISCLOSURE, None),
    placeholder("format.style", "Style", "Style submenu placeholder", DISCLOSURE, None),
    placeholder("format.color", "Color", "Color submenu placeholder", DISCLOSURE, None),
    placeholder("format.highlight", "Highlight", "Highlight submenu placeholder", DISCLOSURE, None),
    placeholder("format.toggle-case", "tOGGLE cASE", "Toggle Case (disabled placeholder)", ITEM, None), SEP,
    placeholder("format.alignment", "Alignment", "Alignment submenu placeholder", DISCLOSURE, None),
    placeholder("format.spacing", "Spacing", "Spacing submenu placeholder", DISCLOSURE, None),
    placeholder("format.space-before", "Space Before", "Space Before submenu placeholder", DISCLOSURE, None),
    placeholder("format.leading", "Leading", "Leading submenu placeholder", DISCLOSURE, None), SEP,
    placeholder("format.beat-board", "Beat Board", "Beat Board submenu placeholder", DISCLOSURE, None), SEP,
    command_item(
        "format.dual-dialogue",
        "Dual Dialogue",
        "Toggle Dual Dialogue",
        None,
        Some(primary_shortcut(ShortcutKey::D, false)),
        Some(display_shortcut(ShortcutKey::D, true, false, true, false)),
        CommandId::EditorDualDialogueToggle,
    ),
    placeholder("format.edit-dual-dialogue", "Edit Dual Dialogue", "Edit Dual Dialogue (disabled placeholder)", ITEM, Some(primary_shortcut(ShortcutKey::D, true))),
];

const INSERT_ENTRIES: &[MenuEntrySpec] = &[
    placeholder("insert.add-alt", "Add Alt", "Add Alt (disabled placeholder)", ITEM, Some(display_shortcut(ShortcutKey::Period, true, false, false, true))),
    placeholder("insert.remove-alt", "Remove Alt", "Remove Alt (disabled placeholder)", ITEM, Some(display_shortcut(ShortcutKey::Comma, true, false, false, true))), SEP,
    placeholder("insert.new-beat", "New Beat", "New Beat (disabled placeholder)", ITEM, Some(primary_shortcut(ShortcutKey::Enter, false))), SEP,
    placeholder("insert.page-break", "Page Break", "Page Break (disabled placeholder)", ITEM, None),
    placeholder("insert.new-scene", "New Scene", "New Scene (disabled placeholder)", ITEM, Some(display_shortcut(ShortcutKey::Pipe, true, false, false, true))), SEP,
    placeholder("insert.bookmark", "Bookmark", "Bookmark (disabled placeholder)", ITEM, Some(display_shortcut(ShortcutKey::Backslash, true, false, false, false))),
    placeholder("insert.script-note", "ScriptNote", "ScriptNote (disabled placeholder)", ITEM, Some(primary_shortcut(ShortcutKey::K, true))),
    placeholder("insert.label", "Label...", "Label (disabled placeholder)", ITEM, Some(primary_shortcut(ShortcutKey::L, true))),
    placeholder("insert.image", "Image...", "Image (disabled placeholder)", ITEM, None), SEP,
    placeholder("insert.non-speaking-character", "Non-Speaking Character", "Non-Speaking Character (disabled placeholder)", ITEM, None), SEP,
    placeholder("insert.send-to-script", "Send to Script", "Send to Script (disabled placeholder)", ITEM, Some(primary_shortcut(ShortcutKey::C, true))),
];

const DOCUMENT_ENTRIES: &[MenuEntrySpec] = &[
    placeholder("document.page-layout", "Page Layout...", "Page Layout (disabled placeholder)", ITEM, None),
    placeholder("document.header-footer", "Header and Footer...", "Header and Footer (disabled placeholder)", ITEM, None),
    placeholder("document.mores-continueds", "Mores and Continueds...", "Mores and Continueds (disabled placeholder)", ITEM, None),
    placeholder("document.watermark", "Watermark...", "Watermark (disabled placeholder)", ITEM, None), SEP,
    placeholder("document.title-page", "Title Page...", "Title Page (disabled placeholder)", ITEM, None),
    placeholder("document.smarttype", "SmartType...", "SmartType (disabled placeholder)", ITEM, Some(primary_shortcut(ShortcutKey::L, false))),
    placeholder("document.macros", "Macros...", "Macros (disabled placeholder)", ITEM, None),
    placeholder("document.bookmarks", "Bookmarks...", "Bookmarks (disabled placeholder)", ITEM, None), SEP,
    placeholder("document.target-script-length", "Change Target Script Length...", "Change Target Script Length (disabled placeholder)", ITEM, None),
];

const TOOLS_ENTRIES: &[MenuEntrySpec] = &[
    placeholder("tools.spelling", "Spelling...", "Spelling (disabled placeholder)", ITEM, Some(display_shortcut(ShortcutKey::Comma, true, false, false, false))),
    placeholder("tools.thesaurus", "Thesaurus", "Thesaurus (disabled placeholder)", ITEM, Some(display_shortcut(ShortcutKey::Semicolon, true, false, false, false))), SEP,
    placeholder("tools.speech-control", "Speech Control...", "Speech Control (disabled placeholder)", ITEM, None),
    placeholder("tools.assign-voices", "Assign Voices...", "Assign Voices (disabled placeholder)", ITEM, None), SEP,
    placeholder("tools.reports", "Reports", "Reports submenu placeholder", DISCLOSURE, None),
    placeholder("tools.combine-files", "Combine Files...", "Combine Files (disabled placeholder)", ITEM, None),
    placeholder("tools.names-database", "Names Database...", "Names Database (disabled placeholder)", ITEM, None),
    placeholder("tools.reformat", "Reformat...", "Reformat (disabled placeholder)", ITEM, Some(primary_shortcut(ShortcutKey::R, false))),
    placeholder("tools.format-assistant", "Format Assistant...", "Format Assistant (disabled placeholder)", ITEM, None),
    placeholder("tools.script-compare", "ScriptCompare...", "ScriptCompare (disabled placeholder)", ITEM, None),
    placeholder("tools.navigator", "Show Navigator", "Show Navigator (disabled placeholder)", ITEM, None),
    placeholder("tools.collaboration", "Collaboration...", "Collaboration (disabled placeholder)", ITEM, None), SEP,
    placeholder("tools.sprint-timer", "Sprint Timer...", "Sprint Timer (disabled placeholder)", ITEM, None),
    placeholder("tools.writing-stats", "Writing Stats...", "Writing Stats (disabled placeholder)", ITEM, None),
];

const PRODUCTION_ENTRIES: &[MenuEntrySpec] = &[
    placeholder("production.scene-numbers", "Scene Numbers...", "Scene Numbers (disabled placeholder)", ITEM, None),
    placeholder("production.edit-scene-number", "Edit Scene Number...", "Edit Scene Number (disabled placeholder)", ITEM, Some(primary_shortcut(ShortcutKey::J, true))), SEP,
    placeholder("production.omit-scene", "Omit Scene", "Omit Scene (disabled placeholder)", ITEM, None), SEP,
    placeholder("production.revisions", "Revisions...", "Revisions (disabled placeholder)", ITEM, None),
    placeholder("production.revision-mode", "Revision Mode", "Revision Mode (disabled placeholder)", ITEM, None),
    placeholder("production.mark-revised", "Mark Revised", "Mark Revised (disabled placeholder)", ITEM, Some(display_shortcut(ShortcutKey::BracketRight, true, false, false, false))),
    placeholder("production.clear-revised", "Clear Revised", "Clear Revised (disabled placeholder)", ITEM, Some(display_shortcut(ShortcutKey::BracketLeft, true, false, false, false))), SEP,
    placeholder("production.lock-pages", "Lock Pages", "Lock Pages (disabled placeholder)", ITEM, None),
    placeholder("production.unlock-a-pages", "Unlock \"A\" Pages", "Unlock A Pages (disabled placeholder)", ITEM, None),
    placeholder("production.unlock-all-pages", "Unlock All Pages", "Unlock All Pages (disabled placeholder)", ITEM, None),
    placeholder("production.locking-tools", "Locking Tools", "Locking Tools submenu placeholder", DISCLOSURE, None), SEP,
    placeholder("production.tags-mode", "Tags Mode", "Tags Mode (disabled placeholder)", ITEM, Some(primary_shortcut(ShortcutKey::T, true))),
    placeholder("production.tag-all-characters", "Tag All Characters", "Tag All Characters (disabled placeholder)", ITEM, None),
];

const WINDOW_ENTRIES: &[MenuEntrySpec] = &[
    placeholder("window.minimize", "Minimize", "Minimize (disabled placeholder)", ITEM, Some(primary_shortcut(ShortcutKey::M, false))),
    placeholder("window.zoom", "Zoom", "Zoom (disabled placeholder)", ITEM, None),
    placeholder("window.fill", "Fill", "Fill (disabled placeholder)", ITEM, Some(display_shortcut(ShortcutKey::F, true, false, true, false))),
    placeholder("window.center", "Center", "Center (disabled placeholder)", ITEM, Some(display_shortcut(ShortcutKey::C, true, false, true, false))), SEP,
    placeholder("window.move-resize", "Move & Resize", "Move & Resize submenu placeholder", DISCLOSURE, None),
    placeholder("window.full-screen-tile", "Full Screen Tile", "Full Screen Tile submenu placeholder", DISCLOSURE, None), SEP,
    placeholder("window.remove-from-set", "Remove Window from Set", "Remove Window from Set (disabled placeholder)", ITEM, None), SEP,
    placeholder("window.bring-all-front", "Bring All to Front", "Bring All to Front (disabled placeholder)", ITEM, None), SEP,
    placeholder("window.previous-tab", "Show Previous Tab", "Show Previous Tab (disabled placeholder)", ITEM, Some(display_shortcut(ShortcutKey::ArrowRight, false, true, false, true))),
    placeholder("window.next-tab", "Show Next Tab", "Show Next Tab (disabled placeholder)", ITEM, Some(display_shortcut(ShortcutKey::ArrowRight, false, true, false, false))),
    placeholder("window.move-tab-new-window", "Move Tab to New Window", "Move Tab to New Window (disabled placeholder)", ITEM, None),
    placeholder("window.merge-all", "Merge All Windows", "Merge All Windows (disabled placeholder)", ITEM, None), SEP,
    placeholder("window.blank-document", "blank document.fdx", "blank document.fdx (checked, disabled placeholder)", CHECKED, None),
];

const HELP_ENTRIES: &[MenuEntrySpec] = &[
    placeholder("help.search", "Search", "Help menu Search placeholder (disabled and noninteractive)", SEARCH, None), SEP,
    placeholder("help.get-support", "Get Support", "Get Support (disabled placeholder)", ITEM, None),
    placeholder("help.submit-feedback", "Submit Feedback", "Submit Feedback (disabled placeholder)", ITEM, None),
    placeholder("help.learn", "Learn", "Learn (disabled placeholder)", ITEM, None),
    placeholder("help.user-guide", "Final Draft User Guide", "Final Draft User Guide (disabled placeholder)", ITEM, None), SEP,
    placeholder("help.welcome", "Welcome to Final Draft", "Welcome to Final Draft (disabled placeholder)", ITEM, None),
    placeholder("help.whats-new", "What's New", "What's New (disabled placeholder)", ITEM, None), SEP,
    placeholder("help.tutorials", "Idea to Draft Tutorials", "Idea to Draft Tutorials submenu placeholder", DISCLOSURE, None),
    placeholder("help.sample-scripts", "Sample Scripts", "Sample Scripts submenu placeholder", DISCLOSURE, None), SEP,
    placeholder("help.check-updates", "Check for Updates", "Check for Updates (disabled placeholder)", ITEM, None), SEP,
    placeholder("help.manage-account", "Manage Account", "Manage Account (disabled placeholder)", ITEM, None),
    placeholder("help.sign-out", "Sign Out...", "Sign Out (disabled placeholder)", ITEM, None),
];

/// The complete ordered catalog shared by the in-window and native renderers.
/// macOS's standard Application menu is deliberately constructed separately.
pub(super) const MENU_CATALOG: &[MenuSpec] = &[
    MenuSpec { id: "file", label: "File", accessibility_name: "File", entries: FILE_ENTRIES },
    MenuSpec { id: "edit", label: "Edit", accessibility_name: "Edit", entries: EDIT_ENTRIES },
    MenuSpec { id: "view", label: "View", accessibility_name: "View", entries: VIEW_ENTRIES },
    MenuSpec { id: "format", label: "Format", accessibility_name: "Format", entries: FORMAT_ENTRIES },
    MenuSpec { id: "insert", label: "Insert", accessibility_name: "Insert", entries: INSERT_ENTRIES },
    MenuSpec { id: "document", label: "Document", accessibility_name: "Document", entries: DOCUMENT_ENTRIES },
    MenuSpec { id: "tools", label: "Tools", accessibility_name: "Tools", entries: TOOLS_ENTRIES },
    MenuSpec { id: "production", label: "Production", accessibility_name: "Production", entries: PRODUCTION_ENTRIES },
    MenuSpec { id: "window", label: "Window", accessibility_name: "Window", entries: WINDOW_ENTRIES },
    MenuSpec { id: "help", label: "Help", accessibility_name: "Help", entries: HELP_ENTRIES },
];

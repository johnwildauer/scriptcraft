use crate::presentation::material_font::{MaterialIconAxes, MaterialIconAxisOverrides};
use eframe::egui::Color32;
use egui_material_icons::{
    icons::{
        ICON_ARTICLE, ICON_BOOK_5, ICON_CANCEL, ICON_DASHBOARD, ICON_DOUBLE_ARROW, ICON_DRAFT,
        ICON_FILE_OPEN, ICON_FORMAT_ALIGN_LEFT, ICON_FORMAT_BOLD, ICON_FORMAT_ITALIC,
        ICON_FORMAT_STRIKETHROUGH, ICON_FORMAT_UNDERLINED, ICON_GRID_VIEW,
        ICON_KEYBOARD_ARROW_DOWN, ICON_NOTES, ICON_PICTURE_AS_PDF, ICON_SAVE, ICON_SAVE_AS,
        ICON_TABLE, ICON_TEXT_FIELDS, ICON_UPLOAD_FILE, ICON_VIEW_LIST, ICON_VISIBILITY,
        ICON_WEB_ASSET,
    },
    MaterialIcon,
};
use scriptcraft_commands::{Command, CommandId, ScriptViewMode};
use crate::settings::Settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToolbarIconVariant {
    Filled,
    Outlined,
}

const SHOW_HIDE_GRAPHIC: ToolbarGraphic = ToolbarGraphic::IconDisclosure {
    primary: IconSpec::new(ICON_VISIBILITY),
    disclosure: IconSpec::new(ICON_KEYBOARD_ARROW_DOWN),
};

const VIEWS_GRAPHIC: ToolbarGraphic = ToolbarGraphic::IconDisclosure {
    primary: IconSpec::new(ICON_WEB_ASSET),
    disclosure: IconSpec::new(ICON_KEYBOARD_ARROW_DOWN),
};

// Material Symbols List 2 (U+FFECA), from the pinned inventory in fonts/MaterialSymbols/README.md.
const ICON_LIST_2: MaterialIcon = MaterialIcon::new("\u{FFECA}");

#[derive(Debug, Clone, Copy)]
pub(super) struct IconSpec {
    pub(super) icon: MaterialIcon,
    pub(super) variant: Option<ToolbarIconVariant>,
    pub(super) material_axes: Option<MaterialIconAxisOverrides>,
    pub(super) point_size: Option<f32>,
    pub(super) enabled_color: Option<Color32>,
    pub(super) disabled_color: Option<Color32>,
}

impl IconSpec {
    pub(super) const fn new(icon: MaterialIcon) -> Self {
        Self {
            icon,
            variant: None,
            material_axes: None,
            point_size: None,
            enabled_color: None,
            disabled_color: None,
        }
    }

    pub(super) const fn with_point_size(mut self, point_size: f32) -> Self {
        self.point_size = Some(point_size);
        self
    }

    pub(super) const fn with_variant(mut self, variant: ToolbarIconVariant) -> Self {
        self.variant = Some(variant);
        self
    }

    #[allow(dead_code)] // Catalog capability for declaring per-icon native font axes.
    pub(super) const fn with_material_axes(mut self, axes: MaterialIconAxisOverrides) -> Self {
        self.material_axes = Some(axes);
        self
    }
}

/// Returns non-default optical-size/grade combinations used by every built-in
/// toolbar item, independent of the user's configured toolbar membership.
pub(crate) fn custom_material_icon_axes() -> Vec<MaterialIconAxes> {
    let mut axes = Vec::new();
    for item in BUILTIN_TOOLBAR_ITEM_CATALOG.iter().copied() {
        match item.graphic {
            ToolbarGraphic::Icon(icon) => add_custom_axes(icon, &mut axes),
            ToolbarGraphic::Icons(icons) => {
                for icon in icons.iter().copied() {
                    add_custom_axes(icon, &mut axes);
                }
            }
            ToolbarGraphic::IconDisclosure {
                primary,
                disclosure,
            } => {
                add_custom_axes(primary, &mut axes);
                add_custom_axes(disclosure, &mut axes);
            }
            ToolbarGraphic::None | ToolbarGraphic::TextDropdown => {}
        }
    }
    axes
}

fn add_custom_axes(icon: IconSpec, axes: &mut Vec<MaterialIconAxes>) {
    let Some(overrides) = icon.material_axes else {
        return;
    };
    let resolved = overrides.resolve(MaterialIconAxes::DEFAULT);
    if (resolved.optical_size != MaterialIconAxes::DEFAULT.optical_size
        || resolved.grade != MaterialIconAxes::DEFAULT.grade)
        && !axes.contains(&resolved)
    {
        axes.push(resolved);
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ToolbarGraphic {
    None,
    Icon(IconSpec),
    /// Icons are independently configurable and laid out left-to-right.
    #[allow(dead_code)]
    // Supported catalog capability for independently configured icon groups.
    Icons(&'static [IconSpec]),
    /// The disclosure icon has its own size and foreground overrides.
    #[allow(dead_code)] // Supported so primary and disclosure icons can vary independently.
    IconDisclosure {
        primary: IconSpec,
        disclosure: IconSpec,
    },
    /// The selected value is supplied by the behavior adapter, not this graphic.
    TextDropdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToolbarBehavior {
    None,
    Action(CommandId),
    ScriptView(ScriptViewMode),
    ElementsRoleDropdown,
    ShowHideDropdown,
    ViewsDropdown,
    StylesInlineGroup,
    AlignmentInlineGroup,
    OverflowMenu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EnablementPolicy {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ToolbarLayoutPolicy {
    Measured,
    FixedWidth(f32),
    FixedSpace(f32),
    FlexibleSpace,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ItemPresentationOverrides {
    pub(super) enabled_color: Option<Color32>,
    pub(super) disabled_color: Option<Color32>,
}

impl ItemPresentationOverrides {
    pub(super) const NONE: Self = Self {
        enabled_color: None,
        disabled_color: None,
    };
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ToolbarItemSpec {
    pub(super) id: &'static str,
    pub(super) label: &'static str,
    pub(super) accessibility_name: &'static str,
    pub(super) caption_visible: bool,
    pub(super) graphic: ToolbarGraphic,
    pub(super) behavior: ToolbarBehavior,
    pub(super) enablement: EnablementPolicy,
    pub(super) layout: ToolbarLayoutPolicy,
    pub(super) presentation: ItemPresentationOverrides,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ViewsAction {
    ScriptMode(ScriptViewMode),
    BeatBoard,
}

impl ViewsAction {
    pub(super) fn command(self) -> Command {
        match self {
            Self::ScriptMode(mode) => Command::ViewScriptViewSet { mode },
            Self::BeatBoard => Command::ViewBeatBoardShow,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ViewsChoiceSpec {
    pub(super) id: &'static str,
    pub(super) label: &'static str,
    pub(super) accessibility_name: &'static str,
    pub(super) icon: IconSpec,
    pub(super) action: Option<ViewsAction>,
}

pub(super) const VIEWS_CHOICES: [ViewsChoiceSpec; 7] = [
    ViewsChoiceSpec {
        id: "script-normal",
        label: "Script - Normal View",
        accessibility_name: "Script - Normal View",
        icon: IconSpec::new(ICON_LIST_2),
        action: Some(ViewsAction::ScriptMode(ScriptViewMode::Normal)),
    },
    ViewsChoiceSpec {
        id: "script-page",
        label: "Script - Page View",
        accessibility_name: "Script - Page View",
        icon: IconSpec::new(ICON_ARTICLE),
        action: Some(ViewsAction::ScriptMode(ScriptViewMode::Page)),
    },
    ViewsChoiceSpec {
        id: "script-speed",
        label: "Script - Speed View",
        accessibility_name: "Script - Speed View",
        icon: IconSpec::new(ICON_NOTES),
        action: Some(ViewsAction::ScriptMode(ScriptViewMode::Speed)),
    },
    ViewsChoiceSpec {
        id: "beat-board",
        label: "Beat Board",
        accessibility_name: "Beat Board",
        icon: IconSpec::new(ICON_DASHBOARD),
        action: Some(ViewsAction::BeatBoard),
    },
    ViewsChoiceSpec {
        id: "scene-view",
        label: "Scene View",
        accessibility_name: "Scene View",
        icon: IconSpec::new(ICON_VIEW_LIST),
        action: None,
    },
    ViewsChoiceSpec {
        id: "index-cards-script",
        label: "Index Cards - Script",
        accessibility_name: "Index Cards - Script",
        icon: IconSpec::new(ICON_GRID_VIEW).with_variant(ToolbarIconVariant::Filled),
        action: None,
    },
    ViewsChoiceSpec {
        id: "index-cards-summary",
        label: "Index Cards - Summary",
        accessibility_name: "Index Cards - Summary",
        icon: IconSpec::new(ICON_GRID_VIEW),
        action: None,
    },
];

const fn script_view_item(choice: ViewsChoiceSpec) -> ToolbarItemSpec {
    let mode = match choice.action {
        Some(ViewsAction::ScriptMode(mode)) => mode,
        _ => panic!("standalone Script toolbar item requires a Script view choice"),
    };
    ToolbarItemSpec {
        id: choice.id,
        label: choice.label,
        accessibility_name: choice.accessibility_name,
        caption_visible: true,
        graphic: ToolbarGraphic::Icon(choice.icon),
        behavior: ToolbarBehavior::ScriptView(mode),
        enablement: EnablementPolicy::Enabled,
        layout: ToolbarLayoutPolicy::Measured,
        presentation: ItemPresentationOverrides::NONE,
    }
}

const SCRIPT_NORMAL_ITEM: ToolbarItemSpec = script_view_item(VIEWS_CHOICES[0]);
const SCRIPT_PAGE_ITEM: ToolbarItemSpec = script_view_item(VIEWS_CHOICES[1]);
const SCRIPT_SPEED_ITEM: ToolbarItemSpec = script_view_item(VIEWS_CHOICES[2]);

/// Immutable effective toolbar composition resolved from the raw settings snapshot.
pub(crate) struct ConfiguredToolbars {
    main: Vec<ToolbarItemSpec>,
    title_page: Vec<ToolbarItemSpec>,
}

impl ConfiguredToolbars {
    /// Resolves both surfaces once, retaining only owned built-in specifications.
    pub(crate) fn resolve(settings: &Settings) -> Self {
        let defaults = Settings::default();
        Self {
            main: resolve_toolbar_list(&settings.toolbar, &defaults.toolbar, "main"),
            title_page: resolve_toolbar_list(
                &settings.title_page_toolbar,
                &defaults.title_page_toolbar,
                "Title Page",
            ),
        }
    }

    pub(super) fn main_items(&self) -> &[ToolbarItemSpec] {
        &self.main
    }

    pub(super) fn title_page_items(&self) -> &[ToolbarItemSpec] {
        &self.title_page
    }
}

fn resolve_toolbar_list(
    configured_ids: &[String],
    default_ids: &[String],
    surface: &str,
) -> Vec<ToolbarItemSpec> {
    let ids = if configured_ids.is_empty() {
        default_ids
    } else {
        configured_ids
    };
    let mut resolved = Vec::with_capacity(ids.len());
    let mut omitted = 0;
    for id in ids {
        if let Some(item) = builtin_item(id) {
            resolved.push(item);
        } else {
            omitted += 1;
        }
    }
    if omitted != 0 {
        eprintln!("Warning: {surface} toolbar omitted {omitted} unknown item ID(s)");
    }
    if resolved.is_empty() {
        default_ids
            .iter()
            .filter_map(|id| builtin_item(id))
            .collect()
    } else {
        resolved
    }
}

fn builtin_item(id: &str) -> Option<ToolbarItemSpec> {
    BUILTIN_TOOLBAR_ITEM_CATALOG
        .iter()
        .find(|item| item.id == id)
        .copied()
}

const fn action(
    id: &'static str,
    label: &'static str,
    icon: MaterialIcon,
    action: CommandId,
) -> ToolbarItemSpec {
    ToolbarItemSpec {
        id,
        label,
        accessibility_name: label,
        caption_visible: true,
        graphic: ToolbarGraphic::Icon(IconSpec::new(icon)),
        behavior: ToolbarBehavior::Action(action),
        enablement: EnablementPolicy::Enabled,
        layout: ToolbarLayoutPolicy::Measured,
        presentation: ItemPresentationOverrides::NONE,
    }
}

const STYLE_GROUP_ICONS: [IconSpec; 5] = [
    IconSpec::new(ICON_FORMAT_BOLD),
    IconSpec::new(ICON_FORMAT_ITALIC),
    IconSpec::new(ICON_FORMAT_UNDERLINED),
    IconSpec::new(ICON_TEXT_FIELDS),
    IconSpec::new(ICON_FORMAT_STRIKETHROUGH),
];

const ALIGNMENT_GROUP_ICONS: [IconSpec; 4] = [
    IconSpec::new(ICON_FORMAT_ALIGN_LEFT),
    IconSpec::new(egui_material_icons::icons::ICON_FORMAT_ALIGN_CENTER),
    IconSpec::new(egui_material_icons::icons::ICON_FORMAT_ALIGN_RIGHT),
    IconSpec::new(egui_material_icons::icons::ICON_FORMAT_ALIGN_JUSTIFY),
];

const STYLE_INLINE_GROUP_ITEM: ToolbarItemSpec = ToolbarItemSpec {
    id: "styles",
    label: "Styles",
    accessibility_name: "Styles",
    caption_visible: true,
    graphic: ToolbarGraphic::Icons(&STYLE_GROUP_ICONS),
    behavior: ToolbarBehavior::StylesInlineGroup,
    enablement: EnablementPolicy::Enabled,
    layout: ToolbarLayoutPolicy::Measured,
    presentation: ItemPresentationOverrides::NONE,
};

const ALIGNMENT_INLINE_GROUP_ITEM: ToolbarItemSpec = ToolbarItemSpec {
    id: "alignment",
    label: "Alignment",
    accessibility_name: "Alignment",
    caption_visible: true,
    graphic: ToolbarGraphic::Icons(&ALIGNMENT_GROUP_ICONS),
    behavior: ToolbarBehavior::AlignmentInlineGroup,
    enablement: EnablementPolicy::Enabled,
    layout: ToolbarLayoutPolicy::Measured,
    presentation: ItemPresentationOverrides::NONE,
};

const FLEXIBLE_SPACE_ITEM: ToolbarItemSpec = ToolbarItemSpec {
    id: "flexible-space",
    label: "Flexible Space",
    accessibility_name: "Flexible Space",
    caption_visible: false,
    graphic: ToolbarGraphic::None,
    behavior: ToolbarBehavior::None,
    enablement: EnablementPolicy::Enabled,
    layout: ToolbarLayoutPolicy::FlexibleSpace,
    presentation: ItemPresentationOverrides::NONE,
};

const CLOSE_TITLE_PAGE_ITEM: ToolbarItemSpec = ToolbarItemSpec {
    id: "close-title-page",
    label: "Close Title Page",
    accessibility_name: "Close Title Page",
    caption_visible: true,
    graphic: ToolbarGraphic::Icon(IconSpec::new(ICON_CANCEL)),
    behavior: ToolbarBehavior::Action(CommandId::WindowTitlePageClose),
    enablement: EnablementPolicy::Enabled,
    layout: ToolbarLayoutPolicy::Measured,
    presentation: ItemPresentationOverrides::NONE,
};

const NEW_ITEM: ToolbarItemSpec = action("new", "New", ICON_DRAFT, CommandId::FileNew);
const OPEN_ITEM: ToolbarItemSpec = action("open", "Open", ICON_FILE_OPEN, CommandId::FileOpenRequest);
const SAVE_ITEM: ToolbarItemSpec = action("save", "Save", ICON_SAVE, CommandId::FileSave);
const SAVE_AS_ITEM: ToolbarItemSpec = action(
    "save-as",
    "Save As",
    ICON_SAVE_AS,
    CommandId::FileSaveAsRequest,
);
const TITLE_PAGE_ITEM: ToolbarItemSpec = action(
    "title-page",
    "Title Page",
    ICON_BOOK_5,
    CommandId::WindowTitlePageOpen,
);
const BEAT_BOARD_ITEM: ToolbarItemSpec = action(
    "beat-board",
    "Beat Board",
    ICON_DASHBOARD,
    CommandId::ViewBeatBoardShow,
);
const ELEMENTS_ITEM: ToolbarItemSpec = ToolbarItemSpec {
    id: "elements",
    label: "Elements",
    accessibility_name: "Elements",
    caption_visible: true,
    graphic: ToolbarGraphic::TextDropdown,
    behavior: ToolbarBehavior::ElementsRoleDropdown,
    enablement: EnablementPolicy::Enabled,
    layout: ToolbarLayoutPolicy::FixedWidth(128.0),
    presentation: ItemPresentationOverrides::NONE,
};
const SHOW_HIDE_ITEM: ToolbarItemSpec = ToolbarItemSpec {
    id: "show-hide",
    label: "Show/Hide",
    accessibility_name: "Show/Hide",
    caption_visible: true,
    graphic: SHOW_HIDE_GRAPHIC,
    behavior: ToolbarBehavior::ShowHideDropdown,
    enablement: EnablementPolicy::Enabled,
    layout: ToolbarLayoutPolicy::Measured,
    presentation: ItemPresentationOverrides::NONE,
};
const VIEWS_ITEM: ToolbarItemSpec = ToolbarItemSpec {
    id: "views",
    label: "Views",
    accessibility_name: "Views",
    caption_visible: true,
    graphic: VIEWS_GRAPHIC,
    behavior: ToolbarBehavior::ViewsDropdown,
    enablement: EnablementPolicy::Enabled,
    layout: ToolbarLayoutPolicy::Measured,
    presentation: ItemPresentationOverrides::NONE,
};
const EXPORT_TEXT_ITEM: ToolbarItemSpec = ToolbarItemSpec {
    id: "export-text",
    label: "Export Text",
    accessibility_name: "Export Text",
    caption_visible: true,
    graphic: ToolbarGraphic::Icon(IconSpec::new(ICON_UPLOAD_FILE)),
    behavior: ToolbarBehavior::None,
    enablement: EnablementPolicy::Disabled,
    layout: ToolbarLayoutPolicy::Measured,
    presentation: ItemPresentationOverrides::NONE,
};
const NAVIGATOR_ITEM: ToolbarItemSpec = ToolbarItemSpec {
    id: "navigator",
    label: "Navigator",
    accessibility_name: "Navigator",
    caption_visible: true,
    graphic: ToolbarGraphic::Icon(IconSpec::new(ICON_TABLE)),
    behavior: ToolbarBehavior::None,
    enablement: EnablementPolicy::Disabled,
    layout: ToolbarLayoutPolicy::Measured,
    presentation: ItemPresentationOverrides::NONE,
};
const EXPORT_PDF_ITEM: ToolbarItemSpec = action(
    "export-pdf",
    "Export PDF",
    ICON_PICTURE_AS_PDF,
    CommandId::FileSaveAsPdf,
);

const BUILTIN_TOOLBAR_ITEM_CATALOG: [ToolbarItemSpec; 21] = [
    FLEXIBLE_SPACE_ITEM,
    SPACE_ITEM_SPEC,
    NEW_ITEM,
    OPEN_ITEM,
    SAVE_ITEM,
    SAVE_AS_ITEM,
    VIEWS_ITEM,
    SCRIPT_NORMAL_ITEM,
    SCRIPT_PAGE_ITEM,
    SCRIPT_SPEED_ITEM,
    BEAT_BOARD_ITEM,
    TITLE_PAGE_ITEM,
    ELEMENTS_ITEM,
    SHOW_HIDE_ITEM,
    NAVIGATOR_ITEM,
    EXPORT_TEXT_ITEM,
    EXPORT_PDF_ITEM,
    STYLE_INLINE_GROUP_ITEM,
    ALIGNMENT_INLINE_GROUP_ITEM,
    CLOSE_TITLE_PAGE_ITEM,
    OVERFLOW_ITEM_SPEC,
];

/// Reusable fixed spacing specification; it is not part of the temporary composition.
#[allow(dead_code)] // Retained for later catalog compositions that request fixed spacing.
pub(super) const SPACE_ITEM_SPEC: ToolbarItemSpec = ToolbarItemSpec {
    id: "space",
    label: "Space",
    accessibility_name: "Space",
    caption_visible: false,
    graphic: ToolbarGraphic::None,
    behavior: ToolbarBehavior::None,
    enablement: EnablementPolicy::Enabled,
    layout: ToolbarLayoutPolicy::FixedSpace(72.0),
    presentation: ItemPresentationOverrides::NONE,
};

pub(super) const OVERFLOW_ITEM_SPEC: ToolbarItemSpec = ToolbarItemSpec {
    id: "overflow",
    label: "More actions",
    accessibility_name: "Toolbar overflow",
    caption_visible: false,
    graphic: ToolbarGraphic::Icon(IconSpec::new(ICON_DOUBLE_ARROW)),
    behavior: ToolbarBehavior::OverflowMenu,
    enablement: EnablementPolicy::Enabled,
    layout: ToolbarLayoutPolicy::FixedWidth(32.0),
    presentation: ItemPresentationOverrides::NONE,
};

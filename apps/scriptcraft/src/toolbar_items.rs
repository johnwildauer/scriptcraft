use eframe::egui::Color32;
use egui_material_icons::{
    icons::{
        ICON_DRAFT, ICON_DOUBLE_ARROW, ICON_UPLOAD_FILE, ICON_FILE_OPEN,
        ICON_KEYBOARD_ARROW_DOWN, ICON_PICTURE_AS_PDF, ICON_SAVE, ICON_SAVE_AS,
        ICON_VISIBILITY,
    },
    MaterialIcon,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToolbarAction {
    New,
    Open,
    Save,
    SaveAs,
    ExportPdf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ToolbarIconVariant {
    Filled,
    Outlined,
}

const SHOW_HIDE_GRAPHIC: ToolbarGraphic = ToolbarGraphic::IconDisclosure {
    primary: IconSpec::new(ICON_VISIBILITY),
    disclosure: IconSpec::new(ICON_KEYBOARD_ARROW_DOWN),
};

#[derive(Debug, Clone, Copy)]
pub(super) struct IconSpec {
    pub(super) icon: MaterialIcon,
    pub(super) variant: Option<ToolbarIconVariant>,
    pub(super) point_size: Option<f32>,
    pub(super) enabled_color: Option<Color32>,
    pub(super) disabled_color: Option<Color32>,
}

impl IconSpec {
    pub(super) const fn new(icon: MaterialIcon) -> Self {
        Self {
            icon,
            variant: None,
            point_size: None,
            enabled_color: None,
            disabled_color: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ToolbarGraphic {
    None,
    Icon(IconSpec),
    /// Icons are independently configurable and laid out left-to-right.
    #[allow(dead_code)] // Supported catalog capability for independently configured icon groups.
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
    Action(ToolbarAction),
    ElementsRoleDropdown,
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

const fn action(
    id: &'static str,
    label: &'static str,
    icon: MaterialIcon,
    action: ToolbarAction,
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

pub(super) const TEMPORARY_TOOLBAR_ITEMS: [ToolbarItemSpec; 13] = [
    ToolbarItemSpec {
        id: "flexible-space-leading",
        label: "Flexible Space",
        accessibility_name: "Flexible Space",
        caption_visible: false,
        graphic: ToolbarGraphic::None,
        behavior: ToolbarBehavior::None,
        enablement: EnablementPolicy::Enabled,
        layout: ToolbarLayoutPolicy::FlexibleSpace,
        presentation: ItemPresentationOverrides::NONE,
    },
    action("new", "New", ICON_DRAFT, ToolbarAction::New),
    action("open", "Open", ICON_FILE_OPEN, ToolbarAction::Open),
    action("save", "Save", ICON_SAVE, ToolbarAction::Save),
    action("save-as", "Save As", ICON_SAVE_AS, ToolbarAction::SaveAs),
    ToolbarItemSpec {
        id: "flexible-space-pre-elements",
        label: "Flexible Space",
        accessibility_name: "Flexible Space",
        caption_visible: false,
        graphic: ToolbarGraphic::None,
        behavior: ToolbarBehavior::None,
        enablement: EnablementPolicy::Enabled,
        layout: ToolbarLayoutPolicy::FlexibleSpace,
        presentation: ItemPresentationOverrides::NONE,
    },
    ToolbarItemSpec {
        id: "elements",
        label: "Elements",
        accessibility_name: "Elements",
        caption_visible: true,
        graphic: ToolbarGraphic::TextDropdown,
        behavior: ToolbarBehavior::ElementsRoleDropdown,
        enablement: EnablementPolicy::Enabled,
        layout: ToolbarLayoutPolicy::FixedWidth(128.0),
        presentation: ItemPresentationOverrides::NONE,
    },
    ToolbarItemSpec {
        id: "flexible-space-post-elements",
        label: "Flexible Space",
        accessibility_name: "Flexible Space",
        caption_visible: false,
        graphic: ToolbarGraphic::None,
        behavior: ToolbarBehavior::None,
        enablement: EnablementPolicy::Enabled,
        layout: ToolbarLayoutPolicy::FlexibleSpace,
        presentation: ItemPresentationOverrides::NONE,
    },
    ToolbarItemSpec {
        id: "show-hide",
        label: "Show/Hide",
        accessibility_name: "Show/Hide",
        caption_visible: true,
        graphic: SHOW_HIDE_GRAPHIC,
        behavior: ToolbarBehavior::None,
        enablement: EnablementPolicy::Disabled,
        layout: ToolbarLayoutPolicy::Measured,
        presentation: ItemPresentationOverrides::NONE,
    },
    ToolbarItemSpec {
        id: "export-text",
        label: "Export Text",
        accessibility_name: "Export Text",
        caption_visible: true,
        graphic: ToolbarGraphic::Icon(IconSpec::new(ICON_UPLOAD_FILE)),
        behavior: ToolbarBehavior::None,
        enablement: EnablementPolicy::Disabled,
        layout: ToolbarLayoutPolicy::Measured,
        presentation: ItemPresentationOverrides::NONE,
    },
    action(
        "export-pdf",
        "Export PDF",
        ICON_PICTURE_AS_PDF,
        ToolbarAction::ExportPdf,
    ),
    ToolbarItemSpec {
        id: "flexible-space-trailing",
        label: "Flexible Space",
        accessibility_name: "Flexible Space",
        caption_visible: false,
        graphic: ToolbarGraphic::None,
        behavior: ToolbarBehavior::None,
        enablement: EnablementPolicy::Enabled,
        layout: ToolbarLayoutPolicy::FlexibleSpace,
        presentation: ItemPresentationOverrides::NONE,
    },
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

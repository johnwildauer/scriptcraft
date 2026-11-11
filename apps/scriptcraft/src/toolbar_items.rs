use eframe::egui::Color32;
use scriptcraft_commands::CommandId;
use crate::material_font::{
    MaterialIconAxes, MaterialIconAxisOverrides,
};
use egui_material_icons::{
    icons::{
        ICON_DRAFT, ICON_DOUBLE_ARROW, ICON_UPLOAD_FILE, ICON_FILE_OPEN,
        ICON_KEYBOARD_ARROW_DOWN, ICON_PICTURE_AS_PDF, ICON_SAVE, ICON_SAVE_AS,
        ICON_VISIBILITY, ICON_BOOK_5, ICON_CANCEL, ICON_FORMAT_ALIGN_LEFT,
        ICON_FORMAT_BOLD, ICON_FORMAT_ITALIC, ICON_FORMAT_UNDERLINED,
        ICON_TEXT_FIELDS, ICON_FORMAT_STRIKETHROUGH,
    },
    MaterialIcon,
};

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

    #[allow(dead_code)] // Catalog capability for declaring per-icon native font axes.
    pub(super) const fn with_material_axes(
        mut self,
        axes: MaterialIconAxisOverrides,
    ) -> Self {
        self.material_axes = Some(axes);
        self
    }
}

/// Returns non-default optical-size/grade combinations explicitly used by the
/// current icon catalog so startup registers only the faces the catalog needs.
pub(super) fn custom_material_icon_axes() -> Vec<MaterialIconAxes> {
    let mut axes = Vec::new();
    for item in TEMPORARY_TOOLBAR_ITEMS {
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
    if let ToolbarGraphic::Icon(icon) = OVERFLOW_ITEM_SPEC.graphic {
        add_custom_axes(icon, &mut axes);
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
    Action(CommandId),
    ElementsRoleDropdown,
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

const FLEXIBLE_CHILD_SPACE: ToolbarItemSpec = ToolbarItemSpec {
    id: "title-page-flexible-space",
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

pub(super) const TITLE_PAGE_TOOLBAR_ITEMS: [ToolbarItemSpec; 6] = [
    FLEXIBLE_CHILD_SPACE,
    STYLE_INLINE_GROUP_ITEM,
    ALIGNMENT_INLINE_GROUP_ITEM,
    FLEXIBLE_CHILD_SPACE,
    CLOSE_TITLE_PAGE_ITEM,
    OVERFLOW_ITEM_SPEC,
];

pub(super) const TEMPORARY_TOOLBAR_ITEMS: [ToolbarItemSpec; 14] = [
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
    action("new", "New", ICON_DRAFT, CommandId::FileNew),
    action("open", "Open", ICON_FILE_OPEN, CommandId::FileOpenRequest),
    action("save", "Save", ICON_SAVE, CommandId::FileSave),
    action("save-as", "Save As", ICON_SAVE_AS, CommandId::FileSaveAsRequest),
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
    action(
        "title-page",
        "Title Page",
        ICON_BOOK_5,
        CommandId::WindowTitlePageOpen,
    ),
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
        CommandId::FileSaveAsPdf,
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

#[cfg(test)]
mod tests {
    use super::{
        ToolbarBehavior, ToolbarGraphic, TITLE_PAGE_TOOLBAR_ITEMS,
        TEMPORARY_TOOLBAR_ITEMS,
    };
    use egui_material_icons::icons::{
        ICON_BOOK_5, ICON_CANCEL, ICON_FORMAT_ALIGN_CENTER, ICON_FORMAT_ALIGN_JUSTIFY,
        ICON_FORMAT_ALIGN_LEFT, ICON_FORMAT_ALIGN_RIGHT, ICON_FORMAT_BOLD,
        ICON_FORMAT_ITALIC, ICON_FORMAT_STRIKETHROUGH, ICON_FORMAT_UNDERLINED,
        ICON_TEXT_FIELDS,
    };
    use scriptcraft_commands::CommandId;

    #[test]
    fn title_page_is_book_5_captioned_accessible_action_immediately_before_elements() {
        let title_index = TEMPORARY_TOOLBAR_ITEMS
            .iter()
            .position(|item| item.id == "title-page");
        assert!(title_index.is_some());
        let Some(title_index) = title_index else { return };
        let title = TEMPORARY_TOOLBAR_ITEMS[title_index];
        let elements = TEMPORARY_TOOLBAR_ITEMS.get(title_index + 1);

        assert_eq!(title.label, "Title Page");
        assert_eq!(title.accessibility_name, "Title Page");
        assert!(title.caption_visible);
        assert_eq!(title.enablement, super::EnablementPolicy::Enabled);
        assert_eq!(title.behavior, ToolbarBehavior::Action(CommandId::WindowTitlePageOpen));
        assert!(matches!(title.graphic, ToolbarGraphic::Icon(icon) if icon.icon.codepoint == ICON_BOOK_5.codepoint));
        assert_eq!(elements.map(|item| item.id), Some("elements"));
    }

    #[test]
    fn disabled_placeholders_remain_commandless_and_disabled() {
        for id in ["show-hide", "export-text"] {
            let item = TEMPORARY_TOOLBAR_ITEMS
                .iter()
                .find(|item| item.id == id);
            assert!(item.is_some());
            if let Some(item) = item {
                assert_eq!(item.enablement, super::EnablementPolicy::Disabled);
                assert_eq!(item.behavior, ToolbarBehavior::None);
            }
        }
    }

    #[test]
    fn inline_formatting_groups_precede_title_page_and_child_keeps_close_far_right() {
        let ids = TEMPORARY_TOOLBAR_ITEMS
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>();
        let styles = ids.iter().position(|id| *id == "styles");
        let alignment = ids.iter().position(|id| *id == "alignment");
        let title = ids.iter().position(|id| *id == "title-page");
        let elements = ids.iter().position(|id| *id == "elements");
        assert!(matches!((styles, alignment, title, elements), (Some(s), Some(a), Some(t), Some(e)) if s < a && a < t && t + 1 == e));
        let elements_item = TEMPORARY_TOOLBAR_ITEMS
            .iter()
            .find(|item| item.id == "elements")
            .copied();
        assert!(matches!(elements_item, Some(item) if item.behavior == ToolbarBehavior::ElementsRoleDropdown && item.layout == super::ToolbarLayoutPolicy::FixedWidth(128.0)));
        assert_eq!(
            TITLE_PAGE_TOOLBAR_ITEMS
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            ["styles", "alignment", "title-page-flexible-space", "close-title-page", "overflow"],
        );
        assert_eq!(TITLE_PAGE_TOOLBAR_ITEMS[3].label, "Close Title Page");
        assert!(TITLE_PAGE_TOOLBAR_ITEMS[3].caption_visible);
        assert_eq!(TITLE_PAGE_TOOLBAR_ITEMS[3].accessibility_name, "Close Title Page");
        assert!(matches!(TITLE_PAGE_TOOLBAR_ITEMS[3].graphic, ToolbarGraphic::Icon(icon) if icon.icon.codepoint == ICON_CANCEL.codepoint));
    }


}

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

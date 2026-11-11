use super::*;
use super::toolbar_items::{
    EnablementPolicy, IconSpec, OVERFLOW_ITEM_SPEC, ToolbarBehavior, ToolbarGraphic,
    ToolbarIconVariant, ToolbarItemSpec, ToolbarLayoutPolicy, TEMPORARY_TOOLBAR_ITEMS,
    TITLE_PAGE_TOOLBAR_ITEMS,
};
use super::app_commands::{command_for_id, CommandInvocation};
use super::app_commands::AppCommandAvailability;
use scriptcraft_commands::{
    Command, CommandOrigin, EditorSurface, ParagraphAlignmentValue, RoleValue, TitleTextStyle,
};
use scriptcraft_core::{
    formatted_text::{project_lit_styles, project_paragraph_alignment, project_style_at_caret},
    formatting::formatting_for_role,
    ParagraphAlignment, RunBoundaryAffinity, RunStyleFlags,
};
use egui_material_icons::icons::{
    ICON_FORMAT_ALIGN_CENTER, ICON_FORMAT_ALIGN_JUSTIFY, ICON_FORMAT_ALIGN_LEFT,
    ICON_FORMAT_ALIGN_RIGHT, ICON_FORMAT_BOLD, ICON_FORMAT_ITALIC,
    ICON_FORMAT_STRIKETHROUGH, ICON_FORMAT_UNDERLINED, ICON_TEXT_FIELDS,
};

// Manual toolbar geometry controls: tune Item group width and row spacing here.
// The 46-point normal slot is the compact baseline recorded by the focused
// geometry tests; labels may still expand a slot when active font metrics need it.
pub(super) const TOOLBAR_NORMAL_ITEM_WIDTH: f32 = 46.0;
pub(super) const TOOLBAR_GROUP_HORIZONTAL_PADDING: f32 = 8.0;
pub(super) const TOOLBAR_ROW_PADDING: f32 = 4.0;
pub(super) const TOOLBAR_ITEM_GAP: f32 = 4.0;
pub(super) const TOOLBAR_TOP_PADDING: f32 = 4.0;
pub(super) const TOOLBAR_CONTROL_CAPTION_GAP: f32 = 1.0;
/// Reserved logical width between inline formatting choices; WO-005 paints into these slots.
pub(super) const TOOLBAR_FORMATTING_SEPARATOR_SLOT_WIDTH: f32 = 2.0;
const TOOLBAR_FORMATTING_SEPARATOR_LENGTH: f32 = 15.0;
pub(super) const TOOLBAR_ELEMENTS_CAPTION_VERTICAL_OFFSET: f32 = 0.0;
// Explicit owner for trailing toolbar allocation; exact panel sizing includes this value.
pub(super) const TOOLBAR_BOTTOM_PADDING: f32 = 4.0;
/// Local width for the toolbar's explicitly painted bottom divider.
pub(super) const TOOLBAR_BOTTOM_BORDER_WIDTH: f32 = 1.5;

#[derive(Debug, Clone, Copy)]
pub(super) struct ToolbarIconHoverPadding {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

impl ToolbarIconHoverPadding {
    pub(super) const fn uniform(value: f32) -> Self {
        Self {
            left: value,
            top: value,
            right: value,
            bottom: value,
        }
    }

    // egui's Button spacing accepts symmetric x/y padding, so average opposite
    // sides to preserve the total extent when this value is used by a menu button.
    fn for_button(self) -> Vec2 {
        Vec2::new(
            (self.left + self.right) * 0.5,
            (self.top + self.bottom) * 0.5,
        )
    }
}

/// Per-edge hit/hover expansion: left, top, right, bottom (logical points).
pub(super) const TOOLBAR_ICON_HOVER_PADDING: ToolbarIconHoverPadding =
    ToolbarIconHoverPadding {
        left: 8.0,
        top: 0.0,
        right: 8.0,
        bottom: 3.0,
    };

/// Toolbar-local visual defaults. These settings are deliberately separate
/// from status-bar presentation and all dimensions are logical points.
mod toolbar_presentation {
    use super::{IconSpec, ToolbarIconVariant, ToolbarItemSpec};
    use eframe::egui::{self, Color32};

    /// Optional enabled foreground for toolbar captions and icons; `None` follows Appearance.
    pub(super) const ENABLED_FOREGROUND_OVERRIDE: Option<Color32> = None;
    /// Optional disabled foreground; `None` uses explicit custom-color alpha or the toolbar palette.
    pub(super) const DISABLED_FOREGROUND_OVERRIDE: Option<Color32> = None;
    /// Alpha multiplier used only when an explicit enabled-color override lacks a disabled color.
    pub(super) const DISABLED_OPACITY_MULTIPLIER: f32 = 0.45;
    /// Agent-default Material glyph size, initially matching the existing Body glyph size.
    pub(super) const DEFAULT_MATERIAL_ICON_POINT_SIZE: f32 = 19.0;
    /// Default Material glyph family; individual icons may override this variant.
    pub(super) const DEFAULT_MATERIAL_ICON_VARIANT: ToolbarIconVariant =
        ToolbarIconVariant::Outlined;
    /// Default toolbar caption and menu text size, in logical points.
    pub(super) const DEFAULT_ITEM_TEXT_POINT_SIZE: f32 = 11.0;
    /// Elements role-option text size, independent of toolbar item text.
    pub(super) const DEFAULT_ELEMENTS_ROLE_OPTION_POINT_SIZE: f32 = 13.0;
    /// Horizontal padding on each side of an Elements role-option row, in points.
    pub(super) const DEFAULT_ELEMENTS_ROLE_OPTION_HORIZONTAL_PADDING: f32 = 18.0;
    /// Stroke width for the closed Elements ComboBox outline, in logical points.
    pub(super) const DEFAULT_ELEMENTS_CLOSED_COMBO_OUTLINE_WIDTH: f32 = 1.0;
    /// Closed Elements ComboBox minimum height, independent of the role popup height.
    pub(super) const DEFAULT_ELEMENTS_CLOSED_COMBO_HEIGHT: f32 = 18.0;
    /// Gap between icons in grouped or primary/disclosure graphics, in logical points.
    pub(super) const DEFAULT_GRAPHIC_GAP: f32 = 0.0;

    pub(super) fn enabled_foreground(
        ui: &egui::Ui,
        item: ToolbarItemSpec,
        icon: Option<IconSpec>,
    ) -> Color32 {
        icon.and_then(|spec| spec.enabled_color)
            .or(item.presentation.enabled_color)
            .or(ENABLED_FOREGROUND_OVERRIDE)
            .unwrap_or_else(|| ui.visuals().text_color())
    }

    fn reduced_alpha_custom_foreground(enabled: Color32) -> Color32 {
        Color32::from_rgba_unmultiplied(
            enabled.r(),
            enabled.g(),
            enabled.b(),
            (f32::from(enabled.a()) * DISABLED_OPACITY_MULTIPLIER)
                .round()
                .clamp(0.0, 255.0) as u8,
        )
    }

    pub(super) fn disabled_foreground_from_overrides(
        icon_disabled: Option<Color32>,
        item_disabled: Option<Color32>,
        global_disabled: Option<Color32>,
        custom_enabled: Option<Color32>,
        ordinary_default: Color32,
    ) -> Color32 {
        icon_disabled
            .or(item_disabled)
            .or(global_disabled)
            .unwrap_or_else(|| {
                custom_enabled
                    .map(reduced_alpha_custom_foreground)
                    .unwrap_or(ordinary_default)
            })
    }

    pub(super) fn disabled_foreground(
        item: ToolbarItemSpec,
        icon: Option<IconSpec>,
        palette: crate::theme::ToolbarPalette,
    ) -> Color32 {
        let custom_enabled = icon
            .and_then(|spec| spec.enabled_color)
            .or(item.presentation.enabled_color)
            .or(ENABLED_FOREGROUND_OVERRIDE);
        disabled_foreground_from_overrides(
            icon.and_then(|spec| spec.disabled_color),
            item.presentation.disabled_color,
            DISABLED_FOREGROUND_OVERRIDE,
            custom_enabled,
            palette.disabled_foreground,
        )
    }

    pub(super) fn disabled_icon_foreground(
        item: ToolbarItemSpec,
        icon: IconSpec,
        palette: crate::theme::ToolbarPalette,
    ) -> Color32 {
        disabled_foreground(item, Some(icon), palette)
    }

    pub(super) fn disabled_caption_foreground(
        item: ToolbarItemSpec,
        palette: crate::theme::ToolbarPalette,
    ) -> Color32 {
        disabled_foreground(item, None, palette)
    }

    pub(super) fn disabled_overflow_label_foreground(
        item: ToolbarItemSpec,
        palette: crate::theme::ToolbarPalette,
    ) -> Option<Color32> {
        (item.behavior == super::ToolbarBehavior::None
            && item.enablement == super::EnablementPolicy::Disabled)
            .then(|| disabled_caption_foreground(item, palette))
    }

    pub(super) fn icon_point_size(icon: IconSpec) -> f32 {
        icon.point_size
            .unwrap_or(DEFAULT_MATERIAL_ICON_POINT_SIZE)
    }

    pub(super) fn elements_role_option_font_id(ui: &egui::Ui) -> egui::FontId {
        let mut font_id = ui
            .style()
            .text_styles
            .get(&egui::TextStyle::Button)
            .cloned()
            .unwrap_or_else(|| {
                egui::FontId::proportional(DEFAULT_ELEMENTS_ROLE_OPTION_POINT_SIZE)
            });
        font_id.size = DEFAULT_ELEMENTS_ROLE_OPTION_POINT_SIZE;
        font_id
    }

    pub(super) fn material_icon_text(icon: IconSpec) -> egui::RichText {
        let material_icon = match icon.variant.unwrap_or(DEFAULT_MATERIAL_ICON_VARIANT) {
            ToolbarIconVariant::Filled => icon.icon.filled(),
            ToolbarIconVariant::Outlined => icon.icon.outlined(),
        };
        crate::material_font::rich_text(material_icon, icon.material_axes)
    }
}

pub(super) const SUPPORTED_SCREENPLAY_ROLES: [(Role, &str); 17] = [
    (Role::SceneHeading, "Scene Heading"),
    (Role::Action, "Action"),
    (Role::Character, "Character"),
    (Role::Dialogue, "Dialogue"),
    (Role::Parenthetical, "Parenthetical"),
    (Role::Transition, "Transition"),
    (Role::Note, "Note"),
    (Role::NewAct, "New Act"),
    (Role::EndOfAct, "End of Act"),
    (Role::CastList, "Cast List"),
    (Role::General, "General"),
    (Role::Shot, "Shot"),
    (Role::Sequence, "Sequence"),
    (Role::Outline1, "Outline 1"),
    (Role::Outline2, "Outline 2"),
    (Role::Outline3, "Outline 3"),
    (Role::Summary, "Summary"),
];

pub(super) fn screenplay_role_label(role: Role) -> &'static str {
    SUPPORTED_SCREENPLAY_ROLES
        .iter()
        .find_map(|(supported, label)| (*supported == role).then_some(*label))
        .unwrap_or("Unknown")
}

/// Desired inner scroll height for all Elements rows, bounded by the usable
/// egui content viewport after accounting for the popup frame.
fn elements_role_popup_max_height(ui: &egui::Ui) -> f32 {
    let button_font = toolbar_presentation::elements_role_option_font_id(ui);
    let spacing = ui.spacing();
    let row_height = SUPPORTED_SCREENPLAY_ROLES
        .iter()
        .map(|(_, label)| {
            let text_height = ui
                .painter()
                .layout_no_wrap(
                    (*label).to_owned(),
                    button_font.clone(),
                    Color32::TRANSPARENT,
                )
                .size()
                .y;
            spacing
                .interact_size
                .y
                .max(text_height + spacing.button_padding.y * 2.0)
        })
        .sum::<f32>();
    let row_gaps = spacing.item_spacing.y
        * SUPPORTED_SCREENPLAY_ROLES.len().saturating_sub(1) as f32;
    let desired_content_height = row_height + row_gaps;

    let popup_margin = egui::Frame::popup(ui.style()).total_margin();
    let available_content_height = (ui.ctx().content_rect().height()
        - popup_margin.top
        - popup_margin.bottom)
        .max(0.0);
    desired_content_height.min(available_content_height)
}

pub(super) fn toolbar_items_for(
    availability: AppCommandAvailability,
) -> [ToolbarItemSpec; 14] {
    let mut items = TEMPORARY_TOOLBAR_ITEMS;
    apply_toolbar_enablement(&mut items, availability);
    items
}

fn title_page_toolbar_items_for(
    availability: AppCommandAvailability,
) -> [ToolbarItemSpec; 6] {
    let mut items = TITLE_PAGE_TOOLBAR_ITEMS;
    apply_toolbar_enablement(&mut items, availability);
    items
}

fn apply_toolbar_enablement(items: &mut [ToolbarItemSpec], availability: AppCommandAvailability) {
    for item in items {
        item.enablement = match item.behavior {
            ToolbarBehavior::Action(command) => {
                if availability.is_enabled(command) {
                    EnablementPolicy::Enabled
                } else {
                    EnablementPolicy::Disabled
                }
            }
            ToolbarBehavior::StylesInlineGroup | ToolbarBehavior::AlignmentInlineGroup => {
                if availability.modal_blocked {
                    EnablementPolicy::Disabled
                } else {
                    EnablementPolicy::Enabled
                }
            }
            ToolbarBehavior::None
            | ToolbarBehavior::ElementsRoleDropdown
            | ToolbarBehavior::OverflowMenu => item.enablement,
        };
    }
}

#[derive(Debug, Clone)]
pub(super) struct ToolbarAllocation {
    pub(super) visible: Vec<(ToolbarItemSpec, f32)>,
    pub(super) overflow: Vec<ToolbarItemSpec>,
    pub(super) overflow_width: f32,
    pub(super) row_width: f32,
}

pub(super) fn is_flexible_toolbar_space(item: ToolbarItemSpec) -> bool {
    matches!(item.layout, ToolbarLayoutPolicy::FlexibleSpace)
}

pub(super) fn toolbar_item_width<F, G>(
    item: ToolbarItemSpec,
    measure_label: F,
    measure_graphic: G,
) -> f32
where
    F: Fn(&str) -> f32,
    G: Fn(&ToolbarGraphic) -> f32,
{
    match item.layout {
        ToolbarLayoutPolicy::FlexibleSpace => 0.0,
        ToolbarLayoutPolicy::FixedWidth(width) | ToolbarLayoutPolicy::FixedSpace(width) => width,
        ToolbarLayoutPolicy::Measured => {
            if matches!(
                item.behavior,
                ToolbarBehavior::StylesInlineGroup | ToolbarBehavior::AlignmentInlineGroup
            ) {
                let row_width = match item.graphic {
                    ToolbarGraphic::Icons(icons) => {
                        icons.iter().copied().map(|icon| {
                            toolbar_presentation::icon_point_size(icon)
                                + TOOLBAR_ICON_HOVER_PADDING.left
                                + TOOLBAR_ICON_HOVER_PADDING.right
                        }).sum::<f32>()
                            + icons.len().saturating_sub(1) as f32
                                * TOOLBAR_FORMATTING_SEPARATOR_SLOT_WIDTH
                    }
                    _ => measure_graphic(&item.graphic),
                };
                let caption_width = if item.caption_visible {
                    measure_label(item.label) + TOOLBAR_GROUP_HORIZONTAL_PADDING
                } else {
                    0.0
                };
                return row_width.max(caption_width);
            }
            let graphic_width = measure_graphic(&item.graphic);
            let caption_width = if item.caption_visible {
                measure_label(item.label) + TOOLBAR_GROUP_HORIZONTAL_PADDING
            } else {
                0.0
            };
            TOOLBAR_NORMAL_ITEM_WIDTH.max(graphic_width).max(caption_width)
        }
    }
}

pub(super) fn toolbar_graphic_width(graphic: &ToolbarGraphic) -> f32 {
    let icon_width = |icon: IconSpec| {
        toolbar_presentation::icon_point_size(icon)
    };
    match graphic {
        ToolbarGraphic::None | ToolbarGraphic::TextDropdown => 0.0,
        ToolbarGraphic::Icon(icon) => icon_width(*icon),
        ToolbarGraphic::Icons(icons) => {
            icons.iter().copied().map(icon_width).sum::<f32>()
                + icons.len().saturating_sub(1) as f32
                    * toolbar_presentation::DEFAULT_GRAPHIC_GAP
        }
        ToolbarGraphic::IconDisclosure {
            primary,
            disclosure,
        } => {
            icon_width(*primary)
                + toolbar_presentation::DEFAULT_GRAPHIC_GAP
                + icon_width(*disclosure)
        }
    }
}

pub(super) fn toolbar_label_width(ui: &egui::Ui, label: &str) -> f32 {
    let font_id = FontId::proportional(toolbar_presentation::DEFAULT_ITEM_TEXT_POINT_SIZE);
    ui.painter()
        .layout_no_wrap(label.to_owned(), font_id, Color32::TRANSPARENT)
        .size()
        .x
}

pub(super) fn toolbar_content_width(
    widths: &[(ToolbarItemSpec, f32)],
    overflow_width: f32,
) -> f32 {
    let child_count = widths.len() as f32 + f32::from(overflow_width > 0.0);
    widths.iter().map(|(_, width)| width).sum::<f32>()
        + TOOLBAR_ROW_PADDING * 2.0
        + (child_count - 1.0).max(0.0) * TOOLBAR_ITEM_GAP
        + overflow_width
}

pub(super) fn toolbar_row_width(
    widths: &[(ToolbarItemSpec, f32)],
    overflow_width: f32,
) -> f32 {
    toolbar_content_width(widths, overflow_width)
}

pub(super) fn toolbar_actual_available_width(
    content_rect: egui::Rect,
    clip_rect: egui::Rect,
    row_left: f32,
) -> f32 {
    let actual_rect = content_rect.intersect(clip_rect);
    (actual_rect.right() - row_left).max(0.0)
}

pub(super) fn elements_closed_text_rect(
    control_rect: egui::Rect,
    button_padding: egui::Vec2,
    icon_spacing: f32,
    icon_width: f32,
) -> egui::Rect {
    let content_rect = control_rect.shrink2(button_padding);
    let text_right = (content_rect.right() - icon_spacing - icon_width).max(content_rect.left());
    egui::Rect::from_min_max(
        content_rect.left_top(),
        egui::pos2(text_right, content_rect.bottom()),
    )
}

pub(super) fn elements_closed_text_position(text_rect: egui::Rect, text_size: Vec2) -> egui::Pos2 {
    egui::Align2::CENTER_CENTER
        .align_size_within_rect(text_size, text_rect)
        .min
}

/// Allocate the transient toolbar without mutating project state.
/// Fixed spacing items have one slot; rendered items use their complete,
/// graphic- and caption-aware measured width. Flexible spaces share only the remaining
/// width once every normal item fits.
pub(super) fn allocate_toolbar<F, G>(
    catalog_items: &[ToolbarItemSpec],
    available_width: f32,
    measure_label: F,
    measure_graphic: G,
) -> ToolbarAllocation
where
    F: Fn(&str) -> f32 + Copy,
    G: Fn(&ToolbarGraphic) -> f32 + Copy,
{
    let items: Vec<_> = catalog_items
        .iter()
        .copied()
        .filter(|item| item.behavior != ToolbarBehavior::OverflowMenu)
        .collect();
    let overflow_width = catalog_items
        .iter()
        .find(|item| item.behavior == ToolbarBehavior::OverflowMenu)
        .and_then(|item| match item.layout {
            ToolbarLayoutPolicy::FixedWidth(width) => Some(width),
            _ => None,
        })
        .unwrap_or(0.0);
    let fixed_width = items
        .iter()
        .copied()
        .map(|item| toolbar_item_width(item, measure_label, measure_graphic))
        .sum::<f32>();
    let full_fixed_width = fixed_width
        + TOOLBAR_ROW_PADDING * 2.0
        + (items.len().saturating_sub(1) as f32) * TOOLBAR_ITEM_GAP;
    let flexible_count = items
        .iter()
            .filter(|item| matches!(item.layout, ToolbarLayoutPolicy::FlexibleSpace))
        .count();

    if full_fixed_width <= available_width {
        let flexible_width = if flexible_count == 0 {
            0.0
        } else {
            ((available_width - full_fixed_width) / flexible_count as f32).max(0.0)
        };
        return ToolbarAllocation {
            visible: items
                .iter()
                .map(|item| {
                    let width = if is_flexible_toolbar_space(*item) {
                        flexible_width
                    } else {
                        toolbar_item_width(*item, measure_label, measure_graphic)
                    };
                    (*item, width)
                })
                .collect(),
            overflow: Vec::new(),
            overflow_width: 0.0,
            row_width: available_width,
        };
    }

    let mut visible = Vec::new();
    let mut overflow = Vec::new();
    let mut fitting = true;
    for item in items.iter().copied() {
        if is_flexible_toolbar_space(item) {
            continue;
        }
        if matches!(item.layout, ToolbarLayoutPolicy::FixedSpace(_)) {
            let mut candidate = visible.clone();
            candidate.push((
                item,
                toolbar_item_width(item, measure_label, measure_graphic),
            ));
            if fitting
                && toolbar_content_width(
                    &candidate,
                    overflow_width,
                ) <= available_width
            {
                visible.push((
                    item,
                    toolbar_item_width(item, measure_label, measure_graphic),
                ));
            }
            continue;
        }
        let mut candidate = visible.clone();
        candidate.push((
            item,
            toolbar_item_width(item, measure_label, measure_graphic),
        ));
        if fitting && toolbar_content_width(&candidate, overflow_width) <= available_width {
            visible.push((
                item,
                toolbar_item_width(item, measure_label, measure_graphic),
            ));
        } else {
            fitting = false;
            overflow.push(item);
        }
    }

    let row_width = toolbar_row_width(&visible, overflow_width);
    ToolbarAllocation {
        visible,
        overflow,
        overflow_width,
        row_width,
    }
}

fn toolbar_icon_galley(
    ui: &egui::Ui,
    icon: IconSpec,
    color: Color32,
) -> std::sync::Arc<egui::epaint::text::Galley> {
    let point_size = toolbar_presentation::icon_point_size(icon);
    WidgetText::from(
        toolbar_presentation::material_icon_text(icon)
            .size(point_size)
            .color(color),
    )
    .into_galley(ui, None, point_size, TextStyle::Button)
}

fn paint_toolbar_galley_with_color(
    ui: &egui::Ui,
    position: egui::Pos2,
    galley: std::sync::Arc<egui::epaint::text::Galley>,
    color: Color32,
) {
    ui.painter()
        .galley_with_override_text_color(position, galley, color);
}

fn paint_disabled_toolbar_overflow_label(
    ui: &mut egui::Ui,
    item: ToolbarItemSpec,
    color: Color32,
) -> egui::Rect {
    let mut layout_job = WidgetText::from(RichText::new(item.label).color(color))
        .into_layout_job(ui.style(), egui::FontSelection::Default, ui.text_valign())
        .as_ref()
        .clone();
    let available_width = ui.available_width();
    match ui.wrap_mode() {
        egui::TextWrapMode::Extend => layout_job.wrap.max_width = f32::INFINITY,
        egui::TextWrapMode::Wrap => layout_job.wrap.max_width = available_width,
        egui::TextWrapMode::Truncate => {
            layout_job.wrap.max_width = available_width;
            layout_job.wrap.max_rows = 1;
            layout_job.wrap.break_anywhere = true;
        }
    }
    layout_job.halign = ui.layout().horizontal_placement();
    layout_job.justify = ui.layout().horizontal_justify();
    let galley = ui.painter().layout_job(layout_job);
    let (_, rect) = ui.allocate_space(galley.size());
    let position = match galley.job.halign {
        egui::Align::LEFT => rect.left_top(),
        egui::Align::Center => rect.center_top(),
        egui::Align::RIGHT => rect.right_top(),
    };
    if ui.is_rect_visible(rect) {
        paint_toolbar_galley_with_color(ui, position, galley, color);
    }

    let accessibility_id = ui.make_persistent_id(("disabled-toolbar-overflow-label", item.id));
    ui.ctx().accesskit_node_builder(accessibility_id, |node| {
        node.set_role(egui::accesskit::Role::Label);
        node.set_label(item.accessibility_name.to_owned());
        node.set_bounds(egui::accesskit::Rect {
            x0: rect.min.x.into(),
            y0: rect.min.y.into(),
            x1: rect.max.x.into(),
            y1: rect.max.y.into(),
        });
    });
    rect
}

fn is_disabled_overflow_label(item: ToolbarItemSpec) -> bool {
    matches!(
        (item.behavior, item.enablement),
        (ToolbarBehavior::None, EnablementPolicy::Disabled)
    )
}

fn paint_toolbar_icon(
    ui: &egui::Ui,
    center: egui::Pos2,
    icon: IconSpec,
    item: ToolbarItemSpec,
    palette: crate::theme::ToolbarPalette,
) {
    let color = if matches!(item.enablement, EnablementPolicy::Enabled) {
        toolbar_presentation::enabled_foreground(ui, item, Some(icon))
    } else {
        toolbar_presentation::disabled_icon_foreground(item, icon, palette)
    };
    let galley = toolbar_icon_galley(ui, icon, color);
    paint_toolbar_galley_with_color(ui, center - galley.size() / 2.0, galley, color);
}

fn paint_toolbar_graphic(
    ui: &egui::Ui,
    center: egui::Pos2,
    graphic: ToolbarGraphic,
    item: ToolbarItemSpec,
    palette: crate::theme::ToolbarPalette,
) {
    let paint_group = |icons: &[IconSpec]| {
        let total_width = icons
            .iter()
            .copied()
            .map(toolbar_presentation::icon_point_size)
            .sum::<f32>()
            + icons.len().saturating_sub(1) as f32
                * toolbar_presentation::DEFAULT_GRAPHIC_GAP;
        let mut x = center.x - total_width / 2.0;
        for icon in icons.iter().copied() {
            let point_size = toolbar_presentation::icon_point_size(icon);
            paint_toolbar_icon(
                ui,
                egui::pos2(x + point_size / 2.0, center.y),
                icon,
                item,
                palette,
            );
            x += point_size + toolbar_presentation::DEFAULT_GRAPHIC_GAP;
        }
    };
    match graphic {
        ToolbarGraphic::None | ToolbarGraphic::TextDropdown => {}
        ToolbarGraphic::Icon(icon) => paint_toolbar_icon(ui, center, icon, item, palette),
        ToolbarGraphic::Icons(icons) => paint_group(icons),
        ToolbarGraphic::IconDisclosure {
            primary,
            disclosure,
        } => paint_group(&[primary, disclosure]),
    }
}

impl ScriptCraftApp {
    pub(super) fn toolbar_action_button(
        &mut self,
        ui: &mut egui::Ui,
        item: ToolbarItemSpec,
        width: f32,
        presentation: ToolbarPresentationMetrics,
    ) {
        let palette = self.theme_palette.toolbar;
        let group_height = toolbar_item_group_height(ui.style());
        let (id, rect) = ui.allocate_space(Vec2::new(width, group_height));
        let icon_control_size = Vec2::splat(presentation.icon_control_size);
        let icon_control_rect = toolbar_icon_control_rect(rect, icon_control_size);
        let icon_rect = toolbar_icon_hover_rect(icon_control_rect, TOOLBAR_ICON_HOVER_PADDING);
        let enabled = toolbar_action_enabled(item);
        let response = ui
            .interact(
                icon_rect,
                id.with("icon"),
                toolbar_action_sense(enabled),
            )
            .on_hover_cursor(egui::CursorIcon::Default);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                enabled,
                item.accessibility_name.to_owned(),
            )
        });
        let fill = if enabled {
            toolbar_action_fill(ui.style(), enabled, response.hovered())
        } else {
            Color32::TRANSPARENT
        };
        if fill != Color32::TRANSPARENT {
            ui.painter().rect_filled(icon_rect, 2.0, fill);
        }
        let body_font = presentation.font_id();
        paint_toolbar_graphic(ui, icon_control_rect.center(), item.graphic, item, palette);
        if item.caption_visible {
            let label_color = if enabled {
                toolbar_presentation::enabled_foreground(ui, item, None)
            } else {
                toolbar_presentation::disabled_caption_foreground(item, palette)
            };
            let label_galley = ui.painter().layout_no_wrap(
                item.label.to_owned(),
                body_font,
                label_color,
            );
            let caption_rect = toolbar_caption_rect(
                rect,
                icon_control_rect.height(),
                TOOLBAR_CONTROL_CAPTION_GAP,
            );
            let label_center = egui::pos2(
                caption_rect.center().x,
                caption_rect.center().y,
            );
            paint_toolbar_galley_with_color(
                ui,
                label_center - label_galley.size() / 2.0,
                label_galley,
                label_color,
            );
        }
        if let Some(action) = toolbar_action_activation(item, response.clicked()) {
            if let Some(command) = command_for_id(action) {
                self.dispatch_command(
                    CommandInvocation::new(command, CommandOrigin::Toolbar),
                    ui.ctx(),
                );
            }
        }
    }

    fn selected_editor_state(
        &self,
        surface: EditorSurface,
    ) -> (RunStyleFlags, Option<ParagraphAlignment>) {
        let project = self.session.project();
        let selection = match surface {
            EditorSurface::Body => self.session.selection(),
            EditorSurface::TitlePage => self.session.title_selection(),
        };
        let typing = match surface {
            EditorSurface::Body => self.session.body_typing_style(),
            EditorSurface::TitlePage => self.session.title_typing_style(),
        };
        let (start, end) = ordered_editor_selection(selection);
        let style = if start == end {
            let paragraph = match surface {
                EditorSurface::Body => project
                    .elements
                    .get(start.element)
                    .map(|element| (&element.text, &element.runs)),
                EditorSurface::TitlePage => project
                    .title_page
                    .get(start.element)
                    .map(|paragraph| (&paragraph.text, &paragraph.runs)),
            };
            paragraph
                .and_then(|(text, runs)| {
                    project_style_at_caret(
                        text,
                        runs,
                        start.offset,
                        RunBoundaryAffinity::Right,
                        typing,
                    )
                    .ok()
                })
                .unwrap_or(typing)
        } else {
            let mut projected = RunStyleFlags::default();
            for index in start.element..=end.element {
                let paragraph = match surface {
                    EditorSurface::Body => project
                        .elements
                        .get(index)
                        .map(|element| (&element.text, &element.runs)),
                    EditorSurface::TitlePage => project
                        .title_page
                        .get(index)
                        .map(|paragraph| (&paragraph.text, &paragraph.runs)),
                };
                let Some((text, runs)) = paragraph else { continue };
                let range_start = if index == start.element { start.offset } else { 0 };
                let range_end = if index == end.element { end.offset } else { text.len() };
                if range_start >= range_end {
                    continue;
                }
                if let Ok(flags) = project_lit_styles(text, runs, range_start..range_end, typing) {
                    projected = style_union(projected, flags);
                }
            }
            projected
        };

        let first_paragraph = start.element;
        let last_paragraph = if start == end { start.element } else { end.element };
        let alignments = (first_paragraph..=last_paragraph)
            .map(|index| match surface {
                EditorSurface::Body => project.elements.get(index).map(|element| {
                    let role = formatting_for_role(element.role).paragraph.alignment;
                    let default = ParagraphAlignment::from_fdx(role)
                        .unwrap_or(ParagraphAlignment::Left);
                    let source = project
                        .preservation
                        .as_ref()
                        .map(|preservation| preservation.source_layout_settings());
                    let role_override = source
                        .map(|settings| settings.paragraph_spec(element.role).alignment)
                        .flatten();
                    let source_direct = source
                        .map(|settings| settings.direct_paragraph_spec(&element.id).paragraph.alignment)
                        .flatten();
                    let semantic_direct = project
                        .direct_paragraph_overrides
                        .get(&element.id)
                        .and_then(|overrides| overrides.paragraph.alignment);
                    semantic_direct
                        .or(source_direct)
                        .or(role_override)
                        .unwrap_or(default)
                }),
                EditorSurface::TitlePage => project.title_page.get(index).map(|paragraph| {
                    paragraph
                        .formatting
                        .alignment
                        .map(ParagraphAlignment::from)
                        .unwrap_or(ParagraphAlignment::Left)
                }),
            })
            .collect::<Vec<_>>();
        let alignment = project_paragraph_alignment(&alignments).alignment;
        (style, alignment)
    }

    fn dispatch_formatting_choice(
        &mut self,
        choice: FormattingChoice,
        surface: EditorSurface,
        ctx: &egui::Context,
    ) {
        self.dispatch_command(
            CommandInvocation::new(formatting_choice_command(choice, surface), CommandOrigin::Toolbar),
            ctx,
        );
    }

    fn formatting_choice_row(
        &self,
        ui: &mut egui::Ui,
        item: ToolbarItemSpec,
        choice: FormattingChoiceSpec,
        active: bool,
        enabled: bool,
    ) -> bool {
        const ROW_HEIGHT: f32 = 28.0;
        const ICON_SIZE: f32 = 18.0;
        let row_width = (ui.available_width() - 4.0).max(120.0);
        let response = ui.add_enabled(
            enabled,
            egui::Button::new("").frame(false).min_size(Vec2::new(row_width, ROW_HEIGHT)),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                enabled,
                choice.label.to_owned(),
            )
        });
        let response = response.on_hover_cursor(toolbar_cursor_icon());
        let fill = toolbar_group_fill(ui.style(), active || response.hovered());
        if fill != Color32::TRANSPARENT {
            ui.painter().rect_filled(response.rect, 2.0, fill);
        }
        let center = egui::pos2(
            response.rect.left() + ui.spacing().button_padding.x + ICON_SIZE / 2.0,
            response.rect.center().y,
        );
        paint_toolbar_icon(ui, center, choice.icon, item, self.theme_palette.toolbar);
        let label_color = toolbar_presentation::enabled_foreground(ui, item, None);
        let label = ui.painter().layout_no_wrap(
            choice.label.to_owned(),
            FontId::proportional(13.0),
            label_color,
        );
        paint_toolbar_galley_with_color(
            ui,
            egui::pos2(center.x + ICON_SIZE / 2.0 + 8.0, response.rect.center().y - label.size().y / 2.0),
            label,
            label_color,
        );
        response.clicked()
    }

    fn formatting_inline_group_button(
        &mut self,
        ui: &mut egui::Ui,
        item: ToolbarItemSpec,
        width: f32,
        presentation: ToolbarPresentationMetrics,
        surface: EditorSurface,
    ) {
        let enabled = item.enablement == EnablementPolicy::Enabled;
        let (styles, alignment) = self.selected_editor_state(surface);
        let style_choices = matches!(item.behavior, ToolbarBehavior::StylesInlineGroup);
        let choices = if style_choices {
            &STYLE_CHOICES[..]
        } else {
            &ALIGNMENT_CHOICES[..]
        };
        let group_height = toolbar_item_group_height(ui.style());
        let group_id = ui.id().with(item.id);
        let (_layout_id, group_rect) = ui.allocate_space(Vec2::new(width, group_height));
        let geometry = formatting_group_geometry(
            group_rect,
            choices,
            presentation.icon_control_size,
            TOOLBAR_CONTROL_CAPTION_GAP,
        );
        let mut chosen = None;
        for (index, choice) in choices.iter().copied().enumerate() {
            let active = formatting_choice_active(choice.choice, styles, alignment);
            let response = ui
                .interact(
                    geometry.interaction_rects[index],
                    group_id.with(("formatting-choice", choice.label)),
                    if enabled { Sense::click() } else { Sense::hover() },
                )
                .on_hover_cursor(if enabled {
                    toolbar_cursor_icon()
                } else {
                    egui::CursorIcon::Default
                });
            response.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::WidgetType::Button,
                    enabled,
                    active,
                    choice.label.to_owned(),
                )
            });
            let fill = formatting_choice_fill(
                enabled,
                active,
                response.hovered(),
                response.is_pointer_button_down_on(),
                self.theme_palette.toolbar,
            );
            if fill != Color32::TRANSPARENT {
                ui.painter().rect_filled(response.rect, 2.0, fill);
            }
            paint_toolbar_icon(
                ui,
                geometry.icon_centers[index],
                choice.icon,
                item,
                self.theme_palette.toolbar,
            );
            let keyboard_activated = response.has_focus()
                && ui.input(|input| {
                    input.key_pressed(egui::Key::Enter)
                        || input.key_pressed(egui::Key::Space)
                });
            if response.clicked() || keyboard_activated {
                chosen = Some(choice.choice);
            }
        }
        paint_formatting_group_separators(ui, &geometry, self.theme_palette.toolbar);
        if item.caption_visible {
            let caption = ui.painter().layout_no_wrap(
                item.label.to_owned(),
                presentation.font_id(),
                toolbar_presentation::enabled_foreground(ui, item, None),
            );
            paint_toolbar_galley_with_color(
                ui,
                geometry.caption_rect.center() - caption.size() / 2.0,
                caption,
                toolbar_presentation::enabled_foreground(ui, item, None),
            );
        }
        if let Some(choice) = chosen {
            self.dispatch_formatting_choice(choice, surface, ui.ctx());
        }
    }

    fn formatting_overflow_submenu(
        &mut self,
        ui: &mut egui::Ui,
        item: ToolbarItemSpec,
        surface: EditorSurface,
    ) {
        let (styles, alignment) = self.selected_editor_state(surface);
        let choices = if matches!(item.behavior, ToolbarBehavior::StylesInlineGroup) {
            &STYLE_CHOICES[..]
        } else {
            &ALIGNMENT_CHOICES[..]
        };
        let mut chosen = None;
        ui.menu_button(item.label, |ui| {
            for choice in choices.iter().copied() {
                if self.formatting_choice_row(
                    ui,
                    item,
                    choice,
                    formatting_choice_active(choice.choice, styles, alignment),
                    item.enablement == EnablementPolicy::Enabled,
                ) {
                    chosen = Some(choice.choice);
                    ui.close();
                }
            }
        });
        if let Some(choice) = chosen {
            self.dispatch_formatting_choice(choice, surface, ui.ctx());
        }
    }

    pub(super) fn focused_role(&self) -> Option<Role> {
        self.session
            .project()
            .elements
            .get(self.session.selection().focus.element)
            .map(|element| element.role)
    }

    pub(super) fn change_role_from_toolbar(&mut self, role: Role, ctx: &egui::Context) {
        self.dispatch_command(
            CommandInvocation::new(
                Command::EditorElementChange {
                    role: RoleValue::new(role),
                    surface: EditorSurface::Body,
                },
                CommandOrigin::Toolbar,
            ),
            ctx,
        );
    }

    pub(super) fn elements_toolbar_item(
        &mut self,
        ui: &mut egui::Ui,
        item: ToolbarItemSpec,
        width: f32,
        presentation: ToolbarPresentationMetrics,
    ) {
        let selected_role = self.focused_role();
        let mut next_role = selected_role.unwrap_or(Role::Action);
        let group_height = toolbar_item_group_height(ui.style());
        let group_response = ui.allocate_ui_with_layout(
            Vec2::new(width, group_height),
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                // ComboBox has an intrinsic minimum. Constrain this child to the
                // same explicit budget used by allocate_toolbar so it cannot
                // advance the parent row past the allocated Elements slot.
                ui.set_width(width);
                let group_rect = ui.max_rect();
                let vertical_item_spacing = ui.spacing().item_spacing.y;
                let original_interact_height = ui.spacing().interact_size.y;
                let role_popup_height = elements_role_popup_max_height(ui);
                // The closed control is followed by explicitly budgeted caption
                // allocations, so do not let ComboBox's post-widget spacing push
                // those allocations beyond this Item group's declared height.
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.spacing_mut().interact_size.y =
                    toolbar_presentation::DEFAULT_ELEMENTS_CLOSED_COMBO_HEIGHT;
                let combo_result = egui::ComboBox::from_id_salt(item.id)
                    .width(width)
                    .height(role_popup_height)
                    .wrap_mode(TextWrapMode::Truncate)
                    // Keep the stock ComboBox button, popup, ID, and keyboard behavior,
                    // but replace its triangle with the outlined up/down Material glyph.
                    .icon(|ui, icon_rect, visuals, _is_open| {
                        let icon_text = crate::material_font::rich_text(
                            egui_material_icons::icons::ICON_UNFOLD_MORE.outlined(),
                            None,
                        )
                        .size(icon_rect.height());
                        let galley = WidgetText::from(icon_text).into_galley(
                            ui,
                            None,
                            icon_rect.width(),
                            TextStyle::Button,
                        );
                        ui.painter().galley(
                            icon_rect.center() - galley.size() / 2.0,
                            galley,
                            visuals.fg_stroke.color,
                        );
                    })
                    .selected_text("")
                    .show_ui(ui, |ui| {
                        // Popup role rows retain the normal vertical rhythm even
                        // though the closed control/caption composition does not.
                        // The closed button's local height must not resize popup rows.
                        ui.spacing_mut().item_spacing.y = vertical_item_spacing;
                        ui.spacing_mut().interact_size.y = original_interact_height;
                        ui.spacing_mut().button_padding.x =
                            toolbar_presentation::DEFAULT_ELEMENTS_ROLE_OPTION_HORIZONTAL_PADDING;
                        let role_option_font =
                            toolbar_presentation::elements_role_option_font_id(ui);
                        for (role, label) in SUPPORTED_SCREENPLAY_ROLES {
                            ui.selectable_value(
                                &mut next_role,
                                role,
                                RichText::new(label).font(role_option_font.clone()),
                            );
                        }
                    });
                // Do not let the closed-control height affect the caption or other
                // controls in this Item group.
                ui.spacing_mut().interact_size.y = original_interact_height;
                let popup_open = combo_result.inner.is_some();
                let combo_response = combo_result.response.on_hover_cursor(toolbar_cursor_icon());
                if !popup_open {
                    let text_rect = elements_closed_text_rect(
                        combo_response.rect,
                        ui.spacing().button_padding,
                        ui.spacing().icon_spacing,
                        ui.spacing().icon_width,
                    );
                    let text_color = toolbar_presentation::enabled_foreground(ui, item, None);
                    let galley = WidgetText::from(
                        RichText::new(screenplay_role_label(next_role))
                            .font(toolbar_presentation::elements_role_option_font_id(ui))
                            .color(text_color),
                    )
                    .into_galley(
                        ui,
                        Some(TextWrapMode::Truncate),
                        text_rect.width(),
                        TextStyle::Button,
                    );
                    let text_position = elements_closed_text_position(text_rect, galley.size());
                    ui.painter().galley(text_position, galley, text_color);
                    // Paint only the closed control's outline, using the app-owned
                    // toolbar hover foreground without changing widget visuals, the
                    // separate hover fill, or popup role rows.
                    ui.painter().rect_stroke(
                        combo_response.rect,
                        ui.visuals().widgets.inactive.corner_radius,
                        egui::Stroke::new(
                            toolbar_presentation::DEFAULT_ELEMENTS_CLOSED_COMBO_OUTLINE_WIDTH,
                            self.theme_palette.toolbar.hover_fill,
                        ),
                        egui::StrokeKind::Inside,
                    );
                }
                // This caption is decorative. Allocate/paint it instead of adding a
                // label response that can compete with the ComboBox cursor owner.
                let remaining_caption_height = (group_height
                    - combo_response.rect.height()
                    - TOOLBAR_CONTROL_CAPTION_GAP)
                    .max(0.0);
                let (_gap_id, _gap_rect) = ui.allocate_space(Vec2::new(
                    width,
                    TOOLBAR_CONTROL_CAPTION_GAP,
                ));
                let (_, label_rect) =
                    ui.allocate_space(Vec2::new(width, remaining_caption_height));
                let label_font = presentation.font_id();
                let caption_rect = toolbar_caption_rect(
                    group_rect,
                    presentation.icon_control_size,
                    TOOLBAR_CONTROL_CAPTION_GAP,
                )
                .translate(Vec2::new(0.0, TOOLBAR_ELEMENTS_CAPTION_VERTICAL_OFFSET));
                ui.painter().text(
                    caption_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    item.label,
                    label_font,
                    toolbar_presentation::enabled_foreground(ui, item, None),
                );
                let _ = label_rect;
                debug_assert!(combo_response.rect.width() <= width + f32::EPSILON);
            },
        );
        // Own the label-only portion at the group boundary without making the
        // decorative label itself interactive or changing ComboBox routing.
        group_response
            .response
            .on_hover_cursor(toolbar_cursor_icon());
        if selected_role != Some(next_role) {
            self.change_role_from_toolbar(next_role, ui.ctx());
        }
    }

    pub(super) fn toolbar_overflow_menu(
        &mut self,
        ui: &mut egui::Ui,
        indicator: ToolbarItemSpec,
        items: &[ToolbarItemSpec],
        surface: EditorSurface,
    ) {
        let mut chosen_action = None;
        let mut chosen_role = None;
        let icon = match indicator.graphic {
            ToolbarGraphic::Icon(icon) => icon,
            _ => unreachable!("overflow indicator must use a single Material icon"),
        };
        let icon_color = toolbar_presentation::enabled_foreground(ui, indicator, Some(icon));
        ui.spacing_mut().button_padding = TOOLBAR_ICON_HOVER_PADDING.for_button();
        ui.visuals_mut().widgets.hovered.corner_radius = egui::CornerRadius::same(2);
        ui.visuals_mut().widgets.active.corner_radius = egui::CornerRadius::same(2);
        let menu_result = ui.menu_button(
            toolbar_presentation::material_icon_text(icon)
                .size(toolbar_presentation::icon_point_size(icon))
                .color(icon_color),
            |ui| {
            for item in items.iter().copied() {
                match item.behavior {
                    ToolbarBehavior::Action(action) => {
                        if ui
                            .add_enabled(
                                item.enablement == EnablementPolicy::Enabled,
                                egui::Button::new(item.label),
                            )
                            .clicked()
                        {
                            chosen_action = Some(action);
                            ui.close();
                        }
                    }
                    ToolbarBehavior::ElementsRoleDropdown => {
                        let selected_role = self.focused_role();
                        ui.menu_button(item.label, |ui| {
                            ui.spacing_mut().button_padding.x = toolbar_presentation::
                                DEFAULT_ELEMENTS_ROLE_OPTION_HORIZONTAL_PADDING;
                            let role_popup_height = elements_role_popup_max_height(ui);
                            let role_option_font =
                                toolbar_presentation::elements_role_option_font_id(ui);
                            egui::ScrollArea::vertical()
                                .max_height(role_popup_height)
                                .show(ui, |ui| {
                                    for (role, label) in SUPPORTED_SCREENPLAY_ROLES {
                                        if ui
                                            .selectable_label(
                                                selected_role == Some(role),
                                                RichText::new(label)
                                                    .font(role_option_font.clone()),
                                            )
                                            .clicked()
                                        {
                                            chosen_role = Some(role);
                                            ui.close();
                                        }
                                    }
                                });
                        });
                    }
                    ToolbarBehavior::StylesInlineGroup | ToolbarBehavior::AlignmentInlineGroup => {
                        if item.enablement == EnablementPolicy::Enabled {
                            self.formatting_overflow_submenu(ui, item, surface);
                        } else {
                            ui.add_enabled(false, egui::Button::new(item.label));
                        }
                    }
                    ToolbarBehavior::None if is_disabled_overflow_label(item) => {
                        if let Some(color) =
                            toolbar_presentation::disabled_overflow_label_foreground(
                                item,
                                self.theme_palette.toolbar,
                            )
                        {
                            paint_disabled_toolbar_overflow_label(ui, item, color);
                        }
                    }
                    ToolbarBehavior::None | ToolbarBehavior::OverflowMenu => {}
                }
            }
            },
        );
        let response = menu_result.response.on_hover_cursor(egui::CursorIcon::Default);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                true,
                indicator.accessibility_name.to_owned(),
            )
        });
        if let Some(action) = chosen_action {
            if let Some(command) = command_for_id(action) {
                self.dispatch_command(
                    CommandInvocation::new(command, CommandOrigin::Toolbar),
                    ui.ctx(),
                );
            }
        }
        if let Some(role) = chosen_role {
            self.change_role_from_toolbar(role, ui.ctx());
        }
    }

    pub(super) fn toolbar(&mut self, ui: &mut egui::Ui) {
        self.toolbar_for_surface(ui, EditorSurface::Body, false);
    }

    pub(super) fn toolbar_for_surface(
        &mut self,
        ui: &mut egui::Ui,
        surface: EditorSurface,
        title_page_toolbar: bool,
    ) {
        let palette = self.theme_palette.toolbar;
        let vertical_item_spacing = ui.spacing().item_spacing.y;
        let visuals = ui.visuals_mut();
        visuals.widgets.noninteractive.fg_stroke.color = palette.foreground;
        visuals.widgets.hovered.bg_fill = palette.hover_fill;
        visuals.widgets.hovered.weak_bg_fill = palette.hover_fill;
        visuals.widgets.hovered.fg_stroke.color = palette.hover_foreground;
        visuals.widgets.active.bg_fill = palette.pressed_fill;
        visuals.widgets.open.bg_fill = palette.open_fill;

        // Use the clipped panel content edge for both allocation and rendering.
        // The logical panel width can extend past the visible viewport after
        // frame/panel and native viewport insets have been applied.
        let content_rect = ui.available_rect_before_wrap();
        let row_left = ui.cursor().left();
        let available_width =
            toolbar_actual_available_width(content_rect, ui.clip_rect(), row_left);
        let availability = self.app_command_availability();
        let toolbar_items = if title_page_toolbar {
            title_page_toolbar_items_for(availability).to_vec()
        } else {
            toolbar_items_for(availability).to_vec()
        };
        let allocation = allocate_toolbar(
            &toolbar_items,
            available_width,
            |label| toolbar_label_width(ui, label),
            toolbar_graphic_width,
        );
        let presentation = live_toolbar_metrics(ui);
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            // Top padding, the Item group, and bottom padding define the panel's
            // complete vertical extent. Do not let the outer top-down layout add
            // implicit gaps between those explicit allocations.
            ui.spacing_mut().item_spacing.y = 0.0;
            for text_style in [TextStyle::Body, TextStyle::Button] {
                if let Some(font) = ui.style_mut().text_styles.get_mut(&text_style) {
                    font.size = toolbar_presentation::DEFAULT_ITEM_TEXT_POINT_SIZE;
                }
            }
            ui.add_space(TOOLBAR_TOP_PADDING);
            ui.allocate_ui_with_layout(
                Vec2::new(
                    allocation.row_width,
                    toolbar_item_group_height(ui.style()),
                ),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    // Descendant controls and popup rows retain the normal style
                    // spacing; only the enclosing panel composition suppresses it.
                    ui.spacing_mut().item_spacing.y = vertical_item_spacing;
                    ui.add_space(TOOLBAR_ROW_PADDING);
                    let mut first = true;
                    for (item, width) in allocation.visible.iter().copied() {
                        if !first {
                            ui.add_space(TOOLBAR_ITEM_GAP);
                        }
                        first = false;
                        ui.push_id(item.id, |ui| match item.behavior {
                            ToolbarBehavior::Action(_) => {
                                self.toolbar_action_button(ui, item, width, presentation)
                            }
                            ToolbarBehavior::ElementsRoleDropdown => {
                                self.elements_toolbar_item(ui, item, width, presentation)
                            }
                            ToolbarBehavior::StylesInlineGroup
                            | ToolbarBehavior::AlignmentInlineGroup => {
                                self.formatting_inline_group_button(
                                    ui,
                                    item,
                                    width,
                                    presentation,
                                    surface,
                                )
                            }
                            ToolbarBehavior::None | ToolbarBehavior::OverflowMenu => {
                                if matches!(
                                    item.layout,
                                    ToolbarLayoutPolicy::FixedSpace(_)
                                        | ToolbarLayoutPolicy::FlexibleSpace
                                ) {
                                    ui.add_space(width);
                                } else {
                                    self.toolbar_action_button(ui, item, width, presentation);
                                }
                            }
                        });
                    }
                    if !allocation.overflow.is_empty() {
                        if !first {
                            ui.add_space(TOOLBAR_ITEM_GAP);
                        }
                        ui.allocate_ui_with_layout(
                            Vec2::new(allocation.overflow_width, ui.spacing().interact_size.y),
                            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                            |ui| {
                                self.toolbar_overflow_menu(
                                    ui,
                                    OVERFLOW_ITEM_SPEC,
                                    &allocation.overflow,
                                    surface,
                                )
                            },
                        );
                    }
                    ui.add_space(TOOLBAR_ROW_PADDING);
                },
            );
            ui.add_space(TOOLBAR_BOTTOM_PADDING);
        });
        if palette.border != Color32::TRANSPARENT {
            let toolbar_rect = ui.max_rect().intersect(ui.clip_rect());
            let y = toolbar_rect.bottom() - 0.5;
            ui.painter().line_segment(
                [
                    egui::pos2(toolbar_rect.left(), y),
                    egui::pos2(toolbar_rect.right(), y),
                ],
                egui::Stroke::new(TOOLBAR_BOTTOM_BORDER_WIDTH, palette.border),
            );
        }
    }
}

#[cfg(test)]
mod toolbar_contract_tests {
    use super::*;
    use crate::app_commands::AppCommandAvailability;
    use scriptcraft_commands::CommandId;

    fn availability(modal_blocked: bool) -> AppCommandAvailability {
        AppCommandAvailability {
            modal_blocked,
            native_output_active: false,
            undo: true,
            redo: true,
            delete: true,
            select_all: true,
            cut: true,
            copy: true,
            paste: true,
            body_only: true,
            dual_dialogue_toggle: true,
            title_open: !modal_blocked,
            title_close: !modal_blocked,
        }
    }

    #[test]
    fn title_page_availability_tracks_migration_modal_gate() {
        let enabled = toolbar_items_for(availability(false));
        let title = enabled.iter().find(|item| item.id == "title-page");
        assert_eq!(title.map(|item| item.enablement), Some(EnablementPolicy::Enabled));

        for blocked in [availability(true)] {
            let items = toolbar_items_for(blocked);
            let title = items.iter().find(|item| item.id == "title-page");
            assert_eq!(title.map(|item| item.enablement), Some(EnablementPolicy::Disabled));
        }

        let title_command = CommandId::WindowTitlePageOpen;
        assert!(!availability(true).is_enabled(title_command));
    }

    #[test]
    fn live_toolbar_projection_preserves_declared_non_dynamic_enablement() {
        let projected = toolbar_items_for(availability(false));

        for id in ["show-hide", "export-text"] {
            let item = projected.iter().find(|item| item.id == id);
            assert_eq!(item.map(|item| item.behavior), Some(ToolbarBehavior::None));
            assert_eq!(item.map(|item| item.enablement), Some(EnablementPolicy::Disabled));
        }

        for id in [
            "flexible-space-leading",
            "flexible-space-pre-elements",
            "flexible-space-post-elements",
            "flexible-space-trailing",
            "elements",
            "overflow",
        ] {
            let projected_item = projected.iter().find(|item| item.id == id);
            let declared_item = TEMPORARY_TOOLBAR_ITEMS
                .iter()
                .find(|item| item.id == id);
            assert_eq!(projected_item.map(|item| item.enablement), declared_item.map(|item| item.enablement), "{id} retains its declared enablement");
            assert_eq!(projected_item.map(|item| item.enablement), Some(EnablementPolicy::Enabled));
        }

        for id in ["new", "open", "save", "save-as", "title-page", "export-pdf"] {
            let item = projected.iter().find(|item| item.id == id);
            assert_eq!(item.map(|item| item.enablement), Some(EnablementPolicy::Enabled), "{id} remains enabled under ordinary availability");
        }
    }

    #[test]
    fn toolbar_allocation_retains_flexible_space_and_overflow_ownership() {
        let items = toolbar_items_for(availability(false));
        let flexible_count = items.iter().filter(|item| is_flexible_toolbar_space(**item)).count();
        assert_eq!(flexible_count, 4);
        assert_eq!(items.last().map(|item| item.behavior), Some(ToolbarBehavior::OverflowMenu));
        assert_eq!(OVERFLOW_ITEM_SPEC.layout, ToolbarLayoutPolicy::FixedWidth(32.0));

        let measure_label = |label: &str| label.chars().count() as f32 * 7.0;
        let measure_graphic = toolbar_graphic_width;
        let ordinary = allocate_toolbar(&items, 1200.0, measure_label, measure_graphic);
        assert!(ordinary.overflow.is_empty());
        let visible_flexible: Vec<_> = ordinary
            .visible
            .iter()
            .filter(|(item, _)| is_flexible_toolbar_space(*item))
            .map(|(_, width)| *width)
            .collect();
        assert_eq!(visible_flexible.len(), flexible_count);
        assert!(visible_flexible.iter().all(|width| (*width - visible_flexible[0]).abs() < f32::EPSILON));

        let narrow = allocate_toolbar(&items, 280.0, measure_label, measure_graphic);
        assert!(!narrow.overflow.is_empty());
        assert!(narrow.overflow_width <= 32.0);
        assert!(narrow.overflow.iter().any(|item| item.id == "title-page"));
    }

    #[test]
    fn formatting_groups_cross_their_measured_allocation_boundary_atomically() {
        let items = toolbar_items_for(availability(false));
        let measure_label = |label: &str| label.chars().count() as f32 * 7.0;
        let measure_graphic = toolbar_graphic_width;
        for item in items
            .iter()
            .copied()
            .filter(|item| matches!(item.behavior, ToolbarBehavior::StylesInlineGroup | ToolbarBehavior::AlignmentInlineGroup))
        {
            let item_width = toolbar_item_width(item, measure_label, measure_graphic);
            let boundary = item_width + TOOLBAR_ROW_PADDING * 2.0;
            let catalog = [item, OVERFLOW_ITEM_SPEC];
            let above = allocate_toolbar(&catalog, boundary + 0.01, measure_label, measure_graphic);
            let below = allocate_toolbar(&catalog, boundary - 0.01, measure_label, measure_graphic);
            assert!(above.visible.iter().any(|(visible, _)| visible.id == item.id));
            assert!(!above.overflow.iter().any(|overflow| overflow.id == item.id));
            assert!(!below.visible.iter().any(|(visible, _)| visible.id == item.id));
            assert_eq!(below.overflow.iter().filter(|overflow| overflow.id == item.id).count(), 1);
        }
    }

    #[test]
    fn formatting_controls_disable_for_migration_modal_and_retain_child_order() {
        let active = toolbar_items_for(availability(false));
        let styles = active.iter().find(|item| item.id == "styles");
        let alignment = active.iter().find(|item| item.id == "alignment");
        assert_eq!(styles.map(|item| item.enablement), Some(EnablementPolicy::Enabled));
        assert_eq!(alignment.map(|item| item.enablement), Some(EnablementPolicy::Enabled));
        for blocked in [availability(true)] {
            let main = toolbar_items_for(blocked);
            assert_eq!(
                main.iter().find(|item| item.id == "styles").map(|item| item.enablement),
                Some(EnablementPolicy::Disabled),
            );
            assert_eq!(
                main.iter().find(|item| item.id == "alignment").map(|item| item.enablement),
                Some(EnablementPolicy::Disabled),
            );
            let child = title_page_toolbar_items_for(blocked);
            assert_eq!(child[0].enablement, EnablementPolicy::Disabled);
            assert_eq!(child[1].enablement, EnablementPolicy::Disabled);
        }
        let child_ids = title_page_toolbar_items_for(availability(false))
            .map(|item: ToolbarItemSpec| item.id);
        assert_eq!(
            child_ids,
            ["title-page-flexible-space", "styles", "alignment", "title-page-flexible-space", "close-title-page", "overflow"],
        );
    }

    #[test]
    fn style_lit_projection_uses_any_selected_run_and_collapsed_typing_state_per_surface() {
        let app_result = crate::ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        app.session.insert("ab").unwrap();
        app.session
            .set_selection(scriptcraft_core::editing::Selection {
                anchor: Position { element: 0, offset: 0 },
                focus: Position { element: 0, offset: 1 },
            })
            .unwrap();
        app.session.toggle_body_run_style(scriptcraft_core::RunStyle::Bold).unwrap();
        app.session
            .set_selection(scriptcraft_core::editing::Selection {
                anchor: Position { element: 0, offset: 0 },
                focus: Position { element: 0, offset: 2 },
            })
            .unwrap();
        let (body_styles, _) = app.selected_editor_state(EditorSurface::Body);
        assert!(body_styles.bold);

        app.session
            .set_selection(scriptcraft_core::editing::Selection::caret(Position {
                element: 0,
                offset: 0,
            }))
            .unwrap();
        let (collapsed_body_styles, _) = app.selected_editor_state(EditorSurface::Body);
        assert!(collapsed_body_styles.bold);

        app.session
            .set_title_selection(scriptcraft_core::editing::Selection::caret(Position {
                element: 17,
                offset: 0,
            }))
            .unwrap();
        let (title_styles, _) = app.selected_editor_state(EditorSurface::TitlePage);
        assert!(title_styles.underline);
        assert!(title_styles.all_caps);
    }

    #[test]
    fn cross_paragraph_selection_unions_any_style_but_mixed_alignment_is_unlit() {
        let project_result = scriptcraft_core::Project::new();
        assert!(project_result.is_ok());
        let Ok(mut project) = project_result else { return };
        project.elements[0].text = "ab".to_owned();
        project.elements.push(
            scriptcraft_core::ScreenplayElement::new(
                "formatting-toolbar-second-paragraph",
                Role::Action,
                "cd",
            )
            .unwrap(),
        );
        let session_result = scriptcraft_core::editing::EditingSession::new(project);
        assert!(session_result.is_ok());
        let Ok(session) = session_result else { return };
        let app_result = crate::ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        app.session = session;
        app.session
            .set_selection(scriptcraft_core::editing::Selection {
                anchor: Position { element: 0, offset: 1 },
                focus: Position { element: 0, offset: 2 },
            })
            .unwrap();
        app.session.toggle_body_run_style(scriptcraft_core::RunStyle::Bold).unwrap();
        app.session
            .set_selection(scriptcraft_core::editing::Selection::caret(Position {
                element: 0,
                offset: 0,
            }))
            .unwrap();
        app.session.set_body_alignment(ParagraphAlignment::Right).unwrap();
        app.session
            .set_selection(scriptcraft_core::editing::Selection {
                anchor: Position { element: 0, offset: 1 },
                focus: Position { element: 1, offset: 1 },
            })
            .unwrap();

        let (styles, alignment) = app.selected_editor_state(EditorSurface::Body);
        assert!(styles.bold);
        assert_eq!(alignment, None);
    }

    #[test]
    fn alignment_lit_projection_is_common_only_and_justified_is_a_command_value() {
        let app_result = crate::ScriptCraftApp::new();
        assert!(app_result.is_ok());
        let Ok(mut app) = app_result else { return };
        app.session
            .set_title_selection(scriptcraft_core::editing::Selection::caret(Position {
                element: 17,
                offset: 0,
            }))
            .unwrap();
        let (_, title_alignment) = app.selected_editor_state(EditorSurface::TitlePage);
        assert_eq!(title_alignment, Some(ParagraphAlignment::Center));

        app.dispatch_formatting_choice(
            FormattingChoice::Alignment(ParagraphAlignmentValue::Justified),
            EditorSurface::TitlePage,
            &egui::Context::default(),
        );
        let (_, justified) = app.selected_editor_state(EditorSurface::TitlePage);
        assert_eq!(justified, Some(ParagraphAlignment::Justified));
        assert!(formatting_choice_active(
            FormattingChoice::Alignment(ParagraphAlignmentValue::Justified),
            RunStyleFlags::default(),
            justified,
        ));
    }


}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ToolbarPresentationMetrics {
    pub(super) icon_control_size: f32,
    pub(super) font_size: f32,
}

impl ToolbarPresentationMetrics {
    pub(super) fn font_id(self) -> FontId {
        FontId::proportional(self.font_size)
    }
}

pub(super) fn live_toolbar_metrics(ui: &egui::Ui) -> ToolbarPresentationMetrics {
    ToolbarPresentationMetrics {
        icon_control_size: ui.spacing().interact_size.y,
        font_size: toolbar_presentation::DEFAULT_ITEM_TEXT_POINT_SIZE,
    }
}

pub(super) fn toolbar_icon_control_rect(
    group_rect: egui::Rect,
    icon_control_size: Vec2,
) -> egui::Rect {
    egui::Rect::from_center_size(
        egui::pos2(
            group_rect.center().x,
            group_rect.min.y + icon_control_size.y / 2.0,
        ),
        icon_control_size,
    )
}

pub(super) fn toolbar_caption_rect(
    group_rect: egui::Rect,
    nominal_control_height: f32,
    control_caption_gap: f32,
) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(
            group_rect.left(),
            group_rect.top() + nominal_control_height + control_caption_gap,
        ),
        group_rect.max,
    )
}

pub(super) fn toolbar_icon_hover_rect(
    icon_control_rect: egui::Rect,
    padding: ToolbarIconHoverPadding,
) -> egui::Rect {
    egui::Rect::from_min_max(
        icon_control_rect.min - Vec2::new(padding.left, padding.top),
        icon_control_rect.max + Vec2::new(padding.right, padding.bottom),
    )
}

#[derive(Debug, Clone, PartialEq)]
struct FormattingGroupGeometry {
    interaction_rects: Vec<egui::Rect>,
    separator_rects: Vec<egui::Rect>,
    control_row_rect: egui::Rect,
    caption_rect: egui::Rect,
    icon_centers: Vec<egui::Pos2>,
}

/// Shared measured layout for an inline formatting group. Each hit target is
/// the 19-point glyph slot expanded by the toolbar's existing per-edge padding;
/// separator slots reserve the exact space WO-005 will later paint into.
fn formatting_group_geometry(
    group_rect: egui::Rect,
    choices: &[FormattingChoiceSpec],
    control_height: f32,
    caption_gap: f32,
) -> FormattingGroupGeometry {
    let row_width = choices
        .iter()
        .map(|choice| {
            toolbar_presentation::icon_point_size(choice.icon)
                + TOOLBAR_ICON_HOVER_PADDING.left
                + TOOLBAR_ICON_HOVER_PADDING.right
        })
        .sum::<f32>()
        + choices.len().saturating_sub(1) as f32 * TOOLBAR_FORMATTING_SEPARATOR_SLOT_WIDTH;
    let row_left = group_rect.center().x - row_width / 2.0;
    let mut x = row_left;
    let mut interaction_rects = Vec::with_capacity(choices.len());
    let mut separator_rects = Vec::with_capacity(choices.len().saturating_sub(1));
    let mut icon_centers = Vec::with_capacity(choices.len());
    let mut row_top = group_rect.top();
    let mut row_bottom = group_rect.top() + control_height;

    for (index, choice) in choices.iter().enumerate() {
        let glyph_size = toolbar_presentation::icon_point_size(choice.icon);
        let glyph_top = group_rect.top() + (control_height - glyph_size) * 0.5;
        let glyph_rect = egui::Rect::from_min_size(
            egui::pos2(x + TOOLBAR_ICON_HOVER_PADDING.left, glyph_top),
            Vec2::splat(glyph_size),
        );
        let interaction_rect = toolbar_icon_hover_rect(glyph_rect, TOOLBAR_ICON_HOVER_PADDING);
        row_top = row_top.min(interaction_rect.top());
        row_bottom = row_bottom.max(interaction_rect.bottom());
        interaction_rects.push(interaction_rect);
        icon_centers.push(glyph_rect.center());
        x = interaction_rect.right();
        if index + 1 < choices.len() {
            let separator_rect = egui::Rect::from_min_size(
                egui::pos2(x, group_rect.top()),
                Vec2::new(TOOLBAR_FORMATTING_SEPARATOR_SLOT_WIDTH, control_height),
            );
            separator_rects.push(separator_rect);
            x = separator_rect.right();
        }
    }

    let control_row_rect = egui::Rect::from_min_max(
        egui::pos2(row_left, row_top),
        egui::pos2(row_left + row_width, row_bottom),
    );
    let caption_top = control_row_rect.bottom() + caption_gap;
    FormattingGroupGeometry {
        interaction_rects,
        separator_rects,
        control_row_rect,
        caption_rect: egui::Rect::from_min_max(
            egui::pos2(group_rect.left(), caption_top),
            group_rect.max,
        ),
        icon_centers,
    }
}

fn formatting_choice_fill(
    enabled: bool,
    selected: bool,
    hovered: bool,
    pressed: bool,
    palette: crate::theme::ToolbarPalette,
) -> Color32 {
    if !enabled {
        Color32::TRANSPARENT
    } else if pressed {
        palette.pressed_fill
    } else if selected {
        palette.selected_fill
    } else if hovered {
        palette.hover_fill
    } else {
        Color32::TRANSPARENT
    }
}

fn formatting_separator_stroke_width(pixels_per_point: f32) -> f32 {
    let scale = if pixels_per_point.is_finite() && pixels_per_point > 0.0 {
        pixels_per_point
    } else {
        1.0
    };
    let width = 1.0 / scale;
    if width.is_finite() && width > 0.0 {
        width
    } else {
        1.0
    }
}

fn formatting_separator_line(slot: egui::Rect) -> [egui::Pos2; 2] {
    let center = slot.center();
    [
        egui::pos2(center.x, center.y - TOOLBAR_FORMATTING_SEPARATOR_LENGTH * 0.5),
        egui::pos2(center.x, center.y + TOOLBAR_FORMATTING_SEPARATOR_LENGTH * 0.5),
    ]
}

fn formatting_separator_clip(control_row: egui::Rect, ui_clip: egui::Rect) -> egui::Rect {
    control_row.intersect(ui_clip)
}

fn formatting_separator_stroke(
    palette: crate::theme::ToolbarPalette,
    pixels_per_point: f32,
) -> egui::Stroke {
    egui::Stroke::new(
        formatting_separator_stroke_width(pixels_per_point),
        palette.formatting_separator,
    )
}

fn paint_formatting_group_separators(
    ui: &egui::Ui,
    geometry: &FormattingGroupGeometry,
    palette: crate::theme::ToolbarPalette,
) {
    let painter = ui.painter().with_clip_rect(formatting_separator_clip(
        geometry.control_row_rect,
        ui.clip_rect(),
    ));
    let stroke = formatting_separator_stroke(palette, ui.ctx().pixels_per_point());
    for slot in &geometry.separator_rects {
        painter.line_segment(formatting_separator_line(*slot), stroke);
    }
}

pub(super) fn toolbar_cursor_icon() -> egui::CursorIcon {
    egui::CursorIcon::Default
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormattingChoice {
    Style(TitleTextStyle),
    Alignment(ParagraphAlignmentValue),
}

fn formatting_choice_command(choice: FormattingChoice, surface: EditorSurface) -> Command {
    match choice {
        FormattingChoice::Style(style) => Command::EditorStyleToggle { surface, style },
        FormattingChoice::Alignment(alignment) => {
            Command::EditorAlignmentSet { surface, alignment }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct FormattingChoiceSpec {
    choice: FormattingChoice,
    label: &'static str,
    icon: IconSpec,
}

const STYLE_CHOICES: [FormattingChoiceSpec; 5] = [
    FormattingChoiceSpec {
        choice: FormattingChoice::Style(TitleTextStyle::Bold),
        label: "Bold",
        icon: IconSpec::new(ICON_FORMAT_BOLD),
    },
    FormattingChoiceSpec {
        choice: FormattingChoice::Style(TitleTextStyle::Italics),
        label: "Italics",
        icon: IconSpec::new(ICON_FORMAT_ITALIC),
    },
    FormattingChoiceSpec {
        choice: FormattingChoice::Style(TitleTextStyle::Underline),
        label: "Underline",
        icon: IconSpec::new(ICON_FORMAT_UNDERLINED),
    },
    FormattingChoiceSpec {
        choice: FormattingChoice::Style(TitleTextStyle::AllCaps),
        label: "AllCaps",
        icon: IconSpec::new(ICON_TEXT_FIELDS),
    },
    FormattingChoiceSpec {
        choice: FormattingChoice::Style(TitleTextStyle::Strikethrough),
        label: "Strikethrough",
        icon: IconSpec::new(ICON_FORMAT_STRIKETHROUGH),
    },
];

const ALIGNMENT_CHOICES: [FormattingChoiceSpec; 4] = [
    FormattingChoiceSpec {
        choice: FormattingChoice::Alignment(ParagraphAlignmentValue::Left),
        label: "Left",
        icon: IconSpec::new(ICON_FORMAT_ALIGN_LEFT),
    },
    FormattingChoiceSpec {
        choice: FormattingChoice::Alignment(ParagraphAlignmentValue::Center),
        label: "Center",
        icon: IconSpec::new(ICON_FORMAT_ALIGN_CENTER),
    },
    FormattingChoiceSpec {
        choice: FormattingChoice::Alignment(ParagraphAlignmentValue::Right),
        label: "Right",
        icon: IconSpec::new(ICON_FORMAT_ALIGN_RIGHT),
    },
    FormattingChoiceSpec {
        choice: FormattingChoice::Alignment(ParagraphAlignmentValue::Justified),
        label: "Justified",
        icon: IconSpec::new(ICON_FORMAT_ALIGN_JUSTIFY),
    },
];

fn ordered_editor_selection(selection: scriptcraft_core::editing::Selection) -> (Position, Position) {
    if (selection.anchor.element, selection.anchor.offset)
        <= (selection.focus.element, selection.focus.offset)
    {
        (selection.anchor, selection.focus)
    } else {
        (selection.focus, selection.anchor)
    }
}

fn style_union(left: RunStyleFlags, right: RunStyleFlags) -> RunStyleFlags {
    RunStyleFlags {
        bold: left.bold || right.bold,
        italics: left.italics || right.italics,
        underline: left.underline || right.underline,
        all_caps: left.all_caps || right.all_caps,
        strikethrough: left.strikethrough || right.strikethrough,
    }
}

fn formatting_choice_active(choice: FormattingChoice, styles: RunStyleFlags, alignment: Option<ParagraphAlignment>) -> bool {
    match choice {
        FormattingChoice::Style(style) => match style {
            TitleTextStyle::Bold => styles.bold,
            TitleTextStyle::Italics => styles.italics,
            TitleTextStyle::Underline => styles.underline,
            TitleTextStyle::AllCaps => styles.all_caps,
            TitleTextStyle::Strikethrough => styles.strikethrough,
        },
        FormattingChoice::Alignment(value) => {
            alignment == Some(value.as_core())
        }
    }
}

pub(super) fn toolbar_group_fill(style: &egui::Style, hovered: bool) -> Color32 {
    if hovered {
        style.visuals.widgets.hovered.bg_fill
    } else {
        Color32::TRANSPARENT
    }
}

fn toolbar_action_enabled(item: ToolbarItemSpec) -> bool {
    item.enablement == EnablementPolicy::Enabled
}

fn toolbar_action_sense(enabled: bool) -> Sense {
    if enabled {
        Sense::click()
    } else {
        // Disabled controls retain their explicit accessibility metadata but have
        // no pointer, hover, focus, or keyboard interaction sensing.
        Sense::empty()
    }
}

fn toolbar_action_fill(style: &egui::Style, enabled: bool, hovered: bool) -> Color32 {
    toolbar_group_fill(style, enabled && hovered)
}

fn toolbar_action_activation(
    item: ToolbarItemSpec,
    activated: bool,
) -> Option<scriptcraft_commands::CommandId> {
    if !activated || !toolbar_action_enabled(item) {
        return None;
    }
    match item.behavior {
        ToolbarBehavior::Action(action) => Some(action),
        _ => None,
    }
}

pub(super) fn toolbar_item_group_height(style: &egui::Style) -> f32 {
    style.spacing.interact_size.y * 2.0
}

pub(super) fn toolbar_content_height(style: &egui::Style) -> f32 {
    TOOLBAR_TOP_PADDING + toolbar_item_group_height(style) + TOOLBAR_BOTTOM_PADDING
}

use super::*;
use super::toolbar_items::{
    EnablementPolicy, IconSpec, OVERFLOW_ITEM_SPEC, ToolbarAction, ToolbarBehavior, ToolbarGraphic,
    ToolbarIconVariant, ToolbarItemSpec, ToolbarLayoutPolicy, TEMPORARY_TOOLBAR_ITEMS,
};

// Manual toolbar geometry controls: tune Item group width and row spacing here.
// The 46-point normal slot is the compact baseline recorded by the focused
// geometry tests; labels may still expand a slot when active font metrics need it.
pub(super) const TOOLBAR_NORMAL_ITEM_WIDTH: f32 = 46.0;
pub(super) const TOOLBAR_GROUP_HORIZONTAL_PADDING: f32 = 8.0;
pub(super) const TOOLBAR_ROW_PADDING: f32 = 4.0;
pub(super) const TOOLBAR_ITEM_GAP: f32 = 4.0;
pub(super) const TOOLBAR_TOP_PADDING: f32 = 6.0;
// Preserve the toolbar's text/control clearance. The trailing span is part of
// the exact top-panel allocation and must not be removed to fix a viewport row.
pub(super) const TOOLBAR_BOTTOM_PADDING: f32 = 6.0;

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
    /// Optional disabled foreground; `None` uses the resolved enabled foreground at reduced alpha.
    pub(super) const DISABLED_FOREGROUND_OVERRIDE: Option<Color32> = None;
    /// Alpha multiplier used only when no explicit disabled-color override resolves.
    pub(super) const DISABLED_OPACITY_MULTIPLIER: f32 = 0.45;
    /// Agent-default Material glyph size, initially matching the existing Body glyph size.
    pub(super) const DEFAULT_MATERIAL_ICON_POINT_SIZE: f32 = 18.0;
    /// Default Material glyph family; individual icons may override this variant.
    pub(super) const DEFAULT_MATERIAL_ICON_VARIANT: ToolbarIconVariant =
        ToolbarIconVariant::Outlined;
    /// Default toolbar caption and menu text size, in logical points.
    pub(super) const DEFAULT_ITEM_TEXT_POINT_SIZE: f32 = 11.0;
    /// Elements role-option text size, independent of toolbar item text.
    pub(super) const DEFAULT_ELEMENTS_ROLE_OPTION_POINT_SIZE: f32 = 14.0;
    /// Horizontal padding on each side of an Elements role-option row, in points.
    pub(super) const DEFAULT_ELEMENTS_ROLE_OPTION_HORIZONTAL_PADDING: f32 = 18.0;
    /// Gap between icons in grouped or primary/disclosure graphics, in logical points.
    pub(super) const DEFAULT_GRAPHIC_GAP: f32 = 4.0;

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

    pub(super) fn disabled_foreground(
        ui: &egui::Ui,
        item: ToolbarItemSpec,
        icon: Option<IconSpec>,
        palette: crate::theme::ToolbarPalette,
    ) -> Color32 {
        icon.and_then(|spec| spec.disabled_color)
            .or(item.presentation.disabled_color)
            .or(DISABLED_FOREGROUND_OVERRIDE)
            .unwrap_or_else(|| {
                let enabled = enabled_foreground(ui, item, icon);
                if enabled == palette.foreground {
                    return palette.disabled_foreground;
                }
                Color32::from_rgba_unmultiplied(
                    enabled.r(),
                    enabled.g(),
                    enabled.b(),
                    (f32::from(enabled.a()) * DISABLED_OPACITY_MULTIPLIER)
                        .round()
                        .clamp(0.0, 255.0) as u8,
                )
            })
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

pub(super) fn default_toolbar_items() -> [ToolbarItemSpec; 13] {
    TEMPORARY_TOOLBAR_ITEMS
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

fn paint_toolbar_icon(
    ui: &egui::Ui,
    center: egui::Pos2,
    icon: IconSpec,
    item: ToolbarItemSpec,
    palette: crate::theme::ToolbarPalette,
) {
    let point_size = toolbar_presentation::icon_point_size(icon);
    let color = if matches!(item.enablement, EnablementPolicy::Enabled) {
        toolbar_presentation::enabled_foreground(ui, item, Some(icon))
    } else {
        toolbar_presentation::disabled_foreground(ui, item, Some(icon), palette)
    };
    let galley = WidgetText::from(toolbar_presentation::material_icon_text(icon).size(point_size))
    .into_galley(
        ui,
        None,
        point_size,
        TextStyle::Button,
    );
    ui.painter()
        .galley(center - galley.size() / 2.0, galley, color);
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
    pub(super) fn dispatch_toolbar_action(&mut self, action: ToolbarAction) {
        match action {
            ToolbarAction::New => {
                match Project::new()
                    .map_err(|error| format!("new project failed: {error:?}"))
                    .and_then(|project| {
                        EditingSession::with_policy(project, self.session.policy())
                            .map_err(|error| format!("session failed: {error:?}"))
                    }) {
                    Ok(session) => {
                        self.session = session;
                        self.page_geometry.reset_for_document_replacement();
                        self.layout_cache = LayoutCache::default();
                        self.surface_focused = false;
                        self.native_path = None;
                        self.fdx_baseline = None;
                        self.pending_fdx_open = None;
                        self.next_cast_list_refresh = Instant::now()
                            + scriptcraft_core::cast_list::CAST_LIST_REFRESH_INTERVAL;
                        self.rendered_revision = None;
                        self.status = "New in-memory project".into();
                        self.clear_error();
                    }
                    Err(error) => self.show_error(error),
                }
            }
            ToolbarAction::Open => self.open_native(),
            ToolbarAction::Save => self.save_native(),
            ToolbarAction::SaveAs => self.save_as_native(),
            ToolbarAction::ExportPdf => self.export_pdf(),
        }
    }

    pub(super) fn toolbar_action_button(
        &mut self,
        ui: &mut egui::Ui,
        item: ToolbarItemSpec,
        width: f32,
        presentation: ToolbarPresentationMetrics,
    ) {
        let palette = self.theme_palette.toolbar;
        let (id, rect) = ui.allocate_space(Vec2::new(width, ui.spacing().interact_size.y * 2.0));
        let icon_control_size = Vec2::splat(presentation.icon_control_size);
        let icon_control_rect = toolbar_icon_control_rect(rect, icon_control_size);
        let icon_rect = toolbar_icon_hover_rect(icon_control_rect, TOOLBAR_ICON_HOVER_PADDING);
        let enabled = item.enablement == EnablementPolicy::Enabled;
        let response = ui
            .interact(
                icon_rect,
                id.with("icon"),
                if enabled {
                    Sense::click()
                } else {
                    // Hover-only keeps disabled accessibility state explicit but
                    // cannot focus, activate, or receive the enabled hover paint.
                    Sense::hover()
                },
            )
            .on_hover_cursor(egui::CursorIcon::Default);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                enabled,
                item.accessibility_name.to_owned(),
            )
        });
        let fill = toolbar_group_fill(ui.style(), enabled && response.hovered());
        if fill != Color32::TRANSPARENT {
            ui.painter().rect_filled(icon_rect, 2.0, fill);
        }
        let body_font = presentation.font_id();
        paint_toolbar_graphic(ui, icon_control_rect.center(), item.graphic, item, palette);
        if item.caption_visible {
            let label_color = if enabled {
                toolbar_presentation::enabled_foreground(ui, item, None)
            } else {
                toolbar_presentation::disabled_foreground(ui, item, None, palette)
            };
            let label_galley = ui.painter().layout_no_wrap(
                item.label.to_owned(),
                body_font,
                label_color,
            );
            let label_center = egui::pos2(
                rect.center().x,
                icon_control_rect.max.y
                    + ui.spacing().item_spacing.y
                    + label_galley.size().y / 2.0,
            );
            ui.painter().galley(
                label_center - label_galley.size() / 2.0,
                label_galley,
                label_color,
            );
        }
        if response.clicked() {
            if let ToolbarBehavior::Action(action) = item.behavior {
                self.dispatch_toolbar_action(action);
            }
        }
    }

    pub(super) fn focused_role(&self) -> Option<Role> {
        self.session
            .project()
            .elements
            .get(self.session.selection().focus.element)
            .map(|element| element.role)
    }

    pub(super) fn change_role_from_toolbar(&mut self, role: Role) {
        let result = self.session.change_role(role);
        self.mutate(result, "Changed focused element role", RevealIntent::None);
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
        let group_response = ui.allocate_ui_with_layout(
            Vec2::new(width, ui.spacing().interact_size.y * 2.0),
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                // ComboBox has an intrinsic minimum. Constrain this child to the
                // same explicit budget used by allocate_toolbar so it cannot
                // advance the parent row past the allocated Elements slot.
                ui.set_width(width);
                let role_popup_height = elements_role_popup_max_height(ui);
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
                }
                // This caption is decorative. Allocate/paint it instead of adding a
                // label response that can compete with the ComboBox cursor owner.
                let (_, label_rect) = ui.allocate_space(ui.spacing().interact_size);
                let label_font = presentation.font_id();
                ui.painter().text(
                    label_rect.left_center(),
                    egui::Align2::LEFT_CENTER,
                    item.label,
                    label_font,
                    toolbar_presentation::enabled_foreground(ui, item, None),
                );
                debug_assert!(combo_response.rect.width() <= width + f32::EPSILON);
            },
        );
        // Own the label-only portion at the group boundary without making the
        // decorative label itself interactive or changing ComboBox routing.
        group_response
            .response
            .on_hover_cursor(toolbar_cursor_icon());
        if selected_role != Some(next_role) {
            self.change_role_from_toolbar(next_role);
        }
    }

    pub(super) fn toolbar_overflow_menu(
        &mut self,
        ui: &mut egui::Ui,
        indicator: ToolbarItemSpec,
        items: &[ToolbarItemSpec],
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
                        if ui.button(item.label).clicked() {
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
                    ToolbarBehavior::None if item.enablement == EnablementPolicy::Disabled => {
                        ui.label(
                            RichText::new(item.label).color(
                                toolbar_presentation::disabled_foreground(
                                    ui,
                                    item,
                                    None,
                                    self.theme_palette.toolbar,
                                ),
                            ),
                        );
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
            self.dispatch_toolbar_action(action);
        }
        if let Some(role) = chosen_role {
            self.change_role_from_toolbar(role);
        }
    }

    pub(super) fn toolbar(&mut self, ui: &mut egui::Ui) {
        let palette = self.theme_palette.toolbar;
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
        let allocation = allocate_toolbar(
            &default_toolbar_items(),
            available_width,
            |label| toolbar_label_width(ui, label),
            toolbar_graphic_width,
        );
        let presentation = live_toolbar_metrics(ui);
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for text_style in [TextStyle::Body, TextStyle::Button] {
                if let Some(font) = ui.style_mut().text_styles.get_mut(&text_style) {
                    font.size = toolbar_presentation::DEFAULT_ITEM_TEXT_POINT_SIZE;
                }
            }
            ui.add_space(TOOLBAR_TOP_PADDING);
            ui.allocate_ui_with_layout(
                Vec2::new(allocation.row_width, ui.spacing().interact_size.y * 2.0),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
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
                egui::Stroke::new(1.0, palette.border),
            );
        }
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

pub(super) fn toolbar_icon_hover_rect(
    icon_control_rect: egui::Rect,
    padding: ToolbarIconHoverPadding,
) -> egui::Rect {
    egui::Rect::from_min_max(
        icon_control_rect.min - Vec2::new(padding.left, padding.top),
        icon_control_rect.max + Vec2::new(padding.right, padding.bottom),
    )
}

pub(super) fn toolbar_cursor_icon() -> egui::CursorIcon {
    egui::CursorIcon::Default
}

pub(super) fn toolbar_group_fill(style: &egui::Style, hovered: bool) -> Color32 {
    if hovered {
        style.visuals.widgets.hovered.bg_fill
    } else {
        Color32::TRANSPARENT
    }
}

pub(super) fn toolbar_content_height(style: &egui::Style) -> f32 {
    style.spacing.interact_size.y * 2.0 + TOOLBAR_BOTTOM_PADDING
}

use crate::focus_mode::FocusModeView;
use crate::theme::{AppearanceMode, StatusPalette};
use crate::toolbar::{
    toolbar_cursor_icon, toolbar_icon_hover_rect,
};
use eframe::egui::{
    self, Align, Color32, Direction, FontSelection, Pos2, Rect, Sense, Vec2, WidgetInfo,
    WidgetText, WidgetType,
};
use egui_material_icons::{
    icons::{ICON_LOCATION_SEARCHING, ICON_MODE_NIGHT, ICON_TIMER},
    MaterialIcon,
};
use scriptcraft_commands::{Appearance, Command, ZoomPercent};
use scriptcraft_core::editing::{EditingSession, ParagraphKeyTarget};
use scriptcraft_core::formatting::formatting_for_role;
use crate::shortcuts::{
    element_hint_bindings, element_hint_chord, modifiers_are_none, ElementHintChord,
    ShortcutPlatform,
};
pub(super) const MIN_ZOOM_PERCENT: u16 = 75;
pub(super) const MAX_ZOOM_PERCENT: u16 = 300;
pub(super) const ZOOM_PRESETS: [u16; 9] = [75, 90, 100, 125, 150, 175, 200, 250, 300];

/// Status-only presentation values. Keep these independent of toolbar and
/// framework defaults so later manual tuning cannot silently affect other UI.
mod presentation {
    use eframe::egui::{self, Color32};

    /// Height of the ordinary status content row, in logical points.
    pub(super) const STATUS_ROW_HEIGHT: f32 = 20.0;
    /// Local width for the status panel's explicitly painted top divider.
    pub(super) const STATUS_PANEL_TOP_BORDER_WIDTH: f32 = 1.0;
    /// Horizontal inset of the bottom status panel, in logical points.
    pub(super) const PANEL_HORIZONTAL_INSET: i8 = 12;
    /// Symmetric top/bottom inset of the bottom status panel, in logical points.
    pub(super) const PANEL_VERTICAL_INSET: i8 = 3;
    /// Padding around each status icon's hover rectangle, in logical points.
    pub(super) const ICON_HOVER_PADDING: f32 = 2.0;
    /// Status icon allocation width, preserving the current ZOOM_SLIDER_MIN_WIDTH slot.
    pub(super) const ICON_SLOT_WIDTH: f32 = 27.0;
    /// Horizontal gap between adjacent right-side status controls, in points.
    pub(super) const ITEM_GAP: f32 = 4.0;
    /// Minimum width of the status zoom slider, in logical points.
    pub(super) const ZOOM_SLIDER_MIN_WIDTH: f32 = 40.0;
    /// Maximum width of the status zoom slider, in logical points.
    pub(super) const ZOOM_SLIDER_MAX_WIDTH: f32 = 84.0;
    /// Percentage text reservation on each horizontal side, in points.
    pub(super) const PERCENTAGE_SIDE_RESERVATION: f32 = 8.0;
    /// Symmetric horizontal inset around the visible-page counter, in points.
    /// Zero preserves the current page-counter geometry.
    pub(super) const PAGE_COUNTER_HORIZONTAL_INSET: f32 = 24.0;
    /// Status-owned Body text size, in logical points.
    pub(super) const BODY_TEXT_SIZE: f32 = 10.0;
    /// Status-owned Button text size, in logical points.
    pub(super) const BUTTON_TEXT_SIZE: f32 = 10.0;
    /// Center helper/operation/error text size, independent of other status text.
    pub(super) const CENTER_TEXT_SIZE: f32 = 12.0;
    /// Compact font size for complete held-modifier lists, in logical points.
    pub(super) const MODIFIER_HINT_TEXT_SIZE: f32 = 8.5;
    /// Minimum modifier-list size before the clipped one-line fallback applies.
    pub(super) const MODIFIER_HINT_MIN_TEXT_SIZE: f32 = 6.0;
    /// Material glyph size used by the existing icon renderer, in logical points.
    pub(super) const ICON_GLYPH_SIZE: f32 = 17.0;
    /// Status zoom slider rail thickness, inherited as 8 logical points.
    /// Adjust this status-only value manually if a thinner rail is desired.
    pub(super) const ZOOM_SLIDER_RAIL_HEIGHT: f32 = 4.0;
    /// Optional app override for every status-owned default/idle text and icon.
    /// Leave `None` to follow the active Appearance theme's text foreground.
    pub(super) const DEFAULT_FOREGROUND_OVERRIDE: Option<Color32> = None;
    /// Optional center-message-only foreground override. Leave `None` to use
    /// the status default foreground without changing other status text colors.
    pub(super) const CENTER_FOREGROUND_OVERRIDE: Option<Color32> = None;
    /// Optional app override shared by every interactive status hover foreground.
    /// Leave `None` to follow the active Appearance theme's hovered-widget text.
    pub(super) const HOVERED_FOREGROUND_OVERRIDE: Option<Color32> = None;

    pub(super) fn default_foreground(ui: &egui::Ui) -> Color32 {
        DEFAULT_FOREGROUND_OVERRIDE.unwrap_or_else(|| ui.visuals().text_color())
    }

    pub(super) fn center_foreground(ui: &egui::Ui) -> Color32 {
        CENTER_FOREGROUND_OVERRIDE.unwrap_or_else(|| default_foreground(ui))
    }

    pub(super) fn hovered_foreground(ui: &egui::Ui) -> Color32 {
        HOVERED_FOREGROUND_OVERRIDE
            .unwrap_or_else(|| ui.visuals().widgets.hovered.text_color())
    }
}

pub(super) fn status_panel_frame(ctx: &egui::Context, palette: StatusPalette) -> egui::Frame {
    egui::Frame::side_top_panel(ctx.global_style().as_ref())
        .inner_margin(egui::Margin::symmetric(
            presentation::PANEL_HORIZONTAL_INSET,
            presentation::PANEL_VERTICAL_INSET,
        ))
        .fill(palette.background)
        .stroke(egui::Stroke::NONE)
}

pub(super) fn paint_panel_top_border(
    ui: &egui::Ui,
    panel_rect: Rect,
    palette: StatusPalette,
) {
    let width = presentation::STATUS_PANEL_TOP_BORDER_WIDTH;
    let y = panel_rect.top() + width * 0.5;
    ui.painter_at(panel_rect).line_segment(
        [
            egui::pos2(panel_rect.left(), y),
            egui::pos2(panel_rect.right(), y),
        ],
        egui::Stroke::new(width, palette.border),
    );
}

pub(super) const fn default_panel_height() -> f32 {
    presentation::STATUS_ROW_HEIGHT + (presentation::PANEL_VERTICAL_INSET as f32 * 2.0)
}

pub(super) fn apply_status_palette(ui: &mut egui::Ui, palette: StatusPalette) {
    let visuals = ui.visuals_mut();
    visuals.widgets.noninteractive.fg_stroke.color = palette.foreground;
    visuals.widgets.hovered.bg_fill = palette.hover_fill;
    visuals.widgets.hovered.weak_bg_fill = palette.hover_fill;
    visuals.widgets.hovered.fg_stroke.color = palette.hover_foreground;
}

fn status_font_id(ui: &egui::Ui, text_style: egui::TextStyle) -> egui::FontId {
    let mut font = text_style.resolve(ui.style());
    font.size = match text_style {
        egui::TextStyle::Body => presentation::BODY_TEXT_SIZE,
        egui::TextStyle::Button => presentation::BUTTON_TEXT_SIZE,
        _ => font.size,
    };
    font
}

fn with_status_text_style<R>(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.scope(|ui| {
        for (text_style, size) in [
            (egui::TextStyle::Body, presentation::BODY_TEXT_SIZE),
            (egui::TextStyle::Button, presentation::BUTTON_TEXT_SIZE),
        ] {
            if let Some(font) = ui.style_mut().text_styles.get_mut(&text_style) {
                font.size = size;
            }
        }
        contents(ui)
    })
    .inner
}

fn with_center_text_style<R>(
    ui: &mut egui::Ui,
    center_foreground: Color32,
    text_size: f32,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    ui.scope(|ui| {
        if let Some(font) = ui.style_mut().text_styles.get_mut(&egui::TextStyle::Body) {
            font.size = text_size;
        }
        ui.visuals_mut().override_text_color = Some(
            presentation::CENTER_FOREGROUND_OVERRIDE.unwrap_or(center_foreground),
        );
        contents(ui)
    })
    .inner
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct StatusRowLayout {
    page: Rect,
    center: Rect,
    icons: [Rect; 3],
    slider: Rect,
    percentage: Rect,
    item_spacing: f32,
}

/// Reserve the right-hand controls first, then bound the page and mirrored center
/// regions against the status row. At a physically unusable width the right-side
/// slots and their gaps shrink proportionally; they remain bounded and
/// nonoverlapping but are not promised usable targets.
fn plan_status_row(
    available: Rect,
    item_spacing: f32,
    page_label_width: f32,
    percentage_width: f32,
    icon_widths: [f32; 3],
    slider_min: f32,
    slider_max: f32,
) -> StatusRowLayout {
    let nonnegative = |value: f32| {
        if value.is_finite() {
            value.max(0.0)
        } else {
            0.0
        }
    };
    let width = nonnegative(available.width());
    let height = nonnegative(available.height());
    let spacing = nonnegative(item_spacing);
    let icons = icon_widths.map(nonnegative);
    let percentage_width = nonnegative(percentage_width);
    let slider_min = nonnegative(slider_min);
    let slider_max = nonnegative(slider_max).max(slider_min);
    let page_label_width = nonnegative(page_label_width);

    let fixed_right_width = icons.iter().sum::<f32>()
        + percentage_width
        + slider_min
        + spacing * 4.0;
    let compression = if width < fixed_right_width && fixed_right_width > 0.0 {
        width / fixed_right_width
    } else {
        1.0
    };
    let spacing = spacing * compression;
    let icons = icons.map(|value| value * compression);
    let percentage_width = percentage_width * compression;
    let mut slider_width = slider_min * compression;
    let page_width = if compression < 1.0 {
        0.0
    } else {
        let left_available = (width - fixed_right_width).max(0.0);
        // The page stays left-aligned and cannot enter the mirrored center
        // region. Before slider growth, its usable left allocation is bounded
        // by both the right-group width and the space before that group.
        let bounded_page_width = fixed_right_width.min(left_available);
        page_label_width.min(bounded_page_width)
    };
    if compression == 1.0 {
        let available_for_growth = (width - fixed_right_width - page_width).max(0.0);
        slider_width += available_for_growth.min(slider_max - slider_min);
    }

    let right_width = icons.iter().sum::<f32>()
        + percentage_width
        + slider_width
        + spacing * 4.0;
    let row_min = available.min;
    let row_right = row_min.x + width;
    let make_rect = |left: f32, width: f32| {
        Rect::from_min_size(egui::pos2(left, row_min.y), Vec2::new(width.max(0.0), height))
    };

    let mut right_edge = row_right;
    let percentage = make_rect(right_edge - percentage_width, percentage_width);
    right_edge = percentage.left() - spacing;
    let slider = make_rect(right_edge - slider_width, slider_width);
    right_edge = slider.left() - spacing;
    let appearance = make_rect(right_edge - icons[2], icons[2]);
    right_edge = appearance.left() - spacing;
    let focus = make_rect(right_edge - icons[1], icons[1]);
    right_edge = focus.left() - spacing;
    let timer = make_rect(right_edge - icons[0], icons[0]);
    let center_width = (width - 2.0 * right_width).max(0.0);
    let center = if center_width > 0.0 {
        make_rect(row_min.x + right_width, center_width)
    } else {
        // At narrow widths the mirrored interval is empty. Keep its collapsed
        // position on Timer's left boundary, without crossing controls.
        make_rect(timer.left(), 0.0)
    };

    StatusRowLayout {
        page: make_rect(row_min.x, page_width),
        center,
        icons: [timer, focus, appearance],
        slider,
        percentage,
        item_spacing: spacing,
    }
}

pub(super) fn resolved_ui_pixels_per_point(native: Option<f32>) -> f32 {
    native
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(1.0)
}

pub(super) fn ui_zoom_factor(native: Option<f32>) -> f32 {
    let valid_native = native.filter(|value| value.is_finite() && *value > 0.0);
    resolved_ui_pixels_per_point(valid_native) / valid_native.unwrap_or(1.0)
}

pub(super) fn zoom_after_native_factor(current_percent: f64, factor: f32) -> Option<f64> {
    if !current_percent.is_finite() || !factor.is_finite() || factor <= 0.0 {
        return None;
    }
    Some(
        (current_percent * f64::from(factor))
            .clamp(MIN_ZOOM_PERCENT as f64, MAX_ZOOM_PERCENT as f64),
    )
}

pub(super) fn normalized_zoom_percent(value: f64) -> u16 {
    if !value.is_finite() {
        return 100;
    }
    value
        .round()
        .clamp(MIN_ZOOM_PERCENT as f64, MAX_ZOOM_PERCENT as f64) as u16
}

fn zoom_controls(
    ui: &mut egui::Ui,
    palette: StatusPalette,
    zoom_percent: &mut u16,
    slider_rect: Rect,
    percentage_rect: Rect,
    percentage_label: &str,
) -> (bool, Option<egui::Response>) {
    let old_value = *zoom_percent;
    let slider_response = (slider_rect.width() > 0.0).then(|| {
        one_line_region_at(ui, slider_rect, |slider_ui| {
            zoom_slider(slider_ui, palette, slider_rect.width(), zoom_percent)
        })
        .inner
    });

    one_line_region_at(ui, percentage_rect, |percentage_ui| {
        zoom_presets_button(percentage_ui, zoom_percent, percentage_label)
    });
    (*zoom_percent != old_value, slider_response)
}

fn zoom_slider(
    ui: &mut egui::Ui,
    palette: StatusPalette,
    width: f32,
    zoom_percent: &mut u16,
) -> egui::Response {
    let response = ui
        .scope(|ui| {
            ui.spacing_mut().slider_rail_height = presentation::ZOOM_SLIDER_RAIL_HEIGHT;
            ui.visuals_mut().widgets.inactive.bg_fill = palette.slider_rail;
            ui.visuals_mut().selection.bg_fill = palette.slider_trailing_fill;
            // egui Slider allocates from Spacing::slider_width internally;
            // add_sized alone constrains its child UI but leaves that stock
            // response at the default 100 points. Match the planner-owned slot.
            ui.spacing_mut().slider_width = width;
            ui.add_sized(
                [width, presentation::STATUS_ROW_HEIGHT],
                egui::Slider::new(zoom_percent, MIN_ZOOM_PERCENT..=MAX_ZOOM_PERCENT)
                    .integer()
                    .trailing_fill(true)
                    .show_value(false)
                    .handle_shape(egui::style::HandleShape::Circle),
            )
        })
        .inner;
    if response.has_focus() {
        if ui.input(|input| input.key_pressed(egui::Key::Home)) {
            *zoom_percent = MIN_ZOOM_PERCENT;
        } else if ui.input(|input| input.key_pressed(egui::Key::End)) {
            *zoom_percent = MAX_ZOOM_PERCENT;
        }
    }
    response.widget_info(|| {
        WidgetInfo::slider(
            response.enabled(),
            f64::from(*zoom_percent),
            "Page zoom",
        )
    });
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_label("Page zoom");
        node.set_numeric_value(f64::from(*zoom_percent));
        node.set_min_numeric_value(f64::from(MIN_ZOOM_PERCENT));
        node.set_max_numeric_value(f64::from(MAX_ZOOM_PERCENT));
        node.set_numeric_value_step(1.0);
    });
    response
}

fn paint_zoom_slider_knob(
    ui: &egui::Ui,
    palette: StatusPalette,
    response: &egui::Response,
    zoom_percent: u16,
    right_group_rect: Rect,
    status_row_rect: Rect,
) {
    let clip_rect = right_group_rect
        .intersect(status_row_rect)
        .intersect(ui.clip_rect());
    if !ui.is_rect_visible(clip_rect) {
        return;
    }

    // Match egui 0.34's horizontal Circle handle geometry: the center range is
    // inset by handle_radius, while interaction expansion affects the painted
    // circle radius but not the value-to-position mapping.
    let handle_radius = response.rect.height() / 2.5;
    let position_range = response.rect.x_range().shrink(handle_radius);
    let normalized = (f32::from(zoom_percent - MIN_ZOOM_PERCENT)
        / f32::from(MAX_ZOOM_PERCENT - MIN_ZOOM_PERCENT))
        .clamp(0.0, 1.0);
    let center = egui::pos2(
        egui::emath::lerp(position_range, normalized),
        response.rect.center().y,
    );

    let visuals = ui.style().interact(response);
    let foreground = if response.hovered() {
        presentation::HOVERED_FOREGROUND_OVERRIDE.unwrap_or(palette.slider_knob_hover)
    } else {
        presentation::DEFAULT_FOREGROUND_OVERRIDE.unwrap_or(palette.slider_knob)
    };
    let painter = ui.painter().with_clip_rect(clip_rect);
    painter.circle(
        center,
        handle_radius + visuals.expansion,
        foreground,
        egui::Stroke::new(visuals.fg_stroke.width, foreground),
    );
}

fn paint_status_hover_control(
    ui: &egui::Ui,
    response: egui::Response,
    hover_rect: Rect,
) -> (egui::Response, Color32) {
    let hover_rect = hover_rect.intersect(ui.max_rect()).intersect(ui.clip_rect());
    let response = response.on_hover_cursor(toolbar_cursor_icon());
    if response.hovered() {
        let fill = ui.style().visuals.widgets.hovered.bg_fill;
        if fill != Color32::TRANSPARENT {
            ui.painter().rect_filled(hover_rect, 2.0, fill);
        }
        (response, presentation::hovered_foreground(ui))
    } else {
        (response, presentation::default_foreground(ui))
    }
}

fn zoom_presets_button(
    ui: &mut egui::Ui,
    zoom_percent: &mut u16,
    percentage_label: &str,
) -> egui::Response {
    let rect = ui.max_rect();
    let hover_rect = rect.intersect(ui.max_rect()).intersect(ui.clip_rect());
    let response = ui
        .interact(
            hover_rect,
            ui.make_persistent_id("status-page-zoom-presets"),
            Sense::click(),
        );
        // .on_hover_text(format!("Page Zoom"));
    let accessible_name = format!("Page zoom presets, currently {percentage_label}");
    let (response, text_color) = paint_status_hover_control(ui, response, hover_rect);
    let button_response = interactive_response(ui, response, &accessible_name);
    let galley = with_status_text_style(ui, |ui| {
        WidgetText::from(percentage_label.to_owned()).into_galley(
            ui,
            None,
            rect.width(),
            egui::TextStyle::Button,
        )
    });
    let text_left = (rect.center().x - galley.size().x / 2.0)
        .clamp(rect.left(), (rect.right() - galley.size().x).max(rect.left()));
    let text_position = egui::pos2(text_left, rect.center().y - galley.size().y / 2.0);
    let painter_clip = rect.intersect(ui.clip_rect());
    ui.painter()
        .with_clip_rect(painter_clip)
        .galley(text_position, galley, text_color);

    egui::Popup::menu(&button_response.response)
        .open_memory(
            button_response
                .activated
                .then_some(egui::SetOpenCommand::Toggle),
        )
        .show(|ui| {
            ui.set_min_width(72.0);
            for preset in ZOOM_PRESETS {
                ui.selectable_value(zoom_percent, preset, format!("{preset}%"));
            }
        });
    button_response.response
}

#[allow(dead_code)] // WO-002 defines reusable interaction capabilities without assigning actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StatusItemKind {
    Text,
    ClickableText,
    ClickableIcon,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StatusItem {
    pub(super) id: &'static str,
    pub(super) label: String,
    pub(super) accessible_name: String,
    pub(super) kind: StatusItemKind,
}

impl StatusItem {
    #[allow(dead_code)] // The reusable text capability is reserved for status-item consumers.
    pub(super) fn text(id: &'static str, label: impl Into<String>) -> Self {
        let label = label.into();
        Self {
            id,
            accessible_name: label.clone(),
            label,
            kind: StatusItemKind::Text,
        }
    }

    #[allow(dead_code)]
    pub(super) fn clickable_text(
        id: &'static str,
        label: impl Into<String>,
        accessible_name: impl Into<String>,
    ) -> Self {
        Self {
            id,
            label: label.into(),
            accessible_name: accessible_name.into(),
            kind: StatusItemKind::ClickableText,
        }
    }

    #[allow(dead_code)]
    pub(super) fn clickable_icon(
        id: &'static str,
        icon: impl Into<String>,
        accessible_name: impl Into<String>,
    ) -> Self {
        Self {
            id,
            label: icon.into(),
            accessible_name: accessible_name.into(),
            kind: StatusItemKind::ClickableIcon,
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)] // WO-002 exposes interaction capabilities without inventing actions.
pub(super) struct StatusItemResponse {
    pub(super) response: egui::Response,
    pub(super) activated: bool,
}

fn interactive_response(
    ui: &egui::Ui,
    response: egui::Response,
    accessible_name: &str,
) -> StatusItemResponse {
    response.widget_info(|| {
        WidgetInfo::labeled(
            WidgetType::Button,
            response.enabled(),
            accessible_name.to_owned(),
        )
    });
    let keyboard_activated = response.has_focus()
        && ui.input(|input| {
            input.key_pressed(egui::Key::Enter) || input.key_pressed(egui::Key::Space)
        });
    let activated = response.clicked() || keyboard_activated;
    StatusItemResponse {
        response,
        activated,
    }
}

fn status_icon_width() -> f32 {
    presentation::ICON_SLOT_WIDTH
}

fn status_icon_control(
    ui: &mut egui::Ui,
    icon: MaterialIcon,
    stable_id: &'static str,
    enabled: bool,
    accessible_name: &str,
) -> StatusItemResponse {
    ui.add_enabled_ui(enabled, |ui| {
        ui.push_id(stable_id, |ui| {
            let padding = presentation::ICON_HOVER_PADDING;
            let row_height = presentation::STATUS_ROW_HEIGHT;
            let width = status_icon_width().min(ui.available_width().max(0.0));
            let icon_control_size = Vec2::new(
                (width - padding * 2.0).max(1.0),
                (row_height - padding * 2.0).max(1.0),
            );
            let slot_size = icon_control_size + Vec2::splat(padding * 2.0);
            let (id, slot_rect) = ui.allocate_space(slot_size);
            let icon_control_rect = Rect::from_center_size(slot_rect.center(), icon_control_size);
            let hover_rect = toolbar_icon_hover_rect(
                icon_control_rect,
                crate::toolbar::ToolbarIconHoverPadding::uniform(padding),
            )
                .intersect(ui.max_rect())
                .intersect(ui.clip_rect());
            let response = ui.interact(hover_rect, id.with("status-icon"), Sense::click());
            let (response, text_color) = paint_status_hover_control(ui, response, hover_rect);

            let galley = WidgetText::from(
                crate::material_font::rich_text(icon, None).size(presentation::ICON_GLYPH_SIZE),
            )
            .into_galley(
                ui,
                None,
                icon_control_rect.width(),
                egui::TextStyle::Button,
            );
            let text_position = icon_control_rect.center() - galley.size() / 2.0;
            ui.painter().galley(text_position, galley, text_color);

            interactive_response(ui, response, accessible_name)
        })
        .inner
    })
    .inner
}

/// Render a status-item capability without assigning it an application action.
/// The caller may route the activation result; no action is stored or invented here.
#[allow(dead_code)] // Kept as the renderer for the reusable status-item capability model.
pub(super) fn render_item(ui: &mut egui::Ui, item: &StatusItem) -> Option<StatusItemResponse> {
    match item.kind {
        StatusItemKind::Text => {
            ui.push_id(item.id, |ui| {
                with_status_text_style(ui, |ui| {
                    paint_noninteractive_text(
                        ui,
                        item.id,
                        &item.label,
                        presentation::default_foreground(ui),
                    );
                });
            });
            None
        }
        StatusItemKind::ClickableText => {
            let response = ui
                .push_id(item.id, |ui| {
                    ui.scope(|ui| {
                        ui.spacing_mut().interact_size.y = presentation::STATUS_ROW_HEIGHT;
                        ui.button(&item.label)
                    })
                    .inner
                })
                .inner;
            Some(interactive_response(ui, response, &item.accessible_name))
        }
        StatusItemKind::ClickableIcon => {
            let response = ui
                .push_id(item.id, |ui| {
                    ui.add_sized(
                        Vec2::new(
                            ui.spacing().interact_size.x,
                            presentation::STATUS_ROW_HEIGHT,
                        ),
                        egui::Button::new(&item.label)
                            .frame(false)
                            .sense(Sense::click()),
                    )
                })
                .inner;
            Some(interactive_response(ui, response, &item.accessible_name))
        }
    }
}

/// Lay out and paint a label without creating an egui `Response` or pointer hit
/// target. Keep the same wrapping and text style as `Ui::label`, and publish an
/// equivalent AccessKit label node when the backend has accessibility enabled.
fn paint_noninteractive_text(
    ui: &mut egui::Ui,
    id_source: &str,
    text: &str,
    color: Color32,
) -> Rect {
    let mut layout_job = WidgetText::from(text)
        .into_layout_job(ui.style(), FontSelection::Default, ui.text_valign())
        .as_ref()
        .clone();
    let available_width = ui.available_width();
    let (galley_pos, galley, bounds) = if ui.wrap_mode() == egui::TextWrapMode::Wrap
        && ui.layout().main_dir() == Direction::LeftToRight
        && ui.layout().main_wrap()
        && available_width.is_finite()
    {
        let cursor = ui.cursor();
        let first_row_indentation = available_width - ui.available_size_before_wrap().x;
        layout_job.wrap.max_width = available_width;
        layout_job.first_row_min_height = cursor.height();
        layout_job.halign = Align::Min;
        layout_job.justify = false;
        if let Some(first_section) = layout_job.sections.first_mut() {
            first_section.leading_space = first_row_indentation;
        }
        let galley = ui.painter().layout_job(layout_job);
        let pos = egui::pos2(ui.max_rect().left(), ui.cursor().top());
        let mut bounds = Rect::NOTHING;
        for row in &galley.rows {
            let row_rect = row.rect().translate(pos.to_vec2());
            ui.advance_cursor_after_rect(row_rect);
            bounds = bounds.union(row_rect);
        }
        (pos, galley, bounds)
    } else {
        match ui.wrap_mode() {
            egui::TextWrapMode::Extend => {
                layout_job.wrap.max_width = f32::INFINITY;
            }
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
        let pos = match galley.job.halign {
            Align::LEFT => rect.left_top(),
            Align::Center => rect.center_top(),
            Align::RIGHT => rect.right_top(),
        };
        (pos, galley, rect)
    };

    if ui.is_rect_visible(bounds) {
        ui.painter().galley(galley_pos, galley, color);
    }
    let accessibility_id = ui.make_persistent_id(("status-label", id_source));
    ui.ctx().accesskit_node_builder(accessibility_id, |node| {
        node.set_role(egui::accesskit::Role::Label);
        node.set_label(text.to_owned());
        node.set_bounds(egui::accesskit::Rect {
            x0: bounds.min.x.into(),
            y0: bounds.min.y.into(),
            x1: bounds.max.x.into(),
            y1: bounds.max.y.into(),
        });
    });
    bounds
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VisiblePage {
    Available { one_based: usize, total: usize },
    Unavailable,
}

impl VisiblePage {
    pub(super) fn label(self) -> String {
        match self {
            Self::Available { one_based, total } => format!("{one_based} of {total}"),
            Self::Unavailable => "Page unavailable".to_owned(),
        }
    }
}

fn valid_rect(rect: Rect) -> bool {
    let min = rect.min;
    let max = rect.max;
    min.x.is_finite()
        && min.y.is_finite()
        && max.x.is_finite()
        && max.y.is_finite()
        && max.x > min.x
        && max.y > min.y
}

/// Select the rendered sheet with the greatest vertical overlap against the
/// actual editor clip. Empty overlap selects the nearest sheet interval.
pub(super) fn select_visible_page(page_rects: &[Rect], clip: Rect) -> VisiblePage {
    if page_rects.is_empty()
        || !valid_rect(clip)
        || page_rects.iter().any(|rect| !valid_rect(*rect))
    {
        return VisiblePage::Unavailable;
    }

    let clip_top = clip.top();
    let clip_bottom = clip.bottom();
    let mut best_index = 0;
    let mut best_overlap = 0.0_f32;

    for (index, page) in page_rects.iter().enumerate() {
        let overlap = (page.bottom().min(clip_bottom) - page.top().max(clip_top)).max(0.0);
        if overlap > best_overlap {
            best_overlap = overlap;
            best_index = index;
        }
    }

    if best_overlap == 0.0 {
        let distance = |page: &Rect| {
            if clip_bottom < page.top() {
                page.top() - clip_bottom
            } else if clip_top > page.bottom() {
                clip_top - page.bottom()
            } else {
                0.0
            }
        };
        let mut nearest = f32::INFINITY;
        for (index, page) in page_rects.iter().enumerate() {
            let candidate = distance(page);
            if candidate < nearest {
                nearest = candidate;
                best_index = index;
            }
        }
    }

    match best_index.checked_add(1) {
        Some(one_based) => VisiblePage::Available {
            one_based,
            total: page_rects.len(),
        },
        None => VisiblePage::Unavailable,
    }
}

#[derive(Debug, Clone, PartialEq)]
struct GeometrySnapshot {
    published_frame: u64,
    layout_revision: u64,
    viewport_rect: Rect,
    scale: f32,
    page_rects: Vec<Rect>,
    clip: Rect,
    scroll_offset: Vec2,
}

/// The viewport-center point expressed relative to a bounded page coordinate.
/// `outside_page_offset_points` retains any gap or edge distance separately,
/// so a center in a gap never becomes a fabricated screenplay position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ViewportCenterAnchor {
    pub(super) page_index: usize,
    pub(super) page_local_document_point: Vec2,
    pub(super) outside_page_offset_points: Vec2,
    pub(super) viewport_center: Pos2,
}

/// Caches only actual page and scroll-clip geometry published by the editor.
/// The footer runs first, so it consumes a snapshot only on the immediately
/// following frame and only while its independent geometry inputs still match.
#[derive(Debug, Default)]
pub(super) struct ViewportPageTracker {
    published: Option<GeometrySnapshot>,
    last_valid: Option<VisiblePage>,
    repaint_after_publish: bool,
}

impl ViewportPageTracker {
    pub(super) fn capture_center_anchor(
        &self,
        frame: u64,
        layout_revision: u64,
        viewport_rect: Rect,
        scale: f32,
    ) -> Option<ViewportCenterAnchor> {
        let snapshot = self.published.as_ref()?;
        let age = frame.checked_sub(snapshot.published_frame)?;
        if age > 1
            || snapshot.layout_revision != layout_revision
            || snapshot.viewport_rect != viewport_rect
            || !scale.is_finite()
            || scale <= 0.0
            || (snapshot.scale - scale).abs() > f32::EPSILON
            || !valid_geometry_rect(snapshot.clip)
            || snapshot.page_rects.is_empty()
            || !snapshot.scroll_offset.x.is_finite()
            || !snapshot.scroll_offset.y.is_finite()
            || snapshot.scroll_offset.x < 0.0
            || snapshot.scroll_offset.y < 0.0
            || snapshot
                .page_rects
                .iter()
                .any(|rect| !valid_geometry_rect(*rect))
        {
            return None;
        }

        let viewport_center = snapshot.clip.center();
        let content_point = viewport_center + snapshot.scroll_offset;
        // egui 0.29 lays out ScrollArea children from `inner_rect.min - offset`,
        // so allocated rectangles move with the viewport. Normalize them into
        // content coordinates before comparing with the offset-adjusted center.
        let (page_index, page_rect) = snapshot
            .page_rects
            .iter()
            .enumerate()
            .map(|(index, rect)| (index, rect.translate(snapshot.scroll_offset)))
            .find(|(_, rect)| rect.contains(content_point))
            .or_else(|| {
                snapshot
                    .page_rects
                    .iter()
                    .enumerate()
                    .map(|(index, rect)| (index, rect.translate(snapshot.scroll_offset)))
                    .min_by(|(_, left), (_, right)| {
                        distance_squared_to_rect(content_point, *left)
                            .total_cmp(&distance_squared_to_rect(content_point, *right))
                    })
            })?;

        let bounded_point = Pos2::new(
            content_point.x.clamp(page_rect.left(), page_rect.right()),
            content_point.y.clamp(page_rect.top(), page_rect.bottom()),
        );
        Some(ViewportCenterAnchor {
            page_index,
            page_local_document_point: (bounded_point - page_rect.min) / scale,
            outside_page_offset_points: (content_point - bounded_point),
            viewport_center,
        })
    }

    pub(super) fn visible_page(
        &mut self,
        frame: u64,
        layout_revision: u64,
        viewport_rect: Rect,
        scale: f32,
        invalidated: bool,
    ) -> VisiblePage {
        if invalidated {
            self.published = None;
            self.repaint_after_publish = true;
            return self.last_valid.unwrap_or(VisiblePage::Unavailable);
        }
        let Some(snapshot) = self.published.as_ref() else {
            self.repaint_after_publish = true;
            return self.last_valid.unwrap_or(VisiblePage::Unavailable);
        };
        if frame.checked_sub(snapshot.published_frame) != Some(1)
            || snapshot.layout_revision != layout_revision
            || snapshot.viewport_rect != viewport_rect
            || !scale.is_finite()
            || (snapshot.scale - scale).abs() > f32::EPSILON
            || snapshot.page_rects.is_empty()
        {
            self.repaint_after_publish = true;
            return self.last_valid.unwrap_or(VisiblePage::Unavailable);
        }
        let selected = select_visible_page(&snapshot.page_rects, snapshot.clip);
        if matches!(selected, VisiblePage::Available { .. }) {
            self.last_valid = Some(selected);
            selected
        } else {
            self.repaint_after_publish = true;
            self.last_valid.unwrap_or(VisiblePage::Unavailable)
        }
    }

    // Snapshot publication keeps geometry fields explicit and avoids a wrapper
    // allocation on each editor frame.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn publish(
        &mut self,
        frame: u64,
        layout_revision: u64,
        viewport_rect: Rect,
        scale: f32,
        page_rects: Vec<Rect>,
        clip: Rect,
        scroll_offset: Vec2,
    ) -> bool {
        let next = GeometrySnapshot {
            published_frame: frame,
            layout_revision,
            viewport_rect,
            scale,
            page_rects,
            clip,
            scroll_offset,
        };
        let changed = self.published.as_ref().is_none_or(|old| {
            old.layout_revision != next.layout_revision
                || old.viewport_rect != next.viewport_rect
                || old.scale != next.scale
                || old.page_rects != next.page_rects
                || old.clip != next.clip
                || old.scroll_offset != next.scroll_offset
        });
        self.published = Some(next);
        let request_repaint = changed || self.repaint_after_publish;
        self.repaint_after_publish = false;
        request_repaint
    }

    pub(super) fn invalidate(&mut self) {
        self.published = None;
        self.repaint_after_publish = true;
    }

    pub(super) fn reset_for_document_replacement(&mut self) {
        self.published = None;
        self.last_valid = None;
        self.repaint_after_publish = true;
    }

    pub(super) fn clip(&self) -> Option<Rect> {
        self.published.as_ref().map(|snapshot| snapshot.clip)
    }
}

fn valid_geometry_rect(rect: Rect) -> bool {
    rect.min.x.is_finite()
        && rect.min.y.is_finite()
        && rect.max.x.is_finite()
        && rect.max.y.is_finite()
        && rect.width() > 0.0
        && rect.height() > 0.0
}

fn distance_squared_to_rect(point: Pos2, rect: Rect) -> f32 {
    let dx = if point.x < rect.left() {
        rect.left() - point.x
    } else if point.x > rect.right() {
        point.x - rect.right()
    } else {
        0.0
    };
    let dy = if point.y < rect.top() {
        rect.top() - point.y
    } else if point.y > rect.bottom() {
        point.y - rect.bottom()
    } else {
        0.0
    };
    dx * dx + dy * dy
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CenterProjection {
    pub(super) text: String,
    pub(super) modifier_list: bool,
}

fn ready_center() -> CenterProjection {
    CenterProjection {
        text: "Ready".to_owned(),
        modifier_list: false,
    }
}

fn modifier_list_text(chord: ElementHintChord) -> Option<String> {
    let extended = chord == ElementHintChord::Extended;
    let rows = element_hint_bindings(extended);
    (!rows.is_empty()).then(|| {
        rows.into_iter()
            .map(|(digit, role)| {
                format!(
                    "[{digit}] {}",
                    formatting_for_role(role).fdx_label
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    })
}

pub(super) fn project_center(
    session: &EditingSession,
    focused: bool,
    ime_active: bool,
    migration_modal: bool,
    modifiers: egui::Modifiers,
    platform: ShortcutPlatform,
) -> CenterProjection {
    if !focused || ime_active || migration_modal {
        return ready_center();
    }

    if let Some(chord) = element_hint_chord(modifiers, platform) {
        if let Some(text) = modifier_list_text(chord) {
            return CenterProjection {
                text,
                modifier_list: true,
            };
        }
    }
    if !modifiers_are_none(modifiers) {
        return ready_center();
    }

    let selection = session.selection();
    if selection.anchor != selection.focus {
        return ready_center();
    }
    let caret = selection.focus;
    let Some(element) = session.project().elements.get(caret.element) else {
        return ready_center();
    };
    if caret.offset != element.text.len() {
        return ready_center();
    }

    let targets = session.paragraph_key_targets();
    let mut parts = Vec::with_capacity(3);
    if let ParagraphKeyTarget::Target(role) = targets.tab {
        parts.push(format!("[Tab] {}", formatting_for_role(role).fdx_label));
    }
    if let ParagraphKeyTarget::Target(role) = targets.enter {
        parts.push(format!("[Enter] {}", formatting_for_role(role).fdx_label));
    }
    if parts.is_empty() {
        return ready_center();
    }

    let hold_label = match platform {
        ShortcutPlatform::MacOS => Some("[Cmd] hold for Elements"),
        ShortcutPlatform::Windows => Some("[Ctrl] hold for Elements"),
        ShortcutPlatform::Other => None,
    };
    if let Some(hold_label) = hold_label {
        parts.push(hold_label.to_owned());
    }
    CenterProjection {
        text: parts.join(", "),
        modifier_list: false,
    }
}

fn modifier_hint_font_size(ui: &egui::Ui, text: &str, available_width: f32) -> f32 {
    let base_size = presentation::MODIFIER_HINT_TEXT_SIZE;
    let width = if available_width.is_finite() {
        available_width.max(0.0)
    } else {
        0.0
    };
    if width <= 0.0 {
        return base_size;
    }
    let mut font = status_font_id(ui, egui::TextStyle::Body);
    font.size = base_size;
    let measured_width = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font, presentation::center_foreground(ui))
        .size()
        .x;
    fitted_modifier_hint_font_size(measured_width, width)
}

fn fitted_modifier_hint_font_size(measured_width: f32, available_width: f32) -> f32 {
    let base_size = presentation::MODIFIER_HINT_TEXT_SIZE;
    if !measured_width.is_finite()
        || !available_width.is_finite()
        || measured_width <= available_width.max(0.0)
        || measured_width <= 0.0
    {
        base_size
    } else {
        (base_size * available_width.max(0.0) / measured_width)
            .max(presentation::MODIFIER_HINT_MIN_TEXT_SIZE)
    }
}

fn one_line_region<R>(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.allocate_ui_with_layout(
        size,
        egui::Layout::left_to_right(egui::Align::Center),
        |region| {
            region.set_clip_rect(region.clip_rect().intersect(region.max_rect()));
            region.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            contents(region)
        },
    )
}

fn one_line_region_at<R>(
    ui: &mut egui::Ui,
    rect: Rect,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |region| {
            region.set_clip_rect(region.clip_rect().intersect(rect));
            region.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            let result = contents(region);
            region.set_min_size(rect.size());
            result
        },
    )
}

fn paint_clipped_single_line(
    ui: &egui::Ui,
    id_source: &str,
    text: &str,
    accessible_name: &str,
    color: Color32,
    rect: Rect,
) {
    let mut layout_job = WidgetText::from(text)
        .into_layout_job(ui.style(), FontSelection::Default, Align::Center)
        .as_ref()
        .clone();
    layout_job.wrap.max_width = rect.width();
    layout_job.wrap.max_rows = 1;
    layout_job.wrap.break_anywhere = true;
    layout_job.halign = Align::Min;
    let galley = ui.painter().layout_job(layout_job);
    let centered_left = rect.center().x - galley.size().x * 0.5;
    let pos = egui::pos2(
        centered_left.clamp(rect.left(), (rect.right() - galley.size().x).max(rect.left())),
        rect.center().y - galley.size().y * 0.5,
    );
    if ui.is_rect_visible(rect) {
        ui.painter().with_clip_rect(rect).galley(pos, galley, color);
    }
    let accessibility_id = ui.make_persistent_id(("status-label", id_source));
    ui.ctx().accesskit_node_builder(accessibility_id, |node| {
        node.set_role(egui::accesskit::Role::Label);
        node.set_label(accessible_name.to_owned());
        node.set_bounds(egui::accesskit::Rect {
            x0: rect.min.x.into(),
            y0: rect.min.y.into(),
            x1: rect.max.x.into(),
            y1: rect.max.y.into(),
        });
    });
}

fn page_counter_content_rect(rect: Rect) -> Rect {
    let inset = presentation::PAGE_COUNTER_HORIZONTAL_INSET;
    let inset = if inset.is_finite() { inset.max(0.0) } else { 0.0 };
    let center_x = rect.center().x;
    let left = (rect.left() + inset).min(center_x);
    let right = (rect.right() - inset).max(center_x);
    Rect::from_min_max(egui::pos2(left, rect.top()), egui::pos2(right, rect.bottom()))
}

fn paint_page_counter(ui: &egui::Ui, text: &str, rect: Rect, color: Color32) {
    let mut layout_job = WidgetText::from(text)
        .into_layout_job(ui.style(), FontSelection::Default, Align::Center)
        .as_ref()
        .clone();
    layout_job.wrap.max_width = rect.width();
    layout_job.wrap.max_rows = 1;
    layout_job.wrap.break_anywhere = true;
    layout_job.halign = Align::Min;
    let galley = ui.painter().layout_job(layout_job);
    let position = egui::pos2(rect.left(), rect.center().y - galley.size().y * 0.5);
    if ui.is_rect_visible(rect) {
        ui.painter().with_clip_rect(rect).galley(position, galley, color);
    }
    let accessibility_id = ui.make_persistent_id(("status-label", "visible-page"));
    ui.ctx().accesskit_node_builder(accessibility_id, |node| {
        node.set_role(egui::accesskit::Role::Label);
        node.set_label(text.to_owned());
        node.set_bounds(egui::accesskit::Rect {
            x0: rect.min.x.into(),
            y0: rect.min.y.into(),
            x1: rect.max.x.into(),
            y1: rect.max.y.into(),
        });
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DiagnosticProjection<'a> {
    pub(super) focused: bool,
    pub(super) focus_lock: bool,
    pub(super) dispatch_enabled: bool,
    pub(super) received_events: u32,
    pub(super) dispatched_frames: u32,
    pub(super) pointer_activations: u32,
    pub(super) last_event_category: &'a str,
    pub(super) performance_events: u64,
    pub(super) layout_builds: u64,
    pub(super) layout_cache_hits: u64,
    pub(super) painted_pages: u64,
    pub(super) repaint_requests: u64,
    pub(super) performance_summary: &'a str,
    pub(super) diagnostic_message: Option<&'a str>,
}

impl DiagnosticProjection<'_> {
    pub(super) fn safe_summary(self) -> String {
        let summary = format!(
            "focus={} lock={} dispatch={} received-events={} dispatched-frames={} pointer-activations={} last-category={} performance-events={} layouts={} cache-hits={} painted-pages={} repaint-requests={}",
            self.focused,
            self.focus_lock,
            self.dispatch_enabled,
            self.received_events,
            self.dispatched_frames,
            self.pointer_activations,
            if self.last_event_category.is_empty() {
                "none"
            } else {
                self.last_event_category
            },
            self.performance_events,
            self.layout_builds,
            self.layout_cache_hits,
            self.painted_pages,
            self.repaint_requests
        );
        if self.performance_summary.is_empty() {
            summary
        } else {
            format!("{summary} {}", self.performance_summary)
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct StatusCommandIntents {
    pub(super) commands: Vec<Command>,
    pub(super) timer_button_rect: Option<Rect>,
}

/// Surface-specific status projection. False options suppress the corresponding
/// widget and accessibility node while retaining the existing row geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StatusSurfaceOptions {
    pub(super) show_center: bool,
    pub(super) show_focus_mode: bool,
    show_diagnostics: bool,
}

fn diagnostics_disclosure_enabled(development_mode: bool, options: StatusSurfaceOptions) -> bool {
    development_mode && options.show_diagnostics
}

impl StatusSurfaceOptions {
    pub(super) const BODY: Self = Self {
        show_center: true,
        show_focus_mode: true,
        show_diagnostics: true,
    };

    pub(super) const TITLE_PAGE: Self = Self {
        show_center: false,
        show_focus_mode: false,
        show_diagnostics: false,
    };
}

#[allow(clippy::too_many_arguments)] // Keep transient timer ownership at the app caller.
pub(super) fn show_with_focus_mode(
    ui: &mut egui::Ui,
    palette: StatusPalette,
    page: VisiblePage,
    center: &CenterProjection,
    diagnostics: Option<DiagnosticProjection<'_>>,
    development_mode: bool,
    zoom_percent: u16,
    appearance: AppearanceMode,
    focus_view: &FocusModeView,
 ) -> StatusCommandIntents {
    show_impl_with_appearance(
        ui,
        palette,
        page,
        center,
        diagnostics,
        development_mode,
        zoom_percent,
        appearance,
        None,
        Some(focus_view),
        StatusSurfaceOptions::BODY,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn show_title_page(
    ui: &mut egui::Ui,
    palette: StatusPalette,
    page: VisiblePage,
    zoom_percent: u16,
    appearance: AppearanceMode,
) -> StatusCommandIntents {
    show_impl_with_appearance(
        ui,
        palette,
        page,
        &ready_center(),
        None,
        false,
        zoom_percent,
        appearance,
        None,
        None,
        StatusSurfaceOptions::TITLE_PAGE,
    )
}

/// Keep the exclusive center projection visible while the migration dialog
/// suppresses the ordinary application panels. No status controls or
/// Diagnostics disclosure are introduced into the modal surface.
pub(super) fn show_modal_ready_status(ui: &mut egui::Ui, palette: StatusPalette) {
    apply_status_palette(ui, palette);
    let available = ui.available_rect_before_wrap().intersect(ui.clip_rect());
    let row_rect = Rect::from_min_size(
        available.min,
        Vec2::new(
            available.width().max(0.0),
            presentation::STATUS_ROW_HEIGHT.min(available.height().max(0.0)),
        ),
    );
    let center = ready_center();
    one_line_region_at(ui, row_rect, |region| {
        let status_rect = region.max_rect().intersect(region.clip_rect());
        with_center_text_style(
            region,
            palette.center_foreground,
            presentation::CENTER_TEXT_SIZE,
            |region| {
                paint_clipped_single_line(
                    region,
                    "status-center",
                    &center.text,
                    &center.text,
                    presentation::center_foreground(region),
                    status_rect,
                );
            },
        );
    });
}

fn measured_percentage_slot_width(ui: &egui::Ui) -> f32 {
    let text_width = ui
        .painter()
        .layout_no_wrap(
            format!("{MAX_ZOOM_PERCENT}%"),
            status_font_id(ui, egui::TextStyle::Button),
            presentation::default_foreground(ui),
        )
    .size()
    .x;
    text_width + presentation::PERCENTAGE_SIDE_RESERVATION * 2.0
}

fn appearance_menu(ui: &mut egui::Ui, appearance: &mut AppearanceMode) -> bool {
    let previous = *appearance;
    let control = status_icon_control(
        ui,
        ICON_MODE_NIGHT.filled(),
        "appearance-status-item",
        true,
        "Appearance mode",
    );
    control
        .response
        .clone()
        .on_hover_text("Appearance");
    egui::Popup::menu(&control.response)
        .open_memory(control.activated.then_some(egui::SetOpenCommand::Toggle))
        .show(|ui| {
        ui.set_min_width(156.0);
        for choice in [
            AppearanceMode::System,
            AppearanceMode::Day,
            AppearanceMode::Night,
            AppearanceMode::Midnight,
        ] {
            ui.selectable_value(appearance, choice, choice.label());
        }
    });
    *appearance != previous
}

#[allow(clippy::too_many_arguments)] // Explicit projections and caller-owned state avoid hidden status globals.
fn show_impl_with_appearance(
    ui: &mut egui::Ui,
    palette: StatusPalette,
    page: VisiblePage,
    center: &CenterProjection,
    diagnostics: Option<DiagnosticProjection<'_>>,
    development_mode: bool,
    zoom_value: u16,
    appearance_value: AppearanceMode,
    diagnostics_open: Option<bool>,
    focus_view: Option<&FocusModeView>,
    options: StatusSurfaceOptions,
 ) -> StatusCommandIntents {
    let mut zoom_percent = zoom_value;
    let mut appearance = appearance_value;
    let mut commands = Vec::new();
    let mut zoom_changed = false;
    let mut appearance_changed = false;
    let mut timer_rect = None;
    let mut zoom_slider_response = None;
    let row_height = presentation::STATUS_ROW_HEIGHT;
    let available = ui.available_rect_before_wrap().intersect(ui.clip_rect());
    let row_rect = Rect::from_min_size(
        available.min,
        Vec2::new(available.width().max(0.0), row_height.min(available.height().max(0.0))),
    );
    let page_label = page.label();
    let page_label_text_width = ui
        .painter()
        .layout_no_wrap(
            page_label.clone(),
            status_font_id(ui, egui::TextStyle::Body),
            presentation::default_foreground(ui),
        )
        .size()
        .x;
    let page_inset = presentation::PAGE_COUNTER_HORIZONTAL_INSET;
    let page_inset = if page_inset.is_finite() {
        page_inset.max(0.0)
    } else {
        0.0
    };
    let page_label_width = page_label_text_width + page_inset * 2.0;
    let percentage_label = format!("{zoom_percent}%");
    let percentage_width = measured_percentage_slot_width(ui);
    let icon_width = status_icon_width();
    let item_spacing = presentation::ITEM_GAP;
    let layout = plan_status_row(
        row_rect,
        item_spacing,
        page_label_width,
        percentage_width,
        [icon_width; 3],
        presentation::ZOOM_SLIDER_MIN_WIDTH,
        presentation::ZOOM_SLIDER_MAX_WIDTH,
    );
    let right_left = layout.icons[0].left();
    let right_rect = Rect::from_min_max(
        egui::pos2(right_left, row_rect.top()),
        egui::pos2(row_rect.right(), row_rect.bottom()),
    );
    let planned_zoom = Rect::from_min_max(
        egui::pos2(layout.slider.left(), row_rect.top()),
        egui::pos2(layout.percentage.right(), row_rect.bottom()),
    );
    // Process the complete right group first so its interaction state is
    // established before the page label is painted.
    one_line_region(ui, row_rect.size(), |row_ui| {
        one_line_region_at(row_ui, right_rect, |right_ui| {
            right_ui.spacing_mut().item_spacing.x = layout.item_spacing;
            right_ui.with_layout(
                egui::Layout::right_to_left(egui::Align::Center),
                |right_ui| {
                    one_line_region_at(
                        right_ui,
                        planned_zoom,
                        |zoom_ui| {
                            let (changed, slider_response) = zoom_controls(
                                zoom_ui,
                                palette,
                                &mut zoom_percent,
                                layout.slider,
                                layout.percentage,
                                &percentage_label,
                            );
                            zoom_changed |= changed;
                            zoom_slider_response = slider_response;
                        },
                    );
                    one_line_region(
                        right_ui,
                        Vec2::new(layout.icons[2].width(), row_height),
                        |icon_ui| {
                            appearance_changed |= appearance_menu(icon_ui, &mut appearance);
                        },
                    );

                    one_line_region(
                        right_ui,
                        Vec2::new(layout.icons[1].width(), row_height),
                        |icon_ui| {
                            if options.show_focus_mode {
                                if let Some(view) = focus_view {
                                let accessible_name = view.accessible_name();
                                let rendered = status_icon_control(
                                    icon_ui,
                                    ICON_LOCATION_SEARCHING.outlined(),
                                    "focus-mode-status-item",
                                    view.enabled(),
                                    &accessible_name,
                                );
                                rendered
                                    .response
                                    .clone()
                                    .on_hover_text("Focus Mode");
                                if rendered.activated {
                                    commands.push(Command::ViewFocusModeToggle);
                                }
                                }
                            }
                        },
                    );

                    one_line_region(
                        right_ui,
                        Vec2::new(layout.icons[0].width(), row_height),
                        |icon_ui| {
                            let rendered = status_icon_control(
                                icon_ui,
                                ICON_TIMER.outlined(),
                                "sprint-timer-status-item",
                                true,
                                "Sprint Timer",
                            );
                            rendered.response.clone().on_hover_text("Sprint Timer");
                            timer_rect = Some(rendered.response.rect);
                            if rendered.activated {
                                commands.push(Command::ToolsSprintTimerTogglePopup);
                            }
                        },
                    );

                    // Paint the custom knob after every ordinary right-group
                    // control so later sibling hover fills cannot cover it.
                    if let Some(response) = zoom_slider_response.as_ref() {
                        paint_zoom_slider_knob(
                            right_ui,
                            palette,
                            response,
                            zoom_percent,
                            right_rect,
                            row_rect,
                        );
                    }
                },
            );
        });
        if options.show_center {
            one_line_region_at(row_ui, layout.center, |region| {
                let status_rect = region.max_rect().intersect(region.clip_rect());
                let text_size = if center.modifier_list {
                    modifier_hint_font_size(region, &center.text, status_rect.width())
                } else {
                    presentation::CENTER_TEXT_SIZE
                };
                with_center_text_style(region, palette.center_foreground, text_size, |region| {
                    paint_clipped_single_line(
                        region,
                        "status-center",
                        &center.text,
                        &center.text,
                        presentation::center_foreground(region),
                        status_rect,
                    );
                });
            });
        }

        one_line_region_at(row_ui, layout.page, |page_ui| {
            let label = page.label();
            let page_rect = page_counter_content_rect(page_ui.max_rect())
                .intersect(page_ui.clip_rect())
                .intersect(page_ui.max_rect());
            with_status_text_style(page_ui, |page_ui| {
                paint_page_counter(
                    page_ui,
                    &label,
                    page_rect,
                    presentation::default_foreground(page_ui),
                );
            });
        });
    });
    if let (true, Some(diagnostics)) =
        (diagnostics_disclosure_enabled(development_mode, options), diagnostics)
    {
        with_status_text_style(ui, |ui| {
            let default_foreground = presentation::default_foreground(ui);
            ui.visuals_mut().override_text_color = Some(default_foreground);
            egui::CollapsingHeader::new("Diagnostics")
                .id_salt("status-bar-diagnostics")
                .default_open(false)
                .open(diagnostics_open)
                .show(ui, |ui| {
                    let summary = diagnostics.safe_summary();
                    let summary_height = ui.text_style_height(&egui::TextStyle::Body);
                    let summary_width = ui.available_width();
                    one_line_region(
                        ui,
                        egui::vec2(summary_width, summary_height),
                        |summary_ui| {
                            paint_noninteractive_text(
                                summary_ui,
                                "status-diagnostics-summary",
                                &summary,
                                presentation::default_foreground(summary_ui),
                            );
                        },
                    );
                    if let Some(message) = diagnostics.diagnostic_message {
                        one_line_region(
                            ui,
                            egui::vec2(summary_width, summary_height),
                            |diagnostic_ui| {
                                paint_noninteractive_text(
                                    diagnostic_ui,
                                    "status-diagnostic-message",
                                    message,
                                    presentation::default_foreground(diagnostic_ui),
                                );
                            },
                        );
                    }
                });
        });
    }
    if zoom_changed {
        if let Ok(percent) = ZoomPercent::new(zoom_percent) {
            commands.push(Command::ViewZoomSet { percent });
        }
    }
    if appearance_changed {
        let appearance = match appearance {
            AppearanceMode::System => Appearance::System,
            AppearanceMode::Day => Appearance::Day,
            AppearanceMode::Night => Appearance::Night,
            AppearanceMode::Midnight => Appearance::Midnight,
        };
        commands.push(Command::ViewAppearanceSet { appearance });
    }
    StatusCommandIntents {
        commands,
        timer_button_rect: timer_rect,
    }
}

#[cfg(test)]
mod diagnostic_status_tests {
    use super::{
        diagnostics_disclosure_enabled, element_hint_bindings, fitted_modifier_hint_font_size,
        modifier_list_text, plan_status_row, project_center, ready_center, CenterProjection,
        ElementHintChord, StatusSurfaceOptions,
    };
    use crate::shortcuts::ShortcutPlatform;
    use eframe::egui;
    use scriptcraft_core::editing::{EditPolicy, EditingSession, Position, Selection};
    use scriptcraft_core::formatting::formatting_for_role;
    use scriptcraft_core::{Project, Role};

    fn valid<T, E>(result: Result<T, E>) -> T {
        result.unwrap_or_else(|_| unreachable!("status projection fixture is valid"))
    }

    fn session(role: Role, text: &str, anchor: usize, focus: usize) -> EditingSession {
        let mut project = valid(Project::new());
        project.elements[0].role = role;
        project.elements[0].text = text.to_owned();
        let mut session = valid(EditingSession::with_policy(project, EditPolicy::default()));
        valid(session.set_selection(Selection {
            anchor: Position { element: 0, offset: anchor },
            focus: Position { element: 0, offset: focus },
        }));
        session
    }

    fn modifiers(platform: ShortcutPlatform, primary: bool, control: bool, shift: bool) -> egui::Modifiers {
        egui::Modifiers {
            alt: false,
            ctrl: platform == ShortcutPlatform::MacOS && control,
            command: primary,
            mac_cmd: platform != ShortcutPlatform::MacOS && control,
            shift,
        }
    }

    fn projection(session: &EditingSession, platform: ShortcutPlatform) -> CenterProjection {
        project_center(
            session,
            true,
            false,
            false,
            egui::Modifiers::NONE,
            platform,
        )
    }

    #[test]
    fn ready_fallback_and_runtime_diagnostics_gate_preserve_body_and_title_page_contracts() {
        assert_eq!(ready_center().text, "Ready");
        assert!(!diagnostics_disclosure_enabled(false, StatusSurfaceOptions::BODY));
        assert!(diagnostics_disclosure_enabled(true, StatusSurfaceOptions::BODY));
        assert!(!diagnostics_disclosure_enabled(true, StatusSurfaceOptions::TITLE_PAGE));
    }

    #[test]
    fn collapsed_empty_and_nonempty_ends_use_shared_targets_for_every_role() {
        for role in Role::ALL {
            for text in ["", "ending"] {
                let editing_session = session(role, text, text.len(), text.len());
                let targets = editing_session.paragraph_key_targets();
                let center = projection(&editing_session, ShortcutPlatform::MacOS);
                let any_target = matches!(
                    targets.tab,
                    scriptcraft_core::editing::ParagraphKeyTarget::Target(_)
                ) || matches!(
                    targets.enter,
                    scriptcraft_core::editing::ParagraphKeyTarget::Target(_)
                );
                assert_eq!(center.text == "Ready", !any_target, "{role:?}: {}", center.text);
                for (key, target) in [("[Tab]", targets.tab), ("[Enter]", targets.enter)] {
                    if let scriptcraft_core::editing::ParagraphKeyTarget::Target(target_role) = target {
                        let expected = format!("{key} {}", formatting_for_role(target_role).fdx_label);
                        assert!(center.text.contains(&expected), "{role:?}: {}", center.text);
                    } else {
                        assert!(!center.text.contains(key), "{role:?}: {}", center.text);
                    }
                }
                if center.text != "Ready" {
                    assert!(center.text.contains("[Cmd] hold for Elements"));
                }
            }
        }
        let empty_action = session(Role::Action, "", 0, 0);
        assert!(projection(&empty_action, ShortcutPlatform::Windows)
            .text
            .contains("[Ctrl] hold for Elements"));
    }

    #[test]
    fn paragraph_hints_require_a_collapsed_end_but_element_chord_has_priority() {
        let interior = session(Role::Action, "abc", 1, 1);
        assert_eq!(projection(&interior, ShortcutPlatform::MacOS).text, "Ready");
        let selected = session(Role::Action, "abc", 0, 3);
        assert_eq!(projection(&selected, ShortcutPlatform::MacOS).text, "Ready");

        let mac_base = modifiers(ShortcutPlatform::MacOS, true, false, false);
        let chord_projection = project_center(
            &selected,
            true,
            false,
            false,
            mac_base,
            ShortcutPlatform::MacOS,
        );
        assert!(chord_projection.text.starts_with("[0] General"));
        assert!(chord_projection.modifier_list);
    }

    #[test]
    fn focus_ime_and_modal_always_suppress_chord_hints() {
        let editing_session = session(Role::Action, "abc", 3, 3);
        for (focused, ime_active, modal) in [
            (false, false, false),
            (true, true, false),
            (true, false, true),
        ] {
            let center = project_center(
                &editing_session,
                focused,
                ime_active,
                modal,
                modifiers(ShortcutPlatform::MacOS, true, false, false),
                ShortcutPlatform::MacOS,
            );
            assert_eq!(center.text, "Ready");
        }
    }

    #[test]
    fn shared_binding_lists_cover_every_role_with_summary_first_extended_order() {
        let base = element_hint_bindings(false);
        let extended = element_hint_bindings(true);
        assert_eq!(base.len(), 10);
        assert_eq!(extended.len(), 7);
        let base_text = modifier_list_text(ElementHintChord::Base).unwrap_or_default();
        let expected_base = [
            "[0] General",
            "[1] Scene Heading",
            "[2] Action",
            "[3] Character",
            "[4] Parenthetical",
            "[5] Dialogue",
            "[6] Transition",
            "[7] Shot",
            "[8] Cast List",
            "[9] New Act",
        ];
        let mut previous = 0;
        for row in expected_base {
            let position = base_text.find(row).unwrap_or(usize::MAX);
            assert!(position >= previous, "{base_text}");
            previous = position;
        }
        let mut roles: Vec<_> = base.iter().chain(&extended).map(|(_, role)| *role).collect();
        roles.sort_unstable();
        let mut all_roles = Role::ALL;
        all_roles.sort_unstable();
        assert_eq!(roles, all_roles);

        let extended_text = modifier_list_text(ElementHintChord::Extended).unwrap_or_default();
        let expected = [
            "[0] Summary",
            "[1] Outline 1",
            "[2] Outline 2",
            "[3] Outline 3",
            "[4] Note",
            "[8] Sequence",
            "[9] End of Act",
        ];
        let mut previous = 0;
        for row in expected {
            let position = extended_text.find(row).unwrap_or(usize::MAX);
            assert!(position >= previous, "{extended_text}");
            previous = position;
        }
    }

    #[test]
    fn host_chords_are_exact_release_restores_paragraph_projection_and_labels_are_host_correct() {
        let editing_session = session(Role::Action, "x", 1, 1);
        for (platform, base, extended, wrong) in [
            (
                ShortcutPlatform::MacOS,
                modifiers(ShortcutPlatform::MacOS, true, false, false),
                modifiers(ShortcutPlatform::MacOS, true, true, false),
                modifiers(ShortcutPlatform::MacOS, true, false, true),
            ),
            (
                ShortcutPlatform::Windows,
                modifiers(ShortcutPlatform::Windows, true, false, false),
                modifiers(ShortcutPlatform::Windows, true, false, true),
                modifiers(ShortcutPlatform::Windows, true, true, false),
            ),
        ] {
            let base_projection = project_center(&editing_session, true, false, false, base, platform);
            assert!(base_projection.text.starts_with("[0] General"));
            let extended_projection =
                project_center(&editing_session, true, false, false, extended, platform);
            assert!(extended_projection.text.starts_with("[0] Summary"));
            assert!(!extended_projection.text.contains("[8] Sequence, [9] End of Act, [0] Summary"));
            let released = project_center(
                &editing_session,
                true,
                false,
                false,
                egui::Modifiers::NONE,
                platform,
            );
            assert!(!released.modifier_list);
            assert!(released.text.contains("[Tab]"));
            let rejected = project_center(&editing_session, true, false, false, wrong, platform);
            assert!(!rejected.modifier_list);
            assert_eq!(rejected.text, "Ready");
        }
        let mac = projection(&editing_session, ShortcutPlatform::MacOS);
        let windows = projection(&editing_session, ShortcutPlatform::Windows);
        assert!(mac.text.ends_with("[Cmd] hold for Elements"));
        assert!(windows.text.ends_with("[Ctrl] hold for Elements"));
    }

    #[test]
    fn modifier_list_font_and_center_geometry_remain_bounded_at_narrow_widths() {
        let base = super::presentation::MODIFIER_HINT_TEXT_SIZE;
        let minimum = super::presentation::MODIFIER_HINT_MIN_TEXT_SIZE;
        assert_eq!(fitted_modifier_hint_font_size(100.0, 200.0), base);
        let reduced = fitted_modifier_hint_font_size(1000.0, 500.0);
        assert!(reduced < base && reduced >= minimum);
        assert_eq!(fitted_modifier_hint_font_size(1000.0, 0.0), minimum);

        for width in [1200.0, 480.0, 320.0] {
            let row = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 20.0));
            let layout = plan_status_row(row, 4.0, 100.0, 40.0, [27.0; 3], 40.0, 84.0);
            assert!(layout.center.left() >= row.left());
            assert!(layout.center.right() <= row.right());
            assert!(layout.center.width() >= 0.0);
            for rect in layout.icons.into_iter().chain([layout.slider, layout.percentage]) {
                assert!(rect.left() >= row.left());
                assert!(rect.right() <= row.right());
            }
        }
    }
}

#[cfg(test)]
mod title_status_surface_tests {
    use super::StatusSurfaceOptions;

    #[test]
    fn title_status_keeps_diagnostics_excluded_while_body_status_allows_runtime_gate() {
        assert_eq!(
            StatusSurfaceOptions::TITLE_PAGE,
            StatusSurfaceOptions {
                show_center: false,
                show_focus_mode: false,
                show_diagnostics: false,
            },
        );
        assert_eq!(
            StatusSurfaceOptions::BODY,
            StatusSurfaceOptions {
                show_center: true,
                show_focus_mode: true,
                show_diagnostics: true,
            },
        );
    }
}

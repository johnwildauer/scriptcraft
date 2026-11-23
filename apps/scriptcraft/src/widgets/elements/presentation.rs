//! Scoped visual helpers shared by the Elements inspector and template selector.

use eframe::egui;

pub(super) const DEFAULT_INSPECTOR_SIZE: egui::Vec2 = egui::vec2(640.0, 480.0);
pub(super) const MIN_INSPECTOR_SIZE: egui::Vec2 = egui::vec2(520.0, 360.0);
pub(super) const DEFAULT_SELECTOR_SIZE: egui::Vec2 = egui::vec2(560.0, 360.0);
pub(super) const MIN_SELECTOR_SIZE: egui::Vec2 = egui::vec2(480.0, 300.0);
pub(super) const VIEWPORT_MARGIN: f32 = 32.0;
pub(super) const LIST_COLUMN_WIDTH: f32 = 176.0;
pub(super) const PANE_GAP: f32 = 10.0;
pub(super) const LIST_INSET: i8 = 4;
pub(super) const DETAIL_INSET: i8 = 8;
pub(super) const SELECTABLE_ROW_HEIGHT: f32 = 23.0;
pub(super) const SELECTABLE_ROW_GAP: f32 = 0.0;
pub(super) const SELECTABLE_ROW_FONT_SIZE: f32 = 12.0;
pub(super) const SELECTABLE_ROW_PADDING_X: f32 = 6.0;
pub(super) const SELECTABLE_ROW_PADDING_Y: f32 = 1.0;
pub(super) const DETAIL_LABEL_COLUMN_WIDTH: f32 = 132.0;
pub(super) const TAB_WIDTH: f32 = 88.0;
pub(super) const TAB_HEIGHT: f32 = 28.0;
pub(super) const INSPECTOR_FOOTER_RESERVED_HEIGHT: f32 = 52.0;
pub(super) const INSPECTOR_FOOTER_HEIGHT: f32 = 34.0;
pub(super) const SELECTOR_FOOTER_ALLOWANCE: f32 = 72.0;
pub(super) const FOOTER_BUTTON_WIDTH: f32 = 88.0;
pub(super) const FOOTER_BUTTON_HEIGHT: f32 = 30.0;
pub(super) const FOOTER_BUTTON_GAP: f32 = 8.0;
pub(super) const WINDOW_CONTENT_HORIZONTAL_ALLOWANCE: f32 = 24.0;
pub(super) const MIN_PANE_WIDTH: f32 = 1.0;
pub(super) const FRAME_BORDER_WIDTH: f32 = 1.0;
pub(super) const FOCUS_STROKE_WIDTH: f32 = 2.0;
pub(super) const FOCUS_STROKE_INSET: f32 = 1.0;
pub(super) const FOOTER_CONTENT_GAP: f32 = 4.0;

pub(super) const DARK_LIST_FILL: egui::Color32 = egui::Color32::from_rgb(29, 29, 29);
pub(super) const DARK_DETAIL_FILL: egui::Color32 = egui::Color32::from_rgb(48, 48, 48);
pub(super) const DARK_PANE_BORDER: egui::Color32 = egui::Color32::from_rgb(68, 68, 68);
pub(super) const DARK_ROW_SELECTED_FILL: egui::Color32 = egui::Color32::from_rgb(49, 95, 191);
pub(super) const DARK_ROW_HOVER_FILL: egui::Color32 = egui::Color32::from_rgb(64, 64, 64);
pub(super) const DARK_ROW_FOCUS_STROKE: egui::Color32 = egui::Color32::from_rgb(198, 215, 255);
pub(super) const DARK_SELECTED_FOREGROUND: egui::Color32 = egui::Color32::WHITE;
pub(super) const DARK_SELECTED_STROKE: egui::Color32 = egui::Color32::WHITE;
pub(super) const DARK_TAB_SELECTED_FILL: egui::Color32 = egui::Color32::from_rgb(92, 92, 92);
pub(super) const DARK_TAB_UNSELECTED_FILL: egui::Color32 = egui::Color32::from_rgb(58, 58, 58);

/// Returns a shared list/gap allocation that preserves room for a detail pane.
pub(super) fn list_pane_widths(available_width: f32) -> (f32, f32) {
    let available_width = available_width.max(MIN_PANE_WIDTH);
    let gap = PANE_GAP.min((available_width - MIN_PANE_WIDTH * 2.0).max(0.0));
    let list_capacity = (available_width - gap - MIN_PANE_WIDTH).max(MIN_PANE_WIDTH);
    (LIST_COLUMN_WIDTH.min(list_capacity), gap)
}

/// Inset frame for a live registry list, with the reference's dark well only in
/// dark appearances. Light appearances retain their normal theme surfaces.
pub(super) fn listbox_frame(ui: &egui::Ui) -> egui::Frame {
    let dark = ui.visuals().dark_mode;
    let visuals = ui.visuals();
    egui::Frame::new()
        .fill(if dark {
            DARK_LIST_FILL
        } else {
            visuals.extreme_bg_color
        })
        .stroke(egui::Stroke::new(FRAME_BORDER_WIDTH, if dark {
            DARK_PANE_BORDER
        } else {
            visuals.widgets.noninteractive.bg_stroke.color
        }))
        .inner_margin(egui::Margin::same(LIST_INSET))
}

/// Inset frame for the selected-definition detail pane.
pub(super) fn detail_frame(ui: &egui::Ui) -> egui::Frame {
    let dark = ui.visuals().dark_mode;
    let visuals = ui.visuals();
    egui::Frame::new()
        .fill(if dark {
            DARK_DETAIL_FILL
        } else {
            visuals.panel_fill
        })
        .stroke(egui::Stroke::new(FRAME_BORDER_WIDTH, if dark {
            DARK_PANE_BORDER
        } else {
            visuals.widgets.noninteractive.bg_stroke.color
        }))
        .inner_margin(egui::Margin::same(DETAIL_INSET))
}

/// Full-width selectable row with stable accessible name/selection metadata.
/// Its scoped visuals preserve theme contrast while matching the dark reference.
pub(super) fn selectable_row(
    ui: &mut egui::Ui,
    label: &str,
    selected: bool,
    focus_requested: bool,
) -> egui::Response {
    let mut style = ui.style().as_ref().clone();
    let visuals = &mut style.visuals;
    let focus_color = if visuals.dark_mode {
        DARK_ROW_FOCUS_STROKE
    } else {
        visuals.selection.stroke.color
    };
    if visuals.dark_mode {
        visuals.selection.bg_fill = DARK_ROW_SELECTED_FILL;
        visuals.selection.stroke = egui::Stroke::new(FRAME_BORDER_WIDTH, DARK_SELECTED_STROKE);
    }
    visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
    visuals.widgets.hovered.bg_fill = if visuals.dark_mode {
        DARK_ROW_HOVER_FILL
    } else {
        visuals.widgets.hovered.bg_fill
    };

    let response = ui.scope(|ui| {
        ui.set_style(style);
        ui.spacing_mut().button_padding = egui::vec2(
            SELECTABLE_ROW_PADDING_X,
            SELECTABLE_ROW_PADDING_Y,
        );
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(
                ui.available_width().max(MIN_PANE_WIDTH),
                SELECTABLE_ROW_HEIGHT,
            ),
            egui::Sense::click(),
        );
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::Button,
                response.enabled(),
                selected,
                label.to_owned(),
            )
        });
        let widget_visuals = ui.style().interact(&response);
        let (fill, foreground) = if selected {
            (
                ui.visuals().selection.bg_fill,
                ui.visuals().selection.stroke.color,
            )
        } else {
            (widget_visuals.bg_fill, widget_visuals.fg_stroke.color)
        };
        ui.painter().rect_filled(rect, widget_visuals.corner_radius, fill);
        if selected {
            ui.painter().rect_stroke(
                rect,
                widget_visuals.corner_radius,
                ui.visuals().selection.stroke,
                egui::StrokeKind::Inside,
            );
        } else if widget_visuals.bg_stroke.width > 0.0 {
            ui.painter().rect_stroke(
                rect,
                widget_visuals.corner_radius,
                widget_visuals.bg_stroke,
                egui::StrokeKind::Inside,
            );
        }
        let label_rect = rect.shrink2(egui::vec2(
            SELECTABLE_ROW_PADDING_X,
            SELECTABLE_ROW_PADDING_Y,
        ));
        let mut label_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(label_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        label_ui.add(
            egui::Label::new(
                egui::RichText::new(label)
                    .size(SELECTABLE_ROW_FONT_SIZE)
                    .color(foreground),
            )
            .halign(egui::Align::Min),
        );
        if focus_requested {
            response.request_focus();
        }
        if response.has_focus() {
            ui.painter().rect_stroke(
                response.rect.shrink(FOCUS_STROKE_INSET),
                ui.visuals().widgets.active.corner_radius,
                egui::Stroke::new(FOCUS_STROKE_WIDTH, focus_color),
                egui::StrokeKind::Inside,
            );
        }
        response
    });
    response.inner
}

/// Small read-only label/value line shared by the inspector detail sections.
pub(super) fn label_value_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal_top(|ui| {
        ui.add_sized(
            [DETAIL_LABEL_COLUMN_WIDTH, ui.spacing().interact_size.y],
            egui::Label::new(label).wrap(),
        );
        ui.label(value);
    });
}

/// A scoped segmented-tab control. The ordinary egui button keeps native focus
/// and keyboard activation behavior while exposing tab selection accessibly.
pub(super) fn tab_button(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let dark = ui.visuals().dark_mode;
    let text = ui.visuals().text_color();
    let (fill, foreground) = if dark {
        if selected {
            (DARK_TAB_SELECTED_FILL, egui::Color32::WHITE)
        } else {
            (DARK_TAB_UNSELECTED_FILL, text)
        }
    } else if selected {
        (ui.visuals().widgets.active.bg_fill, text)
    } else {
        (ui.visuals().widgets.inactive.bg_fill, text)
    };
    let response = ui.add_sized(
        [TAB_WIDTH, ui.spacing().interact_size.y.max(TAB_HEIGHT)],
        egui::Button::new(egui::RichText::new(label).color(foreground))
            .fill(fill)
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Button,
            response.enabled(),
            selected,
            label.to_owned(),
        )
    });
    response
}

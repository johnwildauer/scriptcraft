//! Scoped visual helpers shared by the Elements inspector and template selector.

use eframe::egui;

pub(super) const DEFAULT_INSPECTOR_SIZE: egui::Vec2 = egui::vec2(720.0, 520.0);
pub(super) const DEFAULT_SELECTOR_SIZE: egui::Vec2 = egui::vec2(640.0, 400.0);
pub(super) const VIEWPORT_MARGIN: f32 = 32.0;
pub(super) const LIST_COLUMN_MIN_WIDTH: f32 = 200.0;
pub(super) const LIST_COLUMN_MAX_WIDTH: f32 = 220.0;
pub(super) const PANE_GAP: f32 = 12.0;
pub(super) const PANE_INSET: i8 = 8;
pub(super) const SELECTABLE_ROW_HEIGHT: f32 = 30.0;

/// Inset frame for a live registry list, with the reference's dark well only in
/// dark appearances. Light appearances retain their normal theme surfaces.
pub(super) fn listbox_frame(ui: &egui::Ui) -> egui::Frame {
    let dark = ui.visuals().dark_mode;
    let visuals = ui.visuals();
    egui::Frame::new()
        .fill(if dark {
            egui::Color32::from_rgb(29, 29, 29)
        } else {
            visuals.extreme_bg_color
        })
        .stroke(egui::Stroke::new(
            1.0,
            if dark {
                egui::Color32::from_rgb(68, 68, 68)
            } else {
                visuals.widgets.noninteractive.bg_stroke.color
            },
        ))
        .inner_margin(egui::Margin::same(PANE_INSET))
}

/// Inset frame for the selected-definition detail pane.
pub(super) fn detail_frame(ui: &egui::Ui) -> egui::Frame {
    let dark = ui.visuals().dark_mode;
    let visuals = ui.visuals();
    egui::Frame::new()
        .fill(if dark {
            egui::Color32::from_rgb(48, 48, 48)
        } else {
            visuals.panel_fill
        })
        .stroke(egui::Stroke::new(
            1.0,
            if dark {
                egui::Color32::from_rgb(68, 68, 68)
            } else {
                visuals.widgets.noninteractive.bg_stroke.color
            },
        ))
        .inner_margin(egui::Margin::same(PANE_INSET))
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
        egui::Color32::from_rgb(198, 215, 255)
    } else {
        visuals.selection.stroke.color
    };
    if visuals.dark_mode {
        visuals.selection.bg_fill = egui::Color32::from_rgb(49, 95, 191);
        visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    }
    visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
    visuals.widgets.hovered.bg_fill = if visuals.dark_mode {
        egui::Color32::from_rgb(64, 64, 64)
    } else {
        visuals.widgets.hovered.bg_fill
    };

    let response = ui.scope(|ui| {
        ui.set_style(style);
        let response = ui.add_sized(
            [ui.available_width().max(1.0), SELECTABLE_ROW_HEIGHT],
            egui::Button::selectable(selected, label),
        );
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::Button,
                response.enabled(),
                selected,
                label.to_owned(),
            )
        });
        if focus_requested {
            response.request_focus();
        }
        if response.has_focus() {
            ui.painter().rect_stroke(
                response.rect.shrink(1.0),
                ui.visuals().widgets.active.corner_radius,
                egui::Stroke::new(2.0, focus_color),
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
            [132.0, ui.spacing().interact_size.y],
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
            (egui::Color32::from_rgb(92, 92, 92), egui::Color32::WHITE)
        } else {
            (egui::Color32::from_rgb(58, 58, 58), text)
        }
    } else if selected {
        (ui.visuals().widgets.active.bg_fill, text)
    } else {
        (ui.visuals().widgets.inactive.bg_fill, text)
    };
    let response = ui.add_sized(
        [88.0, ui.spacing().interact_size.y.max(28.0)],
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

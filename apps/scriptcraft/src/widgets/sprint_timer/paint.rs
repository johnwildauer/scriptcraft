use eframe::egui;
use std::time::Duration;

use super::geometry::{
    centered_active_display_panel_rect, mode_focus_rect, mode_switch_knob_center,
    mode_switch_track_rect, PopupGeometryLedger, SprintTimerPopupPlacement,
    POPUP_COUNTDOWN_EDITOR_HEIGHT, POPUP_COUNTDOWN_EDITOR_WIDTH, POPUP_MODE_SWITCH_KNOB_DIAMETER,
};
use super::interaction::ModeSelectorAccessibility;
use super::state::NotificationFeedback;
use super::SprintTimer;

pub(super) const POPUP_MODE_LABEL_FONT_SIZE: f32 = 13.0;
const POPUP_FOCUS_STROKE_WIDTH: f32 = 2.0;
const POPUP_FOCUS_COLOR_OVERRIDE: Option<egui::Color32> = None;
const POPUP_DISPLAY_BACKGROUND_OVERRIDE: Option<egui::Color32> = None;
const POPUP_DISPLAY_TEXT_OVERRIDE: Option<egui::Color32> = None;
pub(super) const POPUP_MODE_FOCUS_CORNER_RADIUS: u8 = 4;
const POPUP_MODE_SWITCH_TRACK_RADIUS: u8 = 11;
pub(super) const POPUP_COUNTDOWN_EDITOR_FONT_SIZE: f32 = 23.0;
const POPUP_DISPLAY_CORNER_RADIUS: u8 = 2;
const POPUP_RUNNING_READOUT_FONT_SIZE: f32 = 30.0;

fn resolve_popup_focus_color(
    visuals: &egui::Visuals,
    override_color: Option<egui::Color32>,
) -> egui::Color32 {
    override_color.unwrap_or(visuals.hyperlink_color)
}

fn resolve_popup_display_background(
    visuals: &egui::Visuals,
    override_color: Option<egui::Color32>,
) -> egui::Color32 {
    override_color.unwrap_or(visuals.widgets.inactive.bg_fill)
}

fn resolve_popup_display_text(
    visuals: &egui::Visuals,
    override_color: Option<egui::Color32>,
) -> egui::Color32 {
    override_color.unwrap_or_else(|| visuals.text_color())
}

fn popup_focus_stroke(visuals: &egui::Visuals) -> egui::Stroke {
    egui::Stroke::new(
        POPUP_FOCUS_STROKE_WIDTH,
        resolve_popup_focus_color(visuals, POPUP_FOCUS_COLOR_OVERRIDE),
    )
}

pub(super) fn popup_display_background(visuals: &egui::Visuals) -> egui::Color32 {
    resolve_popup_display_background(visuals, POPUP_DISPLAY_BACKGROUND_OVERRIDE)
}

pub(super) fn popup_display_text(visuals: &egui::Visuals) -> egui::Color32 {
    resolve_popup_display_text(visuals, POPUP_DISPLAY_TEXT_OVERRIDE)
}

pub(super) fn mode_focus_stroke(visuals: &egui::Visuals) -> egui::Stroke {
    popup_focus_stroke(visuals)
}

pub(super) fn paint_mode_switch(
    ui: &egui::Ui,
    response: &egui::Response,
    enabled: bool,
    checked: bool,
) {
    let visuals = ui.visuals();
    let track = mode_switch_track_rect(response.rect);
    let track_color = if !enabled {
        visuals.widgets.noninteractive.bg_fill
    } else if checked {
        visuals.selection.bg_fill
    } else {
        visuals.widgets.inactive.bg_fill
    };
    ui.painter().rect_filled(
        track,
        egui::CornerRadius::same(POPUP_MODE_SWITCH_TRACK_RADIUS),
        track_color,
    );

    let knob_color = if !enabled {
        visuals.widgets.noninteractive.fg_stroke.color
    } else if checked {
        egui::Color32::WHITE
    } else {
        visuals.widgets.inactive.fg_stroke.color
    };
    ui.painter().circle_filled(
        mode_switch_knob_center(track, checked),
        POPUP_MODE_SWITCH_KNOB_DIAMETER / 2.0,
        knob_color,
    );

    if response.has_focus() {
        ui.painter().rect_stroke(
            mode_focus_rect(response.rect),
            egui::CornerRadius::same(POPUP_MODE_FOCUS_CORNER_RADIUS),
            mode_focus_stroke(visuals),
            egui::StrokeKind::Inside,
        );
    }
}

pub(super) fn popup_feedback_text(timer: &SprintTimer) -> String {
    let mut messages = Vec::new();
    if let Some(completed) = timer.completed_message() {
        messages.push(completed);
    }
    if let Some(error) = timer.validation_error.as_ref() {
        messages.push(error.clone());
    }
    if let Some(feedback) = timer.notification_feedback.as_ref() {
        messages.push(
            match feedback {
                NotificationFeedback::Pending => {
                    "OS notification request pending; delivery is not confirmed."
                }
                NotificationFeedback::Accepted => {
                    "OS accepted the notification request; delivery is not confirmed."
                }
                NotificationFeedback::Unavailable => {
                    "OS notification unavailable; the in-app completion remains available."
                }
            }
            .to_owned(),
        );
    }
    messages.join(" · ")
}

pub(super) fn paint_popup_feedback(
    ui: &mut egui::Ui,
    feedback: egui::Rect,
    timer: &SprintTimer,
    feedback_text: &str,
) {
    let response = ui.interact(
        feedback,
        ui.id().with("sprint-timer-feedback"),
        egui::Sense::hover(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Label, true, feedback_text.to_owned())
    });
    let color = if timer.validation_error.is_some() {
        ui.visuals().error_fg_color
    } else {
        ui.visuals().weak_text_color()
    };
    ui.painter().with_clip_rect(feedback).text(
        egui::pos2(feedback.left() + 1.0, feedback.center().y),
        egui::Align2::LEFT_CENTER,
        feedback_text,
        egui::TextStyle::Small.resolve(ui.style()),
        color,
    );
}

pub(super) fn paint_noninteractive_stage_label(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    text: &str,
    accessible_name: &str,
    id_source: &str,
    font_id: egui::FontId,
    color: egui::Color32,
) {
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        font_id,
        color,
    );
    let accessibility_id = ui.make_persistent_id(("sprint-timer-stage-label", id_source));
    ui.ctx().accesskit_node_builder(accessibility_id, |node| {
        node.set_role(egui::accesskit::Role::Label);
        node.set_label(accessible_name.to_owned());
        node.set_bounds(accesskit_bounds(rect));
    });
}

fn accesskit_bounds(rect: egui::Rect) -> egui::accesskit::Rect {
    egui::accesskit::Rect {
        x0: rect.min.x.into(),
        y0: rect.min.y.into(),
        x1: rect.max.x.into(),
        y1: rect.max.y.into(),
    }
}

/// Paints a bounded, noninteractive readout. Deliberately avoids `interact`
/// and text widgets so active values have no pointer, keyboard, or edit route.
pub(super) fn paint_noninteractive_active_display_panel(
    ui: &egui::Ui,
    stage: egui::Rect,
    text: &str,
    accessible_name: &str,
    id_source: &str,
) -> egui::Rect {
    let panel = centered_active_display_panel_rect(stage);
    ui.painter().rect_filled(
        panel,
        egui::CornerRadius::same(POPUP_DISPLAY_CORNER_RADIUS),
        popup_display_background(ui.visuals()),
    );
    ui.painter().with_clip_rect(panel).text(
        panel.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::monospace(POPUP_RUNNING_READOUT_FONT_SIZE),
        popup_display_text(ui.visuals()),
    );
    let accessibility_id = ui.make_persistent_id(("sprint-timer-active-display", id_source));
    ui.ctx().accesskit_node_builder(accessibility_id, |node| {
        node.set_role(egui::accesskit::Role::Label);
        node.set_label(accessible_name.to_owned());
        node.set_bounds(accesskit_bounds(panel));
    });
    panel
}

#[derive(Debug, Clone)]
pub(super) struct SprintTimerPopupRenderSnapshot {
    pub(super) body: egui::Rect,
    pub(super) notch: egui::Rect,
    pub(super) popup: egui::Rect,
    pub(super) notch_tip: egui::Pos2,
    pub(super) notch_visible: bool,
    pub(super) toggle: egui::Rect,
    pub(super) display: egui::Rect,
    pub(super) feedback: egui::Rect,
    pub(super) primary_action: egui::Rect,
    pub(super) end_action: egui::Rect,
    pub(super) display_text: String,
    pub(super) display_accessible_name: String,
    pub(super) duration_edit_rect: Option<egui::Rect>,
    pub(super) duration_edit_id: Option<egui::Id>,
    pub(super) duration_accessible_name: Option<&'static str>,
    pub(super) duration_caption_rect: Option<egui::Rect>,
    pub(super) active_display_panel_rect: Option<egui::Rect>,
    pub(super) feedback_text: String,
    pub(super) primary_label: &'static str,
    pub(super) primary_id: egui::Id,
    pub(super) end_enabled: bool,
    pub(super) end_id: egui::Id,
    pub(super) primary_active: bool,
    pub(super) primary_has_focus: bool,
    pub(super) primary_clicked: bool,
    pub(super) end_active: bool,
    pub(super) end_has_focus: bool,
    pub(super) end_clicked: bool,
    pub(super) selector_accessibility: ModeSelectorAccessibility,
    pub(super) geometry: PopupGeometryLedger,
    pub(super) opening_progress: f32,
    pub(super) opening_scale: f32,
    pub(super) opening_opacity: f32,
    pub(super) layer_transform: egui::emath::TSTransform,
}

pub(super) fn apply_popup_content_style(ui: &mut egui::Ui) {
    let focus_stroke = popup_focus_stroke(ui.visuals());
    let visuals = &mut ui.style_mut().visuals;
    visuals.selection.stroke = focus_stroke;
    visuals.widgets.active.bg_stroke = focus_stroke;
}

pub(super) fn paint_popup_notch(
    ctx: &egui::Context,
    layer_id: egui::LayerId,
    placement: SprintTimerPopupPlacement,
    opacity: f32,
) {
    if !placement.notch_visible {
        return;
    }
    let mut painter = ctx.layer_painter(layer_id);
    painter.set_opacity(opacity);
    let left_base = placement.notch.left_top();
    let right_base = placement.notch.right_top();
    let tip = placement.notch_tip;
    let fill = ctx.style().visuals.window_fill;
    let border = ctx.style().visuals.window_stroke;
    let triangle = [left_base, right_base, tip];

    painter.add(egui::Shape::convex_polygon(
        triangle.map(|point| point + egui::vec2(0.0, 1.0)).to_vec(),
        egui::Color32::from_black_alpha(24),
        egui::Stroke::NONE,
    ));
    painter.add(egui::Shape::convex_polygon(
        triangle.to_vec(),
        fill,
        egui::Stroke::NONE,
    ));
    // Erase the body's straight bottom stroke across the join so the triangle
    // and rounded body read as one silhouette rather than two stacked shapes.
    painter.line_segment([left_base, right_base], egui::Stroke::new(2.0, fill));
    if border.width > 0.0 && border.color != egui::Color32::TRANSPARENT {
        let notch_stroke = egui::Stroke::new(border.width.max(1.0), border.color);
        painter.line_segment([left_base, tip], notch_stroke);
        painter.line_segment([tip, right_base], notch_stroke);
    }
}

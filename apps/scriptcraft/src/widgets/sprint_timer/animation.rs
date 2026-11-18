use eframe::egui;
use scriptcraft_commands::Command;
use std::time::Duration;

use super::geometry::{
    retained_popup_area_size, sprint_timer_popup_placement, SprintTimerPopupPlacement,
};
use super::interaction::show_popup_contents;
use super::paint::{paint_popup_notch, SprintTimerPopupRenderSnapshot};
use super::SprintTimer;

const POPUP_OPENING_DURATION: Duration = Duration::from_millis(140);
const POPUP_OPENING_REPAINT_INTERVAL: Duration = Duration::from_millis(16);
const POPUP_OPENING_START_SCALE: f32 = 0.2;
pub(super) const POPUP_OPENING_END_SCALE: f32 = 1.0;
const POPUP_CLOSING_DURATION: Duration = Duration::from_millis(120);
const POPUP_CLOSING_REPAINT_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Debug, Clone, Copy, PartialEq)]
struct PopupOpeningPresentation {
    progress: f32,
    scale: f32,
    opacity: f32,
    transform: egui::emath::TSTransform,
    repaint_after: Option<Duration>,
    active: bool,
}

fn popup_opening_progress(started_at: f64, now: f64) -> f32 {
    if !started_at.is_finite() || !now.is_finite() {
        return 1.0;
    }
    let elapsed = (now - started_at).max(0.0);
    let linear = (elapsed / POPUP_OPENING_DURATION.as_secs_f64()).clamp(0.0, 1.0) as f32;
    egui::emath::easing::cubic_out(linear).clamp(0.0, 1.0)
}

fn popup_opening_transform(center: egui::Pos2, scale: f32) -> egui::emath::TSTransform {
    let scale = if scale.is_finite() {
        scale.clamp(POPUP_OPENING_START_SCALE, POPUP_OPENING_END_SCALE)
    } else {
        POPUP_OPENING_END_SCALE
    };
    let translation = center.to_vec2() * (1.0 - scale);
    egui::emath::TSTransform::new(translation, scale)
}

fn popup_opening_presentation(
    started_at: Option<f64>,
    now: f64,
    icon_center: egui::Pos2,
) -> PopupOpeningPresentation {
    let Some(started_at) = started_at else {
        return PopupOpeningPresentation {
            progress: 1.0,
            scale: POPUP_OPENING_END_SCALE,
            opacity: 1.0,
            transform: egui::emath::TSTransform::IDENTITY,
            repaint_after: None,
            active: false,
        };
    };

    let progress = popup_opening_progress(started_at, now);
    let animated_scale = POPUP_OPENING_START_SCALE
        + (POPUP_OPENING_END_SCALE - POPUP_OPENING_START_SCALE) * progress;
    let deadline = started_at + POPUP_OPENING_DURATION.as_secs_f64();
    let active = started_at.is_finite() && now.is_finite() && deadline.is_finite() && now < deadline;
    let scale = if active {
        animated_scale
    } else {
        POPUP_OPENING_END_SCALE
    };
    let opacity = if active { progress } else { 1.0 };
    let repaint_after = if active {
        let remaining = (deadline - now).max(0.0);
        Some(
            Duration::from_secs_f64(remaining)
                .min(POPUP_OPENING_REPAINT_INTERVAL)
                .max(Duration::from_millis(1)),
        )
    } else {
        None
    };

    PopupOpeningPresentation {
        progress,
        scale,
        opacity,
        transform: if active {
            popup_opening_transform(icon_center, scale)
        } else {
            egui::emath::TSTransform::IDENTITY
        },
        repaint_after,
        active,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PopupClosingTransition {
    started_at: f64,
    start_scale: f32,
    start_opacity: f32,
    rest: SprintTimerPopupPlacement,
    icon_anchor: egui::Rect,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PopupClosingPresentation {
    progress: f32,
    scale: f32,
    opacity: f32,
    transform: egui::emath::TSTransform,
    repaint_after: Option<Duration>,
    active: bool,
}

fn popup_closing_presentation(
    closing: PopupClosingTransition,
    now: f64,
) -> PopupClosingPresentation {
    let elapsed = if closing.started_at.is_finite() && now.is_finite() {
        (now - closing.started_at).max(0.0)
    } else {
        POPUP_CLOSING_DURATION.as_secs_f64()
    };
    let linear = (elapsed / POPUP_CLOSING_DURATION.as_secs_f64()).clamp(0.0, 1.0) as f32;
    let progress = linear * linear * linear;
    let active = closing.started_at.is_finite()
        && now.is_finite()
        && now < closing.started_at + POPUP_CLOSING_DURATION.as_secs_f64();
    let scale = if active {
        closing.start_scale + (POPUP_OPENING_START_SCALE - closing.start_scale) * progress
    } else {
        POPUP_OPENING_START_SCALE
    };
    let opacity = if active {
        closing.start_opacity * (1.0 - progress)
    } else {
        0.0
    };
    let repaint_after = if active {
        let deadline = closing.started_at + POPUP_CLOSING_DURATION.as_secs_f64();
        Some(
            Duration::from_secs_f64((deadline - now).max(0.0))
                .min(POPUP_CLOSING_REPAINT_INTERVAL)
                .max(Duration::from_millis(1)),
        )
    } else {
        None
    };

    PopupClosingPresentation {
        progress,
        scale,
        opacity,
        transform: if active {
            popup_opening_transform(closing.icon_anchor.center(), scale)
        } else {
            egui::emath::TSTransform::IDENTITY
        },
        repaint_after,
        active,
    }
}

fn sprint_timer_popup_layer_id() -> egui::LayerId {
    egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("sprint-timer-popup"),
    )
}

impl SprintTimer {
    pub(crate) fn start_popup_opening(&mut self, egui_time: f64) {
        self.opening_started_at = Some(egui_time);
        self.closing = None;
        self.popup_rest_placement = None;
        self.close_completion_pending = false;
    }

    pub(crate) fn clear_popup_transition(&mut self, ctx: &egui::Context) {
        self.opening_started_at = None;
        self.closing = None;
        self.popup_rest_placement = None;
        self.close_completion_pending = false;
        self.just_opened = false;
        ctx.set_transform_layer(
            sprint_timer_popup_layer_id(),
            egui::emath::TSTransform::IDENTITY,
        );
    }

    pub(crate) fn request_popup_close(&mut self, egui_time: f64, viewport: egui::Rect) -> bool {
        if !self.popup_open || self.closing.is_some() {
            return false;
        }
        let rest = self.popup_rest_placement.unwrap_or_else(|| {
            sprint_timer_popup_placement(
                self.popup_anchor.is_positive().then_some(self.popup_anchor),
                viewport,
            )
        });
        let icon_anchor = if self.popup_anchor.is_positive() {
            self.popup_anchor
        } else {
            egui::Rect::from_center_size(rest.body.center(), egui::Vec2::ZERO)
        };
        let opening = popup_opening_presentation(
            self.opening_started_at,
            egui_time,
            icon_anchor.center(),
        );
        self.closing = Some(PopupClosingTransition {
            started_at: egui_time,
            start_scale: opening.scale,
            start_opacity: opening.opacity,
            rest,
            icon_anchor,
        });
        self.opening_started_at = None;
        self.close_completion_pending = false;
        true
    }

    pub(crate) fn take_popup_close_completion(&mut self) -> bool {
        std::mem::take(&mut self.close_completion_pending)
    }

    pub(crate) fn is_closing(&self) -> bool {
        self.closing.is_some()
    }
}

pub(super) fn show_closing_popup(
    ctx: &egui::Context,
    timer: &mut SprintTimer,
    now: Duration,
    viewport: egui::Rect,
    closing: PopupClosingTransition,
    egui_time: f64,
) -> (Vec<Command>, Option<SprintTimerPopupRenderSnapshot>) {
    let commands = Vec::new();
    let presentation = popup_closing_presentation(closing, egui_time);
    if !presentation.active {
        ctx.set_transform_layer(
            sprint_timer_popup_layer_id(),
            egui::emath::TSTransform::IDENTITY,
        );
        timer.popup_rect = egui::Rect::NOTHING;
        timer.close_completion_pending = true;
        return (commands, None);
    }
    ctx.set_transform_layer(sprint_timer_popup_layer_id(), presentation.transform);
    if let Some(delay) = presentation.repaint_after {
        ctx.request_repaint_after(delay);
    }
    let mut popup_commands = Vec::new();
    let output = egui::Area::new(egui::Id::new("sprint-timer-popup"))
        .order(egui::Order::Foreground)
        .fade_in(false)
        .fixed_pos(closing.rest.body.min)
        .show(ctx, |ui| {
            ui.set_opacity(presentation.opacity);
            ui.add_enabled_ui(false, |ui| {
                let frame = egui::Frame::popup(ui.style())
                    .inner_margin(egui::Margin::ZERO)
                    .show(ui, |ui| show_popup_contents(ui, timer, now, &mut popup_commands));
                let mut snapshot = frame.inner;
                snapshot.geometry.frame_outer = frame.response.rect;
                snapshot
            })
            .inner
        });
    paint_popup_notch(ctx, output.response.layer_id, closing.rest, presentation.opacity);
    timer.popup_rect = (presentation.transform * closing.rest.popup).intersect(viewport);
    let mut snapshot = output.inner;
    snapshot.geometry.area_response = output.response.rect;
    snapshot.geometry.next_pass_retained_size = retained_popup_area_size(ctx);
    snapshot.notch = closing.rest.notch;
    snapshot.popup = closing.rest.popup;
    snapshot.notch_tip = closing.rest.notch_tip;
    snapshot.notch_visible = closing.rest.notch_visible;
    snapshot.opening_progress = presentation.progress;
    snapshot.opening_scale = presentation.scale;
    snapshot.opening_opacity = presentation.opacity;
    snapshot.layer_transform = presentation.transform;
    (commands, Some(snapshot))
}

pub(super) fn show_opening_popup(
    ctx: &egui::Context,
    timer: &mut SprintTimer,
    now: Duration,
    viewport: egui::Rect,
    egui_time: f64,
) -> (Vec<Command>, Option<SprintTimerPopupRenderSnapshot>) {
    let mut commands = Vec::new();
    let placement = sprint_timer_popup_placement(
        timer.popup_anchor.is_positive().then_some(timer.popup_anchor),
        viewport,
    );
    let icon_center = if timer.popup_anchor.is_positive() {
        timer.popup_anchor.center()
    } else {
        placement.body.center()
    };
    timer.popup_rest_placement = Some(placement);
    let opening = popup_opening_presentation(timer.opening_started_at, egui_time, icon_center);
    if !opening.active {
        timer.opening_started_at = None;
    }
    let area = egui::Area::new(egui::Id::new("sprint-timer-popup"))
        .order(egui::Order::Foreground)
        .fade_in(false);
    let layer_id = area.layer();
    // This persistent framework transform applies to both layer painting and
    // inverse-transformed input, keeping the popup's visuals and hit targets aligned.
    ctx.set_transform_layer(layer_id, opening.transform);
    if let Some(delay) = opening.repaint_after {
        ctx.request_repaint_after(delay);
    }
    let mut popup_commands = Vec::new();
    let output = area
        .fixed_pos(placement.body.min)
        .show(ctx, |ui| {
            ui.set_opacity(opening.opacity);
            ui.add_enabled_ui(!opening.active, |ui| {
                let frame = egui::Frame::popup(ui.style())
                    .inner_margin(egui::Margin::ZERO)
                    .show(ui, |ui| show_popup_contents(ui, timer, now, &mut popup_commands));
                let mut snapshot = frame.inner;
                snapshot.geometry.frame_outer = frame.response.rect;
                snapshot
            })
            .inner
        });
    if !opening.active {
        commands.extend(popup_commands);
    }
    paint_popup_notch(ctx, output.response.layer_id, placement, opening.opacity);
    timer.popup_rect = (opening.transform * placement.popup).intersect(viewport);
    let mut snapshot = output.inner;
    snapshot.geometry.area_response = output.response.rect;
    snapshot.geometry.next_pass_retained_size = retained_popup_area_size(ctx);
    snapshot.notch = placement.notch;
    snapshot.popup = placement.popup;
    snapshot.notch_tip = placement.notch_tip;
    snapshot.notch_visible = placement.notch_visible;
    snapshot.opening_progress = opening.progress;
    snapshot.opening_scale = opening.scale;
    snapshot.opening_opacity = opening.opacity;
    snapshot.layer_transform = opening.transform;
    (commands, Some(snapshot))
}

use eframe::egui::{self, Key, WidgetInfo, WidgetType};
use scriptcraft_commands::{Command, SprintTimerMode as CommandTimerMode};
use std::process::Command as ProcessCommand;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

const DEFAULT_MINUTES: &str = "25";
const MIN_MINUTES: u16 = 1;
const MAX_MINUTES: u16 = 1_440;
const MIN_REPAINT_DELAY: Duration = Duration::from_millis(10);
const MAX_REPAINT_DELAY: Duration = Duration::from_secs(1);
const POPUP_OPENING_DURATION: Duration = Duration::from_millis(140);
const POPUP_OPENING_REPAINT_INTERVAL: Duration = Duration::from_millis(16);
const POPUP_OPENING_START_SCALE: f32 = 0.2;
const POPUP_OPENING_END_SCALE: f32 = 1.0;
const POPUP_CLOSING_DURATION: Duration = Duration::from_millis(120);
const POPUP_CLOSING_REPAINT_INTERVAL: Duration = Duration::from_millis(16);

// Popup-local logical-point presentation tokens. Keep these independent of
// global egui spacing and use the planner below for every popup presentation.
const POPUP_BODY_WIDTH: f32 = 300.0;
const POPUP_BODY_HEIGHT: f32 = 165.0;
const POPUP_BODY_INSET: f32 = 16.0;
const POPUP_CORNER_RADIUS: f32 = 8.0;
const POPUP_TOGGLE_TOP: f32 = 18.0;
const POPUP_TOGGLE_HEIGHT: f32 = 22.0;
const POPUP_DISPLAY_TOP: f32 = 54.0;
const POPUP_DISPLAY_BOTTOM: f32 = 116.0;
const POPUP_FEEDBACK_TOP: f32 = 116.0;
const POPUP_FEEDBACK_HEIGHT: f32 = 12.0;
const POPUP_ACTION_TOP: f32 = 132.0;
const POPUP_ACTION_HEIGHT: f32 = 20.0;
const POPUP_ACTION_WIDTH: f32 = 98.0;
const POPUP_ACTION_GAP: f32 = 12.0;
const POPUP_NOTCH_HEIGHT: f32 = 20.0;
const POPUP_NOTCH_WIDTH: f32 = 28.0;
const POPUP_ICON_CLEARANCE: f32 = 2.0;
const POPUP_MODE_LABEL_WIDTH: f32 = 80.0;
const POPUP_MODE_LABEL_HEIGHT: f32 = 22.0;
const POPUP_MODE_LABEL_FONT_SIZE: f32 = 13.0;
const POPUP_MODE_FOCUS_INSET: f32 = 1.0;
const POPUP_FOCUS_STROKE_WIDTH: f32 = 2.0;
const POPUP_FOCUS_COLOR_OVERRIDE: Option<egui::Color32> = None;
const POPUP_DISPLAY_BACKGROUND_OVERRIDE: Option<egui::Color32> = None;
const POPUP_DISPLAY_TEXT_OVERRIDE: Option<egui::Color32> = None;
const POPUP_MODE_FOCUS_CORNER_RADIUS: u8 = 4;
const POPUP_MODE_SWITCH_TRACK_WIDTH: f32 = 38.0;
const POPUP_MODE_SWITCH_TRACK_HEIGHT: f32 = 22.0;
const POPUP_MODE_SWITCH_TRACK_RADIUS: u8 = 11;
const POPUP_MODE_SWITCH_KNOB_DIAMETER: f32 = 18.0;
const POPUP_MODE_SWITCH_KNOB_END_INSET: f32 = 11.0;
const POPUP_COUNTDOWN_EDITOR_WIDTH: f32 = 128.0;
const POPUP_COUNTDOWN_EDITOR_HEIGHT: f32 = 38.0;
const POPUP_COUNTDOWN_EDITOR_FONT_SIZE: f32 = 23.0;
const POPUP_ACTIVE_DISPLAY_WIDTH: f32 = 175.0;
const POPUP_ACTIVE_DISPLAY_HEIGHT: f32 = 38.0;
const POPUP_DISPLAY_CORNER_RADIUS: u8 = 2;
const POPUP_RUNNING_READOUT_FONT_SIZE: f32 = 30.0;
const MODE_SELECTOR_GAP: f32 = 8.0;
const MODE_SWITCH_ACCESSIBLE_NAME: &str = "Stopwatch mode";

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
struct PopupClosingTransition {
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

#[derive(Debug, Clone, Copy, PartialEq)]
struct SprintTimerPopupLayout {
    body: egui::Rect,
    toggle: egui::Rect,
    display: egui::Rect,
    feedback: egui::Rect,
    primary_action: egui::Rect,
    end_action: egui::Rect,
}

/// Returns the fixed, popup-local layout in logical points. Placement relative
/// to a viewport or icon belongs to the outer presentation boundary.
fn sprint_timer_popup_layout() -> SprintTimerPopupLayout {
    let body = egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::vec2(POPUP_BODY_WIDTH, POPUP_BODY_HEIGHT),
    );
    let content_width = POPUP_BODY_WIDTH - 2.0 * POPUP_BODY_INSET;
    let content_left = POPUP_BODY_INSET;
    let centered_rect = |top: f32, width: f32, height: f32| {
        egui::Rect::from_min_size(
            egui::pos2((POPUP_BODY_WIDTH - width) / 2.0, top),
            egui::vec2(width, height),
        )
    };
    let pair_width = 2.0 * POPUP_ACTION_WIDTH + POPUP_ACTION_GAP;
    let pair_left = (POPUP_BODY_WIDTH - pair_width) / 2.0;

    SprintTimerPopupLayout {
        body,
        toggle: egui::Rect::from_min_size(
            egui::pos2(content_left, POPUP_TOGGLE_TOP),
            egui::vec2(content_width, POPUP_TOGGLE_HEIGHT),
        ),
        display: centered_rect(
            POPUP_DISPLAY_TOP,
            content_width,
            POPUP_DISPLAY_BOTTOM - POPUP_DISPLAY_TOP,
        ),
        feedback: egui::Rect::from_min_size(
            egui::pos2(content_left, POPUP_FEEDBACK_TOP),
            egui::vec2(content_width, POPUP_FEEDBACK_HEIGHT),
        ),
        primary_action: egui::Rect::from_min_size(
            egui::pos2(pair_left, POPUP_ACTION_TOP),
            egui::vec2(POPUP_ACTION_WIDTH, POPUP_ACTION_HEIGHT),
        ),
        end_action: egui::Rect::from_min_size(
            egui::pos2(pair_left + POPUP_ACTION_WIDTH + POPUP_ACTION_GAP, POPUP_ACTION_TOP),
            egui::vec2(POPUP_ACTION_WIDTH, POPUP_ACTION_HEIGHT),
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SprintTimerPopupPlacement {
    body: egui::Rect,
    notch: egui::Rect,
    popup: egui::Rect,
    notch_tip: egui::Pos2,
    notch_visible: bool,
}

/// Plans the fixed-size popup in the coordinate space of its own viewport.
/// When there is room, the notch stops just short of the icon response and the
/// body remains directly above it. Constrained and missing-anchor cases stay
/// origin-bounded and use no misleading pointer.
fn sprint_timer_popup_placement(
    anchor: Option<egui::Rect>,
    viewport: egui::Rect,
) -> SprintTimerPopupPlacement {
    let viewport = if viewport.is_positive()
        && viewport.min.x.is_finite()
        && viewport.min.y.is_finite()
        && viewport.max.x.is_finite()
        && viewport.max.y.is_finite()
    {
        viewport
    } else {
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1.0, 1.0))
    };
    let width = POPUP_BODY_WIDTH;
    let height = POPUP_BODY_HEIGHT;
    let has_anchor = anchor.is_some_and(|rect| {
        rect.is_positive()
            && rect.min.x.is_finite()
            && rect.min.y.is_finite()
            && rect.max.x.is_finite()
            && rect.max.y.is_finite()
    });
    let anchor = anchor.filter(|_| has_anchor);

    let desired_center_x = anchor.map_or(viewport.center().x, |rect| rect.center().x);
    let max_left = (viewport.right() - width).max(viewport.left());
    let left = (desired_center_x - width / 2.0).clamp(viewport.left(), max_left);
    let body_top_limit = (viewport.bottom() - height).max(viewport.top());
    let anchor_tip_y = anchor.map(|rect| rect.top() - POPUP_ICON_CLEARANCE);
    let above_body_top = anchor_tip_y.map(|tip_y| tip_y - height);
    let above_notch_top = anchor_tip_y.map(|tip_y| tip_y - POPUP_NOTCH_HEIGHT - height);
    let above_body_fits = above_body_top.is_some_and(|top| {
        top >= viewport.top()
            && anchor_tip_y.is_some_and(|tip_y| tip_y <= viewport.bottom())
            && viewport.height() >= height
    });
    let notch_visible = above_body_fits
        && above_notch_top.is_some_and(|top| top >= viewport.top())
        && viewport.width() >= width
        && viewport.height() >= height + POPUP_NOTCH_HEIGHT + POPUP_ICON_CLEARANCE;
    let top = if notch_visible {
        above_notch_top.unwrap_or(viewport.top())
    } else if above_body_fits {
        above_body_top.unwrap_or(viewport.top())
    } else if !has_anchor && viewport.height() >= height {
        (viewport.center().y - height / 2.0).clamp(viewport.top(), body_top_limit)
    } else if let Some(anchor) = anchor.filter(|_| viewport.height() >= height) {
        (anchor.bottom() + POPUP_ICON_CLEARANCE).clamp(viewport.top(), body_top_limit)
    } else {
        // In short viewports keep the fixed body anchored at the visible content
        // origin rather than placing its origin offscreen or chasing the icon.
        viewport.top().clamp(viewport.top(), body_top_limit)
    };
    let body = egui::Rect::from_min_size(egui::pos2(left, top), egui::vec2(width, height));

    if !notch_visible {
        return SprintTimerPopupPlacement {
            body,
            notch: egui::Rect::NOTHING,
            popup: body,
            notch_tip: egui::Pos2::ZERO,
            notch_visible: false,
        };
    }

    let icon_top = anchor.map_or(viewport.top(), |rect| rect.top());
    let tip_y = icon_top - POPUP_ICON_CLEARANCE;
    let safe_half_width = POPUP_NOTCH_WIDTH / 2.0;
    let safe_left = body.left() + POPUP_CORNER_RADIUS + safe_half_width;
    let safe_right = body.right() - POPUP_CORNER_RADIUS - safe_half_width;
    let target_x = anchor.map_or(body.center().x, |rect| rect.center().x);
    let tip_x = target_x.clamp(safe_left, safe_right);
    let notch = egui::Rect::from_min_max(
        egui::pos2(tip_x - safe_half_width, body.bottom()),
        egui::pos2(tip_x + safe_half_width, tip_y),
    );
    let popup = body.union(notch);

    SprintTimerPopupPlacement {
        body,
        notch,
        popup,
        notch_tip: egui::pos2(tip_x, tip_y),
        notch_visible: true,
    }
}

/// Transient host-window geometry observed through the owning egui viewport.
/// Each component stays optional so unavailable observations never imply a
/// fabricated move or resize.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct SprintTimerHostGeometry {
    outer_left: Option<f32>,
    outer_top: Option<f32>,
    inner_width: Option<f32>,
    inner_height: Option<f32>,
}

impl SprintTimerHostGeometry {
    pub(super) fn from_context(ctx: &egui::Context) -> Self {
        ctx.input(|input| {
            let viewport = input.viewport();
            Self::from_rects(viewport.inner_rect, viewport.outer_rect)
        })
    }

    fn from_rects(inner: Option<egui::Rect>, outer: Option<egui::Rect>) -> Self {
        let finite = |value: f32| value.is_finite().then_some(value);
        Self {
            outer_left: outer.and_then(|rect| finite(rect.left())),
            outer_top: outer.and_then(|rect| finite(rect.top())),
            inner_width: inner.and_then(|rect| finite(rect.width())),
            inner_height: inner.and_then(|rect| finite(rect.height())),
        }
    }

    pub(super) fn changed_from(self, captured: Self) -> bool {
        fn changed(captured: Option<f32>, current: Option<f32>) -> bool {
            captured
                .zip(current)
                .is_some_and(|(captured, current)| captured != current)
        }

        changed(captured.outer_left, self.outer_left)
            || changed(captured.outer_top, self.outer_top)
            || changed(captured.inner_width, self.inner_width)
            || changed(captured.inner_height, self.inner_height)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ModeSelectorGeometry {
    toggle: egui::Rect,
    countdown: egui::Rect,
    switch: egui::Rect,
    stopwatch: egui::Rect,
}

impl Default for ModeSelectorGeometry {
    fn default() -> Self {
        Self {
            toggle: egui::Rect::NOTHING,
            countdown: egui::Rect::NOTHING,
            switch: egui::Rect::NOTHING,
            stopwatch: egui::Rect::NOTHING,
        }
    }
}

fn mode_selector_geometry(toggle: egui::Rect) -> ModeSelectorGeometry {
    let group_width = POPUP_MODE_LABEL_WIDTH * 2.0
        + POPUP_MODE_SWITCH_TRACK_WIDTH
        + MODE_SELECTOR_GAP * 2.0;
    let group_left = toggle.center().x - group_width / 2.0;
    let label_top = toggle.center().y - POPUP_MODE_LABEL_HEIGHT / 2.0;
    let switch_left = group_left + POPUP_MODE_LABEL_WIDTH + MODE_SELECTOR_GAP;

    ModeSelectorGeometry {
        toggle,
        countdown: egui::Rect::from_min_size(
            egui::pos2(group_left, label_top),
            egui::vec2(POPUP_MODE_LABEL_WIDTH, POPUP_MODE_LABEL_HEIGHT),
        ),
        switch: egui::Rect::from_min_size(
            egui::pos2(switch_left, label_top),
            egui::vec2(POPUP_MODE_SWITCH_TRACK_WIDTH, POPUP_MODE_SWITCH_TRACK_HEIGHT),
        ),
        stopwatch: egui::Rect::from_min_size(
            egui::pos2(
                switch_left + POPUP_MODE_SWITCH_TRACK_WIDTH + MODE_SELECTOR_GAP,
                label_top,
            ),
            egui::vec2(POPUP_MODE_LABEL_WIDTH, POPUP_MODE_LABEL_HEIGHT),
        ),
    }
}

fn mode_switch_track_rect(response_rect: egui::Rect) -> egui::Rect {
    egui::Rect::from_center_size(
        response_rect.center(),
        egui::vec2(
            POPUP_MODE_SWITCH_TRACK_WIDTH,
            POPUP_MODE_SWITCH_TRACK_HEIGHT,
        ),
    )
}

fn mode_switch_knob_center(track: egui::Rect, checked: bool) -> egui::Pos2 {
    let x = if checked {
        track.right() - POPUP_MODE_SWITCH_KNOB_END_INSET
    } else {
        track.left() + POPUP_MODE_SWITCH_KNOB_END_INSET
    };
    egui::pos2(x, track.center().y)
}

fn mode_focus_rect(rect: egui::Rect) -> egui::Rect {
    rect.shrink(POPUP_MODE_FOCUS_INSET)
}

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

fn popup_display_background(visuals: &egui::Visuals) -> egui::Color32 {
    resolve_popup_display_background(visuals, POPUP_DISPLAY_BACKGROUND_OVERRIDE)
}

fn popup_display_text(visuals: &egui::Visuals) -> egui::Color32 {
    resolve_popup_display_text(visuals, POPUP_DISPLAY_TEXT_OVERRIDE)
}

fn mode_focus_stroke(visuals: &egui::Visuals) -> egui::Stroke {
    popup_focus_stroke(visuals)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ModeSelectorAccessibility {
    countdown_name: &'static str,
    stopwatch_name: &'static str,
    switch_name: &'static str,
    countdown_selected: bool,
    stopwatch_selected: bool,
    switch_checked: bool,
    enabled: bool,
}

impl ModeSelectorAccessibility {
    fn for_mode(mode: TimerMode, enabled: bool) -> Self {
        let stopwatch_selected = mode == TimerMode::Stopwatch;
        Self {
            countdown_name: "Countdown",
            stopwatch_name: "Stopwatch",
            switch_name: MODE_SWITCH_ACCESSIBLE_NAME,
            countdown_selected: !stopwatch_selected,
            stopwatch_selected,
            switch_checked: stopwatch_selected,
            enabled,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct ModeSelectorRender {
    activation: Option<TimerMode>,
    geometry: ModeSelectorGeometry,
    focused: [bool; 3],
    accessibility: ModeSelectorAccessibility,
}

/// Paints one fixed-width, centered labels-plus-switch selector in a WO-001
/// toggle-height slot and returns at most one intentional mode activation.
fn show_mode_selector(
    ui: &mut egui::Ui,
    toggle_rect: egui::Rect,
    mode: TimerMode,
    enabled: bool,
) -> ModeSelectorRender {
    let mut rendered = ModeSelectorRender {
        accessibility: ModeSelectorAccessibility::for_mode(mode, enabled),
        ..ModeSelectorRender::default()
    };
    let geometry = mode_selector_geometry(toggle_rect);
    rendered.geometry = geometry;

    let countdown = mode_label_response(
        ui,
        geometry.countdown,
        rendered.accessibility.countdown_name,
        rendered.accessibility.countdown_selected,
        enabled,
    );
    let switch = ui
        .interact(
            geometry.switch,
            ui.id().with("sprint-timer-mode-switch"),
            if enabled {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            },
        )
        .on_hover_cursor(if enabled {
            egui::CursorIcon::PointingHand
        } else {
            egui::CursorIcon::Default
        });
    switch.widget_info(|| {
        WidgetInfo::selected(
            WidgetType::Checkbox,
            enabled,
            rendered.accessibility.switch_checked,
            rendered.accessibility.switch_name.to_owned(),
        )
    });
    let stopwatch = mode_label_response(
        ui,
        geometry.stopwatch,
        rendered.accessibility.stopwatch_name,
        rendered.accessibility.stopwatch_selected,
        enabled,
    );

    paint_mode_switch(ui, &switch, enabled, mode == TimerMode::Stopwatch);
    rendered.focused = [countdown.has_focus(), switch.has_focus(), stopwatch.has_focus()];

    let key_activation = |response: &egui::Response| {
        response.has_focus()
            && ui.input(|input| input.key_pressed(Key::Enter) || input.key_pressed(Key::Space))
    };
    if enabled {
        if countdown.clicked() || key_activation(&countdown) {
            rendered.activation = Some(TimerMode::Countdown);
        } else if switch.clicked() || key_activation(&switch) {
            rendered.activation = Some(match mode {
                TimerMode::Countdown => TimerMode::Stopwatch,
                TimerMode::Stopwatch => TimerMode::Countdown,
            });
        } else if stopwatch.clicked() || key_activation(&stopwatch) {
            rendered.activation = Some(TimerMode::Stopwatch);
        }
    }

    rendered
}

fn mode_label_response(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    label: &str,
    selected: bool,
    enabled: bool,
) -> egui::Response {
    let response = ui
        .interact(
            rect,
            ui.id().with(("sprint-timer-mode-label", label)),
            if enabled {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            },
        )
        .on_hover_cursor(if enabled {
            egui::CursorIcon::PointingHand
        } else {
            egui::CursorIcon::Default
        });
    response.widget_info(|| {
        WidgetInfo::selected(
            WidgetType::RadioButton,
            enabled,
            selected,
            label.to_owned(),
        )
    });

    let visuals = ui.visuals();
    let color = if !enabled {
        visuals.widgets.noninteractive.text_color()
    } else if selected {
        visuals.text_color()
    } else {
        visuals.weak_text_color()
    };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(POPUP_MODE_LABEL_FONT_SIZE),
        color,
    );
    if response.has_focus() {
        ui.painter().rect_stroke(
            mode_focus_rect(rect),
            egui::CornerRadius::same(POPUP_MODE_FOCUS_CORNER_RADIUS),
            mode_focus_stroke(visuals),
            egui::StrokeKind::Inside,
        );
    }
    response
}

fn paint_mode_switch(ui: &egui::Ui, response: &egui::Response, enabled: bool, checked: bool) {
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

fn mode_activation_commands(activation: Option<TimerMode>) -> Vec<Command> {
    activation
        .map(|mode| {
            vec![Command::ToolsSprintTimerSetMode {
                mode: match mode {
                    TimerMode::Countdown => CommandTimerMode::Countdown,
                    TimerMode::Stopwatch => CommandTimerMode::Stopwatch,
                },
            }]
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum TimerMode {
    #[default]
    Countdown,
    Stopwatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TimerState {
    Idle,
    CountdownRunning {
        deadline: Duration,
        minutes: u16,
        input: String,
        generation: u64,
    },
    CountdownPaused {
        remaining: Duration,
        minutes: u16,
        input: String,
        generation: u64,
    },
    CountdownCompleted {
        minutes: u16,
        generation: u64,
        notification_attempted: bool,
    },
    StopwatchRunning {
        started_at: Duration,
        accumulated: Duration,
    },
    StopwatchPaused {
        elapsed: Duration,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CompletionNotification {
    pub(super) generation: u64,
    pub(super) minutes: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) enum NotificationResult {
    #[default]
    Pending,
    Accepted,
    Unavailable,
}

pub(super) trait NotificationSink {
    fn request(&mut self, notice: CompletionNotification) -> NotificationResult;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum NotificationFeedback {
    Pending,
    Accepted,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct SprintTimer {
    pub(super) mode: TimerMode,
    state: TimerState,
    pub(super) minutes_input: String,
    pub(super) validation_error: Option<String>,
    pub(super) notification_feedback: Option<NotificationFeedback>,
    next_generation: u64,
    pub(super) popup_open: bool,
    pub(super) popup_rect: egui::Rect,
    pub(super) popup_anchor: egui::Rect,
    pub(super) just_opened: bool,
    opening_started_at: Option<f64>,
    closing: Option<PopupClosingTransition>,
    popup_rest_placement: Option<SprintTimerPopupPlacement>,
    close_completion_pending: bool,
}

impl Default for SprintTimer {
    fn default() -> Self {
        Self {
            mode: TimerMode::Countdown,
            state: TimerState::Idle,
            minutes_input: DEFAULT_MINUTES.to_owned(),
            validation_error: None,
            notification_feedback: None,
            next_generation: 0,
            popup_open: false,
            popup_rect: egui::Rect::NOTHING,
            popup_anchor: egui::Rect::NOTHING,
            just_opened: false,
            opening_started_at: None,
            closing: None,
            popup_rest_placement: None,
            close_completion_pending: false,
        }
    }
}

impl SprintTimer {
    pub(super) fn start_popup_opening(&mut self, egui_time: f64) {
        self.opening_started_at = Some(egui_time);
        self.closing = None;
        self.popup_rest_placement = None;
        self.close_completion_pending = false;
    }

    pub(super) fn clear_popup_transition(&mut self, ctx: &egui::Context) {
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

    pub(super) fn request_popup_close(&mut self, egui_time: f64, viewport: egui::Rect) -> bool {
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

    pub(super) fn take_popup_close_completion(&mut self) -> bool {
        std::mem::take(&mut self.close_completion_pending)
    }

    pub(super) fn is_closing(&self) -> bool {
        self.closing.is_some()
    }

    pub(super) fn is_idle(&self) -> bool {
        matches!(self.state, TimerState::Idle)
    }

    pub(super) fn is_running_or_paused(&self) -> bool {
        matches!(
            self.state,
            TimerState::CountdownRunning { .. }
                | TimerState::CountdownPaused { .. }
                | TimerState::StopwatchRunning { .. }
                | TimerState::StopwatchPaused { .. }
        )
    }

    pub(super) fn is_completed(&self) -> bool {
        matches!(self.state, TimerState::CountdownCompleted { .. })
    }

    pub(super) fn mode_switch_enabled(&self) -> bool {
        self.is_idle()
    }

    pub(super) fn set_minutes_input(&mut self, value: String) {
        if self.mode == TimerMode::Countdown && self.is_idle() {
            self.minutes_input = value.chars().filter(char::is_ascii_digit).collect();
            self.validation_error = None;
        }
    }

    pub(super) fn remaining(&self, now: Duration) -> Option<Duration> {
        match &self.state {
            TimerState::CountdownRunning { deadline, .. } => Some(deadline.saturating_sub(now)),
            TimerState::CountdownPaused { remaining, .. } => Some(*remaining),
            _ => None,
        }
    }

    pub(super) fn elapsed(&self, now: Duration) -> Option<Duration> {
        match self.state {
            TimerState::StopwatchRunning {
                started_at,
                accumulated,
            } => Some(accumulated.saturating_add(now.saturating_sub(started_at))),
            TimerState::StopwatchPaused { elapsed } => Some(elapsed),
            _ => None,
        }
    }

    pub(super) fn countdown_display(&self, now: Duration) -> String {
        let remaining = self.remaining(now).unwrap_or_else(|| match self.state {
            TimerState::CountdownCompleted { .. } => Duration::ZERO,
            _ => self
                .minutes_input
                .parse::<u16>()
                .ok()
                .filter(|minutes| (MIN_MINUTES..=MAX_MINUTES).contains(minutes))
                .map_or(Duration::from_secs(25 * 60), |minutes| {
                    Duration::from_secs(u64::from(minutes) * 60)
                }),
        });
        let seconds = ceil_seconds(remaining);
        format!("{:02}:{:02}", seconds / 60, seconds % 60)
    }

    pub(super) fn stopwatch_display(&self, now: Duration) -> String {
        let seconds = self.elapsed(now).unwrap_or(Duration::ZERO).as_secs();
        format!(
            "{:02}:{:02}:{:02}",
            seconds / 3_600,
            seconds / 60 % 60,
            seconds % 60
        )
    }

    pub(super) fn completed_message(&self) -> Option<String> {
        match self.state {
            TimerState::CountdownCompleted { minutes: 1, .. } => {
                Some("Great Work! You completed a 1 Minute Sprint".to_owned())
            }
            TimerState::CountdownCompleted { minutes, .. } => Some(format!(
                "Great Work! You completed a {minutes} Minutes Sprint"
            )),
            _ => None,
        }
    }

    pub(super) fn end_enabled(&self) -> bool {
        self.is_running_or_paused()
    }

    pub(super) fn primary_label(&self) -> &'static str {
        match self.state {
            TimerState::Idle => match self.mode {
                TimerMode::Countdown => "Start Sprint",
                TimerMode::Stopwatch => "Start Sprint",
            },
            TimerState::CountdownRunning { .. } | TimerState::StopwatchRunning { .. } => "Pause",
            TimerState::CountdownPaused { .. } | TimerState::StopwatchPaused { .. } => "Resume",
            TimerState::CountdownCompleted { .. } => "Reset",
        }
    }

    pub(super) fn primary_command(&self) -> Command {
        match self.state {
            TimerState::Idle => match self.mode {
                TimerMode::Countdown => Command::ToolsSprintTimerStart,
                TimerMode::Stopwatch => Command::ToolsSprintTimerStart,
            },
            TimerState::CountdownRunning { .. } | TimerState::StopwatchRunning { .. } => {
                Command::ToolsSprintTimerPause
            }
            TimerState::CountdownPaused { .. } | TimerState::StopwatchPaused { .. } => {
                Command::ToolsSprintTimerResume
            }
            TimerState::CountdownCompleted { .. } => Command::ToolsSprintTimerReset,
        }
    }

    pub(super) fn reconcile(&mut self, now: Duration) -> Option<CompletionNotification> {
        let completed = match &self.state {
            TimerState::CountdownRunning {
                deadline,
                minutes,
                generation,
                ..
            } if now >= *deadline => Some((*minutes, *generation)),
            _ => None,
        };
        completed.map(|(minutes, generation)| {
            self.state = TimerState::CountdownCompleted {
                minutes,
                generation,
                notification_attempted: true,
            };
            CompletionNotification {
                generation,
                minutes,
            }
        })
    }

    pub(super) fn apply_notification_result(
        &mut self,
        generation: u64,
        result: NotificationResult,
    ) -> bool {
        if !matches!(
            self.state,
            TimerState::CountdownCompleted {
                generation: current,
                notification_attempted: true,
                ..
            } if current == generation
        ) {
            return false;
        }
        self.notification_feedback = Some(match result {
            NotificationResult::Pending => NotificationFeedback::Pending,
            NotificationResult::Accepted => NotificationFeedback::Accepted,
            NotificationResult::Unavailable => NotificationFeedback::Unavailable,
        });
        true
    }

    pub(super) fn select_mode(&mut self, mode: TimerMode) -> bool {
        if mode == self.mode || !self.mode_switch_enabled() {
            return false;
        }
        self.mode = mode;
        self.state = TimerState::Idle;
        self.minutes_input = DEFAULT_MINUTES.to_owned();
        self.validation_error = None;
        self.notification_feedback = None;
        true
    }

    pub(super) fn start_countdown(&mut self, now: Duration) -> Option<CompletionNotification> {
        let notification = self.reconcile(now);
        if self.mode != TimerMode::Countdown || !self.is_idle() {
            return notification;
        }
        let parsed = self.minutes_input.parse::<u16>();
        let minutes = match parsed {
            Ok(minutes) if (MIN_MINUTES..=MAX_MINUTES).contains(&minutes) => minutes,
            _ => {
                self.validation_error = Some(format!(
                    "Enter a whole number from {MIN_MINUTES} to {MAX_MINUTES} minutes."
                ));
                return notification;
            }
        };
        let Some(generation) = self.next_generation.checked_add(1) else {
            self.validation_error = Some("Timer generation limit reached; restart the app.".into());
            return notification;
        };
        let duration = Duration::from_secs(u64::from(minutes) * 60);
        let Some(deadline) = now.checked_add(duration) else {
            self.validation_error =
                Some("Timer duration is outside the supported clock range.".into());
            return notification;
        };
        self.next_generation = generation;
        self.validation_error = None;
        self.notification_feedback = None;
        self.state = TimerState::CountdownRunning {
            deadline,
            minutes,
            input: self.minutes_input.clone(),
            generation,
        };
        notification
    }

    pub(super) fn start_stopwatch(&mut self, now: Duration) -> Option<CompletionNotification> {
        let notification = self.reconcile(now);
        if self.mode == TimerMode::Stopwatch && self.is_idle() {
            self.state = TimerState::StopwatchRunning {
                started_at: now,
                accumulated: Duration::ZERO,
            };
            self.validation_error = None;
            self.notification_feedback = None;
        }
        notification
    }

    pub(super) fn pause(&mut self, now: Duration) -> Option<CompletionNotification> {
        let notification = self.reconcile(now);
        self.state = match self.state.clone() {
            TimerState::CountdownRunning {
                deadline,
                minutes,
                input,
                generation,
            } => TimerState::CountdownPaused {
                remaining: deadline.saturating_sub(now),
                minutes,
                input,
                generation,
            },
            TimerState::StopwatchRunning {
                started_at,
                accumulated,
            } => TimerState::StopwatchPaused {
                elapsed: accumulated.saturating_add(now.saturating_sub(started_at)),
            },
            state => state,
        };
        notification
    }

    pub(super) fn resume(&mut self, now: Duration) -> Option<CompletionNotification> {
        let notification = self.reconcile(now);
        self.state = match self.state.clone() {
            TimerState::CountdownPaused {
                remaining,
                minutes,
                input,
                generation,
            } => match now.checked_add(remaining) {
                Some(deadline) => TimerState::CountdownRunning {
                    deadline,
                    minutes,
                    input,
                    generation,
                },
                None => {
                    self.validation_error =
                        Some("Timer duration is outside the supported clock range.".into());
                    TimerState::CountdownPaused {
                        remaining,
                        minutes,
                        input,
                        generation,
                    }
                }
            },
            TimerState::StopwatchPaused { elapsed } => TimerState::StopwatchRunning {
                started_at: now,
                accumulated: elapsed,
            },
            state => state,
        };
        notification
    }

    pub(super) fn end(&mut self, now: Duration) -> Option<CompletionNotification> {
        let notification = self.reconcile(now);
        self.state = match self.state.clone() {
            TimerState::CountdownRunning { input, .. }
            | TimerState::CountdownPaused { input, .. } => {
                self.minutes_input = input;
                TimerState::Idle
            }
            TimerState::StopwatchRunning { .. } | TimerState::StopwatchPaused { .. } => {
                TimerState::Idle
            }
            state => state,
        };
        self.validation_error = None;
        notification
    }

    pub(super) fn reset(&mut self) {
        if self.is_completed() {
            self.mode = TimerMode::Countdown;
            self.state = TimerState::Idle;
            self.minutes_input = DEFAULT_MINUTES.to_owned();
            self.validation_error = None;
            self.notification_feedback = None;
        }
    }

    pub(super) fn close_popup(&mut self) {
        match self.state {
            TimerState::CountdownRunning { .. }
            | TimerState::CountdownPaused { .. }
            | TimerState::StopwatchRunning { .. }
            | TimerState::StopwatchPaused { .. } => {}
            TimerState::CountdownCompleted { .. } => {
                self.mode = TimerMode::Countdown;
                self.state = TimerState::Idle;
                self.minutes_input = DEFAULT_MINUTES.to_owned();
                self.notification_feedback = None;
                self.validation_error = None;
            }
            TimerState::Idle => {
                self.minutes_input = DEFAULT_MINUTES.to_owned();
                self.validation_error = None;
                self.notification_feedback = None;
            }
        }
    }

    pub(super) fn next_repaint_delay(&self, now: Duration) -> Option<Duration> {
        let raw = match self.state {
            TimerState::CountdownRunning { deadline, .. } => {
                let remaining = deadline.saturating_sub(now);
                let shown = ceil_seconds(remaining);
                let until_next_display =
                    remaining.saturating_sub(Duration::from_secs(shown.saturating_sub(1)));
                if remaining.is_zero() {
                    MIN_REPAINT_DELAY
                } else {
                    until_next_display
                }
            }
            TimerState::StopwatchRunning {
                started_at,
                accumulated,
            } => {
                let elapsed = accumulated.saturating_add(now.saturating_sub(started_at));
                Duration::from_secs(1)
                    .saturating_sub(Duration::from_nanos(elapsed.subsec_nanos() as u64))
            }
            _ => return None,
        };
        Some(raw.max(MIN_REPAINT_DELAY).min(MAX_REPAINT_DELAY))
    }
}

fn ceil_seconds(duration: Duration) -> u64 {
    duration
        .as_secs()
        .saturating_add(u64::from(duration.subsec_nanos() > 0))
}

pub(super) fn show_popup(
    ctx: &egui::Context,
    timer: &mut SprintTimer,
    now: Duration,
) -> Vec<Command> {
    show_popup_with_host_geometry(ctx, timer, now, None)
}

pub(super) fn show_popup_with_host_geometry(
    ctx: &egui::Context,
    timer: &mut SprintTimer,
    now: Duration,
    captured_host_geometry: Option<SprintTimerHostGeometry>,
) -> Vec<Command> {
    show_popup_with_snapshot_and_host_geometry(ctx, timer, now, captured_host_geometry).0
}

#[derive(Debug, Clone)]
struct SprintTimerPopupRenderSnapshot {
    body: egui::Rect,
    notch: egui::Rect,
    popup: egui::Rect,
    notch_tip: egui::Pos2,
    notch_visible: bool,
    toggle: egui::Rect,
    display: egui::Rect,
    feedback: egui::Rect,
    primary_action: egui::Rect,
    end_action: egui::Rect,
    display_text: String,
    display_accessible_name: String,
    duration_edit_rect: Option<egui::Rect>,
    duration_edit_id: Option<egui::Id>,
    duration_accessible_name: Option<&'static str>,
    duration_caption_rect: Option<egui::Rect>,
    active_display_panel_rect: Option<egui::Rect>,
    feedback_text: String,
    primary_label: &'static str,
    primary_id: egui::Id,
    end_enabled: bool,
    end_id: egui::Id,
    primary_active: bool,
    primary_has_focus: bool,
    primary_clicked: bool,
    end_active: bool,
    end_has_focus: bool,
    end_clicked: bool,
    selector_accessibility: ModeSelectorAccessibility,
    geometry: PopupGeometryLedger,
    opening_progress: f32,
    opening_scale: f32,
    opening_opacity: f32,
    layer_transform: egui::emath::TSTransform,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PopupGeometryCheckpoint {
    content_min: egui::Rect,
    parent_cursor: egui::Rect,
    parent_available: egui::Rect,
}

impl Default for PopupGeometryCheckpoint {
    fn default() -> Self {
        Self {
            content_min: egui::Rect::NOTHING,
            parent_cursor: egui::Rect::NOTHING,
            parent_available: egui::Rect::NOTHING,
        }
    }
}

impl PopupGeometryCheckpoint {
    fn capture(ui: &egui::Ui) -> Self {
        Self {
            content_min: ui.min_rect(),
            parent_cursor: ui.cursor(),
            parent_available: ui.available_rect_before_wrap(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PopupGeometryLedger {
    planned_body: egui::Rect,
    after_display: PopupGeometryCheckpoint,
    editor_parent_before: PopupGeometryCheckpoint,
    editor_parent_after: PopupGeometryCheckpoint,
    editor_assigned: Option<egui::Rect>,
    editor_child_min: Option<egui::Rect>,
    editor_response: Option<egui::Rect>,
    primary_parent_before: PopupGeometryCheckpoint,
    primary_parent_after: PopupGeometryCheckpoint,
    primary_assigned: egui::Rect,
    primary_child_min: egui::Rect,
    primary_response: egui::Rect,
    end_parent_before: PopupGeometryCheckpoint,
    end_parent_after: PopupGeometryCheckpoint,
    end_assigned: egui::Rect,
    end_child_min: egui::Rect,
    end_response: egui::Rect,
    after_actions: PopupGeometryCheckpoint,
    frame_outer: egui::Rect,
    area_response: egui::Rect,
    next_pass_retained_size: Option<egui::Vec2>,
}

impl Default for PopupGeometryLedger {
    fn default() -> Self {
        Self {
            planned_body: egui::Rect::NOTHING,
            after_display: PopupGeometryCheckpoint::default(),
            editor_parent_before: PopupGeometryCheckpoint::default(),
            editor_parent_after: PopupGeometryCheckpoint::default(),
            editor_assigned: None,
            editor_child_min: None,
            editor_response: None,
            primary_parent_before: PopupGeometryCheckpoint::default(),
            primary_parent_after: PopupGeometryCheckpoint::default(),
            primary_assigned: egui::Rect::NOTHING,
            primary_child_min: egui::Rect::NOTHING,
            primary_response: egui::Rect::NOTHING,
            end_parent_before: PopupGeometryCheckpoint::default(),
            end_parent_after: PopupGeometryCheckpoint::default(),
            end_assigned: egui::Rect::NOTHING,
            end_child_min: egui::Rect::NOTHING,
            end_response: egui::Rect::NOTHING,
            after_actions: PopupGeometryCheckpoint::default(),
            frame_outer: egui::Rect::NOTHING,
            area_response: egui::Rect::NOTHING,
            next_pass_retained_size: None,
        }
    }
}

fn retained_popup_area_size(ctx: &egui::Context) -> Option<egui::Vec2> {
    ctx.memory(|memory| {
        memory
            .area_rect(egui::Id::new("sprint-timer-popup"))
            .map(|rect| rect.size())
    })
}

fn popup_feedback_text(timer: &SprintTimer) -> String {
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

fn paint_noninteractive_stage_label(
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

fn centered_active_display_panel_rect(stage: egui::Rect) -> egui::Rect {
    egui::Rect::from_center_size(
        stage.center(),
        egui::vec2(POPUP_ACTIVE_DISPLAY_WIDTH, POPUP_ACTIVE_DISPLAY_HEIGHT),
    )
}

/// Paints a bounded, noninteractive readout. Deliberately avoids `interact`
/// and text widgets so active values have no pointer, keyboard, or edit route.
fn paint_noninteractive_active_display_panel(
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

fn show_popup_contents(
    ui: &mut egui::Ui,
    timer: &SprintTimer,
    now: Duration,
    commands: &mut Vec<Command>,
) -> SprintTimerPopupRenderSnapshot {
    apply_popup_content_style(ui);
    let layout = sprint_timer_popup_layout();
    let (body_rect, _) = ui.allocate_exact_size(layout.body.size(), egui::Sense::hover());
    let absolute = |rect: egui::Rect| rect.translate(body_rect.min.to_vec2());
    let toggle = absolute(layout.toggle);
    let display = absolute(layout.display);
    let feedback = absolute(layout.feedback);
    let primary_action = absolute(layout.primary_action);
    let end_action = absolute(layout.end_action);
    let mut geometry = PopupGeometryLedger {
        planned_body: body_rect,
        ..PopupGeometryLedger::default()
    };

    let selector = show_mode_selector(ui, toggle, timer.mode, timer.mode_switch_enabled());
    commands.extend(mode_activation_commands(selector.activation));

    let display_text;
    let display_accessible_name;
    let mut duration_edit_rect = None;
    let mut duration_edit_id = None;
    let mut duration_accessible_name = None;
    let mut duration_caption_rect = None;
    let mut active_display_panel_rect = None;
    match timer.mode {
        TimerMode::Countdown if timer.is_idle() => {
            let edit_rect = egui::Rect::from_center_size(
                egui::pos2(display.center().x, display.top() + 20.0),
                egui::vec2(
                    POPUP_COUNTDOWN_EDITOR_WIDTH,
                    POPUP_COUNTDOWN_EDITOR_HEIGHT,
                ),
            );
            let editor_background = popup_display_background(ui.visuals());
            let editor_text = popup_display_text(ui.visuals());
            let mut minutes_input = timer.minutes_input.clone();
            geometry.editor_assigned = Some(edit_rect);
            geometry.editor_parent_before = PopupGeometryCheckpoint::capture(ui);
            let mut editor_ui = ui.new_child(
                egui::UiBuilder::new()
                    .id(egui::Id::new("sprint-timer-minutes-child"))
                    .max_rect(edit_rect)
                    .layout(egui::Layout::centered_and_justified(
                        egui::Direction::TopDown,
                    )),
            );
            let response = editor_ui.add(
                egui::TextEdit::singleline(&mut minutes_input)
                    .id(egui::Id::new("sprint-timer-minutes"))
                    .desired_width(edit_rect.width())
                    .font(egui::FontId::proportional(
                        POPUP_COUNTDOWN_EDITOR_FONT_SIZE,
                    ))
                    .background_color(editor_background)
                    .text_color(editor_text)
                    .horizontal_align(egui::Align::Center)
                    .vertical_align(egui::Align::Center)
                    .hint_text("25"),
            );
            geometry.editor_child_min = Some(editor_ui.min_rect());
            geometry.editor_response = Some(response.rect);
            geometry.editor_parent_after = PopupGeometryCheckpoint::capture(ui);
            response.widget_info(|| {
                WidgetInfo::labeled(
                    WidgetType::TextEdit,
                    true,
                    "Sprint duration in minutes".to_owned(),
                )
            });
            if response.changed() {
                commands.push(Command::ToolsSprintTimerSetMinutes {
                    text: minutes_input,
                });
            }
            let enter = (response.has_focus() || response.lost_focus())
                && ui.input(|input| input.key_pressed(Key::Enter));
            if enter {
                commands.push(Command::ToolsSprintTimerStart);
            }

            let caption_rect = egui::Rect::from_min_max(
                egui::pos2(display.left(), edit_rect.bottom()),
                egui::pos2(display.right(), display.bottom()),
            );
            paint_noninteractive_stage_label(
                ui,
                caption_rect,
                "Minutes",
                "Minutes",
                "minutes-caption",
                egui::TextStyle::Body.resolve(ui.style()),
                ui.visuals().weak_text_color(),
            );
            duration_caption_rect = Some(caption_rect);
            duration_edit_rect = Some(response.rect);
            duration_edit_id = Some(response.id);
            duration_accessible_name = Some("Sprint duration in minutes");
            display_text = timer.minutes_input.clone();
            display_accessible_name = "Sprint duration in minutes".to_owned();
        }
        TimerMode::Countdown => {
            display_text = timer.countdown_display(now);
            display_accessible_name = format!("Countdown remaining time {}", display_text);
            active_display_panel_rect = Some(paint_noninteractive_active_display_panel(
                ui,
                display,
                &display_text,
                &display_accessible_name,
                "countdown-readout",
            ));
        }
        TimerMode::Stopwatch => {
            display_text = timer.stopwatch_display(now);
            display_accessible_name = format!("Stopwatch elapsed time {}", display_text);
            active_display_panel_rect = Some(paint_noninteractive_active_display_panel(
                ui,
                display,
                &display_text,
                &display_accessible_name,
                "stopwatch-readout",
            ));
        }
    }

    geometry.after_display = PopupGeometryCheckpoint::capture(ui);

    let feedback_text = popup_feedback_text(timer);
    if !feedback_text.is_empty() {
        let response = ui.interact(
            feedback,
            ui.id().with("sprint-timer-feedback"),
            egui::Sense::hover(),
        );
        response.widget_info(|| {
            WidgetInfo::labeled(WidgetType::Label, true, feedback_text.clone())
        });
        let color = if timer.validation_error.is_some() {
            ui.visuals().error_fg_color
        } else {
            ui.visuals().weak_text_color()
        };
        ui.painter().with_clip_rect(feedback).text(
            egui::pos2(feedback.left() + 1.0, feedback.center().y),
            egui::Align2::LEFT_CENTER,
            &feedback_text,
            egui::TextStyle::Small.resolve(ui.style()),
            color,
        );
    }

    let primary_label = timer.primary_label();
    geometry.primary_parent_before = PopupGeometryCheckpoint::capture(ui);
    geometry.primary_assigned = primary_action;
    let mut primary_ui = ui.new_child(
        egui::UiBuilder::new()
            .id(egui::Id::new("sprint-timer-primary-action"))
            .max_rect(primary_action)
            .layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight)),
    );
    let primary = primary_ui.add(
        egui::Button::new(primary_label).min_size(primary_action.size()),
    );
    geometry.primary_child_min = primary_ui.min_rect();
    geometry.primary_response = primary.rect;
    geometry.primary_parent_after = PopupGeometryCheckpoint::capture(ui);
    primary.widget_info(|| {
        WidgetInfo::labeled(WidgetType::Button, true, primary_label.to_owned())
    });
    if primary.clicked() {
        commands.push(timer.primary_command());
    }

    let end_enabled = timer.end_enabled();
    geometry.end_parent_before = PopupGeometryCheckpoint::capture(ui);
    geometry.end_assigned = end_action;
    let mut end_builder = egui::UiBuilder::new()
        .id(egui::Id::new("sprint-timer-end-action"))
        .max_rect(end_action)
        .layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight));
    if !end_enabled {
        end_builder = end_builder.disabled();
    }
    let mut end_ui = ui.new_child(end_builder);
    let end = end_ui.add(egui::Button::new("End Sprint").min_size(end_action.size()));
    geometry.end_child_min = end_ui.min_rect();
    geometry.end_response = end.rect;
    geometry.end_parent_after = PopupGeometryCheckpoint::capture(ui);
    end.widget_info(|| {
        WidgetInfo::labeled(
            WidgetType::Button,
            end_enabled,
            "End Sprint".to_owned(),
        )
    });
    if end.clicked() && end_enabled {
        commands.push(Command::ToolsSprintTimerEnd);
    }
    geometry.after_actions = PopupGeometryCheckpoint::capture(ui);

    SprintTimerPopupRenderSnapshot {
        body: body_rect,
        notch: egui::Rect::NOTHING,
        popup: body_rect,
        notch_tip: egui::Pos2::ZERO,
        notch_visible: false,
        toggle,
        display,
        feedback,
        primary_action,
        end_action,
        display_text,
        display_accessible_name,
        duration_edit_rect,
        duration_edit_id,
        duration_accessible_name,
        duration_caption_rect,
        active_display_panel_rect,
        feedback_text,
        primary_label,
        primary_id: primary.id,
        end_enabled: timer.end_enabled(),
        end_id: end.id,
        primary_active: primary.has_focus()
            || primary.is_pointer_button_down_on()
            || primary.clicked(),
        primary_has_focus: primary.has_focus(),
        primary_clicked: primary.clicked(),
        end_active: end.has_focus() || end.is_pointer_button_down_on() || end.clicked(),
        end_has_focus: end.has_focus(),
        end_clicked: end.clicked(),
        selector_accessibility: selector.accessibility,
        geometry,
        opening_progress: 1.0,
        opening_scale: POPUP_OPENING_END_SCALE,
        opening_opacity: 1.0,
        layer_transform: egui::emath::TSTransform::IDENTITY,
    }
}

fn apply_popup_content_style(ui: &mut egui::Ui) {
    let focus_stroke = popup_focus_stroke(ui.visuals());
    let visuals = &mut ui.style_mut().visuals;
    visuals.selection.stroke = focus_stroke;
    visuals.widgets.active.bg_stroke = focus_stroke;
}

fn show_popup_with_snapshot(
    ctx: &egui::Context,
    timer: &mut SprintTimer,
    now: Duration,
) -> (Vec<Command>, Option<SprintTimerPopupRenderSnapshot>) {
    show_popup_with_snapshot_and_host_geometry(ctx, timer, now, None)
}

fn show_popup_with_snapshot_and_host_geometry(
    ctx: &egui::Context,
    timer: &mut SprintTimer,
    now: Duration,
    captured_host_geometry: Option<SprintTimerHostGeometry>,
) -> (Vec<Command>, Option<SprintTimerPopupRenderSnapshot>) {
    let current_host_geometry = SprintTimerHostGeometry::from_context(ctx);
    show_popup_with_host_observation(
        ctx,
        timer,
        now,
        captured_host_geometry,
        current_host_geometry,
    )
}

fn show_popup_with_host_observation(
    ctx: &egui::Context,
    timer: &mut SprintTimer,
    now: Duration,
    captured_host_geometry: Option<SprintTimerHostGeometry>,
    current_host_geometry: SprintTimerHostGeometry,
) -> (Vec<Command>, Option<SprintTimerPopupRenderSnapshot>) {
    let mut commands = Vec::new();
    if !timer.popup_open {
        return (commands, None);
    }

    let viewport = ctx.content_rect();
    let egui_time = ctx.input(|input| input.time);

    if let Some(closing) = timer.closing {
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
        return (commands, Some(snapshot));
    }

    if captured_host_geometry
        .is_some_and(|captured| current_host_geometry.changed_from(captured))
    {
        commands.push(Command::ToolsSprintTimerClosePopup);
        return (commands, None);
    }

    let escape = ctx.input(|input| input.key_pressed(Key::Escape));
    let outside_press = !timer.just_opened
        && ctx.input(|input| {
            input.pointer.button_released(egui::PointerButton::Primary)
                && input
                    .pointer
                    .interact_pos()
                    .is_some_and(|pointer| !timer.popup_rect.contains(pointer))
        });
    timer.just_opened = false;
    if escape || outside_press {
        commands.push(Command::ToolsSprintTimerClosePopup);
        return (commands, None);
    }

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

fn paint_popup_notch(
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
    painter.line_segment(
        [left_base, right_base],
        egui::Stroke::new(2.0, fill),
    );
    if border.width > 0.0 && border.color != egui::Color32::TRANSPARENT {
        let notch_stroke = egui::Stroke::new(border.width.max(1.0), border.color);
        painter.line_segment([left_base, tip], notch_stroke);
        painter.line_segment([tip, right_base], notch_stroke);
    }
}

pub(super) struct NativeNotificationSink {
    results_tx: Sender<(u64, NotificationResult)>,
    repaint_context: Option<egui::Context>,
}

impl NativeNotificationSink {
    pub(super) fn new() -> (Self, Receiver<(u64, NotificationResult)>) {
        let (results_tx, results_rx) = mpsc::channel();
        (
            Self {
                results_tx,
                repaint_context: None,
            },
            results_rx,
        )
    }

    pub(super) fn set_repaint_context(&mut self, ctx: &egui::Context) {
        self.repaint_context = Some(ctx.clone());
    }
}

impl NotificationSink for NativeNotificationSink {
    fn request(&mut self, notice: CompletionNotification) -> NotificationResult {
        #[cfg(target_os = "macos")]
        {
            let message = if notice.minutes == 1 {
                "Great Work! You completed a 1 Minute Sprint".to_owned()
            } else {
                format!(
                    "Great Work! You completed a {} Minutes Sprint",
                    notice.minutes
                )
            };
            // The payload contains only a validated numeric duration and fixed text.
            // Run AppleScript out of the UI thread; successful completion means only
            // that the OS command accepted the request, not that it was displayed.
            let script = format!("display notification \"{message}\" with title \"ScriptCraft\"");
            let sender = self.results_tx.clone();
            let repaint_context = self.repaint_context.clone();
            let generation = notice.generation;
            let spawned = thread::Builder::new()
                .name("scriptcraft-notification".to_owned())
                .spawn(move || {
                    let result = ProcessCommand::new("/usr/bin/osascript")
                        .arg("-e")
                        .arg(script)
                        .output()
                        .map(|output| {
                            if output.status.success() {
                                NotificationResult::Accepted
                            } else {
                                NotificationResult::Unavailable
                            }
                        })
                        .unwrap_or(NotificationResult::Unavailable);
                    let _ = sender.send((generation, result));
                    if let Some(context) = repaint_context {
                        context.request_repaint();
                    }
                });
            if spawned.is_ok() {
                NotificationResult::Pending
            } else {
                NotificationResult::Unavailable
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = notice;
            NotificationResult::Unavailable
        }
    }
}

#[cfg(test)]
mod popup_geometry_tests {
    use super::*;

    fn rect(left: f32, top: f32, right: f32, bottom: f32) -> egui::Rect {
        egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom))
    }

    #[test]
    fn host_snapshot_uses_outer_origin_and_inner_size_with_optional_values() {
        let snapshot = SprintTimerHostGeometry::from_rects(
            Some(rect(0.0, 0.0, 820.0, 640.0)),
            Some(rect(110.0, 125.0, 950.0, 800.0)),
        );
        let partial = SprintTimerHostGeometry::from_rects(None, Some(rect(210.0, 225.0, 0.0, 0.0)));

        assert_eq!(snapshot.outer_left, Some(110.0));
        assert_eq!(snapshot.outer_top, Some(125.0));
        assert_eq!(snapshot.inner_width, Some(820.0));
        assert_eq!(snapshot.inner_height, Some(640.0));
        assert_eq!(partial.outer_left, Some(210.0));
        assert_eq!(partial.outer_top, Some(225.0));
        assert!(partial.inner_width.is_none());
        assert!(partial.inner_height.is_none());
    }

    fn content_rects(layout: SprintTimerPopupLayout) -> [egui::Rect; 5] {
        [
            layout.toggle,
            layout.display,
            layout.feedback,
            layout.primary_action,
            layout.end_action,
        ]
    }

    fn contains_rect(outer: egui::Rect, inner: egui::Rect) -> bool {
        outer.min.x <= inner.min.x
            && outer.min.y <= inner.min.y
            && outer.max.x >= inner.max.x
            && outer.max.y >= inner.max.y
    }

    fn overlap(a: egui::Rect, b: egui::Rect) -> bool {
        a.min.x < b.max.x && b.min.x < a.max.x && a.min.y < b.max.y && b.min.y < a.max.y
    }

    #[test]
    fn action_rectangles_are_independent_of_mode_and_timer_state() {
        // The planner deliberately accepts neither a mode nor a timer state.
        let countdown_layout = sprint_timer_popup_layout();
        let stopwatch_layout = sprint_timer_popup_layout();

        assert_eq!(countdown_layout.toggle, stopwatch_layout.toggle);
        assert_eq!(countdown_layout.primary_action, stopwatch_layout.primary_action);
        assert_eq!(countdown_layout.end_action, stopwatch_layout.end_action);
    }

    #[test]
    fn toggle_display_feedback_and_action_pair_are_centered() {
        let layout = sprint_timer_popup_layout();
        let center_x = layout.body.center().x;

        assert_eq!(layout.toggle.center().x, center_x);
        assert_eq!(layout.display.center().x, center_x);
        assert_eq!(layout.feedback.center().x, center_x);
        assert_eq!(
            (layout.primary_action.min.x + layout.end_action.max.x) / 2.0,
            center_x
        );
        assert_eq!(layout.primary_action.min.y, layout.end_action.min.y);
        assert_eq!(layout.primary_action.max.y, layout.end_action.max.y);
    }

    #[test]
    fn planned_rectangles_are_finite_and_nonoverlapping() {
        let layout = sprint_timer_popup_layout();
        let rects = content_rects(layout);

        for rect in std::iter::once(layout.body).chain(rects) {
            assert!(rect.min.x.is_finite() && rect.min.y.is_finite());
            assert!(rect.max.x.is_finite() && rect.max.y.is_finite());
        }
        for (index, rect) in rects.iter().enumerate() {
            for other in rects.iter().skip(index + 1) {
                assert!(!overlap(*rect, *other), "overlapping rectangles: {rect:?}, {other:?}");
            }
        }
    }

    #[test]
    fn every_planned_content_rect_is_contained_by_the_body() {
        let layout = sprint_timer_popup_layout();

        for rect in content_rects(layout) {
            assert!(contains_rect(layout.body, rect), "outside body: {rect:?}");
        }
    }

    #[test]
    fn planner_is_repeatable() {
        let expected = sprint_timer_popup_layout();

        for _ in 0..128 {
            assert_eq!(sprint_timer_popup_layout(), expected);
        }
    }

    #[test]
    fn popup_placement_sits_above_icon_with_explicit_notch_and_clearance() {
        let anchor = rect(280.0, 370.0, 300.0, 390.0);
        let viewport = rect(0.0, 0.0, 600.0, 500.0);
        let placement = sprint_timer_popup_placement(Some(anchor), viewport);

        assert_eq!(placement.body.size(), egui::vec2(POPUP_BODY_WIDTH, POPUP_BODY_HEIGHT));
        assert_eq!(placement.body.center().x, anchor.center().x);
        assert_eq!(placement.body.bottom(), placement.notch.top());
        assert_eq!(placement.notch_tip.x, anchor.center().x);
        assert_eq!(placement.notch_tip.y, anchor.top() - POPUP_ICON_CLEARANCE);
        assert_eq!(placement.body.top(), 193.0);
        assert!(placement.notch_visible);
        assert_eq!(placement.popup, placement.body.union(placement.notch));
    }

    #[test]
    fn popup_placement_clamps_body_and_keeps_notch_within_safe_corner_clearance() {
        let viewport = rect(0.0, 0.0, 600.0, 500.0);
        let left = sprint_timer_popup_placement(Some(rect(0.0, 370.0, 20.0, 390.0)), viewport);
        let right = sprint_timer_popup_placement(Some(rect(580.0, 370.0, 600.0, 390.0)), viewport);

        assert_eq!(left.body.left(), viewport.left());
        assert_eq!(left.notch_tip.x, left.body.left() + POPUP_CORNER_RADIUS + POPUP_NOTCH_WIDTH / 2.0);
        assert_eq!(right.body.right(), viewport.right());
        assert_eq!(right.notch_tip.x, right.body.right() - POPUP_CORNER_RADIUS - POPUP_NOTCH_WIDTH / 2.0);
        for placement in [left, right] {
            assert!(placement.notch_visible);
            assert!(placement.notch.left() >= placement.body.left() + POPUP_CORNER_RADIUS);
            assert!(placement.notch.right() <= placement.body.right() - POPUP_CORNER_RADIUS);
            assert!(placement.popup.left() >= viewport.left());
            assert!(placement.popup.right() <= viewport.right());
        }
    }

    #[test]
    fn short_and_missing_anchor_placements_use_bounded_pointer_free_fallbacks() {
        let short_viewport = rect(12.0, 20.0, 252.0, 160.0);
        let short = sprint_timer_popup_placement(
            Some(rect(100.0, 100.0, 120.0, 120.0)),
            short_viewport,
        );
        assert!(!short.notch_visible);
        assert_eq!(short.body.min, short_viewport.min);
        assert_eq!(short.notch, egui::Rect::NOTHING);
        assert!(short.popup.intersect(short_viewport).is_positive());

        let viewport = rect(20.0, 30.0, 620.0, 430.0);
        let missing = sprint_timer_popup_placement(None, viewport);
        assert!(!missing.notch_visible);
        assert_eq!(missing.body.center(), viewport.center());
        assert!(viewport.contains(missing.body.min));
        assert!(missing.popup.intersect(viewport).is_positive());
    }

    #[test]
    fn popup_placement_uses_fixed_logical_size_at_high_density_coordinates() {
        let viewport = rect(0.0, 0.0, 1200.0, 800.0);
        let anchor = rect(780.0, 740.0, 820.0, 780.0);
        let placement = sprint_timer_popup_placement(Some(anchor), viewport);

        assert_eq!(placement.body.size(), egui::vec2(300.0, 165.0));
        assert_eq!(placement.body.center().x, 800.0);
        assert_eq!(placement.body.top(), 563.0);
        assert_eq!(placement.notch_tip, egui::pos2(800.0, 738.0));
    }
}

#[cfg(test)]
mod popup_mode_selector_tests {
    use super::*;

    fn render_selector(
        ctx: &egui::Context,
        mode: TimerMode,
        enabled: bool,
        events: Vec<egui::Event>,
    ) -> ModeSelectorRender {
        render_selector_with_output(ctx, mode, enabled, events).0
    }

    fn render_selector_with_output(
        ctx: &egui::Context,
        mode: TimerMode,
        enabled: bool,
        events: Vec<egui::Event>,
    ) -> (ModeSelectorRender, egui::FullOutput) {
        let mut rendered = ModeSelectorRender::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(600.0, 400.0),
            )),
            events,
            ..Default::default()
        };
        let output = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let toggle = egui::Rect::from_min_size(
                    egui::pos2(16.0, 18.0),
                    egui::vec2(POPUP_BODY_WIDTH - 2.0 * POPUP_BODY_INSET, POPUP_TOGGLE_HEIGHT),
                );
                rendered = show_mode_selector(ui, toggle, mode, enabled);
            });
        });
        (rendered, output)
    }

    fn click_selector(
        ctx: &egui::Context,
        mode: TimerMode,
        enabled: bool,
        point: egui::Pos2,
    ) -> ModeSelectorRender {
        let _ = render_selector(
            ctx,
            mode,
            enabled,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        render_selector(
            ctx,
            mode,
            enabled,
            vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )
    }

    fn target_mode(mode: TimerMode) -> CommandTimerMode {
        match mode {
            TimerMode::Countdown => CommandTimerMode::Countdown,
            TimerMode::Stopwatch => CommandTimerMode::Stopwatch,
        }
    }

    fn assert_one_mode_command(activation: Option<TimerMode>, expected: TimerMode) {
        let commands = mode_activation_commands(activation);
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands.as_slice(),
            [Command::ToolsSprintTimerSetMode { mode }] if *mode == target_mode(expected)
        ));
    }

    fn pointer_regions(geometry: ModeSelectorGeometry) -> [(egui::Rect, TimerMode); 3] {
        [
            (geometry.countdown, TimerMode::Countdown),
            (geometry.switch, TimerMode::Stopwatch),
            (geometry.stopwatch, TimerMode::Stopwatch),
        ]
    }

    fn rendered_rect_shapes(output: &egui::FullOutput) -> Vec<&egui::epaint::RectShape> {
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape) => Some(shape),
                _ => None,
            })
            .collect()
    }

    fn rendered_circles(output: &egui::FullOutput) -> Vec<&egui::epaint::CircleShape> {
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Circle(shape) => Some(shape),
                _ => None,
            })
            .collect()
    }

    fn pointer_click_events(point: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
        let mut events = vec![egui::Event::PointerMoved(point)];
        events.push(egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        events
    }

    #[test]
    fn selector_geometry_uses_popup_tokens_for_track_knob_travel_and_label_centers() {
        let toggle = egui::Rect::from_min_size(
            egui::pos2(16.0, 18.0),
            egui::vec2(POPUP_BODY_WIDTH - 2.0 * POPUP_BODY_INSET, POPUP_TOGGLE_HEIGHT),
        );
        let geometry = mode_selector_geometry(toggle);
        let track = mode_switch_track_rect(geometry.switch);
        let unchecked_center = mode_switch_knob_center(track, false);
        let checked_center = mode_switch_knob_center(track, true);

        assert_eq!(POPUP_MODE_LABEL_FONT_SIZE, 14.0);
        assert_eq!(geometry.countdown.width(), POPUP_MODE_LABEL_WIDTH);
        assert_eq!(geometry.stopwatch.width(), POPUP_MODE_LABEL_WIDTH);
        assert_eq!(track.size(), egui::vec2(38.0, 22.0));
        assert_eq!(POPUP_MODE_SWITCH_TRACK_RADIUS, 11);
        assert_eq!(POPUP_MODE_SWITCH_KNOB_DIAMETER, 18.0);
        assert_eq!(POPUP_MODE_SWITCH_KNOB_END_INSET, 11.0);
        assert_eq!(unchecked_center.x - track.left(), 11.0);
        assert_eq!(track.right() - checked_center.x, 11.0);
        assert_eq!(checked_center.x - unchecked_center.x, 16.0);
        assert_eq!(unchecked_center.y, track.center().y);
        assert_eq!(checked_center.y, track.center().y);
        assert_eq!(geometry.countdown.center().y, track.center().y);
        assert_eq!(geometry.stopwatch.center().y, track.center().y);
        assert_eq!(geometry.countdown.center().x, toggle.center().x - 67.0);
        assert_eq!(geometry.stopwatch.center().x, toggle.center().x + 67.0);
    }

    #[test]
    fn rendered_track_and_knob_keep_reference_dimensions_and_theme_enabled_states() {
        for dark in [false, true] {
            for (mode, enabled) in [
                (TimerMode::Countdown, true),
                (TimerMode::Stopwatch, true),
                (TimerMode::Countdown, false),
                (TimerMode::Stopwatch, false),
            ] {
                let ctx = egui::Context::default();
                ctx.set_visuals(if dark {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                });
                let (rendered, output) = render_selector_with_output(&ctx, mode, enabled, Vec::new());
                let track_rect = mode_switch_track_rect(rendered.geometry.switch);
                let track_shape = rendered_rect_shapes(&output)
                    .into_iter()
                    .find(|shape| shape.rect == track_rect)
                    .expect("the switch track is rendered at its planned geometry");
                let knob_shape = rendered_circles(&output)
                    .into_iter()
                    .find(|shape| shape.radius == POPUP_MODE_SWITCH_KNOB_DIAMETER / 2.0)
                    .expect("the switch knob is rendered at the popup-local diameter");
                let visuals = ctx.style().visuals.clone();
                let checked = mode == TimerMode::Stopwatch;
                let expected_track_fill = if !enabled {
                    visuals.widgets.noninteractive.bg_fill
                } else if checked {
                    visuals.selection.bg_fill
                } else {
                    visuals.widgets.inactive.bg_fill
                };
                let expected_knob_color = if !enabled {
                    visuals.widgets.noninteractive.fg_stroke.color
                } else if checked {
                    egui::Color32::WHITE
                } else {
                    visuals.widgets.inactive.fg_stroke.color
                };

                assert_eq!(rendered.accessibility.enabled, enabled);
                assert_eq!(track_shape.rect.size(), egui::vec2(38.0, 22.0));
                assert_eq!(track_shape.corner_radius, egui::CornerRadius::same(11));
                assert_eq!(track_shape.fill, expected_track_fill);
                assert_eq!(knob_shape.center, mode_switch_knob_center(track_rect, checked));
                assert_eq!(knob_shape.fill, expected_knob_color);
                assert_eq!(knob_shape.radius * 2.0, 18.0);
            }
        }
    }

    #[test]
    fn rendered_focus_uses_popup_accent_and_selected_labels_have_no_background_fill() {
        for dark in [false, true] {
            for label_focus in [false, true] {
                let ctx = egui::Context::default();
                let mut visuals = if dark {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                };
                visuals.hyperlink_color = egui::Color32::from_rgb(20, 170, 90);
                visuals.selection.stroke = egui::Stroke::new(2.0, egui::Color32::RED);
                ctx.set_visuals(visuals);

                let (initial, _) = render_selector_with_output(
                    &ctx,
                    TimerMode::Countdown,
                    true,
                    Vec::new(),
                );
                let focused_rect = if label_focus {
                    initial.geometry.countdown
                } else {
                    initial.geometry.switch
                };
                let _ = render_selector_with_output(
                    &ctx,
                    TimerMode::Countdown,
                    true,
                    pointer_click_events(focused_rect.center(), true),
                );
                let (focused, output) = render_selector_with_output(
                    &ctx,
                    TimerMode::Countdown,
                    true,
                    pointer_click_events(focused_rect.center(), false),
                );
                let focused_route = if label_focus { 0 } else { 1 };
                let focus_outline_rect = mode_focus_rect(focused_rect);
                let outline = rendered_rect_shapes(&output)
                    .into_iter()
                    .find(|shape| shape.rect == focus_outline_rect && shape.fill == egui::Color32::TRANSPARENT)
                    .expect("focused selector route has a visible outline");
                let selected_label_fill_rect = focused.geometry.countdown.shrink2(egui::vec2(2.0, 3.0));

                assert!(focused.focused[focused_route]);
                assert_eq!(outline.stroke.width, POPUP_FOCUS_STROKE_WIDTH);
                assert_eq!(outline.stroke, popup_focus_stroke(&ctx.style().visuals));
                assert_ne!(outline.stroke.color, ctx.style().visuals.selection.stroke.color);
                assert!(!rendered_rect_shapes(&output)
                    .iter()
                    .any(|shape| shape.rect == selected_label_fill_rect));
            }
        }
    }

    #[test]
    fn rendered_countdown_and_stopwatch_labels_and_switch_activate_once() {
        for current in [TimerMode::Countdown, TimerMode::Stopwatch] {
            let initial_context = egui::Context::default();
            let initial = render_selector(&initial_context, current, true, Vec::new());
            assert_eq!(initial.geometry.toggle.center().x, initial.geometry.switch.center().x);
            assert_eq!(initial.geometry.countdown.center().y, initial.geometry.switch.center().y);

            for (region_index, (region, label_target)) in
                pointer_regions(initial.geometry).into_iter().enumerate()
            {
                let ctx = egui::Context::default();
                let first = render_selector(&ctx, current, true, Vec::new());
                let rect = match region_index {
                    0 => first.geometry.countdown,
                    1 => first.geometry.switch,
                    _ => first.geometry.stopwatch,
                };
                let expected = if region_index == 1 {
                    match current {
                        TimerMode::Countdown => TimerMode::Stopwatch,
                        TimerMode::Stopwatch => TimerMode::Countdown,
                    }
                } else {
                    label_target
                };
                let activated = click_selector(&ctx, current, true, rect.center());
                assert_one_mode_command(activated.activation, expected);
            }
        }
    }

    #[test]
    fn rendered_mode_names_and_mutually_exclusive_checkbox_state_follow_both_modes() {
        for (mode, countdown_selected, stopwatch_selected) in [
            (TimerMode::Countdown, true, false),
            (TimerMode::Stopwatch, false, true),
        ] {
            let rendered = render_selector(&egui::Context::default(), mode, true, Vec::new());
            assert_eq!(rendered.accessibility.countdown_name, "Countdown");
            assert_eq!(rendered.accessibility.stopwatch_name, "Stopwatch");
            assert_eq!(rendered.accessibility.switch_name, "Stopwatch mode");
            assert_eq!(rendered.accessibility.countdown_selected, countdown_selected);
            assert_eq!(rendered.accessibility.stopwatch_selected, stopwatch_selected);
            assert_eq!(rendered.accessibility.switch_checked, stopwatch_selected);
            assert!(rendered.accessibility.enabled);
        }
    }

    #[test]
    fn running_and_paused_countdown_and_stopwatch_render_all_selector_routes_disabled() {
        let mut countdown_running = SprintTimer::default();
        let _ = countdown_running.start_countdown(Duration::ZERO);
        let mut countdown_paused = countdown_running.clone();
        let _ = countdown_paused.pause(Duration::from_secs(2));

        let mut stopwatch_running = SprintTimer::default();
        assert!(stopwatch_running.select_mode(TimerMode::Stopwatch));
        let _ = stopwatch_running.start_stopwatch(Duration::ZERO);
        let mut stopwatch_paused = stopwatch_running.clone();
        let _ = stopwatch_paused.pause(Duration::from_secs(2));

        for dark in [false, true] {
            for timer in [
                countdown_running.clone(),
                countdown_paused.clone(),
                stopwatch_running.clone(),
                stopwatch_paused.clone(),
            ] {
                assert!(!timer.mode_switch_enabled());
                let ctx = egui::Context::default();
                ctx.set_visuals(if dark {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                });
                let initial =
                    render_selector(&ctx, timer.mode, timer.mode_switch_enabled(), Vec::new());
                assert!(!initial.accessibility.enabled);
                for (region, _) in pointer_regions(initial.geometry) {
                    let rendered = click_selector(
                        &ctx,
                        timer.mode,
                        timer.mode_switch_enabled(),
                        region.center(),
                    );
                    assert_eq!(rendered.activation, None);
                    assert!(mode_activation_commands(rendered.activation).is_empty());
                }
            }
        }
    }

    #[test]
    fn every_focused_route_accepts_enter_and_space_and_emits_one_command() {
        for current in [TimerMode::Countdown, TimerMode::Stopwatch] {
            for region_index in 0..3 {
                for key in [Key::Enter, Key::Space] {
                    let ctx = egui::Context::default();
                    let initial = render_selector(&ctx, current, true, Vec::new());
                    let rect = match region_index {
                        0 => initial.geometry.countdown,
                        1 => initial.geometry.switch,
                        _ => initial.geometry.stopwatch,
                    };
                    let focused = click_selector(&ctx, current, true, rect.center());
                    assert!(focused.focused[region_index]);

                    let activated = render_selector(
                        &ctx,
                        current,
                        true,
                        vec![egui::Event::Key {
                            key,
                            physical_key: Some(key),
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers::NONE,
                        }],
                    );
                    let expected = match region_index {
                        0 => TimerMode::Countdown,
                        1 => match current {
                            TimerMode::Countdown => TimerMode::Stopwatch,
                            TimerMode::Stopwatch => TimerMode::Countdown,
                        },
                        _ => TimerMode::Stopwatch,
                    };
                    assert_one_mode_command(activated.activation, expected);
                }
            }
        }
    }
}

#[cfg(test)]
mod popup_render_tests {
    use super::*;

    fn render_popup_frame(
        ctx: &egui::Context,
        timer: &mut SprintTimer,
        now: Duration,
        events: Vec<egui::Event>,
    ) -> (Vec<Command>, Option<SprintTimerPopupRenderSnapshot>) {
        render_popup_frame_on_viewport(
            ctx,
            timer,
            now,
            events,
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0)),
        )
    }

    fn render_popup_frame_with_output(
        ctx: &egui::Context,
        timer: &mut SprintTimer,
        now: Duration,
        events: Vec<egui::Event>,
    ) -> (
        Vec<Command>,
        Option<SprintTimerPopupRenderSnapshot>,
        egui::FullOutput,
    ) {
        render_popup_frame_with_output_on_viewport(
            ctx,
            timer,
            now,
            events,
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0)),
        )
    }

    fn render_popup_frame_with_output_on_viewport(
        ctx: &egui::Context,
        timer: &mut SprintTimer,
        now: Duration,
        events: Vec<egui::Event>,
        screen_rect: egui::Rect,
    ) -> (
        Vec<Command>,
        Option<SprintTimerPopupRenderSnapshot>,
        egui::FullOutput,
    ) {
        let mut rendered = (Vec::new(), None);
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen_rect),
                events,
                ..Default::default()
            },
            |ctx| {
                rendered = show_popup_with_snapshot(ctx, timer, now);
            },
        );
        (rendered.0, rendered.1, output)
    }

    fn render_popup_frame_on_viewport(
        ctx: &egui::Context,
        timer: &mut SprintTimer,
        now: Duration,
        events: Vec<egui::Event>,
        screen_rect: egui::Rect,
    ) -> (Vec<Command>, Option<SprintTimerPopupRenderSnapshot>) {
        let mut rendered = (Vec::new(), None);
        let input = egui::RawInput {
            screen_rect: Some(screen_rect),
            events,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            rendered = show_popup_with_snapshot(ctx, timer, now);
        });
        rendered
    }

    fn render_popup_frame_at_egui_time(
        ctx: &egui::Context,
        timer: &mut SprintTimer,
        now: Duration,
        egui_time: f64,
        events: Vec<egui::Event>,
        screen_rect: egui::Rect,
    ) -> (Vec<Command>, Option<SprintTimerPopupRenderSnapshot>) {
        let mut rendered = (Vec::new(), None);
        let input = egui::RawInput {
            screen_rect: Some(screen_rect),
            time: Some(egui_time),
            events,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            rendered = show_popup_with_snapshot(ctx, timer, now);
        });
        rendered
    }

    fn render_popup_frame_with_host_geometry(
        ctx: &egui::Context,
        timer: &mut SprintTimer,
        captured: SprintTimerHostGeometry,
        current: SprintTimerHostGeometry,
    ) -> (Vec<Command>, Option<SprintTimerPopupRenderSnapshot>) {
        let mut rendered = (Vec::new(), None);
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(600.0, 400.0),
                )),
                ..Default::default()
            },
            |ctx| {
                rendered = show_popup_with_host_observation(
                    ctx,
                    timer,
                    Duration::ZERO,
                    Some(captured),
                    current,
                );
            },
        );
        rendered
    }

    fn host_geometry(left: f32, top: f32, width: f32, height: f32) -> SprintTimerHostGeometry {
        SprintTimerHostGeometry {
            outer_left: Some(left),
            outer_top: Some(top),
            inner_width: Some(width),
            inner_height: Some(height),
        }
    }

    fn open_timer() -> SprintTimer {
        SprintTimer {
            popup_open: true,
            just_opened: true,
            popup_anchor: egui::Rect::from_min_max(
                egui::pos2(290.0, 370.0),
                egui::pos2(310.0, 390.0),
            ),
            ..SprintTimer::default()
        }
    }

    fn render_passive_popup_snapshot(
        ctx: &egui::Context,
        timer: &mut SprintTimer,
        now: Duration,
    ) -> SprintTimerPopupRenderSnapshot {
        let rendered = render_popup_frame(ctx, timer, now, Vec::new());
        assert!(rendered.0.is_empty(), "passive rendering emits no commands");
        snapshot(rendered)
    }

    fn render_settled_popup_geometry(
        ctx: &egui::Context,
        timer: &mut SprintTimer,
        now: Duration,
        viewport: egui::Rect,
    ) -> (SprintTimerPopupRenderSnapshot, egui::FullOutput) {
        let _ = render_popup_frame_on_viewport(ctx, timer, now, Vec::new(), viewport);
        let (commands, rendered, output) =
            render_popup_frame_with_output_on_viewport(ctx, timer, now, Vec::new(), viewport);
        assert!(commands.is_empty(), "passive rendering emits no commands");
        (snapshot((commands, rendered)), output)
    }

    fn snapshot(
        rendered: (Vec<Command>, Option<SprintTimerPopupRenderSnapshot>),
    ) -> SprintTimerPopupRenderSnapshot {
        assert!(rendered.1.is_some(), "open popup renders a composition");
        match rendered.1 {
            Some(snapshot) => snapshot,
            None => SprintTimerPopupRenderSnapshot {
                body: egui::Rect::NOTHING,
                notch: egui::Rect::NOTHING,
                popup: egui::Rect::NOTHING,
                notch_tip: egui::Pos2::ZERO,
                notch_visible: false,
                toggle: egui::Rect::NOTHING,
                display: egui::Rect::NOTHING,
                feedback: egui::Rect::NOTHING,
                primary_action: egui::Rect::NOTHING,
                end_action: egui::Rect::NOTHING,
                display_text: String::new(),
                display_accessible_name: String::new(),
                duration_edit_rect: None,
                duration_edit_id: None,
                duration_accessible_name: None,
                duration_caption_rect: None,
                active_display_panel_rect: None,
                feedback_text: String::new(),
                primary_label: "",
                primary_id: egui::Id::NULL,
                end_enabled: false,
                end_id: egui::Id::NULL,
                primary_active: false,
                primary_has_focus: false,
                primary_clicked: false,
                end_active: false,
                end_has_focus: false,
                end_clicked: false,
                selector_accessibility: ModeSelectorAccessibility::default(),
                geometry: PopupGeometryLedger::default(),
                opening_progress: 1.0,
                opening_scale: POPUP_OPENING_END_SCALE,
                opening_opacity: 1.0,
                layer_transform: egui::emath::TSTransform::IDENTITY,
            },
        }
    }

    #[test]
    fn controlled_egui_time_covers_initial_middle_and_final_opening_progress() {
        let anchor = egui::pos2(300.0, 380.0);
        let initial = popup_opening_presentation(Some(12.0), 12.0, anchor);
        let middle = popup_opening_presentation(Some(12.0), 12.07, anchor);
        let final_frame = popup_opening_presentation(Some(12.0), 12.14, anchor);

        assert_eq!(initial.progress, 0.0);
        assert_eq!(initial.scale, 0.85);
        assert_eq!(initial.opacity, 0.0);
        assert_eq!(middle.progress, 0.875);
        assert!((middle.scale - 0.98125).abs() < 0.00001);
        assert_eq!(middle.opacity, 0.875);
        assert_eq!(final_frame.progress, 1.0);
        assert_eq!(final_frame.scale, 1.0);
        assert_eq!(final_frame.opacity, 1.0);
        assert!(initial.active && middle.active);
        assert!(!final_frame.active);
    }

    #[test]
    fn opening_transform_scales_exactly_about_the_captured_icon_center() {
        let icon_center = egui::pos2(418.0, 276.0);
        let point = egui::pos2(118.0, 76.0);
        let scale = 0.925;
        let transform = popup_opening_transform(icon_center, scale);
        let expected = icon_center + (point - icon_center) * scale;

        assert_eq!(transform.scaling, scale);
        assert_eq!(transform * icon_center, icon_center);
        assert_eq!(transform * point, expected);
        assert_eq!(transform.translation, icon_center.to_vec2() * (1.0 - scale));
    }

    #[test]
    fn opening_scale_and_opacity_remain_bounded_before_during_and_after_transition() {
        for egui_time in [2.0, 2.001, 2.035, 2.07, 2.139, 2.14, 3.0] {
            let opening = popup_opening_presentation(
                Some(2.0),
                egui_time,
                egui::pos2(100.0, 200.0),
            );
            assert!((0.85..=1.0).contains(&opening.scale));
            assert!((0.0..=1.0).contains(&opening.opacity));
            assert!((0.0..=1.0).contains(&opening.progress));
            assert!(opening.transform.is_valid());
        }
    }

    #[test]
    fn opening_repaint_is_scheduled_only_until_the_transition_finishes() {
        let center = egui::pos2(50.0, 60.0);
        let opening = popup_opening_presentation(Some(1.0), 1.0, center);
        let nearly_done = popup_opening_presentation(Some(1.0), 1.139, center);
        let completed = popup_opening_presentation(Some(1.0), 1.14, center);
        let already_open = popup_opening_presentation(None, 9.0, center);

        assert_eq!(opening.repaint_after, Some(POPUP_OPENING_REPAINT_INTERVAL));
        assert!(nearly_done.repaint_after.is_some_and(|delay| {
            delay > Duration::ZERO && delay <= POPUP_OPENING_REPAINT_INTERVAL
        }));
        assert_eq!(completed.repaint_after, None);
        assert_eq!(already_open.repaint_after, None);
    }

    #[test]
    fn rendered_opening_uses_fixed_rest_geometry_and_restores_identity_at_completion() {
        let ctx = egui::Context::default();
        let mut timer = open_timer();
        timer.just_opened = false;
        timer.start_popup_opening(20.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0));
        let initial = snapshot(render_popup_frame_at_egui_time(
            &ctx,
            &mut timer,
            Duration::ZERO,
            20.0,
            Vec::new(),
            screen,
        ));
        let rest = sprint_timer_popup_placement(Some(timer.popup_anchor), screen);
        let layer_id = sprint_timer_popup_layer_id();

        assert_eq!(initial.body, rest.body);
        assert_eq!(initial.opening_scale, POPUP_OPENING_START_SCALE);
        assert_eq!(initial.opening_opacity, 0.0);
        assert_eq!(initial.layer_transform * timer.popup_anchor.center(), timer.popup_anchor.center());
        assert_eq!(timer.popup_rect, (initial.layer_transform * rest.popup).intersect(screen));

        let final_frame = snapshot(render_popup_frame_at_egui_time(
            &ctx,
            &mut timer,
            Duration::ZERO,
            20.14,
            Vec::new(),
            screen,
        ));
        assert_eq!(final_frame.body, rest.body);
        assert_eq!(final_frame.opening_progress, 1.0);
        assert_eq!(final_frame.opening_opacity, 1.0);
        assert_eq!(final_frame.layer_transform, egui::emath::TSTransform::IDENTITY);
        assert_eq!(ctx.layer_transform_to_global(layer_id), None);
        assert_eq!(timer.opening_started_at, None);
    }

    #[test]
    fn repeated_open_cycles_start_from_the_new_timestamp_without_stale_transform() {
        let ctx = egui::Context::default();
        let mut timer = open_timer();
        timer.just_opened = false;
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0));
        timer.start_popup_opening(3.0);
        let first = snapshot(render_popup_frame_at_egui_time(
            &ctx,
            &mut timer,
            Duration::ZERO,
            3.0,
            Vec::new(),
            screen,
        ));
        timer.clear_popup_transition(&ctx);
        assert_eq!(ctx.layer_transform_to_global(sprint_timer_popup_layer_id()), None);

        timer.start_popup_opening(8.0);
        let reopened = snapshot(render_popup_frame_at_egui_time(
            &ctx,
            &mut timer,
            Duration::ZERO,
            8.0,
            Vec::new(),
            screen,
        ));
        assert_eq!(first.opening_progress, 0.0);
        assert_eq!(reopened.opening_progress, 0.0);
        assert_eq!(reopened.opening_scale, POPUP_OPENING_START_SCALE);
        assert_eq!(reopened.layer_transform, first.layer_transform);
        assert_eq!(timer.opening_started_at, Some(8.0));
    }

    #[test]
    fn root_and_title_page_opening_transforms_use_viewport_local_icon_centers() {
        let root_ctx = egui::Context::default();
        let title_ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640.0, 440.0));
        let mut root_timer = open_timer();
        let mut title_timer = open_timer();
        root_timer.just_opened = false;
        title_timer.just_opened = false;
        root_timer.popup_anchor = egui::Rect::from_min_size(egui::pos2(500.0, 400.0), egui::vec2(20.0, 20.0));
        title_timer.popup_anchor = egui::Rect::from_min_size(egui::pos2(120.0, 300.0), egui::vec2(20.0, 20.0));
        root_timer.start_popup_opening(4.0);
        title_timer.start_popup_opening(4.0);

        let root = snapshot(render_popup_frame_at_egui_time(
            &root_ctx,
            &mut root_timer,
            Duration::ZERO,
            4.07,
            Vec::new(),
            screen,
        ));
        let title = snapshot(render_popup_frame_at_egui_time(
            &title_ctx,
            &mut title_timer,
            Duration::ZERO,
            4.07,
            Vec::new(),
            screen,
        ));

        assert_eq!(root.layer_transform * root_timer.popup_anchor.center(), root_timer.popup_anchor.center());
        assert_eq!(title.layer_transform * title_timer.popup_anchor.center(), title_timer.popup_anchor.center());
        assert_ne!(root.layer_transform.translation, title.layer_transform.translation);
        assert_eq!(root.opening_scale, title.opening_scale);
    }

    #[test]
    fn opening_disables_control_activation_and_preserves_timer_semantics() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0));
        let mut timer = open_timer();
        timer.just_opened = false;
        timer.mode = TimerMode::Stopwatch;
        let _ = timer.start_stopwatch(Duration::from_secs(5));
        timer.start_popup_opening(15.0);
        let state_before = timer.state.clone();
        let mode_before = timer.mode;
        let minutes_before = timer.minutes_input.clone();
        let generation_before = timer.next_generation;
        let initial = snapshot(render_popup_frame_at_egui_time(
            &ctx,
            &mut timer,
            Duration::from_secs(5),
            15.0,
            Vec::new(),
            screen,
        ));
        let click = initial.layer_transform * initial.primary_action.center();
        let events = vec![
            egui::Event::PointerMoved(click),
            egui::Event::PointerButton {
                pos: click,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ];
        let (press_commands, _) = render_popup_frame_at_egui_time(
            &ctx,
            &mut timer,
            Duration::from_secs(5),
            15.01,
            events,
            screen,
        );
        let release_commands = render_popup_frame_at_egui_time(
            &ctx,
            &mut timer,
            Duration::from_secs(5),
            15.02,
            vec![egui::Event::PointerButton {
                pos: click,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            screen,
        )
        .0;

        assert!(press_commands.is_empty());
        assert!(release_commands.is_empty());
        assert_eq!(timer.state, state_before);
        assert_eq!(timer.mode, mode_before);
        assert_eq!(timer.minutes_input, minutes_before);
        assert_eq!(timer.next_generation, generation_before);
        // The popup renderer receives no document/session handle, so an opening
        // frame can emit timer commands only and cannot mutate screenplay state.
    }

    fn click_popup_point(
        ctx: &egui::Context,
        timer: &mut SprintTimer,
        now: Duration,
        point: egui::Pos2,
    ) -> (Vec<Command>, Option<SprintTimerPopupRenderSnapshot>) {
        let _ = render_popup_frame(
            ctx,
            timer,
            now,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        render_popup_frame(
            ctx,
            timer,
            now,
            vec![egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )
    }

    fn release_at(point: egui::Pos2) -> Vec<egui::Event> {
        vec![egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }]
    }

    fn assert_same_fixed_tracks(
        left: &SprintTimerPopupRenderSnapshot,
        right: &SprintTimerPopupRenderSnapshot,
    ) {
        assert_eq!(left.toggle, right.toggle);
        assert_eq!(left.primary_action, right.primary_action);
        assert_eq!(left.end_action, right.end_action);
    }

    fn assert_same_action_ids(
        snapshot: &SprintTimerPopupRenderSnapshot,
        expected: (egui::Id, egui::Id),
    ) {
        assert_eq!(snapshot.primary_id, expected.0);
        assert_eq!(snapshot.end_id, expected.1);
    }

    fn accesskit_action_node<'a>(
        output: &'a egui::FullOutput,
        id: egui::Id,
    ) -> &'a egui::accesskit::Node {
        output
            .platform_output
            .accesskit_update
            .as_ref()
            .expect("enabled accessibility produces a tree update")
            .nodes
            .iter()
            .find_map(|(node_id, node)| (*node_id == id.accesskit_id()).then_some(node))
            .expect("rendered action response has an accessibility node")
    }

    fn assert_no_final_action_id_diagnostic(
        output: &egui::FullOutput,
        rect: egui::Rect,
    ) {
        assert!(!output.shapes.iter().any(|clipped| {
            matches!(
                &clipped.shape,
                egui::Shape::Rect(shape)
                    if shape.rect == rect
                        && shape.stroke == egui::Stroke::new(2.0, egui::Color32::RED)
                        && shape.stroke_kind == egui::StrokeKind::Outside
            )
        }), "final context output contains egui's 2-point red outside ID-change diagnostic at {rect:?}");
    }

    fn assert_complete_popup_pass(
        output: &egui::FullOutput,
        snapshot: &SprintTimerPopupRenderSnapshot,
        expected_ids: (egui::Id, egui::Id),
        expected_actions: (egui::Rect, egui::Rect),
        expected_geometry: PopupGeometryLedger,
        expected_label: &str,
        end_enabled: bool,
    ) {
        assert_same_action_ids(snapshot, expected_ids);
        assert_eq!(snapshot.primary_action, expected_actions.0);
        assert_eq!(snapshot.end_action, expected_actions.1);
        assert_eq!(snapshot.primary_label, expected_label);
        assert_eq!(snapshot.end_enabled, end_enabled);
        let geometry = snapshot.geometry;
        assert_eq!(snapshot.body, geometry.planned_body);
        assert_eq!(geometry.planned_body.size(), egui::vec2(300.0, 165.0));
        assert_eq!(geometry.planned_body, expected_geometry.planned_body);
        assert_eq!(geometry.after_display.content_min, geometry.planned_body);
        assert_eq!(geometry.primary_parent_after.content_min, geometry.planned_body);
        assert_eq!(geometry.end_parent_after.content_min, geometry.planned_body);
        assert_eq!(geometry.after_actions.content_min, geometry.planned_body);
        assert_eq!(geometry.frame_outer, expected_geometry.frame_outer);
        assert_eq!(geometry.area_response, expected_geometry.area_response);
        assert_eq!(
            geometry.next_pass_retained_size,
            expected_geometry.next_pass_retained_size
        );
        assert_eq!(geometry.frame_outer.size(), expected_geometry.frame_outer.size());
        assert_eq!(geometry.area_response.size(), expected_geometry.area_response.size());
        assert_eq!(
            geometry.next_pass_retained_size,
            Some(expected_geometry.area_response.size())
        );
        assert_eq!(geometry.area_response.size(), geometry.frame_outer.size());
        assert_eq!(geometry.primary_assigned, snapshot.primary_action);
        assert!(geometry.primary_assigned.contains_rect(geometry.primary_child_min));
        assert!(geometry.primary_assigned.contains_rect(geometry.primary_response));
        assert_eq!(geometry.end_assigned, snapshot.end_action);
        assert!(geometry.end_assigned.contains_rect(geometry.end_child_min));
        assert!(geometry.end_assigned.contains_rect(geometry.end_response));
        assert_eq!(geometry.primary_parent_before, geometry.primary_parent_after);
        assert_eq!(geometry.end_parent_before, geometry.end_parent_after);
        if let (Some(assigned), Some(child_min), Some(response)) = (
            geometry.editor_assigned,
            geometry.editor_child_min,
            geometry.editor_response,
        ) {
            assert!(assigned.contains_rect(child_min));
            assert!(assigned.contains_rect(response));
            assert_eq!(geometry.editor_parent_before, geometry.editor_parent_after);
        } else {
            assert!(geometry.editor_assigned.is_none());
            assert!(geometry.editor_child_min.is_none());
            assert!(geometry.editor_response.is_none());
        }
        assert_no_final_action_id_diagnostic(output, snapshot.primary_action);
        assert_no_final_action_id_diagnostic(output, snapshot.end_action);

        let primary = accesskit_action_node(output, snapshot.primary_id);
        assert_eq!(primary.role(), egui::accesskit::Role::Button);
        assert_eq!(primary.label(), Some(expected_label));
        assert!(!primary.is_disabled());
        let end = accesskit_action_node(output, snapshot.end_id);
        assert_eq!(end.role(), egui::accesskit::Role::Button);
        assert_eq!(end.label(), Some("End Sprint"));
        assert_eq!(end.is_disabled(), !end_enabled);
    }

    #[test]
    fn complete_popup_passes_keep_action_ids_and_retained_geometry_stable_across_modes_states_contexts_and_density() {
        let root_viewport = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(600.0, 400.0),
        );
        let title_viewport = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(360.0, 260.0),
        );

        for dark in [false, true] {
            for (viewport_name, viewport, pixels_per_point) in [
                ("root", root_viewport),
                ("title-page", title_viewport),
            ]
            .into_iter()
            .flat_map(|(name, viewport)| {
                [1.0, 2.0].map(move |pixels_per_point| (name, viewport, pixels_per_point))
            })
            {
                let ctx = egui::Context::default();
                ctx.enable_accesskit();
                ctx.set_pixels_per_point(pixels_per_point);
                ctx.style_mut(|style| style.debug.warn_if_rect_changes_id = true);
                ctx.set_visuals(if dark {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                });
                assert!(ctx.style().debug.warn_if_rect_changes_id);

                let anchor_center = egui::pos2(viewport.center().x, viewport.bottom() - 30.0);
                let mut timer = open_timer();
                timer.popup_anchor = egui::Rect::from_center_size(anchor_center, egui::vec2(20.0, 20.0));

                let (commands, rendered, output) = render_popup_frame_with_output_on_viewport(
                    &ctx,
                    &mut timer,
                    Duration::ZERO,
                    Vec::new(),
                    viewport,
                );
                assert!(commands.is_empty(), "{viewport_name}, dark={dark}");
                let initial = snapshot((commands, rendered));
                let expected_ids = (initial.primary_id, initial.end_id);
                let expected_actions = (initial.primary_action, initial.end_action);
                let expected_geometry = initial.geometry;
                assert_complete_popup_pass(
                    &output,
                    &initial,
                    expected_ids,
                    expected_actions,
                    expected_geometry,
                    "Start Sprint",
                    false,
                );
                assert_eq!(timer.primary_command(), Command::ToolsSprintTimerStart);

                // Keep keyboard focus on the primary action while changing the
                // mode-specific display allocation on either side of Stopwatch.
                ctx.memory_mut(|memory| memory.request_focus(expected_ids.0));
                let (commands, rendered, output) = render_popup_frame_with_output_on_viewport(
                    &ctx,
                    &mut timer,
                    Duration::ZERO,
                    Vec::new(),
                    viewport,
                );
                let focused_countdown = snapshot((commands.clone(), rendered));
                assert!(commands.is_empty());
                assert!(focused_countdown.primary_has_focus);
                assert_complete_popup_pass(
                    &output,
                    &focused_countdown,
                    expected_ids,
                    expected_actions,
                    expected_geometry,
                    "Start Sprint",
                    false,
                );

                assert!(timer.select_mode(TimerMode::Stopwatch));
                let (commands, rendered, output) = render_popup_frame_with_output_on_viewport(
                    &ctx,
                    &mut timer,
                    Duration::ZERO,
                    Vec::new(),
                    viewport,
                );
                let stopwatch_idle = snapshot((commands.clone(), rendered));
                assert!(commands.is_empty());
                assert!(stopwatch_idle.primary_has_focus);
                assert!(stopwatch_idle.duration_edit_id.is_none());
                assert_complete_popup_pass(
                    &output,
                    &stopwatch_idle,
                    expected_ids,
                    expected_actions,
                    expected_geometry,
                    "Start Sprint",
                    false,
                );

                assert!(timer.select_mode(TimerMode::Countdown));
                let (commands, rendered, output) = render_popup_frame_with_output_on_viewport(
                    &ctx,
                    &mut timer,
                    Duration::ZERO,
                    Vec::new(),
                    viewport,
                );
                let countdown_idle = snapshot((commands.clone(), rendered));
                assert!(commands.is_empty());
                assert!(countdown_idle.primary_has_focus);
                assert_eq!(countdown_idle.duration_edit_id, initial.duration_edit_id);
                assert_complete_popup_pass(
                    &output,
                    &countdown_idle,
                    expected_ids,
                    expected_actions,
                    expected_geometry,
                    "Start Sprint",
                    false,
                );

                assert!(timer.start_countdown(Duration::ZERO).is_none());
                let (commands, rendered, output) = render_popup_frame_with_output_on_viewport(
                    &ctx,
                    &mut timer,
                    Duration::ZERO,
                    Vec::new(),
                    viewport,
                );
                let countdown_running = snapshot((commands.clone(), rendered));
                assert!(commands.is_empty());
                assert!(countdown_running.primary_has_focus);
                assert_eq!(timer.primary_command(), Command::ToolsSprintTimerPause);
                assert!(countdown_running.end_enabled);
                assert_complete_popup_pass(
                    &output,
                    &countdown_running,
                    expected_ids,
                    expected_actions,
                    expected_geometry,
                    "Pause",
                    true,
                );

                let _ = timer.pause(Duration::from_secs(5));
                ctx.memory_mut(|memory| memory.request_focus(expected_ids.0));
                let (commands, rendered, output) = render_popup_frame_with_output_on_viewport(
                    &ctx,
                    &mut timer,
                    Duration::from_secs(5),
                    Vec::new(),
                    viewport,
                );
                let countdown_paused = snapshot((commands.clone(), rendered));
                assert!(commands.is_empty());
                assert!(countdown_paused.primary_has_focus);
                assert_eq!(timer.primary_command(), Command::ToolsSprintTimerResume);
                assert_complete_popup_pass(
                    &output,
                    &countdown_paused,
                    expected_ids,
                    expected_actions,
                    expected_geometry,
                    "Resume",
                    true,
                );

                // End Sprint also retains its stable focus identity while enabled.
                ctx.memory_mut(|memory| memory.request_focus(expected_ids.1));
                let (commands, rendered, output) = render_popup_frame_with_output_on_viewport(
                    &ctx,
                    &mut timer,
                    Duration::from_secs(5),
                    Vec::new(),
                    viewport,
                );
                let focused_end = snapshot((commands.clone(), rendered));
                assert!(commands.is_empty());
                assert!(focused_end.end_has_focus);
                assert_complete_popup_pass(
                    &output,
                    &focused_end,
                    expected_ids,
                    expected_actions,
                    expected_geometry,
                    "Resume",
                    true,
                );

                let _ = timer.resume(Duration::from_secs(5));
                let _ = timer.reconcile(Duration::from_secs(1_500));
                ctx.memory_mut(|memory| memory.request_focus(expected_ids.0));
                let (commands, rendered, output) = render_popup_frame_with_output_on_viewport(
                    &ctx,
                    &mut timer,
                    Duration::from_secs(1_500),
                    Vec::new(),
                    viewport,
                );
                let countdown_completed = snapshot((commands.clone(), rendered));
                assert!(commands.is_empty());
                assert!(countdown_completed.primary_has_focus);
                assert_eq!(timer.primary_command(), Command::ToolsSprintTimerReset);
                assert_complete_popup_pass(
                    &output,
                    &countdown_completed,
                    expected_ids,
                    expected_actions,
                    expected_geometry,
                    "Reset",
                    false,
                );

                timer.reset();
                let (commands, rendered, output) = render_popup_frame_with_output_on_viewport(
                    &ctx,
                    &mut timer,
                    Duration::ZERO,
                    Vec::new(),
                    viewport,
                );
                let reset_countdown = snapshot((commands.clone(), rendered));
                assert!(commands.is_empty());
                assert_eq!(timer.primary_command(), Command::ToolsSprintTimerStart);
                assert_complete_popup_pass(
                    &output,
                    &reset_countdown,
                    expected_ids,
                    expected_actions,
                    expected_geometry,
                    "Start Sprint",
                    false,
                );

                assert!(timer.select_mode(TimerMode::Stopwatch));
                let (commands, rendered, output) = render_popup_frame_with_output_on_viewport(
                    &ctx,
                    &mut timer,
                    Duration::ZERO,
                    Vec::new(),
                    viewport,
                );
                let stopwatch_idle = snapshot((commands.clone(), rendered));
                assert!(commands.is_empty());
                assert_eq!(timer.primary_command(), Command::ToolsSprintTimerStart);
                assert_complete_popup_pass(
                    &output,
                    &stopwatch_idle,
                    expected_ids,
                    expected_actions,
                    expected_geometry,
                    "Start Sprint",
                    false,
                );

                assert!(timer.start_stopwatch(Duration::ZERO).is_none());
                ctx.memory_mut(|memory| memory.request_focus(expected_ids.1));
                let (commands, rendered, output) = render_popup_frame_with_output_on_viewport(
                    &ctx,
                    &mut timer,
                    Duration::ZERO,
                    Vec::new(),
                    viewport,
                );
                let stopwatch_running = snapshot((commands.clone(), rendered));
                assert!(commands.is_empty());
                assert!(stopwatch_running.end_has_focus);
                assert_eq!(timer.primary_command(), Command::ToolsSprintTimerPause);
                assert_complete_popup_pass(
                    &output,
                    &stopwatch_running,
                    expected_ids,
                    expected_actions,
                    expected_geometry,
                    "Pause",
                    true,
                );

                let _ = timer.pause(Duration::from_secs(8));
                let (commands, rendered, output) = render_popup_frame_with_output_on_viewport(
                    &ctx,
                    &mut timer,
                    Duration::from_secs(8),
                    Vec::new(),
                    viewport,
                );
                let stopwatch_paused = snapshot((commands.clone(), rendered));
                assert!(commands.is_empty());
                assert!(stopwatch_paused.end_has_focus);
                assert_eq!(timer.primary_command(), Command::ToolsSprintTimerResume);
                assert_complete_popup_pass(
                    &output,
                    &stopwatch_paused,
                    expected_ids,
                    expected_actions,
                    expected_geometry,
                    "Resume",
                    true,
                );

                let _ = timer.end(Duration::from_secs(8));
                let (commands, rendered, output) = render_popup_frame_with_output_on_viewport(
                    &ctx,
                    &mut timer,
                    Duration::from_secs(8),
                    Vec::new(),
                    viewport,
                );
                let stopwatch_idle_again = snapshot((commands.clone(), rendered));
                assert!(commands.is_empty());
                assert_complete_popup_pass(
                    &output,
                    &stopwatch_idle_again,
                    expected_ids,
                    expected_actions,
                    expected_geometry,
                    "Start Sprint",
                    false,
                );

                assert!(timer.select_mode(TimerMode::Countdown));
                let (commands, rendered, output) = render_popup_frame_with_output_on_viewport(
                    &ctx,
                    &mut timer,
                    Duration::ZERO,
                    Vec::new(),
                    viewport,
                );
                let countdown_returned = snapshot((commands.clone(), rendered));
                assert!(commands.is_empty());
                assert_eq!(countdown_returned.duration_edit_id, initial.duration_edit_id);
                assert_complete_popup_pass(
                    &output,
                    &countdown_returned,
                    expected_ids,
                    expected_actions,
                    expected_geometry,
                    "Start Sprint",
                    false,
                );
            }
        }
    }

    #[test]
    fn primary_and_end_buttons_activate_once_from_keyboard_focus() {
        fn key_event(key: Key) -> egui::Event {
            egui::Event::Key {
                key,
                physical_key: Some(key),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }
        }

        let mut primary_states = Vec::new();
        let mut countdown_idle = open_timer();
        primary_states.push(countdown_idle.clone());

        let mut countdown_running = open_timer();
        assert!(countdown_running.start_countdown(Duration::ZERO).is_none());
        primary_states.push(countdown_running.clone());
        let _ = countdown_running.pause(Duration::from_secs(5));
        primary_states.push(countdown_running.clone());
        let _ = countdown_running.resume(Duration::from_secs(5));
        let _ = countdown_running.reconcile(Duration::from_secs(1_500));
        primary_states.push(countdown_running.clone());

        let mut stopwatch_idle = open_timer();
        assert!(stopwatch_idle.select_mode(TimerMode::Stopwatch));
        primary_states.push(stopwatch_idle.clone());
        let mut stopwatch_running = stopwatch_idle;
        assert!(stopwatch_running.start_stopwatch(Duration::ZERO).is_none());
        primary_states.push(stopwatch_running.clone());
        let _ = stopwatch_running.pause(Duration::from_secs(5));
        primary_states.push(stopwatch_running);

        for timer in primary_states {
            for key in [Key::Enter, Key::Space] {
                let ctx = egui::Context::default();
                let mut timer = timer.clone();
                let initial = render_passive_popup_snapshot(&ctx, &mut timer, Duration::ZERO);
                let expected = timer.primary_command();
                ctx.memory_mut(|memory| memory.request_focus(initial.primary_id));
                let (commands, rendered) = render_popup_frame(
                    &ctx,
                    &mut timer,
                    Duration::ZERO,
                    vec![key_event(key)],
                );
                assert_eq!(commands, vec![expected], "primary key {key:?}");
                assert!(snapshot((commands, rendered)).primary_clicked);
            }
        }

        let mut end_enabled_states = Vec::new();
        let mut countdown_running = open_timer();
        assert!(countdown_running.start_countdown(Duration::ZERO).is_none());
        end_enabled_states.push(countdown_running.clone());
        let _ = countdown_running.pause(Duration::from_secs(5));
        end_enabled_states.push(countdown_running);

        let mut stopwatch_running = open_timer();
        assert!(stopwatch_running.select_mode(TimerMode::Stopwatch));
        assert!(stopwatch_running.start_stopwatch(Duration::ZERO).is_none());
        end_enabled_states.push(stopwatch_running.clone());
        let _ = stopwatch_running.pause(Duration::from_secs(5));
        end_enabled_states.push(stopwatch_running);

        for timer in end_enabled_states {
            for key in [Key::Enter, Key::Space] {
                let ctx = egui::Context::default();
                let mut timer = timer.clone();
                let initial = render_passive_popup_snapshot(&ctx, &mut timer, Duration::ZERO);
                assert!(initial.end_enabled);
                ctx.memory_mut(|memory| memory.request_focus(initial.end_id));
                let (commands, rendered) = render_popup_frame(
                    &ctx,
                    &mut timer,
                    Duration::ZERO,
                    vec![key_event(key)],
                );
                assert_eq!(commands, vec![Command::ToolsSprintTimerEnd], "End Sprint key {key:?}");
                assert!(snapshot((commands, rendered)).end_clicked);
            }
        }

        for timer in [open_timer(), {
            let mut completed = open_timer();
            assert!(completed.start_countdown(Duration::ZERO).is_none());
            let _ = completed.reconcile(Duration::from_secs(1_500));
            completed
        }] {
            for key in [Key::Enter, Key::Space] {
                let ctx = egui::Context::default();
                let mut timer = timer.clone();
                let initial = render_passive_popup_snapshot(&ctx, &mut timer, Duration::ZERO);
                assert!(!initial.end_enabled);
                ctx.memory_mut(|memory| memory.request_focus(initial.end_id));
                let (commands, _) = render_popup_frame(
                    &ctx,
                    &mut timer,
                    Duration::ZERO,
                    vec![key_event(key)],
                );
                assert!(commands.is_empty(), "disabled End Sprint key {key:?}");
            }
        }
    }

    #[test]
    fn move_only_host_change_requests_close_before_popup_paint() {
        let ctx = egui::Context::default();
        let mut timer = open_timer();
        let captured = host_geometry(100.0, 120.0, 800.0, 600.0);
        let moved = host_geometry(101.0, 120.0, 800.0, 600.0);

        let (commands, rendered) =
            render_popup_frame_with_host_geometry(&ctx, &mut timer, captured, moved);

        assert_eq!(commands, vec![Command::ToolsSprintTimerClosePopup]);
        assert!(rendered.is_none());
    }

    #[test]
    fn resize_only_host_change_requests_close_before_popup_paint() {
        let ctx = egui::Context::default();
        let mut timer = open_timer();
        let captured = host_geometry(100.0, 120.0, 800.0, 600.0);
        let resized = host_geometry(100.0, 120.0, 801.0, 600.0);

        let (commands, rendered) =
            render_popup_frame_with_host_geometry(&ctx, &mut timer, captured, resized);

        assert_eq!(commands, vec![Command::ToolsSprintTimerClosePopup]);
        assert!(rendered.is_none());
    }

    #[test]
    fn simultaneous_host_move_and_resize_requests_one_close_before_paint() {
        let ctx = egui::Context::default();
        let mut timer = open_timer();
        let captured = host_geometry(100.0, 120.0, 800.0, 600.0);
        let changed = host_geometry(110.0, 130.0, 900.0, 700.0);

        let (commands, rendered) =
            render_popup_frame_with_host_geometry(&ctx, &mut timer, captured, changed);

        assert_eq!(commands, vec![Command::ToolsSprintTimerClosePopup]);
        assert!(rendered.is_none());
    }

    #[test]
    fn unchanged_host_geometry_keeps_popup_visible_across_frames() {
        let ctx = egui::Context::default();
        let mut timer = open_timer();
        let captured = host_geometry(100.0, 120.0, 800.0, 600.0);

        for _ in 0..2 {
            let (commands, rendered) = render_popup_frame_with_host_geometry(
                &ctx,
                &mut timer,
                captured,
                captured,
            );
            assert!(commands.is_empty());
            assert!(rendered.is_some());
        }
    }

    #[test]
    fn unknown_observations_do_not_move_baseline_and_returned_values_compare_original() {
        let captured = host_geometry(100.0, 120.0, 800.0, 600.0);
        let unknown = SprintTimerHostGeometry::default();
        let returned_changed = host_geometry(100.0, 120.0, 801.0, 600.0);
        let unknown_capture = SprintTimerHostGeometry {
            outer_left: None,
            outer_top: Some(120.0),
            inner_width: None,
            inner_height: Some(600.0),
        };
        let now_known = host_geometry(999.0, 120.0, 500.0, 600.0);

        assert!(!unknown.changed_from(captured));
        assert!(returned_changed.changed_from(captured));
        assert!(!now_known.changed_from(unknown_capture));
    }

    #[test]
    fn body_and_title_page_compare_only_their_own_host_snapshots() {
        let root_ctx = egui::Context::default();
        let title_ctx = egui::Context::default();
        let root_capture = host_geometry(100.0, 120.0, 800.0, 600.0);
        let title_capture = host_geometry(500.0, 300.0, 420.0, 320.0);
        let root_current = host_geometry(101.0, 120.0, 800.0, 600.0);
        let mut root_timer = open_timer();
        let mut title_timer = open_timer();

        let (root_commands, root_rendered) = render_popup_frame_with_host_geometry(
            &root_ctx,
            &mut root_timer,
            root_capture,
            root_current,
        );
        let (title_commands, title_rendered) = render_popup_frame_with_host_geometry(
            &title_ctx,
            &mut title_timer,
            title_capture,
            title_capture,
        );

        assert_eq!(root_commands, vec![Command::ToolsSprintTimerClosePopup]);
        assert!(root_rendered.is_none());
        assert!(title_commands.is_empty());
        assert!(title_rendered.is_some());
    }

    #[test]
    fn rendered_popup_keeps_actual_anchor_and_notch_geometry_for_every_open_frame() {
        let ctx = egui::Context::default();
        let mut timer = open_timer();
        let anchor = timer.popup_anchor;
        let first = snapshot(render_popup_frame(&ctx, &mut timer, Duration::ZERO, Vec::new()));
        let second = snapshot(render_popup_frame(&ctx, &mut timer, Duration::ZERO, Vec::new()));

        assert_eq!(timer.popup_anchor, anchor);
        assert_eq!(first.body, second.body);
        assert_eq!(first.notch, second.notch);
        assert_eq!(first.popup, second.popup);
        assert!(first.notch_visible);
        assert!(first.popup.contains(first.notch.center()));
        assert_eq!(first.body.bottom(), first.notch.top());
        assert!(first.notch_tip.x >= first.body.left() + POPUP_CORNER_RADIUS + POPUP_NOTCH_WIDTH / 2.0);
        assert!(first.notch_tip.x <= first.body.right() - POPUP_CORNER_RADIUS - POPUP_NOTCH_WIDTH / 2.0);
        assert_eq!(timer.popup_rect, first.popup);
    }

    #[test]
    fn root_and_title_page_popup_renderers_use_their_own_viewport_local_anchors() {
        // Both viewport callers enter through show_popup; each gets its own
        // context and passes its status response in that viewport's coordinates.
        let root_ctx = egui::Context::default();
        let title_ctx = egui::Context::default();
        let root_viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0));
        let title_viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(320.0, 240.0));
        let mut root_timer = open_timer();
        let mut title_timer = open_timer();
        root_timer.popup_anchor = egui::Rect::from_min_max(
            egui::pos2(280.0, 360.0),
            egui::pos2(300.0, 380.0),
        );
        title_timer.popup_anchor = egui::Rect::from_min_max(
            egui::pos2(140.0, 210.0),
            egui::pos2(160.0, 230.0),
        );

        let (root, _) = render_settled_popup_geometry(
            &root_ctx,
            &mut root_timer,
            Duration::ZERO,
            root_viewport,
        );
        let (title, _) = render_settled_popup_geometry(
            &title_ctx,
            &mut title_timer,
            Duration::ZERO,
            title_viewport,
        );

        assert!(root.notch_visible && title.notch_visible);
        assert_eq!(root.geometry.next_pass_retained_size, Some(root.geometry.area_response.size()));
        assert_eq!(title.geometry.next_pass_retained_size, Some(title.geometry.area_response.size()));
        assert_eq!(root.geometry.planned_body.size(), egui::vec2(300.0, 165.0));
        assert_eq!(title.geometry.planned_body.size(), egui::vec2(300.0, 165.0));
        assert_eq!(root.body.center().x, 290.0);
        assert_eq!(title.body.center().x, 150.0);
        assert_eq!(root.notch_tip, egui::pos2(290.0, 358.0));
        assert_eq!(title.notch_tip, egui::pos2(150.0, 208.0));
        assert!(root_viewport.contains(root.body.min));
        assert!(title_viewport.contains(title.body.min));
        assert_eq!(root_timer.popup_anchor.center(), egui::pos2(290.0, 370.0));
        assert_eq!(title_timer.popup_anchor.center(), egui::pos2(150.0, 220.0));
    }

    #[test]
    fn outside_release_in_notch_is_inside_and_outside_release_dismisses() {
        let ctx = egui::Context::default();
        let mut timer = open_timer();
        let rendered = snapshot(render_popup_frame(&ctx, &mut timer, Duration::ZERO, Vec::new()));
        let notch_point = egui::pos2(rendered.notch_tip.x, rendered.notch_tip.y - 1.0);
        assert!(timer.popup_rect.contains(notch_point));
        let (inside_commands, _) = click_popup_point(&ctx, &mut timer, Duration::ZERO, notch_point);
        assert!(!inside_commands.contains(&Command::ToolsSprintTimerClosePopup));
        assert!(timer.popup_open);

        let outside = egui::pos2(590.0, 20.0);
        let (outside_commands, _) = click_popup_point(&ctx, &mut timer, Duration::ZERO, outside);
        assert!(outside_commands.contains(&Command::ToolsSprintTimerClosePopup));
    }

    #[test]
    fn first_visible_outside_release_is_ignored_but_later_release_dismisses() {
        let ctx = egui::Context::default();
        let mut timer = open_timer();
        let outside = egui::pos2(590.0, 20.0);

        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(600.0, 400.0),
                )),
                events: vec![egui::Event::PointerButton {
                    pos: outside,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |_| {},
        );

        let (first_commands, first_render) = render_popup_frame(
            &ctx,
            &mut timer,
            Duration::ZERO,
            release_at(outside),
        );
        assert!(first_render.is_some());
        assert!(timer.popup_open);
        assert!(!first_commands.contains(&Command::ToolsSprintTimerClosePopup));

        let (later_commands, later_render) = click_popup_point(&ctx, &mut timer, Duration::ZERO, outside);
        assert!(later_render.is_none());
        assert!(later_commands.contains(&Command::ToolsSprintTimerClosePopup));
    }

    #[test]
    fn escape_dismisses_on_first_and_later_visible_frames() {
        for just_opened in [true, false] {
            let ctx = egui::Context::default();
            let mut timer = open_timer();
            timer.just_opened = just_opened;
            let (commands, rendered) = render_popup_frame(
                &ctx,
                &mut timer,
                Duration::ZERO,
                vec![egui::Event::Key {
                    key: Key::Escape,
                    physical_key: Some(Key::Escape),
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert!(rendered.is_none());
            assert!(commands.contains(&Command::ToolsSprintTimerClosePopup));
        }
    }

    #[test]
    fn rendered_countdown_and_stopwatch_share_toggle_and_action_rectangles() {
        let countdown_ctx = egui::Context::default();
        let stopwatch_ctx = egui::Context::default();
        let mut countdown = open_timer();
        let mut stopwatch = open_timer();
        assert!(stopwatch.select_mode(TimerMode::Stopwatch));

        let countdown_render = snapshot(render_popup_frame(
            &countdown_ctx,
            &mut countdown,
            Duration::ZERO,
            Vec::new(),
        ));
        let stopwatch_render = snapshot(render_popup_frame(
            &stopwatch_ctx,
            &mut stopwatch,
            Duration::ZERO,
            Vec::new(),
        ));

        assert_same_fixed_tracks(&countdown_render, &stopwatch_render);
        assert_eq!(countdown_render.display, stopwatch_render.display);
        assert_eq!(countdown_render.primary_label, "Start Sprint");
        assert_eq!(stopwatch_render.primary_label, "Start Sprint");
        assert_eq!(
            countdown.primary_command(),
            Command::ToolsSprintTimerStart
        );
        assert_eq!(
            stopwatch.primary_command(),
            Command::ToolsSprintTimerStart
        );
    }

    #[test]
    fn action_ids_stay_stable_across_modes_and_timer_states() {
        let ctx = egui::Context::default();
        let mut timer = open_timer();
        let initial = render_passive_popup_snapshot(&ctx, &mut timer, Duration::ZERO);
        let expected_ids = (initial.primary_id, initial.end_id);
        let editor_id = initial
            .duration_edit_id
            .expect("idle Countdown retains its explicitly named minute editor");
        assert_same_action_ids(&initial, expected_ids);

        assert!(timer.select_mode(TimerMode::Stopwatch));
        let stopwatch_idle = render_passive_popup_snapshot(&ctx, &mut timer, Duration::ZERO);
        assert_same_action_ids(&stopwatch_idle, expected_ids);
        assert_same_fixed_tracks(&initial, &stopwatch_idle);
        assert_eq!(stopwatch_idle.primary_label, "Start Sprint");
        assert!(!stopwatch_idle.end_enabled);

        assert!(timer.select_mode(TimerMode::Countdown));
        let countdown_idle = render_passive_popup_snapshot(&ctx, &mut timer, Duration::ZERO);
        assert_same_action_ids(&countdown_idle, expected_ids);
        assert_same_fixed_tracks(&initial, &countdown_idle);
        assert_eq!(countdown_idle.duration_edit_id, Some(editor_id));

        assert!(timer.start_countdown(Duration::ZERO).is_none());
        let countdown_running = render_passive_popup_snapshot(&ctx, &mut timer, Duration::ZERO);
        assert_same_action_ids(&countdown_running, expected_ids);
        assert_same_fixed_tracks(&initial, &countdown_running);
        assert_eq!(countdown_running.primary_label, "Pause");
        assert!(countdown_running.end_enabled);

        let _ = timer.pause(Duration::from_secs(5));
        let countdown_paused =
            render_passive_popup_snapshot(&ctx, &mut timer, Duration::from_secs(5));
        assert_same_action_ids(&countdown_paused, expected_ids);
        assert_eq!(countdown_paused.primary_label, "Resume");
        assert!(countdown_paused.end_enabled);

        let _ = timer.resume(Duration::from_secs(5));
        let _ = timer.reconcile(Duration::from_secs(1_500));
        let countdown_completed =
            render_passive_popup_snapshot(&ctx, &mut timer, Duration::from_secs(1_500));
        assert_same_action_ids(&countdown_completed, expected_ids);
        assert_same_fixed_tracks(&initial, &countdown_completed);
        assert_eq!(countdown_completed.primary_label, "Reset");
        assert!(!countdown_completed.end_enabled);

        timer.reset();
        let _ = render_passive_popup_snapshot(&ctx, &mut timer, Duration::ZERO);
        assert!(timer.select_mode(TimerMode::Stopwatch));
        let stopwatch_idle = render_passive_popup_snapshot(&ctx, &mut timer, Duration::ZERO);
        assert_same_action_ids(&stopwatch_idle, expected_ids);

        assert!(timer.start_stopwatch(Duration::ZERO).is_none());
        let stopwatch_running = render_passive_popup_snapshot(&ctx, &mut timer, Duration::ZERO);
        assert_same_action_ids(&stopwatch_running, expected_ids);
        assert_eq!(stopwatch_running.primary_label, "Pause");
        assert!(stopwatch_running.end_enabled);

        let _ = timer.pause(Duration::from_secs(8));
        let stopwatch_paused =
            render_passive_popup_snapshot(&ctx, &mut timer, Duration::from_secs(8));
        assert_same_action_ids(&stopwatch_paused, expected_ids);
        assert_eq!(stopwatch_paused.primary_label, "Resume");
        assert!(stopwatch_paused.end_enabled);

        let _ = timer.resume(Duration::from_secs(8));
        let stopwatch_resumed =
            render_passive_popup_snapshot(&ctx, &mut timer, Duration::from_secs(8));
        assert_same_action_ids(&stopwatch_resumed, expected_ids);
        assert_same_fixed_tracks(&initial, &stopwatch_resumed);
        assert_eq!(stopwatch_resumed.primary_label, "Pause");
        assert!(stopwatch_resumed.end_enabled);
    }

    #[test]
    fn action_accessibility_names_and_enablement_match_rendered_controls() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut timer = open_timer();
        let (commands, rendered, output) = render_popup_frame_with_output(
            &ctx,
            &mut timer,
            Duration::ZERO,
            Vec::new(),
        );
        let idle = snapshot((commands.clone(), rendered));

        assert!(commands.is_empty());
        assert_eq!(idle.primary_label, "Start Sprint");
        assert!(!idle.end_enabled);
        let primary = accesskit_action_node(&output, idle.primary_id);
        let end = accesskit_action_node(&output, idle.end_id);
        assert_eq!(primary.role(), egui::accesskit::Role::Button);
        assert_eq!(primary.label(), Some("Start Sprint"));
        assert!(!primary.is_disabled());
        assert_eq!(end.role(), egui::accesskit::Role::Button);
        assert_eq!(end.label(), Some("End Sprint"));
        assert!(end.is_disabled());

        assert!(timer.start_countdown(Duration::ZERO).is_none());
        let (commands, rendered, output) = render_popup_frame_with_output(
            &ctx,
            &mut timer,
            Duration::ZERO,
            Vec::new(),
        );
        let running = snapshot((commands.clone(), rendered));
        assert!(commands.is_empty());
        assert_same_action_ids(&running, (idle.primary_id, idle.end_id));
        assert_same_fixed_tracks(&idle, &running);
        assert_eq!(running.primary_label, "Pause");
        assert!(running.end_enabled);
        assert_eq!(
            accesskit_action_node(&output, running.primary_id).label(),
            Some("Pause")
        );
        let end = accesskit_action_node(&output, running.end_id);
        assert_eq!(end.label(), Some("End Sprint"));
        assert!(!end.is_disabled());

        let _ = timer.reconcile(Duration::from_secs(1_500));
        let (commands, rendered, output) = render_popup_frame_with_output(
            &ctx,
            &mut timer,
            Duration::from_secs(1_500),
            Vec::new(),
        );
        let completed = snapshot((commands.clone(), rendered));
        assert!(commands.is_empty());
        assert_same_action_ids(&completed, (idle.primary_id, idle.end_id));
        assert_same_fixed_tracks(&idle, &completed);
        assert_eq!(completed.primary_label, "Reset");
        assert!(!completed.end_enabled);
        assert_eq!(
            accesskit_action_node(&output, completed.primary_id).label(),
            Some("Reset")
        );
        assert!(accesskit_action_node(&output, completed.end_id).is_disabled());
    }

    #[test]
    fn rendered_action_text_galleys_are_centered_for_all_labels_and_enabled_states() {
        let mut cases = vec![(open_timer(), Duration::ZERO)];

        let mut running = open_timer();
        assert!(running.start_countdown(Duration::ZERO).is_none());
        cases.push((running, Duration::ZERO));

        let mut paused = open_timer();
        assert!(paused.start_countdown(Duration::ZERO).is_none());
        let _ = paused.pause(Duration::ZERO);
        cases.push((paused, Duration::ZERO));

        let mut completed = open_timer();
        assert!(completed.start_countdown(Duration::ZERO).is_none());
        let _ = completed.reconcile(Duration::from_secs(1_500));
        cases.push((completed, Duration::from_secs(1_500)));

        for (mut timer, now) in cases {
            let ctx = egui::Context::default();
            let (commands, rendered, output) =
                render_popup_frame_with_output(&ctx, &mut timer, now, Vec::new());
            assert!(commands.is_empty());
            let rendered = snapshot((commands, rendered));

            for (label, action_rect, enabled) in [
                (rendered.primary_label, rendered.primary_action, true),
                ("End Sprint", rendered.end_action, rendered.end_enabled),
            ] {
                let text_shape = output.shapes.iter().find_map(|clipped| {
                    if let egui::Shape::Text(text) = &clipped.shape {
                        (text.galley.job.text == label).then_some(text)
                    } else {
                        None
                    }
                });
                assert!(text_shape.is_some(), "missing rendered galley for {label:?}");
                if let Some(text) = text_shape {
                    let galley_rect = egui::Rect::from_min_size(text.pos, text.galley.rect.size());
                    assert!(
                        (galley_rect.center().x - action_rect.center().x).abs() <= 0.5,
                        "{label:?} horizontal galley center {:?} differs from button center {:?} (enabled={enabled})",
                        galley_rect.center(),
                        action_rect.center(),
                    );
                    assert!(
                        (galley_rect.center().y - action_rect.center().y).abs() <= 0.5,
                        "{label:?} vertical galley center {:?} differs from button center {:?} (enabled={enabled})",
                        galley_rect.center(),
                        action_rect.center(),
                    );
                }
            }
        }
    }

    #[test]
    fn end_action_focus_and_id_survive_running_pause_resume_transitions() {
        let ctx = egui::Context::default();
        let mut timer = open_timer();
        assert!(timer.start_countdown(Duration::ZERO).is_none());
        let initial = render_passive_popup_snapshot(&ctx, &mut timer, Duration::ZERO);
        let ids = (initial.primary_id, initial.end_id);
        assert!(initial.end_enabled);

        let (commands, focused) = click_popup_point(
            &ctx,
            &mut timer,
            Duration::ZERO,
            initial.end_action.center(),
        );
        let focused = snapshot((commands.clone(), focused));
        assert_eq!(commands, vec![Command::ToolsSprintTimerEnd]);
        assert!(focused.end_has_focus);
        assert_same_action_ids(&focused, ids);

        let _ = timer.pause(Duration::from_secs(1));
        let paused = render_passive_popup_snapshot(&ctx, &mut timer, Duration::from_secs(1));
        assert_same_action_ids(&paused, ids);
        assert!(paused.end_enabled);
        assert!(paused.end_has_focus);

        let _ = timer.resume(Duration::from_secs(1));
        let resumed = render_passive_popup_snapshot(&ctx, &mut timer, Duration::from_secs(1));
        assert_same_action_ids(&resumed, ids);
        assert!(resumed.end_enabled);
        assert!(resumed.end_has_focus);
        assert_same_fixed_tracks(&initial, &resumed);
    }

    #[test]
    fn rendered_minute_editor_is_in_the_display_stage_and_enter_uses_typed_commands() {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut timer = open_timer();
        let (commands, rendered, output) = render_popup_frame_with_output(
            &ctx,
            &mut timer,
            Duration::ZERO,
            Vec::new(),
        );
        assert!(commands.is_empty());
        let initial = snapshot((commands, rendered));
        let edit_rect = initial
            .duration_edit_rect
            .expect("idle Countdown renders its minute editor");
        assert!(initial.display.contains_rect(edit_rect));
        assert_eq!(
            initial.duration_edit_id,
            Some(egui::Id::new("sprint-timer-minutes"))
        );
        assert_eq!(
            initial.duration_accessible_name,
            Some("Sprint duration in minutes")
        );
        assert_eq!(initial.geometry.editor_response, Some(edit_rect));
        assert_eq!(initial.geometry.editor_assigned, Some(edit_rect));
        assert!(initial
            .duration_caption_rect
            .is_some_and(|caption| initial.display.contains_rect(caption)));
        let caption_node = output
            .platform_output
            .accesskit_update
            .as_ref()
            .expect("enabled accessibility produces a tree update")
            .nodes
            .iter()
            .find_map(|(_, node)| {
                (node.role() == egui::accesskit::Role::Label
                    && node.label() == Some("Minutes"))
                .then_some(node)
            })
            .expect("the idle duration caption remains accessible");
        assert_eq!(caption_node.label(), Some("Minutes"));

        let _ = click_popup_point(&ctx, &mut timer, Duration::ZERO, edit_rect.center());
        let (commands, _) = render_popup_frame(
            &ctx,
            &mut timer,
            Duration::ZERO,
            vec![
                egui::Event::Text("x".to_owned()),
                egui::Event::Key {
                    key: Key::Enter,
                    physical_key: Some(Key::Enter),
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        assert!(commands.iter().any(|command| matches!(
            command,
            Command::ToolsSprintTimerSetMinutes { .. }
        )));
        assert!(commands.contains(&Command::ToolsSprintTimerStart));

        timer.set_minutes_input("2x5".to_owned());
        assert_eq!(timer.minutes_input, "25");
        timer.set_minutes_input("0".to_owned());
        assert!(timer.start_countdown(Duration::ZERO).is_none());
        assert!(timer.validation_error.is_some());
    }

    #[test]
    fn geometry_ledger_is_fixed_across_modes_states_contexts_and_density() {
        let viewports = [
            (
                egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0)),
                1.0,
            ),
            (
                egui::Rect::from_min_size(egui::pos2(120.0, 80.0), egui::vec2(600.0, 400.0)),
                1.0,
            ),
            (
                egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0)),
                2.0,
            ),
        ];

        for (viewport, pixels_per_point) in viewports {
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(pixels_per_point);
            let mut timer = open_timer();
            timer.popup_anchor = egui::Rect::from_center_size(
                egui::pos2(viewport.center().x, viewport.bottom() - 20.0),
                egui::vec2(20.0, 20.0),
            );
            let mut frames = Vec::new();

            frames.push(render_settled_popup_geometry(
                &ctx,
                &mut timer,
                Duration::ZERO,
                viewport,
            )); // idle Countdown
            assert!(timer.select_mode(TimerMode::Stopwatch));
            frames.push(render_settled_popup_geometry(
                &ctx,
                &mut timer,
                Duration::ZERO,
                viewport,
            )); // idle Stopwatch
            assert!(timer.select_mode(TimerMode::Countdown));
            frames.push(render_settled_popup_geometry(
                &ctx,
                &mut timer,
                Duration::ZERO,
                viewport,
            )); // idle Countdown after alternation
            assert!(timer.select_mode(TimerMode::Stopwatch));
            let _ = timer.start_stopwatch(Duration::ZERO);
            frames.push(render_settled_popup_geometry(
                &ctx,
                &mut timer,
                Duration::ZERO,
                viewport,
            )); // running Stopwatch
            let _ = timer.pause(Duration::from_secs(5));
            frames.push(render_settled_popup_geometry(
                &ctx,
                &mut timer,
                Duration::from_secs(5),
                viewport,
            )); // paused Stopwatch
            let _ = timer.resume(Duration::from_secs(5));
            frames.push(render_settled_popup_geometry(
                &ctx,
                &mut timer,
                Duration::from_secs(6),
                viewport,
            )); // resumed Stopwatch
            let _ = timer.end(Duration::from_secs(7));
            frames.push(render_settled_popup_geometry(
                &ctx,
                &mut timer,
                Duration::from_secs(7),
                viewport,
            )); // idle Stopwatch after end
            assert!(timer.select_mode(TimerMode::Countdown));
            timer.set_minutes_input("0".to_owned());
            let _ = timer.start_countdown(Duration::ZERO);
            frames.push(render_settled_popup_geometry(
                &ctx,
                &mut timer,
                Duration::ZERO,
                viewport,
            )); // idle Countdown with validation feedback
            timer.set_minutes_input("1".to_owned());
            let _ = timer.start_countdown(Duration::ZERO);
            frames.push(render_settled_popup_geometry(
                &ctx,
                &mut timer,
                Duration::ZERO,
                viewport,
            )); // running Countdown
            let _ = timer.pause(Duration::from_secs(15));
            frames.push(render_settled_popup_geometry(
                &ctx,
                &mut timer,
                Duration::from_secs(15),
                viewport,
            )); // paused Countdown
            let _ = timer.resume(Duration::from_secs(20));
            frames.push(render_settled_popup_geometry(
                &ctx,
                &mut timer,
                Duration::from_secs(21),
                viewport,
            )); // resumed Countdown
            let _ = timer.reconcile(Duration::from_secs(60));
            frames.push(render_settled_popup_geometry(
                &ctx,
                &mut timer,
                Duration::from_secs(60),
                viewport,
            )); // completed Countdown
            assert!(timer.apply_notification_result(1, NotificationResult::Pending));
            frames.push(render_settled_popup_geometry(
                &ctx,
                &mut timer,
                Duration::from_secs(60),
                viewport,
            )); // completed with pending notification feedback
            assert!(timer.apply_notification_result(1, NotificationResult::Accepted));
            frames.push(render_settled_popup_geometry(
                &ctx,
                &mut timer,
                Duration::from_secs(60),
                viewport,
            )); // completed with accepted notification feedback
            assert!(timer.apply_notification_result(1, NotificationResult::Unavailable));
            frames.push(render_settled_popup_geometry(
                &ctx,
                &mut timer,
                Duration::from_secs(60),
                viewport,
            )); // completed with unavailable notification feedback
            timer.reset();
            frames.push(render_settled_popup_geometry(
                &ctx,
                &mut timer,
                Duration::from_secs(60),
                viewport,
            )); // reset to idle Countdown

            let baseline = &frames[0].0;
            let expected_ids = (baseline.primary_id, baseline.end_id);
            let expected_frame_size = baseline.geometry.frame_outer.size();
            let expected_area_size = baseline.geometry.area_response.size();
            for (snapshot, output) in &frames {
                let ledger = snapshot.geometry;
                assert_eq!(ledger.planned_body, snapshot.body);
                assert_eq!(ledger.planned_body.size(), egui::vec2(300.0, 165.0));
                assert_eq!(ledger.after_display.content_min, ledger.planned_body);
                assert_eq!(ledger.primary_parent_after.content_min, ledger.planned_body);
                assert_eq!(ledger.end_parent_after.content_min, ledger.planned_body);
                assert_eq!(ledger.after_actions.content_min, ledger.planned_body);
                assert_eq!(ledger.primary_assigned, snapshot.primary_action);
                assert!(ledger.primary_assigned.contains_rect(ledger.primary_child_min));
                assert!(ledger.primary_assigned.contains_rect(ledger.primary_response));
                assert_eq!(ledger.end_assigned, snapshot.end_action);
                assert!(ledger.end_assigned.contains_rect(ledger.end_child_min));
                assert!(ledger.end_assigned.contains_rect(ledger.end_response));
                assert!(ledger.frame_outer.contains_rect(ledger.planned_body));
                assert_eq!(ledger.frame_outer.size(), expected_frame_size);
                assert_eq!(ledger.area_response.size(), expected_area_size);
                assert_eq!(ledger.area_response.size(), ledger.frame_outer.size());
                assert_eq!(
                    ledger.next_pass_retained_size,
                    Some(ledger.area_response.size())
                );
                assert_eq!(ledger.next_pass_retained_size, Some(expected_area_size));

                assert_eq!(ledger.primary_parent_before, ledger.primary_parent_after);
                assert_eq!(ledger.end_parent_before, ledger.end_parent_after);
                assert_eq!(ledger.primary_parent_after, ledger.end_parent_before);
                assert_same_fixed_tracks(snapshot, baseline);
                assert_same_action_ids(snapshot, expected_ids);
                assert_no_final_action_id_diagnostic(output, snapshot.primary_action);
                assert_no_final_action_id_diagnostic(output, snapshot.end_action);

                if let (Some(assigned), Some(child_min), Some(response)) = (
                    ledger.editor_assigned,
                    ledger.editor_child_min,
                    ledger.editor_response,
                ) {
                    assert!(assigned.contains_rect(child_min));
                    assert!(assigned.contains_rect(response));
                    assert_eq!(snapshot.duration_edit_id, Some(egui::Id::new("sprint-timer-minutes")));
                    assert_eq!(ledger.editor_parent_before, ledger.editor_parent_after);
                } else {
                    assert!(ledger.editor_assigned.is_none());
                    assert!(ledger.editor_child_min.is_none());
                    assert!(ledger.editor_response.is_none());
                    assert!(snapshot.duration_edit_id.is_none());
                }
            }
        }
    }

    #[test]
    fn idle_countdown_editor_uses_popup_metrics_and_keeps_its_reference_frame() {
        for dark in [false, true] {
            let ctx = egui::Context::default();
            ctx.set_visuals(if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
            let mut timer = open_timer();
            let (commands, rendered, output) = render_popup_frame_with_output(
                &ctx,
                &mut timer,
                Duration::ZERO,
                Vec::new(),
            );
            let rendered = snapshot((commands.clone(), rendered));
            let editor_rect = rendered
                .duration_edit_rect
                .expect("idle Countdown retains its framed minute editor");

            assert!(commands.is_empty());
            assert_eq!(
                editor_rect.size(),
                egui::vec2(
                    POPUP_COUNTDOWN_EDITOR_WIDTH,
                    POPUP_COUNTDOWN_EDITOR_HEIGHT,
                )
            );
            assert_eq!(POPUP_COUNTDOWN_EDITOR_FONT_SIZE, 23.0);
            assert!(rendered.display.contains_rect(editor_rect));
            assert!(output.shapes.iter().any(|clipped| matches!(
                &clipped.shape,
                egui::Shape::Rect(shape) if shape.rect == editor_rect
            )), "the idle editor keeps its input frame in either appearance");
        }
    }

    #[test]
    fn popup_palette_resolvers_use_day_night_defaults_and_optional_overrides() {
        let override_color = egui::Color32::from_rgb(191, 37, 149);
        for visuals in [egui::Visuals::light(), egui::Visuals::dark()] {
            assert_eq!(
                resolve_popup_focus_color(&visuals, None),
                visuals.hyperlink_color
            );
            assert_eq!(
                resolve_popup_display_background(&visuals, None),
                visuals.widgets.inactive.bg_fill
            );
            assert_eq!(
                resolve_popup_display_text(&visuals, None),
                visuals.text_color()
            );
            assert_eq!(
                resolve_popup_focus_color(&visuals, Some(override_color)),
                override_color
            );
            assert_eq!(
                resolve_popup_display_background(&visuals, Some(override_color)),
                override_color
            );
            assert_eq!(
                resolve_popup_display_text(&visuals, Some(override_color)),
                override_color
            );
        }
        assert_eq!(POPUP_FOCUS_STROKE_WIDTH, 2.0);
        assert_eq!(POPUP_FOCUS_COLOR_OVERRIDE, None);
        assert_eq!(POPUP_DISPLAY_BACKGROUND_OVERRIDE, None);
        assert_eq!(POPUP_DISPLAY_TEXT_OVERRIDE, None);
    }

    #[test]
    fn popup_content_scope_replaces_red_global_selection_without_leaking_style() {
        let ctx = egui::Context::default();
        let accent = egui::Color32::from_rgb(20, 170, 90);
        let red_global_stroke = egui::Stroke::new(7.0, egui::Color32::RED);
        let mut visuals = egui::Visuals::dark();
        visuals.hyperlink_color = accent;
        visuals.selection.stroke = red_global_stroke;
        visuals.widgets.active.bg_stroke = red_global_stroke;
        let original_hover_stroke = visuals.widgets.hovered.bg_stroke;
        let original_disabled_stroke = visuals.widgets.noninteractive.bg_stroke;
        ctx.set_visuals(visuals);

        let mut scoped_stroke = egui::Stroke::NONE;
        let mut scoped_active_stroke = egui::Stroke::NONE;
        let mut scoped_hover_stroke = egui::Stroke::NONE;
        let mut scoped_disabled_stroke = egui::Stroke::NONE;
        let mut parent_stroke = egui::Stroke::NONE;
        let mut parent_active_stroke = egui::Stroke::NONE;
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.scope(|ui| {
                    apply_popup_content_style(ui);
                    scoped_stroke = ui.visuals().selection.stroke;
                    scoped_active_stroke = ui.visuals().widgets.active.bg_stroke;
                    scoped_hover_stroke = ui.visuals().widgets.hovered.bg_stroke;
                    scoped_disabled_stroke = ui.visuals().widgets.noninteractive.bg_stroke;
                });
                parent_stroke = ui.visuals().selection.stroke;
                parent_active_stroke = ui.visuals().widgets.active.bg_stroke;
            });
        });

        assert_eq!(scoped_stroke, egui::Stroke::new(2.0, accent));
        assert_ne!(scoped_stroke.color, egui::Color32::RED);
        assert_eq!(scoped_active_stroke, egui::Stroke::new(2.0, accent));
        assert_eq!(scoped_hover_stroke, original_hover_stroke);
        assert_eq!(scoped_disabled_stroke, original_disabled_stroke);
        assert_eq!(parent_stroke, red_global_stroke);
        assert_eq!(parent_active_stroke, red_global_stroke);
        assert_eq!(ctx.style().visuals.selection.stroke, red_global_stroke);
        assert_eq!(ctx.style().visuals.widgets.active.bg_stroke, red_global_stroke);
    }

    fn assert_action_frame_stroke(
        output: &egui::FullOutput,
        rect: egui::Rect,
        expected: egui::Stroke,
    ) {
        let frame = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape) if shape.rect == rect => Some(shape),
                _ => None,
            })
            .expect("rendered action button has a frame shape at its action rectangle");
        assert_eq!(frame.stroke, expected);
    }

    #[test]
    fn rendered_primary_action_focus_survives_mode_transition_without_red_active_stroke() {
        let accent = egui::Color32::from_rgb(20, 170, 90);
        let red_active_stroke = egui::Stroke::new(9.0, egui::Color32::RED);

        for dark in [false, true] {
            let ctx = egui::Context::default();
            let mut visuals = if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            };
            visuals.hyperlink_color = accent;
            visuals.widgets.active.bg_stroke = red_active_stroke;
            let inactive_stroke = visuals.widgets.inactive.bg_stroke;
            let hovered_stroke = visuals.widgets.hovered.bg_stroke;
            let disabled_stroke = visuals.widgets.noninteractive.bg_stroke;
            ctx.set_visuals(visuals);

            let mut timer = open_timer();
            let (_, initial, initial_output) = render_popup_frame_with_output(
                &ctx,
                &mut timer,
                Duration::ZERO,
                Vec::new(),
            );
            let initial = snapshot((Vec::new(), initial));
            assert!(!initial.primary_active);
            assert_action_frame_stroke(&initial_output, initial.primary_action, inactive_stroke);

            let (hover_commands, hovered, hovered_output) = render_popup_frame_with_output(
                &ctx,
                &mut timer,
                Duration::ZERO,
                vec![egui::Event::PointerMoved(initial.primary_action.center())],
            );
            let hovered = snapshot((hover_commands.clone(), hovered));
            assert!(hover_commands.is_empty());
            assert!(!hovered.primary_active);
            assert_action_frame_stroke(&hovered_output, hovered.primary_action, hovered_stroke);

            let press_point = initial.primary_action.center();
            let (press_commands, pressed, pressed_output) = render_popup_frame_with_output(
                &ctx,
                &mut timer,
                Duration::ZERO,
                vec![
                    egui::Event::PointerMoved(press_point),
                    egui::Event::PointerButton {
                        pos: press_point,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            let pressed = snapshot((press_commands.clone(), pressed));
            assert!(press_commands.is_empty());
            assert!(pressed.primary_active);
            assert_action_frame_stroke(
                &pressed_output,
                pressed.primary_action,
                egui::Stroke::new(POPUP_FOCUS_STROKE_WIDTH, accent),
            );

            let (click_commands, clicked, clicked_output) = render_popup_frame_with_output(
                &ctx,
                &mut timer,
                Duration::ZERO,
                release_at(press_point),
            );
            let clicked = snapshot((click_commands.clone(), clicked));
            assert_eq!(click_commands, vec![Command::ToolsSprintTimerStart]);
            assert!(clicked.primary_clicked);
            assert_action_frame_stroke(
                &clicked_output,
                clicked.primary_action,
                egui::Stroke::new(POPUP_FOCUS_STROKE_WIDTH, accent),
            );

            assert!(timer.select_mode(TimerMode::Stopwatch));
            let (transition_commands, transitioned, transition_output) =
                render_popup_frame_with_output(&ctx, &mut timer, Duration::ZERO, Vec::new());
            let transitioned = snapshot((transition_commands.clone(), transitioned));
            assert!(transition_commands.is_empty());
            assert!(transitioned.primary_has_focus);
            assert_same_action_ids(&transitioned, (initial.primary_id, initial.end_id));
            assert_eq!(transitioned.primary_label, "Start Sprint");
            assert_eq!(timer.primary_command(), Command::ToolsSprintTimerStart);
            assert_same_fixed_tracks(&initial, &transitioned);
            assert_eq!(initial.display, transitioned.display);
            assert!(!transitioned.end_enabled);
            assert_action_frame_stroke(
                &transition_output,
                transitioned.primary_action,
                egui::Stroke::new(POPUP_FOCUS_STROKE_WIDTH, accent),
            );
            assert_action_frame_stroke(
                &transition_output,
                transitioned.end_action,
                disabled_stroke,
            );

            assert!(timer.select_mode(TimerMode::Countdown));
            let (_, returned, returned_output) =
                render_popup_frame_with_output(&ctx, &mut timer, Duration::ZERO, Vec::new());
            let returned = snapshot((Vec::new(), returned));
            assert!(returned.primary_has_focus);
            assert_same_action_ids(&returned, (initial.primary_id, initial.end_id));
            assert_same_fixed_tracks(&initial, &returned);
            assert_action_frame_stroke(
                &returned_output,
                returned.primary_action,
                egui::Stroke::new(POPUP_FOCUS_STROKE_WIDTH, accent),
            );
            assert_eq!(ctx.style().visuals.widgets.active.bg_stroke, red_active_stroke);
        }
    }

    #[test]
    fn rendered_end_action_pointer_and_clicked_states_use_popup_stroke_in_both_themes() {
        let accent = egui::Color32::from_rgb(20, 170, 90);
        let red_active_stroke = egui::Stroke::new(9.0, egui::Color32::RED);

        for dark in [false, true] {
            let ctx = egui::Context::default();
            let mut visuals = if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            };
            visuals.hyperlink_color = accent;
            visuals.widgets.active.bg_stroke = red_active_stroke;
            ctx.set_visuals(visuals);

            let mut timer = open_timer();
            assert!(timer.start_countdown(Duration::ZERO).is_none());
            let (_, initial, _) = render_popup_frame_with_output(
                &ctx,
                &mut timer,
                Duration::ZERO,
                Vec::new(),
            );
            let initial = snapshot((Vec::new(), initial));
            assert!(initial.end_enabled);
            let end_point = initial.end_action.center();

            let (press_commands, pressed, pressed_output) = render_popup_frame_with_output(
                &ctx,
                &mut timer,
                Duration::ZERO,
                vec![
                    egui::Event::PointerMoved(end_point),
                    egui::Event::PointerButton {
                        pos: end_point,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            let pressed = snapshot((press_commands.clone(), pressed));
            assert!(press_commands.is_empty());
            assert!(pressed.end_enabled);
            assert!(pressed.end_active);
            assert_action_frame_stroke(
                &pressed_output,
                pressed.end_action,
                egui::Stroke::new(POPUP_FOCUS_STROKE_WIDTH, accent),
            );

            let (click_commands, clicked, clicked_output) = render_popup_frame_with_output(
                &ctx,
                &mut timer,
                Duration::ZERO,
                release_at(end_point),
            );
            let clicked = snapshot((click_commands.clone(), clicked));
            assert_eq!(click_commands, vec![Command::ToolsSprintTimerEnd]);
            assert!(clicked.end_clicked);
            assert_action_frame_stroke(
                &clicked_output,
                clicked.end_action,
                egui::Stroke::new(POPUP_FOCUS_STROKE_WIDTH, accent),
            );
            assert_same_fixed_tracks(&initial, &clicked);
            assert_eq!(timer.primary_command(), Command::ToolsSprintTimerPause);
            assert_eq!(ctx.style().visuals.widgets.active.bg_stroke, red_active_stroke);
        }
    }

    #[test]
    fn idle_editor_uses_resolved_colors_and_stock_focus_matches_custom_selector() {
        let ctx = egui::Context::default();
        let accent = egui::Color32::from_rgb(20, 170, 90);
        let red_global_stroke = egui::Stroke::new(7.0, egui::Color32::RED);
        let mut visuals = egui::Visuals::light();
        visuals.hyperlink_color = accent;
        visuals.selection.stroke = red_global_stroke;
        ctx.set_visuals(visuals);
        let expected_background = popup_display_background(&ctx.style().visuals);
        let expected_text = popup_display_text(&ctx.style().visuals);
        let mut timer = open_timer();
        let before = timer.clone();
        let initial = snapshot(render_popup_frame(&ctx, &mut timer, Duration::ZERO, Vec::new()));
        let editor_rect = initial.duration_edit_rect.expect("idle Countdown editor exists");

        let _ = click_popup_point(&ctx, &mut timer, Duration::ZERO, editor_rect.center());
        let (focus_commands, focused_snapshot, output) = render_popup_frame_with_output(
            &ctx,
            &mut timer,
            Duration::ZERO,
            release_at(editor_rect.center()),
        );
        let focused_snapshot = snapshot((Vec::new(), focused_snapshot));
        let editor_shapes: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape) if shape.rect == editor_rect => Some(shape),
                _ => None,
            })
            .collect();
        let editor_frame = editor_shapes
            .iter()
            .find(|shape| shape.fill == expected_background)
            .expect("TextEdit uses the resolved popup display background");
        let custom_focus = popup_focus_stroke(&ctx.style().visuals);
        assert!(editor_shapes.iter().any(|shape| shape.stroke == custom_focus));
        assert_eq!(focused_snapshot.duration_edit_rect, Some(editor_rect));
        assert_eq!(editor_frame.fill, expected_background);
        assert_eq!(expected_text, ctx.style().visuals.text_color());
        assert_eq!(custom_focus, egui::Stroke::new(2.0, accent));
        assert!(focus_commands.is_empty());
        assert_eq!(ctx.style().visuals.selection.stroke, red_global_stroke);
        assert_eq!(timer.mode, before.mode);
        assert_eq!(timer.state, before.state);
        assert_eq!(timer.minutes_input, before.minutes_input);
        assert_eq!(initial.toggle, focused_snapshot.toggle);
        assert_eq!(initial.primary_action, focused_snapshot.primary_action);
        assert_eq!(initial.end_action, focused_snapshot.end_action);

    }

    #[test]
    fn active_readouts_use_shared_bounded_panels_and_remain_noninteractive_in_both_themes() {
        for dark in [false, true] {
            for mode in [TimerMode::Countdown, TimerMode::Stopwatch] {
                let ctx = egui::Context::default();
                ctx.set_visuals(if dark {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                });
                let mut timer = open_timer();
                let (expected_accessible_name, expected_state) = match mode {
                    TimerMode::Countdown => {
                        timer.set_minutes_input("2".to_owned());
                        let _ = timer.start_countdown(Duration::ZERO);
                        (
                            "Countdown remaining time 01:50".to_owned(),
                            timer.state.clone(),
                        )
                    }
                    TimerMode::Stopwatch => {
                        assert!(timer.select_mode(TimerMode::Stopwatch));
                        let _ = timer.start_stopwatch(Duration::ZERO);
                        (
                            "Stopwatch elapsed time 00:00:10".to_owned(),
                            timer.state.clone(),
                        )
                    }
                };
                let now = Duration::from_secs(10);
                let (commands, rendered, output) = render_popup_frame_with_output(
                    &ctx,
                    &mut timer,
                    now,
                    Vec::new(),
                );
                let rendered = snapshot((commands.clone(), rendered));

                assert!(commands.is_empty());
                assert_eq!(rendered.display_accessible_name, expected_accessible_name);
                assert!(rendered.duration_edit_rect.is_none());
                let panel = rendered
                    .active_display_panel_rect
                    .expect("active Countdown and Stopwatch have a bounded panel");
                assert_eq!(panel.size(), egui::vec2(175.0, 38.0));
                assert_eq!(panel.center(), rendered.display.center());
                assert!(rendered.display.contains_rect(panel));
                assert_eq!(POPUP_ACTIVE_DISPLAY_WIDTH, 175.0);
                assert_eq!(POPUP_ACTIVE_DISPLAY_HEIGHT, 38.0);
                assert_eq!(POPUP_DISPLAY_CORNER_RADIUS, 2);
                let panel_shape = output
                    .shapes
                    .iter()
                    .find_map(|clipped| match &clipped.shape {
                        egui::Shape::Rect(shape) if shape.rect == panel => Some(shape),
                        _ => None,
                    })
                    .expect("the active readout paints its panel");
                assert_eq!(
                    panel_shape.corner_radius,
                    egui::CornerRadius::same(POPUP_DISPLAY_CORNER_RADIUS)
                );
                assert_eq!(panel_shape.fill, popup_display_background(&ctx.style().visuals));
                assert_eq!(panel_shape.stroke, egui::Stroke::NONE);
                assert!(output.shapes.iter().any(|clipped| {
                    clipped.clip_rect == panel && matches!(&clipped.shape, egui::Shape::Text(_))
                }), "readout text is clipped to the bounded panel without wrapping");
                // The explicit label is published against the exact painted panel bounds.
                let accessible_bounds = accesskit_bounds(panel);
                assert_eq!(accessible_bounds.x0, panel.min.x.into());
                assert_eq!(accessible_bounds.y0, panel.min.y.into());
                assert_eq!(accessible_bounds.x1, panel.max.x.into());
                assert_eq!(accessible_bounds.y1, panel.max.y.into());
                assert!(rendered.display_accessible_name.starts_with(match mode {
                    TimerMode::Countdown => "Countdown remaining time ",
                    TimerMode::Stopwatch => "Stopwatch elapsed time ",
                }));
                assert_eq!(timer.state, expected_state);

                let state_before_readout_click = timer.state.clone();
                let (click_commands, _) = click_popup_point(
                    &ctx,
                    &mut timer,
                    now,
                    rendered.display.center(),
                );
                assert!(click_commands.is_empty());
                assert_eq!(timer.state, state_before_readout_click);
            }
        }
    }

    #[test]
    fn stopwatch_readout_keeps_long_hours_unwrapped_and_clips_to_the_panel_bounds() {
        for dark in [false, true] {
            let ctx = egui::Context::default();
            ctx.set_visuals(if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
            let mut timer = open_timer();
            assert!(timer.select_mode(TimerMode::Stopwatch));
            let _ = timer.start_stopwatch(Duration::ZERO);
            let now = Duration::from_secs(36_000_012);
            let (commands, rendered, output) = render_popup_frame_with_output(
                &ctx,
                &mut timer,
                now,
                Vec::new(),
            );
            let rendered = snapshot((commands.clone(), rendered));

            assert!(commands.is_empty());
            assert_eq!(rendered.display_text, "10000:00:12");
            assert_eq!(
                rendered.display_accessible_name,
                "Stopwatch elapsed time 10000:00:12"
            );
            let panel = rendered
                .active_display_panel_rect
                .expect("long Stopwatch values retain the fixed panel");
            assert_eq!(panel.size(), egui::vec2(175.0, 38.0));
            assert!(rendered.display.contains_rect(panel));
            assert!(output.shapes.iter().any(|clipped| {
                clipped.clip_rect == panel && matches!(&clipped.shape, egui::Shape::Text(_))
            }), "long elapsed text stays on one line and is bounded by the panel");
            assert!(timer.is_running_or_paused());
        }
    }

    #[test]
    fn running_paused_and_completed_displays_keep_the_same_action_tracks() {
        let mut idle = open_timer();
        let idle_render = snapshot(render_popup_frame(
            &egui::Context::default(),
            &mut idle,
            Duration::ZERO,
            Vec::new(),
        ));

        let mut running = open_timer();
        running.set_minutes_input("2".to_owned());
        let _ = running.start_countdown(Duration::ZERO);
        let running_render = snapshot(render_popup_frame(
            &egui::Context::default(),
            &mut running,
            Duration::from_secs(10),
            Vec::new(),
        ));
        assert_eq!(running_render.display_text, "01:50");
        assert_eq!(running_render.primary_label, "Pause");
        assert!(running_render.active_display_panel_rect.is_some());

        let mut paused = running.clone();
        let _ = paused.pause(Duration::from_secs(10));
        let paused_render = snapshot(render_popup_frame(
            &egui::Context::default(),
            &mut paused,
            Duration::from_secs(40),
            Vec::new(),
        ));
        assert_eq!(paused_render.display_text, "01:50");
        assert_eq!(paused_render.primary_label, "Resume");
        assert!(paused_render.active_display_panel_rect.is_some());

        let mut completed = open_timer();
        completed.set_minutes_input("1".to_owned());
        let _ = completed.start_countdown(Duration::ZERO);
        let _ = completed.reconcile(Duration::from_secs(60));
        let completed_render = snapshot(render_popup_frame(
            &egui::Context::default(),
            &mut completed,
            Duration::from_secs(60),
            Vec::new(),
        ));
        assert_eq!(completed_render.display_text, "00:00");
        assert_eq!(completed_render.primary_label, "Reset");
        assert!(completed_render.active_display_panel_rect.is_some());

        for rendered in [&running_render, &paused_render, &completed_render] {
            assert_same_fixed_tracks(&idle_render, rendered);
            assert_eq!(idle_render.display, rendered.display);
        }

        let mut stopwatch = open_timer();
        assert!(stopwatch.select_mode(TimerMode::Stopwatch));
        let _ = stopwatch.start_stopwatch(Duration::ZERO);
        let stopwatch_render = snapshot(render_popup_frame(
            &egui::Context::default(),
            &mut stopwatch,
            Duration::from_secs(360_012),
            Vec::new(),
        ));
        assert_eq!(stopwatch_render.display_text, "100:00:12");
        assert!(stopwatch_render.active_display_panel_rect.is_some());
        assert_same_fixed_tracks(&idle_render, &stopwatch_render);
    }

    #[test]
    fn idle_stopwatch_uses_noneditable_panel_while_idle_countdown_remains_the_editor() {
        let countdown_ctx = egui::Context::default();
        let stopwatch_ctx = egui::Context::default();
        let mut countdown = open_timer();
        let mut stopwatch = open_timer();
        assert!(stopwatch.select_mode(TimerMode::Stopwatch));

        let countdown_render = snapshot(render_popup_frame(
            &countdown_ctx,
            &mut countdown,
            Duration::ZERO,
            Vec::new(),
        ));
        let stopwatch_render = snapshot(render_popup_frame(
            &stopwatch_ctx,
            &mut stopwatch,
            Duration::ZERO,
            Vec::new(),
        ));

        assert!(countdown_render.duration_edit_rect.is_some());
        assert!(countdown_render.active_display_panel_rect.is_none());
        assert!(stopwatch_render.duration_edit_rect.is_none());
        assert_eq!(stopwatch_render.display_text, "00:00:00");
        assert_eq!(
            stopwatch_render.display_accessible_name,
            "Stopwatch elapsed time 00:00:00"
        );
        let panel = stopwatch_render
            .active_display_panel_rect
            .expect("Stopwatch idle uses the read-only panel");
        assert_eq!(panel.center(), stopwatch_render.display.center());
        assert!(stopwatch_render.display.contains_rect(panel));

        let before_pointer_input = stopwatch.clone();
        let (commands, _) = click_popup_point(
            &stopwatch_ctx,
            &mut stopwatch,
            Duration::ZERO,
            panel.center(),
        );
        assert!(commands.is_empty());
        assert_eq!(stopwatch.state, before_pointer_input.state);
        assert_eq!(stopwatch.minutes_input, before_pointer_input.minutes_input);
    }

    #[test]
    fn completion_primary_action_emits_reset_without_moving_the_row() {
        let ctx = egui::Context::default();
        let mut timer = open_timer();
        timer.set_minutes_input("1".to_owned());
        let _ = timer.start_countdown(Duration::ZERO);
        let _ = timer.reconcile(Duration::from_secs(60));
        let completed = snapshot(render_popup_frame(
            &ctx,
            &mut timer,
            Duration::from_secs(60),
            Vec::new(),
        ));
        assert!(completed.feedback_text.contains("completed a 1 Minute Sprint"));
        assert_eq!(completed.primary_label, "Reset");

        let (commands, _) = click_popup_point(
            &ctx,
            &mut timer,
            Duration::from_secs(60),
            completed.primary_action.center(),
        );
        assert!(commands.contains(&Command::ToolsSprintTimerReset));
        assert_eq!(timer.primary_label(), "Reset");
        timer.reset();
        assert!(timer.is_idle());
        assert_eq!(timer.minutes_input, DEFAULT_MINUTES);
    }

    #[test]
    fn rendered_feedback_is_accessible_bounded_and_does_not_shift_action_tracks() {
        let clean_ctx = egui::Context::default();
        let feedback_ctx = egui::Context::default();
        let mut clean = open_timer();
        let mut feedback_timer = open_timer();
        feedback_timer.validation_error = Some("validation detail ".repeat(40));
        feedback_timer.notification_feedback = Some(NotificationFeedback::Unavailable);

        let clean_render = snapshot(render_popup_frame(
            &clean_ctx,
            &mut clean,
            Duration::ZERO,
            Vec::new(),
        ));
        let feedback_render = snapshot(render_popup_frame(
            &feedback_ctx,
            &mut feedback_timer,
            Duration::ZERO,
            Vec::new(),
        ));

        assert_same_fixed_tracks(&clean_render, &feedback_render);
        assert!(feedback_render.body.contains_rect(feedback_render.feedback));
        assert!(!feedback_render.feedback.intersects(feedback_render.toggle));
        assert!(!feedback_render.feedback.intersects(feedback_render.primary_action));
        assert!(!feedback_render.feedback.intersects(feedback_render.end_action));
        assert!(feedback_render.feedback_text.contains("validation detail"));
        assert!(feedback_render
            .feedback_text
            .contains("notification unavailable"));
    }

    #[test]
    fn end_sprint_is_disabled_when_idle_and_enabled_while_running_or_paused() {
        let idle_ctx = egui::Context::default();
        let mut idle = open_timer();
        let idle_render = snapshot(render_popup_frame(
            &idle_ctx,
            &mut idle,
            Duration::ZERO,
            Vec::new(),
        ));
        assert!(!idle_render.end_enabled);
        let (commands, _) = click_popup_point(
            &idle_ctx,
            &mut idle,
            Duration::ZERO,
            idle_render.end_action.center(),
        );
        assert!(!commands.contains(&Command::ToolsSprintTimerEnd));

        let mut running = open_timer();
        let _ = running.start_countdown(Duration::ZERO);
        let running_render = snapshot(render_popup_frame(
            &egui::Context::default(),
            &mut running,
            Duration::ZERO,
            Vec::new(),
        ));
        assert!(running_render.end_enabled);
        let mut paused = running.clone();
        let _ = paused.pause(Duration::from_secs(1));
        let paused_render = snapshot(render_popup_frame(
            &egui::Context::default(),
            &mut paused,
            Duration::from_secs(1),
            Vec::new(),
        ));
        assert!(paused_render.end_enabled);
        assert_same_fixed_tracks(&idle_render, &running_render);
        assert_same_fixed_tracks(&idle_render, &paused_render);
    }

    #[test]
    fn accessible_mode_and_display_names_render_without_semantic_session_mutation() {
        let ctx = egui::Context::default();
        let mut timer = open_timer();
        let semantic_before = timer.clone();
        let (commands, rendered) = render_popup_frame(
            &ctx,
            &mut timer,
            Duration::ZERO,
            Vec::new(),
        );
        let rendered = snapshot((commands.clone(), rendered));

        assert_eq!(
            rendered.selector_accessibility.countdown_name,
            "Countdown"
        );
        assert_eq!(rendered.selector_accessibility.stopwatch_name, "Stopwatch");
        assert_eq!(rendered.selector_accessibility.switch_name, "Stopwatch mode");
        assert!(rendered.selector_accessibility.countdown_selected);
        assert!(!rendered.selector_accessibility.stopwatch_selected);
        assert_eq!(
            rendered.display_accessible_name,
            "Sprint duration in minutes"
        );
        assert_eq!(rendered.primary_label, "Start Sprint");
        assert!(commands.is_empty());

        assert_eq!(timer.mode, semantic_before.mode);
        assert_eq!(timer.state, semantic_before.state);
        assert_eq!(timer.minutes_input, semantic_before.minutes_input);
        assert_eq!(timer.validation_error, semantic_before.validation_error);
        assert_eq!(
            timer.notification_feedback,
            semantic_before.notification_feedback
        );
        // Popup rendering accepts no screenplay/session object; it emits only the existing timer
        // command vocabulary and changes only viewport-local transient popup bookkeeping.
    }

    #[test]
    fn closing_uses_controlled_time_and_returns_to_the_icon_scale_and_zero_opacity() {
        let rest = sprint_timer_popup_placement(
            Some(egui::Rect::from_min_size(egui::pos2(290.0, 370.0), egui::vec2(20.0, 20.0))),
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0)),
        );
        let closing = PopupClosingTransition {
            started_at: 7.0,
            start_scale: 1.0,
            start_opacity: 1.0,
            rest,
            icon_anchor: egui::Rect::from_min_size(
                egui::pos2(290.0, 370.0),
                egui::vec2(20.0, 20.0),
            ),
        };
        let initial = popup_closing_presentation(closing, 7.0);
        let middle = popup_closing_presentation(closing, 7.06);
        let final_frame = popup_closing_presentation(closing, 7.12);

        assert_eq!(initial.scale, 1.0);
        assert_eq!(initial.opacity, 1.0);
        assert_eq!(middle.progress, 0.125);
        assert!((middle.scale - 0.98125).abs() < 0.00001);
        assert!((middle.opacity - 0.875).abs() < 0.00001);
        assert_eq!(final_frame.scale, POPUP_OPENING_START_SCALE);
        assert_eq!(final_frame.opacity, 0.0);
        assert_eq!(final_frame.transform, egui::emath::TSTransform::IDENTITY);
        assert_eq!(final_frame.repaint_after, None);
        assert!(initial.active && middle.active);
        assert!(!final_frame.active);
    }

    #[test]
    fn closing_request_coalesces_and_freezes_rest_geometry_and_icon_anchor() {
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0));
        let mut timer = open_timer();
        timer.just_opened = false;
        let ctx = egui::Context::default();
        let _ = render_popup_frame_at_egui_time(
            &ctx,
            &mut timer,
            Duration::ZERO,
            4.0,
            Vec::new(),
            screen,
        );
        let rest = timer.popup_rest_placement;
        let anchor = timer.popup_anchor;

        assert!(timer.request_popup_close(4.0, screen));
        let closing = timer.closing;
        assert!(!timer.request_popup_close(4.03, screen));
        timer.popup_anchor = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(20.0, 20.0));

        assert_eq!(timer.closing, closing);
        assert_eq!(closing.map(|state| state.rest), rest);
        assert_eq!(closing.map(|state| state.icon_anchor), Some(anchor));
        assert!(timer.popup_open);
        assert!(timer.is_closing());
    }

    #[test]
    fn interruption_during_opening_starts_close_at_current_visual_progress() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0));
        let mut timer = open_timer();
        timer.just_opened = false;
        timer.start_popup_opening(10.0);
        let open_frame = snapshot(render_popup_frame_at_egui_time(
            &ctx,
            &mut timer,
            Duration::ZERO,
            10.07,
            Vec::new(),
            screen,
        ));
        assert!((open_frame.opening_scale - 0.98125).abs() < 0.00001);
        assert_eq!(open_frame.opening_opacity, 0.875);

        assert!(timer.request_popup_close(10.07, screen));
        let closing = timer.closing;
        assert!(closing.is_some());
        if let Some(closing) = closing {
            assert!((closing.start_scale - open_frame.opening_scale).abs() < 0.00001);
            assert_eq!(closing.start_opacity, open_frame.opening_opacity);
        }
        let close_frame = snapshot(render_popup_frame_at_egui_time(
            &ctx,
            &mut timer,
            Duration::ZERO,
            10.07,
            Vec::new(),
            screen,
        ));
        assert_eq!(close_frame.opening_scale, open_frame.opening_scale);
        assert_eq!(close_frame.opening_opacity, open_frame.opening_opacity);
        assert_ne!(close_frame.opening_scale, POPUP_OPENING_END_SCALE);
    }

    #[test]
    fn closing_disables_popup_interaction_and_does_not_reset_idle_state_early() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0));
        let mut timer = open_timer();
        timer.just_opened = false;
        timer.set_minutes_input("7".to_owned());
        let initial = snapshot(render_popup_frame_at_egui_time(
            &ctx,
            &mut timer,
            Duration::ZERO,
            3.0,
            Vec::new(),
            screen,
        ));
        let before = timer.clone();
        assert!(timer.request_popup_close(3.0, screen));
        let click = initial.primary_action.center();
        let commands = render_popup_frame_at_egui_time(
            &ctx,
            &mut timer,
            Duration::ZERO,
            3.03,
            vec![
                egui::Event::PointerMoved(click),
                egui::Event::PointerButton {
                    pos: click,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos: click,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            screen,
        )
        .0;

        assert!(commands.is_empty());
        assert_eq!(timer.minutes_input, before.minutes_input);
        assert_eq!(timer.state, before.state);
        assert!(timer.popup_open);
    }

    #[test]
    fn closing_completion_removes_render_and_restores_layer_before_finalization() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0));
        let mut timer = open_timer();
        timer.just_opened = false;
        let _ = render_popup_frame_at_egui_time(
            &ctx,
            &mut timer,
            Duration::ZERO,
            5.0,
            Vec::new(),
            screen,
        );
        assert!(timer.request_popup_close(5.0, screen));
        let rendered = render_popup_frame_at_egui_time(
            &ctx,
            &mut timer,
            Duration::ZERO,
            5.12,
            Vec::new(),
            screen,
        );

        assert!(rendered.1.is_none());
        assert!(timer.popup_open);
        assert!(timer.take_popup_close_completion());
        assert!(!timer.take_popup_close_completion());
        assert_eq!(timer.popup_rect, egui::Rect::NOTHING);
        assert_eq!(
            ctx.layer_transform_to_global(sprint_timer_popup_layer_id()),
            None
        );
    }

    #[test]
    fn closing_keeps_active_countdown_and_stopwatch_timekeeping_live() {
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0));
        let mut countdown = open_timer();
        countdown.set_minutes_input("2".to_owned());
        let _ = countdown.start_countdown(Duration::ZERO);
        assert!(countdown.request_popup_close(2.0, screen));
        assert_eq!(countdown.remaining(Duration::from_secs(15)), Some(Duration::from_secs(105)));
        assert_eq!(countdown.remaining(Duration::from_secs(20)), Some(Duration::from_secs(100)));

        let mut stopwatch = open_timer();
        assert!(stopwatch.select_mode(TimerMode::Stopwatch));
        let _ = stopwatch.start_stopwatch(Duration::from_secs(4));
        assert!(stopwatch.request_popup_close(2.0, screen));
        assert_eq!(stopwatch.elapsed(Duration::from_secs(9)), Some(Duration::from_secs(5)));
        assert_eq!(stopwatch.elapsed(Duration::from_secs(12)), Some(Duration::from_secs(8)));
        assert!(countdown.is_running_or_paused() && stopwatch.is_running_or_paused());
    }

    #[test]
    fn escape_outside_release_and_host_move_feed_the_same_close_request_state() {
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0));
        let outside = egui::pos2(590.0, 20.0);
        let captured = host_geometry(100.0, 120.0, 800.0, 600.0);
        let moved = host_geometry(101.0, 120.0, 800.0, 600.0);
        let resized = host_geometry(100.0, 120.0, 801.0, 600.0);
        let mut source_timers = Vec::new();

        let escape_ctx = egui::Context::default();
        let mut escape_timer = open_timer();
        escape_timer.just_opened = false;
        let _ = render_popup_frame(&escape_ctx, &mut escape_timer, Duration::ZERO, Vec::new());
        let escape_commands = render_popup_frame_at_egui_time(
            &escape_ctx,
            &mut escape_timer,
            Duration::ZERO,
            30.0,
            vec![egui::Event::Key {
                key: Key::Escape,
                physical_key: Some(Key::Escape),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            screen,
        )
        .0;
        source_timers.push((escape_timer, escape_commands));

        let outside_ctx = egui::Context::default();
        let mut outside_timer = open_timer();
        outside_timer.just_opened = false;
        let _ = render_popup_frame(&outside_ctx, &mut outside_timer, Duration::ZERO, Vec::new());
        let outside_commands = click_popup_point(
            &outside_ctx,
            &mut outside_timer,
            Duration::ZERO,
            outside,
        )
        .0;
        source_timers.push((outside_timer, outside_commands));

        let host_ctx = egui::Context::default();
        let mut host_timer = open_timer();
        let host_commands = render_popup_frame_with_host_geometry(
            &host_ctx,
            &mut host_timer,
            captured,
            moved,
        )
        .0;
        source_timers.push((host_timer, host_commands));

        let resize_ctx = egui::Context::default();
        let mut resize_timer = open_timer();
        let resize_commands = render_popup_frame_with_host_geometry(
            &resize_ctx,
            &mut resize_timer,
            captured,
            resized,
        )
        .0;
        source_timers.push((resize_timer, resize_commands));

        for (mut timer, commands) in source_timers {
            assert_eq!(commands, vec![Command::ToolsSprintTimerClosePopup]);
            assert!(timer.request_popup_close(30.0, screen));
            assert!(timer.is_closing());
            assert!(timer.popup_open);
        }
    }

    #[test]
    fn body_and_title_viewports_close_about_their_own_captured_icon_anchors() {
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640.0, 440.0));
        let root_ctx = egui::Context::default();
        let title_ctx = egui::Context::default();
        let mut root_timer = open_timer();
        let mut title_timer = open_timer();
        root_timer.just_opened = false;
        title_timer.just_opened = false;
        root_timer.popup_anchor = egui::Rect::from_min_size(egui::pos2(500.0, 400.0), egui::vec2(20.0, 20.0));
        title_timer.popup_anchor = egui::Rect::from_min_size(egui::pos2(120.0, 300.0), egui::vec2(20.0, 20.0));
        let _ = render_popup_frame_at_egui_time(
            &root_ctx,
            &mut root_timer,
            Duration::ZERO,
            40.0,
            Vec::new(),
            screen,
        );
        let _ = render_popup_frame_at_egui_time(
            &title_ctx,
            &mut title_timer,
            Duration::ZERO,
            40.0,
            Vec::new(),
            screen,
        );
        assert!(root_timer.request_popup_close(40.0, screen));
        assert!(title_timer.request_popup_close(40.0, screen));
        let root_rest = root_timer.popup_rest_placement;
        let title_rest = title_timer.popup_rest_placement;

        let root = snapshot(render_popup_frame_at_egui_time(
            &root_ctx,
            &mut root_timer,
            Duration::ZERO,
            40.04,
            Vec::new(),
            screen,
        ));
        let title = snapshot(render_popup_frame_at_egui_time(
            &title_ctx,
            &mut title_timer,
            Duration::ZERO,
            40.04,
            Vec::new(),
            screen,
        ));

        assert_eq!(Some(root.body), root_rest.map(|placement| placement.body));
        assert_eq!(Some(title.body), title_rest.map(|placement| placement.body));
        assert_eq!(root.layer_transform * root_timer.popup_anchor.center(), root_timer.popup_anchor.center());
        assert_eq!(title.layer_transform * title_timer.popup_anchor.center(), title_timer.popup_anchor.center());
        assert_ne!(root.layer_transform.translation, title.layer_transform.translation);
    }
}

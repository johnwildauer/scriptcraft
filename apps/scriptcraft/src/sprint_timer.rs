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




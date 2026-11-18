use eframe::egui::{self, Key, WidgetInfo, WidgetType};
use scriptcraft_commands::{Command, SprintTimerMode as CommandTimerMode};
use std::time::Duration;

use super::geometry::{
    mode_focus_rect, mode_selector_geometry, sprint_timer_popup_layout, ModeSelectorGeometry,
    PopupGeometryCheckpoint, PopupGeometryLedger, SprintTimerHostGeometry,
    POPUP_COUNTDOWN_EDITOR_HEIGHT, POPUP_COUNTDOWN_EDITOR_WIDTH,
};
use super::paint::{
    apply_popup_content_style, mode_focus_stroke, paint_mode_switch,
    paint_noninteractive_active_display_panel, paint_noninteractive_stage_label,
    paint_popup_feedback, popup_display_background, popup_display_text, popup_feedback_text,
    SprintTimerPopupRenderSnapshot, POPUP_COUNTDOWN_EDITOR_FONT_SIZE,
    POPUP_MODE_FOCUS_CORNER_RADIUS, POPUP_MODE_LABEL_FONT_SIZE,
};
use super::state::TimerMode;
use super::animation;
use super::animation::POPUP_OPENING_END_SCALE;
use super::SprintTimer;

pub(crate) fn show_popup(
    ctx: &egui::Context,
    timer: &mut SprintTimer,
    now: Duration,
) -> Vec<Command> {
    show_popup_with_host_geometry(ctx, timer, now, None)
}

pub(crate) fn show_popup_with_host_geometry(
    ctx: &egui::Context,
    timer: &mut SprintTimer,
    now: Duration,
    captured_host_geometry: Option<SprintTimerHostGeometry>,
) -> Vec<Command> {
    show_popup_with_snapshot_and_host_geometry(ctx, timer, now, captured_host_geometry).0
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
        return animation::show_closing_popup(ctx, timer, now, viewport, closing, egui_time);
    }

    if host_geometry_changed(captured_host_geometry, current_host_geometry) {
        commands.push(Command::ToolsSprintTimerClosePopup);
        return (commands, None);
    }

    if dismissal_requested(ctx, timer) {
        commands.push(Command::ToolsSprintTimerClosePopup);
        return (commands, None);
    }

    animation::show_opening_popup(ctx, timer, now, viewport, egui_time)
}

const MODE_SWITCH_ACCESSIBLE_NAME: &str = "Stopwatch mode";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct ModeSelectorAccessibility {
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

pub(super) fn show_popup_contents(
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
        paint_popup_feedback(ui, feedback, timer, &feedback_text);
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

pub(super) fn host_geometry_changed(
    captured_host_geometry: Option<SprintTimerHostGeometry>,
    current_host_geometry: SprintTimerHostGeometry,
) -> bool {
    captured_host_geometry
        .is_some_and(|captured| current_host_geometry.changed_from(captured))
}

pub(super) fn dismissal_requested(ctx: &egui::Context, timer: &mut SprintTimer) -> bool {
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
    escape || outside_press
}

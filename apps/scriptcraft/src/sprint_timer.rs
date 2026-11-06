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

#[derive(Debug, Clone, PartialEq, Eq)]
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
        }
    }
}

impl SprintTimer {
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
                TimerMode::Stopwatch => "Start",
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
    let mut commands = Vec::new();
    if !timer.popup_open {
        return commands;
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
        return commands;
    }

    let mut open = true;
    let default_position = if timer.popup_anchor.is_positive() {
        timer.popup_anchor.left_top() - egui::vec2(0.0, 230.0)
    } else {
        ctx.content_rect().center() - egui::vec2(145.0, 90.0)
    };
    let output = egui::Area::new(egui::Id::new("sprint-timer-popup"))
        .order(egui::Order::Foreground)
        .fixed_pos(default_position)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(280.0);
                ui.horizontal(|ui| {
                    ui.heading("Sprint Timer");
                    if ui.button("Close").clicked() {
                        commands.push(Command::ToolsSprintTimerClosePopup);
                        open = false;
                    }
                });
                ui.horizontal(|ui| {
                    let enabled = timer.mode_switch_enabled();
                    let mut selected = timer.mode;
                    ui.add_enabled_ui(enabled, |ui| {
                        ui.selectable_value(&mut selected, TimerMode::Countdown, "Countdown");
                        ui.selectable_value(&mut selected, TimerMode::Stopwatch, "Stopwatch");
                    });
                    if selected != timer.mode {
                        commands.push(Command::ToolsSprintTimerSetMode {
                            mode: match selected {
                                TimerMode::Countdown => CommandTimerMode::Countdown,
                                TimerMode::Stopwatch => CommandTimerMode::Stopwatch,
                            },
                        });
                    }
                });
                ui.separator();

                match timer.mode {
                    TimerMode::Countdown => {
                        ui.label(
                            egui::RichText::new(timer.countdown_display(now))
                                .size(28.0)
                                .monospace(),
                        );
                        if timer.is_idle() {
                            ui.horizontal(|ui| {
                                ui.label("Minutes");
                                let mut minutes_input = timer.minutes_input.clone();
                                let response = ui.add(
                                    egui::TextEdit::singleline(&mut minutes_input)
                                        .id(egui::Id::new("sprint-timer-minutes"))
                                        .desired_width(88.0)
                                        .hint_text("25"),
                                );
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
                            });
                        }
                    }
                    TimerMode::Stopwatch => {
                        ui.label(
                            egui::RichText::new(timer.stopwatch_display(now))
                                .size(28.0)
                                .monospace(),
                        );
                    }
                }

                if let Some(message) = timer.completed_message() {
                    ui.label(message);
                }
                if let Some(error) = timer.validation_error.as_deref() {
                    ui.colored_label(ui.visuals().error_fg_color, error)
                        .widget_info(|| {
                            WidgetInfo::labeled(WidgetType::Label, true, error.to_owned())
                        });
                }
                if let Some(feedback) = timer.notification_feedback.as_ref() {
                    let text = match feedback {
                        NotificationFeedback::Pending => {
                            "OS notification request pending; delivery is not confirmed."
                        }
                        NotificationFeedback::Accepted => {
                            "OS accepted the notification request; delivery is not confirmed."
                        }
                        NotificationFeedback::Unavailable => {
                            "OS notification unavailable; the in-app completion remains available."
                        }
                    };
                    ui.label(text).widget_info(|| {
                        WidgetInfo::labeled(WidgetType::Label, true, text.to_owned())
                    });
                }

                ui.horizontal(|ui| {
                    let primary_label = timer.primary_label();
                    let primary_command = timer.primary_command();
                    let primary = ui.button(primary_label);
                    if primary.clicked() {
                        commands.push(primary_command);
                    }
                    let end = ui.add_enabled(timer.end_enabled(), egui::Button::new("End Sprint"));
                    end.widget_info(|| {
                        WidgetInfo::labeled(
                            WidgetType::Button,
                            timer.end_enabled(),
                            "End Sprint".to_owned(),
                        )
                    });
                    if end.clicked() {
                        commands.push(Command::ToolsSprintTimerEnd);
                    }
                });
            })
        });
    timer.popup_rect = output.response.rect;
    timer.popup_anchor = egui::Rect::NOTHING;
    if !open {
        if !commands
            .iter()
            .any(|command| command.id() == scriptcraft_commands::CommandId::ToolsSprintTimerClosePopup)
        {
            commands.push(Command::ToolsSprintTimerClosePopup);
        }
    }
    commands
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

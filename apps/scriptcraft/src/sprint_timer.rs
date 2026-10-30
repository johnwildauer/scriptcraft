use eframe::egui::{self, Key, WidgetInfo, WidgetType};
use std::process::Command;
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
    #[cfg(test)]
    pub(super) status_item_rect: Option<egui::Rect>,
    #[cfg(test)]
    pub(super) minutes_input_rect: Option<egui::Rect>,
    #[cfg(test)]
    pub(super) primary_button_rect: Option<egui::Rect>,
    #[cfg(test)]
    pub(super) end_button_rect: Option<egui::Rect>,
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
            #[cfg(test)]
            status_item_rect: None,
            #[cfg(test)]
            minutes_input_rect: None,
            #[cfg(test)]
            primary_button_rect: None,
            #[cfg(test)]
            end_button_rect: None,
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

    #[cfg(test)]
    pub(super) fn reconcile_with_sink(
        &mut self,
        now: Duration,
        sink: &mut dyn NotificationSink,
    ) -> Option<CompletionNotification> {
        let notification = self.reconcile(now)?;
        let result = sink.request(notification);
        self.apply_notification_result(notification.generation, result);
        Some(notification)
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
    sink: &mut dyn NotificationSink,
) {
    if !timer.popup_open {
        return;
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
        timer.popup_open = false;
        timer.close_popup();
        return;
    }

    let mut open = true;
    let default_position = if timer.popup_anchor.is_positive() {
        timer.popup_anchor.left_top() - egui::vec2(0.0, 230.0)
    } else {
        ctx.screen_rect().center() - egui::vec2(145.0, 90.0)
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
                        timer.select_mode(selected);
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
                                let response = ui.add(
                                    egui::TextEdit::singleline(&mut timer.minutes_input)
                                        .id(egui::Id::new("sprint-timer-minutes"))
                                        .desired_width(88.0)
                                        .hint_text("25"),
                                );
                                #[cfg(test)]
                                {
                                    timer.minutes_input_rect = Some(response.rect);
                                }
                                response.widget_info(|| {
                                    WidgetInfo::labeled(
                                        WidgetType::TextEdit,
                                        true,
                                        "Sprint duration in minutes".to_owned(),
                                    )
                                });
                                if response.changed() {
                                    timer.set_minutes_input(timer.minutes_input.clone());
                                }
                                let enter = (response.has_focus() || response.lost_focus())
                                    && ui.input(|input| input.key_pressed(Key::Enter));
                                if enter {
                                    if let Some(notice) = timer.start_countdown(now) {
                                        let result = sink.request(notice);
                                        timer.apply_notification_result(notice.generation, result);
                                    }
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
                    let primary = ui.button(primary_label);
                    #[cfg(test)]
                    {
                        timer.primary_button_rect = Some(primary.rect);
                    }
                    if primary.clicked() {
                        match (timer.mode, primary_label) {
                            (TimerMode::Countdown, "Start Sprint") => {
                                if let Some(notice) = timer.start_countdown(now) {
                                    let result = sink.request(notice);
                                    timer.apply_notification_result(notice.generation, result);
                                }
                            }
                            (TimerMode::Stopwatch, "Start") => {
                                if let Some(notice) = timer.start_stopwatch(now) {
                                    let result = sink.request(notice);
                                    timer.apply_notification_result(notice.generation, result);
                                }
                            }
                            (_, "Pause") => {
                                if let Some(notice) = timer.pause(now) {
                                    let result = sink.request(notice);
                                    timer.apply_notification_result(notice.generation, result);
                                }
                            }
                            (_, "Resume") => {
                                if let Some(notice) = timer.resume(now) {
                                    let result = sink.request(notice);
                                    timer.apply_notification_result(notice.generation, result);
                                }
                            }
                            (_, "Reset") => timer.reset(),
                            _ => {}
                        }
                    }
                    let end = ui.add_enabled(timer.end_enabled(), egui::Button::new("End Sprint"));
                    #[cfg(test)]
                    {
                        timer.end_button_rect = Some(end.rect);
                    }
                    end.widget_info(|| {
                        WidgetInfo::labeled(
                            WidgetType::Button,
                            timer.end_enabled(),
                            "End Sprint".to_owned(),
                        )
                    });
                    if end.clicked() {
                        if let Some(notice) = timer.end(now) {
                            let result = sink.request(notice);
                            timer.apply_notification_result(notice.generation, result);
                        }
                    }
                });
            })
        });
    timer.popup_rect = output.response.rect;
    timer.popup_anchor = egui::Rect::NOTHING;
    if !open {
        timer.popup_open = false;
        timer.close_popup();
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
                    let result = Command::new("/usr/bin/osascript")
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
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingSink {
        requests: Vec<CompletionNotification>,
        result: NotificationResult,
    }

    impl NotificationSink for RecordingSink {
        fn request(&mut self, notice: CompletionNotification) -> NotificationResult {
            self.requests.push(notice);
            self.result.clone()
        }
    }

    fn start(minutes: &str) -> SprintTimer {
        let mut timer = SprintTimer::default();
        timer.set_minutes_input(minutes.to_owned());
        timer
    }

    #[test]
    fn wo003_countdown_validates_input_and_preserves_invalid_text() {
        let mut timer = SprintTimer::default();
        assert_eq!(timer.minutes_input, "25");
        for invalid in ["", "0", "1441", "999999999999999999999999999999999999"] {
            timer.set_minutes_input(invalid.to_owned());
            timer.start_countdown(Duration::ZERO);
            assert!(timer.is_idle());
            assert_eq!(timer.minutes_input, invalid);
            assert!(timer.validation_error.is_some());
        }
        timer.set_minutes_input("1".to_owned());
        timer.start_countdown(Duration::ZERO);
        assert!(timer.is_running_or_paused());
        timer.end(Duration::from_secs(2));
        assert_eq!(timer.minutes_input, "1");
        timer.set_minutes_input("1440".to_owned());
        timer.start_countdown(Duration::ZERO);
        assert!(timer.is_running_or_paused());
    }

    #[test]
    fn wo003_digit_filter_keeps_empty_and_removes_signs_and_decimal_separators() {
        let mut timer = SprintTimer::default();
        timer.set_minutes_input("-1.5".to_owned());
        assert_eq!(timer.minutes_input, "15");
        timer.set_minutes_input("１２٣".to_owned());
        assert_eq!(timer.minutes_input, "");
        timer.set_minutes_input("00025".to_owned());
        assert_eq!(timer.minutes_input, "00025");
        timer.start_countdown(Duration::ZERO);
        assert!(timer.is_running_or_paused());
        timer.end(Duration::from_secs(1));
        assert_eq!(timer.minutes_input, "00025");
    }

    #[test]
    fn wo003_countdown_uses_ceiling_seconds_pause_resume_and_exact_end_restore() {
        let mut timer = start("00025");
        timer.start_countdown(Duration::ZERO);
        assert_eq!(timer.countdown_display(Duration::from_millis(1)), "25:00");
        assert_eq!(timer.countdown_display(Duration::from_secs(1)), "24:59");
        timer.pause(Duration::from_secs(10));
        let frozen = timer.remaining(Duration::from_secs(10));
        timer.pause(Duration::from_secs(90));
        assert_eq!(timer.remaining(Duration::from_secs(90)), frozen);
        timer.resume(Duration::from_secs(100));
        timer.end(Duration::from_secs(101));
        assert_eq!(timer.minutes_input, "00025");
        assert!(timer.is_idle());
    }

    #[test]
    fn wo003_early_end_restores_captured_text_from_running_and_paused_states() {
        let mut running = start("00025");
        running.start_countdown(Duration::ZERO);
        running.end(Duration::from_secs(2));
        assert!(running.is_idle());
        assert_eq!(running.minutes_input, "00025");

        let mut paused = start("00025");
        paused.start_countdown(Duration::ZERO);
        paused.pause(Duration::from_secs(2));
        paused.end(Duration::from_secs(10_000));
        assert!(paused.is_idle());
        assert_eq!(paused.minutes_input, "00025");
    }

    #[test]
    fn wo003_completion_is_once_per_generation_and_stale_results_are_ignored() {
        let mut timer = start("1");
        let mut sink = RecordingSink {
            result: NotificationResult::Pending,
            ..Default::default()
        };
        timer.start_countdown(Duration::ZERO);
        let notification = timer
            .reconcile_with_sink(Duration::from_secs(60), &mut sink)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(
            timer.completed_message().as_deref(),
            Some("Great Work! You completed a 1 Minute Sprint")
        );
        assert_eq!(timer.primary_label(), "Reset");
        assert!(!timer.end_enabled());
        assert_eq!(sink.requests.len(), 1);
        assert!(timer
            .reconcile_with_sink(Duration::from_secs(600), &mut sink)
            .is_none());
        timer.reset();
        assert_eq!(timer.minutes_input, "25");
        assert!(
            !timer.apply_notification_result(notification.generation, NotificationResult::Accepted)
        );

        timer.set_minutes_input("2".to_owned());
        timer.start_countdown(Duration::from_secs(700));
        let second = timer
            .reconcile_with_sink(Duration::from_secs(820), &mut sink)
            .unwrap_or_else(|| unreachable!());
        assert_ne!(notification.generation, second.generation);
        assert_eq!(sink.requests.len(), 2);
        assert_eq!(
            timer.completed_message().as_deref(),
            Some("Great Work! You completed a 2 Minutes Sprint")
        );
    }

    #[test]
    fn wo003_expiry_precedes_pause_end_and_reentrant_actions() {
        let mut timer = start("1");
        timer.start_countdown(Duration::ZERO);
        assert!(timer.pause(Duration::from_secs(60)).is_some());
        assert!(timer.is_completed());
        assert!(timer.end(Duration::from_secs(60)).is_none());
        assert!(timer.is_completed());
        assert!(timer.start_countdown(Duration::from_secs(60)).is_none());
        assert_eq!(timer.primary_label(), "Reset");
    }

    #[test]
    fn wo003_stopwatch_saturates_formats_long_hours_and_resumes_from_accumulation() {
        let mut timer = SprintTimer::default();
        timer.select_mode(TimerMode::Stopwatch);
        assert_eq!(timer.stopwatch_display(Duration::ZERO), "00:00:00");
        timer.start_stopwatch(Duration::ZERO);
        assert_eq!(
            timer.stopwatch_display(Duration::from_secs(3_600)),
            "01:00:00"
        );
        timer.pause(Duration::from_secs(3_600));
        assert_eq!(
            timer.stopwatch_display(Duration::from_secs(3_600 + 99 * 3_600)),
            "01:00:00"
        );
        timer.resume(Duration::from_secs(1_000_000));
        assert_eq!(
            timer.stopwatch_display(Duration::from_secs(1_000_000 + 99 * 3_600)),
            "100:00:00"
        );
        timer.end(Duration::from_secs(u64::MAX));
        assert_eq!(
            timer.stopwatch_display(Duration::from_secs(u64::MAX)),
            "00:00:00"
        );
        assert!(timer.is_idle());
    }

    #[test]
    fn wo003_mode_switching_is_idle_only_and_closing_respects_lifetime_rules() {
        let mut timer = SprintTimer::default();
        timer.set_minutes_input("42".to_owned());
        assert!(timer.select_mode(TimerMode::Stopwatch));
        assert_eq!(timer.mode, TimerMode::Stopwatch);
        assert_eq!(timer.minutes_input, "25");
        timer.start_stopwatch(Duration::ZERO);
        assert!(!timer.mode_switch_enabled());
        assert!(!timer.select_mode(TimerMode::Countdown));
        timer.close_popup();
        assert!(timer.is_running_or_paused());
        timer.pause(Duration::from_secs(10));
        timer.close_popup();
        assert!(timer.is_running_or_paused());
        timer.end(Duration::from_secs(10));
        timer.close_popup();
        assert!(timer.is_idle());
        assert!(timer.select_mode(TimerMode::Countdown));
        timer.set_minutes_input("4".to_owned());
        timer.start_countdown(Duration::ZERO);
        timer.reconcile(Duration::from_secs(240));
        assert!(timer.is_completed());
        assert!(!timer.mode_switch_enabled());
        timer.close_popup();
        assert_eq!(timer.mode, TimerMode::Countdown);
        assert_eq!(timer.minutes_input, "25");
        assert!(timer.is_idle());
    }

    #[test]
    fn wo003_repaint_delays_are_bounded_and_stop_when_not_running() {
        let mut timer = start("1");
        assert!(timer.next_repaint_delay(Duration::ZERO).is_none());
        timer.start_countdown(Duration::ZERO);
        let delay = timer
            .next_repaint_delay(Duration::from_millis(1))
            .unwrap_or_default();
        assert!(delay > Duration::ZERO && delay <= MAX_REPAINT_DELAY);
        timer.pause(Duration::from_secs(2));
        assert!(timer.next_repaint_delay(Duration::from_secs(2)).is_none());
        timer.resume(Duration::from_secs(3));
        assert!(timer.next_repaint_delay(Duration::from_secs(3)).is_some());
        timer.end(Duration::from_secs(4));
        assert!(timer.next_repaint_delay(Duration::from_secs(4)).is_none());
    }

    #[test]
    fn wo003_deadline_and_resume_arithmetic_reject_overflow_without_panicking() {
        let mut timer = start("1");
        timer.start_countdown(Duration::MAX);
        assert!(timer.is_idle());
        assert!(timer.validation_error.is_some());

        timer.start_countdown(Duration::ZERO);
        timer.pause(Duration::from_secs(1));
        timer.resume(Duration::MAX);
        assert!(matches!(timer.state, TimerState::CountdownPaused { .. }));
        assert!(timer.validation_error.is_some());

        let mut stopwatch = SprintTimer::default();
        stopwatch.select_mode(TimerMode::Stopwatch);
        stopwatch.start_stopwatch(Duration::MAX);
        assert_eq!(stopwatch.stopwatch_display(Duration::MAX), "00:00:00");
        assert!(stopwatch.next_repaint_delay(Duration::MAX).is_some());
    }

    #[test]
    fn wo003_notification_failure_keeps_completion_and_reset_available() {
        let mut timer = start("1");
        let mut sink = RecordingSink {
            result: NotificationResult::Unavailable,
            ..Default::default()
        };
        timer.start_countdown(Duration::ZERO);
        timer.reconcile_with_sink(Duration::from_secs(60), &mut sink);
        assert_eq!(
            timer.notification_feedback,
            Some(NotificationFeedback::Unavailable)
        );
        assert!(timer.completed_message().is_some());
        assert_eq!(timer.primary_label(), "Reset");
        timer.reset();
        assert!(timer.is_idle());
        assert_eq!(timer.minutes_input, "25");
    }
}

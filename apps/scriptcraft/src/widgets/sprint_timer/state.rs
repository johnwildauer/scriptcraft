use super::animation::PopupClosingTransition;
use super::geometry::SprintTimerPopupPlacement;
use super::notifications::{CompletionNotification, NotificationResult};
use eframe::egui;
use scriptcraft_commands::Command;
use std::time::Duration;

const DEFAULT_MINUTES: &str = "25";
const MIN_MINUTES: u16 = 1;
const MAX_MINUTES: u16 = 1_440;
const MIN_REPAINT_DELAY: Duration = Duration::from_millis(10);
const MAX_REPAINT_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum TimerMode {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NotificationFeedback {
    Pending,
    Accepted,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SprintTimer {
    pub(crate) mode: TimerMode,
    state: TimerState,
    pub(crate) minutes_input: String,
    pub(crate) validation_error: Option<String>,
    pub(crate) notification_feedback: Option<NotificationFeedback>,
    next_generation: u64,
    pub(crate) popup_open: bool,
    pub(crate) popup_rect: egui::Rect,
    pub(crate) popup_anchor: egui::Rect,
    pub(crate) just_opened: bool,
    pub(super) opening_started_at: Option<f64>,
    pub(super) closing: Option<PopupClosingTransition>,
    pub(super) popup_rest_placement: Option<SprintTimerPopupPlacement>,
    pub(super) close_completion_pending: bool,
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
    pub(crate) fn is_idle(&self) -> bool {
        matches!(self.state, TimerState::Idle)
    }

    pub(crate) fn is_running_or_paused(&self) -> bool {
        matches!(
            self.state,
            TimerState::CountdownRunning { .. }
                | TimerState::CountdownPaused { .. }
                | TimerState::StopwatchRunning { .. }
                | TimerState::StopwatchPaused { .. }
        )
    }

    pub(crate) fn is_completed(&self) -> bool {
        matches!(self.state, TimerState::CountdownCompleted { .. })
    }

    pub(crate) fn mode_switch_enabled(&self) -> bool {
        self.is_idle()
    }

    pub(crate) fn set_minutes_input(&mut self, value: String) {
        if self.mode == TimerMode::Countdown && self.is_idle() {
            self.minutes_input = value.chars().filter(char::is_ascii_digit).collect();
            self.validation_error = None;
        }
    }

    pub(crate) fn remaining(&self, now: Duration) -> Option<Duration> {
        match &self.state {
            TimerState::CountdownRunning { deadline, .. } => Some(deadline.saturating_sub(now)),
            TimerState::CountdownPaused { remaining, .. } => Some(*remaining),
            _ => None,
        }
    }

    pub(crate) fn elapsed(&self, now: Duration) -> Option<Duration> {
        match self.state {
            TimerState::StopwatchRunning {
                started_at,
                accumulated,
            } => Some(accumulated.saturating_add(now.saturating_sub(started_at))),
            TimerState::StopwatchPaused { elapsed } => Some(elapsed),
            _ => None,
        }
    }

    pub(crate) fn countdown_display(&self, now: Duration) -> String {
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

    pub(crate) fn stopwatch_display(&self, now: Duration) -> String {
        let seconds = self.elapsed(now).unwrap_or(Duration::ZERO).as_secs();
        format!(
            "{:02}:{:02}:{:02}",
            seconds / 3_600,
            seconds / 60 % 60,
            seconds % 60
        )
    }

    pub(crate) fn completed_message(&self) -> Option<String> {
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

    pub(crate) fn end_enabled(&self) -> bool {
        self.is_running_or_paused()
    }

    pub(crate) fn primary_label(&self) -> &'static str {
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

    pub(crate) fn primary_command(&self) -> Command {
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

    pub(crate) fn reconcile(&mut self, now: Duration) -> Option<CompletionNotification> {
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

    pub(crate) fn apply_notification_result(
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

    pub(crate) fn select_mode(&mut self, mode: TimerMode) -> bool {
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

    pub(crate) fn start_countdown(&mut self, now: Duration) -> Option<CompletionNotification> {
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

    pub(crate) fn start_stopwatch(&mut self, now: Duration) -> Option<CompletionNotification> {
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

    pub(crate) fn pause(&mut self, now: Duration) -> Option<CompletionNotification> {
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

    pub(crate) fn resume(&mut self, now: Duration) -> Option<CompletionNotification> {
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

    pub(crate) fn end(&mut self, now: Duration) -> Option<CompletionNotification> {
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

    pub(crate) fn reset(&mut self) {
        if self.is_completed() {
            self.mode = TimerMode::Countdown;
            self.state = TimerState::Idle;
            self.minutes_input = DEFAULT_MINUTES.to_owned();
            self.validation_error = None;
            self.notification_feedback = None;
        }
    }

    pub(crate) fn close_popup(&mut self) {
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

    pub(crate) fn next_repaint_delay(&self, now: Duration) -> Option<Duration> {
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

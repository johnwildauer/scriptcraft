use crate::app::ScriptCraftApp;
use crate::widgets::sprint_timer::notifications::NotificationSink;
use eframe::egui;
use scriptcraft_commands::{Command, CommandRejection, EditorSurface, SprintTimerMode};

use super::DesktopCommandOutcome;

fn close_sprint_timer_popup_once(timer: &mut crate::widgets::sprint_timer::SprintTimer) -> bool {
    if !timer.popup_open {
        return false;
    }
    timer.popup_open = false;
    timer.close_popup();
    true
}

fn open_sprint_timer_popup(app: &mut ScriptCraftApp, ctx: &egui::Context) -> bool {
    if app.sprint_timer.popup_open && !app.sprint_timer.is_closing() {
        return false;
    }
    app.sprint_timer.clear_popup_transition(ctx);
    app.clear_sprint_timer_popup_geometry();
    app.sprint_timer.popup_open = true;
    app.sprint_timer.just_opened = true;
    app.sprint_timer
        .start_popup_opening(ctx.input(|input| input.time));
    true
}

impl ScriptCraftApp {
    pub(crate) fn attach_sprint_timer_popup_to_surface(
        &mut self,
        anchor: egui::Rect,
        surface: EditorSurface,
        ctx: &egui::Context,
    ) {
        self.sprint_timer.popup_anchor = anchor;
        self.timer_popup_surface = surface;
        self.capture_sprint_timer_host_geometry(surface, ctx);
    }

    pub(crate) fn open_pending_sprint_timer_completion(
        &mut self,
        surface: EditorSurface,
        anchor: Option<egui::Rect>,
        ctx: &egui::Context,
    ) -> bool {
        if self.timer_popup_surface != surface
            || !self.sprint_timer.has_completion_popup_open_request()
        {
            return false;
        }

        if self.sprint_timer.popup_open && !self.sprint_timer.is_closing() {
            self.sprint_timer.take_completion_popup_open_request();
            ctx.request_repaint();
            return true;
        }

        let Some(anchor) = anchor.filter(|rect| rect.is_positive()) else {
            return false;
        };
        open_sprint_timer_popup(self, ctx);
        self.attach_sprint_timer_popup_to_surface(anchor, surface, ctx);
        self.sprint_timer.take_completion_popup_open_request();
        ctx.request_repaint();
        true
    }

    pub(super) fn dispatch_timer_command(
        &mut self,
        command: Command,
        ctx: &egui::Context,
    ) -> DesktopCommandOutcome {
        match command {
            Command::ToolsSprintTimerTogglePopup => {
                if self.sprint_timer.popup_open {
                    self.sprint_timer
                        .request_popup_close(ctx.input(|input| input.time), ctx.content_rect());
                } else {
                    open_sprint_timer_popup(self, ctx);
                }
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ToolsSprintTimerClosePopup => {
                self.sprint_timer
                    .request_popup_close(ctx.input(|input| input.time), ctx.content_rect());
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ToolsSprintTimerSetMode { mode } => {
                let mode = match mode {
                    SprintTimerMode::Countdown => {
                        crate::widgets::sprint_timer::TimerMode::Countdown
                    }
                    SprintTimerMode::Stopwatch => {
                        crate::widgets::sprint_timer::TimerMode::Stopwatch
                    }
                };
                self.sprint_timer.select_mode(mode);
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ToolsSprintTimerSetMinutes { text } => {
                self.sprint_timer.set_minutes_input(text);
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ToolsSprintTimerStart => {
                let now = self.timer_now();
                let notification = match self.sprint_timer.mode {
                    crate::widgets::sprint_timer::TimerMode::Countdown => {
                        self.sprint_timer.start_countdown(now)
                    }
                    crate::widgets::sprint_timer::TimerMode::Stopwatch => {
                        self.sprint_timer.start_stopwatch(now)
                    }
                };
                self.request_timer_notification(notification);
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ToolsSprintTimerPause => {
                let notification = self.sprint_timer.pause(self.timer_now());
                self.request_timer_notification(notification);
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ToolsSprintTimerResume => {
                let notification = self.sprint_timer.resume(self.timer_now());
                self.request_timer_notification(notification);
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ToolsSprintTimerReset => {
                self.sprint_timer.reset();
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ToolsSprintTimerEnd => {
                let notification = self.sprint_timer.end(self.timer_now());
                self.request_timer_notification(notification);
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            other => {
                DesktopCommandOutcome::Rejected(CommandRejection::UnsupportedCommand(other.id()))
            }
        }
    }

    pub(crate) fn request_timer_notification(
        &mut self,
        notification: Option<crate::widgets::sprint_timer::notifications::CompletionNotification>,
    ) {
        if let Some(notification) = notification {
            let result =
                DesktopTimerNotificationEffects::request(&mut self.notification_sink, notification);
            DesktopTimerNotificationEffects::apply_result(
                &mut self.sprint_timer,
                notification.generation,
                result,
            );
        }
    }

    pub(crate) fn apply_timer_notification_result(
        &mut self,
        generation: u64,
        result: crate::widgets::sprint_timer::notifications::NotificationResult,
    ) -> bool {
        DesktopTimerNotificationEffects::apply_result(&mut self.sprint_timer, generation, result)
    }

    /// Completes an app-owned popup transition, or immediately tears it down
    /// when its viewport no longer has a drawable surface. Typed close commands
    /// never call this path; they only request the ordinary animated dismissal.
    pub(crate) fn finalize_sprint_timer_popup(&mut self, ctx: &egui::Context) -> bool {
        let finalized = close_sprint_timer_popup_once(&mut self.sprint_timer);
        self.sprint_timer.clear_popup_transition(ctx);
        self.clear_sprint_timer_popup_geometry();
        ctx.request_repaint();
        finalized
    }
}

/// Completion notification requests and asynchronous results remain outside
/// command payloads; this adapter preserves the existing native sink contract.
struct DesktopTimerNotificationEffects;

impl DesktopTimerNotificationEffects {
    fn request(
        sink: &mut crate::widgets::sprint_timer::notifications::NativeNotificationSink,
        notification: crate::widgets::sprint_timer::notifications::CompletionNotification,
    ) -> crate::widgets::sprint_timer::notifications::NotificationResult {
        sink.request(notification)
    }

    fn apply_result(
        timer: &mut crate::widgets::sprint_timer::SprintTimer,
        generation: u64,
        result: crate::widgets::sprint_timer::notifications::NotificationResult,
    ) -> bool {
        timer.apply_notification_result(generation, result)
    }
}

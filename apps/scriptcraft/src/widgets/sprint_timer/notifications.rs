use crate::app::ScriptCraftApp;
use eframe::egui;
use std::process::Command as ProcessCommand;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CompletionNotification {
    pub(crate) generation: u64,
    pub(crate) minutes: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum NotificationResult {
    #[default]
    Pending,
    Accepted,
    Unavailable,
}

pub(crate) trait NotificationSink {
    fn request(&mut self, notice: CompletionNotification) -> NotificationResult;
}

pub(crate) struct NativeNotificationSink {
    results_tx: Sender<(u64, NotificationResult)>,
    repaint_context: Option<egui::Context>,
}

impl NativeNotificationSink {
    pub(crate) fn new() -> (Self, Receiver<(u64, NotificationResult)>) {
        let (results_tx, results_rx) = mpsc::channel();
        (
            Self {
                results_tx,
                repaint_context: None,
            },
            results_rx,
        )
    }

    pub(crate) fn set_repaint_context(&mut self, ctx: &egui::Context) {
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

impl ScriptCraftApp {
    pub(crate) fn reconcile_sprint_timer(&mut self, now: Duration) {
        let notification = self.sprint_timer.reconcile(now);
        self.request_timer_notification(notification);
        while let Ok((generation, result)) = self.notification_results.try_recv() {
            self.apply_timer_notification_result(generation, result);
        }
    }
}

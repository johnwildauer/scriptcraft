//! Transient desktop decisions and coordination for root-document lifecycle operations.
//!
//! [`UnsavedChangesChoice::DontSave`] is memory-only: it never reverts a
//! session, clears semantic dirty state, or changes a file. A confirmed close
//! is authorized immediately but only for the next root update's close
//! request; it is consumed once or expires if that update has no request.

use crate::app::ScriptCraftApp;
use crate::documents::replacement::should_cancel_replacement_root_close;
use eframe::egui;

/// The user's response to the native unsaved-changes prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnsavedChangesChoice {
    Save,
    DontSave,
    Cancel,
}

/// The typed result of attempting to save a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SaveOutcome {
    Saved,
    Cancelled,
    Failed,
}

/// A lifecycle continuation requires both a successful save and a clean live
/// session; a successful result alone is not sufficient authorization.
pub(crate) fn clean_save_outcome(outcome: SaveOutcome, is_dirty: bool) -> SaveOutcome {
    match (outcome, is_dirty) {
        (SaveOutcome::Saved, false) => SaveOutcome::Saved,
        (SaveOutcome::Cancelled, _) => SaveOutcome::Cancelled,
        (SaveOutcome::Failed, _) | (SaveOutcome::Saved, true) => SaveOutcome::Failed,
    }
}

/// A root-document close observation, with dirty state supplied by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RootCloseRequest {
    pub(crate) is_dirty: bool,
}

/// The coordinator's pure decision for an observed root-document close.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RootCloseDecision {
    /// Permit an ordinary clean close.
    Allow,
    /// The caller must cancel the native close and request a user decision.
    CancelAndPrompt,
    /// Permit the one close authorized by the immediately preceding choice.
    AllowAuthorized,
}

/// One-shot, transient authorization for a confirmed root-document close.
#[derive(Debug, Default)]
pub(crate) struct RootCloseLifecycle {
    confirmed_close_armed: bool,
}

impl RootCloseLifecycle {
    /// Arms an in-memory authorization for the next root-update close request.
    /// This does not access or mutate the document session or its dirty state.
    pub(crate) fn arm_confirmed_close(&mut self) {
        self.confirmed_close_armed = true;
    }

    /// Resolves the close observation at the start of a root update.
    ///
    /// When authorization is armed, that authorization is consumed whether or
    /// not this update reports a close request. Thus it cannot survive beyond
    /// the next update, and a missing request expires it. Dirty state is always
    /// supplied by the caller (the live `EditingSession::dirty()` authority).
    pub(crate) fn begin_root_update(
        &mut self,
        request: Option<RootCloseRequest>,
    ) -> Option<RootCloseDecision> {
        if self.confirmed_close_armed {
            self.confirmed_close_armed = false;
            return request.map(|_| RootCloseDecision::AllowAuthorized);
        }

        request.map(|request| {
            if request.is_dirty {
                RootCloseDecision::CancelAndPrompt
            } else {
                RootCloseDecision::Allow
            }
        })
    }
}

/// Whether a prompt response authorizes reissuing root Close. Save must have
/// completed successfully and left the live session clean; Don't Save is purely
/// an in-memory confirmation and does not change the session.
pub(crate) fn confirmed_close_for_choice(
    choice: UnsavedChangesChoice,
    save_outcome: Option<SaveOutcome>,
) -> bool {
    match choice {
        UnsavedChangesChoice::Cancel => false,
        UnsavedChangesChoice::DontSave => true,
        UnsavedChangesChoice::Save => save_outcome == Some(SaveOutcome::Saved),
    }
}

fn root_close_cleanup_allowed(decision: RootCloseDecision) -> bool {
    matches!(
        decision,
        RootCloseDecision::Allow | RootCloseDecision::AllowAuthorized
    )
}

/// Bounded native error presentation used by lifecycle and document workflows.
pub(crate) fn show_native_lifecycle_error(title: &str, message: &str) {
    let _ = rfd::MessageDialog::new()
        .set_title(title)
        .set_description(message)
        .set_level(rfd::MessageLevel::Error)
        .show();
}

impl ScriptCraftApp {
    fn reset_element_windows_for_root_close(&mut self, decision: RootCloseDecision) {
        if root_close_cleanup_allowed(decision) {
            self.element_windows.reset();
            self.command_projection_cache = None;
        }
    }

    /// Observes root Close before normal update work. A dirty unauthorized close
    /// is cancelled before prompting; confirmed choices request a fresh Close
    /// guarded by one authorization consumed at the next root update boundary.
    /// Replacement pending cancels root Close/Quit separately and never counts
    /// as the child's Opened acknowledgement.
    pub(crate) fn begin_root_close_update(
        &mut self,
        ctx: &egui::Context,
        frame: &eframe::Frame,
    ) -> bool {
        let close_requested = ctx.input(|input| input.viewport().close_requested());
        if self.replacement_open_pending() {
            // A native Close/Quit is not a replacement acknowledgement. Expire
            // any prior one-shot authorization and cancel this root close while
            // the transient replacement lock owns the document.
            let _ = self.root_close_lifecycle.begin_root_update(None);
            if should_cancel_replacement_root_close(true, close_requested) {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
            return false;
        }
        let request = close_requested.then(|| RootCloseRequest {
            is_dirty: self.session.dirty(),
        });
        let Some(decision) = self.root_close_lifecycle.begin_root_update(request) else {
            return false;
        };

        match decision {
            RootCloseDecision::Allow | RootCloseDecision::AllowAuthorized => {
                // Only an actually permitted root close discards transient
                // dialog state. A canceled dirty close keeps it intact.
                self.reset_element_windows_for_root_close(decision);
                root_close_cleanup_allowed(decision)
            }
            RootCloseDecision::CancelAndPrompt => {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);

                let availability = self.app_command_availability();
                if availability.modal_blocked || availability.native_output_active {
                    return false;
                }

                let choice =
                    super::unsaved_changes::prompt_unsaved_changes(&self.native_title, frame);
                let save_outcome = if matches!(choice, UnsavedChangesChoice::Save) {
                    Some(self.save_for_lifecycle(ctx))
                } else {
                    None
                };
                if confirmed_close_for_choice(choice, save_outcome) {
                    self.arm_and_request_root_close(ctx);
                }
                false
            }
        }
    }

    /// Saves for a lifecycle decision; only a clean live session yields Saved.
    pub(crate) fn save_for_lifecycle(&mut self, ctx: &egui::Context) -> SaveOutcome {
        let outcome = self.save_native(ctx);
        let is_dirty = self.session.dirty();
        let lifecycle_outcome = clean_save_outcome(outcome, is_dirty);
        if outcome == SaveOutcome::Saved && is_dirty {
            self.show_error(
                "LIFECYCLE_SAVE_REPORTED_SUCCESS_BUT_DIRTY: the live session remains dirty after save",
            );
        }
        if lifecycle_outcome == SaveOutcome::Failed {
            show_native_lifecycle_error(
                "Could Not Save Document",
                "The document could not be saved. It remains open, and its unsaved changes have been preserved.",
            );
        }
        lifecycle_outcome
    }
}

#[cfg(test)]
mod element_window_root_close_tests {
    use super::*;
    use crate::widgets::elements::state::{ElementWindowState, TemplateSelectorMode};
    use scriptcraft_commands::EditorSurface;

    fn app() -> ScriptCraftApp {
        ScriptCraftApp::new_for_template_lifecycle_tests()
            .unwrap_or_else(|error| panic!("prepared app setup failed: {error}"))
    }

    #[test]
    fn canceled_root_close_retains_dialog_but_authorized_close_resets_it() {
        let mut app = app();
        app.element_windows.open_elements(EditorSurface::TitlePage);
        app.element_windows
            .open_selector(TemplateSelectorMode::ApplyToCurrent);
        let before = app.element_windows;

        app.reset_element_windows_for_root_close(RootCloseDecision::CancelAndPrompt);
        assert_eq!(app.element_windows, before);

        app.reset_element_windows_for_root_close(RootCloseDecision::AllowAuthorized);
        assert_eq!(app.element_windows, ElementWindowState::default());
    }
}

//! Parent-side coordination for acknowledged replacement opens.

use crate::app::ScriptCraftApp;
use crate::documents::{
    lifecycle::{self, show_native_lifecycle_error},
    replacement_protocol as replacement_open,
    replacement_transport as replacement_open_transport,
    unsaved_changes,
};
use crate::chrome::menu;
use eframe::egui;
use scriptcraft_commands::EditorSurface;
use scriptcraft_model::cast_list::CAST_LIST_REFRESH_INTERVAL;
use std::ffi::OsString;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChildLaunchError {
    InvalidDocumentPath,
    ExecutableUnavailable,
    SpawnFailed,
}

pub(super) fn child_arguments(
    development: bool,
    menu_platform: menu::MenuPlatformChoice,
    document_path: Option<&Path>,
) -> Result<Vec<OsString>, ChildLaunchError> {
    let mut arguments = vec![crate::app::startup::menu_platform_argument(menu_platform)];
    if development {
        arguments.push(OsString::from("--dev"));
    }
    if let Some(path) = document_path {
        let Some(path) = path.to_str().filter(|path| !path.is_empty()) else {
            return Err(ChildLaunchError::InvalidDocumentPath);
        };
        arguments.push(OsString::from(format!("--document-path={path}")));
    }
    Ok(arguments)
}

fn launch_child_with(
    executable: &Path,
    arguments: &[OsString],
    spawn: impl FnOnce(&Path, &[OsString]) -> std::io::Result<()>,
) -> Result<(), ChildLaunchError> {
    spawn(executable, arguments).map_err(|_| ChildLaunchError::SpawnFailed)
}

fn launch_child_for_app(
    app: &ScriptCraftApp,
    executable: &Path,
    document_path: Option<&Path>,
    spawn: impl FnOnce(&Path, &[OsString]) -> std::io::Result<()>,
) -> Result<(), ChildLaunchError> {
    let arguments = child_arguments(app.development_mode, app.menu_platform_choice, document_path)?;
    launch_child_with(executable, &arguments, spawn)
}

fn spawn_current_child(
    app: &ScriptCraftApp,
    document_path: Option<&Path>,
) -> Result<(), ChildLaunchError> {
    let executable =
        std::env::current_exe().map_err(|_| ChildLaunchError::ExecutableUnavailable)?;
    launch_child_for_app(app, &executable, document_path, |executable, arguments| {
        std::process::Command::new(executable)
            .args(arguments)
            .spawn()
            .map(|_child| ())
    })
}

/// One active replacement handoff owns its identity, deadline, and child
/// transport as a single record. Dropping it never kills or waits for child.
pub(crate) struct PendingReplacement {
    metadata: replacement_open::PendingReplacementMetadata,
    transport: replacement_open_transport::ReplacementOpenTransport,
}

pub(crate) fn should_cancel_replacement_root_close(
    replacement_pending: bool,
    close_requested: bool,
) -> bool {
    replacement_pending && close_requested
}

fn replacement_resume_surface(
    prior_surface: Option<EditorSurface>,
    title_page_open: bool,
) -> Option<EditorSurface> {
    match prior_surface {
        Some(EditorSurface::TitlePage) if !title_page_open => Some(EditorSurface::Body),
        surface => surface,
    }
}

pub(crate) fn replacement_resume_focus_allowed(
    modal_blocked: bool,
    native_output_active: bool,
) -> bool {
    !modal_blocked && !native_output_active
}

fn replacement_resume_feedback(
    reason: replacement_open::ReplacementOpenResumeReason,
) -> (&'static str, &'static str) {
    if reason == replacement_open::ReplacementOpenResumeReason::ChildCancelled {
        (
            "Document Not Opened",
            "The selected document was not opened. The current document and its in-memory changes remain available.",
        )
    } else {
        (
            "Could Not Open Document",
            "The selected document could not be opened. The current document and its in-memory changes remain available.",
        )
    }
}

impl ScriptCraftApp {
    /// The replacement lock is owned only by the WO-002.5 pending record.
    /// Keep this separate from `pending_fdx_open`, which represents the
    /// migration decision and alone enables File Import Resolve.
    pub(crate) fn replacement_open_pending(&self) -> bool {
        self.pending_replacement.is_some()
    }

    pub(crate) fn launch_child_instance(
        &self,
        document_path: Option<&Path>,
    ) -> Result<(), ChildLaunchError> {
        spawn_current_child(self, document_path)
    }

    pub(crate) fn launch_blank_instance(&self) {
        if self.launch_child_instance(None).is_err() {
            show_native_lifecycle_error(
                "Unable to Start ScriptCraft",
                "A separate blank ScriptCraft instance could not be started. The current document is unchanged.",
            );
        }
    }

    pub(crate) fn process_staged_replacement_open(
        &mut self,
        ctx: &egui::Context,
        frame: &eframe::Frame,
    ) {
        let Some(path) = self.staged_replacement_open.take() else {
            return;
        };
        if self.replacement_open_pending() || self.pending_fdx_open.is_some() {
            self.show_replacement_launch_failure();
            return;
        }

        let is_dirty = self.session.dirty();
        let choice = is_dirty.then(|| {
            unsaved_changes::prompt_unsaved_changes(&self.native_title, frame)
        });
        let save_outcome = if choice == Some(lifecycle::UnsavedChangesChoice::Save) {
            Some(self.save_for_lifecycle(ctx))
        } else {
            None
        };
        if !replacement_open::replacement_open_gate_allows(is_dirty, choice, save_outcome) {
            return;
        }

        let handoff_id = match self.replacement_handoff_ids.next_id() {
            Ok(id) => id,
            Err(_) => {
                self.show_replacement_launch_failure();
                return;
            }
        };
        let executable = match std::env::current_exe() {
            Ok(executable) => executable,
            Err(_) => {
                self.show_replacement_launch_failure();
                return;
            }
        };
        let transport = replacement_open_transport::spawn_acknowledged_replacement(
            &executable,
            self.development_mode,
            self.menu_platform_choice,
            &path,
            handoff_id,
            &self.replacement_open_event_sender,
            ctx,
        );
        match transport {
            Ok(transport) => {
                if !self.retain_pending_replacement(transport, Instant::now()) {
                    self.show_replacement_launch_failure();
                }
            }
            Err(_) => self.show_replacement_launch_failure(),
        }
    }

    fn show_replacement_launch_failure(&self) {
        show_native_lifecycle_error(
            "Unable to Open Document",
            "The selected document could not be opened. The current document and its in-memory changes remain available.",
        );
    }

    /// Retains the sole successful acknowledged spawn. The transport and its
    /// identity are stored together; a second child cannot replace an active
    /// handoff. A rejected transport is merely dropped (never killed/waited).
    fn retain_pending_replacement(
        &mut self,
        transport: replacement_open_transport::ReplacementOpenTransport,
        now: Instant,
    ) -> bool {
        if self.pending_replacement.is_some() {
            return false;
        }
        self.replacement_resume_surface = Some(self.active_editor_surface());
        // This is transient Body presentation state, not session selection or
        // semantic content; stop reconciling it while both editor inputs lock.
        self.set_active_body_group(None);
        let metadata = replacement_open::PendingReplacementMetadata::new(
            transport.handoff_id,
            now,
        );
        self.pending_replacement = Some(PendingReplacement {
            metadata,
            transport,
        });
        true
    }

    /// Reuses the WO-001 transient authorization and ordinary root Close path;
    /// this does not save, revert, clean, or synchronously destroy the session.
    pub(crate) fn arm_and_request_root_close(&mut self, ctx: &egui::Context) {
        self.root_close_lifecycle.arm_confirmed_close();
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// Drains all queued events before the nonblocking child poll, then applies
    /// one terminal decision. Matching Opened arms and requests the next
    /// WO-001 one-shot root Close; every other result resumes with feedback.
    pub(crate) fn poll_pending_replacement(
        &mut self,
        ctx: &egui::Context,
        now: Instant,
    ) -> bool {
        let events: Vec<_> = self.replacement_open_events.try_iter().collect();
        let child_observation = match self.pending_replacement.as_mut() {
            Some(pending) => match pending.transport.try_wait() {
                Ok(Some(_status)) => replacement_open::ReplacementOpenChildObservation::Exited,
                Ok(None) => replacement_open::ReplacementOpenChildObservation::Running,
                Err(_) => replacement_open::ReplacementOpenChildObservation::PollFailed,
            },
            None => replacement_open::ReplacementOpenChildObservation::Running,
        };
        let decision = replacement_open::decide_pending_replacement(
            self.pending_replacement
                .as_ref()
                .map(|pending| &pending.metadata),
            now,
            events,
            child_observation,
        );

        match decision {
            replacement_open::PendingReplacementDecision::Inactive => false,
            replacement_open::PendingReplacementDecision::Pending { repaint_after } => {
                self.performance.repaint_requests =
                    self.performance.repaint_requests.saturating_add(1);
                ctx.request_repaint_after(repaint_after);
                false
            }
            replacement_open::PendingReplacementDecision::AuthorizeClose => {
                // Reject commands queued during the locked interval before
                // clearing the record, so a missed one-shot Close cannot replay
                // them as ordinary unlocked commands on a later frame.
                self.flush_pending_diagnostic_commands(ctx);
                self.dispatch_pending_commands(ctx);
                // Clear the entire record before requesting the WO-001
                // continuation. Dropping the child handle never kills/waits.
                self.pending_replacement = None;
                self.replacement_resume_surface = None;
                self.arm_and_request_root_close(ctx);
                true
            }
            replacement_open::PendingReplacementDecision::Resume(reason) => {
                // Drain the entire queued batch under locked availability before
                // exposing the resumed command state; no catch-up replay.
                self.flush_pending_diagnostic_commands(ctx);
                self.dispatch_pending_commands(ctx);
                // Clearing precedes any future feedback/focus restoration. No
                // session, file baseline, title, selection, or dirty state is
                // touched. Avoid an overdue derived-work catch-up this frame.
                self.pending_replacement = None;
                let (title, message) = replacement_resume_feedback(reason);
                show_native_lifecycle_error(title, message);
                self.next_cast_list_refresh = now
                    .checked_add(CAST_LIST_REFRESH_INTERVAL)
                    .unwrap_or(now);
                let availability = self.app_command_availability();
                let prior_surface = self.replacement_resume_surface.take();
                let resume_surface = replacement_resume_surface(
                    prior_surface,
                    self.title_page_window_open,
                );
                if let Some(surface) = resume_surface {
                    self.set_active_editor_surface(surface);
                    self.reconcile_body_active_group();
                    if replacement_resume_focus_allowed(
                        availability.modal_blocked,
                        availability.native_output_active,
                    ) {
                        self.restore_editor_focus(ctx);
                    }
                }
                false
            }
        }
    }
}

//! Native Save/Save As coordination and document identity projection.

use crate::app::ScriptCraftApp;
use crate::commands::{self, CommandInvocation};
use crate::documents::lifecycle::SaveOutcome;
use crate::views::title_page;
use eframe::egui;
use scriptcraft_commands::{Command, CommandOrigin, DocumentPath};
use scriptcraft_fdx as fdx;
use scriptcraft_layout::LayoutCache;
use scriptcraft_model::cast_list::CAST_LIST_REFRESH_INTERVAL;
use scriptcraft_session::storage::{
    self as fdx_storage, FdxBaseline, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem,
};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub(crate) const UNTITLED_DOCUMENT_TITLE: &str = "Untitled Screenplay";

/// Converts an optional Save As picker result into its typed save result.
/// Picker cancellation is not a dispatched save and is never reported as Saved.
fn save_picker_outcome<T>(
    selection: Option<T>,
    save: impl FnOnce(T) -> SaveOutcome,
) -> SaveOutcome {
    match selection {
        Some(selection) => save(selection),
        None => SaveOutcome::Cancelled,
    }
}

fn native_document_title(path: Option<&Path>) -> String {
    path.and_then(Path::file_name)
        .and_then(std::ffi::OsStr::to_str)
        .filter(|file_name| !file_name.is_empty())
        .unwrap_or(UNTITLED_DOCUMENT_TITLE)
        .to_owned()
}

fn changed_native_document_title(current: &str, path: Option<&Path>) -> Option<String> {
    let next = native_document_title(path);
    (next != current).then_some(next)
}

fn has_extension(path: &std::path::Path, expected: &str) -> bool {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}

fn is_fdxt_source_path(path: &Path) -> bool {
    has_extension(path, "fdxt")
}

fn current_native_save_target<'a>(
    native_path: Option<&'a Path>,
    baseline: Option<&'a FdxBaseline>,
) -> Option<&'a Path> {
    let (Some(path), Some(baseline)) = (native_path, baseline) else {
        return None;
    };
    (baseline.path() == path).then_some(path)
}

impl ScriptCraftApp {
    /// Returns Saved only after the FDX storage transaction publishes; picker
    /// cancellation is Cancelled, and all rejected or failed paths are Failed.
    pub(crate) fn save_native(&mut self, ctx: &egui::Context) -> SaveOutcome {
        let Some(path) =
            current_native_save_target(self.native_path.as_deref(), self.fdx_baseline.as_ref())
                .map(Path::to_path_buf)
        else {
            // Untitled Save must return the actual picker/save result rather
            // than treating dispatch of Save As as successful publication. A
            // path without its matching baseline is not a writable Save target.
            return self.save_as_native(ctx);
        };
        self.save_native_at(&path)
    }

    fn save_native_at(&mut self, path: &std::path::Path) -> SaveOutcome {
        if has_extension(path, "json") {
            self.show_error(
                "LEGACY_JSON_SAVE_REJECTED: destination and current document unchanged",
            );
            return SaveOutcome::Failed;
        }

        if current_native_save_target(Some(path), self.fdx_baseline.as_ref()).is_none() {
            self.show_error(
                "FDX_SAVE_REQUIRES_BASELINE: use Save As; destination and document unchanged",
            );
            return SaveOutcome::Failed;
        }
        self.save_fdx_mode_at(path, FdxSaveMode::Save)
    }

    pub(crate) fn save_fdx_as_at(
        &mut self,
        path: &std::path::Path,
        overwrite: bool,
    ) -> SaveOutcome {
        if !has_extension(path, "fdx") {
            self.show_error(
                "SAVE_AS_EXTENSION_REJECTED: .fdx required; destination and document unchanged",
            );
            return SaveOutcome::Failed;
        }
        let mode = if overwrite {
            FdxSaveMode::SaveAsOverwrite(path.to_owned())
        } else {
            FdxSaveMode::SaveAs(path.to_owned())
        };
        self.save_fdx_mode_at(path, mode)
    }

    /// Save errors and preflight rejections map to Failed. A completed durable
    /// publication maps to Saved even when temporary cleanup only warns.
    fn save_fdx_mode_at(&mut self, path: &std::path::Path, mode: FdxSaveMode) -> SaveOutcome {
        if matches!(&mode, FdxSaveMode::Save) && is_fdxt_source_path(path) {
            self.show_error(
                "FDXT_SAVE_SOURCE_REJECTED: template source is not writable; destination and document unchanged",
            );
            return SaveOutcome::Failed;
        }
        if matches!(
            &mode,
            FdxSaveMode::SaveAs(_) | FdxSaveMode::SaveAsOverwrite(_)
        ) && !has_extension(path, "fdx")
        {
            self.show_error(
                "SAVE_AS_EXTENSION_REJECTED: .fdx required; destination and document unchanged",
            );
            return SaveOutcome::Failed;
        }
        if self.fdx_baseline.is_none()
            && self
                .native_path
                .as_deref()
                .is_some_and(|source_path| has_extension(source_path, "json"))
        {
            self.show_error(
                "FDX_SAVE_REFUSED_LEGACY_SOURCE: no FDX mapping; source, destination, and live document unchanged",
            );
            return SaveOutcome::Failed;
        }
        let mut target = self
            .fdx_baseline
            .clone()
            .map(FdxSaveTarget::from_baseline)
            .unwrap_or_else(FdxSaveTarget::new_unsaved);
        let before_save_project = self.session.project().clone();
        let active_before_save = self.active_body_dual_dialogue_group.clone();
        let text_state = self.root_text_state();
        match fdx_storage::save_session_with_view_state(
            &StdFdxStorageFileSystem,
            &mut self.session,
            &mut target,
            mode,
            self.latest_window_state
                .unwrap_or(fdx::WindowState::DEFAULT),
            text_state,
        ) {
            Ok(outcome) => {
                self.native_path = target.path().map(PathBuf::from);
                self.fdx_baseline = target.baseline().cloned();
                let rebased_active = active_before_save.and_then(|old_id| {
                    let new_id = scriptcraft_model::dual_dialogue::map_group_identity(
                        &before_save_project,
                        self.session.project(),
                        &old_id,
                    )?;
                    let focused_group = scriptcraft_model::dual_dialogue::group_at_element_index(
                        self.session.project(),
                        self.session.selection().focus.element,
                    )?;
                    (focused_group.id == new_id).then_some(new_id)
                });
                self.set_active_body_group(rebased_active);
                // Save preflight always prepares and publishes complete derived
                // data. The session's pending-work state, not a revision marker,
                // keeps the timer from repeating that work.
                self.next_cast_list_refresh = Instant::now() + CAST_LIST_REFRESH_INTERVAL;
                self.invalidate_after_save_rebase();
                if outcome.temporary_cleanup_warning {
                    self.show_error(
                        "FDX_SAVE_CLEANUP_WARNING: save completed, but temporary-file cleanup failed; the document is saved",
                    );
                } else {
                    self.clear_error();
                }
                SaveOutcome::Saved
            }
            Err(error) => {
                self.show_fdx_save_error(error);
                SaveOutcome::Failed
            }
        }
    }

    fn root_text_state(&self) -> fdx::TextState {
        // The immediate title viewport temporarily swaps child-local renderer
        // state into the root fields. Persist the body's zoom in either phase.
        let body_zoom = if self.title_view_state_swapped {
            self.title_view.zoom_percent
        } else {
            self.zoom_percent
        };
        fdx::TextState {
            scaling: body_zoom.clamp(
                scriptcraft_fdx::MIN_TEXT_SCALING,
                scriptcraft_fdx::MAX_TEXT_SCALING,
            ),
            selection: self.session.selection(),
        }
    }

    fn invalidate_after_save_rebase(&mut self) {
        self.layout_cache = LayoutCache::default();
        self.title_layout_cache = LayoutCache::default();
        self.page_geometry.invalidate();
        self.rendered_revision = None;
        self.preferred_vertical_x = None;
        self.page_scale = None;
        self.effective_pixels_per_point = None;
        self.title_view.page_geometry.invalidate();
        self.title_view.rendered_revision = None;
        self.title_view.pending_zoom_anchor = None;
        self.title_view.preferred_vertical_x = None;
        self.title_view.page_scale = None;
        self.title_view.effective_pixels_per_point = None;
        self.last_layout_identity = None;
    }

    fn show_fdx_save_error(&mut self, error: fdx_storage::TransactionalSaveError) {
        self.show_error(format!(
            "FDX_SAVE_FAILED: {}; destination and live document unchanged",
            error
        ));
    }

    pub(crate) fn save_as_native(&mut self, ctx: &egui::Context) -> SaveOutcome {
        let picker_result = rfd::FileDialog::new()
            .add_filter("Final Draft document", &["fdx"])
            .set_file_name("Untitled Screenplay.fdx")
            .save_file();
        save_picker_outcome(picker_result, |path| {
            self.dispatch_save_as_path(&path, true, ctx)
        })
    }

    /// Save As has a typed follow-up boundary so accepted picker destinations
    /// preserve the durable save result; path routing itself is not success.
    fn dispatch_save_as_path(
        &mut self,
        path: &std::path::Path,
        overwrite: bool,
        ctx: &egui::Context,
    ) -> SaveOutcome {
        let Some(path) = path.to_str().and_then(|path| DocumentPath::new(path).ok()) else {
            self.show_error("DOCUMENT_PATH_REJECTED: selected path is not valid UTF-8; active document unchanged");
            return SaveOutcome::Failed;
        };
        let result = self.dispatch_command(
            CommandInvocation::new(
                Command::FileSaveAs { path, overwrite },
                CommandOrigin::InternalFollowUp,
            ),
            ctx,
        );
        match result.outcome {
            commands::DesktopCommandOutcome::SaveCompleted(outcome) => outcome,
            // A non-save outcome is a dispatch failure, never evidence that the
            // destination was durably published.
            _ => SaveOutcome::Failed,
        }
    }

    pub(crate) fn sync_native_title(&mut self, ctx: &egui::Context) {
        if let Some(title) =
            changed_native_document_title(&self.native_title, self.native_path.as_deref())
        {
            commands::DesktopViewportEffects::title(ctx, title.clone());
            ctx.send_viewport_cmd_to(
                title_page::viewport_id(),
                egui::ViewportCommand::Title(title_page::native_title(&title)),
            );
            self.native_title = title;
        }
    }
}

#[cfg(test)]
mod native_save_target_tests {
    use super::*;

    fn valid<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
        result.unwrap_or_else(|error| panic!("prepared regression input failed: {error:?}"))
    }

    #[test]
    fn ordinary_save_requires_a_matching_path_and_verified_baseline() {
        assert!(current_native_save_target(None, None).is_none());

        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("dynamic_controls.fdx");
        let opened = valid(fdx_storage::open_source(&StdFdxStorageFileSystem, &fixture));
        let baseline = opened.baseline;

        assert!(current_native_save_target(Some(&fixture), None).is_none());
        assert_eq!(
            current_native_save_target(Some(&fixture), Some(&baseline)),
            Some(fixture.as_path())
        );
        let other_path = fixture.with_file_name("different.fdx");
        assert!(current_native_save_target(Some(&other_path), Some(&baseline)).is_none());
        assert_eq!(native_document_title(None), UNTITLED_DOCUMENT_TITLE);
    }

    #[test]
    fn template_source_extension_is_never_an_ordinary_save_target() {
        assert!(is_fdxt_source_path(Path::new("external.fdxt")));
        assert!(is_fdxt_source_path(Path::new("external.FDXT")));
        assert!(!is_fdxt_source_path(Path::new("ordinary.fdx")));
        assert!(!is_fdxt_source_path(Path::new("external.fdxt.backup")));
    }
}

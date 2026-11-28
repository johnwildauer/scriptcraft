//! Staged native document opening, migration, and initial-open coordination.

use crate::app::startup::{StartupDocumentInput, StartupDocumentSource};
use crate::app::ScriptCraftApp;
use crate::commands::CommandInvocation;
use crate::documents::lifecycle::show_native_lifecycle_error;
use crate::documents::replacement_protocol as replacement_open;
use crate::documents::save::fresh_document_title_for_template;
use crate::documents::unsaved_changes;
use crate::views::script::RevealIntent;
use eframe::egui::{self, Vec2};
use scriptcraft_commands::{Command, CommandOrigin, DocumentPath, EditorSurface, ImportResolution};
use scriptcraft_fdx::{self as fdx, FdxImportOutcome, FdxMigrationConsent, FdxVersionStatus};
use scriptcraft_layout::LayoutCache;
use scriptcraft_model::Project;
use scriptcraft_session::editing::{EditingSession, Position, Selection};
use scriptcraft_session::storage::{
    self as fdx_storage, FdxBaseline, OpenedFdxSource, StdFdxStorageFileSystem,
};
use std::path::{Path, PathBuf};
use std::time::Instant;

const FINAL_DRAFT_OPEN_EXTENSIONS: &[&str] = &["fdx", "fdxt"];

#[cfg(target_os = "macos")]
use crate::platform::macos::macos_document_open;

/// The terminal or deferred result of opening a staged native document.
/// `AwaitingMigration` is not an activation: the pending source remains owned
/// by this child until the user continues or cancels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StagedOpenOutcome {
    Activated,
    AwaitingMigration,
    Cancelled,
    Failed,
}

fn stage_replacement_open(staged: &mut Option<PathBuf>, path: PathBuf) -> bool {
    if staged.is_some() {
        return false;
    }
    *staged = Some(path);
    true
}

/// The source and bounded declared version retained while migration consent is pending.
pub(crate) struct PendingFdxOpen {
    path: PathBuf,
    source: OpenedFdxSource,
    disposition: StagedOpenDisposition,
    pub(crate) declared_version: String,
}

/// The staged source's native-document lifecycle, fixed at the shared path
/// boundary and carried unchanged through any migration decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StagedOpenDisposition {
    OrdinaryFdx,
    FreshUnsaved,
}

fn staged_open_disposition(path: &Path) -> StagedOpenDisposition {
    if path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case("fdxt"))
    {
        StagedOpenDisposition::FreshUnsaved
    } else {
        StagedOpenDisposition::OrdinaryFdx
    }
}

fn fresh_title_from_template_path(path: &Path) -> String {
    let display_name = path
        .file_stem()
        .and_then(std::ffi::OsStr::to_str)
        .filter(|stem| !stem.is_empty())
        .unwrap_or("");
    fresh_document_title_for_template(display_name)
}

fn pending_open_disposition(
    disposition: StagedOpenDisposition,
    consent: FdxMigrationConsent,
) -> Option<StagedOpenDisposition> {
    (consent == FdxMigrationConsent::Continue).then_some(disposition)
}

/// Persistence assigned when a prepared project becomes the active document.
/// Fresh template-backed projects are never associated with their source path.
pub(crate) enum ActivationPersistence {
    Saved {
        path: PathBuf,
        baseline: FdxBaseline,
    },
    FreshUnsaved,
}

/// Finalizes the initial-document slot once, taking a cold path only when no
/// startup document already owns it. The closure is not called after finalization.
fn startup_document_with_cold_request(
    startup_document: Option<StartupDocumentInput>,
    cold_path: Option<PathBuf>,
) -> Option<StartupDocumentInput> {
    startup_document.or_else(|| {
        cold_path.map(|path| StartupDocumentInput {
            path,
            source: StartupDocumentSource::NativeOpenDocuments,
            handoff_id: None,
        })
    })
}

fn finalize_startup_document_slot(
    startup_document: &mut Option<StartupDocumentInput>,
    finalized: &mut bool,
    take_cold_path: impl FnOnce(bool) -> Option<PathBuf>,
) {
    if *finalized {
        return;
    }

    let startup_document_exists = startup_document.is_some();
    let cold_path = take_cold_path(startup_document_exists);
    *startup_document = startup_document_with_cold_request(startup_document.take(), cold_path);
    *finalized = true;
}

fn take_startup_document_once(
    startup_document: &Option<StartupDocumentInput>,
    consumed: &mut bool,
) -> Option<StartupDocumentInput> {
    if *consumed {
        return None;
    }
    *consumed = true;
    startup_document.clone()
}

fn consume_initial_startup_document(
    startup_document: &Option<StartupDocumentInput>,
    consumed: &mut bool,
    open_staged: impl FnOnce(&Path) -> StagedOpenOutcome,
) -> Option<StagedOpenOutcome> {
    let startup_document = take_startup_document_once(startup_document, consumed)?;
    Some(open_staged(&startup_document.path))
}

fn write_startup_handoff_result(
    startup_document: &mut Option<StartupDocumentInput>,
    outcome: StagedOpenOutcome,
    writer: &mut impl std::io::Write,
) {
    if outcome == StagedOpenOutcome::AwaitingMigration {
        return;
    }
    let Some(startup_document) = startup_document.as_mut() else {
        return;
    };
    // Only application-generated custom-option requests can be replacement
    // children. Positional shell opens and macOS native cold opens never speak
    // the replacement protocol, even if an identity were accidentally attached.
    if startup_document.source != StartupDocumentSource::CustomOption {
        return;
    }
    let _ = replacement_open::write_terminal_handoff_result(
        &mut startup_document.handoff_id,
        outcome,
        writer,
    );
}

fn child_startup_open_failure_copy() -> (&'static str, &'static str) {
    (
        "Unable to Open Document",
        "The selected document could not be opened in this new instance. This instance remains untitled.",
    )
}

fn startup_open_failure_copy(outcome: StagedOpenOutcome) -> Option<(&'static str, &'static str)> {
    (outcome == StagedOpenOutcome::Failed).then_some(child_startup_open_failure_copy())
}

pub(crate) fn show_startup_open_failure(
    outcome: StagedOpenOutcome,
    acknowledged_replacement_child: bool,
) {
    if !acknowledged_replacement_child {
        if let Some((title, message)) = startup_open_failure_copy(outcome) {
            show_native_lifecycle_error(title, message);
        }
    }
}

impl ScriptCraftApp {
    /// True only while this process still owns the identity for an acknowledged
    /// in-app replacement request. Independent custom-option children remain
    /// child-owned because they have no handoff identity.
    pub(crate) fn acknowledged_replacement_startup(&self) -> bool {
        self.startup_document.as_ref().is_some_and(|startup| {
            startup.source == StartupDocumentSource::CustomOption && startup.handoff_id.is_some()
        })
    }
}

#[cfg(target_os = "macos")]
fn macos_document_open_error_copy(
    error: macos_document_open::MacOsDocumentOpenError,
) -> (&'static str, &'static str) {
    let message = match error {
        macos_document_open::MacOsDocumentOpenError::UnsupportedDocumentCount => {
            "Multiple documents in one request are not supported; no documents from that request were opened. The current document is unchanged."
        }
        macos_document_open::MacOsDocumentOpenError::InboxOverflow => {
            "A document request could not be accepted. The current document is unchanged."
        }
        macos_document_open::MacOsDocumentOpenError::EmptyOrMalformedEvent
        | macos_document_open::MacOsDocumentOpenError::UnsupportedUrlOrDescriptor
        | macos_document_open::MacOsDocumentOpenError::InvalidUtf8Path => {
            "The document request could not be opened. The current document is unchanged."
        }
    };
    ("Unable to Open Document", message)
}

fn pending_resolution_outcome(
    pending: Option<()>,
    consent: FdxMigrationConsent,
    continue_outcome: StagedOpenOutcome,
) -> Option<StagedOpenOutcome> {
    if pending.is_none() {
        None
    } else if consent == FdxMigrationConsent::Cancel {
        Some(StagedOpenOutcome::Cancelled)
    } else {
        Some(continue_outcome)
    }
}

fn staged_open_activation_outcome(activation: Result<(), StagedOpenOutcome>) -> StagedOpenOutcome {
    match activation {
        Ok(()) => StagedOpenOutcome::Activated,
        Err(outcome) => outcome,
    }
}

fn session_activation_failure_code(
    error: &scriptcraft_session::editing::EditError,
) -> &'static str {
    use scriptcraft_session::editing::EditError;
    match error {
        EditError::InvalidElement => "EDIT_INVALID_ELEMENT",
        EditError::InvalidUtf8Boundary => "EDIT_INVALID_TEXT_BOUNDARY",
        EditError::InvalidSelection => "EDIT_INVALID_SELECTION",
        EditError::EmptyHistory => "EDIT_EMPTY_HISTORY",
        EditError::TextTooLong => "EDIT_TEXT_LIMIT",
        EditError::TooManyElements => "EDIT_ELEMENT_LIMIT",
        EditError::InlineAltsRoleChange => "EDIT_INLINE_ALTS_ROLE_CHANGE",
        EditError::Preservation(_) => "EDIT_PRESERVATION_CONSTRAINT",
        EditError::InvalidDefinitionCandidate => "EDIT_INVALID_DEFINITION_CANDIDATE",
        EditError::CastList(_) => "EDIT_CAST_LIST_CONSTRAINT",
        EditError::FormattedText(_) => "EDIT_FORMATTING_CONSTRAINT",
        EditError::Model(_) => "EDIT_MODEL_VALIDATION",
    }
}

fn open_read_failure_message(error: &fdx_storage::FdxStorageError) -> String {
    format!("FDX_OPEN_FAILED: {error}; active document unchanged")
}

fn open_import_failure_message(error: &fdx::FdxError) -> String {
    format!("FDX_IMPORT_FAILED: {error}; active document unchanged")
}

fn session_activation_failure_message(error: &scriptcraft_session::editing::EditError) -> String {
    format!(
        "FDX_SESSION_ACTIVATION_FAILED: {}; active document unchanged",
        session_activation_failure_code(error)
    )
}

impl ScriptCraftApp {
    /// Direct-path Open first stages one selected path. Root-frame processing
    /// then applies selected path → dirty gate → spawn/pending → activation
    /// Opened → the WO-001 arm-plus-root-Close continuation.
    pub(crate) fn stage_replacement_document_path(&mut self, path: &Path) {
        if !stage_replacement_open(&mut self.staged_replacement_open, path.to_owned()) {
            self.show_error(
                "OPEN_REPLACEMENT_ALREADY_STAGED: another selected document is already awaiting its lifecycle gate",
            );
        }
    }

    pub(crate) fn finalize_initial_document_slot(&mut self) {
        if self.initial_document_slot_finalized {
            return;
        }

        #[cfg(target_os = "macos")]
        {
            let controller = self.native_open_documents.as_ref();
            finalize_startup_document_slot(
                &mut self.startup_document,
                &mut self.initial_document_slot_finalized,
                |startup_document_exists| {
                    controller.and_then(|controller| {
                        controller.take_cold_startup_path(startup_document_exists)
                    })
                },
            );
        }
        #[cfg(not(target_os = "macos"))]
        finalize_startup_document_slot(
            &mut self.startup_document,
            &mut self.initial_document_slot_finalized,
            |_| None,
        );
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn drain_native_open_documents(&self) {
        let Some(controller) = &self.native_open_documents else {
            return;
        };
        // Finder/Dock events bypass typed in-app Open and its acknowledged
        // replacement lock: no dirty/save prompt, handoff identity, stdout
        // acknowledgement, receiver mutation, close request, or child termination.
        macos_document_open::route_independent_requests(
            controller.drain(),
            |path| self.launch_child_instance(Some(path)).map_err(|_| ()),
            |feedback| match feedback {
                macos_document_open::MacOsDocumentOpenFeedback::IntakeError(error) => {
                    let (title, message) = macos_document_open_error_copy(error);
                    show_native_lifecycle_error(title, message);
                }
                macos_document_open::MacOsDocumentOpenFeedback::IndependentLaunchFailed => {
                    show_native_lifecycle_error(
                        "Unable to Open Document",
                        "A separate instance for the selected document could not be started. The current document is unchanged.",
                    );
                }
                macos_document_open::MacOsDocumentOpenFeedback::InboxOverflow => {
                    let (title, message) = macos_document_open_error_copy(
                        macos_document_open::MacOsDocumentOpenError::InboxOverflow,
                    );
                    show_native_lifecycle_error(title, message);
                }
            },
        );
    }

    pub(crate) fn consume_startup_document_handoff(&mut self) -> Option<StagedOpenOutcome> {
        // Positional shell input already owns this initial-instance slot. It
        // goes straight to staged activation: no child, picker, or replacement.
        let startup_document = take_startup_document_once(
            &self.startup_document,
            &mut self.startup_document_consumed,
        )?;
        Some(self.open_staged_native_at(&startup_document.path))
    }

    /// Emit only a terminal startup result. Taking the identity in the shared
    /// writer consumes it before any fallible stdout operation; migration keeps
    /// the identity attached to the startup document until resolution.
    pub(crate) fn emit_startup_handoff_result(&mut self, outcome: StagedOpenOutcome) {
        let stdout = std::io::stdout();
        let mut writer = stdout.lock();
        write_startup_handoff_result(&mut self.startup_document, outcome, &mut writer);
    }

    pub(crate) fn open_native(&mut self, ctx: &egui::Context) {
        #[cfg(target_os = "macos")]
        {
            use crate::platform::macos::macos_native_dialogs::{
                NativeFilePanelContinuation, NativeFilePanelRequest,
            };
            let result = self.native_operation_coordinator.present_file_panel(
                NativeFilePanelRequest {
                    continuation: NativeFilePanelContinuation::Open,
                    title: "Open Final Draft Document",
                    default_filename: "",
                    allowed_extensions: FINAL_DRAFT_OPEN_EXTENSIONS,
                    include_title_page: false,
                },
            );
            if result.is_err() {
                self.show_error("OPEN_PICKER_UNAVAILABLE: document picker could not be presented; active document unchanged");
            }
            let _ = ctx;
            return;
        }
        #[cfg(not(target_os = "macos"))]
        {
        let dialog = rfd::FileDialog::new().add_filter(
            "Final Draft documents and templates",
            FINAL_DRAFT_OPEN_EXTENSIONS,
        );
        dispatch_open_selection(dialog.pick_file(), |path| {
            self.dispatch_document_path_command(path, ctx);
        });
        }
    }

    pub(crate) fn dispatch_document_path_command(&mut self, path: &Path, ctx: &egui::Context) {
        let Some(path) = path.to_str().and_then(|path| DocumentPath::new(path).ok()) else {
            self.show_error("DOCUMENT_PATH_REJECTED: selected path is not valid UTF-8; active document unchanged");
            return;
        };
        self.dispatch_command(
            CommandInvocation::new(Command::FileOpen { path }, CommandOrigin::InternalFollowUp),
            ctx,
        );
    }

    pub(crate) fn resolve_pending_fdx_import(
        &mut self,
        consent: FdxMigrationConsent,
    ) -> Option<StagedOpenOutcome> {
        let Some(pending) = self.pending_fdx_open.take() else {
            return pending_resolution_outcome(None, consent, StagedOpenOutcome::Failed);
        };
        let continue_outcome = match pending_open_disposition(pending.disposition, consent) {
            None => {
                self.clear_error();
                StagedOpenOutcome::Failed
            }
            Some(_) if self.session.dirty() => {
                self.show_error(
                    "OPEN_CANCELLED_DIRTY: unsaved project and active document unchanged",
                );
                StagedOpenOutcome::Failed
            }
            Some(disposition) => {
                self.finish_fdx_import(pending.path, pending.source, consent, disposition)
            }
        };
        let outcome = pending_resolution_outcome(Some(()), consent, continue_outcome);
        if let Some(outcome) = outcome {
            let acknowledged_replacement_child = self.acknowledged_replacement_startup();
            self.emit_startup_handoff_result(outcome);
            show_startup_open_failure(outcome, acknowledged_replacement_child);
        }
        outcome
    }

    fn open_staged_native_at(&mut self, path: &Path) -> StagedOpenOutcome {
        let disposition = staged_open_disposition(path);
        if self.pending_fdx_open.is_some() {
            self.show_error(
                "OPEN_BLOCKED_BY_MIGRATION: resolve pending FDX import; active document unchanged",
            );
            return StagedOpenOutcome::Failed;
        }
        if self.session.dirty() {
            self.show_error("OPEN_CANCELLED_DIRTY: unsaved project and active document unchanged");
            return StagedOpenOutcome::Failed;
        }
        let source = match fdx_storage::open_source(&StdFdxStorageFileSystem, path) {
            Ok(source) => source,
            Err(error) => {
                self.reconcile_body_active_group();
                self.show_error(open_read_failure_message(&error));
                return StagedOpenOutcome::Failed;
            }
        };
        // Selected bytes go through the bounded native FDX preflight/import path;
        // this boundary does not sniff or dispatch alternate document formats.
        match fdx::preflight_version(source.bytes()) {
            Ok(FdxVersionStatus::Version6) => self.finish_fdx_import(
                path.to_owned(),
                source,
                FdxMigrationConsent::Continue,
                disposition,
            ),
            Ok(FdxVersionStatus::ExplicitNon6 { declared, .. }) => {
                self.pending_fdx_open = Some(PendingFdxOpen {
                    path: path.to_owned(),
                    source,
                    disposition,
                    declared_version: declared,
                });
                self.surface_focused = false;
                self.reconcile_body_active_group();
                self.clear_error();
                StagedOpenOutcome::AwaitingMigration
            }
            Ok(FdxVersionStatus::Missing) => {
                self.show_error("FDX_VERSION_MISSING: active document unchanged");
                StagedOpenOutcome::Failed
            }
            Ok(FdxVersionStatus::Malformed) => {
                self.show_error("FDX_VERSION_INVALID: active document unchanged");
                StagedOpenOutcome::Failed
            }
            Ok(FdxVersionStatus::Duplicate) => {
                self.show_error("FDX_VERSION_DUPLICATE: active document unchanged");
                StagedOpenOutcome::Failed
            }
            Err(_) => {
                self.show_error("FDX_VERSION_CHECK_FAILED: selected document could not be checked; active document unchanged");
                StagedOpenOutcome::Failed
            }
        }
    }

    fn finish_fdx_import(
        &mut self,
        path: PathBuf,
        source: OpenedFdxSource,
        consent: FdxMigrationConsent,
        disposition: StagedOpenDisposition,
    ) -> StagedOpenOutcome {
        if consent == FdxMigrationConsent::Cancel {
            self.clear_error();
            return StagedOpenOutcome::Cancelled;
        }
        let outcome = fdx::import_fdx(source.bytes(), consent);
        match outcome {
            Ok(FdxImportOutcome::Candidate(candidate)) => {
                let migration_status = match candidate.migration {
                    None => None,
                    Some(fdx::FdxMigrationKind::OlderLossy { .. }) => {
                        Some("Opened previous-version FDX; some data may be lost")
                    }
                    Some(fdx::FdxMigrationKind::NewerBestEffort { .. }) => {
                        Some("Opened non-current FDX with best-effort Version 6 import")
                    }
                };
                let fresh_document_title = match disposition {
                    StagedOpenDisposition::OrdinaryFdx => None,
                    StagedOpenDisposition::FreshUnsaved => {
                        Some(fresh_title_from_template_path(&path))
                    }
                };
                let persistence = match disposition {
                    StagedOpenDisposition::OrdinaryFdx => ActivationPersistence::Saved {
                        path,
                        baseline: source.baseline,
                    },
                    // The read baseline belongs only to staging. Dropping it
                    // here guarantees that an FDXT path cannot become a save target.
                    StagedOpenDisposition::FreshUnsaved => ActivationPersistence::FreshUnsaved,
                };
                let document = candidate.document;
                let display_mode = document.display_mode;
                staged_open_activation_outcome(self.activate_opened_project(
                    document.project,
                    persistence,
                    fresh_document_title,
                    migration_status,
                    document.window_state,
                    document.text_state,
                    document.invisibles,
                    display_mode,
                    document.beat_board_view_state,
                    document.story_map_view_state,
                ))
            }
            Ok(FdxImportOutcome::Cancelled { .. }) => {
                self.clear_error();
                StagedOpenOutcome::Cancelled
            }
            Err(error) => {
                self.reconcile_body_active_group();
                self.show_error(open_import_failure_message(&error));
                StagedOpenOutcome::Failed
            }
        }
    }

    pub(crate) fn activate_opened_project(
        &mut self,
        project: Project,
        persistence: ActivationPersistence,
        fresh_document_title: Option<String>,
        migration_status: Option<&str>,
        window_state: Option<fdx::WindowState>,
        text_state: fdx::TextState,
        invisibles: fdx::InvisiblesState,
        display_mode: fdx::ScriptPanelDisplayMode,
        beat_board_view_state: fdx::BeatBoardViewState,
        story_map_view_state: fdx::StoryMapViewState,
    ) -> Result<(), StagedOpenOutcome> {
        let (native_path, fdx_baseline) = match persistence {
            ActivationPersistence::Saved { path, baseline } => {
                if baseline.path() != path.as_path() {
                    self.reconcile_body_active_group();
                    self.show_error(
                        "FDX_SESSION_ACTIVATION_TARGET_MISMATCH: active document unchanged",
                    );
                    return Err(StagedOpenOutcome::Failed);
                }
                (Some(path), Some(baseline))
            }
            ActivationPersistence::FreshUnsaved => (None, None),
        };
        let mut replacement = match EditingSession::with_policy(project, self.session.policy()) {
            Ok(replacement) => replacement,
            Err(error) => {
                self.reconcile_body_active_group();
                self.show_error(session_activation_failure_message(&error));
                return Err(StagedOpenOutcome::Failed);
            }
        };
        let restored_selection = scriptcraft_commands::Selection::from_core(text_state.selection)
            .ok()
            .map(|selection| Command::EditorSelectionSet {
                selection,
                surface: EditorSurface::Body,
            })
            .is_some_and(|command| {
                matches!(
                    scriptcraft_commands::execute_editor_command(&mut replacement, &command),
                    scriptcraft_commands::CommandOutcome::Changed
                        | scriptcraft_commands::CommandOutcome::NoOp
                )
            });
        if !restored_selection {
            let fallback_restored = scriptcraft_commands::Selection::from_core(Selection::caret(
                Position {
                    element: 0,
                    offset: 0,
                },
            ))
            .ok()
            .map(|selection| Command::EditorSelectionSet {
                selection,
                surface: EditorSurface::Body,
            })
            .is_some_and(|command| {
                matches!(
                    scriptcraft_commands::execute_editor_command(&mut replacement, &command),
                    scriptcraft_commands::CommandOutcome::Changed
                        | scriptcraft_commands::CommandOutcome::NoOp
                )
            });
            if !fallback_restored {
                self.show_error("FDX_SESSION_SELECTION_RESTORE_FAILED: active document unchanged");
                return Err(StagedOpenOutcome::Failed);
            }
        }
        let script_view_mode = Self::script_view_mode_from_fdx(display_mode);
        self.active_body_dual_dialogue_group = None;
        self.surface_focused = false;
        self.session = replacement;
        self.reset_body_auto_scroll();
        self.reset_title_auto_scroll();
        self.beat_auto_scroll.reset();
        self.script_note_workspace.reset_document_activation();
        self.script_view_mode = script_view_mode;
        self.invisibles = invisibles;
        self.beat_board_view_state = beat_board_view_state;
        self.story_map_view_state = story_map_view_state;
        self.story_map_pinch_zoom_percent =
            f64::from(story_map_view_state.zoom_milli_percent()) / 1_000.0;
        self.story_map_resize.reset();
        self.story_map_navigation.reset();
        self.story_map_beat_interaction.reset();
        self.beat_drag_coordinator.reset();
        self.story_map_projection.reset_document();
        self.story_map_preview.close();
        self.story_map_target_dialog.reset();
        self.story_map_context_menu.close();
        self.beat_pinch_zoom_percent = f64::from(beat_board_view_state.zoom_milli_percent()) / 1000.0;
        self.beat_viewport = None;
        self.beat_interaction.cancel();
        self.beat_text.blur();
        self.appearance_applications
            .clear(crate::chrome::theme::AppearanceContext::Elements);
        self.appearance_applications
            .clear(crate::chrome::theme::AppearanceContext::TemplateSelector);
        self.element_windows.reset();
        self.element_windows.schedule_native_close();
        self.command_projection_cache = None;
        self.invalidate_dynamic_controls();
        self.zoom_percent = text_state.scaling.clamp(
            scriptcraft_fdx::MIN_TEXT_SCALING,
            scriptcraft_fdx::MAX_TEXT_SCALING,
        );
        self.sync_pinch_zoom_from_controls();
        self.pending_zoom_anchor = None;
        // Publish a native path and conflict baseline only as one persistence
        // disposition. In particular, a template activation clears both.
        self.native_path = native_path;
        self.fdx_baseline = fdx_baseline;
        if let Some(fresh_document_title) = fresh_document_title {
            self.fresh_document_title = fresh_document_title;
        }
        self.pending_window_restore = window_state;
        self.pending_fdx_open = None;
        self.synchronize_cast_list_scheduler(Instant::now());
        self.page_geometry.reset_for_document_replacement();
        self.title_view
            .page_geometry
            .reset_for_document_replacement();
        self.responsiveness.clear_pending_edit();
        self.layout_cache = LayoutCache::default();
        self.title_layout_cache = LayoutCache::default();
        self.last_layout_identity = None;
        self.page_scale = None;
        self.effective_pixels_per_point = None;
        self.rendered_revision = None;
        self.title_view.rendered_revision = None;
        self.title_view.pending_zoom_anchor = None;
        self.title_view.page_scale = None;
        self.title_view.effective_pixels_per_point = None;
        self.title_view.preferred_vertical_x = None;
        self.title_view.reveal_intent = RevealIntent::None;
        self.title_view.drag_anchor = None;
        self.title_view.ime_preedit.clear();
        self.title_view.ime_active = false;
        self.preferred_vertical_x = None;
        self.drag_anchor = None;
        self.ime_preedit.clear();
        self.ime_active = false;
        if let Some(warning) = migration_status {
            self.show_error(warning);
        } else {
            self.clear_error();
        }
        // The replacement session is installed before this sole Activated result.
        Ok(())
    }
}

fn dispatch_open_selection(path: Option<PathBuf>, dispatch: impl FnOnce(&Path)) {
    if let Some(path) = path {
        dispatch(&path);
    }
}

pub(crate) fn show_fdx_migration_dialog(app: &mut ScriptCraftApp, ctx: &egui::Context) {
    let Some(declared_version) = app
        .pending_fdx_open
        .as_ref()
        .map(|pending| pending.declared_version.clone())
    else {
        return;
    };
    let mut choice = None;
    egui::Window::new("Update FDX document")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label(format!(
                "This document declares FDX version {declared_version}. It will be opened with the current Version 6 importer. Updating may contain errors, some data may be lost, and compatibility is not guaranteed."
            ));
            ui.horizontal(|ui| {
                if ui.button("Continue").clicked() {
                    choice = Some(FdxMigrationConsent::Continue);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(FdxMigrationConsent::Cancel);
                }
            });
        });
    if let Some(consent) = choice {
        let resolution = match consent {
            FdxMigrationConsent::Continue => ImportResolution::Continue,
            FdxMigrationConsent::Cancel => ImportResolution::Cancel,
        };
        app.dispatch_command(
            CommandInvocation::new(
                Command::FileImportResolve { resolution },
                CommandOrigin::InternalFollowUp,
            ),
            ctx,
        );
    }
}

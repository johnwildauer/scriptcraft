use crate::app::ScriptCraftApp;
use scriptcraft_commands::{CommandId, EditorSurface};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AppCommandAvailability {
    pub(crate) modal_blocked: bool,
    pub(crate) native_output_active: bool,
    /// Replacement handoff lock; deliberately independent from the migration
    /// modal that alone enables File Import Resolve.
    pub(crate) replacement_pending: bool,
    pub(crate) title_page_only_close: bool,
    pub(crate) undo: bool,
    pub(crate) redo: bool,
    pub(crate) delete: bool,
    pub(crate) select_all: bool,
    pub(crate) cut: bool,
    pub(crate) copy: bool,
    pub(crate) paste: bool,
    pub(crate) body_only: bool,
    pub(crate) dual_dialogue_toggle: bool,
    pub(crate) title_open: bool,
    pub(crate) title_close: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AppCommandProjection {
    pub(crate) availability: AppCommandAvailability,
    pub(crate) say_undual: bool,
}

/// Every non-session input to the shared availability/label projection. Session
/// content, selections, typing styles, and histories are covered by the session
/// mutation token; platform-specific native-output state is reduced to its
/// effective boolean before entering this key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AppCommandProjectionKey {
    session_mutation_token: u64,
    active_editor_surface: EditorSurface,
    migration_pending: bool,
    replacement_pending: bool,
    native_output_active: bool,
    title_page_window_open: bool,
    title_viewport_unsupported: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CachedAppCommandProjection {
    key: AppCommandProjectionKey,
    projection: AppCommandProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DesktopUnavailableReason {
    ModalBlocked,
    UndoHistory,
    RedoHistory,
    DeleteTarget,
    Selection,
}

impl ScriptCraftApp {
    fn native_output_active(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            self.native_output_modal_active
                || self.windows_print_preparation.is_some()
                || self.windows_print_session.is_some()
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.native_output_modal_active
        }
    }

    fn app_command_projection_key(&self) -> AppCommandProjectionKey {
        AppCommandProjectionKey {
            session_mutation_token: self.session.mutation_token(),
            active_editor_surface: self.active_editor_surface,
            migration_pending: self.pending_fdx_open.is_some(),
            replacement_pending: self.replacement_open_pending(),
            native_output_active: self.native_output_active(),
            title_page_window_open: self.title_page_window_open,
            title_viewport_unsupported: self.title_viewport_unsupported,
        }
    }

    pub(crate) fn app_command_projection(&mut self) -> AppCommandProjection {
        let key = self.app_command_projection_key();
        if let Some(cached) = self.command_projection_cache {
            if cached.key == key {
                return cached.projection;
            }
        }

        let dual_dialogue_target = self.dual_dialogue_toggle_target();
        let mut availability = self.app_command_availability_without_dual_dialogue();
        availability.dual_dialogue_toggle =
            !availability.replacement_pending && dual_dialogue_target.is_some();
        let projection = AppCommandProjection {
            availability,
            say_undual: matches!(
                dual_dialogue_target,
                Some(scriptcraft_model::dual_dialogue::ToggleTarget::Disable(_))
            ),
        };
        self.command_projection_cache = Some(CachedAppCommandProjection { key, projection });
        projection
    }

    pub(crate) fn app_command_availability(&mut self) -> AppCommandAvailability {
        self.app_command_projection().availability
    }

    pub(crate) fn dual_dialogue_menu_should_say_undual(&mut self) -> bool {
        self.app_command_projection().say_undual
    }

    /// Dispatch always observes current gates. Most commands do not consume the
    /// dual-dialogue target, so their fresh availability check need not perform
    /// that target resolution and candidate scan.
    pub(super) fn command_dispatch_availability(
        &mut self,
        command: &scriptcraft_commands::Command,
    ) -> AppCommandAvailability {
        let command_id = command.id();
        let mut availability = self.app_command_availability_without_dual_dialogue();
        if command_id == CommandId::EditorDualDialogueToggle {
            availability.dual_dialogue_toggle =
                !availability.replacement_pending && self.dual_dialogue_toggle_target().is_some();
        }
        if let scriptcraft_commands::Command::EditorElementChange { surface, .. } = command {
            let core_selection = match surface {
                EditorSurface::Body => self.session.selection(),
                EditorSurface::TitlePage => self.session.title_selection(),
            };
            availability.body_only = scriptcraft_commands::Selection::from_core(core_selection)
                .ok()
                .is_some_and(|selection| {
                    selection
                        .validate_for_surface(self.session.project(), *surface)
                        .is_ok()
                        && selection.anchor.element == selection.focus.element
                });
        }
        availability
    }

    fn app_command_availability_without_dual_dialogue(&self) -> AppCommandAvailability {
        let migration_pending = self.pending_fdx_open.is_some();
        let replacement_pending = self.replacement_open_pending();
        let native_output_active = self.native_output_active();
        let (selection, text_len, role_is_empty_parenthetical, element_count) =
            match self.active_editor_surface {
                EditorSurface::Body => {
                    let selection = self.session.selection();
                    let element = self.session.project().elements.get(selection.focus.element);
                    (
                        selection,
                        element.map(|e| e.text.len()).unwrap_or(0),
                        element.is_some_and(|e| {
                            e.element_type().as_str() == "Parenthetical"
                                && matches!(e.text.as_str(), "(" | "()" | ")")
                        }),
                        self.session.project().elements.len(),
                    )
                }
                EditorSurface::TitlePage => {
                    let selection = self.session.title_selection();
                    let text_len = self
                        .session
                        .project()
                        .title_page
                        .get(selection.focus.element)
                        .map(|p| p.text.len())
                        .unwrap_or(0);
                    (
                        selection,
                        text_len,
                        false,
                        self.session.project().title_page.len(),
                    )
                }
            };
        let delete_target_exists = if selection.anchor != selection.focus {
            true
        } else {
            selection.focus.offset < text_len || role_is_empty_parenthetical
        };

        AppCommandAvailability {
            modal_blocked: migration_pending && !replacement_pending,
            native_output_active,
            replacement_pending,
            title_page_only_close: self.active_editor_surface() == EditorSurface::TitlePage
                && self.title_page_window_open,
            undo: !migration_pending && !replacement_pending && self.session.can_undo(),
            redo: !migration_pending && !replacement_pending && self.session.can_redo(),
            delete: !migration_pending && !replacement_pending && delete_target_exists,
            // EditingSession projects a document with at least one body element.
            select_all: !migration_pending && !replacement_pending && element_count != 0,
            cut: !migration_pending && !replacement_pending && selection.anchor != selection.focus,
            copy: !migration_pending && !replacement_pending && selection.anchor != selection.focus,
            // Clipboard state is read only when Paste is activated, never per frame.
            paste: !migration_pending && !replacement_pending,
            body_only: self.active_editor_surface == EditorSurface::Body,
            dual_dialogue_toggle: false,
            // In-app Open is a selected-path replacement request and is blocked
            // while the current document is resolving another lifecycle gate.
            title_open: !migration_pending
                && !replacement_pending
                && !self.title_viewport_unsupported,
            // Closing the presentation-only child viewport does not destroy
            // the root document and remains available during replacement.
            title_close: !migration_pending && self.title_page_window_open,
        }
    }

    fn dual_dialogue_toggle_target(
        &mut self,
    ) -> Option<scriptcraft_model::dual_dialogue::ToggleTarget> {
        if self.active_editor_surface != EditorSurface::Body {
            return None;
        }
        self.session
            .dual_dialogue_toggle_target(scriptcraft_model::dual_dialogue::SelectionSurface::Body)
            .ok()
            .flatten()
    }
}

impl AppCommandAvailability {
    /// One command-state projection consumed by both menu presentations and
    /// checked again at dispatch time to reject stale or modal-blocked input.
    pub(crate) const fn is_enabled(self, command: CommandId) -> bool {
        if matches!(command, CommandId::UiStatusSet) {
            return true;
        }
        if self.native_output_active {
            return false;
        }
        if self.replacement_pending {
            return match command {
                // Presentation-only controls remain usable. File Close is
                // allowed only when it resolves to the title-page child.
                CommandId::WindowTitlePageClose => self.title_close,
                CommandId::FileClose => self.title_page_only_close,
                CommandId::ToolsSprintTimerTogglePopup
                | CommandId::ToolsSprintTimerClosePopup
                | CommandId::ToolsSprintTimerSetMode
                | CommandId::ToolsSprintTimerSetMinutes
                | CommandId::ToolsSprintTimerStart
                | CommandId::ToolsSprintTimerPause
                | CommandId::ToolsSprintTimerResume
                | CommandId::ToolsSprintTimerReset
                | CommandId::ToolsSprintTimerEnd => true,
                _ => false,
            };
        }
        if matches!(command, CommandId::FileImportResolve) {
            return self.modal_blocked;
        }
        if self.modal_blocked {
            return false;
        }
        match command {
            CommandId::EditUndo => self.undo,
            CommandId::EditRedo => self.redo,
            CommandId::EditDelete => self.delete,
            CommandId::EditSelectAll => self.select_all,
            CommandId::EditCut => self.cut,
            CommandId::EditCopy => self.copy,
            CommandId::EditPaste => self.paste,
            CommandId::EditorElementChange => self.body_only,
            CommandId::EditorDualDialogueToggle => self.dual_dialogue_toggle,
            CommandId::WindowTitlePageOpen => self.title_open,
            CommandId::WindowTitlePageClose => self.title_close,
            // The global modal gates above still disable Quit during migration
            // and native-output dialogs; otherwise it is focus-independent.
            CommandId::ApplicationQuit => true,
            CommandId::FilePrint => cfg!(any(target_os = "macos", target_os = "windows")),
            _ => true,
        }
    }

    pub(super) const fn unavailable_reason(self, command: CommandId) -> DesktopUnavailableReason {
        if self.modal_blocked || self.native_output_active || self.replacement_pending {
            return DesktopUnavailableReason::ModalBlocked;
        }
        match command {
            CommandId::EditUndo => DesktopUnavailableReason::UndoHistory,
            CommandId::EditRedo => DesktopUnavailableReason::RedoHistory,
            CommandId::EditDelete => DesktopUnavailableReason::DeleteTarget,
            CommandId::EditSelectAll | CommandId::EditCut | CommandId::EditCopy => {
                DesktopUnavailableReason::Selection
            }
            CommandId::EditorDualDialogueToggle => DesktopUnavailableReason::Selection,
            _ => DesktopUnavailableReason::ModalBlocked,
        }
    }
}

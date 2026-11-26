use crate::app::{RootDocumentView, ScriptCraftApp};
use crate::widgets::elements::state::TemplateSelectorMode;
use scriptcraft_commands::{BuiltinTemplateValue, CommandId, EditorSurface};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AppCommandAvailability {
    pub(crate) modal_blocked: bool,
    pub(crate) native_output_active: bool,
    pub(crate) invisibles_visible: bool,
    pub(crate) ruler_visible: bool,
    /// Replacement handoff lock; deliberately independent from the migration
    /// modal that alone enables File Import Resolve.
    pub(crate) replacement_pending: bool,
    pub(crate) title_page_only_close: bool,
    pub(crate) undo: bool,
    pub(crate) redo: bool,
    pub(crate) delete: bool,
    pub(crate) beat_card_delete: bool,
    pub(crate) select_all: bool,
    pub(crate) cut: bool,
    pub(crate) copy: bool,
    pub(crate) paste: bool,
    pub(crate) body_only: bool,
    pub(crate) editor_available: bool,
    pub(crate) beat_available: bool,
    pub(crate) element_change: bool,
    pub(crate) run_formatting: bool,
    pub(crate) beat_formatting: bool,
    pub(crate) note_text_focus: bool,
    pub(crate) note_formatting: bool,
    pub(crate) paragraph_formatting: bool,
    pub(crate) dual_dialogue_toggle: bool,
    pub(crate) dialogue_alt_add: bool,
    pub(crate) dialogue_alt_delete: bool,
    pub(crate) dialogue_alt_navigate: bool,
    pub(crate) title_open: bool,
    pub(crate) title_close: bool,
    pub(crate) element_windows_available: bool,
    pub(crate) elements_viewport_unsupported: bool,
    pub(crate) selector_viewport_unsupported: bool,
    pub(crate) element_inspector_open: bool,
    pub(crate) template_selector_open: bool,
    pub(crate) template_apply_ready: bool,
    pub(crate) template_new_ready: bool,
    pub(crate) script_note_create: bool,
    pub(crate) script_note_open: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AppCommandProjection {
    pub(crate) availability: AppCommandAvailability,
    pub(crate) say_undual: bool,
    pub(crate) invisibles_visible: bool,
    pub(crate) ruler_visible: bool,
}

/// Every non-session input to the shared availability/label projection. Session
/// content, selections, typing styles, and histories are covered by the session
/// mutation token; platform-specific native-output state is reduced to its
/// effective boolean before entering this key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AppCommandProjectionKey {
    session_mutation_token: u64,
    active_editor_surface: EditorSurface,
    root_document_view: RootDocumentView,
    body_surface_focused: bool,
    title_view_state_swapped: bool,
    beat_text_focused: bool,
    script_note_text_focused: bool,
    title_surface_focused: bool,
    invisibles_visible: bool,
    ruler_visible: bool,
    migration_pending: bool,
    replacement_pending: bool,
    native_output_active: bool,
    title_page_window_open: bool,
    title_viewport_unsupported: bool,
    elements_viewport_unsupported: bool,
    selector_viewport_unsupported: bool,
    script_note_open: bool,
    element_inspector_open: bool,
    template_selector_open: bool,
    template_apply_ready: bool,
    template_new_ready: bool,
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
    pub(crate) fn script_note_body_owns_input(&self) -> bool {
        self.root_document_view == RootDocumentView::Script
            && self.active_editor_surface == EditorSurface::Body
            && self.script_note_workspace.focused
            && self.script_note_workspace.viewport_focused
            && self.script_note_workspace.field == Some(scriptcraft_session::editing::ScriptNoteField::Body)
            && self.session.script_note_selection_is_valid()
            && matches!(self.session.script_note_selection(),
                scriptcraft_session::editing::ScriptNoteSelection::Text {
                    note, field: scriptcraft_session::editing::ScriptNoteField::Body, ..
                } if self.script_note_workspace.open_note.as_ref() == Some(note))
            && !self.title_view_state_swapped
            && !self.title_view.surface_focused
            && !self.element_windows.owns_input()
            && self.pending_fdx_open.is_none()
            && !self.replacement_open_pending()
            && !self.native_output_active()
    }

    fn native_output_active(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            self.native_output_modal_active
                || self.windows_print_preparation.is_some()
                || self.windows_print_session.is_some()
        }
        #[cfg(target_os = "macos")]
        {
            self.native_output_modal_active || self.native_operation_coordinator.is_active()
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            self.native_output_modal_active
        }
    }

    fn app_command_projection_key(&self) -> AppCommandProjectionKey {
        AppCommandProjectionKey {
            session_mutation_token: self.session.mutation_token(),
            active_editor_surface: self.active_editor_surface,
            root_document_view: self.root_document_view,
            body_surface_focused: self.surface_focused,
            title_view_state_swapped: self.title_view_state_swapped,
            beat_text_focused: self.beat_text.focused,
            script_note_text_focused: self.script_note_workspace.focused
                && self.script_note_workspace.viewport_focused
                && self.session.script_note_selection_is_valid()
                && matches!(self.session.script_note_selection(),
                    scriptcraft_session::editing::ScriptNoteSelection::Text { note, .. }
                        if self.script_note_workspace.open_note.as_ref() == Some(note)),
            title_surface_focused: self.title_view.surface_focused,
            invisibles_visible: self.active_surface_invisibles_visible(),
            ruler_visible: self.settings_store.ruler_visible(),
            migration_pending: self.pending_fdx_open.is_some(),
            replacement_pending: self.replacement_open_pending(),
            native_output_active: self.native_output_active(),
            title_page_window_open: self.title_page_window_open,
            title_viewport_unsupported: self.title_viewport_unsupported,
            elements_viewport_unsupported: self.element_windows.elements_viewport_unsupported(),
            selector_viewport_unsupported: self.element_windows.selector_viewport_unsupported(),
            script_note_open: self.script_note_workspace.open_note.is_some(),
            element_inspector_open: self.element_windows.elements_open(),
            template_selector_open: self.element_windows.selector_open(),
            template_apply_ready: self.element_windows.selector_apply_ready(),
            template_new_ready: self.element_windows.selector_new_ready(),
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
            invisibles_visible: key.invisibles_visible,
            ruler_visible: key.ruler_visible,
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
            availability.element_change = availability.editor_available && *surface == self.active_editor_surface
                && scriptcraft_commands::Selection::from_core(core_selection)
                .ok()
                .is_some_and(|selection| {
                    selection
                        .validate_for_surface(self.session.project(), *surface)
                        .is_ok()
                        && selection.anchor.element == selection.focus.element
                });
        }
        if let scriptcraft_commands::Command::EditorRunFormattingSet { surface, .. }
            | scriptcraft_commands::Command::EditorParagraphFormattingSet { surface, .. } = command
        {
            let eligible = availability.editor_available && *surface == EditorSurface::Body
                && self.active_editor_surface == EditorSurface::Body
                && scriptcraft_commands::Selection::from_core(self.session.selection())
                    .ok()
                    .is_some_and(|selection| selection.validate_for_surface(self.session.project(), EditorSurface::Body).is_ok());
            availability.run_formatting = eligible || (*surface == EditorSurface::Body && availability.note_formatting);
            availability.paragraph_formatting = eligible;
        }
        if let scriptcraft_commands::Command::EditorStyleToggle { surface, .. }
            | scriptcraft_commands::Command::EditorStyleClear { surface } = command
        {
            if *surface != self.active_editor_surface || availability.note_text_focus {
                availability.beat_formatting = false;
            }
        }
        if let scriptcraft_commands::Command::EditorRunFormattingSet { surface, .. } = command {
            if *surface != EditorSurface::Body {
                availability.beat_formatting = false;
            }
        }
        if let scriptcraft_commands::Command::BeatBoardCardDelete { card } = command {
            availability.beat_card_delete &= matches!(self.session.beat_selection(),
                scriptcraft_session::editing::BeatSelection::Card { card: selected } if selected == card.as_core());
        }
        if matches!(command, scriptcraft_commands::Command::BeatBoardStyleToggle { .. }
            | scriptcraft_commands::Command::BeatBoardStyleClear
            | scriptcraft_commands::Command::BeatBoardRunFormattingSet { .. }) {
            availability.beat_formatting = availability.beat_available
                && self.beat_text.focused
                && matches!(self.session.beat_selection(),
                    scriptcraft_session::editing::BeatSelection::Text {
                        field: scriptcraft_session::editing::BeatTextField::Body, ..
                    }) && self.session.beat_selection_is_valid();
        }
        if matches!(command, scriptcraft_commands::Command::ScriptNoteStyleToggle { .. }
            | scriptcraft_commands::Command::ScriptNoteStyleClear { .. }
            | scriptcraft_commands::Command::ScriptNoteRunFormattingSet { .. }) {
            availability.note_formatting = availability.note_formatting && self.script_note_body_owns_input();
        }
        if let scriptcraft_commands::Command::FormatTemplateApply { template } = command {
            availability.template_apply_ready = self
                .element_windows
                .selector_apply_matches(builtin_template_id(*template));
        }
        if let scriptcraft_commands::Command::FileNewFromTemplate { template } = command {
            availability.template_new_ready = self
                .element_windows
                .selector_new_matches(builtin_template_id(*template));
        }
        if let scriptcraft_commands::Command::ScriptNoteDelete { note }
            | scriptcraft_commands::Command::ScriptNotePropertiesSet {
                properties: scriptcraft_commands::ScriptNotePropertiesValue { note, .. },
            }
            | scriptcraft_commands::Command::ScriptNoteSelectionSet {
                selection: scriptcraft_commands::ScriptNoteSelectionValue::Text { note, .. },
            }
            | scriptcraft_commands::Command::ScriptNotePresent {
                presentation: scriptcraft_commands::ScriptNotePresentValue { note, .. },
            } = command
        {
            availability.script_note_open &= self.session.project().script_notes().get(note.as_core()).is_some()
                && self.script_note_workspace.open_note.as_ref() == Some(note.as_core());
        }
        if let scriptcraft_commands::Command::ScriptNotePresent { presentation } = command {
            availability.script_note_open = self.session.project().script_notes().get(presentation.note.as_core()).is_some()
                && availability.editor_available && self.active_editor_surface == EditorSurface::Body
                && self.root_document_view == RootDocumentView::Script
                && !availability.modal_blocked && !availability.replacement_pending
                && !availability.native_output_active && !self.element_windows.owns_input();
        }
        if matches!(command, scriptcraft_commands::Command::ScriptNoteTextEdit { .. }
            | scriptcraft_commands::Command::ScriptNoteTextNavigate { .. })
        {
            availability.script_note_open &= self.session.script_note_selection_is_valid()
                && matches!(self.session.script_note_selection(),
                    scriptcraft_session::editing::ScriptNoteSelection::Text { note, .. }
                        if self.script_note_workspace.open_note.as_ref() == Some(note));
        }
        availability
    }

    fn app_command_availability_without_dual_dialogue(&self) -> AppCommandAvailability {
        let migration_pending = self.pending_fdx_open.is_some();
        let editor_available = self.active_editor_surface == EditorSurface::TitlePage
            || self.title_view_state_swapped || self.root_document_view == RootDocumentView::Script;
        let replacement_pending = self.replacement_open_pending();
        let native_output_active = self.native_output_active();
        let elements_viewport_unsupported = self.element_windows.elements_viewport_unsupported();
        let selector_viewport_unsupported = self.element_windows.selector_viewport_unsupported();
        let element_inspector_open = self.element_windows.elements_open();
        let template_selector_open = self.element_windows.selector_open();
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
        let element_change = self.active_editor_surface == EditorSurface::Body
            && scriptcraft_commands::Selection::from_core(self.session.selection())
                .ok()
                .is_some_and(|selection| {
                    selection
                        .validate_for_surface(self.session.project(), EditorSurface::Body)
                        .is_ok()
                        && selection.anchor.element == selection.focus.element
                });
        let valid_body_selection = scriptcraft_commands::Selection::from_core(self.session.selection())
            .ok()
            .is_some_and(|selection| {
                selection
                    .validate_for_surface(self.session.project(), EditorSurface::Body)
                    .is_ok()
            });
        let script_note_create = self.root_document_view == RootDocumentView::Script
            && self.active_editor_surface == EditorSurface::Body
            && !self.title_view_state_swapped
            && !self.title_view.surface_focused
            && !self.element_windows.owns_input()
            && !migration_pending && !replacement_pending && !native_output_active
            && valid_body_selection;
        let script_note_open = self.root_document_view == RootDocumentView::Script
            && self.active_editor_surface == EditorSurface::Body
            && !self.title_view_state_swapped
            && !self.title_view.surface_focused
            && !self.element_windows.owns_input()
            && !migration_pending && !replacement_pending && !native_output_active
            && self.script_note_workspace.open_note.is_some();
        let note_text_focus = self.script_note_workspace.focused
            && self.script_note_workspace.viewport_focused
            && self.session.script_note_selection_is_valid()
            && matches!(self.session.script_note_selection(),
                scriptcraft_session::editing::ScriptNoteSelection::Text { note, .. }
                    if self.script_note_workspace.open_note.as_ref() == Some(note))
            && self.active_editor_surface == EditorSurface::Body
            && self.root_document_view == RootDocumentView::Script
            && !self.element_windows.owns_input()
            && !migration_pending && !replacement_pending && !native_output_active;
        let note_body_focus = note_text_focus && matches!(self.session.script_note_selection(),
            scriptcraft_session::editing::ScriptNoteSelection::Text {
                field: scriptcraft_session::editing::ScriptNoteField::Body, ..
            }) && self.script_note_workspace.field == Some(scriptcraft_session::editing::ScriptNoteField::Body);
        let note_has_range = note_text_focus && self.session.script_note_selection().ordered()
            .is_some_and(|(start, end)| start != end);
        let beat_text_focus = self.root_document_view == RootDocumentView::BeatBoard
            && self.active_editor_surface == EditorSurface::Body
            && !self.title_view_state_swapped
            && self.beat_text.focused
            && !self.element_windows.owns_input()
            && !migration_pending && !replacement_pending && !native_output_active;
        let (beat_has_text, beat_has_range) = match self.session.beat_selection() {
            scriptcraft_session::editing::BeatSelection::Text { anchor, focus, .. } => (true, anchor != focus),
            _ => (false, false),
        };
        let beat_card_delete = self.root_document_view == RootDocumentView::BeatBoard
            && self.active_editor_surface == EditorSurface::Body
            && !self.title_view_state_swapped && !self.beat_text.focused
            && !self.title_view.surface_focused && !self.element_windows.owns_input()
            && !migration_pending && !replacement_pending && !native_output_active
            && self.session.beat_selection_is_valid()
            && matches!(self.session.beat_selection(), scriptcraft_session::editing::BeatSelection::Card { .. });
        let dialogue_alt_target = self.session.dialogue_alt_target();
        let dialogue_alt_controls_available = self.root_document_view == RootDocumentView::Script
            && self.active_editor_surface == EditorSurface::Body
            && !self.title_view_state_swapped
            && self.surface_focused
            && !self.title_view.surface_focused
            && !self.element_windows.owns_input()
            && !(self.script_note_workspace.focused && self.script_note_workspace.viewport_focused)
            && !migration_pending
            && !replacement_pending
            && !native_output_active;
        let dialogue_alt_eligible = dialogue_alt_controls_available
            && dialogue_alt_target.as_ref().is_some_and(|target| {
                target.collapsed_dialogue_owner && target.inline_source_supported
            });

        AppCommandAvailability {
            modal_blocked: migration_pending && !replacement_pending,
            native_output_active,
            invisibles_visible: self.active_surface_invisibles_visible(),
            ruler_visible: self.settings_store.ruler_visible(),
            replacement_pending,
            title_page_only_close: self.active_editor_surface() == EditorSurface::TitlePage
                && self.title_page_window_open,
            undo: !migration_pending && !replacement_pending && self.session.can_undo(),
            redo: !migration_pending && !replacement_pending && self.session.can_redo(),
            delete: if note_text_focus { false } else if beat_text_focus { beat_has_range } else if beat_card_delete { true } else { editor_available && !migration_pending && !replacement_pending && delete_target_exists },
            beat_card_delete,
            // EditingSession projects a document with at least one body element.
            select_all: if note_text_focus { true } else if beat_text_focus { beat_has_text } else { editor_available && !migration_pending && !replacement_pending && element_count != 0 },
            cut: if note_text_focus { note_has_range } else if beat_text_focus { beat_has_range } else { editor_available && !migration_pending && !replacement_pending && selection.anchor != selection.focus },
            copy: if note_text_focus { note_has_range } else if beat_text_focus { beat_has_range } else { editor_available && !migration_pending && !replacement_pending && selection.anchor != selection.focus },
            // Clipboard state is read only when Paste is activated, never per frame.
            paste: if note_text_focus { true } else if beat_text_focus { beat_has_text } else { editor_available && !migration_pending && !replacement_pending },
            body_only: editor_available && self.active_editor_surface == EditorSurface::Body,
            editor_available,
            beat_available: self.root_document_view == RootDocumentView::BeatBoard
                && self.active_editor_surface == EditorSurface::Body && !self.title_view_state_swapped
                && !self.title_view.surface_focused && !self.element_windows.owns_input(),
            element_change: editor_available && element_change,
            run_formatting: editor_available && self.active_editor_surface == EditorSurface::Body && valid_body_selection,
            beat_formatting: self.root_document_view == RootDocumentView::BeatBoard
                && self.active_editor_surface == EditorSurface::Body && !self.title_view_state_swapped
                && !self.title_view.surface_focused && self.beat_text.focused
                && !self.element_windows.owns_input() && !migration_pending
                && !replacement_pending && !native_output_active
                && matches!(self.session.beat_selection(),
                    scriptcraft_session::editing::BeatSelection::Text {
                        field: scriptcraft_session::editing::BeatTextField::Body, ..
                    }) && self.session.beat_selection_is_valid(),
            note_text_focus,
            note_formatting: note_body_focus,
            paragraph_formatting: editor_available && self.active_editor_surface == EditorSurface::Body && valid_body_selection,
            dual_dialogue_toggle: false,
            dialogue_alt_add: dialogue_alt_eligible
                && dialogue_alt_target.as_ref().is_some_and(|target| target.can_add),
            dialogue_alt_delete: dialogue_alt_eligible
                && dialogue_alt_target.as_ref().is_some_and(|target| target.can_delete),
            dialogue_alt_navigate: dialogue_alt_eligible
                && dialogue_alt_target.as_ref().is_some_and(|target| target.can_previous && target.can_next),
            // In-app Open is a selected-path replacement request and is blocked
            // while the current document is resolving another lifecycle gate.
            title_open: !migration_pending
                && !replacement_pending
                && !self.title_viewport_unsupported,
            // Closing the presentation-only child viewport does not destroy
            // the root document and remains available during replacement.
            title_close: !migration_pending && self.title_page_window_open,
            // The element UI owns input independently of the migration modal.
            // Its state changes menu/command availability but never changes the
            // migration reason used by File Import Resolve.
            element_windows_available: !migration_pending
                && !replacement_pending
                && !native_output_active,
            elements_viewport_unsupported,
            selector_viewport_unsupported,
            element_inspector_open,
            template_selector_open,
            template_apply_ready: self.element_windows.selector().is_some_and(|selector| {
                selector.mode() == TemplateSelectorMode::ApplyToCurrent
                    && selector.selected_template().is_some()
            }),
            template_new_ready: self.element_windows.selector().is_some_and(|selector| {
                selector.mode() == TemplateSelectorMode::NewDocument
                    && selector.selected_template().is_some()
            }),
            script_note_create,
            script_note_open,
        }
    }

    fn dual_dialogue_toggle_target(
        &mut self,
    ) -> Option<scriptcraft_model::dual_dialogue::ToggleTarget> {
        if self.active_editor_surface != EditorSurface::Body || self.root_document_view == RootDocumentView::BeatBoard {
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
        if self.template_selector_open || self.element_inspector_open {
            return match command {
                // Dialog presentation keeps only its own typed actions, harmless
                // focus activations, status projection, and root lifecycle.
                CommandId::UiStatusSet | CommandId::ApplicationQuit => true,
                CommandId::FormatTemplateSelectorOpen => !self.selector_viewport_unsupported,
                CommandId::FormatElementsOpen => self.element_inspector_open,
                CommandId::FormatElementsClose => {
                    self.element_inspector_open && !self.template_selector_open
                }
                CommandId::FormatTemplateSelectorCancel => self.template_selector_open,
                CommandId::FormatTemplateApply => self.template_apply_ready,
                CommandId::FileNewFromTemplate => self.template_new_ready,
                CommandId::FileClose => !self.title_page_only_close,
                _ => false,
            };
        }
        match command {
            CommandId::BeatBoardCardCreate | CommandId::BeatBoardCardActivate
            | CommandId::BeatBoardCardGeometrySet | CommandId::BeatBoardSelectionSet
            | CommandId::BeatBoardTextEdit | CommandId::BeatBoardNavigation => self.beat_available,
            CommandId::BeatBoardCardDelete => self.beat_card_delete,
            CommandId::BeatBoardStyleToggle | CommandId::BeatBoardStyleClear
            | CommandId::BeatBoardRunFormattingSet => self.beat_formatting,
            CommandId::ScriptNoteStyleToggle | CommandId::ScriptNoteStyleClear
            | CommandId::ScriptNoteRunFormattingSet => self.note_formatting,
            CommandId::ScriptNoteCreate => self.script_note_create,
            CommandId::ScriptNoteDelete | CommandId::ScriptNotePropertiesSet
            | CommandId::ScriptNoteSelectionSet | CommandId::ScriptNoteTextEdit
            | CommandId::ScriptNoteTextNavigate | CommandId::ScriptNotePresent => self.script_note_open,
            CommandId::ScriptNoteDismiss => true,
            CommandId::EditorTextInsert | CommandId::EditorTextPaste | CommandId::EditorDeleteBackward
            | CommandId::EditorParagraphEnter | CommandId::EditorParagraphTab
            | CommandId::EditorSelectionSet | CommandId::EditorSelectionPointer
            | CommandId::EditorNavigationHorizontal | CommandId::EditorNavigationVertical
            | CommandId::EditorNavigationLineEdge | CommandId::EditorFocus
            | CommandId::EditorAlignmentSet => self.editor_available,
            CommandId::EditorStyleToggle | CommandId::EditorStyleClear =>
                if self.note_text_focus { self.note_formatting } else { self.editor_available || self.beat_formatting },
            CommandId::EditUndo => self.undo,
            CommandId::EditRedo => self.redo,
            CommandId::EditDelete => self.delete,
            CommandId::EditSelectAll => self.select_all,
            CommandId::EditCut => self.cut,
            CommandId::EditCopy => self.copy,
            CommandId::EditPaste => self.paste,
            CommandId::EditorElementChange => self.element_change,
            CommandId::EditorRunFormattingSet => if self.note_text_focus { self.note_formatting } else { self.run_formatting || self.beat_formatting },
            CommandId::EditorParagraphFormattingSet => self.paragraph_formatting,
            CommandId::EditorDualDialogueToggle => self.dual_dialogue_toggle,
            CommandId::EditorDialogueAltAdd => self.dialogue_alt_add,
            CommandId::EditorDialogueAltDelete => self.dialogue_alt_delete,
            CommandId::EditorDialogueAltNavigate => self.dialogue_alt_navigate,
            CommandId::WindowTitlePageOpen => self.title_open,
            CommandId::WindowTitlePageClose => self.title_close,
            CommandId::FormatElementsOpen => {
                self.element_windows_available && !self.elements_viewport_unsupported
            }
            CommandId::FormatElementsClose => {
                self.element_windows_available && self.element_inspector_open
            }
            CommandId::FormatTemplateSelectorOpen => {
                self.element_windows_available
                    && self.element_inspector_open
                    && !self.selector_viewport_unsupported
            }
            CommandId::FormatTemplateSelectorCancel => self.template_selector_open,
            CommandId::FormatTemplateApply => self.template_apply_ready,
            CommandId::FileNewFromTemplateOpen => {
                self.element_windows_available && !self.selector_viewport_unsupported
            }
            CommandId::FileNewFromTemplate => self.template_new_ready,
            // The global modal gates above still disable Quit during migration
            // and native-output dialogs; otherwise it is focus-independent.
            CommandId::ApplicationQuit => true,
            CommandId::ViewRulerToggle => true,
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
            CommandId::BeatBoardCardDelete => DesktopUnavailableReason::Selection,
            CommandId::EditSelectAll | CommandId::EditCut | CommandId::EditCopy => {
                DesktopUnavailableReason::Selection
            }
            CommandId::EditorElementChange
            | CommandId::EditorDualDialogueToggle
            | CommandId::EditorDialogueAltAdd
            | CommandId::EditorDialogueAltDelete
            | CommandId::EditorDialogueAltNavigate
            | CommandId::EditorRunFormattingSet
            | CommandId::EditorParagraphFormattingSet
            | CommandId::ScriptNoteCreate | CommandId::ScriptNoteDelete
            | CommandId::ScriptNotePropertiesSet | CommandId::ScriptNoteSelectionSet
            | CommandId::ScriptNoteTextEdit | CommandId::ScriptNoteTextNavigate
            | CommandId::ScriptNotePresent | CommandId::ScriptNoteStyleToggle
            | CommandId::ScriptNoteStyleClear | CommandId::ScriptNoteRunFormattingSet => DesktopUnavailableReason::Selection,
            _ => DesktopUnavailableReason::ModalBlocked,
        }
    }
}

const fn builtin_template_id(value: BuiltinTemplateValue) -> scriptcraft_fdx::BuiltinTemplateId {
    match value {
        BuiltinTemplateValue::Screenplay => scriptcraft_fdx::BuiltinTemplateId::Screenplay,
        BuiltinTemplateValue::Stageplay => scriptcraft_fdx::BuiltinTemplateId::Stageplay,
        BuiltinTemplateValue::DramatistsGuildMusical => {
            scriptcraft_fdx::BuiltinTemplateId::DramatistsGuildMusical
        }
    }
}

use crate::app::ScriptCraftApp;
use crate::views::script::RevealIntent;
use eframe::egui;
use scriptcraft_commands::{
    execute_editor_command_on_surface, Command, CommandId, CommandOutcome, CommandRejection,
    UnavailableReason, EditorSurface,
};

use super::{DesktopCommandOutcome, DesktopUnavailableReason};

impl ScriptCraftApp {
    pub(super) fn dispatch_script_note_host_command(
        &mut self,
        command: Command,
        ctx: &egui::Context,
    ) -> DesktopCommandOutcome {
        match command {
            Command::ScriptNotePresent { presentation } => {
                let same_open_note = self.script_note_workspace.open_note.as_ref() == Some(presentation.note.as_core());
                let Some(note) = self.session.project().script_notes().get(presentation.note.as_core()) else {
                    return DesktopCommandOutcome::Unavailable(DesktopUnavailableReason::Selection);
                };
                if presentation.move_caret {
                    let Some(position) = scriptcraft_model::body_offsets::flat_offset_to_position(
                        self.session.project(), note.range.start(),
                    ) else {
                        return DesktopCommandOutcome::Unavailable(DesktopUnavailableReason::Selection);
                    };
                    let selection = match scriptcraft_commands::Selection::from_core(
                        scriptcraft_model::Selection::caret(position),
                    ) {
                        Ok(selection) => selection,
                        Err(error) => return DesktopCommandOutcome::Rejected(
                            CommandRejection::PositionOutOfRange(error),
                        ),
                    };
                    match self.dispatch_internal_command(Command::EditorSelectionSet {
                        selection,
                        surface: EditorSurface::Body,
                    }, ctx) {
                        DesktopCommandOutcome::Editor(CommandOutcome::Changed | CommandOutcome::NoOp) => {}
                        outcome => return outcome,
                    }
                }
                self.script_note_workspace.present(
                    presentation.note.as_core().clone(), presentation.focus_body,
                );
                self.script_note_workspace.reveal_pending = presentation.move_caret;
                if !same_open_note && !presentation.focus_body && !presentation.move_caret {
                    self.script_note_workspace.focused = false;
                    self.script_note_workspace.field = None;
                    self.script_note_workspace.ime_active = false;
                    self.script_note_workspace.ime_preedit.clear();
                }
                if !presentation.move_caret && !presentation.focus_body {
                    self.script_note_workspace.pointer_anchor = ctx.input(|input| input.pointer.latest_pos());
                }
                ctx.request_repaint();
                DesktopCommandOutcome::EffectDispatched
            }
            Command::ScriptNoteDismiss => {
                self.dismiss_script_note(ctx);
                DesktopCommandOutcome::EffectDispatched
            }
            other => DesktopCommandOutcome::Rejected(CommandRejection::UnsupportedCommand(other.id())),
        }
    }

    pub(crate) fn dismiss_script_note(&mut self, ctx: &egui::Context) {
        self.script_note_workspace.dismiss();
        crate::widgets::script_note::viewport::close(ctx);
        crate::widgets::script_note::properties::flush_pending_closes(
            ctx,
            &mut self.script_note_workspace,
        );
        ctx.request_repaint();
    }

    pub(super) fn dispatch_editing_command(
        &mut self,
        command: Command,
        ctx: &egui::Context,
    ) -> DesktopCommandOutcome {
        match command {
            command @ (Command::BeatBoardCardCreate { .. } | Command::BeatBoardCardActivate { .. }
                | Command::BeatBoardCardGeometrySet { .. } | Command::BeatBoardSelectionSet { .. }
                | Command::BeatBoardTextEdit { .. } | Command::BeatBoardNavigation { .. }
                | Command::BeatBoardCardDelete { .. }
                | Command::BeatBoardStyleToggle { .. } | Command::BeatBoardStyleClear
                | Command::BeatBoardRunFormattingSet { .. }) =>
                self.execute_editor(&command, RevealIntent::None, ctx),
            command @ (Command::EditorDialogueAltAdd | Command::EditorDialogueAltDelete
                | Command::EditorDialogueAltNavigate { .. }) =>
                self.execute_editor(&command, RevealIntent::None, ctx),
            command @ (Command::ScriptNoteCreate { .. } | Command::ScriptNoteDelete { .. }
                | Command::ScriptNotePropertiesSet { .. } | Command::ScriptNoteSelectionSet { .. }
                | Command::ScriptNoteTextEdit { .. } | Command::ScriptNoteTextNavigate { .. }
                | Command::ScriptNoteStyleToggle { .. } | Command::ScriptNoteStyleClear { .. }
                | Command::ScriptNoteRunFormattingSet { .. }) =>
                self.execute_editor(&command, RevealIntent::None, ctx),
            Command::EditorStyleToggle { surface, style } => self.execute_editor(
                &Command::EditorStyleToggle { surface, style },
                RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorStyleClear { surface } => self.execute_editor(
                &Command::EditorStyleClear { surface },
                RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorAlignmentSet { surface, alignment } => self.execute_editor(
                &Command::EditorAlignmentSet { surface, alignment },
                RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorRunFormattingSet { surface, patch } => self.execute_editor(
                &Command::EditorRunFormattingSet { surface, patch },
                RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorParagraphFormattingSet { surface, patch } => self.execute_editor(
                &Command::EditorParagraphFormattingSet { surface, patch },
                RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorDualDialogueToggle => {
                self.execute_editor(&Command::EditorDualDialogueToggle, RevealIntent::None, ctx)
            }
            Command::EditUndo => {
                self.execute_editor(&Command::EditUndo, RevealIntent::Conditional, ctx)
            }
            Command::EditRedo => {
                self.execute_editor(&Command::EditRedo, RevealIntent::Conditional, ctx)
            }
            Command::EditDelete => {
                if self.script_note_workspace.focused
                    && self.script_note_workspace.viewport_focused
                    && self.session.script_note_selection_is_valid()
                    && matches!(self.session.script_note_selection(),
                        scriptcraft_session::editing::ScriptNoteSelection::Text { note, .. }
                            if self.script_note_workspace.open_note.as_ref() == Some(note))
                {
                    // Text-field Delete is handled by the native card as note
                    // text editing. Global EditDelete never deletes a note.
                    return DesktopCommandOutcome::Unavailable(DesktopUnavailableReason::Selection);
                }
                if self.root_document_view == crate::app::RootDocumentView::BeatBoard {
                    match self.session.beat_selection() {
                        scriptcraft_session::editing::BeatSelection::Card { card } => {
                            let delete = Command::BeatBoardCardDelete {
                                card: scriptcraft_commands::BeatCardIdValue::from_core(card.clone()),
                            };
                            let availability = self.command_dispatch_availability(&delete);
                            if !availability.is_enabled(delete.id()) {
                                return super::DesktopCommandOutcome::Unavailable(availability.unavailable_reason(delete.id()));
                            }
                            return self.execute_editor(&delete, RevealIntent::None, ctx);
                        }
                        scriptcraft_session::editing::BeatSelection::Text { anchor, focus, .. } if self.beat_text.focused => {
                            if anchor != focus {
                                return self.execute_editor(&Command::BeatBoardTextEdit { operation: scriptcraft_commands::BeatTextEditValue::DeleteForward }, RevealIntent::None, ctx);
                            }
                            return super::DesktopCommandOutcome::Unavailable(super::DesktopUnavailableReason::Selection);
                        }
                        _ if self.beat_text.focused => return super::DesktopCommandOutcome::Unavailable(super::DesktopUnavailableReason::Selection),
                        _ => {}
                    }
                }
                self.execute_editor(&Command::EditDelete, RevealIntent::Conditional, ctx)
            }
            Command::EditorTextInsert { text, surface } => self.execute_editor(
                &Command::EditorTextInsert { text, surface },
                RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorTextPaste { text, surface } => self.execute_editor(
                &Command::EditorTextPaste { text, surface },
                RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorDeleteBackward => self.execute_editor(
                &Command::EditorDeleteBackward,
                RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorParagraphEnter => self.execute_editor(
                &Command::EditorParagraphEnter,
                RevealIntent::Conditional,
                ctx,
            ),
            Command::EditorParagraphTab => {
                self.execute_editor(&Command::EditorParagraphTab, RevealIntent::Conditional, ctx)
            }
            Command::EditSelectAll => {
                if self.script_note_workspace.focused && self.script_note_workspace.viewport_focused {
                    let Some(note_id) = self.script_note_workspace.open_note.clone() else {
                        return DesktopCommandOutcome::Unavailable(DesktopUnavailableReason::Selection);
                    };
                    let Some(note) = self.session.project().script_notes().get(&note_id) else {
                        return DesktopCommandOutcome::Unavailable(DesktopUnavailableReason::Selection);
                    };
                    let Some(field) = self.script_note_workspace.field else {
                        return DesktopCommandOutcome::Unavailable(DesktopUnavailableReason::Selection);
                    };
                    let command = crate::widgets::script_note::text_input::select_all(&note_id, field, note);
                    return self.execute_editor(&command, RevealIntent::None, ctx);
                }
                if self.root_document_view == crate::app::RootDocumentView::BeatBoard && self.beat_text.focused {
                    if let scriptcraft_session::editing::BeatSelection::Text { card, field, .. } = self.session.beat_selection() {
                        if let Some(card) = self.session.project().beat_board().cards.iter().find(|item| &item.id == card) {
                            let (field_value, anchor, focus) = match field {
                                scriptcraft_session::editing::BeatTextField::Title => (scriptcraft_commands::BeatTextFieldValue::Title,
                                    scriptcraft_session::editing::BeatTextPosition { paragraph: 0, offset: 0 },
                                    scriptcraft_session::editing::BeatTextPosition { paragraph: 0, offset: card.title.len() }),
                                scriptcraft_session::editing::BeatTextField::Body => {
                                    let last = card.paragraphs.last().map_or(0, |paragraph| paragraph.text.len());
                                    (scriptcraft_commands::BeatTextFieldValue::Body,
                                     scriptcraft_session::editing::BeatTextPosition { paragraph: 0, offset: 0 },
                                     scriptcraft_session::editing::BeatTextPosition { paragraph: card.paragraphs.len().saturating_sub(1), offset: last })
                                }
                            };
                            let selection = scriptcraft_commands::BeatSelectionValue::Text {
                                card: scriptcraft_commands::BeatCardIdValue::from_core(card.id.clone()), field: field_value,
                                anchor: scriptcraft_commands::BeatTextPositionValue { paragraph: anchor.paragraph as u32, offset: anchor.offset as u32 },
                                focus: scriptcraft_commands::BeatTextPositionValue { paragraph: focus.paragraph as u32, offset: focus.offset as u32 },
                            };
                            return self.execute_editor(&Command::BeatBoardSelectionSet { selection }, RevealIntent::None, ctx);
                        }
                    }
                    return super::DesktopCommandOutcome::Unavailable(super::DesktopUnavailableReason::Selection);
                }
                self.execute_editor(&Command::EditSelectAll, RevealIntent::Conditional, ctx)
            }
            Command::EditorElementChange {
                element_type,
                surface,
            } => self.execute_editor(
                &Command::EditorElementChange {
                    element_type,
                    surface,
                },
                RevealIntent::None,
                ctx,
            ),
            Command::EditorSelectionSet { selection, surface } => self.execute_editor(
                &Command::EditorSelectionSet { selection, surface },
                RevealIntent::Conditional,
                ctx,
            ),
            other => {
                DesktopCommandOutcome::Rejected(CommandRejection::UnsupportedCommand(other.id()))
            }
        }
    }

    pub(super) fn execute_editor(
        &mut self,
        command: &Command,
        reveal_intent: RevealIntent,
        ctx: &egui::Context,
    ) -> DesktopCommandOutcome {
        let started = self.development_mode.then(std::time::Instant::now);
        if self.development_mode {
            self.responsiveness.begin_editor_command();
        }
        let outcome = self.execute_editor_inner(command, reveal_intent, ctx);
        if let Some(started) = started {
            self.responsiveness.end_editor_command(started.elapsed());
        }
        outcome
    }

    fn execute_editor_inner(
        &mut self,
        command: &Command,
        reveal_intent: RevealIntent,
        ctx: &egui::Context,
    ) -> DesktopCommandOutcome {
        // Defense in depth for stale/direct editor calls that bypass command
        // availability after a replacement handoff has engaged the lock.
        if self.replacement_open_pending() {
            if self.development_mode
                && matches!(
                    command.id(),
                    CommandId::EditorDialogueAltAdd
                        | CommandId::EditorDialogueAltDelete
                        | CommandId::EditorDialogueAltNavigate
                )
            {
                self.responsiveness.record_alt_unavailable();
            }
            return DesktopCommandOutcome::Unavailable(DesktopUnavailableReason::ModalBlocked);
        }
        let edit_input_started = (self.development_mode
            && crate::app::responsiveness::is_text_editing_command(command.id()))
        .then(std::time::Instant::now);
        let active_surface = self.active_editor_surface();
        let beat_command = matches!(command, Command::BeatBoardCardCreate { .. } | Command::BeatBoardCardActivate { .. }
            | Command::BeatBoardCardGeometrySet { .. } | Command::BeatBoardCardDelete { .. } | Command::BeatBoardSelectionSet { .. }
            | Command::BeatBoardTextEdit { .. } | Command::BeatBoardNavigation { .. }
            | Command::BeatBoardStyleToggle { .. } | Command::BeatBoardStyleClear
            | Command::BeatBoardRunFormattingSet { .. });
        if self.development_mode {
            self.session.begin_mutation_diagnostics();
        }
        let executor_started = self.development_mode.then(std::time::Instant::now);
        let outcome = execute_editor_command_on_surface(&mut self.session, command, active_surface);
        if self.development_mode
            && matches!(
                command.id(),
                CommandId::EditorDialogueAltAdd
                    | CommandId::EditorDialogueAltDelete
                    | CommandId::EditorDialogueAltNavigate
            )
        {
            self.responsiveness.record_alt_outcome(&outcome);
        }
        let executor_duration = executor_started.map(|started| started.elapsed());
        let mutation_diagnostics = if self.development_mode {
            self.session.take_mutation_diagnostics()
        } else {
            None
        };
        if let (Some(executor_duration), Some(observation)) =
            (executor_duration, mutation_diagnostics)
        {
            self.responsiveness
                .record_command_detail(executor_duration, observation);
        }
        match &outcome {
            CommandOutcome::Changed => {
                if let Some(accepted_at) = edit_input_started {
                    self.responsiveness
                        .note_accepted_edit(self.session.revision(), accepted_at);
                }
                if !beat_command { self.mutate(Ok(()), reveal_intent); }
            }
            CommandOutcome::NoOp if command.id() != CommandId::EditorDualDialogueToggle => {
                if !beat_command { self.mutate(Ok(()), reveal_intent); }
            }
            CommandOutcome::NoOp => {}
            CommandOutcome::Unavailable(_) | CommandOutcome::Rejected(_) => {}
            CommandOutcome::Failed(error) => self.show_edit_error(error.clone()),
            // Apply Template has its own bounded inline selector error path and
            // is normally routed by `dispatch_elements_command`.
            CommandOutcome::ApplyTemplateFailed(_) => {}
        }
        if !beat_command && matches!(&outcome, CommandOutcome::Changed | CommandOutcome::NoOp)
            && !(command.id() == CommandId::EditorDualDialogueToggle
                && matches!(&outcome, CommandOutcome::NoOp))
        {
            self.reconcile_body_active_group();
        }
        ctx.request_repaint();
        match outcome {
            CommandOutcome::Unavailable(reason) => {
                DesktopCommandOutcome::Unavailable(match reason {
                    UnavailableReason::UndoHistory => DesktopUnavailableReason::UndoHistory,
                    UnavailableReason::RedoHistory => DesktopUnavailableReason::RedoHistory,
                    UnavailableReason::Selection => DesktopUnavailableReason::Selection,
                    UnavailableReason::DualDialogueTarget => DesktopUnavailableReason::Selection,
                })
            }
            CommandOutcome::Rejected(rejection) => DesktopCommandOutcome::Rejected(rejection),
            semantic_outcome => DesktopCommandOutcome::Editor(semantic_outcome),
        }
    }
}

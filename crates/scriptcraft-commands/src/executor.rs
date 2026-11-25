//! Headless semantic execution over the shared transactional editing session.
//!
//! Native viewport and dialog intents are typed in the command vocabulary but
//! remain host effects; this executor never performs them.

use scriptcraft_model::{Position as CorePosition, Selection as CoreSelection};
use scriptcraft_session::editing::{
    ApplyTemplateError, ApplyTemplateOutcome, BeatSelection, BeatTextField, EditError, EditingSession,
    ScriptNoteColorUpdate, ScriptNoteCreationInput, ScriptNoteSelection,
};

use crate::{Command, CommandId, EditorSurface, PointerPhase, PositionError, SelectionError};

/// Result of applying a semantic editor command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandOutcome {
    Changed,
    NoOp,
    Unavailable(UnavailableReason),
    Rejected(CommandRejection),
    Failed(EditError),
    /// A bounded target-stage or current-candidate failure from Apply Template.
    ApplyTemplateFailed(ApplyTemplateError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnavailableReason {
    UndoHistory,
    RedoHistory,
    Selection,
    DualDialogueTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRejection {
    UnsupportedCommand(CommandId),
    InvalidElementTarget,
    UnsupportedSurface,
    InvalidSelection(SelectionError),
    PositionOutOfRange(PositionError),
    InvalidScriptNoteValue,
}

/// Executes with the legacy Body target for generic surface-less commands.
pub fn execute_editor_command(session: &mut EditingSession, command: &Command) -> CommandOutcome {
    execute_editor_command_on_surface(session, command, EditorSurface::Body)
}

/// Executes a command using the host's active surface for generic commands
/// such as Delete and Select All. Explicit surface payloads remain authoritative.
pub fn execute_editor_command_on_surface(
    session: &mut EditingSession,
    command: &Command,
    active_surface: EditorSurface,
) -> CommandOutcome {
    if let Command::EditorElementChange {
        element_type,
        surface,
    } = command
    {
        if session
            .project()
            .element_definitions()
            .get(element_type.as_key())
            .is_none()
        {
            return CommandOutcome::Rejected(CommandRejection::InvalidElementTarget);
        }
        let core_selection = match surface {
            EditorSurface::Body => session.selection(),
            EditorSurface::TitlePage => session.title_selection(),
        };
        let selection = match crate::Selection::from_core(core_selection) {
            Ok(selection) => selection,
            Err(error) => {
                return CommandOutcome::Rejected(CommandRejection::PositionOutOfRange(error))
            }
        };
        if let Err(error) = selection.validate_for_surface(session.project(), *surface) {
            return CommandOutcome::Rejected(CommandRejection::InvalidSelection(error));
        }
        if selection.anchor.element != selection.focus.element {
            return CommandOutcome::Rejected(CommandRejection::InvalidSelection(
                SelectionError::CrossElementRange,
            ));
        }
    }

    if matches!(command, Command::EditUndo) && !session.can_undo() {
        return CommandOutcome::Unavailable(UnavailableReason::UndoHistory);
    }
    if matches!(command, Command::EditRedo) && !session.can_redo() {
        return CommandOutcome::Unavailable(UnavailableReason::RedoHistory);
    }
    if matches!(command, Command::EditSelectAll)
        && match active_surface {
            EditorSurface::Body => session.project().elements.is_empty(),
            EditorSurface::TitlePage => session.project().title_page.is_empty(),
        }
    {
        return CommandOutcome::Unavailable(UnavailableReason::Selection);
    }
    if matches!(command, Command::EditorDualDialogueToggle) {
        if active_surface != EditorSurface::Body {
            return CommandOutcome::Unavailable(UnavailableReason::DualDialogueTarget);
        }
    }

    if let Command::FormatTemplateApply { template } = command {
        return match session.apply_builtin_template(crate::builtin_template_id(*template)) {
            Ok(ApplyTemplateOutcome::Changed) => CommandOutcome::Changed,
            Ok(ApplyTemplateOutcome::NoOp) => CommandOutcome::NoOp,
            Err(error) => CommandOutcome::ApplyTemplateFailed(error),
        };
    }

    let before = session.mutation_token();
    let result = match command {
        Command::BeatBoardCardCreate { position } => session.create_beat_card(position.to_core()),
        Command::BeatBoardCardActivate { card } => session.activate_beat_card(card.as_core().clone()),
        Command::BeatBoardCardGeometrySet { card, rect, activate } => session.set_beat_card_geometry(card.as_core().clone(), rect.to_core(), *activate),
        Command::BeatBoardCardDelete { card } => session.delete_beat_card(card.as_core().clone()),
        Command::BeatBoardSelectionSet { selection } => session.set_beat_selection(match selection {
            value => match crate::beat_selection_to_core(value) {
                Ok(selection) => selection,
                Err(error) => return CommandOutcome::Rejected(CommandRejection::PositionOutOfRange(error)),
            },
        }),
        Command::BeatBoardTextEdit { operation } => {
            if !matches!(session.beat_selection(), BeatSelection::Text { .. }) { return CommandOutcome::Unavailable(UnavailableReason::Selection); }
            session.edit_beat_text(crate::beat_text_edit_to_core(operation))
        }
        Command::BeatBoardNavigation { target, extend } => {
            if !matches!(session.beat_selection(), BeatSelection::Text { .. }) { return CommandOutcome::Unavailable(UnavailableReason::Selection); }
            let (horizontal, edge) = crate::beat_navigation_to_core(*target);
            if let Some(edge) = edge { session.navigate_beat_line_edge(edge, *extend) }
            else { session.navigate_beat_horizontal(horizontal, *extend) }
        }
        Command::BeatBoardStyleToggle { style } => {
            if !session.beat_selection_is_valid() || !matches!(session.beat_selection(), BeatSelection::Text { field: BeatTextField::Body, .. }) {
                return CommandOutcome::Unavailable(UnavailableReason::Selection);
            }
            session.toggle_beat_run_style(style.as_core())
        }
        Command::BeatBoardStyleClear => {
            if !session.beat_selection_is_valid() || !matches!(session.beat_selection(), BeatSelection::Text { field: BeatTextField::Body, .. }) {
                return CommandOutcome::Unavailable(UnavailableReason::Selection);
            }
            session.clear_beat_run_styles()
        }
        Command::BeatBoardRunFormattingSet { patch } => {
            if !session.beat_selection_is_valid() || !matches!(session.beat_selection(), BeatSelection::Text { field: BeatTextField::Body, .. }) {
                return CommandOutcome::Unavailable(UnavailableReason::Selection);
            }
            session.set_beat_run_formatting(patch.as_core())
        }
        Command::ScriptNoteCreate { input } => {
            if input.validate().is_err() {
                return CommandOutcome::Rejected(CommandRejection::InvalidScriptNoteValue);
            }
            session.create_script_note(ScriptNoteCreationInput {
                color: input.color.and_then(|color| color.as_core()),
                note_type: input.note_type.clone(),
                timestamp: input.timestamp.as_str().to_owned(),
            }).map(|_| ())
        }
        Command::ScriptNoteDelete { note } => {
            if session.project().script_notes().get(note.as_core()).is_none() {
                return CommandOutcome::Unavailable(UnavailableReason::Selection);
            }
            session.delete_script_note(note.as_core().clone())
        }
        Command::ScriptNotePropertiesSet { properties } => {
            if properties.validate().is_err() {
                return CommandOutcome::Rejected(CommandRejection::InvalidScriptNoteValue);
            }
            if session.project().script_notes().get(properties.note.as_core()).is_none() {
                return CommandOutcome::Unavailable(UnavailableReason::Selection);
            }
            session.set_script_note_properties(
                properties.note.as_core().clone(), match properties.color {
                    None => ScriptNoteColorUpdate::Preserve,
                    Some(crate::ScriptNoteColorValue::ThemeDefault) => ScriptNoteColorUpdate::ThemeDefault,
                    Some(color) => match color.as_core() {
                        Some(color) => ScriptNoteColorUpdate::Explicit(color),
                        None => ScriptNoteColorUpdate::ThemeDefault,
                    },
                },
                properties.note_type.clone(), properties.modified.as_str().to_owned(),
            )
        }
        Command::ScriptNoteSelectionSet { selection } => {
            let selection = match crate::script_note_selection_to_core(selection) {
                Ok(selection) => selection,
                Err(error) => return CommandOutcome::Rejected(CommandRejection::PositionOutOfRange(error)),
            };
            if let ScriptNoteSelection::Text { note, .. } = &selection {
                if session.project().script_notes().get(note).is_none() {
                    return CommandOutcome::Unavailable(UnavailableReason::Selection);
                }
            }
            match session.set_script_note_selection(selection) {
                Err(EditError::InvalidSelection | EditError::InvalidUtf8Boundary) => {
                    return CommandOutcome::Unavailable(UnavailableReason::Selection);
                }
                result => result,
            }
        }
        Command::ScriptNoteTextEdit { edit } => {
            if edit.operation.validate().is_err() {
                return CommandOutcome::Rejected(CommandRejection::InvalidScriptNoteValue);
            }
            if !session.script_note_selection_is_valid()
                || !matches!(session.script_note_selection(), ScriptNoteSelection::Text { .. })
            {
                return CommandOutcome::Unavailable(UnavailableReason::Selection);
            }
            session.edit_script_note_text(
                crate::script_note_text_operation_to_core(&edit.operation),
                edit.modified.as_str().to_owned(),
            )
        }
        Command::ScriptNoteTextNavigate { target, extend } => {
            if !session.script_note_selection_is_valid()
                || !matches!(session.script_note_selection(), ScriptNoteSelection::Text { .. })
            {
                return CommandOutcome::Unavailable(UnavailableReason::Selection);
            }
            let (horizontal, edge) = crate::script_note_navigation_to_core(*target);
            if let Some(edge) = edge {
                session.navigate_script_note_line_edge(edge, *extend)
            } else {
                session.navigate_script_note_horizontal(horizontal, *extend)
            }
        }
        Command::ScriptNotePresent { .. } | Command::ScriptNoteDismiss => {
            return rejected(command);
        }
        Command::EditorTextInsert { text, surface } => insert(session, *surface, text, false),
        Command::EditorTextPaste { text, surface } => insert(session, *surface, text, true),
        Command::EditorDeleteBackward => delete_backward(session, active_surface),
        Command::EditDelete => delete_forward(session, active_surface),
        Command::EditorParagraphEnter => enter(session, active_surface),
        Command::EditorParagraphTab => tab(session, active_surface),
        Command::EditorDualDialogueToggle => session.toggle_dual_dialogue(),
        Command::EditorElementChange {
            element_type,
            surface,
        } => match surface {
            EditorSurface::Body => session.change_element_type(element_type.clone().into_key()),
            EditorSurface::TitlePage => {
                session.change_title_element_type(Some(element_type.clone().into_key()))
            }
        },
        Command::EditUndo => session.undo(),
        Command::EditRedo => session.redo(),
        Command::EditSelectAll => select_all(session, active_surface),
        Command::EditorSelectionSet { selection, surface } => {
            if let Err(error) = selection.validate_for_surface(session.project(), *surface) {
                return CommandOutcome::Rejected(CommandRejection::InvalidSelection(error));
            }
            match selection.to_core() {
                Ok(selection) => set_selection(session, *surface, selection),
                Err(error) => {
                    return CommandOutcome::Rejected(CommandRejection::PositionOutOfRange(error))
                }
            }
        }
        Command::EditorSelectionPointer {
            position,
            surface,
            phase,
        } => {
            let selection = crate::Selection {
                anchor: *position,
                focus: *position,
            };
            if let Err(error) = selection.validate_for_surface(session.project(), *surface) {
                return CommandOutcome::Rejected(CommandRejection::InvalidSelection(error));
            }
            let core_position = match position.to_core() {
                Ok(position) => position,
                Err(error) => {
                    return CommandOutcome::Rejected(CommandRejection::PositionOutOfRange(error))
                }
            };
            let current = match surface {
                EditorSurface::Body => session.selection(),
                EditorSurface::TitlePage => session.title_selection(),
            };
            let Some(selection) = crate::pointer_selection(
                session.project(),
                current,
                core_position,
                *surface,
                *phase,
            ) else {
                return CommandOutcome::Rejected(CommandRejection::InvalidSelection(
                    SelectionError::ElementOutOfRange,
                ));
            };
            set_selection(session, *surface, selection)
        }
        Command::EditorStyleToggle { surface, style } => match surface {
            EditorSurface::Body => session.toggle_body_run_style(style.as_core()),
            EditorSurface::TitlePage => session.toggle_title_run_style(style.as_core()),
        },
        Command::EditorStyleClear { surface } => match surface {
            EditorSurface::Body => session.clear_body_run_styles(),
            EditorSurface::TitlePage => session.clear_title_run_styles(),
        },
        Command::EditorAlignmentSet { surface, alignment } => match surface {
            EditorSurface::Body => session.set_body_alignment(alignment.as_core()),
            EditorSurface::TitlePage => session.set_title_alignment(alignment.as_core()),
        },
        Command::EditorRunFormattingSet { surface, patch } => {
            if *surface != EditorSurface::Body {
                return CommandOutcome::Rejected(CommandRejection::UnsupportedSurface);
            }
            if active_surface != EditorSurface::Body {
                return CommandOutcome::Unavailable(UnavailableReason::Selection);
            }
            session.set_body_run_formatting(patch.as_core())
        }
        Command::EditorParagraphFormattingSet { surface, patch } => {
            if *surface != EditorSurface::Body {
                return CommandOutcome::Rejected(CommandRejection::UnsupportedSurface);
            }
            if active_surface != EditorSurface::Body {
                return CommandOutcome::Unavailable(UnavailableReason::Selection);
            }
            session.set_body_paragraph_formatting(patch.as_core())
        }
        other => return rejected(other),
    };

    outcome_from_edit_result(result, session.mutation_token() != before)
}

fn outcome_from_edit_result(
    result: Result<(), EditError>,
    session_changed: bool,
) -> CommandOutcome {
    match result {
        Ok(()) if !session_changed => CommandOutcome::NoOp,
        Ok(()) => CommandOutcome::Changed,
        Err(error) => CommandOutcome::Failed(error),
    }
}

fn rejected(command: &Command) -> CommandOutcome {
    CommandOutcome::Rejected(CommandRejection::UnsupportedCommand(command.id()))
}

fn insert(
    session: &mut EditingSession,
    surface: EditorSurface,
    text: &str,
    paste: bool,
) -> Result<(), EditError> {
    match (surface, paste) {
        (EditorSurface::Body, false) => session.insert(text),
        (EditorSurface::Body, true) => session.insert_paste(text),
        (EditorSurface::TitlePage, false) => session.insert_title(text),
        (EditorSurface::TitlePage, true) => session.paste_title(text),
    }
}

fn delete_backward(session: &mut EditingSession, surface: EditorSurface) -> Result<(), EditError> {
    match surface {
        EditorSurface::Body => session.backspace(),
        EditorSurface::TitlePage => session.backspace_title(),
    }
}

fn delete_forward(session: &mut EditingSession, surface: EditorSurface) -> Result<(), EditError> {
    match surface {
        EditorSurface::Body => session.delete(),
        EditorSurface::TitlePage => session.delete_forward_title(),
    }
}

fn enter(session: &mut EditingSession, surface: EditorSurface) -> Result<(), EditError> {
    match surface {
        EditorSurface::Body => session.enter(),
        EditorSurface::TitlePage => session.enter_title(),
    }
}

fn tab(session: &mut EditingSession, surface: EditorSurface) -> Result<(), EditError> {
    match surface {
        EditorSurface::Body => session.tab(),
        EditorSurface::TitlePage => session.tab_title(),
    }
}

fn set_selection(
    session: &mut EditingSession,
    surface: EditorSurface,
    selection: CoreSelection,
) -> Result<(), EditError> {
    match surface {
        EditorSurface::Body => session.set_selection(selection),
        EditorSurface::TitlePage => session.set_title_selection(selection),
    }
}

fn select_all(session: &mut EditingSession, surface: EditorSurface) -> Result<(), EditError> {
    match surface {
        EditorSurface::Body => {
            let Some(last) = session.project().elements.last() else {
                return Err(EditError::InvalidSelection);
            };
            session.set_selection(CoreSelection {
                anchor: CorePosition {
                    element: 0,
                    offset: 0,
                },
                focus: CorePosition {
                    element: session.project().elements.len() - 1,
                    offset: last.text.len(),
                },
            })
        }
        EditorSurface::TitlePage => {
            let Some(last) = session.project().title_page.last() else {
                return Err(EditError::InvalidSelection);
            };
            session.set_title_selection(CoreSelection {
                anchor: CorePosition {
                    element: 0,
                    offset: 0,
                },
                focus: CorePosition {
                    element: session.project().title_page.len() - 1,
                    offset: last.text.len(),
                },
            })
        }
    }
}

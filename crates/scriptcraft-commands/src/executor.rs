//! Headless semantic execution over the shared transactional editing session.
//!
//! Native viewport and dialog intents are typed in the command vocabulary but
//! remain host effects; this executor never performs them.

use scriptcraft_core::editing::{EditError, EditingSession, Position as CorePosition, Selection as CoreSelection};

use crate::{
    Command, CommandId, EditorSurface, PointerPhase, PositionError, SelectionError,
};

/// Result of applying a semantic editor command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandOutcome {
    Changed,
    NoOp,
    Unavailable(UnavailableReason),
    Rejected(CommandRejection),
    Failed(EditError),
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
    InvalidSelection(SelectionError),
    PositionOutOfRange(PositionError),
}

/// Executes with the legacy Body target for generic surface-less commands.
pub fn execute_editor_command(
    session: &mut EditingSession,
    command: &Command,
) -> CommandOutcome {
    execute_editor_command_on_surface(session, command, EditorSurface::Body)
}

/// Executes a command using the host's active surface for generic commands
/// such as Delete and Select All. Explicit surface payloads remain authoritative.
pub fn execute_editor_command_on_surface(
    session: &mut EditingSession,
    command: &Command,
    active_surface: EditorSurface,
) -> CommandOutcome {
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

    let before = session.clone();
    let result = match command {
        Command::EditorTextInsert { text, surface } => insert(session, *surface, text, false),
        Command::EditorTextPaste { text, surface } => insert(session, *surface, text, true),
        Command::EditorDeleteBackward => delete_backward(session, active_surface),
        Command::EditDelete => delete_forward(session, active_surface),
        Command::EditorParagraphEnter => enter(session, active_surface),
        Command::EditorParagraphTab => tab(session, active_surface),
        Command::EditorDualDialogueToggle => session.toggle_dual_dialogue(),
        Command::EditorElementChange { role, surface } => match surface {
            EditorSurface::Body => session.change_role(role.as_core()),
            EditorSurface::TitlePage => {
                return rejected(command);
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
                Err(error) => return CommandOutcome::Rejected(CommandRejection::PositionOutOfRange(error)),
            }
        }
        Command::EditorSelectionPointer { position, surface, phase } => {
            let selection = crate::Selection { anchor: *position, focus: *position };
            if let Err(error) = selection.validate_for_surface(session.project(), *surface) {
                return CommandOutcome::Rejected(CommandRejection::InvalidSelection(error));
            }
            let core_position = match position.to_core() {
                Ok(position) => position,
                Err(error) => return CommandOutcome::Rejected(CommandRejection::PositionOutOfRange(error)),
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
        other => return rejected(other),
    };

    outcome_from_edit_result(result, *session != before)
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

fn insert(session: &mut EditingSession, surface: EditorSurface, text: &str, paste: bool) -> Result<(), EditError> {
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
                anchor: CorePosition { element: 0, offset: 0 },
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
                anchor: CorePosition { element: 0, offset: 0 },
                focus: CorePosition {
                    element: session.project().title_page.len() - 1,
                    offset: last.text.len(),
                },
            })
        }
    }
}

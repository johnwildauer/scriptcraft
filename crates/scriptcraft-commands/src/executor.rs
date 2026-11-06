//! Headless semantic execution over the real transactional editing session.
//!
//! This boundary intentionally accepts no desktop context and delegates every
//! supported operation to the existing session API. It does not implement
//! screenplay transitions or mutate project fields directly.

use scriptcraft_core::editing::{EditError, EditingSession, Selection as CoreSelection};

use crate::{Command, CommandId, PositionError, SelectionError};

/// Result of applying a semantic editor command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandOutcome {
    /// The existing session operation changed observable session state.
    Changed,
    /// The command was accepted, but the session operation made no change.
    NoOp,
    /// The command requires a history or selection state that is not available.
    Unavailable(UnavailableReason),
    /// The command or its parameter is not valid at this editor boundary.
    Rejected(CommandRejection),
    /// An existing core edit operation failed; the original error is retained.
    Failed(EditError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnavailableReason {
    UndoHistory,
    RedoHistory,
    Selection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRejection {
    UnsupportedCommand(CommandId),
    InvalidSelection(SelectionError),
    PositionOutOfRange(PositionError),
}

/// Executes one semantic editor command against an existing session.
///
/// Commands outside the headless editing subset are rejected without touching
/// the session. Selection input is validated before the core setter is called;
/// all other editor semantics remain owned by `EditingSession`.
pub fn execute_editor_command(
    session: &mut EditingSession,
    command: &Command,
) -> CommandOutcome {
    if let Command::EditUndo = command {
        if !session.can_undo() {
            return CommandOutcome::Unavailable(UnavailableReason::UndoHistory);
        }
    }
    if let Command::EditRedo = command {
        if !session.can_redo() {
            return CommandOutcome::Unavailable(UnavailableReason::RedoHistory);
        }
    }

    let before = session.clone();
    let result = match command {
        Command::EditorTextInsert { text } => session.insert(text),
        Command::EditorTextPaste { text } => session.insert_paste(text),
        Command::EditorDeleteBackward => session.backspace(),
        Command::EditDelete => session.delete(),
        Command::EditorParagraphEnter => session.enter(),
        Command::EditorParagraphTab => session.tab(),
        Command::EditorElementChange { role } => session.change_role(role.as_core()),
        Command::EditUndo => session.undo(),
        Command::EditRedo => session.redo(),
        Command::EditSelectAll => {
            let Some(last) = session.project().elements.last() else {
                return CommandOutcome::Unavailable(UnavailableReason::Selection);
            };
            session.set_selection(CoreSelection {
                anchor: scriptcraft_core::editing::Position {
                    element: 0,
                    offset: 0,
                },
                focus: scriptcraft_core::editing::Position {
                    element: session.project().elements.len() - 1,
                    offset: last.text.len(),
                },
            })
        }
        Command::EditorSelectionSet { selection } => {
            if let Err(error) = selection.validate_for(session.project()) {
                return CommandOutcome::Rejected(CommandRejection::InvalidSelection(error));
            }
            match selection.to_core() {
                Ok(selection) => session.set_selection(selection),
                Err(error) => {
                    return CommandOutcome::Rejected(CommandRejection::PositionOutOfRange(error));
                }
            }
        }
        other => {
            return CommandOutcome::Rejected(CommandRejection::UnsupportedCommand(other.id()));
        }
    };

    match result {
        Ok(()) if *session == before => CommandOutcome::NoOp,
        Ok(()) => CommandOutcome::Changed,
        Err(error) => CommandOutcome::Failed(error),
    }
}

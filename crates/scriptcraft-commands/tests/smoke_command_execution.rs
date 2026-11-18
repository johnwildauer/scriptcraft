#![forbid(unsafe_code)]

use scriptcraft_commands::{
    execute_editor_command, Command, CommandOutcome, CommandRejection, EditorSurface, Position,
    Selection, SelectionError,
};
use scriptcraft_session::editing::{
    EditingSession, Position as CorePosition, Selection as CoreSelection,
};
use scriptcraft_model::Project;

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("command smoke uses valid bounded input"))
}

#[test]
fn command_execution_public_api_smoke() {
    const INSERTED_TEXT: &str = "Command café";

    let mut session = valid(EditingSession::new(valid(Project::new())));
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorTextInsert {
                text: INSERTED_TEXT.to_owned(),
                surface: EditorSurface::Body,
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(session.project().elements[0].text, INSERTED_TEXT);
    assert_eq!(
        session.selection(),
        CoreSelection::caret(CorePosition {
            element: 0,
            offset: INSERTED_TEXT.len(),
        })
    );
    assert!(session.dirty());

    let before_rejection = session.clone();
    // The final é occupies two bytes; this points at its continuation byte.
    let invalid_offset = valid(u32::try_from(
        INSERTED_TEXT.len() - "é".len() + 1,
    ));
    let invalid_position = Position {
        element: 0,
        offset: invalid_offset,
    };
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorSelectionSet {
                selection: Selection {
                    anchor: invalid_position,
                    focus: invalid_position,
                },
                surface: EditorSurface::Body,
            },
        ),
        CommandOutcome::Rejected(CommandRejection::InvalidSelection(
            SelectionError::InvalidUtf8Boundary,
        ))
    );

    assert_eq!(session, before_rejection);
    assert_eq!(
        session.project().elements[0].text,
        before_rejection.project().elements[0].text
    );
    assert_eq!(session.selection(), before_rejection.selection());
    assert_eq!(session.revision(), before_rejection.revision());
    assert_eq!(session.dirty(), before_rejection.dirty());
    assert_eq!(session.can_undo(), before_rejection.can_undo());
    assert_eq!(session.can_redo(), before_rejection.can_redo());
}

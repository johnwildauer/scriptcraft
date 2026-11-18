#![forbid(unsafe_code)]

use scriptcraft_model::{Position, Project, Selection};
use scriptcraft_session::editing::EditingSession;

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("editing smoke uses valid bounded input"))
}

#[test]
fn editing_history_public_api_smoke() {
    const EDIT_TEXT: &str = "Café — répétition";

    let original = valid(Project::new());
    let original_selection = Selection::caret(Position { element: 0, offset: 0 });
    let mut session = valid(EditingSession::new(original.clone()));
    let initial_revision = session.revision();
    assert_eq!(session.selection(), original_selection);
    assert!(!session.dirty());
    assert!(!session.can_undo());
    assert!(!session.can_redo());

    let edited_selection = Selection::caret(Position {
        element: original_selection.focus.element,
        offset: original_selection.focus.offset + EDIT_TEXT.len(),
    });
    valid(session.insert(EDIT_TEXT));
    assert_eq!(session.project().elements[0].text, EDIT_TEXT);
    assert_eq!(session.selection(), edited_selection);
    let edited_revision = session.revision();
    assert_eq!(edited_revision, initial_revision + 1);
    assert!(session.dirty());
    assert!(session.can_undo());
    assert!(!session.can_redo());

    valid(session.undo());
    assert_eq!(session.project(), &original);
    assert_eq!(session.selection(), original_selection);
    assert!(!session.dirty());
    assert!(!session.can_undo());
    assert!(session.can_redo());
    let undone_revision = session.revision();
    assert_ne!(undone_revision, edited_revision);

    valid(session.redo());
    assert_eq!(session.project().elements[0].text, EDIT_TEXT);
    assert_eq!(session.selection(), edited_selection);
    assert!(session.dirty());
    assert!(session.can_undo());
    assert!(!session.can_redo());
    assert_eq!(session.revision(), undone_revision + 1);
}

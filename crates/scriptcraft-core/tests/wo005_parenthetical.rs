#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::persistence::{deserialize, serialize};
use scriptcraft_core::{Project, Role};

fn session(role: Role, text: &str) -> EditingSession {
    let mut project = Project::new("wo005", "Title", "Author")
        .unwrap_or_else(|_| unreachable!("fixed fixture must be valid"));
    project.elements[0].role = role;
    project.elements[0].text = text.into();
    EditingSession::new(project).unwrap_or_else(|_| unreachable!("fixed fixture must be valid"))
}

fn caret(session: &mut EditingSession, offset: usize) {
    session
        .set_selection(Selection::caret(Position { element: 0, offset }))
        .unwrap_or_else(|_| unreachable!("fixture caret must be valid"));
}

#[test]
fn tab_enters_parenthetical_with_interior_caret_and_tab_leaves_empty_generated_pair() {
    let mut session = session(Role::Dialogue, "");
    session.tab().unwrap_or_else(|_| unreachable!());
    assert_eq!(session.project().elements[0].role, Role::Parenthetical);
    assert_eq!(session.project().elements[0].text, "()");
    assert_eq!(session.selection().focus.offset, 1);

    session.tab().unwrap_or_else(|_| unreachable!());
    assert_eq!(session.project().elements[0].role, Role::Dialogue);
    assert_eq!(session.project().elements[0].text, "");
    assert_eq!(session.selection().focus.offset, 0);
}

#[test]
fn explicit_role_assignment_generates_pair_and_enter_removes_only_generated_pair() {
    let mut session = session(Role::Action, "");
    session
        .change_role(Role::Parenthetical)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(session.project().elements[0].text, "()");
    assert_eq!(session.selection().focus.offset, 1);

    session.enter().unwrap_or_else(|_| unreachable!());
    assert_eq!(session.project().elements[0].role, Role::Parenthetical);
    assert_eq!(session.project().elements[0].text, "");
    assert_eq!(session.project().elements[1].role, Role::Dialogue);
    assert_eq!(session.project().elements[1].text, "");
    assert_eq!(
        session.selection().focus,
        Position {
            element: 1,
            offset: 0
        }
    );
}

#[test]
fn generated_pair_preserves_content_and_partial_edits() {
    let mut typed = session(Role::Dialogue, "");
    typed.tab().unwrap_or_else(|_| unreachable!());
    typed.insert("hello").unwrap_or_else(|_| unreachable!());
    assert_eq!(typed.project().elements[0].text, "(hello)");
    typed.tab().unwrap_or_else(|_| unreachable!());
    assert_eq!(typed.project().elements[0].text, "(hello)");

    for partial in ["(", ")", "(x", "x)"] {
        let mut partial_session = session(Role::Parenthetical, partial);
        caret(&mut partial_session, partial.len());
        partial_session.enter().unwrap_or_else(|_| unreachable!());
        let expected = match partial {
            "(x" => "(x)",
            "x)" => "(x)",
            _ => partial,
        };
        assert_eq!(partial_session.project().elements[0].text, expected);
    }
}

#[test]
fn editing_away_and_reconstructing_generated_pair_makes_it_user_authored() {
    let mut session = session(Role::Dialogue, "");
    session.tab().unwrap_or_else(|_| unreachable!());

    session.delete().unwrap_or_else(|_| unreachable!());
    assert_eq!(session.project().elements[0].role, Role::Dialogue);
    assert_eq!(session.project().elements[0].text, "");
    session.insert(")").unwrap_or_else(|_| unreachable!());
    assert_eq!(session.project().elements[0].text, ")");

    session.enter().unwrap_or_else(|_| unreachable!());
    assert_eq!(session.project().elements[0].text, ")");
    assert_eq!(session.project().elements[1].role, Role::Action);
}

#[test]
fn user_authored_and_pasted_pairs_are_not_marked_for_cleanup() {
    for pasted in [false, true] {
        let mut session = session(Role::Parenthetical, "");
        if pasted {
            session
                .insert_paste("()")
                .unwrap_or_else(|_| unreachable!());
        } else {
            session.insert("()").unwrap_or_else(|_| unreachable!());
        }
        caret(&mut session, 2);
        session.enter().unwrap_or_else(|_| unreachable!());
        assert_eq!(session.project().elements[0].text, "()");
    }
}

#[test]
fn generated_provenance_is_undoable_and_not_persisted() {
    let mut editing = session(Role::Action, "");
    editing
        .change_role(Role::Parenthetical)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(editing.undo(), Ok(()));
    assert_eq!(editing.project().elements[0].role, Role::Action);
    assert_eq!(editing.redo(), Ok(()));
    assert_eq!(editing.project().elements[0].text, "()");

    editing.enter().unwrap_or_else(|_| unreachable!());
    assert_eq!(editing.project().elements[0].text, "");
    assert_eq!(editing.undo(), Ok(()));
    assert_eq!(editing.project().elements[0].text, "()");
    assert_eq!(editing.redo(), Ok(()));
    assert_eq!(editing.project().elements[0].text, "");

    let mut saved = session(Role::Parenthetical, "");
    saved.insert("()").unwrap_or_else(|_| unreachable!());
    let bytes = serialize(saved.project()).unwrap_or_else(|_| unreachable!());
    assert!(!String::from_utf8_lossy(&bytes).contains("generated"));
    let reopened = deserialize(&bytes).unwrap_or_else(|_| unreachable!());
    let mut reopened_session = EditingSession::new(reopened).unwrap_or_else(|_| unreachable!());
    caret(&mut reopened_session, 2);
    reopened_session.enter().unwrap_or_else(|_| unreachable!());
    assert_eq!(reopened_session.project().elements[0].text, "()");
}

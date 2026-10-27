#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::{EditError, EditingSession, Position, Selection};
use scriptcraft_core::layout::ScreenplayLayout;
use scriptcraft_core::persistence::serialize;
use scriptcraft_core::{Project, Role, MAX_TEXT_BYTES};

fn session() -> EditingSession {
    let mut project = Project::new("paste-fixture", "Paste fixture", "Writer")
        .unwrap_or_else(|_| unreachable!("fixture project must be valid"));
    project.elements[0].role = Role::Action;
    EditingSession::new(project).unwrap_or_else(|_| unreachable!("fixture session must be valid"))
}

#[test]
fn paste_matrix_preserves_one_element_and_remains_layout_and_save_safe() {
    let fixtures = [
        " ",
        "a  b   c",
        "a\tb",
        "one\ntwo\n\nthree",
        "one\r\ntwo\rthree",
        "café🙂 — 東京",
    ];

    for fixture in fixtures {
        let mut session = session();
        session
            .insert_paste(fixture)
            .unwrap_or_else(|_| unreachable!("bounded fixture paste must succeed"));
        assert_eq!(session.project().elements.len(), 1);
        assert_eq!(session.project().elements[0].text, fixture);
        assert!(ScreenplayLayout::build(session.project()).is_ok());
        assert!(serialize(session.project()).is_ok());
        assert!(session.can_undo());
        session
            .undo()
            .unwrap_or_else(|_| unreachable!("paste is one undo step"));
        assert_eq!(session.project().elements[0].text, "");
    }
}

#[test]
fn large_bounded_paste_is_valid_and_oversized_paste_rolls_back() {
    let mut session = session();
    let large = "x".repeat(MAX_TEXT_BYTES - 32);
    session
        .insert_paste(&large)
        .unwrap_or_else(|_| unreachable!("large bounded paste must succeed"));
    assert!(ScreenplayLayout::build(session.project()).is_ok());
    assert!(serialize(session.project()).is_ok());

    session
        .set_selection(Selection::caret(Position {
            element: 0,
            offset: session.project().elements[0].text.len(),
        }))
        .unwrap_or_else(|_| unreachable!("end boundary must be valid"));
    let before = session.clone();
    assert_eq!(
        session.insert_paste("x".repeat(64).as_str()),
        Err(EditError::TextTooLong)
    );
    assert_eq!(session, before);
    session
        .insert_paste("ok")
        .unwrap_or_else(|_| unreachable!("session remains editable after rejection"));
    assert!(session.project().elements[0].text.ends_with("ok"));
}

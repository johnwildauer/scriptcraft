#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::{EditError, EditingSession, Position, Selection};
use scriptcraft_core::persistence::{deserialize, serialize};
use scriptcraft_core::{Project, Role, ScreenplayElement, MAX_ELEMENTS};

fn project(text: &str) -> Project {
    let mut project = Project::from_json(include_str!("fixtures/legacy_project.json"))
        .unwrap_or_else(|_| unreachable!("fixed fixture must be valid"));
    project.elements[0].role = Role::Dialogue;
    project.elements[0].text = text.into();
    project
}

fn at(session: &mut EditingSession, element: usize, offset: usize) {
    session
        .set_selection(Selection::caret(Position { element, offset }))
        .unwrap_or_else(|_| unreachable!("fixture position must be valid"));
}

#[test]
fn dialogue_end_and_empty_enter_keep_action_transition() {
    for text in ["spoken", ""] {
        let mut session = EditingSession::new(project(text))
            .unwrap_or_else(|_| unreachable!("fixed fixture must be valid"));
        let original_id = session.project().elements[0].id.clone();
        at(&mut session, 0, text.len());
        assert_eq!(session.enter(), Ok(()));
        assert_eq!(session.project().elements[0].role, Role::Dialogue);
        assert_eq!(session.project().elements[0].text, text);
        assert_eq!(session.project().elements[0].id, original_id);
        assert_eq!(session.project().elements[1].role, Role::Action);
        assert_eq!(session.project().elements[1].text, "");
        assert_eq!(
            session.selection(),
            Selection::caret(Position {
                element: 1,
                offset: 0
            })
        );
        assert_eq!(session.revision(), 1);
        assert!(session.dirty());
        assert!(session.can_undo());
    }
}

#[test]
fn interior_ascii_and_unicode_enter_split_dialogue_exactly_once() {
    for (text, offset, prefix, suffix) in [
        ("hello world", 5, "hello", " world"),
        ("前🙂後", "前".len(), "前", "🙂後"),
    ] {
        let mut session = EditingSession::new(project(text))
            .unwrap_or_else(|_| unreachable!("fixed fixture must be valid"));
        let original_id = session.project().elements[0].id.clone();
        at(&mut session, 0, offset);
        assert_eq!(session.enter(), Ok(()));
        assert_eq!(session.project().elements.len(), 2);
        assert_eq!(session.project().elements[0].id, original_id);
        assert_ne!(
            session.project().elements[0].id,
            session.project().elements[1].id
        );
        assert!(session.project().elements[1].id.as_str().len() <= 128);
        assert_eq!(session.project().elements[0].role, Role::Dialogue);
        assert_eq!(session.project().elements[1].role, Role::Dialogue);
        assert_eq!(session.project().elements[0].text, prefix);
        assert_eq!(session.project().elements[1].text, suffix);
        assert_eq!(
            session.selection(),
            Selection::caret(Position {
                element: 1,
                offset: 0
            })
        );
        assert_eq!(session.revision(), 1);
        assert!(session.dirty());

        let split = session.project().clone();
        let split_selection = session.selection();
        assert_eq!(session.undo(), Ok(()));
        assert_eq!(session.project(), &project(text));
        assert_eq!(
            session.selection(),
            Selection::caret(Position { element: 0, offset })
        );
        assert_eq!(session.revision(), 2);
        assert!(!session.dirty());
        assert_eq!(session.redo(), Ok(()));
        assert_eq!(session.project(), &split);
        assert_eq!(session.selection(), split_selection);
        assert_eq!(session.revision(), 3);
    }
}

#[test]
fn repeated_interior_splits_preserve_order_and_stable_ids() {
    let mut session = EditingSession::new(project("abcdef"))
        .unwrap_or_else(|_| unreachable!("fixed fixture must be valid"));
    for expected in ["a", "b", "c", "d", "e"] {
        let element = session.selection().focus.element;
        at(&mut session, element, 1);
        session
            .enter()
            .unwrap_or_else(|_| unreachable!("split must succeed"));
        assert_eq!(session.project().elements[element].text, expected);
        assert_eq!(session.project().elements[element].role, Role::Dialogue);
        assert_eq!(session.project().elements[element + 1].role, Role::Dialogue);
    }
    assert_eq!(
        session
            .project()
            .elements
            .iter()
            .map(|e| e.text.as_str())
            .collect::<Vec<_>>(),
        ["a", "b", "c", "d", "e", "f"]
    );
    assert!(session.validate().is_ok());
}

#[test]
fn rejected_dialogue_enters_leave_every_session_observable_unchanged() {
    let mut non_collapsed = EditingSession::new(project("hello"))
        .unwrap_or_else(|_| unreachable!("fixed fixture must be valid"));
    non_collapsed
        .set_selection(Selection {
            anchor: Position {
                element: 0,
                offset: 1,
            },
            focus: Position {
                element: 0,
                offset: 3,
            },
        })
        .unwrap_or_else(|_| unreachable!("fixture selection must be valid"));
    let before = non_collapsed.clone();
    assert_eq!(non_collapsed.enter(), Err(EditError::InvalidSelection));
    assert_eq!(non_collapsed, before);

    let mut invalid_boundary = EditingSession::new(project("🙂"))
        .unwrap_or_else(|_| unreachable!("fixed fixture must be valid"));
    let before = invalid_boundary.clone();
    assert_eq!(
        invalid_boundary.set_selection(Selection::caret(Position {
            element: 0,
            offset: 1
        })),
        Err(EditError::InvalidUtf8Boundary)
    );
    assert_eq!(invalid_boundary, before);

    let mut full = project("x");
    for index in 2..=MAX_ELEMENTS {
        full.elements.push(
            ScreenplayElement::new(format!("element-{index}"), Role::Action, "")
                .unwrap_or_else(|_| unreachable!("bounded fixture element must be valid")),
        );
    }
    let mut at_capacity =
        EditingSession::new(full).unwrap_or_else(|_| unreachable!("maximum fixture must be valid"));
    at(&mut at_capacity, 0, 0);
    let before = at_capacity.clone();
    assert_eq!(at_capacity.enter(), Err(EditError::TooManyElements));
    assert_eq!(at_capacity, before);
}

#[test]
fn dialogue_split_save_reopen_preserves_roles_ids_and_unicode_bytes() {
    let mut session = EditingSession::new(project("左🙂右"))
        .unwrap_or_else(|_| unreachable!("fixed fixture must be valid"));
    at(&mut session, 0, "左".len());
    session
        .enter()
        .unwrap_or_else(|_| unreachable!("split must succeed"));
    let saved = session.project().clone();
    let bytes = serialize(&saved).unwrap_or_else(|_| unreachable!("save must succeed"));
    let reopened = deserialize(&bytes).unwrap_or_else(|_| unreachable!("reopen must succeed"));
    assert_eq!(reopened, saved);
    assert_eq!(reopened.elements[0].text, "左");
    assert_eq!(reopened.elements[1].text, "🙂右");
    assert_eq!(reopened.elements[0].role, Role::Dialogue);
    assert_eq!(reopened.elements[1].role, Role::Dialogue);
}

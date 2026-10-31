#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::{EditPolicy, EditingSession, Position, Selection};
use scriptcraft_core::layout::LayoutCache;
use scriptcraft_core::{ModelError, Project, Role};

fn session(role: Role, text: &str) -> EditingSession {
    let mut project = Project::from_json(include_str!("fixtures/legacy_project.json"))
        .unwrap_or_else(|_| unreachable!("fixture must be valid"));
    project.elements[0].role = role;
    project.elements[0].text = text.into();
    EditingSession::new(project).unwrap_or_else(|_| unreachable!("fixture must be valid"))
}

fn caret(session: &mut EditingSession, element: usize, offset: usize) {
    session
        .set_selection(Selection::caret(Position { element, offset }))
        .unwrap_or_else(|_| unreachable!("fixture caret must be valid"));
}

#[test]
fn settings_are_defaulted_typed_and_round_trip_without_schema_bump() {
    let project = Project::from_json(include_str!("fixtures/legacy_project.json"))
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(
        EditingSession::new(project.clone())
            .unwrap_or_else(|_| unreachable!())
            .policy(),
        EditPolicy::default()
    );
    let json = project.to_json().unwrap_or_else(|_| unreachable!());
    let legacy = json
        .replace(",\"auto_parenthetical\":true", "")
        .replace(",\"strict_transition_boundaries\":true", "");
    let restored = Project::from_json(&legacy).unwrap_or_else(|_| unreachable!());
    assert_eq!(
        EditingSession::new(restored)
            .unwrap_or_else(|_| unreachable!())
            .policy(),
        EditPolicy::default()
    );
    assert_eq!(
        Project::from_json(
            &json.replace("\"auto_parenthetical\":true", "\"auto_parenthetical\":1")
        ),
        Err(ModelError::WrongType("auto_parenthetical"))
    );
    assert_eq!(
        Project::from_json(&json.replace(
            "\"strict_transition_boundaries\":true",
            "\"strict_transition_boundaries\":1"
        )),
        Err(ModelError::WrongType("strict_transition_boundaries"))
    );
    for key in [
        "uppercase_cues",
        "auto_parenthetical",
        "strict_transition_boundaries",
    ] {
        assert_eq!(
            Project::from_json(
                &json.replace(&format!("\"{key}\":true"), &format!("\"{key}\":false"))
            ),
            Err(ModelError::UnsupportedJsonState)
        );
    }
    let mut cache = LayoutCache::default();
    let _ = cache
        .get_or_build(0, &project)
        .unwrap_or_else(|_| unreachable!());
    let (_, hit) = cache
        .get_or_build(0, &project)
        .unwrap_or_else(|_| unreachable!());
    assert!(hit);
    let mut changed_layout = project.clone();
    changed_layout.page_profile.font_size += 1;
    let (_, hit) = cache
        .get_or_build(0, &changed_layout)
        .unwrap_or_else(|_| unreachable!());
    assert!(!hit);
}

#[test]
fn policy_changes_are_transient_non_dirty_and_not_part_of_undo_history() {
    let mut editing = session(Role::Character, "ABCD");
    caret(&mut editing, 0, 2);
    let original_project = editing.project().clone();
    let original_revision = editing.revision();
    let runtime_policy = EditPolicy {
        uppercase_cues: false,
        auto_parenthetical: false,
        strict_transition_boundaries: false,
    };

    editing.set_policy(runtime_policy);
    assert_eq!(editing.policy(), runtime_policy);
    assert_eq!(editing.project(), &original_project);
    assert_eq!(editing.revision(), original_revision);
    assert!(!editing.dirty());
    assert!(!editing.can_undo());

    // The session override does not ride along in project JSON: the transitional
    // adapter emits its legacy defaults and a new session receives those defaults.
    let json = editing
        .project()
        .to_json()
        .unwrap_or_else(|_| unreachable!());
    let reopened = Project::from_json(&json).unwrap_or_else(|_| unreachable!());
    assert_eq!(reopened.elements[0].text, "ABCD");
    assert_eq!(
        EditingSession::new(reopened)
            .unwrap_or_else(|_| unreachable!())
            .policy(),
        EditPolicy::default()
    );

    editing.enter().unwrap_or_else(|_| unreachable!());
    assert_eq!(editing.project().elements.len(), 2);
    assert_eq!(editing.project().elements[0].text, "AB");
    assert_eq!(editing.project().elements[1].role, Role::Dialogue);
    assert_eq!(editing.project().elements[1].text, "CD");
    editing.undo().unwrap_or_else(|_| unreachable!());
    assert_eq!(editing.project(), &original_project);
    assert_eq!(editing.policy(), runtime_policy);
    editing.redo().unwrap_or_else(|_| unreachable!());
    assert_eq!(editing.policy(), runtime_policy);
}

#[test]
fn dialogue_insertion_and_setting_off_precedence_are_exact() {
    let mut on = session(Role::Dialogue, "");
    on.tab().unwrap_or_else(|_| unreachable!());
    assert_eq!(on.project().elements[0].text, "()");
    assert_eq!(on.selection().focus.offset, 1);
    let mut off = session(Role::Dialogue, "");
    off.set_policy(EditPolicy {
        auto_parenthetical: false,
        ..off.policy()
    });
    off.tab().unwrap_or_else(|_| unreachable!());
    assert_eq!(off.project().elements[0].role, Role::Parenthetical);
    assert_eq!(off.project().elements[0].text, "");

    let mut assigned = session(Role::Action, "");
    assigned.set_policy(EditPolicy {
        auto_parenthetical: false,
        ..assigned.policy()
    });
    assigned
        .change_role(Role::Parenthetical)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(assigned.project().elements[0].role, Role::Parenthetical);
    assert_eq!(assigned.project().elements[0].text, "");
}

#[test]
fn empty_delete_matrix_and_noncollapsed_delegation() {
    let mut project = Project::new().unwrap_or_else(|_| unreachable!());
    project.elements[0].role = Role::Action;
    project.elements[0].text = "prior".into();
    project.elements.push(
        scriptcraft_core::ScreenplayElement::new("e2", Role::Parenthetical, "()")
            .unwrap_or_else(|_| unreachable!()),
    );
    let mut with_previous = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
    caret(&mut with_previous, 1, 1);
    with_previous.delete().unwrap_or_else(|_| unreachable!());
    assert_eq!(with_previous.project().elements.len(), 1);
    assert_eq!(
        with_previous.selection().focus,
        Position {
            element: 0,
            offset: 5
        }
    );

    let mut first = session(Role::Parenthetical, "()");
    caret(&mut first, 0, 0);
    first.delete().unwrap_or_else(|_| unreachable!());
    assert_eq!(first.project().elements[0].role, Role::Dialogue);
    assert_eq!(first.selection().focus.offset, 0);

    let mut after = session(Role::Parenthetical, "()");
    caret(&mut after, 0, 2);
    after.delete().unwrap_or_else(|_| unreachable!());
    assert_eq!(after.project().elements[0].role, Role::Dialogue);
    assert_eq!(after.project().elements[0].text, "");
    assert_eq!(after.selection().focus.offset, 0);

    let mut selected = session(Role::Parenthetical, "()");
    selected
        .set_selection(Selection {
            anchor: Position {
                element: 0,
                offset: 0,
            },
            focus: Position {
                element: 0,
                offset: 1,
            },
        })
        .unwrap_or_else(|_| unreachable!());
    selected.delete().unwrap_or_else(|_| unreachable!());
    assert_eq!(selected.project().elements[0].text, ")");
}

#[test]
fn delimiter_only_parentheticals_delete_from_every_caret_position() {
    for text in ["(", "()", ")"] {
        for offset in 0..=text.len() {
            let mut project = Project::new().unwrap_or_else(|_| unreachable!());
            project.elements[0].role = Role::Action;
            project.elements[0].text = "prior".into();
            project.elements.push(
                scriptcraft_core::ScreenplayElement::new("e2", Role::Parenthetical, text)
                    .unwrap_or_else(|_| unreachable!()),
            );
            let mut session = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
            caret(&mut session, 1, offset);
            session.delete().unwrap_or_else(|_| unreachable!());
            assert_eq!(session.project().elements.len(), 1);
            assert_eq!(
                session.selection().focus,
                Position {
                    element: 0,
                    offset: 5
                }
            );
        }
    }
}

#[test]
fn delimiter_only_delete_has_first_only_and_predecessor_fallbacks_with_transactional_history() {
    for text in ["(", "()", ")"] {
        for offset in 0..=text.len() {
            let mut first_project = Project::new().unwrap_or_else(|_| unreachable!());
            first_project.elements[0].role = Role::Parenthetical;
            first_project.elements[0].text = text.into();
            first_project.elements.push(
                scriptcraft_core::ScreenplayElement::new("e2", Role::Action, "next")
                    .unwrap_or_else(|_| unreachable!()),
            );
            let mut first = EditingSession::new(first_project).unwrap_or_else(|_| unreachable!());
            caret(&mut first, 0, offset);
            let first_before = first.clone();
            first.delete().unwrap_or_else(|_| unreachable!());
            assert_eq!(first.project().elements.len(), 2);
            assert_eq!(first.project().elements[0].role, Role::Dialogue);
            assert_eq!(first.project().elements[0].text, "");
            assert_eq!(
                first.selection().focus,
                Position {
                    element: 0,
                    offset: 0
                }
            );
            assert!(first.undo().is_ok());
            assert_eq!(first.project(), first_before.project());
            assert_eq!(first.selection(), first_before.selection());
            assert!(first.redo().is_ok());
            assert_eq!(first.project().elements[0].role, Role::Dialogue);

            let mut only = session(Role::Parenthetical, text);
            caret(&mut only, 0, offset);
            let only_before = only.clone();
            only.delete().unwrap_or_else(|_| unreachable!());
            assert_eq!(only.project().elements.len(), 1);
            assert_eq!(
                only.selection().focus,
                Position {
                    element: 0,
                    offset: 0
                }
            );
            assert!(only.undo().is_ok());
            assert_eq!(only.project(), only_before.project());
            assert_eq!(only.selection(), only_before.selection());
            assert!(only.redo().is_ok());

            let mut project = Project::new().unwrap_or_else(|_| unreachable!());
            project.elements[0].role = Role::Action;
            project.elements[0].text = "prior".into();
            project.elements.push(
                scriptcraft_core::ScreenplayElement::new("e2", Role::Parenthetical, text)
                    .unwrap_or_else(|_| unreachable!()),
            );
            let mut with_predecessor =
                EditingSession::new(project).unwrap_or_else(|_| unreachable!());
            caret(&mut with_predecessor, 1, offset);
            let before = with_predecessor.clone();
            with_predecessor.delete().unwrap_or_else(|_| unreachable!());
            assert_eq!(with_predecessor.project().elements.len(), 1);
            assert_eq!(with_predecessor.project().elements[0].text, "prior");
            assert_eq!(
                with_predecessor.selection().focus,
                Position {
                    element: 0,
                    offset: 5
                }
            );
            assert!(with_predecessor.undo().is_ok());
            assert_eq!(with_predecessor.project(), before.project());
            assert_eq!(with_predecessor.selection(), before.selection());
            assert!(with_predecessor.redo().is_ok());
        }
    }
}

#[test]
fn delimiter_only_parentheticals_tab_to_empty_dialogue_from_every_caret_position() {
    for text in ["(", "()", ")"] {
        for offset in 0..=text.len() {
            let mut session = session(Role::Parenthetical, text);
            let element_id = session.project().elements[0].id.clone();
            caret(&mut session, 0, offset);
            session.tab().unwrap_or_else(|_| unreachable!());
            assert_eq!(session.project().elements.len(), 1);
            assert_eq!(session.project().elements[0].id, element_id);
            assert_eq!(session.project().elements[0].role, Role::Dialogue);
            assert_eq!(session.project().elements[0].text, "");
            assert_eq!(
                session.selection().focus,
                Position {
                    element: 0,
                    offset: 0
                }
            );
            assert!(session.can_undo());
            let changed = session.clone();
            session.undo().unwrap_or_else(|_| unreachable!());
            assert_eq!(session.project().elements[0].role, Role::Parenthetical);
            assert_eq!(session.project().elements[0].text, text);
            session.redo().unwrap_or_else(|_| unreachable!());
            assert_eq!(session.project(), changed.project());
            assert_eq!(session.selection(), changed.selection());
        }
    }
}

#[test]
fn supported_repairs_and_strict_boundaries() {
    for (text, expected) in [("(text", "(text)"), ("text)", "(text)"), ("text", "(text)")] {
        let mut s = session(Role::Parenthetical, text);
        caret(&mut s, 0, text.len());
        s.tab().unwrap_or_else(|_| unreachable!());
        assert_eq!(s.project().elements[0].text, expected);
        assert_eq!(s.project().elements.len(), 2);
        assert_eq!(
            s.selection().focus,
            Position {
                element: 1,
                offset: 0
            }
        );
    }
    for role in [Role::SceneHeading, Role::Character, Role::Transition] {
        let mut s = session(role, "éx");
        caret(&mut s, 0, 2);
        let before = s.clone();
        s.enter().unwrap_or_else(|_| unreachable!());
        assert_eq!(s, before);
        s.tab().unwrap_or_else(|_| unreachable!());
        assert_eq!(s, before);
        caret(&mut s, 0, 3);
        s.enter().unwrap_or_else(|_| unreachable!());
        assert_eq!(s.project().elements.len(), 2);
    }
}

#[test]
fn closed_pair_inner_end_and_missing_close_both_advance_with_tab_and_enter() {
    for command in ["tab", "enter"] {
        let mut closed = session(Role::Parenthetical, "(words)");
        caret(&mut closed, 0, "(words".len());
        if command == "tab" {
            closed.tab().unwrap_or_else(|_| unreachable!());
        } else {
            closed.enter().unwrap_or_else(|_| unreachable!());
        }
        assert_eq!(closed.project().elements[0].text, "(words)");
        assert_eq!(closed.project().elements[1].role, Role::Dialogue);
        assert_eq!(
            closed.selection().focus,
            Position {
                element: 1,
                offset: 0
            }
        );
        assert!(closed.undo().is_ok());
        assert_eq!(closed.project().elements.len(), 1);
        assert!(closed.redo().is_ok());
        assert_eq!(closed.project().elements.len(), 2);

        let mut missing = session(Role::Parenthetical, "(words");
        caret(&mut missing, 0, "(words".len());
        if command == "tab" {
            missing.tab().unwrap_or_else(|_| unreachable!());
        } else {
            missing.enter().unwrap_or_else(|_| unreachable!());
        }
        assert_eq!(missing.project().elements[0].text, "(words)");
        assert_eq!(missing.project().elements[1].role, Role::Dialogue);
        assert_eq!(
            missing.selection().focus,
            Position {
                element: 1,
                offset: 0
            }
        );
    }
}

#[test]
fn closed_pair_inner_end_is_utf8_safe_and_interior_remains_no_op() {
    let text = "(éx)";
    let mut at_inner_end = session(Role::Parenthetical, text);
    caret(&mut at_inner_end, 0, "(éx".len());
    at_inner_end.tab().unwrap_or_else(|_| unreachable!());
    assert_eq!(at_inner_end.project().elements[0].text, text);
    assert_eq!(at_inner_end.project().elements.len(), 2);

    let mut interior = session(Role::Parenthetical, text);
    caret(&mut interior, 0, "(é".len());
    let before = interior.clone();
    interior.enter().unwrap_or_else(|_| unreachable!());
    assert_eq!(interior, before);
    assert!(!interior.can_undo());
}

#[test]
fn before_opening_enter_uses_setting_and_missing_opening_does_not_trigger() {
    let mut on = session(Role::Parenthetical, "(x)");
    caret(&mut on, 0, 0);
    on.enter().unwrap_or_else(|_| unreachable!());
    assert_eq!(on.project().elements[0].text, "()");
    assert_eq!(on.selection().focus.offset, 1);
    assert_eq!(on.project().elements[1].text, "(x)");

    let mut off = session(Role::Parenthetical, "(x)");
    off.set_policy(EditPolicy {
        auto_parenthetical: false,
        ..off.policy()
    });
    off.enter().unwrap_or_else(|_| unreachable!());
    assert_eq!(off.project().elements[0].text, "");
    assert_eq!(off.project().elements[1].text, "(x)");
}

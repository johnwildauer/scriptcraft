#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::{EditError, EditPolicy, EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::formatting::{formatting_for_role, formatting_in_native_order};
use scriptcraft_core::{Project, Role, ScreenplayElement, MAX_ELEMENTS};

const ENTER_MATRIX: [(Role, Role); 17] = [
    (Role::General, Role::General),
    (Role::SceneHeading, Role::Action),
    (Role::Action, Role::Action),
    (Role::Character, Role::Dialogue),
    (Role::Parenthetical, Role::Dialogue),
    (Role::Dialogue, Role::Action),
    (Role::Transition, Role::SceneHeading),
    (Role::Shot, Role::Action),
    (Role::CastList, Role::Action),
    (Role::NewAct, Role::SceneHeading),
    (Role::Sequence, Role::SceneHeading),
    (Role::EndOfAct, Role::NewAct),
    (Role::Summary, Role::Summary),
    (Role::Outline1, Role::Summary),
    (Role::Outline2, Role::Summary),
    (Role::Outline3, Role::Summary),
    (Role::Note, Role::Summary),
];

const TAB_MATRIX: [(Role, Role); 17] = [
    (Role::General, Role::General),
    (Role::SceneHeading, Role::Action),
    (Role::Action, Role::Character),
    (Role::Character, Role::Transition),
    (Role::Parenthetical, Role::Dialogue),
    (Role::Dialogue, Role::Parenthetical),
    (Role::Transition, Role::SceneHeading),
    (Role::Shot, Role::Action),
    (Role::CastList, Role::Action),
    (Role::NewAct, Role::SceneHeading),
    (Role::Sequence, Role::SceneHeading),
    (Role::EndOfAct, Role::NewAct),
    (Role::Summary, Role::Outline1),
    (Role::Outline1, Role::Outline2),
    (Role::Outline2, Role::Outline3),
    (Role::Outline3, Role::Note),
    (Role::Note, Role::Summary),
];

fn session(role: Role, text: &str) -> EditingSession {
    let mut project = Project::new().unwrap_or_else(|_| unreachable!("fixture must be valid"));
    project.elements[0].role = role;
    project.elements[0].text = text.into();
    EditingSession::new(project).unwrap_or_else(|_| unreachable!("fixture must be valid"))
}

fn caret(session: &mut EditingSession, element: usize, offset: usize) {
    session
        .set_selection(Selection::caret(Position { element, offset }))
        .unwrap_or_else(|_| unreachable!("fixture caret must be valid"));
}

fn role_for_label(label: &str) -> Option<Role> {
    formatting_in_native_order()
        .iter()
        .find(|defaults| defaults.fdx_label == label)
        .map(|defaults| defaults.role)
}

#[test]
fn all_seventeen_enter_targets_use_shared_defaults_and_split_even_for_same_role() {
    for (role, expected_target) in ENTER_MATRIX {
        let native_target = role_for_label(formatting_for_role(role).behavior.return_key);
        assert_eq!(native_target, Some(expected_target), "Enter target for {role:?}");

        let text = if role == Role::Parenthetical {
            ""
        } else {
            "prefix"
        };
        let mut editing = session(role, text);
        caret(&mut editing, 0, text.len());
        let original_project = editing.project().clone();
        editing
            .enter()
            .unwrap_or_else(|_| unreachable!("role Enter must split at the valid end"));
        assert_eq!(editing.project().elements.len(), 2, "split for {role:?}");
        assert_eq!(editing.project().elements[0].role, role);
        assert_eq!(editing.project().elements[0].text, text);
        assert_eq!(editing.project().elements[1].role, expected_target);
        assert!(editing.project().elements[1].text.is_empty());
        assert_eq!(editing.selection().focus, Position { element: 1, offset: 0 });
        assert!(editing.dirty());
        assert!(editing.can_undo());

        editing
            .undo()
            .unwrap_or_else(|_| unreachable!("accepted role Enter must be undoable"));
        assert_eq!(editing.project(), &original_project);
        assert!(!editing.dirty());
        editing
            .redo()
            .unwrap_or_else(|_| unreachable!("accepted role Enter must be redoable"));
        assert_eq!(editing.project().elements[1].role, expected_target);
        assert_eq!(editing.selection().focus, Position { element: 1, offset: 0 });
    }
}

#[test]
fn all_seventeen_tab_targets_use_shared_defaults_and_same_role_is_a_noop() {
    for (role, expected_target) in TAB_MATRIX {
        let native_target = role_for_label(formatting_for_role(role).behavior.tab_key);
        assert_eq!(native_target, Some(expected_target), "Tab target for {role:?}");

        let text = if matches!(role, Role::Parenthetical | Role::Dialogue | Role::Character) {
            ""
        } else {
            "existing"
        };
        let mut editing = session(role, text);
        caret(&mut editing, 0, text.len());
        let before = editing.clone();
        editing
            .tab()
            .unwrap_or_else(|_| unreachable!("Tab is always consumed"));

        if role == Role::General {
            assert_eq!(editing, before, "same-role Tab is a no-op");
            assert!(!editing.can_undo());
        } else {
            assert_eq!(editing.project().elements[0].role, expected_target);
            assert!(editing.can_undo());
            assert_eq!(editing.undo(), Ok(()));
            assert_eq!(editing.project(), before.project());
            assert_eq!(editing.redo(), Ok(()));
            assert_eq!(editing.project().elements[0].role, expected_target);
        }

        if role == Role::Dialogue {
            assert_eq!(editing.project().elements[0].text, "()");
            assert_eq!(editing.selection().focus.offset, 1);
        }
    }
}

#[test]
fn dialogue_enter_splits_utf8_safe_interior_and_uses_action_at_end() {
    let mut editing = session(Role::Dialogue, "α🙂ω");
    let original_id = editing.project().elements[0].id.clone();
    caret(&mut editing, 0, "α🙂".len());
    editing
        .enter()
        .unwrap_or_else(|_| unreachable!("UTF-8 interior split must be accepted"));
    assert_eq!(editing.project().elements[0].id, original_id);
    assert_eq!(editing.project().elements[0].text, "α🙂");
    assert_eq!(editing.project().elements[1].role, Role::Dialogue);
    assert_eq!(editing.project().elements[1].text, "ω");
    assert_eq!(editing.selection().focus, Position { element: 1, offset: 0 });

    let mut at_end = session(Role::Dialogue, "line");
    caret(&mut at_end, 0, "line".len());
    at_end
        .enter()
        .unwrap_or_else(|_| unreachable!("Dialogue end Enter must be accepted"));
    assert_eq!(at_end.project().elements[1].role, Role::Action);
}

#[test]
fn blank_character_and_parenthetical_runtime_policies_remain_special_cases() {
    let mut blank_character = session(Role::Character, "");
    blank_character.tab().unwrap_or_else(|_| unreachable!());
    assert_eq!(blank_character.project().elements[0].role, Role::Transition);
    assert!(blank_character.can_undo());

    let mut nonblank_character = session(Role::Character, "BOB");
    caret(&mut nonblank_character, 0, 1);
    let before = nonblank_character.clone();
    nonblank_character
        .tab()
        .unwrap_or_else(|_| unreachable!("nonempty Character Tab is consumed"));
    assert_eq!(nonblank_character, before);
    assert!(!nonblank_character.can_undo());

    let mut authored_parenthetical = session(Role::Parenthetical, "aside");
    caret(&mut authored_parenthetical, 0, "aside".len());
    let before = authored_parenthetical.clone();
    authored_parenthetical
        .tab()
        .unwrap_or_else(|_| unreachable!("unsupported authored parenthetical Tab is consumed"));
    assert_eq!(authored_parenthetical, before);
    assert!(!authored_parenthetical.can_undo());

    let mut generated_parenthetical = session(Role::Dialogue, "");
    generated_parenthetical
        .tab()
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(generated_parenthetical.project().elements[0].role, Role::Parenthetical);
    assert_eq!(generated_parenthetical.project().elements[0].text, "()");
    assert_eq!(generated_parenthetical.selection().focus.offset, 1);
    generated_parenthetical
        .tab()
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(generated_parenthetical.project().elements[0].role, Role::Dialogue);
    assert!(generated_parenthetical.project().elements[0].text.is_empty());
    assert_eq!(generated_parenthetical.undo(), Ok(()));
    assert_eq!(generated_parenthetical.project().elements[0].role, Role::Parenthetical);
    assert_eq!(generated_parenthetical.project().elements[0].text, "()");
}

#[test]
fn disabling_runtime_special_policies_leaves_native_targets_in_control() {
    let mut no_auto_pair = session(Role::Dialogue, "");
    no_auto_pair.set_policy(EditPolicy {
        auto_parenthetical: false,
        ..no_auto_pair.policy()
    });
    no_auto_pair.tab().unwrap_or_else(|_| unreachable!());
    assert_eq!(no_auto_pair.project().elements[0].role, Role::Parenthetical);
    assert!(no_auto_pair.project().elements[0].text.is_empty());

    no_auto_pair.tab().unwrap_or_else(|_| unreachable!());
    assert_eq!(no_auto_pair.project().elements[0].role, Role::Dialogue);

    let mut no_strict_boundary = session(Role::SceneHeading, "abcd");
    no_strict_boundary.set_policy(EditPolicy {
        strict_transition_boundaries: false,
        ..no_strict_boundary.policy()
    });
    caret(&mut no_strict_boundary, 0, 2);
    no_strict_boundary
        .enter()
        .unwrap_or_else(|_| unreachable!("disabled strict boundary accepts interior split"));
    assert_eq!(no_strict_boundary.project().elements[0].text, "ab");
    assert_eq!(no_strict_boundary.project().elements[1].text, "cd");
    assert_eq!(no_strict_boundary.project().elements[1].role, Role::Action);
}

#[test]
fn strict_boundaries_invalid_utf8_and_capacity_failures_are_atomic() {
    for role in [Role::SceneHeading, Role::Character, Role::Transition] {
        let mut editing = session(role, "abcd");
        caret(&mut editing, 0, 2);
        let before = editing.clone();
        editing.enter().unwrap_or_else(|_| unreachable!());
        assert_eq!(editing, before, "strict Enter boundary for {role:?}");
        editing.tab().unwrap_or_else(|_| unreachable!());
        assert_eq!(editing, before, "strict Tab boundary for {role:?}");
    }

    let mut invalid_boundary = session(Role::Dialogue, "🙂");
    let before_invalid = invalid_boundary.clone();
    assert_eq!(
        invalid_boundary.set_selection(Selection::caret(Position {
            element: 0,
            offset: 1,
        })),
        Err(EditError::InvalidUtf8Boundary)
    );
    assert_eq!(invalid_boundary, before_invalid);

    let mut range_selection = session(Role::Action, "range");
    range_selection
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
        .unwrap_or_else(|_| unreachable!("fixture range must be valid"));
    let before_range = range_selection.clone();
    assert_eq!(range_selection.enter(), Err(EditError::InvalidSelection));
    assert_eq!(range_selection, before_range);
    range_selection.tab().unwrap_or_else(|_| unreachable!());
    assert_eq!(range_selection, before_range);

    let mut project = Project::new().unwrap_or_else(|_| unreachable!());
    project.elements[0].role = Role::Action;
    project.elements[0].text = "last".into();
    for index in 1..MAX_ELEMENTS {
        project.elements.push(
            ScreenplayElement::new(format!("wo005-element-{index}"), Role::Action, "")
                .unwrap_or_else(|_| unreachable!("bounded capacity fixture must be valid")),
        );
    }
    let mut full = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
    caret(&mut full, 0, "last".len());
    let before_full = full.clone();
    assert_eq!(full.enter(), Err(EditError::TooManyElements));
    assert_eq!(full, before_full);
}

#[test]
fn imported_behavior_stays_source_owned_while_keyboard_uses_application_defaults() {
    let source = b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Action'><Behavior PaginateAs='Action' ReturnKey='Transition' Shortcut='vendor-shortcut' TabKey='Note' WinShortcut='vendor-win'/></ElementSettings><Content><Paragraph Type='Action' id='source-action' SpaceBefore='19'><Text Style='Bold'>line</Text></Paragraph></Content></FinalDraft>";
    let project = read_v6(source)
        .unwrap_or_else(|_| unreachable!("custom Behavior fixture must import"))
        .project;
    let source_id = project.elements[0].id.clone();
    let mut editing = EditingSession::new(project.clone())
        .unwrap_or_else(|_| unreachable!("imported project must form a session"));
    caret(&mut editing, 0, "line".len());

    editing.tab().unwrap_or_else(|_| unreachable!());
    assert_eq!(editing.project().elements[0].role, Role::Character);
    assert_eq!(editing.undo(), Ok(()));
    assert_eq!(editing.project().elements[0].role, Role::Action);
    let source_runs = editing.project().elements[0].runs.clone();
    editing.enter().unwrap_or_else(|_| unreachable!());
    assert_eq!(editing.project().elements[1].role, Role::Action);
    assert_ne!(editing.project().elements[0].id, editing.project().elements[1].id);
    assert_eq!(editing.project().elements[0].runs, source_runs);
    assert!(editing.project().elements[1].runs.is_empty());

    let preservation = editing.project().preservation.as_ref().unwrap_or_else(|| unreachable!());
    assert!(preservation
        .source_layout_settings()
        .direct_paragraph_overrides
        .contains_key(&source_id));
    assert!(!preservation
        .source_layout_settings()
        .direct_paragraph_overrides
        .contains_key(&editing.project().elements[1].id));

    let output = write_v6(editing.project())
        .unwrap_or_else(|_| unreachable!("safe keyboard edits must remain serializable"));
    let output_text = String::from_utf8(output.clone())
        .unwrap_or_else(|_| unreachable!("writer emits UTF-8 FDX"));
    assert!(output_text.contains("ReturnKey=\"Transition\""));
    assert!(output_text.contains("TabKey=\"Note\""));
    assert!(output_text.contains("Shortcut=\"vendor-shortcut\""));
    assert!(output_text.contains("WinShortcut=\"vendor-win\""));
    assert!(output_text.contains("SpaceBefore=\"19\""));
    let reopened = read_v6(&output)
        .unwrap_or_else(|_| unreachable!("edited native FDX must reopen structurally"));
    assert_eq!(reopened.project.elements[1].role, Role::Action);
    let reopened_xml = String::from_utf8(output)
        .unwrap_or_else(|_| unreachable!("writer emits UTF-8 FDX"));
    assert!(reopened_xml.contains("ReturnKey=\"Transition\""));
    assert!(reopened_xml.contains("TabKey=\"Note\""));

    let mut reopened_session = EditingSession::new(reopened.project)
        .unwrap_or_else(|_| unreachable!("reopened project must form a session"));
    reopened_session
        .set_selection(Selection::caret(Position {
            element: 0,
            offset: 0,
        }))
        .unwrap_or_else(|_| unreachable!());
    reopened_session
        .tab()
        .unwrap_or_else(|_| unreachable!("application Tab default remains available after reopen"));
    assert_eq!(reopened_session.project().elements[0].role, Role::Character);
}

#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::{
    EditError, EditPolicy, EditingSession, ParagraphKeyTarget, ParagraphKeyTargets, Position,
    Selection,
};
use scriptcraft_core::{Project, Role};

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("projection scenario uses valid bounded input"))
}

fn session(role: Role, text: &str, offset: usize, policy: EditPolicy) -> EditingSession {
    let mut project = valid(Project::new());
    project.elements[0].role = role;
    project.elements[0].text = text.to_owned();
    let mut session = valid(EditingSession::with_policy(project, policy));
    valid(session.set_selection(Selection::caret(Position { element: 0, offset })));
    session
}

fn assert_projected_key_matches_edit(
    editing_session: &mut EditingSession,
    target: ParagraphKeyTarget,
    enter: bool,
) {
    let before = editing_session.clone();
    let result = if enter {
        editing_session.enter()
    } else {
        editing_session.tab()
    };
    match target {
        ParagraphKeyTarget::Target(role) => {
            valid(result);
            let caret = editing_session.selection().focus;
            assert_eq!(editing_session.project().elements[caret.element].role, role);
        }
        ParagraphKeyTarget::NoOp => {
            valid(result);
            assert_eq!(*editing_session, before);
        }
        ParagraphKeyTarget::Unavailable => {
            if enter {
                assert_eq!(result, Err(EditError::InvalidSelection));
                assert_eq!(*editing_session, before);
            } else {
                valid(result);
                assert_eq!(*editing_session, before);
            }
        }
    }
}

#[test]
fn projection_matches_enter_and_tab_commands_for_all_roles_empty_and_nonempty() {
    for role in Role::ALL {
        for text in ["", "x"] {
            let mut editing_session = session(role, text, text.len(), EditPolicy::default());
            let before = editing_session.clone();
            let targets = editing_session.paragraph_key_targets();
            assert_eq!(editing_session.paragraph_key_targets(), targets);
            assert_eq!(editing_session, before, "projection mutated {role:?}/{text:?}");

            assert_projected_key_matches_edit(&mut editing_session, targets.enter, true);

            let mut editing_session = session(role, text, text.len(), EditPolicy::default());
            let targets = editing_session.paragraph_key_targets();
            assert_projected_key_matches_edit(&mut editing_session, targets.tab, false);
        }
    }
}

#[test]
fn strict_boundaries_use_utf8_byte_offsets_and_policy() {
    for role in [Role::SceneHeading, Role::Character, Role::Transition] {
        let mut interior = session(role, "éx", 2, EditPolicy::default());
        assert_eq!(
            interior.paragraph_key_targets(),
            ParagraphKeyTargets {
                enter: ParagraphKeyTarget::NoOp,
                tab: ParagraphKeyTarget::NoOp,
            }
        );
        let before = interior.clone();
        valid(interior.enter());
        assert_eq!(interior, before);
        valid(interior.tab());
        assert_eq!(interior, before);

        let mut end = session(role, "éx", "éx".len(), EditPolicy::default());
        assert!(matches!(
            end.paragraph_key_targets().enter,
            ParagraphKeyTarget::Target(_)
        ));
        if role == Role::Character {
            assert_eq!(end.paragraph_key_targets().tab, ParagraphKeyTarget::NoOp);
        } else {
            assert!(matches!(
                end.paragraph_key_targets().tab,
                ParagraphKeyTarget::Target(_)
            ));
        }

        let policy = EditPolicy {
            strict_transition_boundaries: false,
            ..EditPolicy::default()
        };
        let relaxed = session(role, "éx", 2, policy).paragraph_key_targets();
        assert!(matches!(relaxed.enter, ParagraphKeyTarget::Target(_)));
        if role != Role::Character {
            assert!(matches!(relaxed.tab, ParagraphKeyTarget::Target(_)));
        } else {
            assert_eq!(relaxed.tab, ParagraphKeyTarget::NoOp);
        }
    }

    let empty_character = session(Role::Character, "", 0, EditPolicy::default());
    assert_eq!(
        empty_character.paragraph_key_targets().tab,
        ParagraphKeyTarget::Target(Role::Transition)
    );
    let nonempty_character = session(Role::Character, "cue", 3, EditPolicy::default());
    assert_eq!(
        nonempty_character.paragraph_key_targets().tab,
        ParagraphKeyTarget::NoOp
    );
}

#[test]
fn noncollapsed_selection_is_unavailable_for_hints_and_preserves_command_semantics() {
    let mut editing_session = session(Role::Action, "café", 0, EditPolicy::default());
    valid(editing_session.set_selection(Selection {
        anchor: Position {
            element: 0,
            offset: 2,
        },
        focus: Position {
            element: 0,
            offset: "café".len(),
        },
    }));
    assert_eq!(
        editing_session.paragraph_key_targets(),
        ParagraphKeyTargets {
            enter: ParagraphKeyTarget::Unavailable,
            tab: ParagraphKeyTarget::Unavailable,
        }
    );
    let before = editing_session.clone();
    assert_eq!(editing_session.enter(), Err(EditError::InvalidSelection));
    assert_eq!(editing_session, before);
    valid(editing_session.tab());
    assert_eq!(editing_session, before);

    valid(editing_session.set_selection(Selection {
        anchor: Position {
            element: 0,
            offset: "café".len(),
        },
        focus: Position {
            element: 0,
            offset: 2,
        },
    }));
    assert_eq!(
        editing_session.paragraph_key_targets().enter,
        ParagraphKeyTarget::Unavailable
    );
}

#[test]
fn dialogue_interior_and_end_resolve_different_enter_targets() {
    let mut interior = session(Role::Dialogue, "café", 2, EditPolicy::default());
    assert_eq!(
        interior.paragraph_key_targets().enter,
        ParagraphKeyTarget::Target(Role::Dialogue)
    );
    assert_projected_key_matches_edit(
        &mut interior,
        ParagraphKeyTarget::Target(Role::Dialogue),
        true,
    );

    let mut end = session(Role::Dialogue, "café", "café".len(), EditPolicy::default());
    assert_eq!(
        end.paragraph_key_targets().enter,
        ParagraphKeyTarget::Target(Role::Action)
    );
    assert_projected_key_matches_edit(
        &mut end,
        ParagraphKeyTarget::Target(Role::Action),
        true,
    );
}

#[test]
fn parenthetical_delimiters_repairs_generated_provenance_and_policy_are_resolved() {
    for (text, offset) in [("(", 0), ("()", 1), (")", 1)] {
        let mut editing_session = session(Role::Parenthetical, text, offset, EditPolicy::default());
        let targets = editing_session.paragraph_key_targets();
        assert_eq!(targets.tab, ParagraphKeyTarget::Target(Role::Dialogue));
        assert_projected_key_matches_edit(&mut editing_session, targets.tab, false);
    }

    for (text, offset) in [("(aside", 6), ("aside)", 7), ("aside", 5), ("(aside)", 6)] {
        let mut editing_session = session(Role::Parenthetical, text, offset, EditPolicy::default());
        let targets = editing_session.paragraph_key_targets();
        assert_eq!(targets.enter, ParagraphKeyTarget::Target(Role::Dialogue));
        assert_projected_key_matches_edit(&mut editing_session, targets.enter, true);

        let mut editing_session = session(Role::Parenthetical, text, offset, EditPolicy::default());
        let targets = editing_session.paragraph_key_targets();
        assert_eq!(targets.tab, ParagraphKeyTarget::Target(Role::Dialogue));
        assert_projected_key_matches_edit(&mut editing_session, targets.tab, false);
    }

    let mut generated = session(Role::Dialogue, "", 0, EditPolicy::default());
    valid(generated.change_role(Role::Parenthetical));
    let generated_targets = generated.paragraph_key_targets();
    assert_eq!(generated_targets.enter, ParagraphKeyTarget::Target(Role::Dialogue));
    assert_eq!(generated_targets.tab, ParagraphKeyTarget::Target(Role::Dialogue));
    let before_projection = generated.clone();
    assert_eq!(generated.paragraph_key_targets(), generated_targets);
    assert_eq!(generated, before_projection);
    assert_projected_key_matches_edit(&mut generated, generated_targets.enter, true);

    let no_auto_policy = EditPolicy {
        auto_parenthetical: false,
        ..EditPolicy::default()
    };
    let mut no_auto = session(Role::Parenthetical, "()", 1, no_auto_policy);
    let no_auto_targets = no_auto.paragraph_key_targets();
    assert_eq!(no_auto_targets.tab, ParagraphKeyTarget::Target(Role::Dialogue));
    assert_projected_key_matches_edit(&mut no_auto, no_auto_targets.tab, false);
    assert_eq!(no_auto.project().elements[0].text, "()");
}

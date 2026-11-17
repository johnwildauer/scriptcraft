#![forbid(unsafe_code)]

use scriptcraft_core::editing::{EditError, EditingSession, ParagraphKeyTarget, Position, Selection};
use scriptcraft_core::fdx::read_v6;
use scriptcraft_core::{
    BodyOwner, ParagraphAlignment, Project, Role, RunStyle, ScreenplayElement, MAX_TEXT_BYTES,
};
use std::sync::Arc;

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("editing case uses valid input"))
}

fn assert_same_owner(left: &Project, right: &Project, index: usize) {
    assert!(Arc::ptr_eq(&left.elements[index], &right.elements[index]));
    assert_eq!(left.elements[index].revision(), right.elements[index].revision());
}

fn two_sided_project() -> Project {
    let mut project = valid(Project::new());
    project.elements = [
        ("left-cue", Role::Character, "LEFT"),
        ("left-line", Role::Dialogue, "café"),
        ("right-cue", Role::Character, "RIGHT"),
        ("right-line", Role::Dialogue, "你好"),
    ]
    .into_iter()
    .map(|(id, role, text)| BodyOwner::shared(valid(ScreenplayElement::new(id, role, text))))
    .collect();
    project
}

#[test]
fn development_mutation_diagnostics_record_only_actual_body_candidate_invocations() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    assert!(session.take_mutation_diagnostics().is_none());

    session.begin_mutation_diagnostics();
    valid(session.insert(""));
    let Some(no_op) = session.take_mutation_diagnostics() else {
        unreachable!("an actual Body candidate invocation produces one observation")
    };
    assert!(no_op.no_op);
    assert_eq!(no_op.scoped_candidate_attempts, 1);
    assert_eq!(no_op.scoped_candidate_accepts, 1);
    assert!(no_op.scoped_validation_nanos.is_some());
    assert!(no_op.no_op_equality_nanos.is_some());
    assert!(no_op.derived_classification_nanos.is_none());
    assert!(no_op.dirty_comparison_nanos.is_none());
    assert!(no_op.history_publication_nanos.is_none());

    session.begin_mutation_diagnostics();
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    assert!(session.take_mutation_diagnostics().is_none());

    session.begin_mutation_diagnostics();
    valid(session.insert("x"));
    let Some(changed) = session.take_mutation_diagnostics() else {
        unreachable!("a changed Body candidate invocation produces one observation")
    };
    assert!(!changed.no_op);
    assert!(changed.no_op_equality_nanos.is_some());
    assert!(changed.derived_classification_nanos.is_some());
    assert!(changed.dirty_comparison_nanos.is_some());
    assert!(changed.history_publication_nanos.is_some());
    assert!(session.take_mutation_diagnostics().is_none());
}

#[test]
fn body_and_title_history_share_one_timeline_and_restore_dirty_state() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.set_title_selection(Selection::caret(Position { element: 17, offset: 0 })));
    valid(session.insert_title("Feature"));
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.insert("INT. CAFE — DAY"));
    assert!(session.dirty());

    valid(session.undo());
    assert_eq!(session.project().elements[0].text, "");
    assert_eq!(session.project().title_page[17].text, "FeatureScript Title");
    valid(session.undo());
    assert_eq!(session.project().title_page[17].text, "Script Title");
    assert!(!session.dirty());
    valid(session.redo());
    valid(session.redo());
    assert_eq!(session.project().elements[0].text, "INT. CAFE — DAY");
}

#[test]
fn utf8_editing_formatting_and_navigation_keep_byte_offsets_and_roles() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.insert("café"));
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: "café".len(),
    })));
    let keys = session.paragraph_key_targets();
    assert!(matches!(keys.enter, ParagraphKeyTarget::Target(_)));
    let before_invalid = session.clone();
    assert!(session
        .set_selection(Selection::caret(Position { element: 0, offset: 4 }))
        .is_err());
    assert_eq!(session, before_invalid);

    valid(session.set_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 0, offset: "café".len() },
    }));
    let role = session.project().elements[0].role;
    valid(session.toggle_body_run_style(RunStyle::Bold));
    valid(session.set_body_alignment(ParagraphAlignment::Right));
    assert_eq!(session.project().elements[0].role, role);
    assert_eq!(
        session.project().direct_paragraph_overrides[&session.project().elements[0].id]
            .paragraph
            .alignment,
        Some(ParagraphAlignment::Right)
    );
    assert!(session.project().elements[0].runs.iter().any(|run| {
        run.attributes.iter().any(|attribute| {
            attribute.name == "Style" && attribute.value.contains("Bold")
        })
    }));
}

#[test]
fn paragraph_key_projection_is_read_only_and_matches_character_boundary_rules() {
    let mut session = valid(EditingSession::new(two_sided_project()));
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 4 })));
    let before = session.clone();
    let projected = session.paragraph_key_targets();
    assert_eq!(session.paragraph_key_targets(), projected);
    assert_eq!(session, before);
    assert_eq!(projected.tab, ParagraphKeyTarget::NoOp);
    assert!(matches!(projected.enter, ParagraphKeyTarget::Target(_)));

    valid(session.enter());
    assert_ne!(session, before);
    assert_eq!(session.project().elements.len(), 5);
}

#[test]
fn dual_dialogue_toggle_is_atomic_reversible_and_ineligible_toggle_is_a_noop() {
    let mut session = valid(EditingSession::new(two_sided_project()));
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    let members = session.project().elements.clone();
    valid(session.toggle_dual_dialogue());
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
    assert_eq!(session.project().elements, members);
    valid(session.undo());
    assert!(session.project().dual_dialogue_groups.is_empty());
    valid(session.redo());
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);

    let mut ordinary = valid(EditingSession::new(valid(Project::new())));
    let before = ordinary.clone();
    valid(ordinary.toggle_dual_dialogue());
    assert_eq!(ordinary, before);
}

#[test]
fn group_unwrap_preserves_visible_members_and_invalid_selection_rolls_back() {
    let mut session = valid(EditingSession::new(two_sided_project()));
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.toggle_dual_dialogue());
    let before_invalid_selection = session.clone();
    assert!(session
        .set_selection(Selection::caret(Position { element: 1, offset: usize::MAX }))
        .is_err());
    assert_eq!(session, before_invalid_selection);

    valid(session.set_selection(Selection::caret(Position { element: 3, offset: 0 })));
    valid(session.toggle_dual_dialogue());
    assert!(session.project().dual_dialogue_groups.is_empty());
    assert_eq!(
        session.project().elements.iter().map(|item| item.text.as_str()).collect::<Vec<_>>(),
        ["LEFT", "café", "RIGHT", "你好"]
    );
    assert!(session.can_undo());
}

#[test]
fn empty_neighbors_are_removed_before_role_checks_and_preserve_the_surviving_caret() {
    let mut forward_project = valid(Project::new());
    forward_project.elements = vec![
        BodyOwner::shared(valid(ScreenplayElement::new("action", Role::Action, "keep"))),
        BodyOwner::shared(valid(ScreenplayElement::new("empty-dialogue", Role::Dialogue, ""))),
        BodyOwner::shared(valid(ScreenplayElement::new("right", Role::Transition, "right"))),
    ];
    let mut forward = valid(EditingSession::new(forward_project));
    valid(forward.set_selection(Selection::caret(Position { element: 0, offset: 4 })));
    valid(forward.delete());
    assert_eq!(forward.project().elements.len(), 2);
    assert_eq!(forward.project().elements[0].id.as_str(), "action");
    assert_eq!(forward.project().elements[1].id.as_str(), "right");
    assert_eq!(forward.selection(), Selection::caret(Position { element: 0, offset: 4 }));

    let mut backward_project = valid(Project::new());
    backward_project.elements = vec![
        BodyOwner::shared(valid(ScreenplayElement::new("empty-action", Role::Action, ""))),
        BodyOwner::shared(valid(ScreenplayElement::new("dialogue", Role::Dialogue, "café"))),
    ];
    let mut backward = valid(EditingSession::new(backward_project));
    valid(backward.set_selection(Selection::caret(Position { element: 1, offset: 0 })));
    valid(backward.backspace());
    assert_eq!(backward.project().elements.len(), 1);
    assert_eq!(backward.project().elements[0].id.as_str(), "dialogue");
    assert_eq!(backward.project().elements[0].text, "café");
    assert_eq!(backward.selection(), Selection::caret(Position { element: 0, offset: 0 }));
}

#[test]
fn incompatible_nonempty_boundary_joins_reject_transactionally_in_both_directions() {
    for backward in [false, true] {
        let mut project = valid(Project::new());
        project.elements = vec![
            BodyOwner::shared(valid(ScreenplayElement::new("left", Role::Action, "left"))),
            BodyOwner::shared(valid(ScreenplayElement::new("right", Role::Dialogue, "right"))),
        ];
        let mut session = valid(EditingSession::new(project));
        valid(session.set_selection(Selection::caret(if backward {
            Position { element: 1, offset: 0 }
        } else {
            Position { element: 0, offset: 4 }
        })));
        let before = session.clone();
        let result = if backward {
            session.backspace()
        } else {
            session.delete()
        };
        assert_eq!(result, Err(scriptcraft_core::editing::EditError::InvalidSelection));
        assert_eq!(session, before);
    }
}

#[test]
fn one_final_empty_element_is_a_silent_noop_in_both_directions() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    for backward in [false, true] {
        let before = session.clone();
        if backward {
            valid(session.backspace());
        } else {
            valid(session.delete());
        }
        assert_eq!(session, before);
        assert!(!session.can_undo());
        assert!(!session.dirty());
        assert_eq!(session.revision(), before.revision());
    }
}

#[test]
fn one_final_empty_title_paragraph_is_a_silent_noop_in_both_directions() {
    let mut project = valid(Project::new());
    project.title_page.truncate(1);
    project.title_page[0].text.clear();
    project.title_page[0].runs.clear();
    let mut session = valid(EditingSession::new(project));
    valid(session.set_title_selection(Selection::caret(Position { element: 0, offset: 0 })));
    for backward in [false, true] {
        let before = session.clone();
        if backward {
            valid(session.backspace_title());
        } else {
            valid(session.delete_title());
        }
        assert_eq!(session, before);
        assert!(!session.can_undo());
        assert!(!session.dirty());
        assert_eq!(session.revision(), before.revision());
    }
}

#[test]
fn irrelevant_action_edits_skip_derived_work_while_character_and_heading_edits_invalidate_one_scene() {
    let mut project = valid(Project::new());
    project.elements = [
        ("heading", Role::SceneHeading, "INT. ROOM - DAY"),
        ("action", Role::Action, "Action"),
        ("cue", Role::Character, "ALICE"),
        ("dialogue", Role::Dialogue, "Hello"),
    ]
    .into_iter()
    .map(|(id, role, text)| BodyOwner::shared(valid(ScreenplayElement::new(id, role, text))))
    .collect();
    let mut session = valid(EditingSession::new(project));
    valid(session.refresh_cast_lists());
    assert!(!session.pending_derived_work_summary().pending);

    valid(session.set_selection(Selection::caret(Position { element: 1, offset: 6 })));
    valid(session.insert(" changed"));
    assert!(session.last_edit_used_scoped_validation());
    assert!(!session.pending_derived_work_summary().pending);

    valid(session.set_selection(Selection::caret(Position { element: 2, offset: 5 })));
    valid(session.insert("X"));
    assert!(session.last_edit_used_scoped_validation());
    let character_work = session.pending_derived_work_summary();
    assert!(character_work.pending);
    assert!(!character_work.complete);
    assert_eq!(character_work.affected_scene_count, 1);
    valid(session.refresh_cast_lists());
    assert!(!session.pending_derived_work_summary().pending);

    let heading_offset = session.project().elements[0].text.len();
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: heading_offset,
    })));
    valid(session.insert("!"));
    assert!(session.last_edit_used_scoped_validation());
    let heading_work = session.pending_derived_work_summary();
    assert!(heading_work.pending);
    assert!(!heading_work.complete);
    assert_eq!(heading_work.affected_scene_count, 1);
}

fn assert_publication_state_unchanged(actual: &EditingSession, expected: &EditingSession) {
    assert_eq!(actual, expected);
    assert_eq!(actual.project(), expected.project());
    assert_eq!(actual.selection(), expected.selection());
    assert_eq!(actual.title_selection(), expected.title_selection());
    assert_eq!(actual.body_typing_style(), expected.body_typing_style());
    assert_eq!(actual.title_typing_style(), expected.title_typing_style());
    assert_eq!(actual.revision(), expected.revision());
    assert_eq!(actual.mutation_token(), expected.mutation_token());
    assert_eq!(actual.dirty(), expected.dirty());
    assert_eq!(actual.can_undo(), expected.can_undo());
    assert_eq!(actual.can_redo(), expected.can_redo());
}

#[test]
fn ordinary_body_insert_delete_publish_exact_candidates_and_restore_exact_history() {
    let original = valid(Project::new());
    let original_selection = Selection::caret(Position { element: 0, offset: 0 });
    let mut session = valid(EditingSession::new(original.clone()));
    let initial_revision = session.revision();
    let initial_token = session.mutation_token();

    valid(session.insert("alpha"));
    let mut expected_inserted = original.clone();
    BodyOwner::update(&mut expected_inserted.elements[0], |element| {
        element.text = "alpha".to_owned();
    });
    assert_eq!(session.project(), &expected_inserted);
    assert_eq!(
        session.selection(),
        Selection::caret(Position { element: 0, offset: "alpha".len() })
    );
    assert_eq!(session.revision(), initial_revision + 1);
    assert_eq!(session.mutation_token(), initial_token + 1);
    assert!(session.dirty());
    assert!(session.can_undo());
    assert!(!session.can_redo());

    valid(session.undo());
    assert_eq!(session.project(), &original);
    assert_eq!(session.selection(), original_selection);
    assert!(!session.dirty());
    assert!(session.can_redo());
    valid(session.redo());
    assert_eq!(session.project(), &expected_inserted);
    let inserted = session.project().clone();
    let inserted_selection = session.selection();

    valid(session.backspace());
    let mut expected_deleted = inserted.clone();
    BodyOwner::update(&mut expected_deleted.elements[0], |element| {
        element.text = "alph".to_owned();
    });
    assert_eq!(session.project(), &expected_deleted);
    assert_eq!(
        session.selection(),
        Selection::caret(Position { element: 0, offset: "alph".len() })
    );
    valid(session.undo());
    assert_eq!(session.project(), &inserted);
    assert_eq!(session.selection(), inserted_selection);
    valid(session.redo());
    assert_eq!(session.project(), &expected_deleted);
}

#[test]
fn repeated_body_edits_preserve_history_eviction_dirty_redo_and_mutation_order() {
    let mut session = valid(EditingSession::with_history_limit(valid(Project::new()), 2));
    valid(session.insert("a"));
    let project_a = session.project().clone();
    session.mark_saved();
    assert!(!session.dirty());

    valid(session.insert("b"));
    let project_ab = session.project().clone();
    valid(session.insert("c"));
    let project_abc = session.project().clone();
    assert_eq!(project_abc.elements[0].text, "abc");
    let revision = session.revision();
    let token = session.mutation_token();

    valid(session.undo());
    assert_eq!(session.project(), &project_ab);
    assert!(session.dirty());
    valid(session.undo());
    assert_eq!(session.project(), &project_a);
    assert!(!session.dirty());
    assert!(session.can_redo());
    let evicted = session.clone();
    assert_eq!(session.undo(), Err(EditError::EmptyHistory));
    assert_publication_state_unchanged(&session, &evicted);
    assert_eq!(session.revision(), revision + 2);
    assert_eq!(session.mutation_token(), token + 2);

    valid(session.redo());
    assert_eq!(session.project(), &project_ab);
    assert!(session.dirty());
    valid(session.redo());
    assert_eq!(session.project(), &project_abc);
    valid(session.undo());
    assert_eq!(session.project(), &project_ab);
    assert!(session.can_redo());

    let revision_before_branch = session.revision();
    let token_before_branch = session.mutation_token();
    valid(session.insert("!"));
    assert_eq!(session.project().elements[0].text, "ab!");
    assert!(!session.can_redo());
    assert!(session.dirty());
    assert_eq!(session.revision(), revision_before_branch + 1);
    assert_eq!(session.mutation_token(), token_before_branch + 1);
    let branched = session.project().clone();
    valid(session.undo());
    assert_eq!(session.project(), &project_ab);
    valid(session.redo());
    assert_eq!(session.project(), &branched);
}

#[test]
fn body_noops_and_practical_failures_preserve_project_history_redo_revision_and_token() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.insert("seed"));
    let seeded = session.project().clone();
    valid(session.undo());
    assert!(session.can_redo());

    let before_noop = session.clone();
    valid(session.insert(""));
    assert_publication_state_unchanged(&session, &before_noop);

    let oversized = "x".repeat(MAX_TEXT_BYTES + 1);
    assert_eq!(session.insert(&oversized), Err(EditError::TextTooLong));
    assert_publication_state_unchanged(&session, &before_noop);
    valid(session.redo());
    assert_eq!(session.project(), &seeded);

    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    let before_boundary = session.clone();
    valid(session.backspace());
    assert_publication_state_unchanged(&session, &before_boundary);

    let mut project = valid(Project::new());
    project.elements = vec![
        BodyOwner::shared(valid(ScreenplayElement::new("left", Role::Action, "left"))),
        BodyOwner::shared(valid(ScreenplayElement::new("right", Role::Dialogue, "right"))),
    ];
    let mut invalid = valid(EditingSession::new(project));
    valid(invalid.change_role(Role::General));
    valid(invalid.undo());
    assert!(invalid.can_redo());
    valid(invalid.set_selection(Selection::caret(Position { element: 0, offset: 4 })));
    let before_invalid = invalid.clone();
    assert_eq!(invalid.delete(), Err(EditError::InvalidSelection));
    assert_publication_state_unchanged(&invalid, &before_invalid);
}

#[test]
fn structural_dual_unwrap_retries_from_the_grouped_pre_edit_baseline() {
    let original = two_sided_project();
    let mut grouped = valid(EditingSession::new(original.clone()));
    valid(grouped.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(grouped.toggle_dual_dialogue());
    let grouped_baseline = grouped.project().clone();
    assert_eq!(grouped_baseline.dual_dialogue_groups.len(), 1);

    valid(grouped.set_selection(Selection::caret(Position { element: 1, offset: 0 })));
    let grouped_selection = grouped.selection();
    let mut reference = valid(EditingSession::new(original));
    valid(reference.set_selection(Selection::caret(Position { element: 1, offset: 0 })));
    valid(reference.change_role(Role::Action));
    let expected = reference.project().clone();

    valid(grouped.change_role(Role::Action));
    assert_eq!(grouped.project(), &expected);
    assert_eq!(grouped.selection(), reference.selection());
    assert!(grouped.project().dual_dialogue_groups.is_empty());
    valid(grouped.undo());
    assert_eq!(grouped.project(), &grouped_baseline);
    assert_eq!(grouped.selection(), grouped_selection);
    valid(grouped.redo());
    assert_eq!(grouped.project(), &expected);
}

#[test]
fn source_backed_publication_restores_source_and_preservation_failure_keeps_redo() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="body"><Text>Opening <InlineOpaque/>action.</Text><Text> tail</Text></Paragraph></Content></FinalDraft>"#;
    let project = valid(read_v6(source)).project;
    let initial = project.clone();
    let mut session = valid(EditingSession::new(project));
    let text_len = session.project().elements[0].text.len();
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: text_len,
    })));
    valid(session.insert("!"));
    let inserted = session.project().clone();
    assert!(inserted.preservation.is_some());
    assert_eq!(inserted.elements[0].text.len(), text_len + 1);

    valid(session.undo());
    assert_eq!(session.project(), &initial);
    assert!(Arc::ptr_eq(&initial.elements[0], &session.project().elements[0]));
    assert_eq!(initial.elements[0].revision(), session.project().elements[0].revision());
    assert!(session.can_redo());
    valid(session.set_selection(Selection {
        anchor: Position {
            element: 0,
            offset: "Opening".len(),
        },
        focus: Position {
            element: 0,
            offset: "Opening a".len(),
        },
    }));
    let before_failure = session.clone();
    assert!(session.insert("").is_err());
    assert_publication_state_unchanged(&session, &before_failure);
    valid(session.redo());
    assert_eq!(session.project(), &inserted);
    assert!(Arc::ptr_eq(&inserted.elements[0], &session.project().elements[0]));
    assert_eq!(inserted.elements[0].revision(), session.project().elements[0].revision());
}

#[test]
fn generated_parenthetical_provenance_survives_move_to_history_and_undo_redo() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.change_role(Role::Dialogue));
    valid(session.tab());
    assert_eq!(session.project().elements[0].role, Role::Parenthetical);
    assert_eq!(session.project().elements[0].text, "()");
    assert_eq!(
        session.selection(),
        Selection::caret(Position { element: 0, offset: 1 })
    );
    let generated_project = session.project().clone();
    let generated_selection = session.selection();

    valid(session.insert("quiet"));
    assert_eq!(session.project().elements[0].text, "(quiet)");
    valid(session.undo());
    assert_eq!(session.project(), &generated_project);
    assert_eq!(session.selection(), generated_selection);

    valid(session.enter());
    assert_eq!(session.project().elements.len(), 2);
    assert_eq!(session.project().elements[0].text, "");
    assert_eq!(session.project().elements[1].role, Role::Dialogue);
    let entered = session.project().clone();
    valid(session.undo());
    assert_eq!(session.project(), &generated_project);
    assert_eq!(session.selection(), generated_selection);
    valid(session.redo());
    assert_eq!(session.project(), &entered);
}

#[test]
fn ordinary_body_publication_has_one_candidate_clone_and_moves_history_project() {
    let source = include_str!("../src/editing.rs");
    let function = source
        .split("fn mutate_with_scene_heading_deletions<F>")
        .nth(1)
        .and_then(|tail| tail.split("\n    fn mutate_title<F>").next())
        .unwrap_or_else(|| unreachable!("Body mutation helper remains present"));
    assert!(function.contains("let baseline = &self.project;"));
    assert!(function.contains("let mut first_project = baseline.clone();"));
    assert!(function.contains("std::mem::replace(&mut self.project, project)"));
    assert!(!function.contains("let baseline = self.project.clone();"));
    assert!(!function.contains("project: self.project.clone(),"));
}

#[test]
fn body_owner_clones_candidates_history_and_clean_restoration_share_exact_survivors() {
    let project = two_sided_project();
    let cloned = project.clone();
    for index in 0..project.elements.len() {
        assert_same_owner(&project, &cloned, index);
    }

    let mut session = valid(EditingSession::new(project));
    let before = session.project().clone();
    valid(session.set_selection(Selection::caret(Position { element: 1, offset: 4 })));
    valid(session.insert("!"));
    let edited = session.project().clone();
    assert!(!Arc::ptr_eq(&before.elements[1], &edited.elements[1]));
    assert_ne!(before.elements[1].revision(), edited.elements[1].revision());
    for index in [0, 2, 3] {
        assert_same_owner(&before, &edited, index);
    }

    let save_candidate = valid(session.prepare_cast_list_save_candidate());
    for index in 0..edited.elements.len() {
        assert_same_owner(&edited, &save_candidate, index);
    }

    valid(session.undo());
    for index in 0..before.elements.len() {
        assert_same_owner(&before, session.project(), index);
    }
    valid(session.redo());
    for index in 0..edited.elements.len() {
        assert_same_owner(&edited, session.project(), index);
    }

    session.mark_saved();
    valid(session.backspace());
    assert!(session.dirty());
    valid(session.insert("!"));
    assert!(!session.dirty());
    assert_eq!(session.project(), &edited);
    assert!(!Arc::ptr_eq(&edited.elements[1], &session.project().elements[1]));
    assert_ne!(edited.elements[1].revision(), session.project().elements[1].revision());
    for index in [0, 2, 3] {
        assert_same_owner(&edited, session.project(), index);
    }

    let before_title = session.project().clone();
    valid(session.insert_title("Title edit"));
    for index in 0..before_title.elements.len() {
        assert_same_owner(&before_title, session.project(), index);
    }

    let mut no_op = session.project().clone();
    for owner in &mut no_op.elements {
        BodyOwner::update(owner, |_| {});
    }
    for index in 0..no_op.elements.len() {
        assert_same_owner(session.project(), &no_op, index);
    }
}

#[test]
fn body_owner_revisions_follow_split_join_delete_move_and_history_lifecycles() {
    let mut project = valid(Project::new());
    project.elements = [
        ("left", Role::Action, "left"),
        ("right", Role::Action, "right"),
        ("tail", Role::Note, "tail"),
    ]
    .into_iter()
    .map(|(id, role, text)| BodyOwner::shared(valid(ScreenplayElement::new(id, role, text))))
    .collect();
    let mut session = valid(EditingSession::new(project));
    let before = session.project().clone();
    let removed_id = before.elements[1].id.clone();
    let prior_revisions = before.elements.iter().map(|owner| owner.revision()).collect::<Vec<_>>();

    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 3 })));
    valid(session.split());
    assert!(!session.last_edit_used_scoped_validation());
    let split = session.project().clone();
    assert!(!Arc::ptr_eq(&before.elements[0], &split.elements[0]));
    assert_ne!(before.elements[0].revision(), split.elements[0].revision());
    assert_ne!(split.elements[1].id, split.elements[0].id);
    assert!(!prior_revisions.contains(&split.elements[1].revision()));
    assert!(Arc::ptr_eq(&before.elements[2], &split.elements[3]));
    assert_eq!(before.elements[2].revision(), split.elements[3].revision());

    let mut moved = split.clone();
    moved.elements.swap(0, 1);
    assert!(Arc::ptr_eq(&split.elements[0], &moved.elements[1]));
    assert!(Arc::ptr_eq(&split.elements[1], &moved.elements[0]));
    assert_eq!(split.elements[0].revision(), moved.elements[1].revision());
    assert_eq!(split.elements[1].revision(), moved.elements[0].revision());

    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 3 })));
    valid(session.join_with_next());
    let joined = session.project().clone();
    assert_eq!(joined.elements[0].text, "left");
    assert_ne!(split.elements[0].revision(), joined.elements[0].revision());
    assert!(Arc::ptr_eq(&split.elements[3], &joined.elements[2]));
    assert_eq!(split.elements[3].revision(), joined.elements[2].revision());
    valid(session.undo());
    for index in 0..split.elements.len() {
        assert_same_owner(&split, session.project(), index);
    }
    valid(session.redo());
    for index in 0..joined.elements.len() {
        assert_same_owner(&joined, session.project(), index);
    }

    valid(session.set_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 1, offset: 0 },
    }));
    valid(session.delete());
    let deleted = session.project().clone();
    assert_eq!(deleted.elements.len(), 2);
    assert!(deleted.elements.iter().all(|owner| owner.id != removed_id));
    assert!(Arc::ptr_eq(&joined.elements[2], &deleted.elements[1]));
    assert_eq!(joined.elements[2].revision(), deleted.elements[1].revision());
    valid(session.undo());
    for index in 0..joined.elements.len() {
        assert_same_owner(&joined, session.project(), index);
    }
    valid(session.redo());
    for index in 0..deleted.elements.len() {
        assert_same_owner(&deleted, session.project(), index);
    }
}

#[test]
fn body_owner_formatting_roles_dual_dialogue_and_noops_have_local_revisions() {
    let mut session = valid(EditingSession::new(two_sided_project()));
    let before = session.project().clone();
    valid(session.set_selection(Selection {
        anchor: Position { element: 1, offset: 0 },
        focus: Position { element: 1, offset: 4 },
    }));
    valid(session.toggle_body_run_style(RunStyle::Bold));
    let formatted = session.project().clone();
    assert_ne!(before.elements[1].revision(), formatted.elements[1].revision());
    for index in [0, 2, 3] {
        assert_same_owner(&before, &formatted, index);
    }

    valid(session.set_body_alignment(ParagraphAlignment::Right));
    let aligned = session.project().clone();
    assert_eq!(formatted.elements[1].as_element(), aligned.elements[1].as_element());
    assert!(!Arc::ptr_eq(&formatted.elements[1], &aligned.elements[1]));
    assert_ne!(formatted.elements[1].revision(), aligned.elements[1].revision());
    for index in [0, 2, 3] {
        assert_same_owner(&formatted, &aligned, index);
    }
    valid(session.set_body_alignment(ParagraphAlignment::Right));
    for index in 0..aligned.elements.len() {
        assert_same_owner(&aligned, session.project(), index);
    }

    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.change_role(Role::Action));
    let role_changed = session.project().clone();
    assert_ne!(aligned.elements[0].revision(), role_changed.elements[0].revision());
    for index in 1..role_changed.elements.len() {
        assert_same_owner(&aligned, &role_changed, index);
    }
    valid(session.change_role(Role::Action));
    for index in 0..role_changed.elements.len() {
        assert_same_owner(&role_changed, session.project(), index);
    }
    valid(session.undo());
    for index in 0..aligned.elements.len() {
        assert_same_owner(&aligned, session.project(), index);
    }
    valid(session.redo());
    for index in 0..role_changed.elements.len() {
        assert_same_owner(&role_changed, session.project(), index);
    }

    let mut dual = valid(EditingSession::new(two_sided_project()));
    let before_dual = dual.project().clone();
    valid(dual.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(dual.toggle_dual_dialogue());
    assert_eq!(dual.project().dual_dialogue_groups.len(), 1);
    for index in 0..before_dual.elements.len() {
        assert_same_owner(&before_dual, dual.project(), index);
    }
}

#[test]
fn derived_cast_updates_replace_only_derived_owners_and_reuse_unchanged_results() {
    let mut project = valid(Project::new());
    project.elements = [
        ("heading", Role::SceneHeading, "INT. ROOM - DAY"),
        ("cue", Role::Character, "Alice"),
        ("cast", Role::CastList, ""),
        ("tail", Role::Action, "tail"),
    ]
    .into_iter()
    .map(|(id, role, text)| BodyOwner::shared(valid(ScreenplayElement::new(id, role, text))))
    .collect();
    let mut session = valid(EditingSession::new(project));
    let before = session.project().clone();
    assert_eq!(valid(session.refresh_cast_lists()), true);
    let corrected = session.project().clone();
    assert_eq!(corrected.elements[2].text, "(ALICE)");
    for index in [0, 1, 3] {
        assert_same_owner(&before, &corrected, index);
    }
    assert_ne!(before.elements[2].revision(), corrected.elements[2].revision());
    assert_eq!(valid(session.refresh_cast_lists()), false);
    for index in 0..corrected.elements.len() {
        assert_same_owner(&corrected, session.project(), index);
    }

    valid(session.set_selection(Selection {
        anchor: Position { element: 1, offset: 0 },
        focus: Position { element: 1, offset: 5 },
    }));
    valid(session.insert("Bob"));
    assert!(session.last_edit_used_scoped_validation());
    let character_edited = session.project().clone();
    for index in [0, 2, 3] {
        assert_same_owner(&corrected, &character_edited, index);
    }
    assert_eq!(valid(session.refresh_cast_lists()), true);
    assert_eq!(session.project().elements[2].text, "(BOB)");
    for index in [0, 1, 3] {
        assert_same_owner(&character_edited, session.project(), index);
    }
    assert_ne!(character_edited.elements[2].revision(), session.project().elements[2].revision());
}

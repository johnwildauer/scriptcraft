#![forbid(unsafe_code)]

use scriptcraft_core::editing::{EditingSession, ParagraphKeyTarget, Position, Selection};
use scriptcraft_core::dual_dialogue::{
    DualDialogueError, DualDialogueGroup, DualDialogueSide, DualDialogueWrapperProvenance,
};
use scriptcraft_core::formatting::SourceParagraphOverrides;
use scriptcraft_core::{
    ModelError, ParagraphAlignment, Project, Role, RunAttribute, RunStyle, ScreenplayElement,
    StableId, TextRun,
};
use std::ops::Range;

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("editing case uses valid input"))
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
    .map(|(id, role, text)| valid(ScreenplayElement::new(id, role, text)))
    .collect();
    project
}

fn group_from_ranges(
    project: &Project,
    id: &str,
    left: Range<usize>,
    right: Range<usize>,
) -> DualDialogueGroup {
    DualDialogueGroup {
        id: valid(StableId::new(id)),
        left: DualDialogueSide {
            character: project.elements[left.start].id.clone(),
            paragraphs: project.elements[left.start + 1..left.end]
                .iter()
                .map(|element| element.id.clone())
                .collect(),
        },
        right: DualDialogueSide {
            character: project.elements[right.start].id.clone(),
            paragraphs: project.elements[right.start + 1..right.end]
                .iter()
                .map(|element| element.id.clone())
                .collect(),
        },
        wrapper: DualDialogueWrapperProvenance::default(),
    }
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
    assert!(session
        .set_selection(Selection::caret(Position { element: 1, offset: 0 }))
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

fn deletion_project(empty_index: usize) -> Project {
    let mut project = valid(Project::new());
    let elements = [
        ("left", Role::Action, "left"),
        ("middle", Role::Dialogue, "middle"),
        ("right", Role::Transition, "right"),
    ];
    project.elements = elements
        .into_iter()
        .enumerate()
        .map(|(index, (id, role, text))| {
            valid(ScreenplayElement::new(
                id,
                role,
                if index == empty_index { "" } else { text },
            ))
        })
        .collect();
    project
}

#[test]
fn collapsed_delete_and_backspace_remove_empty_current_at_each_position_with_directional_carets() {
    for index in 0..3 {
        let mut forward = valid(EditingSession::new(deletion_project(index)));
        valid(forward.set_selection(Selection::caret(Position { element: index, offset: 0 })));
        let before_forward = forward.clone();
        valid(forward.delete());
        let forward_after = forward.clone();
        let expected_forward = match index {
            0 => (0, 0, "middle"),
            1 => (1, 0, "right"),
            _ => (1, "middle".len(), "middle"),
        };
        assert_eq!(forward.project().elements.len(), 2);
        assert_eq!(forward.project().elements[expected_forward.0].id.as_str(), expected_forward.2);
        assert_eq!(
            forward.selection(),
            Selection::caret(Position {
                element: expected_forward.0,
                offset: expected_forward.1,
            })
        );
        valid(forward.undo());
        assert_eq!(forward, before_forward);
        valid(forward.redo());
        assert_eq!(forward, forward_after);

        let mut backward = valid(EditingSession::new(deletion_project(index)));
        valid(backward.set_selection(Selection::caret(Position { element: index, offset: 0 })));
        let before_backward = backward.clone();
        valid(backward.backspace());
        let backward_after = backward.clone();
        let expected_backward = match index {
            0 => (0, 0, "middle"),
            1 => (0, "left".len(), "left"),
            _ => (1, "middle".len(), "middle"),
        };
        assert_eq!(backward.project().elements.len(), 2);
        assert_eq!(backward.project().elements[expected_backward.0].id.as_str(), expected_backward.2);
        assert_eq!(
            backward.selection(),
            Selection::caret(Position {
                element: expected_backward.0,
                offset: expected_backward.1,
            })
        );
        valid(backward.undo());
        assert_eq!(backward, before_backward);
        valid(backward.redo());
        assert_eq!(backward, backward_after);
    }
}

#[test]
fn empty_neighbors_are_removed_before_role_checks_and_preserve_the_surviving_caret() {
    let mut forward_project = valid(Project::new());
    forward_project.elements = vec![
        valid(ScreenplayElement::new("action", Role::Action, "keep")),
        valid(ScreenplayElement::new("empty-dialogue", Role::Dialogue, "")),
        valid(ScreenplayElement::new("empty-transition", Role::Transition, "")),
    ];
    let mut forward = valid(EditingSession::new(forward_project));
    valid(forward.set_selection(Selection::caret(Position { element: 0, offset: 4 })));
    valid(forward.delete());
    assert_eq!(forward.project().elements.len(), 2);
    assert_eq!(forward.project().elements[0].id.as_str(), "action");
    assert_eq!(forward.selection(), Selection::caret(Position { element: 0, offset: 4 }));

    let mut backward_project = valid(Project::new());
    backward_project.elements = vec![
        valid(ScreenplayElement::new("empty-action", Role::Action, "")),
        valid(ScreenplayElement::new("dialogue", Role::Dialogue, "café")),
    ];
    let mut backward = valid(EditingSession::new(backward_project));
    valid(backward.set_selection(Selection::caret(Position { element: 1, offset: 0 })));
    valid(backward.backspace());
    assert_eq!(backward.project().elements.len(), 1);
    assert_eq!(backward.project().elements[0].id.as_str(), "dialogue");
    assert_eq!(backward.project().elements[0].text, "café");
    assert_eq!(backward.selection(), Selection::caret(Position { element: 0, offset: 0 }));

    let mut both_empty_project = valid(Project::new());
    both_empty_project.elements = vec![
        valid(ScreenplayElement::new("empty-action", Role::Action, "")),
        valid(ScreenplayElement::new("empty-dialogue", Role::Dialogue, "")),
        valid(ScreenplayElement::new("right", Role::Transition, "right")),
    ];
    let mut both_empty = valid(EditingSession::new(both_empty_project));
    valid(both_empty.set_selection(Selection::caret(Position { element: 1, offset: 0 })));
    valid(both_empty.delete());
    assert_eq!(both_empty.project().elements.len(), 2);
    assert_eq!(both_empty.project().elements[0].id.as_str(), "empty-action");
    assert_eq!(both_empty.project().elements[1].id.as_str(), "right");
    assert_eq!(both_empty.selection(), Selection::caret(Position { element: 1, offset: 0 }));
}

fn join_project() -> Project {
    let mut project = valid(Project::new());
    let bold = vec![RunAttribute { name: "Style".into(), value: "Bold".into() }];
    let italic = vec![RunAttribute { name: "Style".into(), value: "Italic".into() }];
    let mut left = valid(ScreenplayElement::new("kept", Role::Action, "é"));
    left.runs = vec![TextRun { text: "é".into(), attributes: bold.clone() }];
    let mut right = valid(ScreenplayElement::new("removed", Role::Action, "界!"));
    right.runs = vec![
        TextRun { text: "界".into(), attributes: bold },
        TextRun { text: "!".into(), attributes: italic },
    ];
    project.elements = vec![left, right];
    project
}

fn assert_joined_unicode_and_runs(session: &EditingSession) {
    assert_eq!(session.project().elements.len(), 1);
    assert_eq!(session.project().elements[0].id.as_str(), "kept");
    assert_eq!(session.project().elements[0].role, Role::Action);
    assert_eq!(session.project().elements[0].text, "é界!");
    assert_eq!(
        session.project().elements[0].runs,
        vec![
            TextRun {
                text: "é界".into(),
                attributes: vec![RunAttribute { name: "Style".into(), value: "Bold".into() }],
            },
            TextRun {
                text: "!".into(),
                attributes: vec![RunAttribute { name: "Style".into(), value: "Italic".into() }],
            },
        ]
    );
}

#[test]
fn same_role_unicode_boundary_joins_keep_left_identity_coalesce_runs_and_restore_history() {
    for backward in [false, true] {
        let mut session = valid(EditingSession::new(join_project()));
        let before = session.clone();
        valid(session.set_selection(Selection::caret(if backward {
            Position { element: 1, offset: 0 }
        } else {
            Position { element: 0, offset: "é".len() }
        })));
        let before_join = session.clone();
        if backward {
            valid(session.backspace());
        } else {
            valid(session.delete());
        }
        assert_joined_unicode_and_runs(&session);
        assert_eq!(session.selection(), Selection::caret(Position { element: 0, offset: "é".len() }));
        let after_join = session.clone();
        valid(session.undo());
        assert_eq!(session, before_join);
        valid(session.redo());
        assert_eq!(session, after_join);
        assert_ne!(before, after_join);
    }
}

#[test]
fn incompatible_nonempty_boundary_joins_reject_transactionally_in_both_directions() {
    for backward in [false, true] {
        let mut project = valid(Project::new());
        project.elements = vec![
            valid(ScreenplayElement::new("left", Role::Action, "left")),
            valid(ScreenplayElement::new("right", Role::Dialogue, "right")),
        ];
        let mut session = valid(EditingSession::new(project));
        valid(session.set_selection(Selection::caret(if backward {
            Position { element: 1, offset: 0 }
        } else {
            Position { element: 0, offset: 4 }
        })));
        let before = session.clone();
        let result = if backward { session.backspace() } else { session.delete() };
        assert_eq!(result, Err(scriptcraft_core::editing::EditError::IncompatibleParagraphJoin));
        assert_eq!(session, before);
    }
}

#[test]
fn one_final_empty_element_and_document_edges_are_silent_noops() {
    let mut only_empty = valid(EditingSession::new(valid(Project::new())));
    for backspace in [false, true] {
        let before = only_empty.clone();
        if backspace {
            valid(only_empty.backspace());
        } else {
            valid(only_empty.delete());
        }
        assert_eq!(only_empty, before);
        assert!(!only_empty.can_undo());
        assert!(!only_empty.dirty());
        assert_eq!(only_empty.revision(), before.revision());
    }

    let mut project = valid(Project::new());
    project.elements = vec![
        valid(ScreenplayElement::new("first", Role::Action, "first")),
        valid(ScreenplayElement::new("last", Role::Action, "last")),
    ];
    for (backspace, position) in [
        (true, Position { element: 0, offset: 0 }),
        (false, Position { element: 1, offset: 4 }),
    ] {
        let mut session = valid(EditingSession::new(project.clone()));
        valid(session.set_selection(Selection::caret(position)));
        let before = session.clone();
        if backspace {
            valid(session.backspace());
        } else {
            valid(session.delete());
        }
        assert_eq!(session, before);
        assert!(!session.can_undo());
        assert_eq!(session.revision(), before.revision());
    }
}

#[test]
fn delimiter_parenthetical_special_case_and_noncollapsed_cross_role_deletion_remain_unchanged() {
    let mut project = valid(Project::new());
    project.elements = vec![
        valid(ScreenplayElement::new("action", Role::Action, "before")),
        valid(ScreenplayElement::new("existing-aside", Role::Parenthetical, "()")),
    ];
    let mut parenthetical = valid(EditingSession::new(project));
    valid(parenthetical.set_selection(Selection::caret(Position { element: 1, offset: 0 })));
    valid(parenthetical.enter());
    assert_eq!(parenthetical.project().elements[1].text, "()");
    assert_eq!(parenthetical.project().elements[2].id.as_str(), "existing-aside");
    let before_parenthetical = parenthetical.clone();
    valid(parenthetical.delete());
    assert_eq!(parenthetical.project().elements.len(), 2);
    assert_eq!(parenthetical.project().elements[0].id.as_str(), "action");
    assert_eq!(parenthetical.selection(), Selection::caret(Position { element: 0, offset: 6 }));
    let after_parenthetical = parenthetical.clone();
    valid(parenthetical.undo());
    assert_eq!(parenthetical, before_parenthetical);
    valid(parenthetical.redo());
    assert_eq!(parenthetical, after_parenthetical);

    let mut cross_role_project = valid(Project::new());
    cross_role_project.elements = vec![
        valid(ScreenplayElement::new("action", Role::Action, "abc")),
        valid(ScreenplayElement::new("dialogue", Role::Dialogue, "def")),
    ];
    let mut cross_role = valid(EditingSession::new(cross_role_project));
    valid(cross_role.set_selection(Selection {
        anchor: Position { element: 0, offset: 1 },
        focus: Position { element: 1, offset: 2 },
    }));
    valid(cross_role.delete());
    assert_eq!(cross_role.project().elements.len(), 1);
    assert_eq!(cross_role.project().elements[0].text, "af");
    assert_eq!(cross_role.project().elements[0].role, Role::Action);
}

#[test]
fn body_owner_removal_cleans_dual_members_cues_and_wrapper_registrations_with_history() {
    let mut project = valid(Project::new());
    project.elements = vec![
        valid(ScreenplayElement::new("left-cue", Role::Character, "")),
        valid(ScreenplayElement::new("left-line", Role::Dialogue, "")),
        valid(ScreenplayElement::new("right-cue", Role::Character, "RIGHT")),
        valid(ScreenplayElement::new("right-line", Role::Dialogue, "line")),
    ];
    let group_id = valid(StableId::new("dual-wrapper"));
    project.dual_dialogue_groups.push(DualDialogueGroup {
        id: group_id.clone(),
        left: DualDialogueSide {
            character: project.elements[0].id.clone(),
            paragraphs: vec![project.elements[1].id.clone()],
        },
        right: DualDialogueSide {
            character: project.elements[2].id.clone(),
            paragraphs: vec![project.elements[3].id.clone()],
        },
        wrapper: DualDialogueWrapperProvenance::default(),
    });
    project
        .direct_paragraph_overrides
        .insert(project.elements[1].id.clone(), SourceParagraphOverrides::default());
    project
        .direct_paragraph_overrides
        .insert(project.elements[0].id.clone(), SourceParagraphOverrides::default());
    project
        .dual_dialogue_paragraph_overrides
        .insert(group_id.clone(), SourceParagraphOverrides::default());
    let mut session = valid(EditingSession::new(project));

    valid(session.set_selection(Selection::caret(Position { element: 1, offset: 0 })));
    let before_member_removal = session.clone();
    valid(session.delete());
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
    assert!(session.project().dual_dialogue_groups[0].left.paragraphs.is_empty());
    assert!(!session.project().direct_paragraph_overrides.contains_key(&valid(StableId::new("left-line"))));
    assert_eq!(session.project().validate(), Ok(()));
    let after_member_removal = session.clone();
    valid(session.undo());
    assert_eq!(session, before_member_removal);
    valid(session.redo());
    assert_eq!(session, after_member_removal);

    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.delete());
    assert!(session.project().dual_dialogue_groups.is_empty());
    assert!(!session.project().dual_dialogue_paragraph_overrides.contains_key(&group_id));
    assert!(!session.project().direct_paragraph_overrides.contains_key(&valid(StableId::new("left-cue"))));
    assert!(session.project().dual_dialogue_groups.iter().all(|group| {
        group.member_ids().all(|member| session.project().elements.iter().any(|element| &element.id == member))
    }));
    assert_eq!(session.project().validate(), Ok(()));
}

#[test]
fn dual_group_position_lookup_preserves_member_error_categories() {
    let mut missing = two_sided_project();
    missing
        .dual_dialogue_groups
        .push(group_from_ranges(&missing, "missing-group", 0..2, 2..4));
    missing.dual_dialogue_groups[0].left.character = valid(StableId::new("missing-member"));
    assert_eq!(
        missing.validate_current_state(),
        Err(ModelError::InvalidDualDialogue(DualDialogueError::MissingMember)),
    );

    let mut duplicate_member = two_sided_project();
    duplicate_member
        .dual_dialogue_groups
        .push(group_from_ranges(&duplicate_member, "duplicate-member-group", 0..2, 2..4));
    let repeated = duplicate_member.dual_dialogue_groups[0].left.paragraphs[0].clone();
    duplicate_member.dual_dialogue_groups[0]
        .left
        .paragraphs
        .push(repeated);
    assert_eq!(
        duplicate_member.validate_current_state(),
        Err(ModelError::InvalidDualDialogue(DualDialogueError::DuplicateMember)),
    );

    let mut noncontiguous = valid(Project::new());
    noncontiguous.elements = [
        ("gap-left-cue", Role::Character),
        ("gap-left-line", Role::Dialogue),
        ("outside-group", Role::Action),
        ("gap-right-cue", Role::Character),
        ("gap-right-line", Role::Dialogue),
    ]
    .into_iter()
    .map(|(id, role)| valid(ScreenplayElement::new(id, role, "")))
    .collect();
    noncontiguous
        .dual_dialogue_groups
        .push(group_from_ranges(&noncontiguous, "noncontiguous-group", 0..2, 3..5));
    assert_eq!(
        noncontiguous.validate_current_state(),
        Err(ModelError::InvalidDualDialogue(
            DualDialogueError::NonContiguousMembers,
        )),
    );

    let mut out_of_order = valid(Project::new());
    out_of_order.elements = (0..8)
        .map(|index| {
            let role = match index % 4 {
                0 | 2 => Role::Character,
                _ => Role::Dialogue,
            };
            valid(ScreenplayElement::new(format!("ordered-{index}"), role, ""))
        })
        .collect();
    out_of_order.dual_dialogue_groups.push(group_from_ranges(
        &out_of_order,
        "later-group",
        4..6,
        6..8,
    ));
    out_of_order.dual_dialogue_groups.push(group_from_ranges(
        &out_of_order,
        "earlier-group",
        0..2,
        2..4,
    ));
    assert_eq!(
        out_of_order.validate_current_state(),
        Err(ModelError::InvalidDualDialogue(DualDialogueError::GroupsOutOfOrder)),
    );

    let mut invalid_role = valid(Project::new());
    invalid_role.elements = [
        ("role-left-cue", Role::Character),
        ("role-left-line", Role::Action),
        ("role-right-cue", Role::Character),
        ("role-right-line", Role::Dialogue),
    ]
    .into_iter()
    .map(|(id, role)| valid(ScreenplayElement::new(id, role, "")))
    .collect();
    invalid_role
        .dual_dialogue_groups
        .push(group_from_ranges(&invalid_role, "invalid-role-group", 0..2, 2..4));
    assert_eq!(
        invalid_role.validate_current_state(),
        Err(ModelError::InvalidDualDialogue(DualDialogueError::InvalidRolePattern)),
    );
}

#[test]
fn many_dual_groups_validate_without_reordering_or_changing_membership() {
    const GROUP_COUNT: usize = 64;
    let mut project = valid(Project::new());
    project.elements = (0..GROUP_COUNT)
        .flat_map(|group_index| {
            [
                (format!("many-{group_index}-left-cue"), Role::Character),
                (format!("many-{group_index}-left-line"), Role::Dialogue),
                (format!("many-{group_index}-right-cue"), Role::Character),
                (format!("many-{group_index}-right-line"), Role::Dialogue),
            ]
        })
        .map(|(id, role)| valid(ScreenplayElement::new(id, role, "")))
        .collect();
    for group_index in 0..GROUP_COUNT {
        let start = group_index * 4;
        project.dual_dialogue_groups.push(group_from_ranges(
            &project,
            &format!("many-group-{group_index}"),
            start..start + 2,
            start + 2..start + 4,
        ));
    }
    let original_groups = project.dual_dialogue_groups.clone();

    assert_eq!(project.validate_current_state(), Ok(()));
    assert_eq!(project.dual_dialogue_groups, original_groups);
}

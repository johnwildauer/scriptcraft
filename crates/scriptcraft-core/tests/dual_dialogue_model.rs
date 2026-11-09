use scriptcraft_core::dual_dialogue::{
    self, DualDialogueGroup, DualDialogueSide, DualDialogueSideKind,
    DualDialogueWrapperProvenance, SelectionSurface, ToggleTarget,
};
use scriptcraft_core::editing::{Position, Selection};
use scriptcraft_core::formatting::{SourceParagraphOverrides, SourceParagraphSpec};
use scriptcraft_core::text_state::{flat_offset_to_position, selection_to_flat_offsets};
use scriptcraft_core::{ModelError, Project, Role, ScreenplayElement, StableId};

fn project_with_roles(roles: &[Role]) -> Project {
    let mut project = Project::new().expect("project construction");
    project.elements = roles
        .iter()
        .enumerate()
        .map(|(index, role)| {
            ScreenplayElement::new(format!("body-{index}"), *role, "").expect("element construction")
        })
        .collect();
    project
}

fn group(project: &Project, group_id: &str, left_range: std::ops::Range<usize>, right_range: std::ops::Range<usize>) -> DualDialogueGroup {
    let side = |range: std::ops::Range<usize>| DualDialogueSide {
        character: project.elements[range.start].id.clone(),
        paragraphs: project.elements[range.start + 1..range.end]
            .iter()
            .map(|element| element.id.clone())
            .collect(),
    };
    DualDialogueGroup {
        id: StableId::new(group_id).expect("group identity"),
        left: side(left_range),
        right: side(right_range),
        wrapper: DualDialogueWrapperProvenance::default(),
    }
}

fn minimal_project() -> Project {
    let mut project = project_with_roles(&[Role::Character, Role::Dialogue, Role::Character, Role::Dialogue]);
    project.dual_dialogue_groups.push(group(&project, "wrapper-1", 0..2, 2..4));
    project
}

#[test]
fn accepts_minimal_multi_paragraph_empty_and_adjacent_groups() {
    let mut project = project_with_roles(&[
        Role::Character, Role::Dialogue, Role::Character, Role::Dialogue,
        Role::Character, Role::Dialogue, Role::Parenthetical, Role::Character,
        Role::Parenthetical, Role::Dialogue,
    ]);
    project.dual_dialogue_groups.push(group(&project, "wrapper-1", 0..2, 2..4));
    project.dual_dialogue_groups.push(group(&project, "wrapper-2", 4..7, 7..10));
    assert_eq!(project.validate(), Ok(()));
    assert!(project.elements.iter().all(|element| element.text.is_empty()));
}

#[test]
fn accepts_empty_sides_and_rejects_invalid_roles() {
    for (roles, left_range, right_range) in [
        (vec![Role::Character, Role::Character], 0..1, 1..2),
        (vec![Role::Character, Role::Character, Role::Dialogue], 0..1, 1..3),
        (vec![Role::Character, Role::Dialogue, Role::Character], 0..2, 2..3),
        (vec![Role::Character, Role::Character, Role::Parenthetical], 0..1, 1..3),
        (vec![Role::Character, Role::Parenthetical, Role::Character], 0..2, 2..3),
    ] {
        let mut project = project_with_roles(&roles);
        project.dual_dialogue_groups.push(group(&project, "empty-side", left_range, right_range));
        assert_eq!(project.validate(), Ok(()));
    }

    for invalid in [Role::Action, Role::Character, Role::Transition, Role::General] {
        let mut project = project_with_roles(&[Role::Character, invalid, Role::Character, Role::Dialogue]);
        project.dual_dialogue_groups.push(group(&project, "wrapper-invalid", 0..2, 2..4));
        assert!(matches!(project.validate(), Err(ModelError::InvalidDualDialogue(_))));
    }
}

#[test]
fn rejects_gaps_reordering_overlap_and_duplicate_members() {
    let mut gap = project_with_roles(&[
        Role::Character, Role::Dialogue, Role::Action, Role::Character, Role::Dialogue,
    ]);
    gap.dual_dialogue_groups.push(group(&gap, "wrapper-gap", 0..2, 3..5));
    assert!(matches!(gap.validate(), Err(ModelError::InvalidDualDialogue(_))));

    let mut reordered = project_with_roles(&[
        Role::Character, Role::Dialogue, Role::Character, Role::Dialogue,
        Role::Character, Role::Dialogue,
    ]);
    let mut reversed = group(&reordered, "wrapper-order", 0..2, 2..4);
    reversed.right.paragraphs.push(reordered.elements[5].id.clone());
    reordered.dual_dialogue_groups.push(reversed);
    assert!(matches!(reordered.validate(), Err(ModelError::InvalidDualDialogue(_))));

    let mut duplicate = minimal_project();
    let original = duplicate.dual_dialogue_groups[0].clone();
    let mut second = original.clone();
    second.id = StableId::new("wrapper-2").expect("group identity");
    duplicate.dual_dialogue_groups.push(second);
    assert!(matches!(duplicate.validate(), Err(ModelError::InvalidDualDialogue(_))));

    let mut repeated = minimal_project();
    repeated.dual_dialogue_groups[0].right.paragraphs[0] = repeated.elements[1].id.clone();
    assert!(matches!(repeated.validate(), Err(ModelError::InvalidDualDialogue(_))));
}

#[test]
fn wrapper_ids_are_unique_against_members_and_other_wrappers() {
    let mut member_collision = minimal_project();
    member_collision.dual_dialogue_groups[0].id = member_collision.elements[0].id.clone();
    assert!(matches!(member_collision.validate(), Err(ModelError::InvalidDualDialogue(_))));

    let mut two = project_with_roles(&[
        Role::Character, Role::Dialogue, Role::Character, Role::Dialogue,
        Role::Character, Role::Dialogue, Role::Character, Role::Dialogue,
    ]);
    two.dual_dialogue_groups.push(group(&two, "wrapper-same", 0..2, 2..4));
    let mut second = group(&two, "wrapper-same", 4..6, 6..8);
    second.id = two.dual_dialogue_groups[0].id.clone();
    two.dual_dialogue_groups.push(second);
    assert!(matches!(two.validate(), Err(ModelError::InvalidDualDialogue(_))));
}

#[test]
fn wrapper_override_keys_are_wrapper_owned_and_fields_are_bounded() {
    let mut valid = minimal_project();
    valid.dual_dialogue_paragraph_overrides.insert(
        valid.dual_dialogue_groups[0].id.clone(),
        SourceParagraphOverrides {
            paragraph: SourceParagraphSpec {
                left_indent: Some(scriptcraft_core::FixedPoint::from_raw(100)),
                right_indent: None,
                space_before: None,
                ..SourceParagraphSpec::default()
            },
            starts_new_page: Some(true),
        },
    );
    assert_eq!(valid.validate(), Ok(()));

    let mut wrong_owner = minimal_project();
    wrong_owner.dual_dialogue_paragraph_overrides.insert(
        wrong_owner.elements[0].id.clone(),
        SourceParagraphOverrides::default(),
    );
    assert!(matches!(wrong_owner.validate(), Err(ModelError::InvalidDualDialogue(_))));

    let mut unsupported = minimal_project();
    unsupported.dual_dialogue_paragraph_overrides.insert(
        unsupported.dual_dialogue_groups[0].id.clone(),
        SourceParagraphOverrides {
            paragraph: SourceParagraphSpec { alignment: Some(scriptcraft_core::ParagraphAlignment::Center), ..SourceParagraphSpec::default() },
            starts_new_page: None,
        },
    );
    assert!(matches!(unsupported.validate(), Err(ModelError::InvalidDualDialogue(_))));
}

#[test]
fn identity_and_index_queries_resolve_current_flat_ranges() {
    let project = minimal_project();
    let group = &project.dual_dialogue_groups[0];
    assert_eq!(dual_dialogue::group_at_element_index(&project, 0), Some(group));
    assert_eq!(dual_dialogue::group_at_element_id(&project, &project.elements[3].id), Some(group));
    assert_eq!(dual_dialogue::first_member(&project, group), Some(0));
    assert_eq!(dual_dialogue::last_member(&project, group), Some(3));
    assert_eq!(dual_dialogue::left_flat_range(&project, group), Some(0..2));
    assert_eq!(dual_dialogue::right_flat_range(&project, group), Some(2..4));
    assert_eq!(dual_dialogue::side_at_element_index(&project, 1), Some(DualDialogueSideKind::Left));
    assert_eq!(dual_dialogue::side_at_element_index(&project, 3), Some(DualDialogueSideKind::Right));
    assert_eq!(dual_dialogue::group_order(&project).count(), 1);
}

#[test]
fn empty_side_queries_use_the_cue_as_a_single_member_range_and_preserve_order() {
    for (roles, left_range, right_range, expected_left, expected_right) in [
        (vec![Role::Character, Role::Character], 0..1, 1..2, 0..1, 1..2),
        (vec![Role::Character, Role::Character, Role::Dialogue], 0..1, 1..3, 0..1, 1..3),
        (vec![Role::Character, Role::Dialogue, Role::Character], 0..2, 2..3, 0..2, 2..3),
        (vec![Role::Character, Role::Character, Role::Parenthetical], 0..1, 1..3, 0..1, 1..3),
        (vec![Role::Character, Role::Parenthetical, Role::Character], 0..2, 2..3, 0..2, 2..3),
    ] {
        let mut project = project_with_roles(&roles);
        project.dual_dialogue_groups.push(group(&project, "query-empty", left_range, right_range));
        let group = &project.dual_dialogue_groups[0];
        assert_eq!(dual_dialogue::left_flat_range(&project, group), Some(expected_left));
        assert_eq!(dual_dialogue::right_flat_range(&project, group), Some(expected_right));
        assert_eq!(dual_dialogue::first_member(&project, group), Some(0));
        assert_eq!(dual_dialogue::last_member(&project, group), Some(project.elements.len() - 1));
        assert_eq!(group.member_ids().count(), project.elements.len());
        assert_eq!(dual_dialogue::group_at_element_index(&project, project.elements.len() - 1), Some(group));
    }
}

#[test]
fn caret_queries_enable_only_valid_first_cues_and_disable_from_every_member() {
    let ungrouped = project_with_roles(&[
        Role::Character, Role::Dialogue, Role::Parenthetical, Role::Character,
        Role::Parenthetical, Role::Dialogue, Role::Character, Role::Dialogue,
    ]);
    let selection = Selection::caret(Position { element: 0, offset: 0 });
    assert!(matches!(
        dual_dialogue::resolve_toggle_target(&ungrouped, SelectionSurface::Body, selection),
        Ok(Some(ToggleTarget::Enable(_)))
    ));
    assert_eq!(dual_dialogue::resolve_toggle_target(&ungrouped, SelectionSurface::TitlePage, selection), Ok(None));
    assert_eq!(dual_dialogue::resolve_toggle_target(
        &ungrouped, SelectionSurface::Body,
        Selection { anchor: Position { element: 0, offset: 0 }, focus: Position { element: 1, offset: 0 } },
    ), Ok(None));
    assert_eq!(dual_dialogue::resolve_toggle_target(
        &ungrouped, SelectionSurface::Body,
        Selection::caret(Position { element: 6, offset: 0 }),
    ), Ok(None));

    let grouped = minimal_project();
    for element in 0..grouped.elements.len() {
        let target = dual_dialogue::resolve_toggle_target(
            &grouped,
            SelectionSurface::Body,
            Selection::caret(Position { element, offset: 0 }),
        ).expect("grouped caret resolution");
        assert_eq!(target, Some(ToggleTarget::Disable(grouped.dual_dialogue_groups[0].id.clone())));
    }
    assert_eq!(dual_dialogue::caret_side(&grouped, Position { element: 3, offset: 0 }), Some(DualDialogueSideKind::Right));
}

#[test]
fn candidate_runs_stop_at_third_cue_or_non_dialogue_role_and_reject_existing_group_ambiguity() {
    let project = project_with_roles(&[
        Role::Character, Role::Dialogue, Role::Character, Role::Dialogue,
        Role::Character, Role::Dialogue, Role::Action,
    ]);
    assert!(dual_dialogue::recognize_candidate(&project, 0).expect("third cue is not part of a candidate").is_none());
    assert!(dual_dialogue::recognize_candidate(&project, 1).expect("not a candidate").is_none());

    let project = project_with_roles(&[Role::Character, Role::Dialogue, Role::Character, Role::Dialogue, Role::Action]);
    let candidate = dual_dialogue::recognize_candidate(&project, 0).expect("candidate recognition").expect("valid two-side candidate");
    assert_eq!(candidate.flat_range, 0..4);
    assert_eq!(candidate.right.character, project.elements[2].id);

    let grouped = minimal_project();
    assert!(dual_dialogue::recognize_candidate(&grouped, 0).expect("group ambiguity").is_none());
}

#[test]
fn candidate_recognition_accepts_exact_empty_side_grammar_and_rejects_one_cue_or_wrong_roles() {
    for roles in [
        vec![Role::Character, Role::Character],
        vec![Role::Character, Role::Character, Role::Dialogue],
        vec![Role::Character, Role::Dialogue, Role::Character],
        vec![Role::Character, Role::Character, Role::Parenthetical],
        vec![Role::Character, Role::Parenthetical, Role::Character],
        vec![Role::Character, Role::Dialogue, Role::Parenthetical, Role::Character, Role::Parenthetical],
    ] {
        let project = project_with_roles(&roles);
        let candidate = dual_dialogue::recognize_candidate(&project, 0)
            .expect("candidate recognition")
            .expect("approved exact two-cue shape");
        assert_eq!(candidate.flat_range, 0..roles.len());
        assert_eq!(candidate.left.character, project.elements[0].id);
        assert_eq!(candidate.right.character, project.elements[candidate.left.paragraphs.len() + 1].id);
    }
    for roles in [
        vec![Role::Character],
        vec![Role::Character, Role::Dialogue],
        vec![Role::Character, Role::Action, Role::Character],
        vec![Role::Character, Role::Character, Role::Character],
    ] {
        let project = project_with_roles(&roles);
        assert!(dual_dialogue::recognize_candidate(&project, 0)
            .expect("malformed candidates are rejected without repair")
            .is_none());
    }
}

#[test]
fn wrapper_is_not_a_visible_element_or_text_state_paragraph_and_json_rejects_it() {
    let project = minimal_project();
    assert_eq!(project.elements.len(), 4);
    assert_eq!(flat_offset_to_position(&project, 3), Some(Position { element: 3, offset: 0 }));
    assert_eq!(selection_to_flat_offsets(&project, Selection::caret(Position { element: 3, offset: 0 })), Some((3, 3)));
    assert_eq!(project.to_json(), Err(ModelError::UnsupportedJsonState));

    let ungrouped = project_with_roles(&[Role::SceneHeading]);
    assert_eq!(ungrouped.dual_dialogue_groups.len(), 0);
    assert_eq!(ungrouped.elements.len(), 1);
}

#[test]
fn flat_text_state_offsets_count_only_visible_members_and_one_separator_per_paragraph() {
    let mut project = minimal_project();
    for (element, text) in project.elements.iter_mut().zip(["A", "xy", "界", "z✨"]) {
        element.text = text.to_owned();
    }

    assert_eq!(flat_offset_to_position(&project, 0), Some(Position { element: 0, offset: 0 }));
    assert_eq!(flat_offset_to_position(&project, 2), Some(Position { element: 1, offset: 0 }));
    assert_eq!(flat_offset_to_position(&project, 5), Some(Position { element: 2, offset: 0 }));
    assert_eq!(flat_offset_to_position(&project, 7), Some(Position { element: 3, offset: 0 }));
    assert_eq!(flat_offset_to_position(&project, 10), Some(Position { element: 3, offset: "z✨".len() }));
    assert_eq!(flat_offset_to_position(&project, 11), None);
    assert_eq!(
        selection_to_flat_offsets(
            &project,
            Selection::caret(Position { element: 3, offset: "z✨".len() }),
        ),
        Some((10, 10)),
    );
}

#[test]
fn legacy_json_remains_compatible_when_ungrouped() {
    let input = r#"{"schema_version":1,"project":{"id":"legacy-project","schema_version":1,"metadata":{"title":"","author":"","contact":"","created_at":"","updated_at":""},"title_page":[],"page_profile":{"name":"us_letter_screenplay","format_version":1,"paper_width":612,"paper_height":792,"margin_top":72,"margin_bottom":72,"margin_left":108,"margin_right":72,"chars_per_line":55,"lines_per_page":55,"line_advance":12,"boundary_gap":12,"uppercase_cues":true,"auto_parenthetical":true,"strict_transition_boundaries":true,"font_family":"Courier Prime","font_size":12,"char_width":7,"role_boxes":[{"x":108,"width":432},{"x":108,"width":432},{"x":259,"width":175},{"x":180,"width":216},{"x":216,"width":180},{"x":432,"width":108},{"x":108,"width":432}]},"elements":[{"id":"body-1","role":"scene_heading","text":""}]}}"#;
    let project = Project::from_json(input).expect("legacy JSON fixture");
    assert!(project.dual_dialogue_groups.is_empty());
    assert!(project.to_json().is_ok());
}

#[test]
fn group_and_membership_capacity_boundaries_are_rejected() {
    let mut too_many_groups = minimal_project();
    let template = too_many_groups.dual_dialogue_groups[0].clone();
    too_many_groups.dual_dialogue_groups = (0..=scriptcraft_core::MAX_ELEMENTS)
        .map(|_| template.clone())
        .collect();
    assert!(matches!(too_many_groups.validate(), Err(ModelError::InvalidDualDialogue(_))));

    let mut too_many_members = minimal_project();
    let repeated_id = too_many_members.elements[1].id.clone();
    too_many_members.dual_dialogue_groups[0].left.paragraphs
        .extend((0..scriptcraft_core::MAX_ELEMENTS).map(|_| repeated_id.clone()));
    assert!(matches!(too_many_members.validate(), Err(ModelError::InvalidDualDialogue(_))));
}

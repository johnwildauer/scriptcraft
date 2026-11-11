#![forbid(unsafe_code)]

use scriptcraft_core::dual_dialogue::{self, DualDialogueGroup, DualDialogueSide, DualDialogueWrapperProvenance, SelectionSurface, ToggleTarget};
use scriptcraft_core::editing::{Position, Selection};
use scriptcraft_core::{ModelError, Project, Role, RunAttribute, ScreenplayElement, StableId, TextRun, TitlePageParagraph};
use std::ops::Range;

fn project_with_roles(roles: &[Role]) -> Project {
    let mut project = Project::new().expect("valid default project");
    project.elements = roles
        .iter()
        .enumerate()
        .map(|(index, role)| ScreenplayElement::new(format!("p-{index}"), *role, "").expect("valid element"))
        .collect();
    project
}

fn side(project: &Project, range: Range<usize>) -> DualDialogueSide {
    DualDialogueSide {
        character: project.elements[range.start].id.clone(),
        paragraphs: project.elements[range.start + 1..range.end]
            .iter()
            .map(|element| element.id.clone())
            .collect(),
    }
}

fn group(project: &Project, id: &str, left: Range<usize>, right: Range<usize>) -> DualDialogueGroup {
    DualDialogueGroup {
        id: StableId::new(id).expect("valid wrapper id"),
        left: side(project, left),
        right: side(project, right),
        wrapper: DualDialogueWrapperProvenance::default(),
    }
}

#[test]
fn semantic_validation_enforces_identity_and_dual_group_grammar_ranges_and_carets() {
    let mut project = project_with_roles(&[
        Role::Character,
        Role::Dialogue,
        Role::Parenthetical,
        Role::Character,
        Role::Dialogue,
    ]);
    project.dual_dialogue_groups.push(group(&project, "wrapper", 0..3, 3..5));
    assert_eq!(project.validate(), Ok(()));

    let grouped = &project.dual_dialogue_groups[0];
    assert_eq!(dual_dialogue::left_flat_range(&project, grouped), Some(0..3));
    assert_eq!(dual_dialogue::right_flat_range(&project, grouped), Some(3..5));
    assert_eq!(dual_dialogue::group_at_element_index(&project, 4), Some(grouped));
    assert_eq!(dual_dialogue::first_member(&project, grouped), Some(0));
    assert_eq!(dual_dialogue::last_member(&project, grouped), Some(4));
    for element in 0..5 {
        assert_eq!(
            dual_dialogue::resolve_toggle_target(
                &project,
                SelectionSurface::Body,
                Selection::caret(Position { element, offset: 0 }),
            ),
            Ok(Some(ToggleTarget::Disable(grouped.id.clone()))),
        );
    }

    let mut invalid = project.clone();
    invalid.dual_dialogue_groups[0].id = invalid.elements[0].id.clone();
    assert!(matches!(invalid.validate(), Err(ModelError::InvalidDualDialogue(_))));
    let mut bad_grammar = project_with_roles(&[Role::Character, Role::Action, Role::Character, Role::Dialogue]);
    bad_grammar.dual_dialogue_groups.push(group(&bad_grammar, "bad", 0..2, 2..4));
    assert!(matches!(bad_grammar.validate(), Err(ModelError::InvalidDualDialogue(_))));

    let candidate = project_with_roles(&[Role::Character, Role::Dialogue, Role::Character, Role::Dialogue]);
    assert_eq!(dual_dialogue::recognize_candidate(&candidate, 0).expect("candidate query").map(|value| value.flat_range), Some(0..4));
    assert!(dual_dialogue::recognize_candidate(&candidate, 1).expect("non-cue query").is_none());
}

#[test]
fn title_page_template_and_import_model_constraints_are_bounded() {
    let project = Project::new().expect("valid default project");
    assert_eq!(project.title_page.len(), 54);
    assert_eq!(project.title_page[17].text, "Script Title");
    assert_eq!(project.title_page[21].text, "Written by");
    assert!(project.title_page.iter().all(|paragraph| paragraph.validate().is_ok()));

    let oversized = TitlePageParagraph::new(
        "too-long",
        "x".repeat(scriptcraft_core::MAX_TEXT_BYTES + 1),
        Default::default(),
    );
    assert_eq!(oversized.err(), Some(ModelError::TextTooLong));

    let mut duplicate_id = project.clone();
    duplicate_id.elements[0] = ScreenplayElement::new(
        duplicate_id.title_page[0].id.as_str(),
        Role::SceneHeading,
        "",
    )
    .expect("well-formed element with colliding identity");
    assert_eq!(duplicate_id.validate(), Err(ModelError::DuplicateId));
}

#[test]
fn formatted_text_projection_and_attribute_limits_are_semantic_invariants() {
    let mut project = Project::new().expect("valid project");
    project.elements[0].text = "café".into();
    project.elements[0].runs = vec![TextRun {
        text: "café".into(),
        attributes: vec![RunAttribute {
            name: "Style".into(),
            value: "Bold+VendorToken".into(),
        }],
    }];
    assert_eq!(project.validate(), Ok(()));
    project.elements[0].runs[0].text = "cafe".into();
    assert_eq!(project.validate(), Err(ModelError::InvalidRunProjection));
}

#[test]
fn alts_fixture_keeps_alternates_out_of_the_canonical_model_projection() {
    let imported = scriptcraft_core::fdx::read_v6(include_bytes!("fixtures/dialogue_alts.fdx"))
        .expect("checked-in Alts fixture imports");
    let project = imported.project;
    assert_eq!(project.validate(), Ok(()));
    assert_eq!(project.elements[0].text, "ordinary canonical");
    assert_eq!(project.elements[3].text, "member canonical");
    assert_eq!(project.dual_dialogue_groups.len(), 1);
    let serialized = scriptcraft_core::fdx::write_v6(&project).expect("Alts ownership serializes");
    let xml = String::from_utf8(serialized).expect("FDX serializer emits UTF-8");
    for marker in ["INLINE_ALT_SENTINEL", "MEMBER_ALT_SENTINEL", "COLLECTION_MEMBER_SENTINEL", "ORPHAN_SENTINEL"] {
        assert!(xml.contains(marker));
    }
}

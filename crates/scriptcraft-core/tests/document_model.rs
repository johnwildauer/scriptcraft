#![forbid(unsafe_code)]

use scriptcraft_core::dual_dialogue::{self, DualDialogueGroup, DualDialogueSide, DualDialogueWrapperProvenance, SelectionSurface, ToggleTarget};
use scriptcraft_core::editing::{Position, Selection};
use scriptcraft_core::{BodyOwner, ModelError, Project, Role, RunAttribute, ScreenplayElement, StableId, TextRun, TitlePageParagraph};
use std::ops::Range;

fn project_with_roles(roles: &[Role]) -> Project {
    let mut project = Project::new().expect("valid default project");
    project.elements = roles
        .iter()
        .enumerate()
        .map(|(index, role)| BodyOwner::shared(ScreenplayElement::new(format!("p-{index}"), *role, "").expect("valid element")))
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
    assert!(matches!(invalid.validate_current_state(), Err(ModelError::InvalidDualDialogue(_))));
    let mut bad_grammar = project_with_roles(&[Role::Character, Role::Action, Role::Character, Role::Dialogue]);
    bad_grammar.dual_dialogue_groups.push(group(&bad_grammar, "bad", 0..2, 2..4));
    assert!(matches!(bad_grammar.validate(), Err(ModelError::InvalidDualDialogue(_))));
    assert!(matches!(bad_grammar.validate_current_state(), Err(ModelError::InvalidDualDialogue(_))));

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
    duplicate_id.elements[0] = BodyOwner::shared(ScreenplayElement::new(
        duplicate_id.title_page[0].id.as_str(),
        Role::SceneHeading,
        "",
    )
    .expect("well-formed element with colliding identity"));
    assert_eq!(duplicate_id.validate(), Err(ModelError::DuplicateId));
    assert_eq!(duplicate_id.validate_current_state(), Err(ModelError::DuplicateId));
}

#[test]
fn formatted_text_projection_and_attribute_limits_are_semantic_invariants() {
    let mut project = Project::new().expect("valid project");
    BodyOwner::update(&mut project.elements[0], |element| {
        element.text = "café".into();
        element.runs = vec![TextRun {
            text: "café".into(),
            attributes: vec![RunAttribute {
                name: "Style".into(),
                value: "Bold+VendorToken".into(),
            }],
        }];
    });
    assert_eq!(project.validate(), Ok(()));
    BodyOwner::update(&mut project.elements[0], |element| {
        element.runs[0].text = "cafe".into();
    });
    assert_eq!(project.validate(), Err(ModelError::InvalidRunProjection));
}

#[test]
fn current_state_validation_keeps_canonical_failure_controls_strict() {
    let mut stale_override = valid_project_for_validation();
    stale_override.direct_paragraph_overrides.insert(
        StableId::new("deleted-owner").expect("valid stale owner ID"),
        Default::default(),
    );
    assert_eq!(
        stale_override.validate_current_state(),
        Err(ModelError::InvalidParagraphOverride),
    );

    let mut duplicate = valid_project_for_validation();
    duplicate.elements.push(duplicate.elements[0].clone());
    assert_eq!(duplicate.validate_current_state(), Err(ModelError::DuplicateId));

    let mut invalid_runs = valid_project_for_validation();
    invalid_runs.elements[0].text = "semantic text".into();
    invalid_runs.elements[0].runs = vec![TextRun {
        text: "different text".into(),
        attributes: Vec::new(),
    }];
    assert_eq!(
        invalid_runs.validate_current_state(),
        Err(ModelError::InvalidRunProjection),
    );

    let mut capacity = valid_project_for_validation();
    capacity.elements = (0..=scriptcraft_core::MAX_ELEMENTS)
        .map(|index| {
            ScreenplayElement::new(format!("capacity-{index}"), Role::Action, "")
                .expect("capacity-control element is semantically valid")
        })
        .collect();
    assert_eq!(capacity.validate_current_state(), Err(ModelError::TooManyElements));
}

#[test]
fn direct_override_validation_uses_body_identity_and_rejects_title_only_ids() {
    let mut many_overrides = Project::new().expect("default project is valid");
    many_overrides.elements = (0..512)
        .map(|index| {
            ScreenplayElement::new(format!("override-owner-{index}"), Role::Action, "")
                .expect("override owner is valid")
        })
        .collect();
    for element in &many_overrides.elements {
        many_overrides
            .direct_paragraph_overrides
            .insert(element.id.clone(), SourceParagraphOverrides::default());
    }
    assert_eq!(many_overrides.validate_current_state(), Ok(()));

    let mut title_only_override = Project::new().expect("default project is valid");
    title_only_override.direct_paragraph_overrides.insert(
        title_only_override.title_page[0].id.clone(),
        SourceParagraphOverrides::default(),
    );
    assert_eq!(
        title_only_override.validate_current_state(),
        Err(ModelError::InvalidParagraphOverride),
    );
}

#[test]
fn scoped_validation_distinguishes_owner_acceptance_fallback_and_invalid_state() {
    let (baseline, mut candidate) = source_backed_scoped_candidate();
    assert_eq!(
        candidate.validate_scoped_single_element_edit(&baseline, 0),
        Ok(ScopedEditValidationOutcome::Validated),
    );

    let (baseline, mut changed_elsewhere) = source_backed_scoped_candidate();
    let other_id = baseline.elements[1].id.clone();
    let preservation = changed_elsewhere.preservation.as_mut().expect("source is retained");
    let other = preservation.sections.iter_mut().find_map(|section| {
        find_managed_owner_mut(&mut section.children, &other_id)
    }).expect("second semantic owner is managed");
    other.attributes.push(scriptcraft_core::preservation::PreservedAttribute {
        name: SourceName {
            namespace_uri: None,
            local_name: "vendor-change".into(),
            qualified_name: Some("vendor-change".into()),
        },
        value: "forces complete validation".into(),
    });
    assert_eq!(
        changed_elsewhere.validate_scoped_single_element_edit(&baseline, 0),
        Ok(ScopedEditValidationOutcome::CompleteValidationRequired),
        "a changed non-target source owner is a fallback, not scoped success",
    );

    let (baseline, mut moved_owner) = source_backed_scoped_candidate();
    let target_id = baseline.elements[0].id.clone();
    let preservation = moved_owner.preservation.as_mut().expect("source is retained");
    assert!(preservation.sections.iter_mut().any(|section| {
        swap_managed_owner_with_next(&mut section.children, &target_id)
    }));
    preservation.rebuild_indexes();
    assert_eq!(
        moved_owner.validate_scoped_single_element_edit(&baseline, 0),
        Ok(ScopedEditValidationOutcome::CompleteValidationRequired),
        "a changed current source location requests complete validation",
    );

    let (baseline, mut missing_owner) = source_backed_scoped_candidate();
    let target_id = baseline.elements[0].id.clone();
    let preservation = missing_owner.preservation.as_mut().expect("source is retained");
    assert!(preservation.sections.iter_mut().any(|section| {
        remove_managed_owner(&mut section.children, &target_id)
    }));
    preservation.rebuild_indexes();
    assert_eq!(
        missing_owner.validate_scoped_single_element_edit(&baseline, 0),
        Ok(ScopedEditValidationOutcome::CompleteValidationRequired),
        "a missing current owner location does not become a scoped model failure",
    );

    let (baseline, mut ambiguous_owner) = source_backed_scoped_candidate();
    let target_id = baseline.elements[0].id.clone();
    let preservation = ambiguous_owner.preservation.as_mut().expect("source is retained");
    let duplicate = preservation.sections.iter()
        .find_map(|section| find_managed_owner(&section.children, &target_id))
        .expect("target owner exists");
    let content = preservation.sections.iter_mut().find_map(|section| {
        find_content_mut(&mut section.children)
    }).expect("current Content exists");
    content.children.push(PreservedNode::Element(duplicate));
    preservation.rebuild_indexes();
    assert_eq!(
        ambiguous_owner.validate_scoped_single_element_edit(&baseline, 0),
        Ok(ScopedEditValidationOutcome::CompleteValidationRequired),
        "ambiguous current locations request complete validation",
    );

    let (baseline, mut wrong_text) = source_backed_scoped_candidate();
    wrong_text.elements[0].text = "not the retained projection".into();
    wrong_text.elements[0].runs.clear();
    assert!(matches!(
        wrong_text.validate_scoped_single_element_edit(&baseline, 0),
        Err(ModelError::InvalidPreservationState(
            scriptcraft_core::preservation::PreservationError::ManagedTextMismatchAt(_)
        )),
    ));

    let (baseline, mut wrong_owner) = source_backed_scoped_candidate();
    let target_id = baseline.elements[0].id.clone();
    let wrong_id = StableId::new("not-the-semantic-owner").expect("valid comparison owner ID");
    let owner = wrong_owner.preservation.as_mut().expect("source is retained").sections
        .iter_mut().find_map(|section| find_managed_owner_mut(&mut section.children, &target_id))
        .expect("target owner exists");
    owner.ownership = SourceOwnership::Managed(wrong_id);
    assert!(matches!(
        wrong_owner.validate_scoped_single_element_edit(&baseline, 0),
        Err(ModelError::InvalidPreservationState(
            scriptcraft_core::preservation::PreservationError::UnsupportedTextMapping
        )),
    ));

    let (baseline, mut removed_attachment) = source_backed_scoped_candidate();
    let target_id = baseline.elements[0].id.clone();
    let owner = removed_attachment.preservation.as_mut().expect("source is retained").sections
        .iter_mut().find_map(|section| find_managed_owner_mut(&mut section.children, &target_id))
        .expect("target owner exists");
    owner.children.retain(|node| !matches!(node,
        PreservedNode::Element(element) if element.name.local_name == "UserParagraphData"
    ));
    assert!(matches!(
        removed_attachment.validate_scoped_single_element_edit(&baseline, 0),
        Err(ModelError::InvalidPreservationState(
            scriptcraft_core::preservation::PreservationError::StructuralAttachmentConflict
        )),
    ));

    let (baseline, mut wrong_role) = source_backed_scoped_candidate();
    let target_id = baseline.elements[0].id.clone();
    let owner = wrong_role.preservation.as_mut().expect("source is retained").sections
        .iter_mut().find_map(|section| find_managed_owner_mut(&mut section.children, &target_id))
        .expect("target owner exists");
    owner.source_type = Some("Dialogue".into());
    let role_attribute = owner.attributes.iter_mut().find(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Type"
    }).expect("managed role attribute exists");
    role_attribute.value = "Dialogue".into();
    assert!(matches!(
        wrong_role.validate_scoped_single_element_edit(&baseline, 0),
        Err(ModelError::InvalidPreservationState(
            scriptcraft_core::preservation::PreservationError::UnsupportedRoleMapping
        )),
    ));

    let (baseline, mut byte_overflow) = source_backed_scoped_candidate();
    let target_id = baseline.elements[0].id.clone();
    let owner = byte_overflow.preservation.as_mut().expect("source is retained").sections
        .iter_mut().find_map(|section| find_managed_owner_mut(&mut section.children, &target_id))
        .expect("target owner exists");
    let text = owner.children.iter_mut().find_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "Text" => Some(element),
        _ => None,
    }).expect("current Text projection exists");
    let mut opaque_payload = text.clone();
    opaque_payload.name.local_name = "OversizedPayload".into();
    opaque_payload.name.qualified_name = Some("OversizedPayload".into());
    opaque_payload.attributes.clear();
    opaque_payload.source_type = None;
    opaque_payload.ownership = SourceOwnership::Opaque;
    opaque_payload.anchor = None;
    opaque_payload.children = vec![PreservedNode::Text(
        "x".repeat(scriptcraft_core::preservation::MAX_SOURCE_TEXT_BYTES).into(),
    )].into();
    text.children.push(PreservedNode::Element(opaque_payload));
    assert!(matches!(
        byte_overflow.validate_scoped_single_element_edit(&baseline, 0),
        Err(ModelError::InvalidPreservationState(
            scriptcraft_core::preservation::PreservationError::TextTooLarge
        )),
    ));
}

fn valid_project_for_validation() -> Project {
    Project::new().expect("default project is valid")
}

fn source_backed_scoped_candidate() -> (Project, Project) {
    let imported = scriptcraft_core::fdx::read_v6(include_bytes!("fixtures/user_paragraph_data.fdx"))
        .expect("source fixture imports");
    let mut session = EditingSession::new(imported.project).expect("source project validates");
    let baseline = session.project().clone();
    let text_len = session.project().elements[0].text.len();
    session
        .set_selection(Selection::caret(Position { element: 0, offset: text_len }))
        .expect("body selection is valid");
    session.insert("!").expect("ordinary source-backed edit succeeds");
    (baseline, session.project().clone())
}

fn find_managed_owner_mut<'a>(
    nodes: &'a mut [PreservedNode],
    id: &StableId,
) -> Option<&'a mut PreservedElement> {
    for node in nodes {
        let PreservedNode::Element(element) = node else { continue };
        if matches!(&element.ownership, SourceOwnership::Managed(owner) if owner == id) {
            return Some(element);
        }
        if let Some(found) = find_managed_owner_mut(&mut element.children, id) {
            return Some(found);
        }
    }
    None
}

fn find_managed_owner(
    nodes: &[PreservedNode],
    id: &StableId,
) -> Option<PreservedElement> {
    for node in nodes {
        let PreservedNode::Element(element) = node else { continue };
        if matches!(&element.ownership, SourceOwnership::Managed(owner) if owner == id) {
            return Some(element.clone());
        }
        if let Some(found) = find_managed_owner(&element.children, id) {
            return Some(found);
        }
    }
    None
}

fn remove_managed_owner(nodes: &mut Vec<PreservedNode>, id: &StableId) -> bool {
    let mut index = 0;
    while index < nodes.len() {
        let is_target = matches!(nodes.get(index), Some(PreservedNode::Element(element))
            if matches!(&element.ownership, SourceOwnership::Managed(owner) if owner == id));
        if is_target {
            nodes.remove(index);
            return true;
        }
        if let Some(PreservedNode::Element(element)) = nodes.get_mut(index) {
            if remove_managed_owner(&mut element.children, id) {
                return true;
            }
        }
        index += 1;
    }
    false
}

fn swap_managed_owner_with_next(nodes: &mut [PreservedNode], id: &StableId) -> bool {
    for index in 0..nodes.len() {
        let is_target = matches!(nodes.get(index), Some(PreservedNode::Element(element))
            if matches!(&element.ownership, SourceOwnership::Managed(owner) if owner == id));
        if is_target {
            if index + 1 < nodes.len() {
                nodes.swap(index, index + 1);
                return true;
            }
            return false;
        }
        if let Some(PreservedNode::Element(element)) = nodes.get_mut(index) {
            if swap_managed_owner_with_next(&mut element.children, id) {
                return true;
            }
        }
    }
    false
}

fn find_content_mut(nodes: &mut [PreservedNode]) -> Option<&mut PreservedElement> {
    for node in nodes {
        let PreservedNode::Element(element) = node else { continue };
        if element.name.local_name == "Content" {
            return Some(element);
        }
        if let Some(content) = find_content_mut(&mut element.children) {
            return Some(content);
        }
    }
    None
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

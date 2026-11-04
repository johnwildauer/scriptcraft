use scriptcraft_core::editing::{EditError, EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::preservation::{
    ForeignIdDomain, ForeignIdentity, ForeignReference, PreservedElement, PreservedNode,
    PreservationError, ReferencePolicy,
};
use scriptcraft_core::cast_list::correct_cast_lists;
use scriptcraft_core::{Project, Role};

const EMPTY_ARC_SOURCE: &[u8] = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Action" id="before"><Text>before</Text></Paragraph>
<Paragraph Type="Scene Heading" id="heading"><SceneProperties Length="1/8" Page="1" Title=""><SceneArcBeats/></SceneProperties></Paragraph>
<Paragraph Type="Action" id="after"><Text>after</Text></Paragraph>
</Content></FinalDraft>"#;

const NONEMPTY_ARC_SOURCE: &[u8] = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Action" id="before"><Text>before</Text></Paragraph>
<Paragraph Type="Scene Heading" id="heading"><SceneProperties Length="1/8" Page="1" Title=""><SceneArcBeats>
<CharacterArcBeat Name="ALPHA"><Paragraph Alignment="Left" id="arc-alpha"><Text Style="Bold">alpha </Text><Text Style="Italic">arc</Text></Paragraph></CharacterArcBeat>
</SceneArcBeats></SceneProperties><Text>INT. ROOM - DAY</Text></Paragraph>
<Paragraph Type="Character" id="cue-alpha"><Text>ALPHA</Text></Paragraph>
<Paragraph Type="Action" id="after"><Text>after</Text></Paragraph>
</Content></FinalDraft>"#;

fn imported(source: &[u8]) -> Project {
    read_v6(source)
        .unwrap_or_else(|error| panic!("WO-005 fixture import failed: {error:?}"))
        .project
}

fn deletion_selection(project: &Project) -> Selection {
    let heading_index = project
        .elements
        .iter()
        .position(|element| element.role == Role::SceneHeading)
        .unwrap_or_else(|| unreachable!("fixture has a Scene Heading"));
    let heading = &project.elements[heading_index];
    Selection {
        anchor: Position {
            element: heading_index - 1,
            offset: project.elements[heading_index - 1].text.len(),
        },
        focus: Position {
            element: heading_index,
            offset: heading.text.len(),
        },
    }
}

fn paragraph_identity_values(project: &Project) -> Vec<String> {
    project
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("fixture retains source preservation"))
        .identities
        .iter()
        .filter(|identity| identity.domain == ForeignIdDomain::Paragraph)
        .map(|identity| identity.value.clone())
        .collect()
}

fn source_paragraph_by_id<'a>(project: &'a Project, id: &str) -> Option<&'a PreservedElement> {
    fn find<'a>(nodes: &'a [PreservedNode], id: &str) -> Option<&'a PreservedElement> {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if element.name.local_name == "Paragraph"
                && element.attributes.iter().any(|attribute| {
                    attribute.name.local_name.eq_ignore_ascii_case("id")
                        && attribute.value == id
                })
            {
                return Some(element);
            }
            if let Some(found) = find(&element.children, id) {
                return Some(found);
            }
        }
        None
    }

    project.preservation.as_ref()?.sections.iter().find_map(|section| {
        find(&section.children, id)
    })
}

fn assert_selection_deletion_round_trip(source: &[u8], expected_arc_id: Option<&str>) {
    let project = imported(source);
    let mut session = EditingSession::new(project)
        .unwrap_or_else(|_| unreachable!("imported project forms an editing session"));
    let original = session.project().clone();
    let mut expected_undo = original.clone();
    correct_cast_lists(&mut expected_undo)
        .unwrap_or_else(|error| panic!("WO-005 expected derived undo correction failed: {error:?}"));
    let original_paragraph_ids = paragraph_identity_values(&original);
    assert_eq!(
        expected_arc_id.is_some(),
        original_paragraph_ids
            .iter()
            .any(|identity| Some(identity.as_str()) == expected_arc_id)
    );
    session
        .set_selection(deletion_selection(session.project()))
        .unwrap_or_else(|_| unreachable!("cross-paragraph selection is valid"));
    assert_eq!(session.delete(), Ok(()));

    let deleted = session.project().clone();
    assert!(deleted
        .elements
        .iter()
        .all(|element| element.role != Role::SceneHeading));
    let deleted_ids = paragraph_identity_values(&deleted);
    assert!(!deleted_ids.iter().any(|identity| identity == "heading"));
    if let Some(arc_id) = expected_arc_id {
        assert!(!deleted_ids.iter().any(|identity| identity == arc_id));
    }
    assert_eq!(deleted.elements[0].text, "before");
    if expected_arc_id.is_some() {
        assert_eq!(deleted.elements.len(), 3);
        assert_eq!(deleted.elements[1].role, Role::Character);
        assert_eq!(deleted.elements[1].text, "ALPHA");
        assert_eq!(deleted.elements[2].text, "after");
        assert!(deleted_ids.iter().any(|identity| identity == "cue-alpha"));
    } else {
        assert_eq!(deleted.elements.len(), 2);
        assert_eq!(deleted.elements[1].text, "after");
    }

    assert_eq!(session.undo(), Ok(()));
    assert_eq!(
        session.project(),
        &expected_undo,
        "undo restores the source snapshot after its scheduled derived correction"
    );
    assert_eq!(
        session.project().elements,
        original.elements,
        "undo retains the original semantic elements and IDs"
    );
    assert_eq!(paragraph_identity_values(session.project()), original_paragraph_ids);
    if let Some(arc_id) = expected_arc_id {
        assert_eq!(
            source_paragraph_by_id(session.project(), arc_id),
            source_paragraph_by_id(&original, arc_id),
            "undo retains the nested arc source subtree and run content"
        );
    }
    assert_eq!(session.redo(), Ok(()));
    assert_eq!(session.project(), &deleted, "redo repeats the same identity cleanup");

    let serialized = write_v6(session.project())
        .unwrap_or_else(|_| unreachable!("deleted project serializes"));
    let reopened = read_v6(&serialized)
        .unwrap_or_else(|_| unreachable!("serialized deletion reopens"))
        .project;
    assert!(reopened
        .elements
        .iter()
        .all(|element| element.role != Role::SceneHeading));
    let reopened_ids = paragraph_identity_values(&reopened);
    assert!(!reopened_ids.iter().any(|identity| identity == "heading"));
    if expected_arc_id.is_some() {
        assert!(reopened_ids.iter().any(|identity| identity == "cue-alpha"));
    }
    if let Some(arc_id) = expected_arc_id {
        assert!(!reopened_ids.iter().any(|identity| identity == arc_id));
    }
}

#[test]
fn bounded_selection_deletes_empty_and_supported_nonempty_generated_scene_subtrees() {
    assert_selection_deletion_round_trip(EMPTY_ARC_SOURCE, None);
    assert_selection_deletion_round_trip(NONEMPTY_ARC_SOURCE, Some("arc-alpha"));
}

fn assert_rejected_without_publication(mut session: EditingSession, expected: PreservationError) {
    session
        .set_selection(deletion_selection(session.project()))
        .unwrap_or_else(|_| unreachable!("safe-stop selection is valid"));
    let before = session.clone();
    assert_eq!(session.delete(), Err(EditError::Preservation(expected)));
    assert_eq!(session, before, "safe-stop leaves project, selection, history and dirty state intact");
}

#[test]
fn unknown_duplicate_referenced_and_unregistered_scene_data_safe_stop() {
    let with_unknown_attachment = String::from_utf8(EMPTY_ARC_SOURCE.to_vec())
        .unwrap_or_else(|_| unreachable!())
        .replace(
            "<SceneProperties Length=\"1/8\" Page=\"1\" Title=\"\"><SceneArcBeats/></SceneProperties>",
            "<SceneProperties Length=\"1/8\" Page=\"1\" Title=\"\"><SceneArcBeats/></SceneProperties><VendorAttachment/>",
        );
    assert_rejected_without_publication(
        EditingSession::new(imported(with_unknown_attachment.as_bytes()))
            .unwrap_or_else(|_| unreachable!("unknown attachment fixture imports")),
        PreservationError::StructuralAttachmentConflict,
    );

    let duplicate_beat = String::from_utf8(NONEMPTY_ARC_SOURCE.to_vec())
        .unwrap_or_else(|_| unreachable!())
        .replace(
            "</SceneArcBeats>",
            "<CharacterArcBeat Name=\"ALPHA\"><Paragraph id=\"arc-duplicate\"><Text>duplicate</Text></Paragraph></CharacterArcBeat></SceneArcBeats>",
        );
    assert_rejected_without_publication(
        EditingSession::new(imported(duplicate_beat.as_bytes()))
            .unwrap_or_else(|_| unreachable!("duplicate-beat fixture imports")),
        PreservationError::StructuralAttachmentConflict,
    );

    let mut externally_referenced = imported(NONEMPTY_ARC_SOURCE);
    let state = externally_referenced
        .preservation
        .as_mut()
        .unwrap_or_else(|| unreachable!("source preservation exists"));
    state.references.push(ForeignReference {
        source: ForeignIdentity::new(ForeignIdDomain::Paragraph, "before")
            .unwrap_or_else(|_| unreachable!("reference source identity is valid")),
        target: ForeignIdentity::new(ForeignIdDomain::Paragraph, "arc-alpha")
            .unwrap_or_else(|_| unreachable!("reference target identity is valid")),
        policy: ReferencePolicy::RequiredInternal,
    });
    state.rebuild_indexes();
    assert_rejected_without_publication(
        EditingSession::new(externally_referenced)
            .unwrap_or_else(|_| unreachable!("referenced fixture forms a session")),
        PreservationError::PotentialReferenceConflict,
    );

    let mut unregistered = imported(NONEMPTY_ARC_SOURCE);
    let state = unregistered
        .preservation
        .as_mut()
        .unwrap_or_else(|| unreachable!("source preservation exists"));
    state.identities.retain(|identity| {
        !(identity.domain == ForeignIdDomain::Paragraph && identity.value == "arc-alpha")
    });
    state.rebuild_indexes();
    assert_rejected_without_publication(
        EditingSession::new(unregistered)
            .unwrap_or_else(|_| unreachable!("unregistered-identity fixture forms a session")),
        PreservationError::PotentialReferenceConflict,
    );
}

#[test]
fn backspace_join_and_split_do_not_gain_generated_heading_deletion_permission() {
    let project = imported(EMPTY_ARC_SOURCE);
    let mut selected_backspace = EditingSession::new(project.clone())
        .unwrap_or_else(|_| unreachable!("fixture forms a session"));
    selected_backspace
        .set_selection(deletion_selection(selected_backspace.project()))
        .unwrap_or_else(|_| unreachable!("cross-paragraph selection is valid"));
    let before_backspace = selected_backspace.clone();
    assert_eq!(
        selected_backspace.backspace(),
        Err(EditError::Preservation(
            PreservationError::StructuralAttachmentConflict
        ))
    );
    assert_eq!(selected_backspace, before_backspace);

    let mut join = EditingSession::new(project.clone())
        .unwrap_or_else(|_| unreachable!("fixture forms a session"));
    join.set_selection(Selection::caret(Position {
        element: 0,
        offset: join.project().elements[0].text.len(),
    }))
    .unwrap_or_else(|_| unreachable!("join caret is valid"));
    let before_join = join.clone();
    assert_eq!(
        join.join_with_next(),
        Err(EditError::Preservation(
            PreservationError::StructuralAttachmentConflict
        ))
    );
    assert_eq!(join, before_join);

    let mut split = EditingSession::new(imported(NONEMPTY_ARC_SOURCE))
        .unwrap_or_else(|_| unreachable!("fixture forms a session"));
    split.set_selection(Selection::caret(Position {
        element: 1,
        offset: 4,
    }))
    .unwrap_or_else(|_| unreachable!("heading caret is valid"));
    let before_split = split.clone();
    assert_eq!(
        split.split(),
        Err(EditError::Preservation(
            PreservationError::StructuralAttachmentConflict
        ))
    );
    assert_eq!(split, before_split);

    let mut role_change = EditingSession::new(imported(NONEMPTY_ARC_SOURCE))
        .unwrap_or_else(|_| unreachable!("fixture forms a session"));
    role_change
        .set_selection(Selection::caret(Position {
            element: 1,
            offset: 0,
        }))
        .unwrap_or_else(|_| unreachable!("heading caret is valid"));
    let before_role_change = role_change.clone();
    assert_eq!(
        role_change.change_role(Role::Action),
        Err(EditError::Preservation(
            PreservationError::StructuralAttachmentConflict
        ))
    );
    assert_eq!(role_change, before_role_change);
}

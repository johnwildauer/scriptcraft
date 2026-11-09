#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, FdxDocument};
use scriptcraft_core::formatted_text::RunStyle;
use scriptcraft_core::formatting::ParagraphAlignment;
use scriptcraft_core::preservation::{
    AnchorRelation, AnchorTarget, ForeignIdDomain, ForeignIdentity, ForeignReference,
    PreservedElement, PreservedNode, ReferencePolicy, SourceAnchor, SourceAnchorId,
    SourceOwnership,
};
use scriptcraft_core::{Project, Role, ScreenplayElement};

fn ungrouped_project() -> Option<Project> {
    let mut project = Project::new().ok()?;
    project.elements = [
        ("cue-left", Role::Character, "LEFT"),
        ("dialogue-left", Role::Dialogue, "left text"),
        ("cue-right", Role::Character, "RIGHT"),
        ("dialogue-right", Role::Dialogue, "right text"),
    ]
    .into_iter()
    .map(|(id, role, text)| ScreenplayElement::new(id, role, text).ok())
    .collect::<Option<Vec<_>>>()?;
    Some(project)
}

fn grouped_project() -> Option<Project> {
    let project = ungrouped_project()?;
    let mut session = EditingSession::new(project).ok()?;
    session.set_selection(Selection::caret(Position { element: 0, offset: 0 })).ok()?;
    session.toggle_dual_dialogue().ok()?;
    Some(session.project().clone())
}

fn import(source: &str) -> Option<FdxDocument> {
    read_v6(source.as_bytes()).ok()
}

fn count_named(nodes: &[PreservedNode], name: &str) -> usize {
    nodes.iter().map(|node| match node {
        PreservedNode::Element(element) => {
            usize::from(element.name.local_name == name) + count_named(&element.children, name)
        }
        _ => 0,
    }).sum()
}

fn find_wrapper<'a>(nodes: &'a [PreservedNode], output: &mut Vec<&'a PreservedElement>) {
    for node in nodes {
        if let PreservedNode::Element(element) = node {
            if matches!(&element.ownership, SourceOwnership::DualDialogueWrapper(_)) {
                output.push(element);
            }
            find_wrapper(&element.children, output);
        }
    }
}

fn find_managed<'a>(nodes: &'a [PreservedNode], id: &str) -> Option<&'a PreservedElement> {
    for node in nodes {
        let PreservedNode::Element(element) = node else { continue };
        if matches!(&element.ownership, SourceOwnership::Managed(owner) if owner.as_str() == id) {
            return Some(element);
        }
        if let Some(found) = find_managed(&element.children, id) {
            return Some(found);
        }
    }
    None
}

fn find_wrapper_owner<'a>(nodes: &'a [PreservedNode], id: &scriptcraft_core::StableId) -> Option<&'a PreservedElement> {
    for node in nodes {
        let PreservedNode::Element(element) = node else { continue };
        if matches!(&element.ownership, SourceOwnership::DualDialogueWrapper(owner) if owner == id) {
            return Some(element);
        }
        if let Some(found) = find_wrapper_owner(&element.children, id) {
            return Some(found);
        }
    }
    None
}

fn find_named<'a>(nodes: &'a [PreservedNode], name: &str) -> Option<&'a PreservedElement> {
    for node in nodes {
        let PreservedNode::Element(element) = node else { continue };
        if element.name.local_name == name {
            return Some(element);
        }
        if let Some(found) = find_named(&element.children, name) {
            return Some(found);
        }
    }
    None
}

fn nested_spell_payload(project: &Project) -> Option<PreservedElement> {
    let group = project.dual_dialogue_groups.first()?;
    let wrapper = project.preservation.as_ref()
        .and_then(|preservation| find_wrapper_owner(&preservation.sections[0].children, &group.id))?;
    find_named(&wrapper.children, "SpellCheckIgnoreLists").cloned()
}

fn add_managed_attachment(nodes: &mut [PreservedNode], id: &str) -> bool {
    for node in nodes {
        let PreservedNode::Element(element) = node else { continue };
        if matches!(&element.ownership, SourceOwnership::Managed(owner) if owner.as_str() == id) {
            let Some(PreservedNode::Element(mut attachment)) = element.children.iter().find(|child| {
                matches!(child, PreservedNode::Element(text) if text.name.local_name == "Text")
            }).cloned() else { return false };
            attachment.name.local_name = "VendorAttachment".to_owned();
            attachment.ownership = SourceOwnership::Opaque;
            element.children.push(PreservedNode::Element(attachment));
            return true;
        }
        if add_managed_attachment(&mut element.children, id) {
            return true;
        }
    }
    false
}

#[test]
fn enable_disable_preserves_flat_semantics_selection_and_one_step_history() {
    let Some(project) = ungrouped_project() else { return };
    let Ok(mut session) = EditingSession::new(project.clone()) else { return };
    let selection = Selection::caret(Position { element: 0, offset: 0 });
    assert!(session.set_selection(selection).is_ok());

    assert!(session.dual_dialogue_toggle_target(
        scriptcraft_core::dual_dialogue::SelectionSurface::Body,
    ).is_ok_and(|target| target.is_some()));
    assert!(session.toggle_dual_dialogue().is_ok());
    assert_eq!(session.project().elements, project.elements);
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
    assert_eq!(session.selection(), selection);
    assert!(session.can_undo());
    assert!(session.undo().is_ok());
    assert_eq!(session.project(), &project);
    assert_eq!(session.selection(), selection);
    assert!(session.can_redo());
    assert!(session.redo().is_ok());
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);

    let Ok(mut session) = EditingSession::new(session.project().clone()) else { return };
    let disable_caret = Selection::caret(Position { element: 3, offset: 2 });
    assert!(session.set_selection(disable_caret).is_ok());
    let grouped = session.project().clone();
    assert!(session.toggle_dual_dialogue().is_ok());
    assert!(session.project().dual_dialogue_groups.is_empty());
    assert_eq!(session.project().elements, grouped.elements);
    assert_eq!(session.selection(), disable_caret);
    assert!(session.undo().is_ok());
    assert_eq!(session.project(), &grouped);
}

#[test]
fn ineligible_selections_are_exact_session_noops() {
    let Some(project) = ungrouped_project() else { return };
    let Ok(mut session) = EditingSession::new(project) else { return };
    let ranged = Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 1, offset: 0 },
    };
    assert!(session.set_selection(ranged).is_ok());
    let before = session.clone();
    assert!(session.toggle_dual_dialogue().is_ok());
    assert_eq!(session, before);

    let collapsed_wrong_role = Selection::caret(Position { element: 1, offset: 0 });
    assert!(session.set_selection(collapsed_wrong_role).is_ok());
    let before = session.clone();
    assert!(session.toggle_dual_dialogue().is_ok());
    assert_eq!(session, before);

    assert!(session.dual_dialogue_toggle_target(
        scriptcraft_core::dual_dialogue::SelectionSurface::TitlePage,
    ).is_ok_and(|target| target.is_none()));
}

#[test]
fn invalid_side_patterns_are_consumed_without_history_or_project_changes() {
    let invalid_role_patterns = [
        vec![Role::Character, Role::Character, Role::Dialogue],
        vec![Role::Character, Role::Dialogue, Role::Character, Role::Action],
        vec![Role::Character, Role::Action, Role::Character, Role::Dialogue],
    ];
    for roles in invalid_role_patterns {
        let Some(mut project) = Project::new().ok() else { return };
        let elements = roles.into_iter().enumerate().map(|(index, role)| {
            ScreenplayElement::new(format!("invalid-{index}"), role, "").ok()
        }).collect::<Option<Vec<_>>>();
        let Some(elements) = elements else { return };
        project.elements = elements;
        let Ok(mut session) = EditingSession::new(project) else { return };
        assert!(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })).is_ok());
        let before = session.clone();
        assert!(session.toggle_dual_dialogue().is_ok());
        assert_eq!(session, before);
        assert!(!session.can_undo());
    }
}

#[test]
fn adjacent_preservation_free_groups_toggle_independently_and_keep_members_flat() {
    let mut project = Project::new().ok();
    let Some(mut project) = project.take() else { return };
    let roles = [
        Role::Character, Role::Dialogue, Role::Character, Role::Parenthetical,
        Role::Character, Role::Dialogue, Role::Character, Role::Dialogue,
    ];
    let elements = roles.into_iter().enumerate().map(|(index, role)| {
        ScreenplayElement::new(format!("adjacent-{index}"), role, format!("text-{index}"))
    }).collect::<Result<Vec<_>, _>>();
    let Ok(elements) = elements else { return };
    project.elements = elements;
    let original_elements = project.elements.clone();
    let Ok(mut session) = EditingSession::new(project) else { return };

    for index in [0, 4] {
        assert!(session.set_selection(Selection::caret(Position { element: index, offset: 0 })).is_ok());
        assert!(session.dual_dialogue_toggle_target(
            scriptcraft_core::dual_dialogue::SelectionSurface::Body,
        ).is_ok_and(|target| target.is_some()));
        assert!(session.toggle_dual_dialogue().is_ok());
    }
    assert_eq!(session.project().dual_dialogue_groups.len(), 2);
    assert_eq!(session.project().elements, original_elements);
    assert!(session.project().preservation.is_none());
    let unrelated = session.project().dual_dialogue_groups[1].clone();
    assert!(session.set_selection(Selection::caret(Position { element: 1, offset: 3 })).is_ok());
    assert!(session.toggle_dual_dialogue().is_ok());
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
    assert_eq!(session.project().dual_dialogue_groups[0], unrelated);
    assert_eq!(session.project().elements, original_elements);
}

#[test]
fn disable_resolves_every_member_and_preserves_each_caret_offset() {
    let Some(project) = ungrouped_project() else { return };
    let Ok(mut source) = EditingSession::new(project) else { return };
    assert!(source.set_selection(Selection::caret(Position { element: 0, offset: 0 })).is_ok());
    assert!(source.toggle_dual_dialogue().is_ok());
    let grouped = source.project().clone();
    for (element, offset) in [(0, 2), (1, 3), (2, 1), (3, 4)] {
        let Ok(mut session) = EditingSession::new(grouped.clone()) else { return };
        let selection = Selection::caret(Position { element, offset });
        assert!(session.set_selection(selection).is_ok());
        assert!(session.toggle_dual_dialogue().is_ok());
        assert!(session.project().dual_dialogue_groups.is_empty());
        assert_eq!(session.project().elements, grouped.elements);
        assert_eq!(session.selection(), selection);
    }
}

#[test]
fn source_backed_disable_removes_only_wrapper_and_nested_spell_data() {
    let Some(imported) = import(include_str!("fixtures/dual_dialogue_basic.fdx")) else { return };
    let original = imported.project.clone();
    let Ok(mut session) = EditingSession::new(original.clone()) else { return };
    let caret = Selection::caret(Position { element: 3, offset: 5 });
    assert!(session.set_selection(caret).is_ok());
    assert!(session.toggle_dual_dialogue().is_ok());
    assert_eq!(session.project().dual_dialogue_groups.len(), 0);
    assert_eq!(session.project().elements, original.elements);
    assert!(session.selection() == caret);
    let Some(preservation) = session.project().preservation.as_ref() else { return };
    let content = preservation.sections.iter().flat_map(|section| section.children.iter())
        .filter_map(|node| match node {
            PreservedNode::Element(root) if root.name.local_name == "FinalDraft" => Some(root),
            _ => None,
        })
        .flat_map(|root| root.children.iter())
        .filter_map(|node| match node {
            PreservedNode::Element(content) if content.name.local_name == "Content" => Some(content),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(content.iter().map(|element| count_named(&element.children, "Paragraph")).sum::<usize>(), original.elements.len());
    assert_eq!(count_named(&preservation.sections[0].children, "SpellCheckIgnoreLists"), 1);
    let mut wrappers = Vec::new();
    find_wrapper(&preservation.sections[0].children, &mut wrappers);
    assert!(wrappers.is_empty());

    assert!(session.undo().is_ok());
    assert_eq!(session.project(), &original);
    assert!(session.redo().is_ok());
    assert_eq!(session.project().dual_dialogue_groups.len(), 0);
}

#[test]
fn source_backed_enable_safe_stops_opaque_interleaving_without_publishing() {
    let source = "<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content>\
        <Paragraph Type=\"Character\" id=\"lc\"><Text>Left</Text></Paragraph>\
        <Paragraph Type=\"Dialogue\" id=\"ld\"><Text>left</Text></Paragraph>\
        <Metadata vendor=\"kept\"/>\
        <Paragraph Type=\"Character\" id=\"rc\"><Text>Right</Text></Paragraph>\
        <Paragraph Type=\"Dialogue\" id=\"rd\"><Text>right</Text></Paragraph>\
    </Content><SpellCheckIgnoreLists><IgnoredRanges/><IgnoredWords/></SpellCheckIgnoreLists></FinalDraft>";
    let Some(imported) = import(source) else { return };
    let Ok(mut session) = EditingSession::new(imported.project) else { return };
    let selection = Selection::caret(Position { element: 0, offset: 0 });
    assert!(session.set_selection(selection).is_ok());
    let before = session.clone();
    assert!(session.toggle_dual_dialogue().is_err());
    assert_eq!(session, before);
}

#[test]
fn nested_split_preserves_ids_runs_utf8_source_siblings_and_history() {
    let Some(imported) = import(include_str!("fixtures/dual_dialogue_basic.fdx")) else { return };
    let original = imported.project.clone();
    let left_id = original.elements[2].id.clone();
    let old_foreign = original.elements[2].source_provenance.as_ref()
        .and_then(|provenance| provenance.foreign_id.clone());
    let split = "First half—".len();
    let Ok(mut session) = EditingSession::new(original.clone()) else { return };
    let selection = Selection::caret(Position { element: 2, offset: split });
    assert!(session.set_selection(selection).is_ok());
    let revision = session.revision();

    assert!(session.split().is_ok());
    assert_eq!(session.project().elements.len(), original.elements.len() + 1);
    assert_eq!(session.project().elements[2].id, left_id);
    assert_eq!(session.project().elements[2].text, "First half—");
    assert_eq!(session.project().elements[3].text, "then the rest.");
    assert_eq!(session.project().elements[2].runs.len(), 1);
    assert_eq!(session.project().elements[2].runs[0].text, "First half—");
    assert_eq!(session.project().elements[3].runs.len(), 1);
    assert_eq!(session.project().elements[3].runs[0].text, "then the rest.");
    assert_ne!(session.project().elements[3].id, left_id);
    assert_eq!(session.project().elements[2].source_provenance.as_ref()
        .and_then(|provenance| provenance.foreign_id.as_deref()), old_foreign.as_deref());
    assert!(session.project().elements[3].source_provenance.as_ref()
        .and_then(|provenance| provenance.foreign_id.as_deref()).is_some_and(|id| Some(id) != old_foreign.as_deref()));
    assert_eq!(session.project().dual_dialogue_groups[0].left.paragraphs.len(),
        original.dual_dialogue_groups[0].left.paragraphs.len() + 1);
    assert!(session.project().dual_dialogue_groups[0].left.paragraphs[1]
        == session.project().elements[3].id);
    let Some(preservation) = session.project().preservation.as_ref() else { return };
    let Some(new_source) = find_managed(&preservation.sections[0].children,
        session.project().elements[3].id.as_str()) else { return };
    assert!(new_source.attributes.iter().all(|attribute|
        matches!(attribute.name.local_name.as_str(), "Type" | "id" | "ID")));
    assert!(preservation.anchors.iter().any(|anchor|
        matches!(&anchor.target, AnchorTarget::ProjectElement(id) if id == &session.project().elements[3].id)));
    assert_eq!(preservation.identities.iter().filter(|identity|
        identity.domain == ForeignIdDomain::Paragraph
            && session.project().elements[3].source_provenance.as_ref()
                .and_then(|provenance| provenance.foreign_id.as_deref()) == Some(identity.value.as_str())
    ).count(), 1);
    assert_eq!(session.selection(), Selection::caret(Position { element: 3, offset: 0 }));
    assert!(session.dirty());
    assert_eq!(session.revision(), revision.saturating_add(1));
    let split_project = session.project().clone();
    assert!(session.undo().is_ok());
    assert_eq!(session.project(), &original);
    assert_eq!(session.selection(), selection);
    assert!(session.redo().is_ok());
    assert_eq!(session.project(), &split_project);

    let Ok(mut right_split) = EditingSession::new(original.clone()) else { return };
    let unicode_boundary = "你".len();
    assert!(right_split.set_selection(Selection::caret(Position { element: 5, offset: unicode_boundary })).is_ok());
    assert!(right_split.split().is_ok());
    assert_eq!(right_split.project().elements[5].text, "你");
    assert_eq!(right_split.project().elements[6].text, "好, world.");
    assert_eq!(right_split.project().dual_dialogue_groups[0].right.paragraphs.len(),
        original.dual_dialogue_groups[0].right.paragraphs.len() + 1);
    assert!(right_split.project().elements[6].text.is_char_boundary(3));

    let Ok(mut ascii_split) = EditingSession::new(original.clone()) else { return };
    assert!(ascii_split.set_selection(Selection::caret(Position { element: 2, offset: "First".len() })).is_ok());
    assert!(ascii_split.split().is_ok());
    assert_eq!(ascii_split.project().elements[2].text, "First");
    assert_eq!(ascii_split.project().elements[3].text, " half—then the rest.");
}

#[test]
fn nested_dialogue_and_generated_parenthetical_insertions_stay_on_the_selected_side() {
    let Some(imported) = import(include_str!("fixtures/dual_dialogue_basic.fdx")) else { return };
    let Ok(mut generated) = EditingSession::with_policy(
        imported.project.clone(),
        scriptcraft_core::editing::EditPolicy {
            auto_parenthetical: true,
            ..scriptcraft_core::editing::EditPolicy::default()
        },
    ) else { return };
    let old_left = generated.project().dual_dialogue_groups[0].left.paragraphs.clone();
    assert!(generated.set_selection(Selection::caret(Position { element: 3, offset: 0 })).is_ok());
    assert!(generated.enter().is_ok());
    let inserted_id = generated.project().elements[3].id.clone();
    assert_eq!(generated.project().elements[3].role, Role::Parenthetical);
    assert_eq!(generated.project().elements[3].text, "()");
    assert_eq!(generated.project().dual_dialogue_groups[0].left.paragraphs[0], inserted_id);
    assert_eq!(&generated.project().dual_dialogue_groups[0].left.paragraphs[1..], old_left.as_slice());
    let Some(preservation) = generated.project().preservation.as_ref() else { return };
    assert!(find_managed(&preservation.sections[0].children, inserted_id.as_str()).is_some());
    let inserted_snapshot = generated.project().clone();
    assert!(generated.undo().is_ok());
    assert!(generated.redo().is_ok());
    assert_eq!(generated.project(), &inserted_snapshot);
    assert!(generated.set_selection(Selection::caret(Position { element: 3, offset: 1 })).is_ok());
    assert!(generated.enter().is_ok());
    assert_eq!(generated.project().elements[3].text, "");

    let malformed_closer = include_str!("fixtures/dual_dialogue_basic.fdx").replace("(quietly)", "quietly)");
    let Some(imported) = import(&malformed_closer) else { return };
    let mut dialogue_session = EditingSession::new(imported.project.clone());
    let Ok(ref mut dialogue_session) = dialogue_session else { return };
    let parenthetical_index = 3;
    assert!(dialogue_session.set_selection(Selection::caret(Position {
        element: parenthetical_index,
        offset: "quietly)".len(),
    })).is_ok());
    assert!(dialogue_session.enter().is_ok());
    let group = &dialogue_session.project().dual_dialogue_groups[0];
    assert_eq!(group.left.paragraphs.len(), 3);
    assert_eq!(dialogue_session.project().elements[parenthetical_index].text, "(quietly)");
    assert_eq!(dialogue_session.project().elements[parenthetical_index + 1].role, Role::Dialogue);
    let Some(preservation) = dialogue_session.project().preservation.as_ref() else { return };
    assert!(find_managed(&preservation.sections[0].children,
        dialogue_session.project().elements[parenthetical_index + 1].id.as_str()).is_some());

    let right_dialogue_source = include_str!("fixtures/dual_dialogue_basic.fdx").replace(
        "<Paragraph Type=\"Parenthetical\" id=\"right-parenthetical-001\"/>",
        "<Paragraph Type=\"Parenthetical\" id=\"right-parenthetical-001\"><Text>quietly)</Text></Paragraph>",
    );
    let Some(imported) = import(&right_dialogue_source) else { return };
    let Ok(mut right_dialogue) = EditingSession::new(imported.project) else { return };
    assert!(right_dialogue.set_selection(Selection::caret(Position { element: 6, offset: "quietly)".len() })).is_ok());
    assert!(right_dialogue.enter().is_ok());
    assert_eq!(right_dialogue.project().dual_dialogue_groups[0].right.paragraphs.len(), 3);
    assert_eq!(right_dialogue.project().elements[7].role, Role::Dialogue);
}

#[test]
fn nested_same_side_join_and_member_deletion_update_only_group_members_and_history() {
    let Some(imported) = import(include_str!("fixtures/dual_dialogue_basic.fdx")) else { return };
    let original = imported.project.clone();
    let removed_parenthetical = original.elements[3].id.clone();
    let Ok(mut joined) = EditingSession::new(original.clone()) else { return };
    assert!(joined.set_selection(Selection::caret(Position {
        element: 2,
        offset: original.elements[2].text.len(),
    })).is_ok());
    assert!(joined.join_with_next().is_ok());
    assert_eq!(joined.project().elements[2].text, "First half—then the rest.(quietly)");
    assert!(!joined.project().dual_dialogue_groups[0].left.paragraphs.contains(&removed_parenthetical));
    let Some(preservation) = joined.project().preservation.as_ref() else { return };
    assert!(!preservation.identities.iter().any(|identity| identity.domain == ForeignIdDomain::Paragraph
        && original.elements[3].source_provenance.as_ref().and_then(|item| item.foreign_id.as_deref()) == Some(identity.value.as_str())));
    assert!(!preservation.anchors.iter().any(|anchor|
        matches!(&anchor.target, AnchorTarget::ProjectElement(id) if id == &removed_parenthetical)));
    let joined_project = joined.project().clone();
    assert!(joined.undo().is_ok());
    assert_eq!(joined.project(), &original);
    assert!(joined.redo().is_ok());
    assert_eq!(joined.project(), &joined_project);

    let Ok(mut deleted) = EditingSession::new(original.clone()) else { return };
    let right_parenthetical = original.elements[6].id.clone();
    let right_text_end = original.elements[5].text.len();
    assert!(deleted.set_selection(Selection {
        anchor: Position { element: 5, offset: right_text_end },
        focus: Position { element: 6, offset: 0 },
    }).is_ok());
    assert!(deleted.delete().is_ok());
    assert_eq!(deleted.project().dual_dialogue_groups[0].right.paragraphs.len(), 1);
    assert!(!deleted.project().dual_dialogue_groups[0].member_ids().any(|id| id == &right_parenthetical));
    let Some(preservation) = deleted.project().preservation.as_ref() else { return };
    assert!(find_managed(&preservation.sections[0].children, right_parenthetical.as_str()).is_none());
    assert!(!preservation.anchors.iter().any(|anchor|
        matches!(&anchor.target, AnchorTarget::ProjectElement(id) if id == &right_parenthetical)));
    assert!(deleted.undo().is_ok());
    assert_eq!(deleted.project(), &original);
}

#[test]
fn nested_cue_boundary_formatting_attachment_and_reference_safe_stops_are_transactional() {
    let formatted = include_str!("fixtures/dual_dialogue_basic.fdx")
        .replace("id=\"left-parenthetical-001\"", "id=\"left-parenthetical-001\" LeftIndent=\"1.00\"");
    let Some(imported) = import(&formatted) else { return };
    let Ok(mut formatted_session) = EditingSession::new(imported.project) else { return };
    assert!(formatted_session.set_selection(Selection::caret(Position {
        element: 2,
        offset: formatted_session.project().elements[2].text.len(),
    })).is_ok());
    let before_formatting_join = formatted_session.clone();
    assert!(formatted_session.join_with_next().is_err());
    assert_eq!(formatted_session, before_formatting_join);

    let Some(imported) = import(include_str!("fixtures/dual_dialogue_basic.fdx")) else { return };
    let Ok(mut boundary) = EditingSession::new(imported.project) else { return };
    assert!(boundary.set_selection(Selection::caret(Position {
        element: 3,
        offset: boundary.project().elements[3].text.len(),
    })).is_ok());
    let before_boundary_join = boundary.project().clone();
    let revision = boundary.revision();
    assert!(boundary.join_with_next().is_ok());
    assert!(boundary.project().dual_dialogue_groups.is_empty());
    assert!(boundary.project().elements[3].text.contains("RIGHT"));
    let Some(unwrapped_preservation) = boundary.project().preservation.as_ref() else { return };
    assert_eq!(count_named(&unwrapped_preservation.sections[0].children, "SpellCheckIgnoreLists"), 1);
    assert_eq!(boundary.revision(), revision.saturating_add(1));
    let unwrapped_join = boundary.project().clone();
    assert!(boundary.undo().is_ok());
    assert_eq!(boundary.project(), &before_boundary_join);
    assert!(boundary.redo().is_ok());
    assert_eq!(boundary.project(), &unwrapped_join);

    let Some(mut imported) = import(include_str!("fixtures/dual_dialogue_basic.fdx")) else { return };
    let Some(target_foreign) = imported.project.elements[3].source_provenance.as_ref()
        .and_then(|provenance| provenance.foreign_id.clone()) else { return };
    let Some(source_identity) = imported.project.preservation.as_ref()
        .and_then(|preservation| preservation.identities.iter()
            .find(|identity| identity.value == "after-001").cloned()) else { return };
    let Some(preservation) = imported.project.preservation.as_mut() else { return };
    let Ok(target_identity) = ForeignIdentity::new(ForeignIdDomain::Paragraph, target_foreign) else { return };
    preservation.references.push(ForeignReference {
        source: source_identity,
        target: target_identity,
        policy: ReferencePolicy::RequiredInternal,
    });
    preservation.rebuild_indexes();
    let Ok(mut referenced) = EditingSession::new(imported.project) else { return };
    assert!(referenced.set_selection(Selection::caret(Position {
        element: 2,
        offset: referenced.project().elements[2].text.len(),
    })).is_ok());
    let before_reference_join = referenced.clone();
    assert!(referenced.join_with_next().is_err());
    assert_eq!(referenced, before_reference_join);

    let Some(mut imported) = import(include_str!("fixtures/dual_dialogue_basic.fdx")) else { return };
    let Some(target_foreign) = imported.project.elements[3].source_provenance.as_ref()
        .and_then(|provenance| provenance.foreign_id.clone()) else { return };
    let Ok(target_identity) = ForeignIdentity::new(ForeignIdDomain::Paragraph, target_foreign) else { return };
    let Some(preservation) = imported.project.preservation.as_mut() else { return };
    let Ok(anchor_id) = SourceAnchorId::new("wo005-external-anchor") else { return };
    preservation.anchors.push(SourceAnchor {
        id: anchor_id,
        target: AnchorTarget::Foreign(target_identity),
        relation: AnchorRelation::Attached,
    });
    preservation.rebuild_indexes();
    let Ok(mut anchored) = EditingSession::new(imported.project) else { return };
    assert!(anchored.set_selection(Selection::caret(Position {
        element: 2,
        offset: anchored.project().elements[2].text.len(),
    })).is_ok());
    let before_anchor_join = anchored.clone();
    assert!(anchored.join_with_next().is_err());
    assert_eq!(anchored, before_anchor_join);

    let Some(mut imported) = import(include_str!("fixtures/dual_dialogue_basic.fdx")) else { return };
    let Some(preservation) = imported.project.preservation.as_mut() else { return };
    assert!(add_managed_attachment(&mut preservation.sections[0].children, "left-parenthetical-001"));
    preservation.rebuild_indexes();
    let Ok(mut attached) = EditingSession::new(imported.project) else { return };
    assert!(attached.set_selection(Selection::caret(Position {
        element: 2,
        offset: attached.project().elements[2].text.len(),
    })).is_ok());
    let before_attachment_join = attached.clone();
    assert!(attached.join_with_next().is_err());
    assert_eq!(attached, before_attachment_join);
}

#[test]
fn nested_edits_isolate_adjacent_source_groups_and_leave_direct_content_unchanged() {
    let mut second_wrapper = {
        let source = include_str!("fixtures/dual_dialogue_basic.fdx");
        let Some(start) = source.find("<Paragraph Type=\"General\"") else { return };
        let Some(length) = source[start..].find("</DualDialogue></Paragraph>") else { return };
        source[start..start + length + "</DualDialogue></Paragraph>".len()].to_owned()
    };
    for (old, new) in [
        ("outer-dual-001", "outer-dual-002"),
        ("left-cue-001", "left-cue-002"),
        ("left-dialogue-001", "left-dialogue-002"),
        ("left-parenthetical-001", "left-parenthetical-002"),
        ("right-cue-001", "right-cue-002"),
        ("right-dialogue-001", "right-dialogue-002"),
        ("right-parenthetical-001", "right-parenthetical-002"),
    ] {
        second_wrapper = second_wrapper.replace(old, new);
    }
    let source = include_str!("fixtures/dual_dialogue_basic.fdx");
    let Some(insert_at) = source.find("<Paragraph Type=\"Action\" id=\"after-001\"") else { return };
    let mut source_with_adjacent_groups = String::with_capacity(source.len() + second_wrapper.len());
    source_with_adjacent_groups.push_str(&source[..insert_at]);
    source_with_adjacent_groups.push_str(&second_wrapper);
    source_with_adjacent_groups.push_str(&source[insert_at..]);
    let Some(imported) = import(&source_with_adjacent_groups) else { return };
    assert_eq!(imported.project.dual_dialogue_groups.len(), 2);
    let original = imported.project.clone();
    let second_group = original.dual_dialogue_groups[1].clone();
    let Some(original_second_wrapper) = original.preservation.as_ref()
        .and_then(|preservation| find_wrapper_owner(&preservation.sections[0].children, &second_group.id))
        .cloned() else { return };
    let mut session = EditingSession::new(original.clone());
    let Ok(ref mut session) = session else { return };
    assert!(session.set_selection(Selection::caret(Position {
        element: 2,
        offset: session.project().elements[2].text.len(),
    })).is_ok());
    assert!(session.join_with_next().is_ok());
    assert_eq!(session.project().dual_dialogue_groups.len(), 2);
    assert_eq!(session.project().dual_dialogue_groups[1], second_group);
    let Some(after_second_wrapper) = session.project().preservation.as_ref()
        .and_then(|preservation| find_wrapper_owner(&preservation.sections[0].children, &second_group.id))
        else { return };
    assert_eq!(after_second_wrapper, &original_second_wrapper);
    assert!(session.undo().is_ok());
    assert_eq!(session.project(), &original);

    let Some(single_group) = import(include_str!("fixtures/dual_dialogue_basic.fdx")) else { return };
    let original_group = single_group.project.dual_dialogue_groups[0].clone();
    let Some(original_wrapper) = single_group.project.preservation.as_ref()
        .and_then(|preservation| find_wrapper_owner(&preservation.sections[0].children, &original_group.id))
        .cloned() else { return };
    let Ok(mut direct_edit) = EditingSession::new(single_group.project) else { return };
    assert!(direct_edit.set_selection(Selection::caret(Position { element: 0, offset: "Before".len() })).is_ok());
    assert!(direct_edit.split().is_ok());
    assert_eq!(direct_edit.project().elements[0].text, "Before");
    assert_eq!(direct_edit.project().elements[1].text, " the exchange.");
    assert_eq!(direct_edit.project().dual_dialogue_groups[0], original_group);
    let Some(after_wrapper) = direct_edit.project().preservation.as_ref()
        .and_then(|preservation| find_wrapper_owner(&preservation.sections[0].children, &original_group.id))
        else { return };
    assert_eq!(after_wrapper, &original_wrapper);
}

#[test]
fn structural_planner_reports_all_four_group_outcomes_without_mutating_inputs() {
    let Some(baseline) = grouped_project() else { return };
    let selected_group = Selection::caret(Position { element: 1, offset: 0 });
    let unchanged = scriptcraft_core::dual_dialogue::classify_structural_result(&baseline, selected_group, &baseline);
    assert_eq!(unchanged[0].outcome, scriptcraft_core::dual_dialogue::StructuralGroupOutcome::UnchangedValid);
    assert!(unchanged[0].touched);

    let mut updated = baseline.clone();
    let Ok(inserted) = ScreenplayElement::new("planner-inserted", Role::Parenthetical, "(aside)") else { return };
    let inserted_id = inserted.id.clone();
    updated.elements.insert(2, inserted);
    updated.dual_dialogue_groups[0].left.paragraphs.push(inserted_id);
    assert_eq!(scriptcraft_core::dual_dialogue::classify_structural_result(&baseline, selected_group, &updated)[0].outcome,
        scriptcraft_core::dual_dialogue::StructuralGroupOutcome::UpdatedValid);

    let mut invalid = baseline.clone();
    invalid.elements[1].role = Role::Action;
    assert_eq!(scriptcraft_core::dual_dialogue::classify_structural_result(&baseline, selected_group, &invalid)[0].outcome,
        scriptcraft_core::dual_dialogue::StructuralGroupOutcome::InvalidRequiresUnwrap);

    let mut removed = baseline.clone();
    removed.elements.clear();
    assert_eq!(scriptcraft_core::dual_dialogue::classify_structural_result(&baseline, selected_group, &removed)[0].outcome,
        scriptcraft_core::dual_dialogue::StructuralGroupOutcome::Removed);
    assert_eq!(baseline.dual_dialogue_groups.len(), 1);
    assert_eq!(baseline.elements.len(), 4);
}

#[test]
fn valid_tab_parenthetical_and_same_side_edits_retain_nested_spell_scope_and_history() {
    let Some(imported) = import(include_str!("fixtures/dual_dialogue_basic.fdx")) else { return };
    let original = imported.project.clone();
    let Some(original_spell) = nested_spell_payload(&original) else { return };
    let Ok(mut session) = EditingSession::new(original.clone()) else { return };
    assert!(session.set_selection(Selection::caret(Position {
        element: 1,
        offset: original.elements[1].text.len(),
    })).is_ok());
    let revision = session.revision();
    assert!(session.tab().is_ok());
    assert_eq!(session.project().elements[1].role, Role::Parenthetical);
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
    assert_eq!(nested_spell_payload(session.project()), Some(original_spell));
    assert_eq!(session.revision(), revision.saturating_add(1));
    let retained = session.project().clone();
    assert!(session.undo().is_ok());
    assert_eq!(session.project(), &original);
    assert!(session.redo().is_ok());
    assert_eq!(session.project(), &retained);
}

#[test]
fn ordinary_text_paste_run_style_and_alignment_edits_do_not_unwrap_groups() {
    let Some(original) = grouped_project() else { return };
    let member_ids = original.dual_dialogue_groups[0].member_ids().cloned().collect::<Vec<_>>();
    let Ok(mut session) = EditingSession::new(original) else { return };
    assert!(session.set_selection(Selection::caret(Position { element: 1, offset: 2 })).is_ok());
    assert!(session.insert(" typed").is_ok());
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
    assert!(session.insert_paste(" pasted").is_ok());
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
    assert!(session.set_selection(Selection {
        anchor: Position { element: 1, offset: 0 },
        focus: Position { element: 1, offset: 2 },
    }).is_ok());
    assert!(session.toggle_body_run_style(RunStyle::Bold).is_ok());
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
    assert!(session.set_selection(Selection::caret(Position { element: 1, offset: 0 })).is_ok());
    assert!(session.set_body_alignment(ParagraphAlignment::Center).is_ok());
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
    assert_eq!(session.project().dual_dialogue_groups[0].member_ids().cloned().collect::<Vec<_>>(), member_ids);
}

#[test]
fn left_enter_unwraps_before_action_and_is_one_undo_redo_step() {
    let Some(original) = grouped_project() else { return };
    let Ok(mut session) = EditingSession::new(original.clone()) else { return };
    assert!(session.set_selection(Selection::caret(Position {
        element: 1,
        offset: original.elements[1].text.len(),
    })).is_ok());
    let revision = session.revision();
    assert!(session.enter().is_ok());
    assert!(session.project().dual_dialogue_groups.is_empty());
    assert_eq!(session.project().elements[2].role, Role::Action);
    assert_eq!(session.selection(), Selection::caret(Position { element: 2, offset: 0 }));
    assert_eq!(session.revision(), revision.saturating_add(1));
    let unwrapped = session.project().clone();
    assert!(session.undo().is_ok());
    assert_eq!(session.project(), &original);
    assert!(session.redo().is_ok());
    assert_eq!(session.project(), &unwrapped);
}

#[test]
fn final_right_enter_keeps_wrapper_and_places_action_after_nested_members() {
    let source = include_str!("fixtures/dual_dialogue_basic.fdx")
        .replace("<Paragraph Type=\"Parenthetical\" id=\"right-parenthetical-001\"/>", "");
    let Some(imported) = import(&source) else { return };
    let original = imported.project.clone();
    let Some(original_spell) = nested_spell_payload(&original) else { return };
    let right_dialogue = original.elements.len().saturating_sub(2);
    let Ok(mut session) = EditingSession::new(original.clone()) else { return };
    assert!(session.set_selection(Selection::caret(Position {
        element: right_dialogue,
        offset: original.elements[right_dialogue].text.len(),
    })).is_ok());
    assert!(session.enter().is_ok());
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
    let action_index = right_dialogue + 1;
    assert_eq!(session.project().elements[action_index].role, Role::Action);
    assert!(!session.project().dual_dialogue_groups[0].member_ids()
        .any(|id| id == &session.project().elements[action_index].id));
    assert_eq!(session.selection(), Selection::caret(Position { element: action_index, offset: 0 }));
    assert_eq!(nested_spell_payload(session.project()), Some(original_spell));
    let group = &session.project().dual_dialogue_groups[0];
    let Some(wrapper) = session.project().preservation.as_ref()
        .and_then(|preservation| find_wrapper_owner(&preservation.sections[0].children, &group.id)) else { return };
    assert!(find_managed(&wrapper.children, session.project().elements[action_index].id.as_str()).is_none());
    let Some(preservation) = session.project().preservation.as_ref() else { return };
    assert!(find_managed(&preservation.sections[0].children,
        session.project().elements[action_index].id.as_str()).is_some());
    let after = session.project().clone();
    assert!(session.undo().is_ok());
    assert_eq!(session.project(), &original);
    assert!(session.redo().is_ok());
    assert_eq!(session.project(), &after);
}

#[test]
fn cue_only_sides_accept_character_templated_insertion_and_final_dialogue_deletion_retains_cc() {
    let Some(imported) = import(include_str!("fixtures/dual_dialogue_empty_sides.fdx")) else { return };
    let project = imported.project;
    let empty_group = project.dual_dialogue_groups.iter().find(|group| {
        group.left.paragraphs.is_empty() && group.right.paragraphs.is_empty()
    }).cloned();
    assert!(empty_group.is_some(), "the fixture contains a source-backed C,C group");
    let Some(empty_group) = empty_group else { return };
    let member_ids = empty_group.member_ids().cloned().collect::<Vec<_>>();
    let left_index = project.elements.iter().position(|element| element.id == empty_group.left.character);
    assert!(left_index.is_some());
    let Some(left_index) = left_index else { return };
    let session_result = EditingSession::new(project.clone());
    assert!(session_result.is_ok());
    let Ok(mut inserted) = session_result else { return };
    let left_end = inserted.project().elements[left_index].text.len();
    assert!(inserted.set_selection(Selection::caret(Position { element: left_index, offset: left_end })).is_ok());
    assert!(inserted.enter().is_ok());
    let after_left = inserted.project().dual_dialogue_groups.iter().find(|group| group.id == empty_group.id);
    assert!(after_left.is_some_and(|group| group.left.paragraphs.len() == 1 && group.right.paragraphs.is_empty()));
    assert_eq!(after_left.map(|group| group.member_ids().filter(|id| member_ids.contains(id)).count()), Some(2));

    let right_index = inserted.project().elements.iter()
        .position(|element| element.id == empty_group.right.character);
    assert!(right_index.is_some());
    let Some(right_index) = right_index else { return };
    let right_end = inserted.project().elements[right_index].text.len();
    assert!(inserted.set_selection(Selection::caret(Position { element: right_index, offset: right_end })).is_ok());
    assert!(inserted.enter().is_ok());
    let after_both = inserted.project().dual_dialogue_groups.iter().find(|group| group.id == empty_group.id);
    assert!(after_both.is_some_and(|group| group.left.paragraphs.len() == 1 && group.right.paragraphs.len() == 1));
    let both_sides = inserted.project().clone();
    assert!(inserted.undo().is_ok());
    assert!(inserted.project().dual_dialogue_groups.iter().find(|group| group.id == empty_group.id)
        .is_some_and(|group| group.left.paragraphs.len() == 1 && group.right.paragraphs.is_empty()));
    assert!(inserted.redo().is_ok());
    assert_eq!(inserted.project(), &both_sides);
    let bytes = scriptcraft_core::fdx::write_v6(inserted.project());
    assert!(bytes.is_ok(), "two-side insertion remains a complete writer candidate");
    if let Some(bytes) = bytes.ok() {
        let reopened = read_v6(&bytes);
        assert!(reopened.is_ok());
        if let Some(reopened) = reopened.ok() {
            assert!(reopened.project.dual_dialogue_groups.iter().any(|group| {
                group.member_ids().any(|id| id.as_str() == empty_group.left.character.as_str())
                    && group.left.paragraphs.len() == 1 && group.right.paragraphs.len() == 1
            }));
        }
    }

    let ccd = project.dual_dialogue_groups.iter().find(|group| {
        group.left.paragraphs.is_empty() && group.right.paragraphs.len() == 1
    }).cloned();
    assert!(ccd.is_some(), "the fixture contains the C,C,D deletion boundary");
    let Some(ccd) = ccd else { return };
    let right_cue_index = project.elements.iter().position(|element| element.id == ccd.right.character);
    let dialogue_id = ccd.right.paragraphs[0].clone();
    let dialogue_index = project.elements.iter().position(|element| element.id == dialogue_id);
    assert!(right_cue_index.is_some() && dialogue_index.is_some());
    let (Some(right_cue_index), Some(dialogue_index)) = (right_cue_index, dialogue_index) else { return };
    let session_result = EditingSession::new(project.clone());
    assert!(session_result.is_ok());
    let Ok(mut deleted) = session_result else { return };
    assert!(deleted.set_selection(Selection {
        anchor: Position { element: right_cue_index, offset: deleted.project().elements[right_cue_index].text.len() },
        focus: Position { element: dialogue_index, offset: deleted.project().elements[dialogue_index].text.len() },
    }).is_ok());
    assert!(deleted.delete().is_ok());
    let retained = deleted.project().dual_dialogue_groups.iter().find(|group| {
        group.left.character == ccd.left.character && group.right.character == ccd.right.character
    });
    assert!(retained.is_some(), "deleting the final right Dialogue keeps both original cues grouped");
    assert!(retained.is_some_and(|group| group.left.paragraphs.is_empty() && group.right.paragraphs.is_empty()));
    assert_eq!(deleted.project().elements.iter().filter(|element| element.id == dialogue_id).count(), 0);
    let deleted_project = deleted.project().clone();
    assert!(deleted.undo().is_ok());
    assert_eq!(deleted.project(), &project);
    assert!(deleted.redo().is_ok());
    assert_eq!(deleted.project(), &deleted_project);
}

#[test]
fn role_breaking_boundary_and_delimiter_only_edits_unwrap_then_edit_once() {
    let Some(original) = grouped_project() else { return };
    let Ok(mut role_change) = EditingSession::new(original.clone()) else { return };
    assert!(role_change.set_selection(Selection::caret(Position { element: 0, offset: 0 })).is_ok());
    assert!(role_change.change_role(Role::Action).is_ok());
    assert!(role_change.project().dual_dialogue_groups.is_empty());
    assert_eq!(role_change.project().elements[0].role, Role::Action);
    let role_result = role_change.project().clone();
    assert!(role_change.undo().is_ok());
    assert_eq!(role_change.project(), &original);
    assert!(role_change.redo().is_ok());
    assert_eq!(role_change.project(), &role_result);

    let mut boundary_project = original.clone();
    let Ok(prefix) = ScreenplayElement::new("outside-before", Role::Action, "before") else { return };
    boundary_project.elements.insert(0, prefix);
    let Ok(mut backspace) = EditingSession::new(boundary_project.clone()) else { return };
    assert!(backspace.set_selection(Selection::caret(Position { element: 3, offset: 0 })).is_ok());
    assert!(backspace.backspace().is_ok());
    assert!(backspace.project().dual_dialogue_groups.is_empty());
    let backspace_result = backspace.project().clone();
    assert!(backspace.undo().is_ok());
    assert_eq!(backspace.project(), &boundary_project);
    assert!(backspace.redo().is_ok());
    assert_eq!(backspace.project(), &backspace_result);

    let mut delimiter_project = Project::new().ok();
    let Some(mut delimiter_project) = delimiter_project.take() else { return };
    let roles = [Role::Character, Role::Parenthetical, Role::Character, Role::Dialogue];
    let elements = roles.into_iter().enumerate().map(|(index, role)| {
        ScreenplayElement::new(format!("delimiter-{index}"), role,
            if role == Role::Parenthetical { "()" } else { "text" })
    }).collect::<Result<Vec<_>, _>>();
    let Ok(elements) = elements else { return };
    delimiter_project.elements = elements;
    let Ok(mut delimiter) = EditingSession::new(delimiter_project) else { return };
    assert!(delimiter.set_selection(Selection::caret(Position { element: 0, offset: 0 })).is_ok());
    assert!(delimiter.toggle_dual_dialogue().is_ok());
    let delimiter_original = delimiter.project().clone();
    assert!(delimiter.set_selection(Selection::caret(Position { element: 1, offset: 0 })).is_ok());
    assert!(delimiter.delete().is_ok());
    assert!(delimiter.project().dual_dialogue_groups.is_empty());
    let delimiter_result = delimiter.project().clone();
    assert!(delimiter.undo().is_ok());
    assert_eq!(delimiter.project(), &delimiter_original);
    assert!(delimiter.redo().is_ok());
    assert_eq!(delimiter.project(), &delimiter_result);
}

#[test]
fn partial_and_complete_group_selections_unwrap_and_keep_atomic_history() {
    let Some(original) = grouped_project() else { return };
    for selection in [
        Selection {
            anchor: Position { element: 0, offset: 0 },
            focus: Position { element: 2, offset: 0 },
        },
        Selection {
            anchor: Position { element: 0, offset: 0 },
            focus: Position { element: 3, offset: original.elements[3].text.len() },
        },
    ] {
        let Ok(mut session) = EditingSession::new(original.clone()) else { return };
        assert!(session.set_selection(selection).is_ok());
        let revision = session.revision();
        assert!(session.delete().is_ok());
        assert!(session.project().dual_dialogue_groups.is_empty());
        assert_eq!(session.revision(), revision.saturating_add(1));
        let edited = session.project().clone();
        assert!(session.undo().is_ok());
        assert_eq!(session.project(), &original);
        assert!(session.redo().is_ok());
        assert_eq!(session.project(), &edited);
    }
}

#[test]
fn cross_paragraph_replacement_and_boundary_delete_use_the_unwrapped_retry() {
    let Some(original) = grouped_project() else { return };
    let Ok(mut boundary_delete) = EditingSession::new(original.clone()) else { return };
    assert!(boundary_delete.set_selection(Selection {
        anchor: Position { element: 1, offset: original.elements[1].text.len() },
        focus: Position { element: 2, offset: 0 },
    }).is_ok());
    assert!(boundary_delete.delete().is_ok());
    assert!(boundary_delete.project().dual_dialogue_groups.is_empty());
    let deleted = boundary_delete.project().clone();
    assert!(boundary_delete.undo().is_ok());
    assert_eq!(boundary_delete.project(), &original);
    assert!(boundary_delete.redo().is_ok());
    assert_eq!(boundary_delete.project(), &deleted);

    let Ok(mut replacement) = EditingSession::new(original.clone()) else { return };
    assert!(replacement.set_selection(Selection {
        anchor: Position { element: 1, offset: 2 },
        focus: Position { element: 3, offset: 2 },
    }).is_ok());
    assert!(replacement.insert("replacement").is_ok());
    assert!(replacement.project().dual_dialogue_groups.is_empty());
    assert!(replacement.project().elements.iter().any(|element| element.text.contains("replacement")));
    let replaced = replacement.project().clone();
    assert!(replacement.undo().is_ok());
    assert_eq!(replacement.project(), &original);
    assert!(replacement.redo().is_ok());
    assert_eq!(replacement.project(), &replaced);
}

#[test]
fn cue_split_and_multi_group_selection_unwrap_every_affected_group() {
    let Some(original) = grouped_project() else { return };
    let Ok(mut split) = EditingSession::new(original.clone()) else { return };
    assert!(split.set_selection(Selection::caret(Position { element: 0, offset: 2 })).is_ok());
    assert!(split.split().is_ok());
    assert!(split.project().dual_dialogue_groups.is_empty());
    assert_eq!(split.project().elements[1].role, Role::Character);
    let split_result = split.project().clone();
    assert!(split.undo().is_ok());
    assert_eq!(split.project(), &original);
    assert!(split.redo().is_ok());
    assert_eq!(split.project(), &split_result);

    let mut adjacent = Project::new().ok();
    let Some(mut adjacent) = adjacent.take() else { return };
    let roles = [
        Role::Character, Role::Dialogue, Role::Character, Role::Dialogue,
        Role::Character, Role::Dialogue, Role::Character, Role::Dialogue,
    ];
    let elements = roles.into_iter().enumerate().map(|(index, role)| {
        ScreenplayElement::new(format!("multi-{index}"), role, format!("text-{index}"))
    }).collect::<Result<Vec<_>, _>>();
    let Ok(elements) = elements else { return };
    adjacent.elements = elements;
    let Ok(mut session) = EditingSession::new(adjacent) else { return };
    for index in [0, 4] {
        assert!(session.set_selection(Selection::caret(Position { element: index, offset: 0 })).is_ok());
        assert!(session.toggle_dual_dialogue().is_ok());
    }
    let before = session.project().clone();
    let unaffected = before.dual_dialogue_groups[1].clone();
    let Ok(mut isolated) = EditingSession::new(before.clone()) else { return };
    assert!(isolated.set_selection(Selection::caret(Position {
        element: 1,
        offset: before.elements[1].text.len(),
    })).is_ok());
    assert!(isolated.enter().is_ok());
    assert_eq!(isolated.project().dual_dialogue_groups.len(), 1);
    assert_eq!(isolated.project().dual_dialogue_groups[0], unaffected);

    assert!(session.set_selection(Selection {
        anchor: Position { element: 1, offset: 0 },
        focus: Position { element: 7, offset: session.project().elements[7].text.len() },
    }).is_ok());
    assert!(session.delete().is_ok());
    assert!(session.project().dual_dialogue_groups.is_empty());
    let after = session.project().clone();
    assert!(session.undo().is_ok());
    assert_eq!(session.project(), &before);
    assert!(session.redo().is_ok());
    assert_eq!(session.project(), &after);
}

#[test]
fn failed_post_unwrap_source_edit_keeps_revision_history_redo_and_dirty_state() {
    let source = include_str!("fixtures/dual_dialogue_basic.fdx")
        .replace("id=\"left-parenthetical-001\"", "id=\"left-parenthetical-001\" LeftIndent=\"1.00\"");
    let Some(imported) = import(&source) else { return };
    let Ok(mut session) = EditingSession::new(imported.project) else { return };
    assert!(session.set_selection(Selection::caret(Position {
        element: 2,
        offset: session.project().elements[2].text.len(),
    })).is_ok());
    let before = session.clone();
    assert!(session.join_with_next().is_err());
    assert_eq!(session, before);
    assert!(!session.dirty());
    assert_eq!(session.revision(), before.revision());
}

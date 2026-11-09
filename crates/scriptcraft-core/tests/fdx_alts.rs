#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::fdx::{read_v6, write_v6, FdxError};
use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::preservation::{ForeignIdDomain, PreservedElement, PreservedNode, SourceOwnership};
use scriptcraft_core::{ParagraphAlignment, Role, RunStyle};

const FIXTURE: &str = include_str!("fixtures/dialogue_alts.fdx");

fn imported(source: &str) -> Option<scriptcraft_core::fdx::FdxDocument> {
    let result = read_v6(source.as_bytes());
    assert!(result.is_ok(), "the approved Alts source shape should import");
    result.ok()
}

fn collect<'a>(nodes: &'a [PreservedNode], output: &mut Vec<&'a PreservedElement>) {
    for node in nodes {
        if let PreservedNode::Element(element) = node {
            output.push(element);
            collect(&element.children, output);
        }
    }
}

fn attr<'a>(element: &'a PreservedElement, name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|attribute| attribute.name.local_name == name)
        .map(|attribute| attribute.value.as_str())
}

fn by_id<'a>(elements: &[&'a PreservedElement], id: &str) -> Option<&'a PreservedElement> {
    elements
        .iter()
        .copied()
        .find(|element| attr(element, "id") == Some(id))
}

fn find_managed<'a>(nodes: &'a [PreservedNode], id: &scriptcraft_core::StableId) -> Option<&'a PreservedElement> {
    for node in nodes {
        let PreservedNode::Element(element) = node else { continue };
        if matches!(&element.ownership, SourceOwnership::Managed(owner) if owner == id) {
            return Some(element);
        }
        if let Some(found) = find_managed(&element.children, id) {
            return Some(found);
        }
    }
    None
}

fn inline_alts(project: &scriptcraft_core::Project, id: &scriptcraft_core::StableId) -> Option<PreservedElement> {
    let source = project.preservation.as_ref()?;
    let owner = find_managed(&source.sections[0].children, id)?;
    owner.children.iter().find_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "Alts" => Some(element.clone()),
        _ => None,
    })
}

fn alt_collection(project: &scriptcraft_core::Project) -> Option<PreservedElement> {
    let source = project.preservation.as_ref()?;
    let mut elements = Vec::new();
    collect(&source.sections[0].children, &mut elements);
    elements.into_iter().find(|element| element.name.local_name == "AltCollection").cloned()
}

fn paired_dialogues(alts_on_left: bool) -> String {
    let left_children = if alts_on_left {
        "<Text>left</Text><Alts Current=\"1\"><AltId Id=\"pair-alt\"/><Choice><Text>PRIVATE_INLINE</Text></Choice></Alts>"
    } else {
        "<Text>left</Text>"
    };
    let right_children = if alts_on_left {
        "<Text>right</Text>"
    } else {
        "<Text>right</Text><Alts Current=\"1\"><AltId Id=\"pair-alt\"/><Choice><Text>PRIVATE_INLINE</Text></Choice></Alts>"
    };
    let owner_id = if alts_on_left { "left-owner" } else { "right-owner" };
    doc(
        &format!(
            "{}{}",
            paragraph("Dialogue", if alts_on_left { "left-owner" } else { "left-plain" }, left_children),
            paragraph("Dialogue", if alts_on_left { "right-plain" } else { "right-owner" }, right_children),
        ),
        &format!(
            "<AltCollection><Alt Id=\"pair-alt\"><Paragraph Type=\"Dialogue\" id=\"{owner_id}\"><Text>collection copy</Text></Paragraph></Alt><Alt Id=\"orphan\"><Paragraph Type=\"Dialogue\" id=\"pair-orphan\"><Text>orphan</Text></Paragraph></Alt></AltCollection>"
        ),
    )
}

fn named_count(nodes: &[PreservedNode], name: &str) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            PreservedNode::Element(element) => {
                usize::from(element.name.local_name == name)
                    + named_count(&element.children, name)
            }
            _ => 0,
        })
        .sum()
}

fn doc(content: &str, suffix: &str) -> String {
    format!(
        "<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content>{content}</Content>{suffix}</FinalDraft>"
    )
}

fn paragraph(role: &str, id: &str, children: &str) -> String {
    format!("<Paragraph Type=\"{role}\" id=\"{id}\">{children}</Paragraph>")
}

#[test]
fn opaque_inline_alts_and_exact_alternate_copies_round_trip_without_semantic_projection() {
    let Some(imported) = imported(FIXTURE) else { return };
    let project = &imported.project;
    assert_eq!(project.elements.len(), 4);
    assert_eq!(project.dual_dialogue_groups.len(), 1);
    assert_eq!(project.elements[0].role, Role::Dialogue);
    assert_eq!(project.elements[0].text, "ordinary canonical");
    assert_eq!(project.elements[1].role, Role::Character);
    assert_eq!(project.elements[2].role, Role::Character);
    assert_eq!(project.elements[3].role, Role::Dialogue);
    assert_eq!(project.elements[3].text, "member canonical");
    assert!(project.elements.iter().all(|element| {
        !element.text.contains("SENTINEL")
            && element.runs.iter().all(|run| !run.text.contains("SENTINEL"))
    }));
    assert!(project.elements.iter().all(|element| element.role != Role::General));

    let Some(source) = project.preservation.as_ref() else { return };
    let mut elements = Vec::new();
    collect(&source.sections[0].children, &mut elements);
    let ordinary = by_id(&elements, "alts-ordinary");
    assert!(ordinary.is_some());
    let Some(ordinary) = ordinary else { return };
    let ordinary_children = ordinary.children.iter().filter_map(|node| match node {
        PreservedNode::Element(element) => Some(element),
        _ => None,
    }).collect::<Vec<_>>();
    assert_eq!(ordinary_children.iter().map(|element| element.name.local_name.as_str()).collect::<Vec<_>>(),
        vec!["Text", "Alts", "Text"]);
    assert_eq!(ordinary_children[1].ownership, SourceOwnership::Opaque);
    assert_eq!(attr(ordinary_children[1], "Current"), Some("5"));
    assert_eq!(attr(ordinary_children[1], "vendor-inline"), Some("ordinary"));

    let member = by_id(&elements, "alts-member");
    assert!(member.is_some());
    let Some(member) = member else { return };
    assert!(member.children.iter().any(|node| matches!(node,
        PreservedNode::Element(element) if element.name.local_name == "Alts"
            && element.ownership == SourceOwnership::Opaque
            && attr(element, "vendor-inline") == Some("member"))));

    let collection = elements.iter().copied().find(|element| {
        element.name.local_name == "AltCollection"
    });
    assert!(collection.is_some());
    let Some(collection) = collection else { return };
    assert_eq!(collection.ownership, SourceOwnership::Opaque);
    assert_eq!(named_count(&collection.children, "Alt"), 4);
    assert_eq!(named_count(&collection.children, "Paragraph"), 4);
    let copies = elements.iter().filter(|element| {
        element.name.local_name == "Paragraph"
            && matches!(element.ownership, SourceOwnership::Opaque)
            && ["alts-member", "alts-ordinary", "orphan-copy"].contains(&attr(element, "id").unwrap_or_default())
    }).map(|element| attr(element, "id").unwrap_or_default()).collect::<Vec<_>>();
    assert_eq!(copies, vec!["alts-member", "alts-ordinary", "alts-ordinary", "orphan-copy"]);

    let paragraph_ids = source.identities.iter()
        .filter(|identity| identity.domain == ForeignIdDomain::Paragraph)
        .map(|identity| identity.value.as_str()).collect::<Vec<_>>();
    assert_eq!(paragraph_ids.iter().filter(|id| **id == "alts-ordinary").count(), 1);
    assert_eq!(paragraph_ids.iter().filter(|id| **id == "alts-member").count(), 1);
    assert!(!paragraph_ids.contains(&"orphan-copy"));

    let output = write_v6(project);
    assert!(output.is_ok(), "direct output must pass strict source, semantic, and identity gates");
    let Some(output) = output.ok() else { return };
    let reopened = read_v6(&output);
    assert!(reopened.is_ok(), "direct output must re-read");
    let Some(reopened) = reopened.ok() else { return };
    assert_eq!(reopened.project.elements, project.elements);
    assert_eq!(reopened.project.dual_dialogue_groups, project.dual_dialogue_groups);
    let Some(reopened_source) = reopened.project.preservation.as_ref() else { return };
    assert_eq!(reopened_source.sections, source.sections);
    assert_eq!(named_count(&reopened_source.sections[0].children, "AltCollection"), 1);
    assert_eq!(named_count(&reopened_source.sections[0].children, "Alts"), 2);
}

#[test]
fn inline_alts_are_rejected_outside_the_single_same_namespace_dialogue_attachment() {
    let alternatives = [
        doc(&paragraph("Action", "p", "<Text>x</Text><Alts/>").to_string(), ""),
        doc(&paragraph("Dialogue", "p", "<Text>x</Text><Alts/><Alts/>").to_string(), ""),
        doc(&paragraph("Dialogue", "p", "<Text>x</Text><x:Alts xmlns:x=\"urn:other\"/>").to_string(), ""),
        doc(&paragraph("Dialogue", "p", "<Text>x<Alts/></Text>").to_string(), ""),
        "<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content/><TitlePage><Content><Paragraph Type=\"Dialogue\" id=\"title\"><Text>x</Text><Alts/></Paragraph></Content></TitlePage></FinalDraft>".to_owned(),
        doc("<Paragraph Type=\"Parenthetical\" id=\"cue\"><Text>x</Text><Alts/></Paragraph>", ""),
    ];
    for source in alternatives {
        assert_eq!(read_v6(source.as_bytes()).err(), Some(FdxError::UnsupportedConstruct));
    }
}

#[test]
fn only_exact_altcollection_alt_paragraph_ancestry_is_exempt_from_global_identity_checks() {
    let orphan_only = doc(
        "<Paragraph Type=\"Action\" id=\"plain\"><Text>plain</Text></Paragraph>",
        "<AltCollection><Alt Id=\"orphan\"><Paragraph Type=\"Dialogue\" id=\"orphan-id\"><Text>orphan</Text></Paragraph></Alt></AltCollection>",
    );
    assert!(read_v6(orphan_only.as_bytes()).is_ok());

    let body_owner = paragraph("Dialogue", "duplicate", "<Text>x</Text>");
    let variants = [
        doc(&format!("{body_owner}{body_owner}"), ""),
        doc(&format!(
            "{}<Opaque>{}</Opaque>",
            body_owner,
            paragraph("Dialogue", "duplicate", "<Text>opaque</Text>")
        ), ""),
        doc(
            "<OpaqueA><Paragraph Type=\"Dialogue\" id=\"opaque-repeat\"><Text>a</Text></Paragraph></OpaqueA><OpaqueB><Paragraph Type=\"Dialogue\" id=\"opaque-repeat\"><Text>b</Text></Paragraph></OpaqueB>",
            "",
        ),
        doc(&body_owner, &format!("<Alt><Paragraph Type=\"Dialogue\" id=\"duplicate\"><Text>copy</Text></Paragraph></Alt>")),
        doc(&body_owner, &format!("<AltCollection><Group><Paragraph Type=\"Dialogue\" id=\"duplicate\"><Text>copy</Text></Paragraph></Group></AltCollection>")),
        doc(&body_owner, &format!("<AltCollection xmlns=\"urn:lookalike\"><Alt><Paragraph Type=\"Dialogue\" id=\"duplicate\"><Text>copy</Text></Paragraph></Alt></AltCollection>")),
    ];
    for source in variants {
        assert_eq!(read_v6(source.as_bytes()).err(), Some(FdxError::AmbiguousIdentity));
    }

    let body = paragraph("Dialogue", "shared", "<Text>x</Text>");
    let member = paragraph("Dialogue", "shared", "<Text>member</Text>");
    let dual = format!(
        "<Paragraph Type=\"General\" id=\"wrapper\"><DualDialogue><SpellCheckIgnoreLists><IgnoredRanges/><IgnoredWords/></SpellCheckIgnoreLists><Paragraph Type=\"Character\" id=\"left\"><Text>L</Text></Paragraph><Paragraph Type=\"Character\" id=\"right\"><Text>R</Text></Paragraph>{member}</DualDialogue></Paragraph>"
    );
    assert_eq!(read_v6(doc(&format!("{body}{dual}"), "").as_bytes()).err(), Some(FdxError::AmbiguousIdentity));
}

#[test]
fn inline_alts_are_accepted_on_a_dual_dialogue_dialogue_member_only() {
    let member = paragraph(
        "Parenthetical",
        "member",
        "<Text>(aside)</Text><Alts><Choice>opaque</Choice></Alts>",
    );
    let source = doc(
        &format!(
            "<Paragraph Type=\"General\" id=\"wrapper\"><DualDialogue><SpellCheckIgnoreLists><IgnoredRanges/><IgnoredWords/></SpellCheckIgnoreLists><Paragraph Type=\"Character\" id=\"left\"><Text>L</Text></Paragraph><Paragraph Type=\"Character\" id=\"right\"><Text>R</Text></Paragraph>{member}</DualDialogue></Paragraph>"
        ),
        "",
    );
    assert_eq!(read_v6(source.as_bytes()).err(), Some(FdxError::UnsupportedConstruct));
}

#[test]
fn typing_paste_style_alignment_and_emptying_keep_ordinary_and_member_alts_owner_attached() {
    let Some(imported) = imported(FIXTURE) else { return };
    for index in [0usize, 3usize] {
        let original = imported.project.clone();
        let owner_id = original.elements[index].id.clone();
        let original_alts = inline_alts(&original, &owner_id);
        assert!(original_alts.is_some());
        let Some(original_alts) = original_alts else { return };
        let session_result = EditingSession::new(original);
        assert!(session_result.is_ok());
        let Ok(mut session) = session_result else { return };

        assert!(session.set_selection(Selection::caret(Position { element: index, offset: 1 })).is_ok());
        assert!(session.insert(" typed").is_ok());
        assert_eq!(inline_alts(session.project(), &owner_id), Some(original_alts.clone()));
        assert!(session.insert_paste(" pasted").is_ok());
        assert_eq!(inline_alts(session.project(), &owner_id), Some(original_alts.clone()));

        assert!(session.set_selection(Selection {
            anchor: Position { element: index, offset: 0 },
            focus: Position { element: index, offset: 1 },
        }).is_ok());
        assert!(session.toggle_body_run_style(RunStyle::Bold).is_ok());
        assert_eq!(inline_alts(session.project(), &owner_id), Some(original_alts.clone()));
        assert!(session.set_body_alignment(ParagraphAlignment::Center).is_ok());
        assert_eq!(inline_alts(session.project(), &owner_id), Some(original_alts.clone()));

        let text_len = session.project().elements[index].text.len();
        assert!(session.set_selection(Selection {
            anchor: Position { element: index, offset: 0 },
            focus: Position { element: index, offset: text_len },
        }).is_ok());
        assert!(session.delete().is_ok());
        assert!(session.project().elements[index].text.is_empty());
        assert_eq!(inline_alts(session.project(), &owner_id), Some(original_alts));
        let bytes = write_v6(session.project());
        assert!(bytes.is_ok(), "an empty canonical owner retains readable inline Alts");
        if let Some(bytes) = bytes.ok() {
            let reopened = read_v6(&bytes);
            assert!(reopened.is_ok());
            if let Some(reopened) = reopened.ok() {
                assert_eq!(inline_alts(&reopened.project, &owner_id), inline_alts(session.project(), &owner_id));
                assert!(reopened.project.elements[index].text.is_empty());
            }
        }
    }
}

#[test]
fn split_retains_alts_on_original_left_owner_and_final_right_enter_keeps_action_outside() {
    let Some(imported) = imported(FIXTURE) else { return };
    let project = imported.project.clone();
    let owner_id = project.elements[3].id.clone();
    let original_alts = inline_alts(&project, &owner_id);
    assert!(original_alts.is_some());
    let Some(original_alts) = original_alts else { return };

    let session_result = EditingSession::new(project.clone());
    assert!(session_result.is_ok());
    let Ok(mut split) = session_result else { return };
    let boundary = split.project().elements[3].text.len() / 2;
    assert!(split.set_selection(Selection::caret(Position { element: 3, offset: boundary })).is_ok());
    assert!(split.split().is_ok());
    assert_eq!(split.project().elements[3].id, owner_id);
    assert_eq!(inline_alts(split.project(), &owner_id), Some(original_alts.clone()));
    let right_id = split.project().elements.get(4).map(|element| element.id.clone());
    assert!(right_id.is_some());
    if let Some(right_id) = right_id {
        assert!(inline_alts(split.project(), &right_id).is_none());
    }
    let split_project = split.project().clone();
    assert!(split.undo().is_ok());
    assert_eq!(split.project(), &project);
    assert!(split.redo().is_ok());
    assert_eq!(split.project(), &split_project);

    let session_result = EditingSession::new(project);
    assert!(session_result.is_ok());
    let Ok(mut entered) = session_result else { return };
    let end = entered.project().elements[3].text.len();
    assert!(entered.set_selection(Selection::caret(Position { element: 3, offset: end })).is_ok());
    assert!(entered.enter().is_ok());
    assert_eq!(entered.project().dual_dialogue_groups.len(), 1);
    assert_eq!(inline_alts(entered.project(), &owner_id), Some(original_alts));
    let action = entered.project().elements.get(4);
    assert!(action.is_some_and(|element| element.role == Role::Action));
    assert!(action.is_some_and(|element| inline_alts(entered.project(), &element.id).is_none()));
    assert_eq!(entered.project().dual_dialogue_groups[0].member_ids().count(), 3);
}

#[test]
fn wrap_unwrap_and_join_keep_only_the_surviving_owners_exact_alts() {
    let Some(fixture_doc) = imported(FIXTURE) else { return };
    let toggled_result = EditingSession::new(fixture_doc.project.clone());
    assert!(toggled_result.is_ok());
    let Some(mut toggled) = toggled_result.ok() else { return };
    let owner = toggled.project().elements[3].id.clone();
    let alts = inline_alts(toggled.project(), &owner);
    assert!(alts.is_some());
    let Some(alts) = alts else { return };
    assert!(toggled.set_selection(Selection::caret(Position { element: 3, offset: 0 })).is_ok());
    assert!(toggled.toggle_dual_dialogue().is_ok());
    assert!(toggled.project().dual_dialogue_groups.is_empty());
    assert_eq!(inline_alts(toggled.project(), &owner), Some(alts.clone()));
    assert!(toggled.toggle_dual_dialogue().is_ok());
    assert_eq!(toggled.project().dual_dialogue_groups.len(), 1);
    assert_eq!(inline_alts(toggled.project(), &owner), Some(alts));

    let source = paired_dialogues(true);
    let Some(pair) = imported(&source) else { return };
    let left_id = pair.project.elements[0].id.clone();
    let left_alts = inline_alts(&pair.project, &left_id);
    assert!(left_alts.is_some());
    let Some(left_alts) = left_alts else { return };
    let session_result = EditingSession::new(pair.project.clone());
    assert!(session_result.is_ok());
    let Ok(mut joined) = session_result else { return };
    assert!(joined.set_selection(Selection::caret(Position {
        element: 0,
        offset: joined.project().elements[0].text.len(),
    })).is_ok());
    assert!(joined.join_with_next().is_ok());
    assert_eq!(joined.project().elements[0].text, "leftright");
    assert_eq!(inline_alts(joined.project(), &left_id), Some(left_alts.clone()));
    let joined_project = joined.project().clone();
    assert!(joined.undo().is_ok());
    assert_eq!(joined.project(), &pair.project);
    assert!(joined.redo().is_ok());
    assert_eq!(joined.project(), &joined_project);
    let written = write_v6(joined.project());
    assert!(written.is_ok());
    if let Some(written) = written.ok() {
        let reopened = read_v6(&written);
        assert!(reopened.is_ok());
        if let Some(reopened) = reopened.ok() {
            assert_eq!(inline_alts(&reopened.project, &left_id), Some(left_alts));
        }
    }
}

#[test]
fn deleted_alts_owners_disappear_without_pruning_collection_on_backspace_delete_or_range() {
    for operation in 0..3 {
        let source = paired_dialogues(false);
        let Some(imported) = imported(&source) else { return };
        let project = imported.project.clone();
        let owner_id = project.elements[1].id.clone();
        let collection = alt_collection(&project);
        assert!(collection.is_some());
        let Some(collection) = collection else { return };
        let owner_alts = inline_alts(&project, &owner_id);
        assert!(owner_alts.is_some());
        let session_result = EditingSession::new(project.clone());
        assert!(session_result.is_ok());
        let Ok(mut session) = session_result else { return };
        match operation {
            0 => {
                assert!(session.set_selection(Selection::caret(Position { element: 1, offset: 0 })).is_ok());
                assert!(session.backspace().is_ok());
                assert_eq!(session.project().elements[0].text, "leftright");
            }
            1 => {
                let end = session.project().elements[0].text.len();
                assert!(session.set_selection(Selection::caret(Position { element: 0, offset: end })).is_ok());
                assert!(session.delete().is_ok());
            }
            _ => {
                assert!(session.set_selection(Selection {
                    anchor: Position { element: 0, offset: session.project().elements[0].text.len() },
                    focus: Position { element: 1, offset: session.project().elements[1].text.len() },
                }).is_ok());
                assert!(session.delete().is_ok());
            }
        }
        assert_eq!(session.project().elements.len(), 1);
        assert!(session.project().dual_dialogue_groups.is_empty());
        assert!(inline_alts(session.project(), &owner_id).is_none());
        assert_eq!(alt_collection(session.project()), Some(collection.clone()));
        let after = session.project().clone();
        assert!(session.undo().is_ok());
        assert_eq!(session.project(), &project);
        assert_eq!(inline_alts(session.project(), &owner_id), owner_alts);
        assert!(session.redo().is_ok());
        assert_eq!(session.project(), &after);
        assert_eq!(alt_collection(session.project()), Some(collection));
        let bytes = write_v6(session.project());
        assert!(bytes.is_ok());
        if let Some(bytes) = bytes.ok() {
            let reopened = read_v6(&bytes);
            assert!(reopened.is_ok());
            if let Some(reopened) = reopened.ok() {
                assert!(inline_alts(&reopened.project, &owner_id).is_none());
                assert_eq!(alt_collection(&reopened.project), alt_collection(session.project()));
            }
        }
    }
}

#[test]
fn deleting_a_dual_member_owner_removes_only_inline_alts_and_keeps_orphaned_copies() {
    let Some(fixture_doc) = imported(FIXTURE) else { return };
    let project = fixture_doc.project;
    let group = project.dual_dialogue_groups.first();
    assert!(group.is_some());
    let Some(group) = group else { return };
    let right_cue_id = group.right.character.clone();
    let member_id = group.right.paragraphs.first().cloned();
    assert!(member_id.is_some());
    let Some(member_id) = member_id else { return };
    let collection = alt_collection(&project);
    assert!(collection.is_some());
    let Some(collection) = collection else { return };
    let owner_alt = inline_alts(&project, &member_id);
    assert!(owner_alt.is_some());

    let right_cue_index = project.elements.iter().position(|element| element.id == right_cue_id);
    let member_index = project.elements.iter().position(|element| element.id == member_id);
    assert!(right_cue_index.is_some() && member_index.is_some());
    let (Some(right_cue_index), Some(member_index)) = (right_cue_index, member_index) else { return };
    let session_result = EditingSession::new(project.clone());
    assert!(session_result.is_ok());
    let Ok(mut session) = session_result else { return };
    assert!(session.set_selection(Selection {
        anchor: Position { element: right_cue_index, offset: session.project().elements[right_cue_index].text.len() },
        focus: Position { element: member_index, offset: session.project().elements[member_index].text.len() },
    }).is_ok());
    assert!(session.delete().is_ok());
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
    assert!(session.project().dual_dialogue_groups[0].left.paragraphs.is_empty());
    assert!(session.project().dual_dialogue_groups[0].right.paragraphs.is_empty());
    assert!(inline_alts(session.project(), &member_id).is_none());
    assert_eq!(alt_collection(session.project()), Some(collection.clone()));
    let deleted = session.project().clone();
    assert!(session.undo().is_ok());
    assert_eq!(inline_alts(session.project(), &member_id), owner_alt);
    assert!(session.redo().is_ok());
    assert_eq!(session.project(), &deleted);
    assert_eq!(alt_collection(session.project()), Some(collection));
}

#[test]
fn automatic_unwrap_moves_the_alts_owner_without_changing_its_attachment() {
    let Some(fixture_doc) = imported(FIXTURE) else { return };
    let project = fixture_doc.project;
    let owner_id = project.elements[3].id.clone();
    let original_alts = inline_alts(&project, &owner_id);
    assert!(original_alts.is_some());
    let Some(original_alts) = original_alts else { return };
    let cue_index = project.elements.iter().position(|element| element.role == Role::Character);
    assert!(cue_index.is_some());
    let Some(cue_index) = cue_index else { return };

    let session_result = EditingSession::new(project.clone());
    assert!(session_result.is_ok());
    let Ok(mut session) = session_result else { return };
    let cue_offset = session.project().elements[cue_index]
        .text
        .char_indices()
        .nth(1)
        .map_or(0, |(offset, _)| offset);
    assert!(session.set_selection(Selection::caret(Position {
        element: cue_index,
        offset: cue_offset,
    })).is_ok());
    assert!(session.split().is_ok());
    assert!(session.project().dual_dialogue_groups.is_empty());
    assert!(session.project().elements.iter().any(|element| element.id == owner_id));
    assert_eq!(inline_alts(session.project(), &owner_id), Some(original_alts.clone()));

    let unwrapped = session.project().clone();
    assert!(session.undo().is_ok());
    assert_eq!(session.project(), &project);
    assert!(session.redo().is_ok());
    assert_eq!(session.project(), &unwrapped);
    let bytes = write_v6(session.project());
    assert!(bytes.is_ok());
    if let Some(bytes) = bytes.ok() {
        let reopened = read_v6(&bytes);
        assert!(reopened.is_ok());
        if let Some(reopened) = reopened.ok() {
            assert!(reopened.project.dual_dialogue_groups.is_empty());
            assert_eq!(inline_alts(&reopened.project, &owner_id), Some(original_alts));
        }
    }
}

#[test]
fn role_change_rejects_transactionally_and_real_references_or_arbitrary_attachments_remain_safe_stops() {
    let Some(fixture_doc) = imported(FIXTURE) else { return };
    let session_result = EditingSession::new(fixture_doc.project);
    assert!(session_result.is_ok());
    let Ok(mut session) = session_result else { return };
    let owner_id = session.project().elements[3].id.clone();
    assert!(session.set_selection(Selection::caret(Position { element: 3, offset: 0 })).is_ok());
    let before = session.clone();
    assert!(session.change_role(Role::Action).is_err());
    assert_eq!(session, before, "rejected role change cannot unwrap or touch history/source state");
    assert!(session.change_role(Role::Dialogue).is_ok());
    assert_eq!(inline_alts(session.project(), &owner_id), inline_alts(before.project(), &owner_id));

    let reference_source = paired_dialogues(false).replace(
        "</Content>",
        "<Opaque ExternalRef=\"right-owner\"/></Content>",
    );
    let Some(referenced_project) = imported(&reference_source) else { return };
    let session_result = EditingSession::new(referenced_project.project);
    assert!(session_result.is_ok());
    let Ok(mut referenced) = session_result else { return };
    assert!(referenced.set_selection(Selection::caret(Position { element: 1, offset: 0 })).is_ok());
    let before_reference_delete = referenced.clone();
    assert!(referenced.backspace().is_err());
    assert_eq!(referenced, before_reference_delete);

    let attachment_source = paired_dialogues(false).replace(
        "</Paragraph></Content>",
        "<VendorData keep=\"opaque\"/></Paragraph></Content>",
    );
    let Some(attached_project) = imported(&attachment_source) else { return };
    let session_result = EditingSession::new(attached_project.project);
    assert!(session_result.is_ok());
    let Ok(mut attached) = session_result else { return };
    assert!(attached.set_selection(Selection::caret(Position { element: 1, offset: 0 })).is_ok());
    let before_attachment_delete = attached.clone();
    assert!(attached.backspace().is_err());
    assert_eq!(attached, before_attachment_delete);
}

#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::{EditError, EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::preservation::{PreservedNode, PreservationError};
use scriptcraft_core::{Project, Role};

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

fn role_type(role: Role) -> &'static str {
    match role {
        Role::SceneHeading => "Scene Heading",
        Role::Action => "Action",
        Role::Character => "Character",
        Role::Dialogue => "Dialogue",
        Role::Parenthetical => "Parenthetical",
        Role::Transition => "Transition",
        Role::Note => "Note",
        Role::NewAct => "New Act",
        Role::EndOfAct => "End of Act",
        Role::CastList => "Cast List",
        Role::General => "General",
        Role::Shot => "Shot",
        Role::Sequence => "Sequence",
        Role::Outline1 => "Outline 1",
        Role::Outline2 => "Outline 2",
        Role::Outline3 => "Outline 3",
        Role::Summary => "Summary",
    }
}

fn fixture_text(role: Role) -> &'static str {
    match role {
        Role::Character => "",
        Role::Dialogue => "",
        Role::Parenthetical => "()",
        _ => "source",
    }
}

fn imported(role: Role, attachments: &str) -> Project {
    let xml = format!(
        "<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content><Paragraph Type=\"{}\" ID=\"source-paragraph\"><Text>{}</Text>{}</Paragraph></Content></FinalDraft>",
        role_type(role),
        fixture_text(role),
        attachments,
    );
    read_v6(xml.as_bytes())
        .unwrap_or_else(|_| unreachable!("role fixture must import"))
        .project
}

fn select_end(session: &mut EditingSession) {
    let element = &session.project().elements[0];
    session
        .set_selection(Selection::caret(Position {
            element: 0,
            offset: element.text.len(),
        }))
        .unwrap_or_else(|_| unreachable!("fixture end is a valid caret"));
}

fn preserved_role(project: &Project) -> (&str, &str) {
    let paragraph = match &project
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("imported project retains preservation"))
        .sections[0]
        .children[0]
    {
        PreservedNode::Element(paragraph) => paragraph,
        _ => unreachable!("first content child is the managed paragraph"),
    };
    let type_attribute = paragraph
        .attributes
        .iter()
        .find(|attribute| attribute.name.local_name == "Type")
        .unwrap_or_else(|| unreachable!("managed paragraph retains Type"));
    (
        paragraph
            .source_type
            .as_deref()
            .unwrap_or_else(|| unreachable!("managed paragraph retains source Type")),
        type_attribute.value.as_str(),
    )
}

fn assert_role_sources(project: &Project, role: Role) {
    let expected = role_type(role);
    assert_eq!(project.elements[0].role, role);
    assert_eq!(
        project.elements[0]
            .source_provenance
            .as_ref()
            .and_then(|provenance| provenance.foreign_role.as_deref()),
        Some(expected)
    );
    assert_eq!(preserved_role(project), (expected, expected));
}

#[test]
fn every_imported_tab_transition_synchronizes_source_and_round_trips() {
    for (old_role, new_role) in TAB_MATRIX {
        let mut session = EditingSession::new(imported(old_role, ""))
            .unwrap_or_else(|_| unreachable!("imported project must form a session"));
        select_end(&mut session);
        let before = session.project().clone();

        session
            .tab()
            .unwrap_or_else(|_| unreachable!("supported imported Tab must succeed"));

        if old_role == new_role {
            assert_eq!(session.project(), &before, "same-role Tab is a no-op");
            assert!(!session.can_undo());
            continue;
        }

        assert_role_sources(session.project(), new_role);
        if new_role == Role::Parenthetical {
            assert_eq!(session.project().elements[0].text, "()");
        }
        assert!(session.can_undo());
        let changed = session.project().clone();
        assert_eq!(session.undo(), Ok(()));
        assert_eq!(session.project(), &before);
        assert_eq!(session.redo(), Ok(()));
        assert_eq!(session.project(), &changed);

        let bytes = write_v6(session.project())
            .unwrap_or_else(|_| unreachable!("synchronized role must serialize"));
        let reopened = read_v6(&bytes)
            .unwrap_or_else(|_| unreachable!("serialized role must reopen"))
            .project;
        assert_role_sources(&reopened, new_role);
        assert_eq!(reopened.elements[0].text, session.project().elements[0].text);
    }
}

#[test]
fn delimiter_only_parenthetical_delete_synchronizes_source_role() {
    let mut session = EditingSession::new(imported(Role::Parenthetical, ""))
        .unwrap_or_else(|_| unreachable!("imported project must form a session"));
    session
        .set_selection(Selection::caret(Position {
            element: 0,
            offset: 1,
        }))
        .unwrap_or_else(|_| unreachable!("parenthetical caret is valid"));

    session
        .delete()
        .unwrap_or_else(|_| unreachable!("supported delimiter-only deletion must succeed"));

    assert_role_sources(session.project(), Role::Dialogue);
    assert!(session.project().elements[0].text.is_empty());

    let bytes = write_v6(session.project())
        .unwrap_or_else(|_| unreachable!("synchronized deletion must serialize"));
    let reopened = read_v6(&bytes)
        .unwrap_or_else(|_| unreachable!("synchronized deletion must reopen"))
        .project;
    assert_role_sources(&reopened, Role::Dialogue);
    assert!(reopened.elements[0].text.is_empty());
}

#[test]
fn tab_role_mapping_failures_and_scene_attachments_roll_back() {
    let mut mismatched = imported(Role::Action, "");
    mismatched.elements[0]
        .source_provenance
        .as_mut()
        .unwrap_or_else(|| unreachable!("imported element has provenance"))
        .foreign_role = Some("Dialogue".into());
    let mut session = EditingSession::new(mismatched)
        .unwrap_or_else(|_| unreachable!("known provenance spelling remains valid"));
    select_end(&mut session);
    let before = session.clone();
    assert_eq!(
        session.tab(),
        Err(EditError::Preservation(
            PreservationError::UnsupportedRoleMapping
        ))
    );
    assert_eq!(session, before);
    assert!(!session.dirty());
    assert!(!session.can_undo());

    let attached_scene = "<SceneProperties ID=\"scene-1\"><SceneArcBeats><SceneArcBeat ID=\"beat-1\"><Text>authored beat</Text></SceneArcBeat></SceneArcBeats></SceneProperties>";
    let mut scene = EditingSession::new(imported(Role::SceneHeading, attached_scene))
        .unwrap_or_else(|_| unreachable!("attached scene fixture must form a session"));
    select_end(&mut scene);
    let before_scene = scene.clone();
    assert_eq!(
        scene.tab(),
        Err(EditError::Preservation(
            PreservationError::StructuralAttachmentConflict
        ))
    );
    assert_eq!(scene, before_scene);
    assert!(!scene.dirty());
    assert!(!scene.can_undo());
}

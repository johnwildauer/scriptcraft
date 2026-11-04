use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::Role;

const IMPORTED_ACTIONS: &[u8] = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Action" ID="before"><Text>before</Text></Paragraph>
<Paragraph Type="Action" ID="target"><Text>alpha beta</Text></Paragraph>
<Paragraph Type="Action" ID="after"><Text>after</Text></Paragraph>
</Content></FinalDraft>"#;

fn imported_session() -> EditingSession {
    let project = read_v6(IMPORTED_ACTIONS)
        .unwrap_or_else(|_| unreachable!("focused imported-Action fixture imports"))
        .project;
    EditingSession::new(project)
        .unwrap_or_else(|_| unreachable!("imported-Action fixture forms an editing session"))
}

fn caret(session: &mut EditingSession, element: usize, offset: usize) {
    session
        .set_selection(Selection::caret(Position { element, offset }))
        .unwrap_or_else(|_| unreachable!("focused paragraph caret is valid"));
}

fn insert_scene_heading_at(position: usize, use_enter: bool) {
    let mut session = imported_session();
    // Match the user's sequence: let the initial complete derived refresh finish
    // before performing the structural edit.
    session
        .refresh_cast_lists_with_outcome()
        .unwrap_or_else(|_| unreachable!("initial derived refresh is valid for this fixture"));
    let original_ids = session
        .project()
        .elements
        .iter()
        .map(|element| element.id.clone())
        .collect::<Vec<_>>();

    caret(&mut session, 1, position);
    if use_enter {
        session
            .enter()
            .unwrap_or_else(|_| unreachable!("Enter splits the imported Action paragraph"));
    } else {
        session
            .split()
            .unwrap_or_else(|_| unreachable!("split inserts into the imported Action paragraph"));
    }
    assert_eq!(session.project().elements[1].id, original_ids[1]);
    assert_eq!(session.project().elements.len(), 4);
    assert_ne!(session.project().elements[2].id, original_ids[1]);
    session
        .change_role(Role::SceneHeading)
        .unwrap_or_else(|_| unreachable!("newly inserted paragraph accepts Scene Heading role"));

    let inserted = session.project().clone();
    assert_eq!(inserted.elements[2].role, Role::SceneHeading);
    assert_eq!(inserted.elements[2].text, "alpha beta"[position..]);
    session
        .undo()
        .unwrap_or_else(|_| unreachable!("role transition has an undo snapshot"));
    assert_eq!(session.project().elements[2].role, Role::Action);
    session
        .redo()
        .unwrap_or_else(|_| unreachable!("role transition has a redo snapshot"));
    assert_eq!(session.project(), &inserted);

    // Exercise later ordinary edits on both sides of the new heading.
    let preceding_end = session.project().elements[0].text.len();
    caret(&mut session, 0, preceding_end);
    session
        .insert("!")
        .unwrap_or_else(|_| unreachable!("preceding imported Action remains editable"));
    let heading_end = session.project().elements[2].text.len();
    caret(&mut session, 2, heading_end);
    session
        .insert(" X")
        .unwrap_or_else(|_| unreachable!("new Scene Heading remains editable"));

    let bytes = write_v6(session.project())
        .unwrap_or_else(|_| unreachable!("edited imported FDX serializes"));
    let reopened = read_v6(&bytes)
        .unwrap_or_else(|_| unreachable!("edited FDX reopens"))
        .project;
    assert_eq!(reopened.elements[0].id, original_ids[0]);
    assert_eq!(reopened.elements[1].id, original_ids[1]);
    assert_eq!(reopened.elements[3].id, original_ids[2]);
    assert_eq!(reopened.elements[0].text, "before!");
    assert_eq!(reopened.elements[1].text, "alpha beta"[..position]);
    assert_eq!(reopened.elements[2].role, Role::SceneHeading);
    assert_eq!(reopened.elements[2].text, "alpha beta"[position..].to_owned() + " X");
    assert_eq!(reopened.elements[3].text, "after");
    assert!(reopened.elements[2].source_provenance.is_some());
}

#[test]
fn enter_scene_heading_insertions_at_action_start_interior_and_end_round_trip() {
    for position in [0, 5, "alpha beta".len()] {
        insert_scene_heading_at(position, true);
    }
}

#[test]
fn split_scene_heading_insertions_at_action_start_interior_and_end_round_trip() {
    for position in [0, 5, "alpha beta".len()] {
        insert_scene_heading_at(position, false);
    }
}

#[test]
fn unsupported_attached_source_rolls_back_structural_edit_exactly() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Action" ID="attached"><Text>alpha beta</Text><VendorPayload Marker="retain"/></Paragraph>
<Paragraph Type="Action" ID="after"><Text>after</Text></Paragraph>
</Content></FinalDraft>"#;
    let project = read_v6(source)
        .unwrap_or_else(|_| unreachable!("attached imported-Action fixture imports"))
        .project;
    let mut session = EditingSession::new(project)
        .unwrap_or_else(|_| unreachable!("attached fixture forms a session"));
    session
        .refresh_cast_lists_with_outcome()
        .unwrap_or_else(|_| unreachable!("initial derived refresh is valid for attached fixture"));
    caret(&mut session, 0, 5);
    let before = session.clone();

    assert!(session.enter().is_err());
    assert_eq!(session, before);
}

use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::preservation::{ForeignIdDomain, PreservationError};
use scriptcraft_core::Role;

const SOURCE: &[u8] = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Action" ID="p1"><Text Style="Bold">alpha</Text>
<Text Style="Italic">beta</Text></Paragraph>
<Paragraph Type="Custom" ID="scriptcraft-paragraph-1"><Text>opaque paragraph</Text></Paragraph>
<Paragraph Type="Dialogue" ID="p2"><Text>reply</Text></Paragraph>
<Paragraph Type="Action" ID="p3"><Text>end</Text></Paragraph>
</Content><ListItems><ListItem ID="item-1"/></ListItems><DisplayBoards><DisplayBoard ID="board-1"><Item id="item-1"/></DisplayBoard></DisplayBoards></FinalDraft>"#;

fn imported_session(source: &[u8]) -> EditingSession {
    let project = read_v6(source).unwrap_or_else(|_| unreachable!()).project;
    EditingSession::new(project).unwrap_or_else(|_| unreachable!())
}

fn select(session: &mut EditingSession, start: Position, end: Position) {
    session
        .set_selection(Selection {
            anchor: start,
            focus: end,
        })
        .unwrap_or_else(|_| unreachable!());
}

fn save_and_reopen(session: &EditingSession) -> (Vec<u8>, scriptcraft_core::Project) {
    let serialized = write_v6(session.project());
    assert!(serialized.is_ok(), "writer error: {serialized:?}");
    let bytes = serialized.unwrap_or_else(|_| unreachable!());
    let reopened = read_v6(&bytes).unwrap_or_else(|_| unreachable!()).project;
    (bytes, reopened)
}

#[test]
fn split_and_insert_keep_ids_runs_unknown_order_and_board_data_through_undo_redo() {
    let mut session = imported_session(SOURCE);
    let original = session.project().clone();
    select(
        &mut session,
        Position {
            element: 0,
            offset: 5,
        },
        Position {
            element: 0,
            offset: 5,
        },
    );
    assert_eq!(session.split(), Ok(()));
    assert_eq!(session.project().elements[0].text, "alpha");
    assert_eq!(session.project().elements[1].text, "beta");
    assert_eq!(session.project().elements[1].role, Role::Action);
    let state = session
        .project()
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!());
    let paragraph_ids = state
        .identities
        .iter()
        .filter(|identity| identity.domain == ForeignIdDomain::Paragraph)
        .map(|identity| identity.value.as_str())
        .collect::<Vec<_>>();
    assert!(paragraph_ids.contains(&"p1"));
    let generated_ids = paragraph_ids
        .iter()
        .filter_map(|id| {
            let parsed = uuid::Uuid::parse_str(id).ok()?;
            let canonical = parsed.hyphenated().to_string();
            (parsed.get_version() == Some(uuid::Version::Random)
                && parsed.get_variant() == uuid::Variant::RFC4122
                && *id == canonical)
                .then_some(*id)
        })
        .collect::<Vec<_>>();
    assert_eq!(generated_ids.len(), 2);
    assert_eq!(
        generated_ids
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        generated_ids.len()
    );
    assert_eq!(
        state.board_graph,
        original
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!())
            .board_graph
    );
    let (bytes, reopened) = save_and_reopen(&session);
    let xml = std::str::from_utf8(&bytes).unwrap_or("");
    assert!(xml.contains("opaque paragraph"));
    assert!(generated_ids
        .iter()
        .all(|generated_id| xml.contains(&format!("ID=\"{generated_id}\""))));
    let opaque_position = xml.find("opaque paragraph").unwrap_or(usize::MAX);
    assert!(generated_ids.iter().any(|generated_id| {
        opaque_position < xml.find(generated_id).unwrap_or(usize::MAX)
    }));
    assert!(xml.contains("ID=\"scriptcraft-paragraph-1\""));
    assert_eq!(
        reopened
            .elements
            .iter()
            .map(|element| (element.role, element.text.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (Role::Action, "alpha"),
            (Role::Action, "beta"),
            (Role::Dialogue, "reply"),
            (Role::Action, "end"),
        ]
    );
    assert_eq!(
        reopened
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!())
            .board_graph,
        state.board_graph
    );

    assert_eq!(session.undo(), Ok(()));
    assert_eq!(session.project(), &original);
    assert_eq!(session.redo(), Ok(()));
    assert_eq!(session.project().elements[0].text, "alpha");
    assert_eq!(session.project().elements[1].text, "beta");
    assert!(write_v6(session.project()).is_ok());
}

#[test]
fn joined_deleted_role_changed_and_cross_element_edits_round_trip() {
    let mut joined = imported_session(SOURCE);
    select(
        &mut joined,
        Position {
            element: 1,
            offset: 5,
        },
        Position {
            element: 1,
            offset: 5,
        },
    );
    assert_eq!(joined.join_with_next(), Ok(()));
    let (_, reopened) = save_and_reopen(&joined);
    assert_eq!(reopened.elements[1].text, "replyend");
    assert_eq!(reopened.elements.len(), 2);

    let mut role_changed = imported_session(SOURCE);
    select(
        &mut role_changed,
        Position {
            element: 0,
            offset: 0,
        },
        Position {
            element: 0,
            offset: 0,
        },
    );
    assert_eq!(role_changed.change_role(Role::Transition), Ok(()));
    let (_, reopened) = save_and_reopen(&role_changed);
    assert_eq!(reopened.elements[0].role, Role::Transition);
    assert_eq!(reopened.elements[0].text, "alphabeta");

    let mut cross_edit = imported_session(SOURCE);
    select(
        &mut cross_edit,
        Position {
            element: 0,
            offset: 2,
        },
        Position {
            element: 1,
            offset: 2,
        },
    );
    assert_eq!(cross_edit.insert("X"), Ok(()));
    let (_, reopened) = save_and_reopen(&cross_edit);
    assert_eq!(reopened.elements[0].text, "alXply");
    assert_eq!(reopened.elements.len(), 2);

    let mut delete = imported_session(SOURCE);
    select(
        &mut delete,
        Position {
            element: 0,
            offset: 0,
        },
        Position {
            element: 1,
            offset: 5,
        },
    );
    assert_eq!(delete.delete(), Ok(()));
    let (_, reopened) = save_and_reopen(&delete);
    assert_eq!(reopened.elements.len(), 2);
    assert_eq!(reopened.elements[0].text, "");
    assert_eq!(reopened.elements[0].role, Role::Action);
}

#[test]
fn unsafe_nested_payload_and_unknown_reference_reject_without_session_mutation() {
    let nested = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Scene Heading" ID="scene-paragraph"><Text>INT. ROOM</Text><SceneProperties ID="scene-1"><SceneArcBeats><SceneArcBeat ID="beat-1"><Text>authored</Text></SceneArcBeat></SceneArcBeats></SceneProperties></Paragraph>
<Paragraph Type="Action" ID="next"><Text>action</Text></Paragraph>
</Content></FinalDraft>"#;
    let mut session = imported_session(nested);
    select(
        &mut session,
        Position {
            element: 0,
            offset: 0,
        },
        Position {
            element: 0,
            offset: 0,
        },
    );
    let before = session.clone();
    assert_eq!(
        session.change_role(Role::Action),
        Err(scriptcraft_core::editing::EditError::Preservation(
            PreservationError::StructuralAttachmentConflict
        ))
    );
    assert_eq!(session, before);

    select(
        &mut session,
        Position {
            element: 0,
            offset: 4,
        },
        Position {
            element: 0,
            offset: 4,
        },
    );
    let before = session.clone();
    assert_eq!(
        session.split(),
        Err(scriptcraft_core::editing::EditError::Preservation(
            PreservationError::StructuralAttachmentConflict
        ))
    );
    assert_eq!(session, before);

    let with_reference = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Action" ID="one"><Text>one</Text></Paragraph><Paragraph Type="Dialogue" ID="two"><Text>two</Text></Paragraph>
</Content><FutureLinks><Link Target="two"/></FutureLinks></FinalDraft>"#;
    let mut referenced = imported_session(with_reference);
    select(
        &mut referenced,
        Position {
            element: 0,
            offset: 0,
        },
        Position {
            element: 1,
            offset: 3,
        },
    );
    let before = referenced.clone();
    assert_eq!(
        referenced.delete(),
        Err(scriptcraft_core::editing::EditError::Preservation(
            PreservationError::PotentialReferenceConflict
        ))
    );
    assert_eq!(referenced, before);
    assert!(!referenced.dirty());
    assert!(!referenced.can_undo());
}

#[test]
fn insertion_into_a_new_empty_paragraph_is_source_anchored_and_serializable() {
    let mut session = imported_session(SOURCE);
    select(
        &mut session,
        Position {
            element: 0,
            offset: 9,
        },
        Position {
            element: 0,
            offset: 9,
        },
    );
    assert_eq!(session.enter(), Ok(()));
    assert_eq!(session.project().elements.len(), 4);
    assert_eq!(session.insert("fresh"), Ok(()));
    let (_, reopened) = save_and_reopen(&session);
    assert_eq!(reopened.elements[1].text, "fresh");
    assert_eq!(reopened.elements[1].role, Role::Action);
    assert!(reopened.elements[1].source_provenance.is_some());
}

#[test]
fn in_paragraph_edits_keep_run_formatting_and_reject_opaque_crossings_atomically() {
    let mut session = imported_session(SOURCE);
    select(
        &mut session,
        Position {
            element: 0,
            offset: 5,
        },
        Position {
            element: 0,
            offset: 5,
        },
    );
    assert_eq!(session.insert("X"), Ok(()));
    let (_, reopened) = save_and_reopen(&session);
    assert_eq!(reopened.elements[0].text, "alphaXbeta");
    assert_eq!(reopened.elements[0].runs[0].attributes[0].value, "Bold");
    assert_eq!(reopened.elements[0].runs[1].attributes[0].value, "Italic");

    let with_opaque_run = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Action" ID="mixed"><Text>a</Text><UnknownInline ref="retained"/><Text>b</Text></Paragraph>
</Content></FinalDraft>"#;
    let mut mixed = imported_session(with_opaque_run);
    select(
        &mut mixed,
        Position {
            element: 0,
            offset: 0,
        },
        Position {
            element: 0,
            offset: 2,
        },
    );
    let before = mixed.clone();
    assert_eq!(
        mixed.insert("replacement"),
        Err(scriptcraft_core::editing::EditError::Preservation(
            PreservationError::UnsupportedTextMapping
        ))
    );
    assert_eq!(mixed, before);
    assert!(!mixed.dirty());
    assert!(!mixed.can_undo());
}

#[test]
fn script_deletion_does_not_block_on_or_rewrite_board_owned_cross_domain_data() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Action" ID="one"><Text>one</Text></Paragraph><Paragraph Type="Dialogue" ID="two"><Text>two</Text></Paragraph>
</Content><DisplayBoards><DisplayBoard ID="board"><ScriptLink Target="two"/><Item id="item-1"/></DisplayBoard></DisplayBoards><ListItems><ListItem ID="item-1"/></ListItems></FinalDraft>"#;
    let mut session = imported_session(source);
    let board_graph = session
        .project()
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!())
        .board_graph
        .clone();
    select(
        &mut session,
        Position {
            element: 0,
            offset: 0,
        },
        Position {
            element: 1,
            offset: 3,
        },
    );
    assert_eq!(session.delete(), Ok(()));
    let (bytes, reopened) = save_and_reopen(&session);
    assert!(std::str::from_utf8(&bytes)
        .unwrap_or("")
        .contains("Target=\"two\""));
    assert_eq!(
        reopened
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!())
            .board_graph,
        board_graph
    );
}

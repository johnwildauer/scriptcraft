use scriptcraft_core::{
    editing::{EditPolicy, EditingSession, Position, Selection},
    persistence::{deserialize, serialize},
    Project, Role,
};

fn session(role: Role, text: &str) -> EditingSession {
    let mut project = Project::new().unwrap_or_else(|_| unreachable!());
    project.elements[0].role = role;
    project.elements[0].text = text.into();
    EditingSession::new(project).unwrap_or_else(|_| unreachable!())
}

fn caret(session: &mut EditingSession, offset: usize) {
    session
        .set_selection(Selection::caret(Position { element: 0, offset }))
        .unwrap_or_else(|_| unreachable!());
}

#[test]
fn insertion_and_paste_normalize_cues_and_map_unicode_expansion() {
    let mut inserted = session(Role::SceneHeading, "");
    inserted
        .insert("int. room")
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(inserted.project().elements[0].text, "INT. ROOM");
    assert_eq!(inserted.selection().focus.offset, "INT. ROOM".len());

    let mut pasted = session(Role::Character, "");
    pasted.insert_paste("ǰa").unwrap_or_else(|_| unreachable!());
    assert_eq!(pasted.project().elements[0].text, "J̌A");
    assert_eq!(pasted.selection().focus.offset, "J̌A".len());

    let mut prefix = session(Role::Character, "x");
    caret(&mut prefix, 0);
    prefix.insert("ǰ").unwrap_or_else(|_| unreachable!());
    assert_eq!(prefix.project().elements[0].text, "J̌X");
    assert_eq!(prefix.selection().focus.offset, "J̌".len());
}

#[test]
fn role_assignment_and_tab_transition_normalize_existing_text_transactionally() {
    let mut assigned = session(Role::Action, "straße");
    caret(&mut assigned, "straße".len());
    assigned
        .change_role(Role::Character)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(assigned.project().elements[0].text, "STRASSE");
    assert!(assigned.can_undo());
    assigned.undo().unwrap_or_else(|_| unreachable!());
    assert_eq!(assigned.project().elements[0].text, "straße");
    assigned.redo().unwrap_or_else(|_| unreachable!());
    assert_eq!(assigned.project().elements[0].text, "STRASSE");

    let mut transitioned = session(Role::Transition, "cut to:");
    caret(&mut transitioned, "cut to:".len());
    transitioned.tab().unwrap_or_else(|_| unreachable!());
    assert_eq!(transitioned.project().elements[0].role, Role::SceneHeading);
    assert_eq!(transitioned.project().elements[0].text, "CUT TO:");
    assert_eq!(transitioned.selection().focus.offset, "CUT TO:".len());
}

#[test]
fn disabled_policy_and_non_cue_roles_preserve_source_casing() {
    let mut project = Project::new().unwrap_or_else(|_| unreachable!());
    project.elements[0].role = Role::SceneHeading;
    let mut disabled = EditingSession::new(project).unwrap_or_else(|_| unreachable!());
    let clean = disabled.project().clone();
    let revision = disabled.revision();
    disabled.set_policy(EditPolicy {
        uppercase_cues: false,
        ..disabled.policy()
    });
    assert_eq!(disabled.project(), &clean);
    assert_eq!(disabled.revision(), revision);
    assert!(!disabled.dirty());
    assert!(!disabled.can_undo());
    disabled.insert("MiXeD").unwrap_or_else(|_| unreachable!());
    assert_eq!(disabled.project().elements[0].text, "MiXeD");

    let mut action = session(Role::Action, "");
    action.insert("MiXeD").unwrap_or_else(|_| unreachable!());
    assert_eq!(action.project().elements[0].text, "MiXeD");
}

#[test]
fn runtime_policy_is_not_document_state_and_legacy_nondefault_json_is_rejected() {
    let mut project = Project::from_json(include_str!("fixtures/legacy_project.json"))
        .unwrap_or_else(|_| unreachable!());
    project.elements[0].role = Role::Character;
    project.elements[0].text = "saved lower text".into();
    let bytes = serialize(&project).unwrap_or_else(|_| unreachable!());
    let reopened = deserialize(&bytes).unwrap_or_else(|_| unreachable!());
    assert_eq!(reopened.elements[0].text, "saved lower text");
    assert_eq!(
        EditingSession::new(reopened)
            .unwrap_or_else(|_| unreachable!())
            .policy(),
        EditPolicy::default()
    );

    let text = String::from_utf8(bytes).unwrap_or_else(|_| unreachable!());
    let legacy = text.clone().replace(",\"uppercase_cues\":true", "");
    let decoded = Project::from_json(&legacy).unwrap_or_else(|_| unreachable!());
    assert_eq!(decoded.elements[0].text, "saved lower text");

    let incompatible = text.replace("\"uppercase_cues\":true", "\"uppercase_cues\":false");
    assert_eq!(
        Project::from_json(&incompatible),
        Err(scriptcraft_core::ModelError::UnsupportedJsonState)
    );
}

#[test]
fn fdx_import_preserves_mixed_case_until_a_writer_edit_applies_policy() {
    let fdx = b"<FinalDraft DocumentType='Script' Version='6'><Content><Paragraph Type='Character' id='cue-1'><Text>MiXeD</Text></Paragraph></Content></FinalDraft>";
    let imported = scriptcraft_core::fdx::read_v6(fdx)
        .unwrap_or_else(|_| unreachable!())
        .project;
    assert_eq!(imported.elements[0].text, "MiXeD");
    let session = EditingSession::new(imported).unwrap_or_else(|_| unreachable!());
    assert_eq!(session.project().elements[0].text, "MiXeD");
    assert_eq!(session.policy(), EditPolicy::default());
}

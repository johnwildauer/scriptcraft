#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::preservation::{PreservedAttribute, PreservedElement, PreservedNode};
use scriptcraft_core::{ParagraphAlignment, RunStyle};

const FIXTURE: &[u8] = include_bytes!("fixtures/user_paragraph_data.fdx");

fn valid<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("lifecycle case failed: {error:?}"))
}

fn attachments_by_paragraph_id(project: &scriptcraft_core::Project) -> BTreeMap<String, PreservedElement> {
    fn visit(nodes: &[PreservedNode], output: &mut BTreeMap<String, PreservedElement>) {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if element.name.local_name == "Paragraph" {
                let owner_id = element.attributes.iter().find(|attribute| {
                    attribute.name.namespace_uri.is_none()
                        && attribute.name.local_name.eq_ignore_ascii_case("id")
                }).map(|attribute| attribute.value.clone());
                if let Some(owner_id) = owner_id {
                    for child in &element.children {
                        if let PreservedNode::Element(attachment) = child {
                            if attachment.name.local_name == "UserParagraphData"
                                && attachment.name.namespace_uri == element.name.namespace_uri
                            {
                                output.insert(owner_id.clone(), attachment.clone());
                            }
                        }
                    }
                }
            }
            visit(&element.children, output);
        }
    }

    let mut output = BTreeMap::new();
    if let Some(preservation) = &project.preservation {
        for section in &preservation.sections {
            visit(&section.children, &mut output);
        }
    }
    output
}

fn paragraph_by_foreign_id(project: &scriptcraft_core::Project, id: &str) -> Option<PreservedElement> {
    fn visit(nodes: &[PreservedNode], id: &str) -> Option<PreservedElement> {
        for node in nodes {
            let PreservedNode::Element(element) = node else { continue };
            let found_id = element.attributes.iter().find(|attribute| {
                attribute.name.namespace_uri.is_none()
                    && attribute.name.local_name.eq_ignore_ascii_case("id")
            }).map(|attribute| attribute.value.as_str());
            if element.name.local_name == "Paragraph" && found_id == Some(id) {
                return Some(element.clone());
            }
            if let Some(found) = visit(&element.children, id) { return Some(found); }
        }
        None
    }
    project.preservation.as_ref()?.sections.iter()
        .find_map(|section| visit(&section.children, id))
}

fn owner_opaque_snapshot(paragraph: &PreservedElement) -> (Vec<PreservedAttribute>, Vec<PreservedNode>) {
    let attributes = paragraph.attributes.iter().filter(|attribute| {
        !(attribute.name.namespace_uri.is_none()
            && (attribute.name.local_name.eq_ignore_ascii_case("id")
                || attribute.name.local_name == "Type"))
    }).cloned().collect();
    let children = paragraph.children.iter().filter(|node| !matches!(node,
        PreservedNode::Element(element)
            if element.name.local_name == "Text"
                && element.name.namespace_uri == paragraph.name.namespace_uri
    )).cloned().collect();
    (attributes, children)
}

fn fixture_session() -> EditingSession {
    let imported = read_v6(FIXTURE).expect("synthetic lifecycle fixture imports");
    valid(EditingSession::new(imported.project))
}

fn assert_direct_roundtrip_attachments(session: &EditingSession) {
    let source = write_v6(session.project()).expect("edited source writes directly");
    let reopened = read_v6(&source).expect("edited source reimports");
    assert_eq!(
        attachments_by_paragraph_id(&reopened.project),
        attachments_by_paragraph_id(session.project()),
    );
}

fn assert_owner_opaque_roundtrip(session: &EditingSession, foreign_id: &str) {
    let expected = owner_opaque_snapshot(
        &paragraph_by_foreign_id(session.project(), foreign_id).expect("source owner exists"),
    );
    let bytes = write_v6(session.project()).expect("source writes");
    let reopened = read_v6(&bytes).expect("source rereads");
    let actual = owner_opaque_snapshot(
        &paragraph_by_foreign_id(&reopened.project, foreign_id).expect("reopened owner exists"),
    );
    assert_eq!(actual, expected);
}

fn assert_title_native_semantics_eq(
    actual: &[scriptcraft_core::TitlePageParagraph],
    expected: &[scriptcraft_core::TitlePageParagraph],
) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(actual.role, expected.role);
        assert_eq!(actual.text, expected.text);
        assert_eq!(actual.runs, expected.runs);
        assert_eq!(actual.formatting, expected.formatting);
        let actual_source = actual.source_provenance.as_ref().expect("actual title source");
        let expected_source = expected.source_provenance.as_ref().expect("expected title source");
        assert_eq!(actual_source.format, expected_source.format);
        assert_eq!(actual_source.section, expected_source.section);
        assert_eq!(actual_source.foreign_id, expected_source.foreign_id);
        assert_eq!(actual_source.foreign_role, expected_source.foreign_role);
    }
}

#[test]
fn body_typing_style_alignment_clear_retype_and_history_keep_the_owner_payload() {
    let mut session = fixture_session();
    let original_payloads = attachments_by_paragraph_id(session.project());
    let first_text = session.project().elements[0].text.clone();

    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: first_text.len(),
    })));
    valid(session.insert_paste(" revised"));
    let edited_len = session.project().elements[0].text.len();
    valid(session.set_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 0, offset: edited_len },
    }));
    valid(session.toggle_body_run_style(RunStyle::Bold));
    valid(session.set_body_alignment(ParagraphAlignment::Right));
    assert_eq!(attachments_by_paragraph_id(session.project()), original_payloads);
    assert_direct_roundtrip_attachments(&session);

    valid(session.insert(""));
    assert!(session.project().elements[0].text.is_empty());
    assert_eq!(attachments_by_paragraph_id(session.project()), original_payloads);
    assert_direct_roundtrip_attachments(&session);

    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.insert("retyped"));
    assert_eq!(attachments_by_paragraph_id(session.project()), original_payloads);
    valid(session.undo());
    assert!(session.project().elements[0].text.is_empty());
    assert_eq!(attachments_by_paragraph_id(session.project()), original_payloads);
    valid(session.redo());
    assert_eq!(session.project().elements[0].text, "retyped");
    assert_eq!(attachments_by_paragraph_id(session.project()), original_payloads);
    assert_direct_roundtrip_attachments(&session);
}

#[test]
fn scene_heading_role_transitions_keep_user_data_opaque_and_owner_bound() {
    let mut session = fixture_session();
    let before = attachments_by_paragraph_id(session.project());
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.change_role(scriptcraft_core::Role::SceneHeading));
    assert_eq!(session.project().elements[0].role, scriptcraft_core::Role::SceneHeading);
    assert_eq!(attachments_by_paragraph_id(session.project()), before);
    valid(session.change_role(scriptcraft_core::Role::Action));
    assert_eq!(session.project().elements[0].role, scriptcraft_core::Role::Action);
    assert_eq!(attachments_by_paragraph_id(session.project()), before);
    assert_direct_roundtrip_attachments(&session);
}

#[test]
fn generic_direct_opaque_children_and_bookmark_survive_ordinary_edits() {
    fn replace_first_attachment_name(nodes: &mut [PreservedNode]) -> bool {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if element.name.local_name == "Paragraph" {
                if let Some(PreservedNode::Element(attachment)) = element.children.iter_mut().find(|child| {
                    matches!(child, PreservedNode::Element(candidate)
                        if candidate.name.local_name == "UserParagraphData")
                }) {
                    attachment.name.local_name = "UnrelatedAttachment".to_owned();
                    return true;
                }
            }
            if replace_first_attachment_name(&mut element.children) {
                return true;
            }
        }
        false
    }

    let source = std::str::from_utf8(FIXTURE).expect("fixture is UTF-8")
        .replace("<Paragraph Type=\"Action\" id=\"body-001\">",
            "<Paragraph Type=\"Action\" id=\"body-001\" Bookmark=\"body-bookmark\" vendor-flag=\"keep\">");
    let mut project = read_v6(source.as_bytes()).expect("synthetic lifecycle fixture imports").project;
    let preservation = project.preservation.as_mut().expect("fixture has source preservation");
    assert!(preservation.sections.iter_mut().any(|section| {
        replace_first_attachment_name(&mut section.children)
    }));
    preservation.rebuild_indexes();

    let mut session = valid(EditingSession::new(project));
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    let before = paragraph_by_foreign_id(session.project(), "body-001")
        .expect("opaque-owner paragraph exists");
    let opaque_before = owner_opaque_snapshot(&before);
    valid(session.insert("x"));
    let after = paragraph_by_foreign_id(session.project(), "body-001")
        .expect("edited opaque-owner paragraph exists");
    assert_eq!(owner_opaque_snapshot(&after), opaque_before);
    valid(session.set_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 0, offset: 1 },
    }));
    valid(session.toggle_body_run_style(RunStyle::Bold));
    valid(session.set_body_alignment(ParagraphAlignment::Right));
    valid(session.change_role(scriptcraft_core::Role::General));
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(session.project(), "body-001").unwrap()), opaque_before);
    let edited_text_len = session.project().elements[0].text.len();
    valid(session.set_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 0, offset: edited_text_len },
    }));
    valid(session.insert(""));
    assert!(session.project().elements[0].text.is_empty());
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(session.project(), "body-001").unwrap()), opaque_before);
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.insert("retyped"));
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(session.project(), "body-001").unwrap()), opaque_before);
    valid(session.undo());
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(session.project(), "body-001").unwrap()), opaque_before);
    valid(session.redo());
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(session.project(), "body-001").unwrap()), opaque_before);
    let split_at = 1usize;
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: split_at })));
    valid(session.split());
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(session.project(), "body-001").unwrap()), opaque_before);
    let right_id = session.project().elements[1].source_provenance.as_ref()
        .and_then(|provenance| provenance.foreign_id.as_deref())
        .expect("right split paragraph gets a managed source identity")
        .to_owned();
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(session.project(), &right_id).unwrap()), (Vec::new(), Vec::new()));
    valid(session.undo());
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(session.project(), "body-001").unwrap()), opaque_before);
    valid(session.redo());
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(session.project(), "body-001").unwrap()), opaque_before);
    let serialized = write_v6(session.project()).expect("generic opaque source remains writable");
    let reopened = read_v6(&serialized).expect("edited source rereads");
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(&reopened.project, "body-001").unwrap()), opaque_before);
}

#[test]
fn unambiguous_run_edits_survive_opaque_inline_topology_but_crossing_edits_rollback() {
    let source = std::str::from_utf8(FIXTURE).expect("fixture is UTF-8")
        .replace("<Text>Opening action.</Text>",
            "<Text>Opening <InlineOpaque/>action.</Text><Text> tail</Text>");
    let imported = read_v6(source.as_bytes()).expect("opaque inline fixture imports");
    let mut session = valid(EditingSession::new(imported.project));
    let full_text = session.project().elements[0].text.clone();
    let opaque_owner_before = owner_opaque_snapshot(
        &paragraph_by_foreign_id(session.project(), "body-001").unwrap(),
    );

    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: full_text.len(),
    })));
    valid(session.insert("!"));
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(session.project(), "body-001").unwrap()), opaque_owner_before);
    let inline_after = paragraph_by_foreign_id(session.project(), "body-001").unwrap();
    assert!(inline_after.children.iter().any(|node| matches!(node,
        PreservedNode::Element(text) if text.name.local_name == "Text"
            && text.children.iter().any(|child| matches!(child,
                PreservedNode::Element(opaque) if opaque.name.local_name == "InlineOpaque"))
    )));

    valid(session.set_selection(Selection {
        anchor: Position { element: 0, offset: "Opening".len() },
        focus: Position { element: 0, offset: "Opening a".len() },
    }));
    let before_ambiguous_edit = session.clone();
    assert!(session.insert("").is_err());
    assert_eq!(session, before_ambiguous_edit);
}

#[test]
fn imported_empty_owner_removal_discards_subtree_history_and_allows_raw_orphan_roundtrip() {
    let source = r#"<FinalDraft DocumentType="Script" Version="6"><Content>
      <Paragraph Type="Action" id="empty-owner" Bookmark="owned-bookmark" vendor-owner="discard-me">
        <Text></Text><AltCollection><Alt><Paragraph Type="Action" id="empty-owner"><Text>alternate</Text></Paragraph></Alt></AltCollection>
        <ScriptNote ID="owned-note">note payload</ScriptNote>
        <UserParagraphData vendor="owner"><Payload id="opaque-id"><Paragraph id="empty-owner"/><!--owner comment--><?owner keep?><Opaque/></Payload></UserParagraphData>
      </Paragraph>
      <Paragraph Type="Action" id="survivor"><Text>Keep</Text></Paragraph>
      <Paragraph Type="Action" id="raw-use" vendor-ref="empty-owner"><Text>Raw reference</Text></Paragraph>
    </Content></FinalDraft>"#;
    let imported = read_v6(source.as_bytes()).expect("empty source-owner fixture imports");
    let mut session = valid(EditingSession::new(imported.project));
    let before = session.project().clone();
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.delete());
    let after = session.project().clone();

    assert_eq!(after.elements.len(), before.elements.len() - 1);
    assert!(paragraph_by_foreign_id(session.project(), "empty-owner").is_none());
    let preservation = session.project().preservation.as_ref().expect("source preservation remains");
    assert!(!preservation.identities.iter().any(|identity| identity.value == "empty-owner"));
    assert!(session.dirty());
    let serialized = write_v6(session.project()).expect("post-removal source writes");
    let serialized = std::str::from_utf8(&serialized).expect("writer emits UTF-8");
    assert!(!serialized.contains("owned-bookmark"));
    assert!(!serialized.contains("owned-note"));
    assert!(!serialized.contains("opaque-id"));
    assert!(!serialized.contains("owner comment"));
    assert!(!serialized.contains("owner keep"));
    assert!(serialized.contains("vendor-ref=\"empty-owner\""));
    let reread = read_v6(serialized.as_bytes()).expect("orphaned raw use rereads");
    assert_eq!(reread.project.elements, after.elements);
    assert!(paragraph_by_foreign_id(&reread.project, "empty-owner").is_none());

    valid(session.undo());
    assert_eq!(session.project(), &before);
    assert!(!session.dirty());
    valid(session.redo());
    assert_eq!(session.project(), &after);
    assert!(session.dirty());

    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.split());
    let recreated = session.project().elements[1].source_provenance.as_ref()
        .and_then(|provenance| provenance.foreign_id.as_deref())
        .expect("recreated element receives a source identity");
    assert_ne!(recreated, "empty-owner");
    let recreated_source = write_v6(session.project()).expect("recreated source writes");
    let recreated_source = std::str::from_utf8(&recreated_source).expect("writer emits UTF-8");
    assert!(!recreated_source.contains("owned-note"));
    assert!(!recreated_source.contains("opaque-id"));
    assert!(recreated_source.contains("vendor-ref=\"empty-owner\""));
    read_v6(recreated_source.as_bytes()).expect("recreated post-deletion source reopens");
}

#[test]
fn same_role_source_join_discards_only_right_owner_and_never_inherits_it_on_recreation() {
    let source = r#"<FinalDraft DocumentType="Script" Version="6"><Content>
      <Paragraph Type="Action" id="join-left" Bookmark="left-bookmark"><Text>Left</Text><UserParagraphData owner="left"><LeftPayload/></UserParagraphData></Paragraph>
      <Paragraph Type="Action" id="join-right" Alignment="Right" Bookmark="right-bookmark"><Text>Right</Text><UserParagraphData owner="right"><RightPayload id="right-opaque-id"/></UserParagraphData></Paragraph>
      <Paragraph Type="Action" id="raw-use" vendor-ref="join-right"><Text>Raw reference</Text></Paragraph>
    </Content></FinalDraft>"#;
    let imported = read_v6(source.as_bytes()).expect("same-role join fixture imports");
    let mut session = valid(EditingSession::new(imported.project));
    let before = session.project().clone();
    let removed_id = session.project().elements[1].id.clone();
    let left_owner = owner_opaque_snapshot(
        &paragraph_by_foreign_id(session.project(), "join-left").expect("left owner exists"),
    );
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 4 })));
    valid(session.delete());
    let after = session.project().clone();

    assert_eq!(session.project().elements[0].text, "LeftRight");
    assert_eq!(session.selection(), Selection::caret(Position { element: 0, offset: 4 }));
    assert!(!session.project().direct_paragraph_overrides.contains_key(&removed_id));
    assert!(!session.project().preservation.as_ref().expect("source preservation remains")
        .source_layout_settings().direct_paragraph_overrides.contains_key(&removed_id));
    assert_eq!(
        owner_opaque_snapshot(&paragraph_by_foreign_id(session.project(), "join-left").unwrap()),
        left_owner,
    );
    assert!(paragraph_by_foreign_id(session.project(), "join-right").is_none());
    let joined_source = write_v6(session.project()).expect("joined source writes");
    let joined_source = std::str::from_utf8(&joined_source).expect("writer emits UTF-8");
    assert!(joined_source.contains("vendor-ref=\"join-right\""));
    assert!(!joined_source.contains("right-bookmark"));
    assert!(!joined_source.contains("right-opaque-id"));
    let reopened = read_v6(joined_source.as_bytes()).expect("joined orphan source rereads");
    assert!(paragraph_by_foreign_id(&reopened.project, "join-right").is_none());
    assert_eq!(reopened.project.elements, after.elements);

    valid(session.undo());
    assert_eq!(session.project(), &before);
    valid(session.redo());
    assert_eq!(session.project(), &after);
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 4 })));
    valid(session.split());
    let recreated = session.project().elements[1].source_provenance.as_ref()
        .and_then(|provenance| provenance.foreign_id.as_deref())
        .expect("new right owner receives a fresh source identity");
    assert_ne!(recreated, "join-right");
    let recreated_source = write_v6(session.project()).expect("recreated joined source writes");
    let recreated_source = std::str::from_utf8(&recreated_source).expect("writer emits UTF-8");
    assert!(!recreated_source.contains("right-bookmark"));
    assert!(!recreated_source.contains("right-opaque-id"));
    assert!(recreated_source.contains("vendor-ref=\"join-right\""));
    read_v6(recreated_source.as_bytes()).expect("recreated source reopens with orphan intact");
}

#[test]
fn source_backed_dual_member_and_cue_removal_update_membership_wrapper_and_history() {
    let member_source = std::str::from_utf8(FIXTURE).expect("fixture is UTF-8")
        .replace("<Text>Hello there.</Text>", "<Text></Text>");
    let mut member_session = valid(EditingSession::new(
        read_v6(member_source.as_bytes()).expect("empty dual-member fixture imports").project,
    ));
    let member_before = member_session.project().clone();
    let removed_member = member_session.project().elements[2].id.clone();
    valid(member_session.set_selection(Selection::caret(Position { element: 2, offset: 0 })));
    valid(member_session.delete());
    let member_after = member_session.project().clone();
    assert_eq!(member_session.project().dual_dialogue_groups.len(), 1);
    assert!(member_session.project().dual_dialogue_groups[0].member_ids()
        .all(|id| id != &removed_member));
    assert!(paragraph_by_foreign_id(member_session.project(), "left-dialogue-001").is_none());
    assert!(paragraph_by_foreign_id(member_session.project(), "outer-dual-001").is_some());
    assert_direct_roundtrip_attachments(&member_session);
    valid(member_session.undo());
    assert_eq!(member_session.project(), &member_before);
    valid(member_session.redo());
    assert_eq!(member_session.project(), &member_after);

    let cue_source = std::str::from_utf8(FIXTURE).expect("fixture is UTF-8")
        .replace("<Text Style=\"AllCaps\">LINA</Text>", "<Text Style=\"AllCaps\"></Text>");
    let mut cue_session = valid(EditingSession::new(
        read_v6(cue_source.as_bytes()).expect("empty dual-cue fixture imports").project,
    ));
    let cue_before = cue_session.project().clone();
    let removed_group = cue_session.project().dual_dialogue_groups[0].id.clone();
    valid(cue_session.set_selection(Selection::caret(Position { element: 1, offset: 0 })));
    valid(cue_session.delete());
    let cue_after = cue_session.project().clone();
    assert!(cue_session.project().dual_dialogue_groups.is_empty());
    assert!(!cue_session.project().dual_dialogue_paragraph_overrides.contains_key(&removed_group));
    assert!(paragraph_by_foreign_id(cue_session.project(), "left-cue-001").is_none());
    assert!(paragraph_by_foreign_id(cue_session.project(), "outer-dual-001").is_none());
    for survivor in ["left-dialogue-001", "right-cue-001", "right-dialogue-001"] {
        assert!(paragraph_by_foreign_id(cue_session.project(), survivor).is_some());
    }
    assert_direct_roundtrip_attachments(&cue_session);
    valid(cue_session.undo());
    assert_eq!(cue_session.project(), &cue_before);
    valid(cue_session.redo());
    assert_eq!(cue_session.project(), &cue_after);
}

#[test]
fn body_split_keeps_payload_left_and_join_discards_only_the_removed_owner_payload() {
    let mut session = fixture_session();
    let before = attachments_by_paragraph_id(session.project());
    let first_text = session.project().elements[0].text.clone();
    let split_at = "Opening ".len();
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: split_at,
    })));
    valid(session.split());

    let right = &session.project().elements[1];
    let right_foreign_id = right.source_provenance.as_ref()
        .and_then(|provenance| provenance.foreign_id.as_deref())
        .expect("split right paragraph has a fresh source identity");
    let split_payloads = attachments_by_paragraph_id(session.project());
    assert_eq!(split_payloads.get("body-001"), before.get("body-001"));
    assert!(!split_payloads.contains_key(right_foreign_id));
    assert_direct_roundtrip_attachments(&session);
    valid(session.undo());
    assert_eq!(session.project().elements[0].text, first_text);
    assert_eq!(attachments_by_paragraph_id(session.project()), before);
    valid(session.redo());
    assert_eq!(attachments_by_paragraph_id(session.project()), split_payloads);

    let opaque_join_source = std::str::from_utf8(FIXTURE).expect("fixture is UTF-8")
        .replace("<Paragraph Type=\"Character\" id=\"left-cue-001\">",
            "<Paragraph Type=\"Character\" id=\"left-cue-001\" Bookmark=\"removed-owner\" vendor-owner=\"left-cue\"><OpaquePayload id=\"body-001\"><Nested/></OpaquePayload>");
    let mut join_session = valid(EditingSession::new(
        read_v6(opaque_join_source.as_bytes()).expect("generic opaque join fixture imports").project,
    ));
    let before_join = attachments_by_paragraph_id(join_session.project());
    let removed_owner_before = owner_opaque_snapshot(
        &paragraph_by_foreign_id(join_session.project(), "left-cue-001").unwrap(),
    );
    let survivor_text = join_session.project().elements[0].text.len();
    let removed_text = join_session.project().elements[1].text.len();
    valid(join_session.set_selection(Selection {
        anchor: Position { element: 0, offset: survivor_text },
        focus: Position { element: 1, offset: removed_text },
    }));
    valid(join_session.insert(""));
    let after_join = attachments_by_paragraph_id(join_session.project());
    assert_eq!(after_join.get("body-001"), before_join.get("body-001"));
    assert!(!after_join.contains_key("left-cue-001"));
    assert!(paragraph_by_foreign_id(join_session.project(), "left-cue-001").is_none());
    assert_eq!(removed_owner_before.0.iter().find(|attribute| attribute.name.local_name == "Bookmark").map(|attribute| attribute.value.as_str()), Some("removed-owner"));
    valid(join_session.undo());
    assert_eq!(attachments_by_paragraph_id(join_session.project()), before_join);
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(join_session.project(), "left-cue-001").unwrap()), removed_owner_before);
    valid(join_session.redo());
    assert_eq!(attachments_by_paragraph_id(join_session.project()), after_join);
    assert!(paragraph_by_foreign_id(join_session.project(), "left-cue-001").is_none());
    assert_direct_roundtrip_attachments(&join_session);

    let referenced_source = std::str::from_utf8(FIXTURE).expect("fixture is UTF-8")
        .replace("<Paragraph Type=\"Action\" id=\"body-001\">",
            "<Paragraph Type=\"Action\" id=\"body-001\"><OpaqueExternal ref=\"left-cue-001\"/>");
    let mut referenced_session = valid(EditingSession::new(
        read_v6(referenced_source.as_bytes()).expect("external-reference fixture imports").project,
    ));
    let survivor_len = referenced_session.project().elements[0].text.len();
    let removed_len = referenced_session.project().elements[1].text.len();
    valid(referenced_session.set_selection(Selection {
        anchor: Position { element: 0, offset: survivor_len },
        focus: Position { element: 1, offset: removed_len },
    }));
    valid(referenced_session.insert(""));
    assert!(paragraph_by_foreign_id(referenced_session.project(), "left-cue-001").is_none());
    let orphan_bytes = write_v6(referenced_session.project()).expect("raw orphan source writes");
    let orphan_source = std::str::from_utf8(&orphan_bytes).expect("writer emits UTF-8");
    assert!(orphan_source.contains("ref=\"left-cue-001\""));
    let orphan_reopened = read_v6(orphan_source.as_bytes()).expect("raw orphan source rereads");
    assert!(paragraph_by_foreign_id(&orphan_reopened.project, "left-cue-001").is_none());
}

#[test]
fn user_data_split_handles_multiple_utf8_safe_boundaries_without_copying() {
    let source = std::str::from_utf8(FIXTURE).expect("fixture is UTF-8")
        .replace("<Text>Opening action.</Text>", "<Text>Opening café.</Text>")
        .replace("<Paragraph Type=\"Action\" id=\"body-001\">",
            "<Paragraph Type=\"Action\" id=\"body-001\" Bookmark=\"utf8-left\">")
        .replace("<UserParagraphData vendor=\"body\"",
            "<OpaqueUtf8Payload value=\"left-only\"/><UserParagraphData vendor=\"body\"");
    for split_at in ["Opening ".len(), "Opening café".len()] {
        let imported = read_v6(source.as_bytes()).expect("UTF-8 split fixture imports");
        let mut session = valid(EditingSession::new(imported.project));
        let before = attachments_by_paragraph_id(session.project());
        let opaque_before = owner_opaque_snapshot(&paragraph_by_foreign_id(session.project(), "body-001").unwrap());
        valid(session.set_selection(Selection::caret(Position { element: 0, offset: split_at })));
        valid(session.split());
        let after = attachments_by_paragraph_id(session.project());
        assert_eq!(after.get("body-001"), before.get("body-001"));
        assert_eq!(after.len(), before.len());
        assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(session.project(), "body-001").unwrap()), opaque_before);
        let right_id = session.project().elements[1].source_provenance.as_ref()
            .and_then(|provenance| provenance.foreign_id.as_deref())
            .expect("right UTF-8 split owner has an identity");
        assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(session.project(), right_id).unwrap()), (Vec::new(), Vec::new()));
        assert_eq!(session.project().elements[0].text.len(), split_at);
        valid(session.undo());
        assert_eq!(attachments_by_paragraph_id(session.project()), before);
        valid(session.redo());
        assert_eq!(attachments_by_paragraph_id(session.project()), after);
        assert_direct_roundtrip_attachments(&session);
    }
}

#[test]
fn empty_title_owner_removal_needs_no_text_recipient_and_restores_history() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="body"><OpaqueExternal ref="title-empty"/><Text>Body</Text></Paragraph></Content><TitlePage><Content><Paragraph Type="Title" id="title-empty" Bookmark="discard-title"><Text></Text><VendorTitlePayload id="discard-title-id"/><UserParagraphData owner="discard"><Opaque/></UserParagraphData></Paragraph><Paragraph Type="Title" id="title-survivor"><Text>Surviving card</Text></Paragraph></Content></TitlePage></FinalDraft>"#;
    let imported = read_v6(source).expect("empty title-owner fixture imports");
    let mut session = valid(EditingSession::new(imported.project));
    let before = session.project().clone();
    valid(session.set_title_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.delete_title());
    let after = session.project().clone();

    assert_eq!(session.project().title_page.len(), 1);
    assert_eq!(session.project().title_page[0].text, "Surviving card");
    assert_eq!(session.title_selection(), Selection::caret(Position { element: 0, offset: 0 }));
    assert!(paragraph_by_foreign_id(session.project(), "title-empty").is_none());
    let bytes = write_v6(session.project()).expect("title owner-removal source writes");
    let serialized = std::str::from_utf8(&bytes).expect("writer emits UTF-8");
    assert!(!serialized.contains("discard-title"));
    assert!(!serialized.contains("discard-title-id"));
    assert!(serialized.contains("ref=\"title-empty\""));
    let reopened = read_v6(serialized.as_bytes()).expect("title owner-removal source reopens");
    assert_title_native_semantics_eq(&reopened.project.title_page, &after.title_page);
    valid(session.undo());
    assert_eq!(session.project(), &before);
    valid(session.redo());
    assert_eq!(session.project(), &after);
}

#[test]
fn consecutive_empty_title_owners_delete_without_survivor_text_mapping_constraints() {
    for backward in [false, true] {
        let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="body" vendor-ref="title-empty-second"><Text>Body</Text></Paragraph></Content><TitlePage><Content><Paragraph Type="Title" id="title-empty-first"><Text/></Paragraph><Paragraph Type="Title" id="title-empty-second" Bookmark="discard-title"><Text Style="Bold"></Text><UserParagraphData owner="discard"><Opaque id="discard-title-id"/></UserParagraphData></Paragraph><Paragraph Type="Title" id="title-survivor"><Text>Keep</Text></Paragraph></Content></TitlePage></FinalDraft>"#;
        let imported = read_v6(source).expect("consecutive empty title fixture imports");
        let mut session = valid(EditingSession::new(imported.project));
        let before = session.project().clone();
        valid(session.set_title_selection(Selection::caret(Position { element: 1, offset: 0 })));
        if backward {
            valid(session.backspace_title());
        } else {
            valid(session.delete_title());
        }
        let after = session.project().clone();

        assert_eq!(session.project().title_page.len(), 2);
        assert!(session.project().title_page[0].text.is_empty());
        assert_eq!(session.project().title_page[1].text, "Keep");
        assert!(paragraph_by_foreign_id(session.project(), "title-empty-second").is_none());
        let bytes = write_v6(session.project()).expect("consecutive empty title result writes");
        let serialized = std::str::from_utf8(&bytes).expect("writer emits UTF-8");
        assert!(!serialized.contains("discard-title"));
        assert!(!serialized.contains("discard-title-id"));
        assert!(serialized.contains("vendor-ref=\"title-empty-second\""));
        let reopened = read_v6(serialized.as_bytes()).expect("consecutive empty title result rereads");
        assert_title_native_semantics_eq(&reopened.project.title_page, &after.title_page);
        valid(session.undo());
        assert_eq!(session.project(), &before);
        valid(session.redo());
        assert_eq!(session.project(), &after);
    }
}

#[test]
fn title_empty_deletions_at_first_interior_and_last_positions_keep_exact_history_and_native_output() {
    for backward in [false, true] {
        for empty_index in [0usize, 2, 4] {
            let mut titles = String::new();
            for index in 0..5 {
                let text = if index == empty_index {
                    String::new()
                } else {
                    format!("Title {index}")
                };
                titles.push_str(&format!(
                    "<Paragraph Type=\"Title\" id=\"title-{index}\"><Text>{text}</Text></Paragraph>"
                ));
            }
            let source = format!(
                "<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content><Paragraph Type=\"Action\" id=\"body\"><Text>Body</Text></Paragraph></Content><TitlePage><Content>{titles}</Content></TitlePage></FinalDraft>"
            );
            let imported = read_v6(source.as_bytes()).expect("title boundary fixture imports");
            let mut session = valid(EditingSession::new(imported.project));
            let before = session.project().clone();
            let initial_revision = session.revision();
            valid(session.set_title_selection(Selection::caret(Position {
                element: empty_index,
                offset: 0,
            })));
            if backward {
                valid(session.backspace_title());
            } else {
                valid(session.delete_title());
            }
            let after = session.project().clone();
            assert_eq!(after.title_page.len(), 4);
            assert_eq!(session.revision(), initial_revision + 1);
            assert!(session.dirty());
            assert!(session.can_undo());

            let expected_caret = if backward && empty_index > 0 {
                Position {
                    element: empty_index - 1,
                    offset: format!("Title {}", empty_index - 1).len(),
                }
            } else if !backward && empty_index == 4 {
                Position {
                    element: 3,
                    offset: "Title 3".len(),
                }
            } else {
                Position {
                    element: empty_index,
                    offset: 0,
                }
            };
            assert_eq!(session.title_selection(), Selection::caret(expected_caret));
            let bytes = write_v6(session.project()).expect("title boundary result writes");
            let reopened = read_v6(&bytes).expect("title boundary result rereads");
            assert_title_native_semantics_eq(&reopened.project.title_page, &after.title_page);

            valid(session.undo());
            assert_eq!(session.project(), &before);
            assert!(!session.dirty());
            assert!(session.can_redo());
            valid(session.redo());
            assert_eq!(session.project(), &after);
            assert!(session.dirty());
        }
    }
}

#[test]
fn title_join_with_removed_formatting_and_subsequent_split_roundtrip() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="body" vendor-ref="title-right"><Text>Body</Text></Paragraph></Content><TitlePage><Content><Paragraph Type="Title" id="title-left"><Text>Left</Text><UserParagraphData owner="left"/></Paragraph><Paragraph Type="Title" id="title-right" Alignment="Right" Bookmark="discard-title"><Text>Right</Text><UserParagraphData owner="right"><Opaque id="discard-title-id"/></UserParagraphData></Paragraph></Content></TitlePage></FinalDraft>"#;
    let imported = read_v6(source).expect("title formatting join fixture imports");
    let mut session = valid(EditingSession::new(imported.project));
    let before = session.project().clone();
    valid(session.set_title_selection(Selection::caret(Position { element: 1, offset: 0 })));
    valid(session.backspace_title());
    let joined = session.project().clone();

    assert_eq!(session.project().title_page.len(), 1);
    assert_eq!(session.project().title_page[0].text, "LeftRight");
    assert_eq!(session.title_selection(), Selection::caret(Position { element: 0, offset: 4 }));
    assert!(paragraph_by_foreign_id(session.project(), "title-right").is_none());
    valid(session.undo());
    assert_eq!(session.project(), &before);
    valid(session.redo());
    assert_eq!(session.project(), &joined);

    valid(session.set_title_selection(Selection::caret(Position { element: 0, offset: 4 })));
    valid(session.enter_title());
    assert_eq!(session.project().title_page.len(), 2);
    assert_eq!(session.project().title_page[0].text, "Left");
    assert_eq!(session.project().title_page[1].text, "Right");
    let recreated = session.project().title_page[1].source_provenance.as_ref()
        .and_then(|provenance| provenance.foreign_id.as_deref())
        .expect("split title owner receives a fresh foreign identity");
    assert_ne!(recreated, "title-right");

    let bytes = write_v6(session.project()).expect("title join/split result writes");
    let serialized = std::str::from_utf8(&bytes).expect("writer emits UTF-8");
    assert!(!serialized.contains("discard-title"));
    assert!(!serialized.contains("discard-title-id"));
    assert!(serialized.contains("vendor-ref=\"title-right\""));
    let reopened = read_v6(serialized.as_bytes()).expect("title join/split result rereads");
    assert_title_native_semantics_eq(&reopened.project.title_page, &session.project().title_page);
}

#[test]
fn title_split_join_and_dual_unwrap_follow_distinct_owner_lifetimes() {
    let source = std::str::from_utf8(FIXTURE).expect("fixture is UTF-8");
    let title_two = source
        .replace("<Paragraph Type=\"Title\" id=\"title-001\">",
            "<Paragraph Type=\"Title\" id=\"title-001\" Bookmark=\"title-left\" vendor-owner=\"left-only\">")
        .replace("<UserParagraphData owner=\"title\">",
            "<VendorTitlePayload flag=\"left-owner\"/><UserParagraphData owner=\"title\">")
        .replace(
            "</Content>\n  </TitlePage>",
            "<Paragraph Type=\"Title\" id=\"title-002\" Bookmark=\"title-right\" vendor-owner=\"right-only\"><Text>Second card</Text><UserParagraphData owner=\"title-two\"><Extra/></UserParagraphData></Paragraph></Content>\n  </TitlePage>",
        );
    let imported = read_v6(title_two.as_bytes()).expect("two-owner title fixture imports");
    let mut title_session = valid(EditingSession::new(imported.project));
    let title_before = attachments_by_paragraph_id(title_session.project());
    let title_owner_before = owner_opaque_snapshot(
        &paragraph_by_foreign_id(title_session.project(), "title-001").unwrap(),
    );
    let initial_title_len = title_session.project().title_page[0].text.len();
    valid(title_session.set_title_selection(Selection::caret(Position {
        element: 0,
        offset: initial_title_len,
    })));
    valid(title_session.insert_title(" revised"));
    let edited_title_len = title_session.project().title_page[0].text.len();
    valid(title_session.set_title_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 0, offset: edited_title_len },
    }));
    valid(title_session.toggle_title_run_style(RunStyle::Bold));
    valid(title_session.set_title_alignment(ParagraphAlignment::Right));
    assert_eq!(attachments_by_paragraph_id(title_session.project()), title_before);
    assert_direct_roundtrip_attachments(&title_session);
    let title_text = title_session.project().title_page[0].text.clone();
    valid(title_session.set_title_selection(Selection::caret(Position {
        element: 0,
        offset: "A Small ".len(),
    })));
    valid(title_session.enter_title());
    let split_right_id = title_session.project().title_page[1].source_provenance.as_ref()
        .and_then(|provenance| provenance.foreign_id.as_deref())
        .expect("split title paragraph has a fresh source identity")
        .to_owned();
    let title_split = attachments_by_paragraph_id(title_session.project());
    assert_eq!(title_split.get("title-001"), title_before.get("title-001"));
    assert!(!title_split.contains_key(&split_right_id));
    let split_right = paragraph_by_foreign_id(title_session.project(), &split_right_id)
        .expect("new right title owner exists");
    assert!(owner_opaque_snapshot(&split_right).0.iter().all(|attribute| {
        attribute.name.local_name != "Bookmark" && attribute.name.local_name != "vendor-owner"
    }));
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(title_session.project(), "title-001").unwrap()), title_owner_before);
    assert_owner_opaque_roundtrip(&title_session, "title-001");
    valid(title_session.undo());
    assert_eq!(title_session.project().title_page[0].text, title_text);
    assert_eq!(attachments_by_paragraph_id(title_session.project()), title_before);
    valid(title_session.redo());
    assert_eq!(attachments_by_paragraph_id(title_session.project()), title_split);
    assert_direct_roundtrip_attachments(&title_session);

    let last_title = title_session.project().title_page.len() - 1;
    let survivor_len = title_session.project().title_page[0].text.len();
    let removed_len = title_session.project().title_page[last_title].text.len();
    valid(title_session.set_title_selection(Selection {
        anchor: Position { element: 0, offset: survivor_len },
        focus: Position { element: last_title, offset: removed_len },
    }));
    valid(title_session.insert_title(""));
    let title_joined = attachments_by_paragraph_id(title_session.project());
    assert_eq!(title_joined.get("title-001"), title_before.get("title-001"));
    assert!(!title_joined.contains_key("title-002"));
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(title_session.project(), "title-001").unwrap()), title_owner_before);
    assert!(paragraph_by_foreign_id(title_session.project(), "title-002").is_none());
    assert_owner_opaque_roundtrip(&title_session, "title-001");
    valid(title_session.undo());
    assert_eq!(attachments_by_paragraph_id(title_session.project()), title_split);
    valid(title_session.redo());
    assert_eq!(attachments_by_paragraph_id(title_session.project()), title_joined);
    assert_direct_roundtrip_attachments(&title_session);

    let managed_conflict_source = source.replace(
        "</Content>\n  </TitlePage>",
        "<Paragraph Type=\"Title\" id=\"title-conflict\" Alignment=\"Right\"><Text>Conflicting card</Text></Paragraph></Content>\n  </TitlePage>",
    );
    let mut conflict_session = valid(EditingSession::new(
        read_v6(managed_conflict_source.as_bytes()).expect("managed-format conflict fixture imports").project,
    ));
    let first_len = conflict_session.project().title_page[0].text.len();
    let second_index = conflict_session.project().title_page.len() - 1;
    let second_len = conflict_session.project().title_page[second_index].text.len();
    valid(conflict_session.set_title_selection(Selection {
        anchor: Position { element: 0, offset: first_len },
        focus: Position { element: second_index, offset: second_len },
    }));
    let before_managed_conflict = conflict_session.clone();
    valid(conflict_session.insert_title(""));
    let after_managed_conflict = conflict_session.clone();
    assert_eq!(conflict_session.project().title_page.len(), 1);
    assert!(paragraph_by_foreign_id(conflict_session.project(), "title-conflict").is_none());
    let joined = conflict_session.project().title_page[0].text.clone();
    let joined_bytes = write_v6(conflict_session.project())
        .expect("title structure writes from current semantics");
    let joined_reopened = read_v6(&joined_bytes).expect("title structure rereads");
    assert_eq!(joined_reopened.project.title_page[0].text, joined);
    valid(conflict_session.undo());
    assert_eq!(conflict_session, before_managed_conflict);
    valid(conflict_session.redo());
    assert_eq!(conflict_session, after_managed_conflict);

    let dual_source = std::str::from_utf8(FIXTURE).expect("fixture is UTF-8")
        .replace("<Paragraph Type=\"Dialogue\" id=\"left-dialogue-001\">",
            "<Paragraph Type=\"Dialogue\" id=\"left-dialogue-001\" Bookmark=\"member-bookmark\" vendor-member=\"kept\"><OpaqueMemberData ref=\"member-source\"/>");
    let mut dual_split_session = valid(EditingSession::new(
        read_v6(dual_source.as_bytes()).expect("generic dual-member fixture imports").project,
    ));
    let dual_member_before = owner_opaque_snapshot(
        &paragraph_by_foreign_id(dual_split_session.project(), "left-dialogue-001").unwrap(),
    );
    valid(dual_split_session.set_selection(Selection::caret(Position { element: 2, offset: 5 })));
    valid(dual_split_session.split());
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(dual_split_session.project(), "left-dialogue-001").unwrap()), dual_member_before);
    let dual_right_id = dual_split_session.project().elements[3].source_provenance.as_ref()
        .and_then(|provenance| provenance.foreign_id.as_deref())
        .expect("split dual member receives a fresh source identity");
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(dual_split_session.project(), dual_right_id).unwrap()), (Vec::new(), Vec::new()));
    valid(dual_split_session.undo());
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(dual_split_session.project(), "left-dialogue-001").unwrap()), dual_member_before);
    valid(dual_split_session.redo());
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(dual_split_session.project(), "left-dialogue-001").unwrap()), dual_member_before);

    let mut dual_session = valid(EditingSession::new(
        read_v6(dual_source.as_bytes()).expect("generic dual-member fixture imports").project,
    ));
    let member_opaque_before = owner_opaque_snapshot(
        &paragraph_by_foreign_id(dual_session.project(), "left-dialogue-001").unwrap(),
    );
    let member_text_len = dual_session.project().elements[2].text.len();
    valid(dual_session.set_selection(Selection::caret(Position {
        element: 2,
        offset: member_text_len,
    })));
    valid(dual_session.insert("!"));
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(dual_session.project(), "left-dialogue-001").unwrap()), member_opaque_before);
    let dual_before = attachments_by_paragraph_id(dual_session.project());
    assert_eq!(dual_before.len(), 7);
    assert_direct_roundtrip_attachments(&dual_session);
    valid(dual_session.set_selection(Selection::caret(Position { element: 1, offset: 0 })));
    valid(dual_session.toggle_dual_dialogue());
    let unwrapped = attachments_by_paragraph_id(dual_session.project());
    assert!(!unwrapped.contains_key("outer-dual-001"));
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(dual_session.project(), "left-dialogue-001").unwrap()), member_opaque_before);
    for owner in ["left-cue-001", "left-dialogue-001", "right-cue-001", "right-dialogue-001"] {
        assert_eq!(unwrapped.get(owner), dual_before.get(owner));
    }
    valid(dual_session.undo());
    assert_eq!(attachments_by_paragraph_id(dual_session.project()), dual_before);
    valid(dual_session.redo());
    assert_eq!(attachments_by_paragraph_id(dual_session.project()), unwrapped);
    assert_direct_roundtrip_attachments(&dual_session);
}

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::preservation::{
    AnchorRelation, AnchorTarget, ForeignIdDomain, ForeignIdentity, ForeignReference,
    PreservedAttribute, PreservedElement, PreservedNode, ReferencePolicy, SourceAnchor,
    SourceAnchorId,
};
use scriptcraft_core::{
    ParagraphAlignment, Project, Role, RunStyle, ScopedEditValidationOutcome,
};

const FIXTURE: &[u8] = include_bytes!("fixtures/user_paragraph_data.fdx");

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("lifecycle case uses supported input"))
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

fn source_paragraph_by_foreign_id_mut<'a>(
    nodes: &'a mut [PreservedNode],
    id: &str,
) -> Option<&'a mut PreservedElement> {
    for node in nodes {
        let PreservedNode::Element(element) = node else { continue };
        let foreign_id = element.attributes.iter().find(|attribute| {
            attribute.name.namespace_uri.is_none()
                && attribute.name.local_name.eq_ignore_ascii_case("id")
        }).map(|attribute| attribute.value.as_str());
        if element.name.local_name == "Paragraph" && foreign_id == Some(id) {
            return Some(element);
        }
        if let Some(found) = source_paragraph_by_foreign_id_mut(&mut element.children, id) {
            return Some(found);
        }
    }
    None
}

fn remove_source_paragraph_by_foreign_id(nodes: &mut Vec<PreservedNode>, id: &str) -> bool {
    if let Some(index) = nodes.iter().position(|node| matches!(node,
        PreservedNode::Element(element) if element.name.local_name == "Paragraph"
            && element.attributes.iter().any(|attribute| {
                attribute.name.namespace_uri.is_none()
                    && attribute.name.local_name.eq_ignore_ascii_case("id")
                    && attribute.value == id
            })
    )) {
        nodes.remove(index);
        return true;
    }
    for node in nodes {
        if let PreservedNode::Element(element) = node {
            if remove_source_paragraph_by_foreign_id(&mut element.children, id) {
                return true;
            }
        }
    }
    false
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

fn source_new_pair(source: &[u8]) -> (EditingSession, EditingSession) {
    let imported = read_v6(source).expect("source-backed parity fixture imports").project;
    let mut source_less = imported.clone();
    source_less.preservation = None;
    for element in &mut source_less.elements {
        element.source_provenance = None;
    }
    for paragraph in &mut source_less.title_page {
        paragraph.source_provenance = None;
    }
    (
        valid(EditingSession::new(imported)),
        valid(EditingSession::new(source_less)),
    )
}

fn assert_source_new_parity(backed: &EditingSession, source_less: &EditingSession) {
    let semantic_state = |project: &Project| {
        (
            project
                .elements
                .iter()
                .map(|element| (element.role, element.text.clone(), element.runs.clone()))
                .collect::<Vec<_>>(),
            project
                .title_page
                .iter()
                .map(|paragraph| (paragraph.text.clone(), paragraph.runs.clone()))
                .collect::<Vec<_>>(),
            project.direct_paragraph_overrides.clone(),
        )
    };
    assert_eq!(semantic_state(backed.project()), semantic_state(source_less.project()));
    assert_eq!(backed.selection(), source_less.selection());
    assert_eq!(backed.title_selection(), source_less.title_selection());
    assert_eq!(backed.dirty(), source_less.dirty());
    assert_eq!(backed.can_undo(), source_less.can_undo());
    assert_eq!(backed.can_redo(), source_less.can_redo());

    let backed_bytes = write_v6(backed.project()).expect("source-backed current projection serializes");
    let detached_bytes = write_v6(source_less.project()).expect("source-less current projection serializes");
    let backed_reopened = read_v6(&backed_bytes).expect("source-backed output rereads");
    let detached_reopened = read_v6(&detached_bytes).expect("source-less output rereads");
    assert_eq!(semantic_state(&backed_reopened.project), semantic_state(backed.project()));
    assert_eq!(semantic_state(&detached_reopened.project), semantic_state(source_less.project()));
    assert_eq!(semantic_state(&backed_reopened.project), semantic_state(&detached_reopened.project));
}

fn assert_command_parity(
    backed: &mut EditingSession,
    source_less: &mut EditingSession,
    operation: impl Fn(&mut EditingSession),
) {
    operation(backed);
    operation(source_less);
    assert_source_new_parity(backed, source_less);
    if backed.can_undo() {
        valid(backed.undo());
        valid(source_less.undo());
        assert_source_new_parity(backed, source_less);
        valid(backed.redo());
        valid(source_less.redo());
        assert_source_new_parity(backed, source_less);
    }
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

#[test]
fn text_only_and_role_projections_keep_the_owner_opaque_across_native_reread() {
    let mut session = fixture_session();
    let paragraph_index = 0;
    let owner_id = session.project().elements[paragraph_index]
        .source_provenance
        .as_ref()
        .and_then(|provenance| provenance.foreign_id.as_deref())
        .expect("fixture body owner has a source identity")
        .to_owned();
    let opaque_before = owner_opaque_snapshot(
        &paragraph_by_foreign_id(session.project(), &owner_id).expect("source owner exists"),
    );

    let initial_len = session.project().elements[paragraph_index].text.len();
    valid(session.set_selection(Selection::caret(Position {
        element: paragraph_index,
        offset: initial_len,
    })));
    valid(session.insert("!"));
    valid(session.backspace());

    let current_len = session.project().elements[paragraph_index].text.len();
    valid(session.set_selection(Selection {
        anchor: Position {
            element: paragraph_index,
            offset: 0,
        },
        focus: Position {
            element: paragraph_index,
            offset: current_len,
        },
    }));
    valid(session.delete());
    valid(session.insert("retyped"));

    let retyped_len = session.project().elements[paragraph_index].text.len();
    valid(session.set_selection(Selection {
        anchor: Position {
            element: paragraph_index,
            offset: 0,
        },
        focus: Position {
            element: paragraph_index,
            offset: retyped_len,
        },
    }));
    valid(session.insert("é界"));
    valid(session.toggle_body_run_style(RunStyle::Bold));
    valid(session.change_role(Role::General));

    assert_eq!(session.project().elements[paragraph_index].text, "é界");
    assert_eq!(
        owner_opaque_snapshot(
            &paragraph_by_foreign_id(session.project(), &owner_id).expect("projected source owner exists"),
        ),
        opaque_before,
    );
    assert_owner_opaque_roundtrip(&session, &owner_id);
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
    valid(session.redo());
    assert_eq!(session.project(), &after);

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

fn formatted_empty_owner_source() -> &'static [u8] {
    br#"<FinalDraft DocumentType="Script" Version="6"><Content>
      <Paragraph Type="Action" id="before"><Text>Before</Text></Paragraph>
      <Paragraph Type="Action" id="formatted-empty" Alignment="Right" FirstIndent="0.25" Leading="Regular" LeftIndent="1.25" RightIndent="2.5" SpaceBefore="3" Spacing="1" StartsNewPage="Yes"><Text></Text></Paragraph>
      <Paragraph Type="Action" id="after"><Text>After</Text></Paragraph>
    </Content></FinalDraft>"#
}

fn assert_formatted_owner_removed(project: &scriptcraft_core::Project) {
    let id = project.elements.iter().find(|element| element.id.as_str() == "formatted-empty");
    assert!(id.is_none());
    assert!(!project.direct_paragraph_overrides.keys().any(|key| key.as_str() == "formatted-empty"));
    assert_source_override_absent(project, "formatted-empty");
    assert_eq!(project.validate(), Ok(()));
}

fn assert_source_override_absent(project: &scriptcraft_core::Project, id: &str) {
    let source_overrides = &project
        .preservation
        .as_ref()
        .expect("imported project keeps preservation state")
        .source_layout_settings()
        .direct_paragraph_overrides;
    assert!(!source_overrides.keys().any(|key| key.as_str() == id));
}

#[test]
fn imported_empty_directly_formatted_owner_removal_cleans_both_models_and_history() {
    let imported = read_v6(formatted_empty_owner_source()).expect("all recognized direct fields import");
    let formatted_id = imported.project.elements[1].id.clone();
    assert!(imported.project.direct_paragraph_overrides.contains_key(&formatted_id));
    assert!(imported
        .project
        .preservation
        .as_ref()
        .expect("preservation exists")
        .source_layout_settings()
        .direct_paragraph_overrides
        .contains_key(&formatted_id));
    let formatted_override = imported.project.direct_paragraph_overrides[&formatted_id];

    for backward in [false, true] {
        let imported = read_v6(formatted_empty_owner_source()).expect("formatted fixture imports");
        let mut source_backed = valid(EditingSession::new(imported.project));
        valid(source_backed.set_selection(Selection::caret(Position { element: 1, offset: 0 })));
        let before = source_backed.clone();
        if backward {
            valid(source_backed.backspace());
        } else {
            valid(source_backed.delete());
        }
        let after = source_backed.clone();
        assert_formatted_owner_removed(source_backed.project());
        assert_eq!(
            source_backed.selection(),
            if backward {
                Selection::caret(Position { element: 0, offset: "Before".len() })
            } else {
                Selection::caret(Position { element: 1, offset: 0 })
            },
        );
        assert!(source_backed.can_undo());
        valid(source_backed.undo());
        assert_eq!(source_backed, before);
        assert!(source_backed.project().direct_paragraph_overrides.contains_key(&formatted_id));
        valid(source_backed.redo());
        assert_eq!(source_backed, after);
        assert_formatted_owner_removed(source_backed.project());

        let mut source_less_project = valid(Project::new());
        source_less_project.elements = vec![
            valid(scriptcraft_core::ScreenplayElement::new("before", Role::Action, "Before")),
            valid(scriptcraft_core::ScreenplayElement::new("formatted-empty", Role::Action, "")),
            valid(scriptcraft_core::ScreenplayElement::new("after", Role::Action, "After")),
        ];
        source_less_project
            .direct_paragraph_overrides
            .insert(formatted_id.clone(), formatted_override);
        let mut source_less = valid(EditingSession::new(source_less_project));
        valid(source_less.set_selection(Selection::caret(Position { element: 1, offset: 0 })));
        if backward {
            valid(source_less.backspace());
        } else {
            valid(source_less.delete());
        }
        assert_eq!(
            source_less.project().elements.iter().map(|element| (element.role, element.text.as_str(), &element.runs)).collect::<Vec<_>>(),
            source_backed.project().elements.iter().map(|element| (element.role, element.text.as_str(), &element.runs)).collect::<Vec<_>>(),
        );
        assert_eq!(source_less.project().direct_paragraph_overrides, source_backed.project().direct_paragraph_overrides);
    }
}

#[test]
fn imported_empty_previous_next_neighbors_and_ranged_drains_remove_all_owner_overrides() {
    let neighbor_source = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
      <Paragraph Type="Action" id="empty-previous" Alignment="Left"><Text></Text></Paragraph>
      <Paragraph Type="Action" id="middle"><Text>Middle</Text></Paragraph>
      <Paragraph Type="Action" id="empty-next" StartsNewPage="Yes"><Text></Text></Paragraph>
    </Content></FinalDraft>"#;
    let mut backward = valid(EditingSession::new(read_v6(neighbor_source).expect("neighbor fixture imports").project));
    valid(backward.set_selection(Selection::caret(Position { element: 1, offset: 0 })));
    valid(backward.backspace());
    assert!(!backward.project().direct_paragraph_overrides.keys().any(|id| id.as_str() == "empty-previous"));
    assert_source_override_absent(backward.project(), "empty-previous");
    assert!(backward.project().elements.iter().all(|element| element.id.as_str() != "empty-previous"));
    assert_eq!(backward.project().validate(), Ok(()));
    assert_eq!(backward.selection(), Selection::caret(Position { element: 0, offset: 0 }));

    let mut forward = valid(EditingSession::new(read_v6(neighbor_source).expect("neighbor fixture imports").project));
    valid(forward.set_selection(Selection::caret(Position { element: 1, offset: "Middle".len() })));
    valid(forward.delete());
    assert!(!forward.project().direct_paragraph_overrides.keys().any(|id| id.as_str() == "empty-next"));
    assert_source_override_absent(forward.project(), "empty-next");
    assert!(forward.project().elements.iter().all(|element| element.id.as_str() != "empty-next"));
    assert_eq!(forward.project().validate(), Ok(()));
    assert_eq!(forward.selection(), Selection::caret(Position { element: 1, offset: "Middle".len() }));

    let ranged_source = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
      <Paragraph Type="Action" id="range-left"><Text>L</Text></Paragraph>
      <Paragraph Type="Action" id="range-one" Alignment="Center"><Text>one</Text></Paragraph>
      <Paragraph Type="Action" id="range-two" Spacing="1"><Text>two</Text></Paragraph>
      <Paragraph Type="Action" id="range-three" StartsNewPage="Yes"><Text>three</Text></Paragraph>
      <Paragraph Type="Action" id="range-right"><Text>R</Text></Paragraph>
    </Content></FinalDraft>"#;
    let mut ranged = valid(EditingSession::new(read_v6(ranged_source).expect("ranged fixture imports").project));
    valid(ranged.set_selection(Selection {
        anchor: Position { element: 0, offset: 1 },
        focus: Position { element: 4, offset: 0 },
    }));
    let before = ranged.clone();
    valid(ranged.delete());
    for id in ["range-one", "range-two", "range-three"] {
        assert!(!ranged.project().direct_paragraph_overrides.keys().any(|key| key.as_str() == id));
        assert_source_override_absent(ranged.project(), id);
        assert!(ranged.project().elements.iter().all(|element| element.id.as_str() != id));
    }
    assert_eq!(ranged.project().elements[0].text, "LR");
    assert_eq!(ranged.project().validate(), Ok(()));
    assert_eq!(ranged.selection(), Selection::caret(Position { element: 0, offset: 1 }));
    let after = ranged.clone();
    valid(ranged.undo());
    assert_eq!(ranged, before);
    valid(ranged.redo());
    assert_eq!(ranged, after);
}

#[test]
fn same_role_source_join_discards_only_right_owner_and_never_inherits_it_on_recreation() {
    let source = r#"<FinalDraft DocumentType="Script" Version="6"><Content>
      <Paragraph Type="Action" id="join-left" Alignment="Center" Bookmark="left-bookmark" vendor-left="kept"><Text>Left</Text><UserParagraphData owner="left"><LeftPayload/></UserParagraphData></Paragraph>
      <Paragraph Type="Action" id="join-right" Alignment="Right" StartsNewPage="Yes" Bookmark="right-bookmark" vendor-right="discarded"><Text>Right</Text><UserParagraphData owner="right"><RightPayload id="right-opaque-id"/></UserParagraphData></Paragraph>
      <Paragraph Type="Action" id="join-ref" vendor-ref="join-right"><OpaqueExternal ref="join-right"/><Text>Unrelated survivor</Text></Paragraph>
    </Content></FinalDraft>"#;
    let mut imported = read_v6(source.as_bytes()).expect("same-role source fixture imports");
    let left_identity = valid(ForeignIdentity::new(ForeignIdDomain::Paragraph, "join-left"));
    let right_identity = valid(ForeignIdentity::new(ForeignIdDomain::Paragraph, "join-right"));
    let anchor_id = valid(SourceAnchorId::new("test-orphan-anchor"));
    fn anchor_raw_reference(nodes: &mut [PreservedNode], anchor_id: &SourceAnchorId) -> bool {
        for node in nodes {
            let PreservedNode::Element(element) = node else { continue };
            let is_raw_reference = element.name.local_name == "OpaqueExternal"
                && element.attributes.iter().any(|attribute| {
                    attribute.name.namespace_uri.is_none()
                        && attribute.name.local_name == "ref"
                        && attribute.value == "join-right"
                });
            if is_raw_reference {
                element.anchor = Some(anchor_id.clone());
                return true;
            }
            if anchor_raw_reference(&mut element.children, anchor_id) {
                return true;
            }
        }
        false
    }
    {
        let preservation = imported.project.preservation.as_mut().expect("source preservation remains");
        let anchored = preservation.sections.iter_mut().any(|section| {
            anchor_raw_reference(&mut section.children, &anchor_id)
        });
        assert!(anchored);
        preservation.anchors.push(SourceAnchor {
            id: anchor_id.clone(),
            target: AnchorTarget::Foreign(right_identity.clone()),
            relation: AnchorRelation::After,
        });
        preservation.references.push(ForeignReference {
            source: left_identity,
            target: right_identity,
            policy: ReferencePolicy::RequiredInternal,
        });
    }
    let mut session = valid(EditingSession::new(imported.project));
    let before = session.project().clone();
    let left_owner = owner_opaque_snapshot(&paragraph_by_foreign_id(session.project(), "join-left").unwrap());
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 4 })));
    valid(session.delete());
    let after = session.project().clone();

    assert_eq!(session.project().elements[0].text, "LeftRight");
    assert!(session.project().direct_paragraph_overrides.contains_key(&session.project().elements[0].id));
    assert!(!session.project().direct_paragraph_overrides.keys().any(|id| id.as_str() == "join-right"));
    assert_source_override_absent(session.project(), "join-right");
    assert!(session
        .project()
        .preservation
        .as_ref()
        .expect("source preservation remains")
        .source_layout_settings()
        .direct_paragraph_overrides
        .contains_key(&session.project().elements[0].id));
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(session.project(), "join-left").unwrap()), left_owner);
    assert!(paragraph_by_foreign_id(session.project(), "join-right").is_none());
    let preservation = session.project().preservation.as_ref().expect("source preservation remains");
    assert!(!preservation.references.iter().any(|reference| reference.target.value == "join-right"));
    assert!(!preservation.anchors.iter().any(|anchor| matches!(
        &anchor.target,
        AnchorTarget::Foreign(identity) if identity.value == "join-right"
    )));
    fn contains_anchor(nodes: &[PreservedNode], anchor_id: &SourceAnchorId) -> bool {
        nodes.iter().any(|node| match node {
            PreservedNode::Element(element) => element.anchor.as_ref() == Some(anchor_id)
                || contains_anchor(&element.children, anchor_id),
            _ => false,
        })
    }
    assert!(preservation.sections.iter().all(|section| {
        !contains_anchor(&section.children, &anchor_id)
    }));
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
fn empty_generated_scene_heading_removal_discards_unrecognized_nested_scene_data() {
    let source = r#"<FinalDraft DocumentType="Script" Version="6"><Content>
      <Paragraph Type="Scene Heading" id="empty-scene"><Text></Text><SceneProperties vendor-scene="discard"><UnknownGenerated id="opaque-scene-id"><Paragraph Type="Action" id="nested-scene-paragraph"><Text>generated payload</Text></Paragraph></UnknownGenerated></SceneProperties><UserParagraphData owner="scene"><PrivateSceneData/></UserParagraphData></Paragraph>
      <Paragraph Type="Action" id="scene-survivor"><Text>Keep</Text></Paragraph>
    </Content></FinalDraft>"#;
    let imported = read_v6(source.as_bytes()).expect("opaque generated-scene fixture imports");
    let mut session = valid(EditingSession::new(imported.project));
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.delete());

    let source_bytes = write_v6(session.project()).expect("scene-removal source writes");
    let source_text = std::str::from_utf8(&source_bytes).expect("writer emits UTF-8");
    assert!(!source_text.contains("empty-scene"));
    assert!(!source_text.contains("opaque-scene-id"));
    assert!(!source_text.contains("nested-scene-paragraph"));
    assert!(!source_text.contains("PrivateSceneData"));
    let reopened = read_v6(source_text.as_bytes()).expect("scene-removal source reopens");
    assert_eq!(reopened.project.elements, session.project().elements);
}

#[test]
fn empty_title_owner_removal_needs_no_text_recipient_and_restores_history() {
    let source = std::str::from_utf8(FIXTURE).expect("fixture is UTF-8")
        .replace(
            "<Paragraph Type=\"Title\" id=\"title-001\"><Text>A Small Story</Text>",
            "<Paragraph Type=\"Title\" id=\"title-empty\" Bookmark=\"discard-title\"><Text></Text><VendorTitlePayload id=\"discard-title-id\"/>",
        )
        .replace(
            "</Content>\n  </TitlePage>",
            "<Paragraph Type=\"Title\" id=\"title-survivor\"><Text>Surviving card</Text></Paragraph></Content>\n  </TitlePage>",
        );
    let imported = read_v6(source.as_bytes()).expect("empty title-owner fixture imports");
    let mut session = valid(EditingSession::new(imported.project));
    let before = session.project().clone();
    valid(session.set_title_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.delete_title());
    let after = session.project().clone();
    assert_eq!(session.project().title_page.len(), 1);
    assert_eq!(session.project().title_page[0].text, "Surviving card");
    let bytes = write_v6(session.project()).expect("title owner-removal source writes");
    let serialized = std::str::from_utf8(&bytes).expect("writer emits UTF-8");
    assert!(!serialized.contains("discard-title"));
    assert!(!serialized.contains("discard-title-id"));
    let reopened = read_v6(serialized.as_bytes()).expect("title owner-removal source reopens");
    assert_eq!(reopened.project.title_page, after.title_page);
    valid(session.undo());
    assert_eq!(session.project(), &before);
    valid(session.redo());
    assert_eq!(session.project(), &after);
}

#[test]
fn title_empty_deletions_at_first_interior_and_last_positions_keep_exact_history_and_native_output() {
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
        valid(session.delete_title());
        let after = session.project().clone();
        assert_eq!(after.title_page.len(), 4);
        assert_eq!(session.revision(), initial_revision + 1);
        assert!(session.dirty());
        assert!(session.can_undo());

        let bytes = write_v6(session.project()).expect("title boundary result writes");
        let reopened = read_v6(&bytes).expect("title boundary result rereads");
        assert_eq!(reopened.project.title_page, after.title_page);

        let accepted_revision = session.revision();
        valid(session.set_title_selection(Selection::caret(Position {
            element: 0,
            offset: 0,
        })));
        valid(session.insert_title(""));
        assert_eq!(session.project(), &after);
        assert_eq!(session.revision(), accepted_revision);
        assert!(session.dirty());
        assert!(session.can_undo());

        valid(session.undo());
        assert_eq!(session.project(), &before);
        assert!(!session.dirty());
        assert!(session.can_redo());
        valid(session.redo());
        assert_eq!(session.project(), &after);
        assert!(session.dirty());
    }
}

#[test]
fn title_join_keeps_repeated_raw_foreign_ids_distinct_by_semantic_owner() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="body"><Text>Body</Text></Paragraph></Content><TitlePage><Content><Paragraph Type="Title" id="duplicate-title" Bookmark="left"><Text>One</Text><UserParagraphData owner="left"/></Paragraph><Paragraph Type="Title" id="duplicate-title" Bookmark="right"><Text>Two</Text><UserParagraphData owner="right"/></Paragraph></Content></TitlePage></FinalDraft>"#;
    let imported = read_v6(source).expect("repeated title identity fixture imports");
    assert_eq!(imported.project.title_page.len(), 2);
    assert_ne!(imported.project.title_page[0].id, imported.project.title_page[1].id);
    let mut session = valid(EditingSession::new(imported.project));
    let before = session.project().clone();
    let right_end = session.project().title_page[1].text.len();
    valid(session.set_title_selection(Selection {
        anchor: Position { element: 0, offset: 3 },
        focus: Position { element: 1, offset: right_end },
    }));
    valid(session.insert_title(""));
    assert_eq!(session.project().title_page.len(), 1);
    assert_eq!(session.project().title_page[0].text, "OneTwo");
    let bytes = write_v6(session.project()).expect("repeated-ID title join writes");
    let serialized = std::str::from_utf8(&bytes).expect("writer emits UTF-8");
    assert_eq!(serialized.matches("id=\"duplicate-title\"").count(), 1);
    assert!(serialized.contains("Bookmark=\"left\""));
    assert!(serialized.contains("owner=\"left\""));
    assert!(!serialized.contains("Bookmark=\"right\""));
    assert!(!serialized.contains("owner=\"right\""));
    let reopened = read_v6(serialized.as_bytes()).expect("repeated-ID title join rereads");
    assert_eq!(reopened.project.title_page, session.project().title_page);

    valid(session.undo());
    assert_eq!(session.project(), &before);
    valid(session.redo());
    assert_eq!(session.project().title_page[0].text, "OneTwo");
    assert_eq!(session.project().title_page.len(), 1);
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
fn every_supported_role_unconditionally_projects_type_and_keeps_alts_owner_data() {
    let role_name = |role: Role| match role {
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
    };
    let roles = [
        Role::SceneHeading, Role::Action, Role::Character, Role::Dialogue,
        Role::Parenthetical, Role::Transition, Role::Note, Role::NewAct,
        Role::EndOfAct, Role::CastList, Role::General, Role::Shot,
        Role::Sequence, Role::Outline1, Role::Outline2, Role::Outline3,
        Role::Summary,
    ];
    for role in roles {
        let base = std::str::from_utf8(include_bytes!("fixtures/dialogue_alts.fdx"))
            .expect("checked-in Alts fixture is UTF-8");
        let source = base.replace(
            "<Paragraph Type=\"Dialogue\" id=\"alts-ordinary\">",
            "<Paragraph Type=\"Dialogue\" id=\"alts-ordinary\" Bookmark=\"keep\" vendor-role=\"opaque\"><UserParagraphData vendor=\"keep\"><Payload/></UserParagraphData>",
        );
        let mut project = read_v6(source.as_bytes()).expect("Alts role fixture imports").project;
        let semantic_id = project.elements[0].id.clone();
        project.elements[0].source_provenance.as_mut().unwrap().foreign_role = Some("ForeignOldRole".into());
        let preservation = project.preservation.as_mut().unwrap();
        let source_owner = preservation.sections.iter_mut().find_map(|section| {
            source_paragraph_by_foreign_id_mut(&mut section.children, "alts-ordinary")
        }).unwrap();
        if role == Role::Summary {
            source_owner.attributes.retain(|attribute| {
                attribute.name.namespace_uri.is_some() || attribute.name.local_name != "Type"
            });
            source_owner.source_type = None;
        } else {
            source_owner.source_type = Some("ForeignOldRole".into());
            let type_attribute = source_owner.attributes.iter_mut().find(|attribute| {
                attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Type"
            }).unwrap();
            type_attribute.value = "ForeignOldRole".into();
            source_owner.attributes.push(PreservedAttribute {
                name: scriptcraft_core::preservation::SourceName {
                    namespace_uri: None,
                    local_name: "Type".into(),
                    qualified_name: Some("Type".into()),
                },
                value: "Dialogue".into(),
            });
        }
        preservation.rebuild_indexes();
        let opaque_before = owner_opaque_snapshot(
            &paragraph_by_foreign_id(&project, "alts-ordinary").unwrap(),
        );
        let mut session = valid(EditingSession::new(project));
        let index = session.project().elements.iter().position(|element| element.id == semantic_id).unwrap();
        valid(session.set_selection(Selection::caret(Position { element: index, offset: 0 })));
        valid(session.change_role(Role::Action));
        valid(session.change_role(role));
        assert_eq!(session.project().elements[index].role, role);
        let owner = paragraph_by_foreign_id(session.project(), "alts-ordinary").unwrap();
        let type_attributes = owner.attributes.iter().filter(|attribute| {
            attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Type"
        }).collect::<Vec<_>>();
        assert_eq!(type_attributes.len(), 1);
        assert_eq!(type_attributes[0].value, role_name(role));
        assert_eq!(
            session.project().elements[index].source_provenance.as_ref().unwrap().foreign_role.as_deref(),
            Some(role_name(role)),
        );
        assert_eq!(owner_opaque_snapshot(&owner), opaque_before);
        if role == Role::Dialogue {
            let bytes = write_v6(session.project()).expect("representative role-projected source writes");
            let reopened = read_v6(&bytes).expect("representative role-projected source rereads");
            assert_eq!(reopened.project.elements[index].role, role);
            assert_eq!(
                owner_opaque_snapshot(&paragraph_by_foreign_id(&reopened.project, "alts-ordinary").unwrap()),
                opaque_before,
            );
        }
    }
}

#[test]
fn direct_alignment_upsert_removes_duplicate_managed_fields_and_keeps_unknown_attributes() {
    let source = std::str::from_utf8(FIXTURE).expect("fixture is UTF-8")
        .replace("<Paragraph Type=\"Action\" id=\"body-001\">",
            "<Paragraph Type=\"Action\" id=\"body-001\" Alignment=\"Left\" vendor-field=\"keep\">");
    let mut project = read_v6(source.as_bytes()).expect("direct formatting fixture imports").project;
    let owner = project.preservation.as_mut().unwrap().sections.iter_mut().find_map(|section| {
        source_paragraph_by_foreign_id_mut(&mut section.children, "body-001")
    }).unwrap();
    owner.attributes.push(PreservedAttribute {
        name: scriptcraft_core::preservation::SourceName {
            namespace_uri: None,
            local_name: "Alignment".into(),
            qualified_name: Some("Alignment".into()),
        },
        value: "Center".into(),
    });
    project.preservation.as_mut().unwrap().rebuild_indexes();
    let mut session = valid(EditingSession::new(project));
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.set_body_alignment(ParagraphAlignment::Right));
    let owner = paragraph_by_foreign_id(session.project(), "body-001").unwrap();
    let alignment = owner.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Alignment"
    }).collect::<Vec<_>>();
    assert_eq!(alignment.len(), 1);
    assert_eq!(alignment[0].value, "Right");
    assert!(owner.attributes.iter().any(|attribute| {
        attribute.name.local_name == "vendor-field" && attribute.value == "keep"
    }));
    let bytes = write_v6(session.project()).expect("direct-field projection writes");
    let reopened = read_v6(&bytes).expect("direct-field projection rereads");
    assert_eq!(reopened.project.direct_paragraph_overrides, session.project().direct_paragraph_overrides);
}

#[test]
fn missing_managed_body_and_title_sources_are_recreated_before_projection() {
    let mut project = read_v6(FIXTURE).expect("source fixture imports").project;
    let body_id = project.elements[0].id.clone();
    let body_foreign = project.elements[0].source_provenance.as_ref().unwrap().foreign_id.clone().unwrap();
    let title_id = project.title_page[0].id.clone();
    let title_foreign = project.title_page[0].source_provenance.as_ref().unwrap().foreign_id.clone().unwrap();
    let preservation = project.preservation.as_mut().unwrap();
    for foreign_id in [&body_foreign, &title_foreign] {
        let mut removed = false;
        for section in &mut preservation.sections {
            if remove_source_paragraph_by_foreign_id(&mut section.children, foreign_id) {
                removed = true;
                break;
            }
        }
        assert!(removed);
        preservation.identities.retain(|identity| identity.value != *foreign_id);
    }
    preservation.anchors.retain(|anchor| !matches!(&anchor.target,
        AnchorTarget::ProjectElement(id) if id == &body_id || id == &title_id
    ));
    preservation.rebuild_indexes();

    let mut session = valid(EditingSession::new(project));
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.insert("recovered "));
    let recovered_body = session.project().elements[0].source_provenance.as_ref().unwrap();
    assert_ne!(recovered_body.foreign_id.as_deref(), Some(body_foreign.as_str()));
    assert!(paragraph_by_foreign_id(session.project(), recovered_body.foreign_id.as_deref().unwrap()).is_some());

    valid(session.set_title_selection(Selection::caret(Position {
        element: 0,
        offset: session.project().title_page[0].text.len(),
    })));
    valid(session.insert_title("!"));
    let recovered_title = session.project().title_page[0].source_provenance.as_ref().unwrap();
    assert_ne!(recovered_title.foreign_id.as_deref(), Some(title_foreign.as_str()));
    assert!(paragraph_by_foreign_id(session.project(), recovered_title.foreign_id.as_deref().unwrap()).is_some());
    let bytes = write_v6(session.project()).expect("recovered body/title owners write");
    let reopened = read_v6(&bytes).expect("recovered owner projections reread");
    assert_eq!(reopened.project.elements[0].text, session.project().elements[0].text);
    assert_eq!(reopened.project.title_page[0].text, session.project().title_page[0].text);
}

#[test]
fn source_backed_and_source_less_ordinary_body_title_edits_have_identical_semantics_and_history() {
    fn semantic_state(project: &Project) -> (Vec<(Role, String, Vec<scriptcraft_core::TextRun>)>, Vec<(String, Vec<scriptcraft_core::TextRun>)>, std::collections::BTreeMap<scriptcraft_core::StableId, scriptcraft_core::formatting::SourceParagraphOverrides>) {
        (
            project.elements.iter().map(|element| (element.role, element.text.clone(), element.runs.clone())).collect(),
            project.title_page.iter().map(|paragraph| (paragraph.text.clone(), paragraph.runs.clone())).collect(),
            project.direct_paragraph_overrides.clone(),
        )
    }

    let imported = read_v6(FIXTURE).expect("parity fixture imports").project;
    let mut source_less = imported.clone();
    source_less.preservation = None;
    for element in &mut source_less.elements {
        element.source_provenance = None;
    }
    for paragraph in &mut source_less.title_page {
        paragraph.source_provenance = None;
    }
    let mut backed = valid(EditingSession::new(imported));
    let mut detached = valid(EditingSession::new(source_less));
    for session in [&mut backed, &mut detached] {
        valid(session.set_selection(Selection {
            anchor: Position { element: 0, offset: 0 },
            focus: Position { element: 0, offset: 5 },
        }));
        valid(session.toggle_body_run_style(RunStyle::Bold));
        valid(session.set_body_alignment(ParagraphAlignment::Right));
        valid(session.change_role(Role::General));
        valid(session.set_title_selection(Selection::caret(Position { element: 0, offset: 0 })));
        valid(session.insert_title("Title: "));
        valid(session.set_title_selection(Selection {
            anchor: Position { element: 0, offset: 0 },
            focus: Position { element: 0, offset: 6 },
        }));
        valid(session.toggle_title_run_style(RunStyle::Italics));
    }
    assert_eq!(semantic_state(backed.project()), semantic_state(detached.project()));
    assert_eq!(backed.selection(), detached.selection());
    assert_eq!(backed.title_selection(), detached.title_selection());
    assert_eq!(backed.dirty(), detached.dirty());
    for _ in 0..4 {
        valid(backed.undo());
        valid(detached.undo());
        assert_eq!(semantic_state(backed.project()), semantic_state(detached.project()));
        assert_eq!(backed.selection(), detached.selection());
        assert_eq!(backed.title_selection(), detached.title_selection());
        assert_eq!(backed.dirty(), detached.dirty());
    }
    for _ in 0..4 {
        valid(backed.redo());
        valid(detached.redo());
        assert_eq!(semantic_state(backed.project()), semantic_state(detached.project()));
        assert_eq!(backed.selection(), detached.selection());
        assert_eq!(backed.title_selection(), detached.title_selection());
        assert_eq!(backed.dirty(), detached.dirty());
    }
}

#[test]
fn empty_title_text_slots_are_removed_and_retyped_from_canonical_runs() {
    let source = std::str::from_utf8(FIXTURE).expect("fixture is UTF-8")
        .replace(
            "<Paragraph Type=\"Title\" id=\"title-001\"><Text>A Small Story</Text>",
            "<Paragraph Type=\"Title\" id=\"title-001\"><Text/>",
        );
    let imported = read_v6(source.as_bytes()).expect("empty title source imports");
    let mut session = valid(EditingSession::new(imported.project));
    valid(session.set_title_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.insert_title("Typed title"));
    assert_eq!(session.project().title_page[0].text, "Typed title");
    valid(session.set_title_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 0, offset: "Typed title".len() },
    }));
    valid(session.insert_title(""));
    assert!(session.project().title_page[0].text.is_empty());
    let title_source = paragraph_by_foreign_id(session.project(), "title-001").unwrap();
    assert!(!title_source.children.iter().any(|node| matches!(node,
        PreservedNode::Element(text) if text.name.local_name == "Text"
    )));
    valid(session.set_title_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.insert_title("Retyped"));
    let bytes = write_v6(session.project()).expect("retyped title writes");
    let reopened = read_v6(&bytes).expect("retyped title rereads");
    assert_eq!(reopened.project.title_page[0].text, "Retyped");
}

#[test]
fn title_inline_boundary_edits_reproject_runs_and_retain_unrelated_opaque_siblings() {
    let source = std::str::from_utf8(FIXTURE).expect("fixture is UTF-8")
        .replace(
            "<Paragraph Type=\"Title\" id=\"title-001\"><Text>A Small Story</Text>",
            "<Paragraph Type=\"Title\" id=\"title-001\"><Text>A <InlineTitle/>Small Story</Text><!--title-gap--><?title keep?><Text/>",
        );
    let imported = read_v6(source.as_bytes()).expect("opaque title topology imports");
    let mut session = valid(EditingSession::new(imported.project));
    valid(session.set_title_selection(Selection {
        anchor: Position { element: 0, offset: 2 },
        focus: Position { element: 0, offset: 3 },
    }));
    valid(session.insert_title(""));
    assert_eq!(session.project().title_page[0].text, "A mall Story");
    let owner = paragraph_by_foreign_id(session.project(), "title-001").unwrap();
    assert!(!owner.children.iter().any(|node| matches!(node,
        PreservedNode::Element(text) if text.name.local_name == "Text"
            && text.children.iter().any(|child| matches!(child,
                PreservedNode::Element(inline) if inline.name.local_name == "InlineTitle"))
    )));
    assert!(owner.children.iter().any(|node| matches!(node,
        PreservedNode::Comment(value) if value.as_str() == "title-gap"
    )));
    assert!(owner.children.iter().any(|node| matches!(node,
        PreservedNode::ProcessingInstruction { target, data }
            if target.as_str() == "title" && !data.is_empty()
    )));
    valid(session.set_title_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 0, offset: session.project().title_page[0].text.len() },
    }));
    valid(session.toggle_title_run_style(RunStyle::Bold));
    let bytes = write_v6(session.project()).expect("edited title with opaque siblings writes");
    let reopened = read_v6(&bytes).expect("edited title projection rereads");
    assert_eq!(reopened.project.title_page[0].text, session.project().title_page[0].text);
    assert_eq!(reopened.project.title_page[0].runs, session.project().title_page[0].runs);
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
fn ordinary_edits_reproject_opaque_inline_runs_at_and_across_boundaries() {
    let source = std::str::from_utf8(FIXTURE).expect("fixture is UTF-8")
        .replace("<Text>Opening action.</Text>",
            "<Text>Opening <InlineOpaque/><![CDATA[action.]]></Text><!-- opaque-gap --><?preserve gap?><Text hint=\"keep\"> tail</Text><Text/>");
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
    valid(session.insert(""));
    assert_eq!(session.project().elements[0].text, format!("{}{}!", &full_text[..8], &full_text[9..]));
    let projected = paragraph_by_foreign_id(session.project(), "body-001").unwrap();
    assert!(!projected.children.iter().any(|node| matches!(node,
        PreservedNode::Element(text) if text.name.local_name == "Text"
            && text.children.iter().any(|child| matches!(child,
                PreservedNode::Element(opaque) if opaque.name.local_name == "InlineOpaque"))
    )), "the changed inline Text occurrence is discarded with its opaque descendant");
    assert!(projected.children.iter().any(|node| matches!(node,
        PreservedNode::Comment(value) if value.as_str() == " opaque-gap "
    )), "the unrelated opaque gap remains in source order");
    assert!(projected.children.iter().any(|node| matches!(node,
        PreservedNode::ProcessingInstruction { target, data }
            if target.as_str() == "preserve" && !data.is_empty()
    )));
    assert!(!projected.children.iter().any(|node| matches!(node,
        PreservedNode::Element(text) if text.name.local_name == "Text"
            && text.children.is_empty()
    )), "empty source Text slots do not capture neighboring canonical content");
    valid(session.undo());
    assert_eq!(session.project().elements[0].text, full_text);
    valid(session.redo());

    let mut across_gap = valid(EditingSession::new(
        read_v6(source.as_bytes()).expect("same opaque topology imports for cross-gap edit").project,
    ));
    valid(across_gap.set_selection(Selection {
        anchor: Position { element: 0, offset: 14 },
        focus: Position { element: 0, offset: 16 },
    }));
    valid(across_gap.insert(""));
    assert_eq!(across_gap.project().elements[0].text, format!("{}{}", &full_text[..14], &full_text[16..]));
    let gap_owner = paragraph_by_foreign_id(across_gap.project(), "body-001").unwrap();
    assert!(gap_owner.children.iter().any(|node| matches!(node,
        PreservedNode::Comment(value) if value.as_str() == " opaque-gap "
    )));
    assert!(gap_owner.children.iter().any(|node| matches!(node,
        PreservedNode::ProcessingInstruction { target, .. } if target.as_str() == "preserve"
    )));

    let mut styled = valid(EditingSession::new(
        read_v6(source.as_bytes()).expect("same opaque topology imports for run style").project,
    ));
    valid(styled.set_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 0, offset: full_text.len() },
    }));
    valid(styled.toggle_body_run_style(RunStyle::Bold));
    assert_eq!(styled.project().elements[0].text, full_text);
    assert!(styled.project().elements[0].runs.iter().all(|run| run.attributes.iter().any(|attribute| {
        attribute.name == "Style" && attribute.value.contains("Bold")
    })));
    let styled_owner = paragraph_by_foreign_id(styled.project(), "body-001").unwrap();
    assert!(!styled_owner.children.iter().any(|node| matches!(node,
        PreservedNode::Element(text) if text.name.local_name == "Text"
            && text.children.iter().any(|child| matches!(child,
                PreservedNode::Element(opaque) if opaque.name.local_name == "InlineOpaque"))
    )));
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
    valid(conflict_session.insert_title(""));
    assert_eq!(conflict_session.project().title_page.len(), 1);
    assert!(paragraph_by_foreign_id(conflict_session.project(), "title-conflict").is_none());
    let joined = conflict_session.project().title_page[0].text.clone();
    let joined_bytes = write_v6(conflict_session.project()).expect("title structure rereads from current semantics");
    let joined_reopened = read_v6(&joined_bytes).expect("title structure rereads");
    assert_eq!(joined_reopened.project.title_page[0].text, joined);
    valid(conflict_session.undo());
    assert!(paragraph_by_foreign_id(conflict_session.project(), "title-conflict").is_some());
    valid(conflict_session.redo());
    assert!(paragraph_by_foreign_id(conflict_session.project(), "title-conflict").is_none());

    let dual_source = std::str::from_utf8(FIXTURE).expect("fixture is UTF-8")
        .replace(
            "<Paragraph Type=\"Action\" id=\"body-001\">",
            "<Paragraph Type=\"Action\" id=\"body-001\"><OpaqueExternal ref=\"outer-dual-001\"/>",
        )
        .replace(
            "<DualDialogue>\n        <SpellCheckIgnoreLists>",
            "<DualDialogue><WrapperOpaque id=\"wrapper-only\"/><SpellCheckIgnoreLists>",
        )
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
    let unwrapped_xml = write_v6(dual_session.project()).expect("unwrapped source writes");
    let unwrapped_xml = std::str::from_utf8(&unwrapped_xml).expect("writer emits UTF-8");
    assert!(!unwrapped_xml.contains("wrapper-only"));
    assert!(unwrapped_xml.contains("ref=\"outer-dual-001\""));
    assert_eq!(owner_opaque_snapshot(&paragraph_by_foreign_id(dual_session.project(), "left-dialogue-001").unwrap()), member_opaque_before);
    for owner in ["left-cue-001", "left-dialogue-001", "right-cue-001", "right-dialogue-001"] {
        assert_eq!(unwrapped.get(owner), dual_before.get(owner));
    }
    valid(dual_session.undo());
    assert_eq!(attachments_by_paragraph_id(dual_session.project()), dual_before);
    valid(dual_session.redo());
    assert_eq!(attachments_by_paragraph_id(dual_session.project()), unwrapped);
    assert_direct_roundtrip_attachments(&dual_session);
    read_v6(unwrapped_xml.as_bytes()).expect("wrapper-orphan source rereads");
}

#[test]
fn structural_ranged_replacement_uses_current_semantics_not_old_text_partitions() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
      <Paragraph Type="Action" id="partition-a" vendor-owner="left"><Text>ABCDE</Text><OpaqueLeft/></Paragraph>
      <Paragraph Type="Action" id="partition-b" vendor-owner="right"><Text>uvwxyz</Text><OpaqueRight/></Paragraph>
      <Paragraph Type="Action" id="partition-c"><Text>Tail</Text></Paragraph>
    </Content></FinalDraft>"#;
    let imported = read_v6(source).expect("structural partition fixture imports");
    let mut backed_project = imported.project;
    let mut detached_project = backed_project.clone();
    detached_project.preservation = None;
    let mut backed = valid(EditingSession::new(backed_project));
    let mut detached = valid(EditingSession::new(detached_project));
    for session in [&mut backed, &mut detached] {
        valid(session.set_selection(Selection {
            anchor: Position { element: 0, offset: 2 },
            focus: Position { element: 1, offset: 3 },
        }));
        valid(session.insert("!X"));
    }
    assert_eq!(backed.project().elements[0].text, "AB!Xxyz");
    assert_eq!(
        backed.project().elements.iter().map(|element| (element.role, element.text.clone(), element.runs.clone())).collect::<Vec<_>>(),
        detached.project().elements.iter().map(|element| (element.role, element.text.clone(), element.runs.clone())).collect::<Vec<_>>(),
    );
    assert_eq!(backed.selection(), detached.selection());
    assert!(paragraph_by_foreign_id(backed.project(), "partition-b").is_none());
    let emitted = write_v6(backed.project()).expect("current structural projection writes");
    let reopened = read_v6(&emitted).expect("current structural projection rereads");
    assert_eq!(reopened.project.elements, backed.project().elements);
    valid(backed.undo());
    assert_eq!(backed.project().elements[0].text, "ABCDE");
    valid(backed.redo());
    assert_eq!(backed.project().elements[0].text, "AB!Xxyz");
    assert!(paragraph_by_foreign_id(backed.project(), "partition-b").is_none());
}

#[test]
fn source_new_command_matrix_matches_text_formatting_roles_titles_and_history() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
      <Paragraph Type="Action" id="action-a"><Text>Alpha</Text><UserParagraphData a="owner-a"/></Paragraph>
      <Paragraph Type="Action" id="action-b"><Text>Beta</Text><UserParagraphData b="owner-b"/></Paragraph>
      <Paragraph Type="Character" id="cue"><Text>LINA</Text></Paragraph>
      <Paragraph Type="Dialogue" id="dialogue"><Text>Hello</Text></Paragraph>
    </Content><TitlePage><Content><Paragraph Type="Title" id="title"><Text>Story</Text><UserParagraphData t="owner-title"/></Paragraph></Content></TitlePage></FinalDraft>"#;

    let (mut backed, mut detached) = source_new_pair(source);
    assert_command_parity(&mut backed, &mut detached, |session| {
        valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
        valid(session.insert("New "));
    });

    let (mut backed, mut detached) = source_new_pair(source);
    assert_command_parity(&mut backed, &mut detached, |session| {
        valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
        valid(session.insert_paste("Paste "));
    });

    let (mut backed, mut detached) = source_new_pair(source);
    assert_command_parity(&mut backed, &mut detached, |session| {
        valid(session.set_selection(Selection {
            anchor: Position { element: 0, offset: 1 },
            focus: Position { element: 0, offset: 3 },
        }));
        valid(session.delete());
    });

    let (mut backed, mut detached) = source_new_pair(source);
    assert_command_parity(&mut backed, &mut detached, |session| {
        valid(session.set_selection(Selection::caret(Position { element: 0, offset: 3 })));
        valid(session.backspace());
    });

    let (mut backed, mut detached) = source_new_pair(source);
    assert_command_parity(&mut backed, &mut detached, |session| {
        let text_len = session.project().elements[0].text.len();
        valid(session.set_selection(Selection {
            anchor: Position { element: 0, offset: 0 },
            focus: Position { element: 0, offset: text_len },
        }));
        valid(session.insert(""));
        valid(session.insert("Retyped"));
    });

    let (mut backed, mut detached) = source_new_pair(source);
    assert_command_parity(&mut backed, &mut detached, |session| {
        valid(session.set_selection(Selection {
            anchor: Position { element: 0, offset: 0 },
            focus: Position { element: 0, offset: 5 },
        }));
        valid(session.toggle_body_run_style(RunStyle::Bold));
        valid(session.clear_body_run_styles());
        valid(session.set_body_alignment(ParagraphAlignment::Right));
    });

    let (mut backed, mut detached) = source_new_pair(source);
    for role in Role::ALL {
        assert_command_parity(&mut backed, &mut detached, |session| {
            valid(session.change_role(role));
        });
    }

    let (mut backed, mut detached) = source_new_pair(source);
    assert_command_parity(&mut backed, &mut detached, |session| {
        valid(session.set_selection(Selection {
            anchor: Position { element: 0, offset: 2 },
            focus: Position { element: 1, offset: 2 },
        }));
        valid(session.insert("#"));
    });

    let (mut backed, mut detached) = source_new_pair(source);
    assert_command_parity(&mut backed, &mut detached, |session| {
        valid(session.set_selection(Selection::caret(Position { element: 0, offset: 5 })));
        valid(session.join_with_next());
    });

    let (mut backed, mut detached) = source_new_pair(source);
    assert_command_parity(&mut backed, &mut detached, |session| {
        valid(session.set_selection(Selection::caret(Position { element: 0, offset: 2 })));
        valid(session.split());
    });

    let (mut backed, mut detached) = source_new_pair(source);
    assert_command_parity(&mut backed, &mut detached, |session| {
        valid(session.set_title_selection(Selection::caret(Position { element: 0, offset: 0 })));
        valid(session.insert_title("The "));
        valid(session.set_title_selection(Selection {
            anchor: Position { element: 0, offset: 0 },
            focus: Position { element: 0, offset: 4 },
        }));
        valid(session.toggle_title_run_style(RunStyle::Italics));
        valid(session.set_title_alignment(ParagraphAlignment::Left));
        valid(session.set_title_selection(Selection::caret(Position { element: 0, offset: 3 })));
        valid(session.enter_title());
    });

    let mut incompatible_source = source_new_pair(source);
    for session in [&mut incompatible_source.0, &mut incompatible_source.1] {
        let right_len = session.project().elements[2].text.len();
        valid(session.set_selection(Selection {
            anchor: Position { element: 0, offset: session.project().elements[0].text.len() },
            focus: Position { element: 2, offset: right_len },
        }));
    }
    let before_backed = incompatible_source.0.project().clone();
    let before_detached = incompatible_source.1.project().clone();
    assert!(incompatible_source.0.insert("").is_err());
    assert!(incompatible_source.1.insert("").is_err());
    assert_eq!(incompatible_source.0.project().elements, before_backed.elements);
    assert_eq!(incompatible_source.1.project().elements, before_detached.elements);
    assert_source_new_parity(&incompatible_source.0, &incompatible_source.1);
}

#[test]
fn source_new_empty_removal_dual_wrap_and_repeated_native_cycles_match() {
    let empty_source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="empty"><Text/></Paragraph><Paragraph Type="Action" id="survivor"><Text>Still here</Text></Paragraph></Content></FinalDraft>"#;
    let (mut backed, mut detached) = source_new_pair(empty_source);
    assert_command_parity(&mut backed, &mut detached, |session| {
        valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
        valid(session.delete());
    });

    let empty_neighbor_source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="left"><Text>A</Text></Paragraph><Paragraph Type="Action" id="empty"><Text/></Paragraph><Paragraph Type="Action" id="right"><Text>B</Text></Paragraph></Content></FinalDraft>"#;
    let (mut backed, mut detached) = source_new_pair(empty_neighbor_source);
    assert_command_parity(&mut backed, &mut detached, |session| {
        valid(session.set_selection(Selection::caret(Position { element: 0, offset: 1 })));
        valid(session.delete());
    });
    let (mut backed, mut detached) = source_new_pair(empty_neighbor_source);
    assert_command_parity(&mut backed, &mut detached, |session| {
        valid(session.set_selection(Selection::caret(Position { element: 2, offset: 0 })));
        valid(session.backspace());
    });

    let (mut backed, mut detached) = source_new_pair(FIXTURE);
    assert_command_parity(&mut backed, &mut detached, |session| {
        valid(session.set_selection(Selection::caret(Position { element: 1, offset: 0 })));
        valid(session.toggle_dual_dialogue());
        valid(session.set_selection(Selection::caret(Position { element: 1, offset: 0 })));
        valid(session.toggle_dual_dialogue());
    });

    let (mut backed, mut detached) = source_new_pair(FIXTURE);
    assert_command_parity(&mut backed, &mut detached, |session| {
        let cue_len = session.project().elements[1].text.len();
        valid(session.set_selection(Selection {
            anchor: Position { element: 1, offset: 0 },
            focus: Position { element: 1, offset: cue_len },
        }));
        valid(session.insert(""));
        valid(session.delete());
    });

    let two_title_source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="body"><Text>Body</Text></Paragraph></Content><TitlePage><Content><Paragraph Type="Title" id="title-a"><Text>One</Text></Paragraph><Paragraph Type="Title" id="title-b"><Text>Two</Text></Paragraph></Content></TitlePage></FinalDraft>"#;
    let (mut backed, mut detached) = source_new_pair(two_title_source);
    assert_command_parity(&mut backed, &mut detached, |session| {
        valid(session.set_title_selection(Selection {
            anchor: Position { element: 0, offset: 3 },
            focus: Position { element: 1, offset: 3 },
        }));
        valid(session.insert_title(""));
    });

    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Scene Heading" id="scene"><Text>INT. ROOM - DAY</Text></Paragraph><Paragraph Type="Character" id="cue"><Text>LINA</Text></Paragraph><Paragraph Type="Cast List" id="cast"><Text>(OLD)</Text></Paragraph></Content></FinalDraft>"#;
    let (mut backed, mut detached) = source_new_pair(source);
    for session in [&mut backed, &mut detached] {
        valid(session.set_selection(Selection {
            anchor: Position { element: 1, offset: 0 },
            focus: Position { element: 1, offset: 4 },
        }));
        valid(session.insert("NORA"));
        valid(session.refresh_cast_lists());
    }
    assert_source_new_parity(&backed, &detached);
    valid(backed.undo());
    valid(detached.undo());
    assert_source_new_parity(&backed, &detached);
    valid(backed.redo());
    valid(detached.redo());
    assert_source_new_parity(&backed, &detached);
}

#[test]
fn ordinary_single_owner_text_commands_use_scoped_validation_for_both_source_modes() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
      <Paragraph Type="Action" id="scoped-owner"><Text>ordinary text</Text><UserParagraphData vendor="owner"><Payload/></UserParagraphData></Paragraph>
    </Content></FinalDraft>"#;
    let (mut backed, mut source_less) = source_new_pair(source);

    let apply_to_pair = |backed: &mut EditingSession,
                         source_less: &mut EditingSession,
                         operation: fn(&mut EditingSession)| {
        operation(backed);
        operation(source_less);
        assert!(backed.last_edit_used_scoped_validation());
        assert!(source_less.last_edit_used_scoped_validation());
        assert_source_new_parity(backed, source_less);
    };

    for session in [&mut backed, &mut source_less] {
        let end = session.project().elements[0].text.len();
        valid(session.set_selection(Selection::caret(Position { element: 0, offset: end })));
    }
    apply_to_pair(&mut backed, &mut source_less, |session| {
        valid(session.insert_paste(" café"));
    });

    for session in [&mut backed, &mut source_less] {
        let end = session.project().elements[0].text.len();
        valid(session.set_selection(Selection::caret(Position { element: 0, offset: end })));
    }
    apply_to_pair(&mut backed, &mut source_less, |session| {
        valid(session.backspace());
    });

    for session in [&mut backed, &mut source_less] {
        valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    }
    apply_to_pair(&mut backed, &mut source_less, |session| {
        valid(session.delete());
    });

    for session in [&mut backed, &mut source_less] {
        let end = session.project().elements[0].text.len();
        valid(session.set_selection(Selection {
            anchor: Position { element: 0, offset: 0 },
            focus: Position { element: 0, offset: end },
        }));
    }
    apply_to_pair(&mut backed, &mut source_less, |session| {
        valid(session.delete());
    });

    for session in [&mut backed, &mut source_less] {
        valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    }
    apply_to_pair(&mut backed, &mut source_less, |session| {
        valid(session.insert("retyped"));
    });
}

#[test]
fn scoped_current_owner_accepts_generated_text_runs_unicode_empty_and_opaque_attachments() {
    let source = std::str::from_utf8(include_bytes!("fixtures/dialogue_alts.fdx"))
        .expect("checked-in Alts fixture is UTF-8")
        .replace(
            "<Text>ordinary </Text>",
            "<Text Style=\"Bold\">ordinary </Text><!--scoped-comment--><![CDATA[  ]]><?scoped keep?><UserParagraphData vendor=\"owner\"><OpaquePayload/></UserParagraphData><VendorScopedAttachment flag=\"retained\"><Nested/></VendorScopedAttachment>",
        )
        .replace(
            "<Text>canonical</Text>",
            "<Text Style=\"Italic\">canonical</Text>",
        );
    let imported = read_v6(source.as_bytes()).expect("owner-attachment fixture imports");

    let mut unicode_session = valid(EditingSession::new(imported.project.clone()));
    let baseline = unicode_session.project().clone();
    let old_len = unicode_session.project().elements[0].text.len();
    valid(unicode_session.set_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 0, offset: old_len },
    }));
    valid(unicode_session.insert("café 雪"));
    assert!(unicode_session.last_edit_used_scoped_validation());
    let mut candidate = unicode_session.project().clone();
    assert_eq!(
        candidate.validate_scoped_single_element_edit(&baseline, 0),
        Ok(ScopedEditValidationOutcome::Validated),
        "deterministic replacement Text accepts Unicode with opaque owner data",
    );
    assert_eq!(candidate.elements[0].text, "café 雪");
    assert!(paragraph_by_foreign_id(&candidate, "alts-ordinary")
        .expect("current owner remains present")
        .children
        .iter()
        .any(|node| matches!(node,
            PreservedNode::Comment(value) if value.as_str() == "scoped-comment"
        )));

    let mut empty_session = valid(EditingSession::new(imported.project.clone()));
    let baseline = empty_session.project().clone();
    let old_len = empty_session.project().elements[0].text.len();
    valid(empty_session.set_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 0, offset: old_len },
    }));
    valid(empty_session.insert(""));
    assert!(empty_session.last_edit_used_scoped_validation());
    let mut candidate = empty_session.project().clone();
    assert_eq!(
        candidate.validate_scoped_single_element_edit(&baseline, 0),
        Ok(ScopedEditValidationOutcome::Validated),
        "empty canonical text accepts the current no-Text projection",
    );
    assert!(candidate.elements[0].text.is_empty());

    let mut runs_session = valid(EditingSession::new(imported.project));
    let baseline = runs_session.project().clone();
    let old_len = runs_session.project().elements[0].text.len();
    valid(runs_session.set_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 0, offset: old_len },
    }));
    valid(runs_session.toggle_body_run_style(RunStyle::Bold));
    let mut candidate = runs_session.project().clone();
    assert!(candidate.elements[0].runs.len() >= 2);
    assert_eq!(
        candidate.validate_scoped_single_element_edit(&baseline, 0),
        Ok(ScopedEditValidationOutcome::Validated),
        "ordered multiple runs remain coherent with surviving Alts and attachments",
    );
}

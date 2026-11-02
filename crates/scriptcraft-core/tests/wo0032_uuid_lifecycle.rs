use scriptcraft_core::cast_list::correct_cast_lists;
use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::preservation::{ForeignIdDomain, PreservedNode};
use scriptcraft_core::{Project, Role, ScreenplayElement};
use std::collections::{BTreeMap, BTreeSet};

fn imported(source: &[u8]) -> Project {
    read_v6(source)
        .unwrap_or_else(|_| unreachable!("lifecycle fixture must import"))
        .project
}

fn paragraph_ids(project: &Project) -> BTreeSet<String> {
    project
        .preservation
        .as_ref()
        .map(|state| {
            state
                .identities
                .iter()
                .filter(|identity| identity.domain == ForeignIdDomain::Paragraph)
                .map(|identity| identity.value.clone())
                .collect()
        })
        .unwrap_or_default()
}

fn uuid_v4(value: &str) -> bool {
    let Ok(parsed) = uuid::Uuid::parse_str(value) else {
        return false;
    };
    parsed.get_version() == Some(uuid::Version::Random)
        && parsed.get_variant() == uuid::Variant::RFC4122
        && value == parsed.hyphenated().to_string()
}

fn arc_ids_by_name(project: &Project) -> BTreeMap<String, String> {
    fn visit(nodes: &[PreservedNode], arcs: &mut BTreeMap<String, String>) {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if element.name.local_name == "CharacterArcBeat" {
                let name = element
                    .attributes
                    .iter()
                    .find(|attribute| attribute.name.local_name == "Name")
                    .map(|attribute| attribute.value.clone());
                let paragraph_id = element.children.iter().find_map(|child| match child {
                    PreservedNode::Element(paragraph)
                        if paragraph.name.local_name == "Paragraph" => paragraph
                            .attributes
                            .iter()
                            .find(|attribute| {
                                attribute.name.local_name.eq_ignore_ascii_case("id")
                            })
                            .map(|attribute| attribute.value.clone()),
                    _ => None,
                });
                if let (Some(name), Some(id)) = (name, paragraph_id) {
                    arcs.insert(name, id);
                }
            }
            visit(&element.children, arcs);
        }
    }

    let mut arcs = BTreeMap::new();
    if let Some(state) = &project.preservation {
        for section in &state.sections {
            visit(&section.children, &mut arcs);
        }
    }
    arcs
}

fn set_caret(session: &mut EditingSession, element: usize, offset: usize) {
    session
        .set_selection(Selection::caret(Position { element, offset }))
        .unwrap_or_else(|_| unreachable!("fixture selection must be valid"));
}

#[test]
fn ordinary_edits_role_changes_cast_refresh_and_codec_reopen_retain_established_ids() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Scene Heading" id="heading"><Text>INT. ROOM - DAY</Text></Paragraph>
<Paragraph Type="Action" id="action"><Text>walks in</Text></Paragraph>
<Paragraph Type="Character" id="character"><Text>ALPHA</Text></Paragraph>
</Content></FinalDraft>"#;
    let mut session = EditingSession::new(imported(source)).unwrap_or_else(|_| unreachable!());
    session
        .refresh_cast_lists()
        .unwrap_or_else(|_| unreachable!("supported scene projection must refresh"));
    let established = paragraph_ids(session.project());
    assert!(established.contains("heading"));
    assert!(established.contains("action"));
    assert!(established.contains("character"));

    set_caret(&mut session, 1, "walks in".len());
    session
        .insert(" quietly")
        .unwrap_or_else(|_| unreachable!("ordinary text edit must preserve identity"));
    session
        .refresh_cast_lists()
        .unwrap_or_else(|_| unreachable!("scene-cast refresh must preserve a matched arc"));
    assert_eq!(paragraph_ids(session.project()), established);

    set_caret(&mut session, 1, 0);
    session
        .change_role(Role::Transition)
        .unwrap_or_else(|_| unreachable!("supported role change must retain identity"));
    session
        .refresh_cast_lists()
        .unwrap_or_else(|_| unreachable!("role refresh must preserve the unchanged cast"));
    assert_eq!(paragraph_ids(session.project()), established);

    let encoded = write_v6(session.project()).unwrap_or_else(|_| unreachable!());
    let reopened = read_v6(&encoded).unwrap_or_else(|_| unreachable!()).project;
    assert_eq!(paragraph_ids(&reopened), established);
}

#[test]
fn split_join_and_delete_change_only_the_removed_or_new_paragraph_identity() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Action" id="left"><Text>alpha beta</Text></Paragraph>
<Paragraph Type="Dialogue" id="middle"><Text>reply</Text></Paragraph>
<Paragraph Type="Action" id="right"><Text>ending</Text></Paragraph>
</Content></FinalDraft>"#;

    let mut split = EditingSession::new(imported(source)).unwrap_or_else(|_| unreachable!());
    let original = split.project().clone();
    let before_split = paragraph_ids(split.project());
    set_caret(&mut split, 0, 5);
    split.split().unwrap_or_else(|_| unreachable!("plain paragraph split"));
    let after_split = paragraph_ids(split.project());
    let allocated = after_split
        .difference(&before_split)
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(allocated.len(), 1);
    assert!(uuid_v4(&allocated[0]));
    assert!(after_split.contains("left"));
    assert_eq!(after_split.len(), before_split.len() + 1);
    let split_state = split.project().clone();
    split.undo().unwrap_or_else(|_| unreachable!("split undo"));
    assert_eq!(split.project(), &original);
    split.redo().unwrap_or_else(|_| unreachable!("split redo"));
    assert_eq!(split.project(), &split_state);

    let mut joined = EditingSession::new(imported(source)).unwrap_or_else(|_| unreachable!());
    let before_join = joined.project().clone();
    let ids_before_join = paragraph_ids(joined.project());
    set_caret(&mut joined, 1, "reply".len());
    joined
        .join_with_next()
        .unwrap_or_else(|_| unreachable!("plain paragraphs can join"));
    let ids_after_join = paragraph_ids(joined.project());
    assert_eq!(ids_after_join.len(), ids_before_join.len() - 1);
    assert!(ids_after_join.contains("middle"));
    assert!(!ids_after_join.contains("right"));
    let joined_state = joined.project().clone();
    joined.undo().unwrap_or_else(|_| unreachable!("join undo"));
    assert_eq!(joined.project(), &before_join);
    joined.redo().unwrap_or_else(|_| unreachable!("join redo"));
    assert_eq!(joined.project(), &joined_state);

    let mut deleted = EditingSession::new(imported(source)).unwrap_or_else(|_| unreachable!());
    let before_delete = deleted.project().clone();
    let ids_before_delete = paragraph_ids(deleted.project());
    deleted
        .set_selection(Selection {
            anchor: Position {
                element: 0,
                offset: "alpha beta".len(),
            },
            focus: Position {
                element: 1,
                offset: "reply".len(),
            },
        })
        .unwrap_or_else(|_| unreachable!("paragraph deletion selection"));
    deleted.delete().unwrap_or_else(|_| unreachable!("paragraph deletion"));
    let ids_after_delete = paragraph_ids(deleted.project());
    assert_eq!(ids_after_delete.len(), ids_before_delete.len() - 1);
    assert!(!ids_after_delete.contains("middle"));
    assert!(ids_after_delete.contains("left"));
    assert!(ids_after_delete.contains("right"));
    let deleted_state = deleted.project().clone();
    deleted.undo().unwrap_or_else(|_| unreachable!("delete undo"));
    assert_eq!(deleted.project(), &before_delete);
    deleted.redo().unwrap_or_else(|_| unreachable!("delete redo"));
    assert_eq!(deleted.project(), &deleted_state);
}

#[test]
fn content_reintroduced_after_deletion_gets_a_new_uuid_without_tombstones() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Action" id="survivor"><Text>keep</Text></Paragraph>
<Paragraph Type="Action" id="removed"><Text>old content</Text></Paragraph>
</Content></FinalDraft>"#;
    let mut session = EditingSession::new(imported(source)).unwrap_or_else(|_| unreachable!());
    let original = session.project().clone();
    session
        .set_selection(Selection {
            anchor: Position {
                element: 0,
                offset: "keep".len(),
            },
            focus: Position {
                element: 1,
                offset: "old content".len(),
            },
        })
        .unwrap_or_else(|_| unreachable!("delete selection must be valid"));
    session.delete().unwrap_or_else(|_| unreachable!("delete old paragraph"));
    let after_delete = paragraph_ids(session.project());
    assert_eq!(after_delete, BTreeSet::from(["survivor".to_owned()]));

    set_caret(&mut session, 0, "keep".len());
    session.enter().unwrap_or_else(|_| unreachable!("create a new paragraph"));
    set_caret(&mut session, 1, 0);
    session
        .insert("reintroduced")
        .unwrap_or_else(|_| unreachable!("write newly reintroduced content"));
    let after_reintroduction = paragraph_ids(session.project());
    let fresh = after_reintroduction
        .difference(&after_delete)
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(fresh.len(), 1);
    assert!(uuid_v4(&fresh[0]));
    assert_ne!(fresh[0], "removed");
    assert!(!after_reintroduction.contains("removed"));

    session.undo().unwrap_or_else(|_| unreachable!("new text undo"));
    session.undo().unwrap_or_else(|_| unreachable!("paragraph creation undo"));
    session.undo().unwrap_or_else(|_| unreachable!("paragraph deletion undo"));
    assert_eq!(session.project(), &original);
    session.redo().unwrap_or_else(|_| unreachable!("paragraph deletion redo"));
    session.redo().unwrap_or_else(|_| unreachable!("paragraph creation redo"));
    session.redo().unwrap_or_else(|_| unreachable!("new text redo"));
    assert_eq!(paragraph_ids(session.project()), after_reintroduction);
}

#[test]
fn scene_arc_matching_retains_ids_and_new_or_removed_names_reconcile_by_identity() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Scene Heading" id="scene"><Text>INT. ROOM</Text></Paragraph>
<Paragraph Type="Character" id="alpha"><Text>ALPHA</Text></Paragraph>
</Content></FinalDraft>"#;
    let mut project = imported(source);
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    let initial_arcs = arc_ids_by_name(&project);
    let alpha_id = initial_arcs.get("ALPHA").cloned().unwrap_or_default();
    assert!(uuid_v4(&alpha_id));

    project.elements.push(
        ScreenplayElement::new("beta-semantic", Role::Character, "BETA")
            .unwrap_or_else(|_| unreachable!()),
    );
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    let expanded_arcs = arc_ids_by_name(&project);
    assert_eq!(expanded_arcs.get("ALPHA"), Some(&alpha_id));
    let beta_id = expanded_arcs.get("BETA").cloned().unwrap_or_default();
    assert!(uuid_v4(&beta_id));
    assert_ne!(beta_id, alpha_id);

    project.elements[1].text = "alpha".to_owned();
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    let case_renamed_arcs = arc_ids_by_name(&project);
    assert_eq!(case_renamed_arcs.get("ALPHA"), Some(&alpha_id));
    assert_eq!(case_renamed_arcs.get("BETA"), Some(&beta_id));

    project.elements[2].text = "GAMMA".to_owned();
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    let renamed_arcs = arc_ids_by_name(&project);
    assert_eq!(renamed_arcs.get("ALPHA"), Some(&alpha_id));
    let gamma_id = renamed_arcs.get("GAMMA").cloned().unwrap_or_default();
    assert!(uuid_v4(&gamma_id));
    assert_ne!(gamma_id, beta_id);
    assert!(!renamed_arcs.contains_key("BETA"));

    project.elements.pop();
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    let reduced_arcs = arc_ids_by_name(&project);
    assert_eq!(reduced_arcs, BTreeMap::from([("ALPHA".to_owned(), alpha_id)]));
    assert!(!paragraph_ids(&project).contains(&gamma_id));
    assert!(!paragraph_ids(&project).contains(&beta_id));
}

#[test]
fn separately_reopened_stale_imported_scene_arc_is_reconciled_as_derived_data() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Scene Heading" id="scene"><Text>INT. ROOM</Text></Paragraph>
<Paragraph Type="Character" id="alpha"><Text>ALPHA</Text></Paragraph>
</Content></FinalDraft>"#;
    let mut authored = imported(source);
    assert!(correct_cast_lists(&mut authored).unwrap_or(false));
    let saved = write_v6(&authored).unwrap_or_else(|_| unreachable!());
    let mut reopened = read_v6(&saved).unwrap_or_else(|_| unreachable!()).project;
    reopened.elements.pop();
    assert!(correct_cast_lists(&mut reopened).unwrap_or(false));
    let arcs = arc_ids_by_name(&reopened);
    assert!(arcs.is_empty());
    let output = write_v6(&reopened).unwrap_or_else(|_| unreachable!());
    let reopened_again = read_v6(&output).unwrap_or_else(|_| unreachable!()).project;
    assert!(arc_ids_by_name(&reopened_again).is_empty());
}

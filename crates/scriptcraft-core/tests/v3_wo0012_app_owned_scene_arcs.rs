use scriptcraft_core::cast_list::correct_cast_lists;
use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::fdx_storage::{
    open, save_session, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem,
};
use scriptcraft_core::preservation::{ForeignIdDomain, PreservedNode};
use scriptcraft_core::{Project, Role};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn imported(source: &[u8]) -> Project {
    read_v6(source)
        .unwrap_or_else(|_| unreachable!("fixture must import"))
        .project
}

fn derived_document(source: &[u8]) -> Vec<u8> {
    let mut project = imported(source);
    correct_cast_lists(&mut project).unwrap_or_else(|_| unreachable!("supported arcs reconcile"));
    write_v6(&project).unwrap_or_else(|_| unreachable!("derived fixture serializes"))
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
                let id = element.children.iter().find_map(|child| match child {
                    PreservedNode::Element(paragraph) if paragraph.name.local_name == "Paragraph" => paragraph
                        .attributes
                        .iter()
                        .find(|attribute| attribute.name.local_name.eq_ignore_ascii_case("id"))
                        .map(|attribute| attribute.value.clone()),
                    _ => None,
                });
                if let (Some(name), Some(id)) = (name, id) {
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

fn uuid_v4(value: &str) -> bool {
    uuid::Uuid::parse_str(value)
        .is_ok_and(|id| id.get_version() == Some(uuid::Version::Random))
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "scriptcraft-v3-wo0012-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&path).unwrap_or_else(|_| unreachable!("unique temp directory"));
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn delete_character(session: &mut EditingSession, cue: &str) {
    let index = session
        .project()
        .elements
        .iter()
        .position(|element| element.role == Role::Character && element.text == cue)
        .unwrap_or_else(|| unreachable!("fixture Character exists"));
    let previous = index.saturating_sub(1);
    let previous_end = session.project().elements[previous].text.len();
    session
        .set_selection(Selection {
            anchor: Position {
                element: previous,
                offset: previous_end,
            },
            focus: Position {
                element: index,
                offset: cue.len(),
            },
        })
        .unwrap_or_else(|_| unreachable!("whole cue selection is valid"));
    session
        .delete()
        .unwrap_or_else(|_| unreachable!("Character deletion is supported"));
}

const BASIC_SOURCE: &[u8] = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Scene Heading" id="heading"><Text>INT. ROOM - DAY</Text></Paragraph>
<Paragraph Type="Action" id="action"><Text>A beat.</Text></Paragraph>
<Paragraph Type="Character" id="character"><Text>ALPHA</Text></Paragraph>
</Content></FinalDraft>"#;

#[test]
fn separately_reopened_character_deletion_allows_save_and_save_as() {
    let directory = TestDirectory::new();
    let source_bytes = derived_document(BASIC_SOURCE);
    let source_path = directory.path("source.fdx");
    fs::write(&source_path, &source_bytes).unwrap_or_else(|_| unreachable!("fixture write"));
    let fs_backend = StdFdxStorageFileSystem;

    let opened = open(&fs_backend, &source_path).unwrap_or_else(|_| unreachable!("separate reopen"));
    let mut session = EditingSession::new(opened.document.project)
        .unwrap_or_else(|_| unreachable!("session initialization"));
    delete_character(&mut session, "ALPHA");
    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    save_session(&fs_backend, &mut session, &mut target, FdxSaveMode::Save)
        .unwrap_or_else(|_| unreachable!("stale imported arc must not block Save"));
    let saved = fs::read(&source_path).unwrap_or_else(|_| unreachable!("saved destination exists"));
    assert!(arc_ids_by_name(&imported(&saved)).is_empty());
    assert!(read_v6(&saved).unwrap_or_else(|_| unreachable!()).project.elements.iter().all(|e| e.role != Role::Character));

    let save_as_source = directory.path("save-as-source.fdx");
    fs::write(&save_as_source, &source_bytes).unwrap_or_else(|_| unreachable!("fixture write"));
    let opened = open(&fs_backend, &save_as_source).unwrap_or_else(|_| unreachable!("separate reopen"));
    let mut save_as_session = EditingSession::new(opened.document.project)
        .unwrap_or_else(|_| unreachable!("session initialization"));
    delete_character(&mut save_as_session, "ALPHA");
    let mut save_as_target = FdxSaveTarget::new_unsaved();
    let destination = directory.path("copy.fdx");
    save_session(
        &fs_backend,
        &mut save_as_session,
        &mut save_as_target,
        FdxSaveMode::SaveAs(destination.clone()),
    )
    .unwrap_or_else(|_| unreachable!("stale imported arc must not block Save As"));
    assert_eq!(fs::read(&save_as_source).unwrap_or_default(), source_bytes);
    let copied = fs::read(destination).unwrap_or_else(|_| unreachable!("Save As destination exists"));
    assert!(arc_ids_by_name(&imported(&copied)).is_empty());
}

#[test]
fn arc_association_uses_normalized_names_not_position_and_renames_allocate_fresh_ids() {
    let source = br#"<FinalDraft Version="6"><Content>
<Paragraph Type="Scene Heading" id="heading"><Text>INT. ROOM</Text></Paragraph>
<Paragraph Type="Character" id="alpha"><Text>ALPHA</Text></Paragraph>
<Paragraph Type="Character" id="beta"><Text>BETA</Text></Paragraph>
</Content></FinalDraft>"#;
    let mut project = imported(&derived_document(source));
    let original = arc_ids_by_name(&project);
    let alpha_id = original.get("ALPHA").cloned().unwrap_or_default();
    let beta_id = original.get("BETA").cloned().unwrap_or_default();
    assert!(uuid_v4(&alpha_id));
    assert!(uuid_v4(&beta_id));

    project.elements[1].text = "aLpHa (V.O.)".to_owned();
    project.elements.remove(2);
    project.elements.insert(
        1,
        scriptcraft_core::ScreenplayElement::new("charlie", Role::Character, "CHARLIE")
            .unwrap_or_else(|_| unreachable!()),
    );
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    let updated = arc_ids_by_name(&project);
    assert_eq!(updated.get("ALPHA"), Some(&alpha_id));
    assert!(!updated.contains_key("BETA"));
    let charlie_id = updated.get("CHARLIE").cloned().unwrap_or_default();
    assert!(uuid_v4(&charlie_id));
    assert_ne!(charlie_id, alpha_id);
    assert_eq!(project.elements[2].text, "aLpHa (V.O.)");
    let ordered = String::from_utf8(write_v6(&project).unwrap_or_else(|_| unreachable!()))
        .unwrap_or_else(|_| unreachable!());
    assert!(ordered.find("<CharacterArcBeat Name=\"CHARLIE\">").unwrap_or(usize::MAX)
        < ordered.find("<CharacterArcBeat Name=\"ALPHA\">").unwrap_or(0));

    project.elements[2].text = "GAMMA".to_owned();
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    let renamed = arc_ids_by_name(&project);
    assert!(!renamed.contains_key("ALPHA"));
    assert_eq!(renamed.get("CHARLIE"), Some(&charlie_id));
    let gamma_id = renamed.get("GAMMA").cloned().unwrap_or_default();
    assert!(uuid_v4(&gamma_id));
    assert_ne!(gamma_id, alpha_id);
}

#[test]
fn mixed_case_and_delivery_extension_cues_keep_canonical_text_and_uppercase_arc_names() {
    let source = br#"<FinalDraft Version="6"><Content>
<Paragraph Type="Scene Heading" id="heading"><SceneProperties><SceneArcBeats>
<CharacterArcBeat Name="HELEN"><Paragraph Alignment="Left" FirstIndent="0.00" Leading="Regular" LeftIndent="0.00" OutlineLevel="0" RightIndent="1.39" SpaceBefore="0" Spacing="1" StartsNewPage="No" id="arc-helen"/></CharacterArcBeat>
</SceneArcBeats></SceneProperties><Text>INT. ROOM</Text></Paragraph>
<Paragraph Type="Character" id="helen"><Text>hElEn (V.O.)</Text></Paragraph>
<Paragraph Type="Character" id="alice"><Text>aLiCe (O.S.)</Text></Paragraph>
</Content></FinalDraft>"#;
    let mut project = imported(source);
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    let arcs = arc_ids_by_name(&project);
    assert_eq!(arcs.get("HELEN").map(String::as_str), Some("arc-helen"));
    assert!(arcs.contains_key("ALICE"));
    assert_eq!(project.elements[1].text, "hElEn (V.O.)");
    assert_eq!(project.elements[2].text, "aLiCe (O.S.)");
    assert!(project
        .preservation
        .as_ref()
        .is_some_and(|state| state.identities.iter().any(|id| id.domain == ForeignIdDomain::Paragraph && id.value == "arc-helen")));
}

#[test]
fn undo_redo_restore_exact_character_and_derived_arc_state() {
    let mut session = EditingSession::new(imported(&derived_document(BASIC_SOURCE)))
        .unwrap_or_else(|_| unreachable!());
    let before = session.project().clone();
    let character_index = session.project().elements.iter().position(|element| element.role == Role::Character)
        .unwrap_or_else(|| unreachable!());
    session
        .set_selection(Selection {
            anchor: Position { element: character_index, offset: 0 },
            focus: Position { element: character_index, offset: "ALPHA".len() },
        })
        .unwrap_or_else(|_| unreachable!());
    session.insert("BETA").unwrap_or_else(|_| unreachable!());
    session.refresh_cast_lists().unwrap_or_else(|_| unreachable!());
    let after = session.project().clone();
    assert_eq!(arc_ids_by_name(&after).keys().cloned().collect::<Vec<_>>(), vec!["BETA"]);

    session.undo().unwrap_or_else(|_| unreachable!());
    assert_eq!(session.project(), &before);
    session.redo().unwrap_or_else(|_| unreachable!());
    assert_eq!(session.project(), &after);
}

#[test]
fn unsafe_source_mismatch_and_existing_save_as_destination_remain_fatal() {
    let mut project = imported(&derived_document(BASIC_SOURCE));
    let character_id = project.elements[1].id.clone();
    let state = project.preservation.as_mut().unwrap_or_else(|| unreachable!());
    fn corrupt_managed_character(nodes: &mut [PreservedNode], target: &scriptcraft_core::StableId) -> bool {
        for node in nodes {
            let PreservedNode::Element(element) = node else { continue };
            if matches!(&element.ownership, scriptcraft_core::preservation::SourceOwnership::Managed(id) if id == target) {
                for child in &mut element.children {
                    if let PreservedNode::Element(text) = child {
                        if text.name.local_name == "Text" {
                            text.children = vec![PreservedNode::Text("SOURCE MISMATCH".to_owned())];
                            return true;
                        }
                    }
                }
            }
            if corrupt_managed_character(&mut element.children, target) { return true; }
        }
        false
    }
    assert!(state.sections.iter_mut().any(|section| corrupt_managed_character(&mut section.children, &character_id)));
    match EditingSession::new(project) {
        Ok(session) => assert!(session.prepare_cast_list_save_candidate().is_err()),
        Err(_) => (),
    }

    let directory = TestDirectory::new();
    let source_path = directory.path("source.fdx");
    let bytes = derived_document(BASIC_SOURCE);
    fs::write(&source_path, &bytes).unwrap_or_else(|_| unreachable!());
    let opened = open(&StdFdxStorageFileSystem, &source_path).unwrap_or_else(|_| unreachable!());
    let mut session = EditingSession::new(opened.document.project).unwrap_or_else(|_| unreachable!());
    delete_character(&mut session, "ALPHA");
    let destination = directory.path("already-exists.fdx");
    fs::write(&destination, b"keep existing target").unwrap_or_else(|_| unreachable!());
    let mut target = FdxSaveTarget::new_unsaved();
    assert!(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(destination.clone()),
    )
    .is_err());
    assert_eq!(
        fs::read(destination).unwrap_or_default().as_slice(),
        b"keep existing target"
    );
}

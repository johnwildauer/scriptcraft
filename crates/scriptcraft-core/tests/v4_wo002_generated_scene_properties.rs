use scriptcraft_core::cast_list::correct_cast_lists;
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::fdx_storage::{
    open, save_session, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem,
};
use scriptcraft_core::preservation::{PreservedElement, PreservedNode, SourceOwnership};
use scriptcraft_core::{editing::EditingSession, Project, Role, ScreenplayElement};
use std::fs;
use std::path::PathBuf;

const ARC_COPY: &[u8] = include_bytes!("../../../.reference/arc copy.fdx");

fn imported(source: &[u8]) -> Project {
    read_v6(source)
        .unwrap_or_else(|_| unreachable!("reference fixture must import"))
        .project
}

fn managed_heading<'a>(project: &'a Project, text: &str) -> Option<&'a PreservedElement> {
    fn find<'a>(nodes: &'a [PreservedNode], id: &scriptcraft_core::StableId) -> Option<&'a PreservedElement> {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if matches!(&element.ownership, SourceOwnership::Managed(target) if target == id) {
                return Some(element);
            }
            if let Some(found) = find(&element.children, id) {
                return Some(found);
            }
        }
        None
    }

    let id = &project
        .elements
        .iter()
        .find(|element| element.role == Role::SceneHeading && element.text == text)?
        .id;
    project
        .preservation
        .as_ref()?
        .sections
        .iter()
        .find_map(|section| find(&section.children, id))
}

fn scene_properties(heading: &PreservedElement) -> Option<&PreservedElement> {
    heading.children.iter().find_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "SceneProperties" => {
            Some(element)
        }
        _ => None,
    })
}

fn arc_paragraph<'a>(project: &'a Project, name: &str) -> Option<&'a PreservedElement> {
    let heading = managed_heading(project, "InT. House")?;
    let arcs = scene_properties(heading)?.children.iter().find_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "SceneArcBeats" => {
            Some(element)
        }
        _ => None,
    })?;
    let beat = arcs.children.iter().find_map(|node| match node {
        PreservedNode::Element(element)
            if element.name.local_name == "CharacterArcBeat"
                && element.attributes.iter().any(|attribute| {
                    attribute.name.local_name == "Name" && attribute.value == name
                }) =>
        {
            Some(element)
        }
        _ => None,
    })?;
    beat.children.iter().find_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "Paragraph" => Some(element),
        _ => None,
    })
}

fn arc_id(paragraph: &PreservedElement) -> Option<&str> {
    paragraph
        .attributes
        .iter()
        .find(|attribute| {
            attribute.name.namespace_uri.is_none()
                && attribute.name.local_name.eq_ignore_ascii_case("id")
        })
        .map(|attribute| attribute.value.as_str())
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "scriptcraft-v4-wo002-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&path).unwrap_or_else(|_| unreachable!("unique test directory"));
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

fn add_character(project: &mut Project, name: &str, at: usize) {
    project.elements.insert(
        at,
        ScreenplayElement::new(format!("test-{name}"), Role::Character, name)
            .unwrap_or_else(|_| unreachable!("test Character is valid")),
    );
}

#[test]
fn arc_copy_regenerates_supported_container_and_retains_john_arc_through_round_trips() {
    let source = String::from_utf8(ARC_COPY.to_vec())
        .unwrap_or_else(|_| unreachable!("reference is UTF-8"))
        .replace(
            "<SceneProperties Length=\"3/8\" Page=\"1\" Title=\"\">",
            "<SceneProperties Length=\"9/8\" Page=\"99\" Title=\"legacy\" Vendor=\"discard\"><VendorGeneratedField/>",
        );
    let mut project = imported(source.as_bytes());
    let john = arc_paragraph(&project, "JOHN")
        .cloned()
        .unwrap_or_else(|| unreachable!("reference JOHN arc exists"));
    assert_eq!(arc_id(&john), Some("c2c099af-cad8-41e5-b9cb-14668d869666"));
    assert!(!project.elements.iter().any(|element| element.text.contains("Arc 1")));

    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    assert_eq!(arc_paragraph(&project, "JOHN"), Some(&john));
    let heading = managed_heading(&project, "InT. House")
        .unwrap_or_else(|| unreachable!("reference heading exists"));
    let properties = scene_properties(heading)
        .unwrap_or_else(|| unreachable!("generated properties exist"));
    assert_eq!(
        properties
            .attributes
            .iter()
            .map(|attribute| (attribute.name.local_name.as_str(), attribute.value.as_str()))
            .collect::<Vec<_>>(),
        [("Length", "1/8"), ("Page", "1"), ("Title", "")]
    );
    assert!(!properties.children.iter().any(|node| matches!(
        node,
        PreservedNode::Element(element) if element.name.local_name == "VendorGeneratedField"
    )));
    let names = properties
        .children
        .iter()
        .find_map(|node| match node {
            PreservedNode::Element(arcs) if arcs.name.local_name == "SceneArcBeats" => Some(
                arcs.children
                    .iter()
                    .filter_map(|node| match node {
                        PreservedNode::Element(beat)
                            if beat.name.local_name == "CharacterArcBeat" => beat
                                .attributes
                                .iter()
                                .find(|attribute| attribute.name.local_name == "Name")
                                .map(|attribute| attribute.value.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("generated arc container exists"));
    assert_eq!(names, ["JOHN", "TIM", "ANOTHER TEST"]);

    let saved = write_v6(&project).unwrap_or_else(|_| unreachable!("reference serializes"));
    let mut reopened = imported(&saved);
    assert_eq!(arc_paragraph(&reopened, "JOHN"), Some(&john));
    assert!(!correct_cast_lists(&mut reopened).unwrap_or(true));
    let saved_again = write_v6(&reopened)
        .unwrap_or_else(|_| unreachable!("reopened scene serializes again"));
    let reopened_again = imported(&saved_again);
    assert_eq!(arc_paragraph(&reopened_again, "JOHN"), Some(&john));
}

#[test]
fn roster_additions_deletions_case_changes_and_rename_follow_character_association() {
    let mut project = imported(ARC_COPY);
    let _ = correct_cast_lists(&mut project).unwrap_or(false);
    let john = arc_paragraph(&project, "JOHN")
        .cloned()
        .unwrap_or_else(|| unreachable!("reference JOHN arc exists"));

    let john_index = project
        .elements
        .iter()
        .position(|element| element.role == Role::Character && element.text == "JOHN")
        .unwrap_or_else(|| unreachable!("JOHN cue exists"));
    add_character(&mut project, "BEFORE JOHN", john_index);
    let john_index = project
        .elements
        .iter()
        .position(|element| element.role == Role::Character && element.text == "JOHN")
        .unwrap_or_else(|| unreachable!("JOHN cue remains"));
    add_character(&mut project, "AFTER JOHN", john_index + 1);
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    assert_eq!(arc_paragraph(&project, "JOHN"), Some(&john));
    for name in ["BEFORE JOHN", "AFTER JOHN"] {
        let added = arc_paragraph(&project, name)
            .unwrap_or_else(|| unreachable!("new Character receives an arc"));
        assert!(added.children.is_empty());
        assert_ne!(arc_id(added), arc_id(&john));
    }

    let cue = project
        .elements
        .iter_mut()
        .find(|element| element.role == Role::Character && element.text == "JOHN")
        .unwrap_or_else(|| unreachable!("JOHN cue exists"));
    cue.text = "jOhN (V.O.)".to_owned();
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    assert_eq!(arc_paragraph(&project, "JOHN"), Some(&john));
    assert!(project.elements.iter().any(|element| {
        element.role == Role::Character && element.text == "jOhN (V.O.)"
    }));

    project.elements.retain(|element| {
        !(element.role == Role::Character
            && ["TIM", "Another Test", "ANOTHER TEST"].contains(&element.text.as_str()))
    });
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    assert_eq!(arc_paragraph(&project, "JOHN"), Some(&john));
    assert!(arc_paragraph(&project, "TIM").is_none());
    assert!(arc_paragraph(&project, "ANOTHER TEST").is_none());

    let cue = project
        .elements
        .iter_mut()
        .find(|element| element.role == Role::Character && element.text == "jOhN (V.O.)")
        .unwrap_or_else(|| unreachable!("normalized JOHN cue exists"));
    cue.text = "JANE".to_owned();
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    assert!(arc_paragraph(&project, "JOHN").is_none());
    let jane = arc_paragraph(&project, "JANE")
        .unwrap_or_else(|| unreachable!("renamed Character receives a new arc"));
    assert!(jane.children.is_empty());
    assert_ne!(arc_id(jane), arc_id(&john));
}

#[test]
fn ambiguous_arc_payload_keeps_the_original_arc_copy_scene_subtree() {
    let source = String::from_utf8(ARC_COPY.to_vec())
        .unwrap_or_else(|_| unreachable!("reference is UTF-8"))
        .replace(
            "</Paragraph>\n          </CharacterArcBeat>",
            "</Paragraph><VendorArcPayload Marker=\"preserve\"/>\n          </CharacterArcBeat>",
        );
    let mut project = imported(source.as_bytes());
    let original_heading = managed_heading(&project, "InT. House")
        .cloned()
        .unwrap_or_else(|| unreachable!("reference heading exists"));
    let cue = project
        .elements
        .iter_mut()
        .find(|element| element.role == Role::Character && element.text == "JOHN")
        .unwrap_or_else(|| unreachable!("JOHN cue exists"));
    cue.text = "JANE".to_owned();

    let _ = correct_cast_lists(&mut project).unwrap_or(false);
    assert_eq!(
        managed_heading(&project, "InT. House"),
        Some(&original_heading)
    );
    assert!(project.elements.iter().any(|element| {
        element.role == Role::Character && element.text == "JANE"
    }));
}

#[test]
fn arc_copy_save_reopen_save_as_and_repeated_save_keep_john_arc() {
    let directory = TestDirectory::new();
    let source_path = directory.path("arc-copy.fdx");
    fs::write(&source_path, ARC_COPY).unwrap_or_else(|_| unreachable!("reference fixture written"));
    let fs_backend = StdFdxStorageFileSystem;
    let opened = open(&fs_backend, &source_path).unwrap_or_else(|_| unreachable!("reference opens"));
    let mut session = EditingSession::new(opened.document.project)
        .unwrap_or_else(|_| unreachable!("session initializes"));
    let expected = arc_paragraph(session.project(), "JOHN")
        .cloned()
        .unwrap_or_else(|| unreachable!("reference JOHN arc exists"));
    let mut source_target = FdxSaveTarget::from_baseline(opened.baseline);
    save_session(
        &fs_backend,
        &mut session,
        &mut source_target,
        FdxSaveMode::Save,
    )
    .unwrap_or_else(|_| unreachable!("reference Save succeeds"));
    assert_eq!(arc_paragraph(session.project(), "JOHN"), Some(&expected));

    let reopened = open(&fs_backend, &source_path)
        .unwrap_or_else(|_| unreachable!("saved source reopens"));
    assert_eq!(arc_paragraph(&reopened.document.project, "JOHN"), Some(&expected));
    let mut copy_session = EditingSession::new(reopened.document.project)
        .unwrap_or_else(|_| unreachable!("Save As session initializes"));
    let copy_path = directory.path("arc-copy-as.fdx");
    let mut copy_target = FdxSaveTarget::new_unsaved();
    save_session(
        &fs_backend,
        &mut copy_session,
        &mut copy_target,
        FdxSaveMode::SaveAs(copy_path.clone()),
    )
    .unwrap_or_else(|_| unreachable!("reference Save As succeeds"));

    let copied = open(&fs_backend, &copy_path)
        .unwrap_or_else(|_| unreachable!("Save As copy reopens"));
    assert_eq!(arc_paragraph(&copied.document.project, "JOHN"), Some(&expected));
    let mut repeated_session = EditingSession::new(copied.document.project)
        .unwrap_or_else(|_| unreachable!("repeated Save session initializes"));
    let mut repeated_target = FdxSaveTarget::from_baseline(copied.baseline);
    save_session(
        &fs_backend,
        &mut repeated_session,
        &mut repeated_target,
        FdxSaveMode::Save,
    )
    .unwrap_or_else(|_| unreachable!("repeated Save succeeds"));
    assert_eq!(arc_paragraph(repeated_session.project(), "JOHN"), Some(&expected));
}

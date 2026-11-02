#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::cast_list::correct_cast_lists;
use scriptcraft_core::editing::EditingSession;
use scriptcraft_core::fdx::{self, FdxError};
use scriptcraft_core::fdx_storage::{
    open, save_session, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem,
};
use scriptcraft_core::preservation::{PreservedElement, PreservedNode, SourceOwnership};
use scriptcraft_core::{Project, Role};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const ARC_COPY: &[u8] = include_bytes!("../../../.reference/arc copy.fdx");
static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "scriptcraft-v5-wo003-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path)
            .unwrap_or_else(|_| unreachable!("unique scene-formatting directory is creatable"));
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

fn import(source: &[u8]) -> Project {
    read_project(source)
}

fn read_project(source: &[u8]) -> Project {
    fdx::read_v6(source)
        .unwrap_or_else(|_| unreachable!("test FDX fixture imports"))
        .project
}

fn simple_scene(character_paragraphs: &str, heading_attributes: &str) -> Vec<u8> {
    format!(
        "<FinalDraft DocumentType=\"Script\" Version=\"6\"><Content><Paragraph Type=\"Scene Heading\" id=\"heading\" {heading_attributes}><Text>INT. ROOM</Text></Paragraph>{character_paragraphs}</Content></FinalDraft>"
    )
    .into_bytes()
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

fn arc_paragraph<'a>(project: &'a Project, heading_text: &str, name: &str) -> Option<&'a PreservedElement> {
    let heading = managed_heading(project, heading_text)?;
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

fn inject_source_type_mismatch(project: &mut Project) -> bool {
    fn visit(nodes: &mut [PreservedNode]) -> bool {
        for node in nodes {
            if let PreservedNode::Element(element) = node {
                if element.source_type.is_some() {
                    element.source_type = Some("independent-code30-control".to_owned());
                    return true;
                }
                if visit(&mut element.children) {
                    return true;
                }
            }
        }
        false
    }

    project
        .preservation
        .as_mut()
        .map(|state| state.sections.iter_mut().any(|section| visit(&mut section.children)))
        .unwrap_or(false)
}

#[test]
fn generated_scene_properties_use_actual_heading_depth_and_keep_empty_arcs_self_closing() {
    let source = simple_scene("", "");
    let mut project = import(&source);
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    let output = fdx::write_v6(&project)
        .unwrap_or_else(|_| unreachable!("generated empty scene properties serialize"));
    let output = String::from_utf8(output)
        .unwrap_or_else(|_| unreachable!("generated FDX is UTF-8"));

    assert!(output.contains(
        "<Paragraph Type=\"Scene Heading\" id=\"heading\">\n      <SceneProperties Length=\"1/8\" Page=\"1\" Title=\"\">\n        <SceneArcBeats/>\n      </SceneProperties>\n      <Text>INT. ROOM</Text>\n    </Paragraph>"
    ));
    assert!(!output.contains("<SceneArcBeats>\n"));
}

#[test]
fn generated_scene_properties_indentation_tracks_nested_source_depth() {
    let source = simple_scene("", "")
        .into_iter()
        .map(char::from)
        .collect::<String>()
        .replace("<Content>", "<Content><VendorContainer>")
        .replace("</Content>", "</VendorContainer></Content>");
    let mut project = import(source.as_bytes());
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    let output = fdx::write_v6(&project)
        .unwrap_or_else(|_| unreachable!("nested source scene serializes"));
    let output = String::from_utf8(output)
        .unwrap_or_else(|_| unreachable!("nested source FDX is UTF-8"));

    assert!(output.contains(
        "<Paragraph Type=\"Scene Heading\" id=\"heading\">\n        <SceneProperties Length=\"1/8\" Page=\"1\" Title=\"\">\n          <SceneArcBeats/>\n        </SceneProperties>\n        <Text>INT. ROOM</Text>\n      </Paragraph>"
    ));
}

#[test]
fn multiple_character_arcs_are_indented_without_adding_whitespace_to_arc_paragraphs() {
    let source = simple_scene(
        "<Paragraph Type=\"Character\" id=\"john-cue\"><Text>JOHN</Text></Paragraph><Paragraph Type=\"Character\" id=\"mary-cue\"><Text>MARY</Text></Paragraph>",
        "",
    );
    let mut project = import(&source);
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    let output = fdx::write_v6(&project)
        .unwrap_or_else(|_| unreachable!("generated character arcs serialize"));
    let output = String::from_utf8(output)
        .unwrap_or_else(|_| unreachable!("generated FDX is UTF-8"));

    assert!(output.contains(
        "        <SceneArcBeats>\n          <CharacterArcBeat Name=\"JOHN\">\n            <Paragraph Alignment=\"Left\""
    ));
    assert!(output.contains(
        "          </CharacterArcBeat>\n          <CharacterArcBeat Name=\"MARY\">\n            <Paragraph Alignment=\"Left\""
    ));
    assert!(output.contains(
        "          </CharacterArcBeat>\n        </SceneArcBeats>\n      </SceneProperties>\n      <Text>INT. ROOM</Text>"
    ));
    assert!(!output.contains("<Paragraph Alignment=\"Left\"\n"));
}

#[test]
fn retained_arc_subtree_and_id_survive_formatting_and_repeated_direct_serialization() {
    let mut project = import(ARC_COPY);
    let retained = arc_paragraph(&project, "InT. House", "JOHN")
        .cloned()
        .unwrap_or_else(|| unreachable!("reference JOHN arc exists"));
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    assert_eq!(arc_paragraph(&project, "InT. House", "JOHN"), Some(&retained));

    let first = fdx::write_v6(&project)
        .unwrap_or_else(|_| unreachable!("formatted retained arc serializes"));
    let reopened = import(&first);
    assert_eq!(arc_paragraph(&reopened, "InT. House", "JOHN"), Some(&retained));
    let second = fdx::write_v6(&reopened)
        .unwrap_or_else(|_| unreachable!("reopened retained arc serializes again"));
    assert_eq!(first, second);
    assert_eq!(arc_paragraph(&import(&second), "InT. House", "JOHN"), Some(&retained));
}

#[test]
fn filesystem_save_save_as_reopen_and_repeated_save_keep_formatting_and_retained_ids() {
    let directory = TestDirectory::new();
    let source_path = directory.path("arc-copy.fdx");
    let copy_path = directory.path("arc-copy-as.fdx");
    fs::write(&source_path, ARC_COPY)
        .unwrap_or_else(|_| unreachable!("disposable arc reference is writable"));
    let backend = StdFdxStorageFileSystem;
    let opened = open(&backend, &source_path)
        .unwrap_or_else(|_| unreachable!("arc reference opens"));
    let expected = arc_paragraph(&opened.document.project, "InT. House", "JOHN")
        .cloned()
        .unwrap_or_else(|| unreachable!("reference JOHN arc exists"));
    let mut session = EditingSession::new(opened.document.project)
        .unwrap_or_else(|_| unreachable!("arc reference initializes a session"));
    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    save_session(&backend, &mut session, &mut target, FdxSaveMode::Save)
        .unwrap_or_else(|_| unreachable!("formatted source Save succeeds"));
    let first = fs::read(&source_path)
        .unwrap_or_else(|_| unreachable!("first source Save output is readable"));
    assert_eq!(arc_paragraph(session.project(), "InT. House", "JOHN"), Some(&expected));

    let reopened = open(&backend, &source_path)
        .unwrap_or_else(|_| unreachable!("first Save output reopens"));
    let mut save_as_session = EditingSession::new(reopened.document.project)
        .unwrap_or_else(|_| unreachable!("reopened project initializes for Save As"));
    let mut save_as_target = FdxSaveTarget::new_unsaved();
    save_session(
        &backend,
        &mut save_as_session,
        &mut save_as_target,
        FdxSaveMode::SaveAs(copy_path.clone()),
    )
    .unwrap_or_else(|_| unreachable!("formatted Save As succeeds"));
    let copied = fs::read(&copy_path)
        .unwrap_or_else(|_| unreachable!("Save As output is readable"));
    assert_eq!(first, copied);

    let reopened_copy = open(&backend, &copy_path)
        .unwrap_or_else(|_| unreachable!("Save As output reopens"));
    assert_eq!(
        arc_paragraph(&reopened_copy.document.project, "InT. House", "JOHN"),
        Some(&expected)
    );
    let mut repeated_session = EditingSession::new(reopened_copy.document.project)
        .unwrap_or_else(|_| unreachable!("Save As output initializes for repeated Save"));
    let mut repeated_target = FdxSaveTarget::from_baseline(reopened_copy.baseline);
    save_session(
        &backend,
        &mut repeated_session,
        &mut repeated_target,
        FdxSaveMode::Save,
    )
    .unwrap_or_else(|_| unreachable!("repeated Save after Save As succeeds"));
    assert_eq!(
        fs::read(&copy_path)
            .unwrap_or_else(|_| unreachable!("repeated Save output is readable")),
        copied
    );
    assert_eq!(arc_paragraph(repeated_session.project(), "InT. House", "JOHN"), Some(&expected));
}

#[test]
fn deferred_scene_subtree_is_not_reformatted_and_xml_space_preserve_skips_formatting() {
    let source = String::from_utf8(ARC_COPY.to_vec())
        .unwrap_or_else(|_| unreachable!("arc reference is UTF-8"))
        .replace(
            "</Paragraph>\n          </CharacterArcBeat>",
            "</Paragraph><VendorArcPayload Marker=\"retain\"/>\n          </CharacterArcBeat>",
        );
    let mut deferred = import(source.as_bytes());
    let original_heading = managed_heading(&deferred, "InT. House")
        .cloned()
        .unwrap_or_else(|| unreachable!("deferred scene heading exists"));
    let _ = correct_cast_lists(&mut deferred).unwrap_or(false);
    assert_eq!(managed_heading(&deferred, "InT. House"), Some(&original_heading));

    let preserved_source = simple_scene(
        "<Paragraph Type=\"Character\" id=\"john-cue\"><Text>JOHN</Text></Paragraph>",
        "xml:space=\"preserve\"",
    )
    .into_iter()
    .map(char::from)
    .collect::<String>()
    .replace("<Content>", "<Content xml:space=\"preserve\">");
    let mut preserved = import(preserved_source.as_bytes());
    assert!(correct_cast_lists(&mut preserved).unwrap_or(false));
    let output = fdx::write_v6(&preserved)
        .unwrap_or_else(|_| unreachable!("xml:space-preserved scene still serializes"));
    let output = String::from_utf8(output)
        .unwrap_or_else(|_| unreachable!("preserved FDX is UTF-8"));
    assert!(output.contains(
        "<Paragraph Type=\"Scene Heading\" id=\"heading\" xml:space=\"preserve\"><SceneProperties"
    ));
    assert!(!output.contains("xml:space=\"preserve\">\n      <SceneProperties"));
}

#[test]
fn genuine_expected_tree_mismatch_still_fails_with_code30() {
    let mut project = import(&simple_scene("", ""));
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    assert!(inject_source_type_mismatch(&mut project));
    let error = fdx::write_v6(&project)
        .err()
        .unwrap_or_else(|| unreachable!("strict source-tree mismatch still rejects output"));
    assert!(matches!(error, FdxError::SaveDiagnosticSourceTreeMismatch(_)));
    assert!(error.to_string().starts_with("30: FDX contains a construct"));
}

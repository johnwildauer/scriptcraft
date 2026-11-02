use scriptcraft_core::cast_list::correct_cast_lists;
use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::fdx::read_v6;
use scriptcraft_core::fdx_storage::{
    open, save_session, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem,
};
use scriptcraft_core::preservation::{PreservedElement, PreservedNode, SourceOwnership};
use scriptcraft_core::{Project, Role, RunAttribute};
use std::fs;
use std::path::PathBuf;

const MIXED_SCENES: &[u8] = br#"<FinalDraft Version="6"><Content>
<Paragraph Type="Scene Heading" id="heading-supported"><SceneProperties Length="1/8" Page="1" Title="supported"><SceneArcBeats><CharacterArcBeat Name="ALPHA"><Paragraph Alignment="Left" id="arc-alpha"><Text Style="Bold">retained arc</Text></Paragraph></CharacterArcBeat></SceneArcBeats></SceneProperties><Text>INT. SUPPORTED - DAY</Text></Paragraph>
<Paragraph Type="Character" id="cue-alpha"><Text>Alpha (V.O.)</Text></Paragraph>
<Paragraph Type="Cast List" id="cast-formatted"><Text Style="Bold">(formatted stale)</Text></Paragraph>
<Paragraph Type="Cast List" id="cast-plain"><Text>(plain stale)</Text></Paragraph>
<Paragraph Type="Scene Heading" id="heading-deferred"><SceneProperties Length="3/8" Page="7" Title="deferred"><SceneArcBeats><CharacterArcBeat Name="BETA"><Paragraph Alignment="Left" id="arc-beta"><VendorArcPayload Marker="preserve exactly"/></Paragraph></CharacterArcBeat></SceneArcBeats></SceneProperties><Text>EXT. DEFERRED - NIGHT</Text></Paragraph>
<Paragraph Type="Character" id="cue-beta"><Text>BETA</Text></Paragraph>
<Paragraph Type="General" id="canonical-general"><Text>canonical source</Text></Paragraph>
<Paragraph Type="Cast List" id="cast-deferred"><Text>(deferred stale)</Text></Paragraph>
</Content></FinalDraft>"#;

fn imported(source: &[u8]) -> Project {
    read_v6(source)
        .unwrap_or_else(|_| unreachable!("regression fixture imports"))
        .project
}

fn scene_properties<'a>(project: &'a Project, heading_text: &str) -> &'a PreservedElement {
    fn find_managed<'a>(
        nodes: &'a [PreservedNode],
        id: &scriptcraft_core::StableId,
    ) -> Option<&'a PreservedElement> {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if matches!(&element.ownership, SourceOwnership::Managed(target) if target == id) {
                return Some(element);
            }
            if let Some(found) = find_managed(&element.children, id) {
                return Some(found);
            }
        }
        None
    }

    let heading_id = &project
        .elements
        .iter()
        .find(|element| element.role == Role::SceneHeading && element.text == heading_text)
        .unwrap_or_else(|| unreachable!("fixture heading exists"))
        .id;
    let state = project
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("import has preservation state"));
    let heading = state
        .sections
        .iter()
        .find_map(|section| find_managed(&section.children, heading_id))
        .unwrap_or_else(|| unreachable!("heading source mapping exists"));
    heading
        .children
        .iter()
        .find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "SceneProperties" => {
                Some(element)
            }
            _ => None,
        })
        .unwrap_or_else(|| unreachable!("fixture properties exist"))
}

fn replace_element_text(session: &mut EditingSession, old_text: &str, new_text: &str) {
    let index = session
        .project()
        .elements
        .iter()
        .position(|element| element.text == old_text)
        .unwrap_or_else(|| unreachable!("fixture text exists"));
    session
        .set_selection(Selection {
            anchor: Position { element: index, offset: 0 },
            focus: Position {
                element: index,
                offset: old_text.len(),
            },
        })
        .unwrap_or_else(|_| unreachable!("whole paragraph selection is valid"));
    session
        .insert(new_text)
        .unwrap_or_else(|_| unreachable!("canonical source edit is supported"));
}

fn cast_text<'a>(project: &'a Project, id: &str) -> &'a str {
    project
        .elements
        .iter()
        .find(|element| element.id.as_str() == id)
        .unwrap_or_else(|| unreachable!("fixture Cast List exists"))
        .text
        .as_str()
}

fn assert_formatted_cast_unchanged(project: &Project) {
    let cast = project
        .elements
        .iter()
        .find(|element| element.id.as_str() == "cast-formatted")
        .unwrap_or_else(|| unreachable!("formatted Cast List exists"));
    assert_eq!(cast.text, "(formatted stale)");
    assert_eq!(cast.runs[0].text, "(formatted stale)");
    assert_eq!(
        cast.runs[0].attributes,
        vec![RunAttribute {
            name: "Style".into(),
            value: "Bold".into(),
        }]
    );
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "scriptcraft-v3-wo0014-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&path).unwrap_or_else(|_| unreachable!("unique temporary directory"));
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

#[test]
fn supported_scene_refresh_and_save_save_as_preserve_deferred_scene_and_canonical_edits() {
    let directory = TestDirectory::new();
    let source_path = directory.path("source.fdx");
    fs::write(&source_path, MIXED_SCENES).unwrap_or_else(|_| unreachable!("fixture is written"));
    let filesystem = StdFdxStorageFileSystem;
    let opened = open(&filesystem, &source_path).unwrap_or_else(|_| unreachable!("fixture opens"));
    let deferred_before = scene_properties(&opened.document.project, "EXT. DEFERRED - NIGHT").clone();
    let mut session = EditingSession::new(opened.document.project)
        .unwrap_or_else(|_| unreachable!("editing session initializes"));

    replace_element_text(&mut session, "Alpha (V.O.)", "Alpha Updated (V.O.)");
    replace_element_text(&mut session, "canonical source", "canonical edit");
    assert!(session.refresh_cast_lists().unwrap_or(false));
    assert_eq!(cast_text(session.project(), "cast-plain"), "(ALPHA UPDATED)");
    assert_eq!(cast_text(session.project(), "cast-deferred"), "(deferred stale)");
    assert_formatted_cast_unchanged(session.project());
    assert_eq!(
        scene_properties(session.project(), "EXT. DEFERRED - NIGHT"),
        &deferred_before
    );

    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    save_session(&filesystem, &mut session, &mut target, FdxSaveMode::Save)
        .unwrap_or_else(|_| unreachable!("Save publishes canonical edits despite deferral"));
    let saved = open(&filesystem, &source_path).unwrap_or_else(|_| unreachable!("saved file reopens"));
    assert_eq!(
        scene_properties(&saved.document.project, "EXT. DEFERRED - NIGHT"),
        &deferred_before
    );
    assert!(saved.document.project.elements.iter().any(|element| element.text == "canonical edit"));
    assert_eq!(cast_text(&saved.document.project, "cast-plain"), "(ALPHA UPDATED)");
    assert_eq!(cast_text(&saved.document.project, "cast-deferred"), "(deferred stale)");
    assert_formatted_cast_unchanged(&saved.document.project);

    let save_as_path = directory.path("copy.fdx");
    save_session(
        &filesystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(save_as_path.clone()),
    )
    .unwrap_or_else(|_| unreachable!("Save As publishes the same canonical edits"));
    let copied = open(&filesystem, &save_as_path).unwrap_or_else(|_| unreachable!("Save As file reopens"));
    assert_eq!(
        scene_properties(&copied.document.project, "EXT. DEFERRED - NIGHT"),
        &deferred_before
    );
    assert!(copied.document.project.elements.iter().any(|element| element.text == "canonical edit"));
    assert_eq!(cast_text(&copied.document.project, "cast-plain"), "(ALPHA UPDATED)");
    assert_eq!(cast_text(&copied.document.project, "cast-deferred"), "(deferred stale)");
    assert_formatted_cast_unchanged(&copied.document.project);
}

#[test]
fn undo_redo_with_deferred_scene_never_applies_partial_scene_corrections() {
    let mut project = imported(MIXED_SCENES);
    let deferred_before = scene_properties(&project, "EXT. DEFERRED - NIGHT").clone();
    let mut session = EditingSession::new(project)
        .unwrap_or_else(|_| unreachable!("editing session initializes"));
    replace_element_text(&mut session, "BETA", "DELTA");

    assert!(session.refresh_cast_lists().unwrap_or(false));
    assert_eq!(cast_text(session.project(), "cast-plain"), "(ALPHA)");
    assert_eq!(cast_text(session.project(), "cast-deferred"), "(deferred stale)");
    assert_eq!(scene_properties(session.project(), "EXT. DEFERRED - NIGHT"), &deferred_before);

    session.undo().unwrap_or_else(|_| unreachable!("undo remains available"));
    assert_eq!(cast_text(session.project(), "cast-plain"), "(ALPHA)");
    assert_eq!(cast_text(session.project(), "cast-deferred"), "(deferred stale)");
    assert_eq!(scene_properties(session.project(), "EXT. DEFERRED - NIGHT"), &deferred_before);

    session.redo().unwrap_or_else(|_| unreachable!("redo remains available"));
    assert_eq!(cast_text(session.project(), "cast-plain"), "(ALPHA)");
    assert_eq!(cast_text(session.project(), "cast-deferred"), "(deferred stale)");
    assert_eq!(scene_properties(session.project(), "EXT. DEFERRED - NIGHT"), &deferred_before);
}

#[test]
fn preservation_free_cast_lists_use_uppercase_derived_names() {
    let mut project = Project::new().unwrap_or_else(|_| unreachable!());
    project.elements = vec![
        scriptcraft_core::ScreenplayElement::new("heading", Role::SceneHeading, "INT. ROOM")
            .unwrap_or_else(|_| unreachable!()),
        scriptcraft_core::ScreenplayElement::new("cue", Role::Character, "mIxEd cAsE")
            .unwrap_or_else(|_| unreachable!()),
        scriptcraft_core::ScreenplayElement::new("cast", Role::CastList, "(stale)")
            .unwrap_or_else(|_| unreachable!()),
    ];

    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    assert_eq!(project.elements[1].text, "mIxEd cAsE");
    assert_eq!(project.elements[2].text, "(MIXED CASE)");
}

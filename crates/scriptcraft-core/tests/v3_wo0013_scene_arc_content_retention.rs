use scriptcraft_core::cast_list::correct_cast_lists;
use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::fdx_storage::{open, save_session, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem};
use scriptcraft_core::preservation::{PreservedElement, PreservedNode, SourceOwnership};
use scriptcraft_core::{Project, Role, ScreenplayElement};
use std::fs;
use std::path::PathBuf;

const ARCS_SOURCE: &[u8] = br#"<FinalDraft Version="6"><Content>
<Paragraph Type="Scene Heading" id="heading"><SceneProperties Length="3/8" Page="7" Title="kept"><SceneArcBeats>
<CharacterArcBeat Name="ALPHA"><Paragraph Alignment="Left" FirstIndent="0.00" Leading="Regular" LeftIndent="0.00" OutlineLevel="0" RightIndent="1.39" SpaceBefore="0" Spacing="1" StartsNewPage="No" VendorParagraphAttr="preserve" id="arc-alpha"><Text Font="Courier Final Draft" Style="Bold">alpha </Text><Text Font="Courier Final Draft" Style="Italic">arc</Text></Paragraph></CharacterArcBeat>
<CharacterArcBeat Name="BETA"><Paragraph Alignment="Left" FirstIndent="0.00" Leading="Regular" LeftIndent="0.00" OutlineLevel="0" RightIndent="1.39" SpaceBefore="0" Spacing="1" StartsNewPage="No" id="arc-beta"><Text Style="Underline">beta arc</Text></Paragraph></CharacterArcBeat>
</SceneArcBeats></SceneProperties><Text>INT. ROOM - DAY</Text></Paragraph>
<Paragraph Type="Character" id="alpha-cue"><Text>ALPHA</Text></Paragraph>
<Paragraph Type="Character" id="beta-cue"><Text>BETA</Text></Paragraph>
</Content></FinalDraft>"#;

fn imported(source: &[u8]) -> Project {
    read_v6(source)
        .unwrap_or_else(|_| unreachable!("fixture must import"))
        .project
}

fn heading_node<'a>(project: &'a Project, heading_text: &str) -> Option<&'a PreservedElement> {
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
        .find(|element| element.role == Role::SceneHeading && element.text == heading_text)?
        .id;
    let state = project.preservation.as_ref()?;
    state
        .sections
        .iter()
        .find_map(|section| find_managed(&section.children, heading_id))
}

fn arc_paragraph<'a>(
    project: &'a Project,
    heading_text: &str,
    arc_name: &str,
) -> Option<&'a PreservedElement> {
    let heading = heading_node(project, heading_text)?;
    let properties = heading.children.iter().find_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "SceneProperties" => {
            Some(element)
        }
        _ => None,
    })?;
    let arcs = properties.children.iter().find_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "SceneArcBeats" => {
            Some(element)
        }
        _ => None,
    })?;
    let beat = arcs.children.iter().find_map(|node| match node {
        PreservedNode::Element(element)
            if element.name.local_name == "CharacterArcBeat"
                && element
                    .attributes
                    .iter()
                    .any(|attribute| attribute.name.local_name == "Name" && attribute.value == arc_name) =>
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

fn replace_character_text(session: &mut EditingSession, cue: &str, replacement: &str) {
    let index = session
        .project()
        .elements
        .iter()
        .position(|element| element.role == Role::Character && element.text == cue)
        .unwrap_or_else(|| unreachable!("fixture Character exists"));
    session
        .set_selection(Selection {
            anchor: Position {
                element: index,
                offset: 0,
            },
            focus: Position {
                element: index,
                offset: cue.len(),
            },
        })
        .unwrap_or_else(|_| unreachable!("whole cue selection is valid"));
    session
        .insert(replacement)
        .unwrap_or_else(|_| unreachable!("Character text replacement is supported"));
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "scriptcraft-v3-wo0013-{}",
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

#[test]
fn imported_arc_text_ordered_runs_attributes_and_id_survive_save_and_reopen() {
    let mut project = imported(ARCS_SOURCE);
    let _ = correct_cast_lists(&mut project).unwrap_or(false);
    let expected = arc_paragraph(&project, "INT. ROOM - DAY", "ALPHA")
        .cloned()
        .unwrap_or_else(|| unreachable!("supported ALPHA arc remains"));
    assert_eq!(arc_id(&expected), Some("arc-alpha"));

    let saved = write_v6(&project).unwrap_or_else(|_| unreachable!("supported arc serializes"));
    let xml = std::str::from_utf8(&saved).unwrap_or("");
    assert!(xml.find("Style=\"Bold\">alpha </Text>").unwrap_or(usize::MAX)
        < xml.find("Style=\"Italic\">arc</Text>").unwrap_or(0));
    let reopened = imported(&saved);
    assert_eq!(
        arc_paragraph(&reopened, "INT. ROOM - DAY", "ALPHA"),
        Some(&expected)
    );
    assert!(reopened
        .elements
        .iter()
        .all(|element| !element.text.contains("alpha arc")));
}

#[test]
fn unrelated_character_addition_removal_and_reordering_keep_retained_arc_unchanged() {
    let mut project = imported(ARCS_SOURCE);
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    let expected = arc_paragraph(&project, "INT. ROOM - DAY", "ALPHA")
        .cloned()
        .unwrap_or_else(|| unreachable!("ALPHA arc exists"));

    let alpha_index = project
        .elements
        .iter()
        .position(|element| element.role == Role::Character && element.text == "ALPHA")
        .unwrap_or_else(|| unreachable!());
    let beta_index = project
        .elements
        .iter()
        .position(|element| element.role == Role::Character && element.text == "BETA")
        .unwrap_or_else(|| unreachable!());
    project.elements.swap(alpha_index, beta_index);
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    assert_eq!(
        arc_paragraph(&project, "INT. ROOM - DAY", "ALPHA"),
        Some(&expected)
    );

    project
        .elements
        .retain(|element| !(element.role == Role::Character && element.text == "BETA"));
    project.elements.push(
        ScreenplayElement::new("gamma", Role::Character, "GAMMA")
            .unwrap_or_else(|_| unreachable!()),
    );
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    assert_eq!(
        arc_paragraph(&project, "INT. ROOM - DAY", "ALPHA"),
        Some(&expected)
    );
    assert!(arc_paragraph(&project, "INT. ROOM - DAY", "BETA").is_none());
    assert!(arc_paragraph(&project, "INT. ROOM - DAY", "GAMMA")
        .is_some_and(|paragraph| paragraph.children.is_empty()));
}

#[test]
fn extension_bearing_mixed_case_cue_updates_derived_name_without_losing_arc_content() {
    let source = ARCS_SOURCE
        .iter()
        .copied()
        .collect::<Vec<_>>();
    let source = String::from_utf8(source)
        .unwrap_or_else(|_| unreachable!("fixture is UTF-8"))
        .replace("Name=\"ALPHA\"", "Name=\"alpha\"")
        .replace("<Text>ALPHA</Text>", "<Text>aLpHa (V.O.)</Text>");
    let mut project = imported(source.as_bytes());
    let expected = arc_paragraph(&project, "INT. ROOM - DAY", "alpha")
        .cloned()
        .unwrap_or_else(|| unreachable!("source arc exists"));

    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    assert_eq!(project.elements[1].text, "aLpHa (V.O.)");
    assert_eq!(
        arc_paragraph(&project, "INT. ROOM - DAY", "ALPHA"),
        Some(&expected)
    );
    let xml = String::from_utf8(write_v6(&project).unwrap_or_else(|_| unreachable!()))
        .unwrap_or_else(|_| unreachable!("serialized FDX is UTF-8"));
    assert!(xml.contains("<CharacterArcBeat Name=\"ALPHA\">"));
    assert!(xml.contains("<Text>aLpHa (V.O.)</Text>"));
}

#[test]
fn removed_character_removes_the_entire_derived_arc_and_its_text() {
    let mut project = imported(ARCS_SOURCE);
    let expected = arc_paragraph(&project, "INT. ROOM - DAY", "ALPHA")
        .cloned()
        .unwrap_or_else(|| unreachable!("source arc exists"));
    assert!(expected.children.iter().any(|node| match node {
        PreservedNode::Element(run) => run.children.iter().any(
            |part| matches!(part, PreservedNode::Text(text) if text == "alpha "),
        ),
        _ => false,
    }));

    project
        .elements
        .retain(|element| !(element.role == Role::Character && element.text == "ALPHA"));
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    assert!(arc_paragraph(&project, "INT. ROOM - DAY", "ALPHA").is_none());
    let output = write_v6(&project).unwrap_or_else(|_| unreachable!("reconciled project serializes"));
    let xml = String::from_utf8(output).unwrap_or_else(|_| unreachable!("serialized FDX is UTF-8"));
    assert!(!xml.contains(">alpha </Text>"));
    assert!(!xml.contains(">arc</Text>"));
    assert!(!xml.contains("Name=\"ALPHA\""));
}

#[test]
fn renamed_character_receives_a_blank_arc_instead_of_old_arc_text_by_id_or_position() {
    let mut project = imported(ARCS_SOURCE);
    let old = arc_paragraph(&project, "INT. ROOM - DAY", "ALPHA")
        .cloned()
        .unwrap_or_else(|| unreachable!("source arc exists"));
    let old_id = arc_id(&old).unwrap_or_default().to_owned();
    let cue = project
        .elements
        .iter_mut()
        .find(|element| element.role == Role::Character && element.text == "ALPHA")
        .unwrap_or_else(|| unreachable!("ALPHA cue exists"));
    cue.text = "GAMMA".to_owned();

    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    assert!(arc_paragraph(&project, "INT. ROOM - DAY", "ALPHA").is_none());
    let new = arc_paragraph(&project, "INT. ROOM - DAY", "GAMMA")
        .unwrap_or_else(|| unreachable!("renamed Character gets a derived arc"));
    assert_ne!(arc_id(new), Some(old_id.as_str()));
    assert!(new.children.is_empty());
}

#[test]
    fn ambiguous_duplicate_beats_and_multiple_nested_paragraphs_are_preserved() {
    let base = String::from_utf8(ARCS_SOURCE.to_vec())
        .unwrap_or_else(|_| unreachable!("fixture is UTF-8"));
    let duplicate_beat = base.replace(
        "</SceneArcBeats>",
        "<CharacterArcBeat Name=\"ALPHA\"><Paragraph id=\"arc-duplicate\"><Text>other arc</Text></Paragraph></CharacterArcBeat></SceneArcBeats>",
    );
    let multiple_paragraphs = base.replace(
        "</Paragraph></CharacterArcBeat>",
        "</Paragraph><Paragraph id=\"arc-second\"><Text>other arc</Text></Paragraph></CharacterArcBeat>",
    );

        for malformed in [duplicate_beat, multiple_paragraphs] {
            let mut project = imported(malformed.as_bytes());
            let before = project.clone();
            let _ = correct_cast_lists(&mut project).unwrap_or(false);
            assert_eq!(project, before);
        }
}

#[test]
fn same_character_name_keeps_independent_arc_content_per_scene() {
    let source = br#"<FinalDraft Version="6"><Content>
<Paragraph Type="Scene Heading" id="h1"><SceneProperties><SceneArcBeats><CharacterArcBeat Name="ALPHA"><Paragraph Alignment="Left" FirstIndent="0.00" Leading="Regular" LeftIndent="0.00" OutlineLevel="0" RightIndent="1.39" SpaceBefore="0" Spacing="1" StartsNewPage="No" id="arc-one"><Text Style="Bold">first scene</Text></Paragraph></CharacterArcBeat></SceneArcBeats></SceneProperties><Text>INT. FIRST - DAY</Text></Paragraph>
<Paragraph Type="Character" id="c1"><Text>ALPHA</Text></Paragraph>
<Paragraph Type="Scene Heading" id="h2"><SceneProperties><SceneArcBeats><CharacterArcBeat Name="ALPHA"><Paragraph Alignment="Left" FirstIndent="0.00" Leading="Regular" LeftIndent="0.00" OutlineLevel="0" RightIndent="1.39" SpaceBefore="0" Spacing="1" StartsNewPage="No" id="arc-two"><Text Style="Italic">second scene</Text></Paragraph></CharacterArcBeat></SceneArcBeats></SceneProperties><Text>INT. SECOND - DAY</Text></Paragraph>
<Paragraph Type="Character" id="c2"><Text>ALPHA</Text></Paragraph>
</Content></FinalDraft>"#;
    let mut project = imported(source);
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    let first = arc_paragraph(&project, "INT. FIRST - DAY", "ALPHA")
        .cloned()
        .unwrap_or_else(|| unreachable!("first-scene arc exists"));
    let second = arc_paragraph(&project, "INT. SECOND - DAY", "ALPHA")
        .cloned()
        .unwrap_or_else(|| unreachable!("second-scene arc exists"));
    assert_ne!(arc_id(&first), arc_id(&second));

    project.elements.insert(
        2,
        ScreenplayElement::new("beta", Role::Character, "BETA")
            .unwrap_or_else(|_| unreachable!()),
    );
    assert!(correct_cast_lists(&mut project).unwrap_or(false));
    assert_eq!(arc_paragraph(&project, "INT. FIRST - DAY", "ALPHA"), Some(&first));
    assert_eq!(arc_paragraph(&project, "INT. SECOND - DAY", "ALPHA"), Some(&second));
}

#[test]
fn undo_redo_save_and_save_as_keep_character_arc_association_and_content() {
    let directory = TestDirectory::new();
    let mut source_project = imported(ARCS_SOURCE);
    let _ = correct_cast_lists(&mut source_project).unwrap_or(false);
    let original_bytes = write_v6(&source_project).unwrap_or_else(|_| unreachable!());
    let source_path = directory.path("source.fdx");
    fs::write(&source_path, &original_bytes).unwrap_or_else(|_| unreachable!("fixture write"));
    let fs_backend = StdFdxStorageFileSystem;

    let opened = open(&fs_backend, &source_path).unwrap_or_else(|_| unreachable!("source opens"));
    let mut session = EditingSession::new(opened.document.project)
        .unwrap_or_else(|_| unreachable!("editing session initializes"));
    let original_arc = arc_paragraph(session.project(), "INT. ROOM - DAY", "ALPHA")
        .cloned()
        .unwrap_or_else(|| unreachable!("retained arc exists"));
    replace_character_text(&mut session, "ALPHA", "aLpHa (V.O.)");
    session
        .refresh_cast_lists()
        .unwrap_or_else(|_| unreachable!("supported arc refresh succeeds"));
    let edited_arc = arc_paragraph(session.project(), "INT. ROOM - DAY", "ALPHA")
        .cloned()
        .unwrap_or_else(|| unreachable!("normalized association remains"));
    assert_eq!(edited_arc, original_arc);

    session.undo().unwrap_or_else(|_| unreachable!("undo succeeds"));
    assert_eq!(
        arc_paragraph(session.project(), "INT. ROOM - DAY", "ALPHA"),
        Some(&original_arc)
    );
    session.redo().unwrap_or_else(|_| unreachable!("redo succeeds"));
    assert_eq!(
        arc_paragraph(session.project(), "INT. ROOM - DAY", "ALPHA"),
        Some(&edited_arc)
    );

    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    save_session(&fs_backend, &mut session, &mut target, FdxSaveMode::Save)
        .unwrap_or_else(|_| unreachable!("supported retained arc saves"));
    let saved_bytes = fs::read(&source_path).unwrap_or_else(|_| unreachable!("saved source exists"));
    let reopened = imported(&saved_bytes);
    let reopened_arc = arc_paragraph(&reopened, "INT. ROOM - DAY", "ALPHA")
        .cloned()
        .unwrap_or_else(|| unreachable!("saved arc reopens"));
    assert_eq!(reopened_arc, original_arc);
    assert!(reopened
        .elements
        .iter()
        .any(|element| element.role == Role::Character && element.text == "aLpHa (V.O.)"));

    let reopened_file = open(&fs_backend, &source_path).unwrap_or_else(|_| unreachable!("saved source reopens"));
    let mut save_as_session = EditingSession::new(reopened_file.document.project)
        .unwrap_or_else(|_| unreachable!("Save As session initializes"));
    let mut save_as_target = FdxSaveTarget::new_unsaved();
    let copy_path = directory.path("copy.fdx");
    save_session(
        &fs_backend,
        &mut save_as_session,
        &mut save_as_target,
        FdxSaveMode::SaveAs(copy_path.clone()),
    )
    .unwrap_or_else(|_| unreachable!("supported retained arc saves as a copy"));
    assert_eq!(fs::read(&source_path).unwrap_or_default(), saved_bytes);
    let copied = imported(&fs::read(copy_path).unwrap_or_else(|_| unreachable!("Save As copy exists")));
    assert_eq!(
        arc_paragraph(&copied, "INT. ROOM - DAY", "ALPHA"),
        Some(&original_arc)
    );
}

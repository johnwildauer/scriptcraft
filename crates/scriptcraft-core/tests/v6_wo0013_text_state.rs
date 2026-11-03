use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::fdx::{self, FdxDocument, TextState};
use scriptcraft_core::fdx_storage::{
    save_session_with_view_state, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem,
};
use scriptcraft_core::preservation::{PreservedElement, PreservedNode, PreservedSection};
use scriptcraft_core::text_state::{selection_to_flat_offsets, MAX_TEXT_SCALING, MIN_TEXT_SCALING};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let serial = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("scriptcraft-v6-wo0013-{}-{serial}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap_or_else(|_| unreachable!("test directory can be created"));
        Self(path)
    }
    fn path(&self, name: &str) -> PathBuf { self.0.join(name) }
}

impl Drop for TestDirectory {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
}

fn read(bytes: &[u8]) -> FdxDocument {
    fdx::read_v6(bytes).unwrap_or_else(|_| unreachable!("TextState fixture opens"))
}

fn root_children(sections: &[PreservedSection]) -> &[PreservedNode] {
    let root = sections.iter().flat_map(|section| &section.children).find_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "FinalDraft" => Some(element),
        _ => None,
    }).unwrap_or_else(|| unreachable!("root exists"));
    &root.children
}

fn named_child<'a>(children: &'a [PreservedNode], name: &str) -> Vec<&'a PreservedElement> {
    children.iter().filter_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == name => Some(element),
        _ => None,
    }).collect()
}

fn element_index(children: &[PreservedNode], name: &str) -> Option<usize> {
    children.iter().position(|node| matches!(node,
        PreservedNode::Element(element) if element.name.local_name == name
    ))
}

fn attr<'a>(element: &'a PreservedElement, name: &str) -> Option<&'a str> {
    element.attributes.iter().find(|attribute| attribute.name.namespace_uri.is_none() && attribute.name.local_name == name)
        .map(|attribute| attribute.value.as_str())
}

fn direct_xrefs(document_ref: &PreservedElement) -> Vec<&PreservedElement> {
    document_ref.children.iter().filter_map(|node| match node {
        PreservedNode::Element(element) if element.name.local_name == "XRef" => Some(element),
        _ => None,
    }).collect()
}

const SIMPLE: &[u8] = r#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="p1"><Text>abé🙂</Text></Paragraph><Paragraph Type="Action" id="p2"><Text>xyz</Text></Paragraph></Content></FinalDraft>"#.as_bytes();

#[test]
fn checked_in_references_project_root_text_state_and_exclude_title_page_state() {
    let references: [(&[u8], u16, (usize, usize)); 4] = [
        (include_bytes!("../../../.reference/reference.fdx"), 100, (0, 0)),
        (include_bytes!("../../../.reference/reference2.fdx"), 100, (403, 403)),
        (include_bytes!("../../../.reference/arc copy.fdx"), 100, (48, 48)),
        (include_bytes!("../../../.reference/real_script_example.fdx"), 75, (1277, 1277)),
    ];
    for (bytes, scaling, offsets) in references {
        let document = read(bytes);
        assert_eq!(document.text_state.scaling, scaling);
        assert_eq!(selection_to_flat_offsets(&document.project, document.text_state.selection), Some(offsets));
    }

    let nested = br#"<FinalDraft DocumentType="Script" Version="6"><TextState Scaling="125" Selection="1,2"/><Content><Paragraph Type="Action" id="p1"><Text>body</Text></Paragraph></Content><TitlePage><TextState Scaling="300" Selection="2,4"/></TitlePage></FinalDraft>"#;
    let document = read(nested);
    assert_eq!(document.text_state.scaling, 125);
    assert_eq!(selection_to_flat_offsets(&document.project, document.text_state.selection), Some((1, 2)));
}

#[test]
fn missing_duplicate_malformed_and_out_of_range_values_use_atomic_safe_defaults() {
    let cases: [(&[u8], u16); 10] = [
        (SIMPLE, 100),
        (br#"<FinalDraft DocumentType="Script" Version="6"><TextState Scaling="0" Selection="0,0"/><Content/></FinalDraft>"#, 100),
        (br#"<FinalDraft DocumentType="Script" Version="6"><TextState Scaling="-2" Selection="0,0"/><Content/></FinalDraft>"#, 100),
        (br#"<FinalDraft DocumentType="Script" Version="6"><TextState Scaling="bad" Selection="0,0"/><Content/></FinalDraft>"#, 100),
        (br#"<FinalDraft DocumentType="Script" Version="6"><TextState Scaling="18446744073709551616" Selection="0,0"/><Content/></FinalDraft>"#, 100),
        (br#"<FinalDraft DocumentType="Script" Version="6"><TextState Scaling="12" Selection="0,0"/><Content/></FinalDraft>"#, MIN_TEXT_SCALING),
        (br#"<FinalDraft DocumentType="Script" Version="6"><TextState Scaling="900" Selection="0,0"/><Content/></FinalDraft>"#, MAX_TEXT_SCALING),
        (br#"<FinalDraft DocumentType="Script" Version="6"><TextState Scaling="125" Selection="-1,2"/><Content/></FinalDraft>"#, 125),
        (br#"<FinalDraft DocumentType="Script" Version="6"><TextState Scaling="125" Selection="2,1"/><Content/></FinalDraft>"#, 125),
        (br#"<FinalDraft DocumentType="Script" Version="6"><TextState Scaling="125" Selection="0,999"/><Content/></FinalDraft>"#, 125),
    ];
    for (bytes, expected_scaling) in cases {
        let state = read(bytes).text_state;
        assert_eq!(state.scaling, expected_scaling);
        if expected_scaling == 125 || expected_scaling == 100 && bytes != SIMPLE {
            assert_eq!(state.selection, TextState::DEFAULT.selection);
        }
    }
    let duplicate = br#"<FinalDraft DocumentType="Script" Version="6"><TextState Scaling="200" Selection="0,0"/><TextState Scaling="250" Selection="1,1"/><Content/></FinalDraft>"#;
    assert_eq!(read(duplicate).text_state, TextState::DEFAULT);
    let malformed = br#"<FinalDraft DocumentType="Script" Version="6"><TextState Scaling="100" Selection="1,2,3"/><Content/></FinalDraft>"#;
    assert_eq!(read(malformed).text_state.selection, TextState::DEFAULT.selection);
    let nested_only = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="p1"><Text>x</Text></Paragraph></Content><TitlePage><TextState Scaling="200" Selection="0,1"/></TitlePage></FinalDraft>"#;
    assert_eq!(read(nested_only).text_state, TextState::DEFAULT);
}

#[test]
fn actual_save_overwrites_root_state_canonically_but_direct_serialization_and_title_page_are_preserved() {
    let source = r#"<FinalDraft DocumentType="Script" Version="6"><DocumentRef DateTime="20261001T120000" id="doc"/><WindowState Height="800" Left="1" Mode="Normal" Top="2" Width="900"/><TextState Scaling="100" Selection="1,1" ShowInvisibles="Yes"/><Content><Paragraph Type="Action" id="p1"><Text>abé🙂</Text></Paragraph><Paragraph Type="Action" id="p2"><Text>xyz</Text></Paragraph></Content><TextState Scaling="90" Selection="2,2"/><ElementSettings Type="Action"/><TitlePage><TextState Scaling="200" Selection="4,5" Vendor="preserve"/></TitlePage></FinalDraft>"#.as_bytes();
    let opened = read(source);
    assert_eq!(opened.text_state, TextState::DEFAULT);
    let original_sections = opened.project.preservation.as_ref().unwrap_or_else(|| unreachable!()).sections.clone();
    let direct = read(&fdx::write_v6(&opened.project).unwrap_or_else(|_| unreachable!("direct serialization retains source state")));
    assert_eq!(named_child(root_children(&direct.project.preservation.as_ref().unwrap_or_else(|| unreachable!()).sections), "TextState").len(), 2);

    let mut session = EditingSession::new(opened.project).unwrap_or_else(|_| unreachable!());
    let selection = Selection {
        anchor: Position { element: 1, offset: 2 },
        focus: Position { element: 0, offset: 1 },
    };
    session.set_selection(selection).unwrap_or_else(|_| unreachable!());
    let revision = session.revision();
    let directory = TestDirectory::new();
    let destination = directory.path("text-state-save-as.fdx");
    let mut target = FdxSaveTarget::new_unsaved();
    save_session_with_view_state(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(destination.clone()),
        fdx::WindowState::DEFAULT,
        TextState { scaling: 147, selection },
    ).unwrap_or_else(|_| unreachable!("current TextState Save As succeeds"));
    assert_eq!(session.revision(), revision);
    assert!(!session.dirty());

    let saved = read(&std::fs::read(destination).unwrap_or_else(|_| unreachable!()));
    assert_eq!(saved.window_state, Some(fdx::WindowState::DEFAULT));
    assert_eq!(saved.text_state.scaling, 147);
    assert_eq!(selection_to_flat_offsets(&saved.project, saved.text_state.selection), Some((1, 8)));
    let children = root_children(&saved.project.preservation.as_ref().unwrap_or_else(|| unreachable!()).sections);
    let states = named_child(children, "TextState");
    assert_eq!(states.len(), 1);
    assert_eq!(attr(states[0], "Scaling"), Some("147"));
    assert_eq!(attr(states[0], "Selection"), Some("1,7"));
    assert_eq!(attr(states[0], "ShowInvisibles"), Some("No"));
    let attrs = states[0].attributes.iter().map(|attribute| attribute.name.local_name.as_str()).collect::<Vec<_>>();
    assert_eq!(attrs, ["Scaling", "Selection", "ShowInvisibles"]);
    assert_eq!(named_child(root_children(&original_sections), "TitlePage"), named_child(children, "TitlePage"));
    let document_ref = named_child(children, "DocumentRef")[0];
    assert_eq!(direct_xrefs(document_ref).len(), 1);
}

#[test]
fn ordinary_save_persists_current_zoom_and_caret_without_dirtying_the_session() {
    let directory = TestDirectory::new();
    let path = directory.path("ordinary-save.fdx");
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><DocumentRef DateTime="20261001T120000" id="doc"/><WindowState Height="854" Left="0" Mode="Normal" Top="95" Width="870"/><TextState Scaling="100" Selection="0,0" ShowInvisibles="No"/><Content><Paragraph Type="Action" id="p1"><Text>line</Text></Paragraph></Content></FinalDraft>"#;
    std::fs::write(&path, source).unwrap_or_else(|_| unreachable!("source fixture is written"));
    let opened = scriptcraft_core::fdx_storage::open_source(&StdFdxStorageFileSystem, &path)
        .unwrap_or_else(|_| unreachable!("source fixture opens"));
    let mut session = EditingSession::new(read(source).project).unwrap_or_else(|_| unreachable!());
    let selection = Selection::caret(Position { element: 0, offset: 2 });
    session.set_selection(selection).unwrap_or_else(|_| unreachable!());
    let revision = session.revision();
    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    save_session_with_view_state(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
        fdx::WindowState::DEFAULT,
        TextState { scaling: 180, selection },
    ).unwrap_or_else(|_| unreachable!("ordinary Save persists current TextState"));
    let saved = read(&std::fs::read(path).unwrap_or_else(|_| unreachable!()));
    assert_eq!(saved.text_state.scaling, 180);
    assert_eq!(saved.text_state.selection, selection);
    assert_eq!(session.revision(), revision);
    assert!(!session.dirty());
}

#[test]
fn selection_endpoints_round_trip_non_bmp_and_paragraph_separator_boundaries() {
    let document = read(SIMPLE);
    let project = &document.project;
    for (flat, expected) in [
        (0, Position { element: 0, offset: 0 }),
        (2, Position { element: 0, offset: 2 }),
        (3, Position { element: 0, offset: 4 }),
        (5, Position { element: 0, offset: 8 }),
        (6, Position { element: 1, offset: 0 }),
        (9, Position { element: 1, offset: 3 }),
    ] {
        assert_eq!(scriptcraft_core::text_state::flat_offset_to_position(project, flat), Some(expected));
        assert_eq!(selection_to_flat_offsets(project, Selection::caret(expected)), Some((flat, flat)));
    }
    assert_eq!(scriptcraft_core::text_state::flat_offset_to_position(project, 4), None);

    let mixed_runs = r#"<FinalDraft DocumentType="Script" Version="6"><TextState Scaling="100" Selection="4,4"/><Content><Paragraph Type="Action" id="mixed"><Text>a</Text><Text>é🙂</Text></Paragraph></Content></FinalDraft>"#.as_bytes();
    let mixed = read(mixed_runs);
    assert_eq!(mixed.project.elements[0].text, "aé🙂");
    assert_eq!(selection_to_flat_offsets(&mixed.project, mixed.text_state.selection), Some((4, 4)));
}

#[test]
fn missing_and_generated_text_state_occupy_the_window_state_anchor() {
    let directory = TestDirectory::new();
    let mut session = EditingSession::new(read(SIMPLE).project).unwrap_or_else(|_| unreachable!());
    let destination = directory.path("missing-text-state.fdx");
    let mut target = FdxSaveTarget::new_unsaved();
    save_session_with_view_state(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(destination.clone()),
        fdx::WindowState::DEFAULT,
        TextState::DEFAULT,
    ).unwrap_or_else(|_| unreachable!("missing TextState is inserted"));
    let imported_output = read(&std::fs::read(destination).unwrap_or_else(|_| unreachable!()));
    let imported_children = root_children(&imported_output.project.preservation.as_ref().unwrap_or_else(|| unreachable!()).sections);
    let window = element_index(imported_children, "WindowState").unwrap_or_else(|| unreachable!());
    let text = element_index(imported_children, "TextState").unwrap_or_else(|| unreachable!());
    let settings = element_index(imported_children, "ElementSettings").unwrap_or(imported_children.len());
    assert!(window < text && text < settings);

    let new_project = scriptcraft_core::Project::new().unwrap_or_else(|_| unreachable!());
    let generated = read(&fdx::write_v6(&new_project).unwrap_or_else(|_| unreachable!("new output is generated")));
    let generated_children = root_children(&generated.project.preservation.as_ref().unwrap_or_else(|| unreachable!()).sections);
    let document_ref = element_index(generated_children, "DocumentRef").unwrap_or_else(|| unreachable!());
    let content = element_index(generated_children, "Content").unwrap_or_else(|| unreachable!());
    let window = element_index(generated_children, "WindowState").unwrap_or_else(|| unreachable!());
    let text = element_index(generated_children, "TextState").unwrap_or_else(|| unreachable!());
    let settings = element_index(generated_children, "ElementSettings").unwrap_or_else(|| unreachable!());
    assert!(document_ref < content && content < window && window < text && text < settings);
    assert_eq!(generated.text_state, TextState::DEFAULT);
}

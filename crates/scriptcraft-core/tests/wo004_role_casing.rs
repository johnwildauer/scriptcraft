use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::layout::{LayoutLine, LayoutPurpose, ScreenplayLayout};
use scriptcraft_core::{LayoutInputs, Project, Role};

const DEFAULT_ALL_CAPS_ROLES: &[Role] = &[
    Role::SceneHeading,
    Role::Character,
    Role::Transition,
    Role::Shot,
    Role::CastList,
    Role::NewAct,
    Role::Sequence,
    Role::EndOfAct,
];

fn session(role: Role, text: &str) -> EditingSession {
    let mut project = Project::new().unwrap_or_else(|_| unreachable!("new project must be valid"));
    project.elements[0].role = role;
    project.elements[0].text = text.into();
    EditingSession::new(project).unwrap_or_else(|_| unreachable!("fixture must form a session"))
}

fn caret(session: &mut EditingSession, element: usize, offset: usize) {
    session
        .set_selection(Selection::caret(Position { element, offset }))
        .unwrap_or_else(|_| unreachable!("fixture caret must be valid"));
}

fn line_for(project: &Project, element: usize) -> LayoutLine {
    let layout = ScreenplayLayout::build_with_inputs_for_purpose(
        project,
        &LayoutInputs::default(),
        LayoutPurpose::Editor,
    )
    .unwrap_or_else(|_| unreachable!("fixture must produce a layout"));
    layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .find(|line| line.element == element && !line.text.is_empty())
        .cloned()
        .unwrap_or_else(|| unreachable!("fixture text must produce a layout line"))
}

fn round_trip(project: &Project) -> Project {
    let bytes = write_v6(project).unwrap_or_else(|_| unreachable!("native FDX must serialize"));
    read_v6(&bytes)
        .unwrap_or_else(|_| unreachable!("native FDX output must reopen"))
        .project
}

#[test]
fn typing_and_paste_keep_exact_case_for_every_shared_default_all_caps_role() {
    for &role in DEFAULT_ALL_CAPS_ROLES {
        let mut editing = session(role, "");
        editing
            .insert_paste("MiXeD")
            .unwrap_or_else(|_| unreachable!("paste must preserve exact case"));

        let element = &editing.project().elements[0];
        assert_eq!(element.text, "MiXeD", "canonical source for {role:?}");
        let line = line_for(editing.project(), 0);
        assert!(line.text_style.all_caps, "shared AllCaps default for {role:?}");
        assert_eq!(line.text, "MiXeD", "layout canonical text for {role:?}");
        assert_eq!(line.display_text, "MIXED", "display text for {role:?}");
    }
}

#[test]
fn edit_role_transition_split_join_delete_and_history_retain_canonical_case() {
    let mut editing = session(Role::Action, "straße");
    caret(&mut editing, 0, "straße".len());
    editing
        .change_role(Role::Character)
        .unwrap_or_else(|_| unreachable!("supported role assignment must succeed"));
    assert_eq!(editing.project().elements[0].text, "straße");
    assert_eq!(editing.selection().focus.offset, "straße".len());

    editing
        .insert(" McKay")
        .unwrap_or_else(|_| unreachable!("insertion must preserve case"));
    assert_eq!(editing.project().elements[0].text, "straße McKay");
    editing.undo().unwrap_or_else(|_| unreachable!("insert undo must succeed"));
    assert_eq!(editing.project().elements[0].text, "straße");
    editing.redo().unwrap_or_else(|_| unreachable!("insert redo must succeed"));
    assert_eq!(editing.project().elements[0].text, "straße McKay");

    caret(&mut editing, 0, "straße".len());
    editing
        .split()
        .unwrap_or_else(|_| unreachable!("same-role split must preserve both sides"));
    assert_eq!(editing.project().elements[0].text, "straße");
    assert_eq!(editing.project().elements[1].text, " McKay");
    caret(&mut editing, 0, "straße".len());
    editing
        .join_with_next()
        .unwrap_or_else(|_| unreachable!("join must preserve both source spellings"));
    assert_eq!(editing.project().elements[0].text, "straße McKay");

    editing
        .set_selection(Selection {
            anchor: Position {
                element: 0,
                offset: 0,
            },
            focus: Position {
                element: 0,
                offset: 1,
            },
        })
        .unwrap_or_else(|_| unreachable!("ASCII selection must be valid"));
    editing.delete().unwrap_or_else(|_| unreachable!("delete must succeed"));
    assert_eq!(editing.project().elements[0].text, "traße McKay");
    editing.undo().unwrap_or_else(|_| unreachable!("delete undo must succeed"));
    assert_eq!(editing.project().elements[0].text, "straße McKay");

    let mut transitioned = session(Role::Transition, "cut to:");
    caret(&mut transitioned, 0, "cut to:".len());
    transitioned
        .tab()
        .unwrap_or_else(|_| unreachable!("Transition Tab must succeed"));
    assert_eq!(transitioned.project().elements[0].role, Role::SceneHeading);
    assert_eq!(transitioned.project().elements[0].text, "cut to:");
}

#[test]
fn expanding_unicode_is_display_only_and_keeps_source_byte_mapping() {
    let mut editing = session(Role::Character, "");
    editing
        .insert("straße")
        .unwrap_or_else(|_| unreachable!("typed Unicode must be retained"));
    assert_eq!(editing.selection().focus.offset, "straße".len());
    let line = line_for(editing.project(), 0);
    assert_eq!(line.text, "straße");
    assert_eq!(line.display_text, "STRASSE");
    let expansion = line
        .display_source_map
        .iter()
        .find(|segment| segment.source_start == 4 && segment.source_end == 6)
        .unwrap_or_else(|| unreachable!("sharp-s expansion must map to its source scalar"));
    assert_eq!((expansion.display_start, expansion.display_end), (4, 6));
    assert_eq!(line.source_for_display_char(5), 4);
    assert_eq!(line.display_char_for_source(5, false), 4);
    assert_eq!(line.display_char_for_source(5, true), 6);
}

#[test]
fn new_document_save_and_reopen_keep_mixed_case_with_display_all_caps() {
    for &role in DEFAULT_ALL_CAPS_ROLES {
        let mut editing = session(role, "");
        editing
            .insert("MiXeD Straße")
            .unwrap_or_else(|_| unreachable!("new-document edit must retain case"));
        let reopened = round_trip(editing.project());
        assert_eq!(reopened.elements[0].text, "MiXeD Straße", "native source for {role:?}");
        let line = line_for(&reopened, 0);
        assert_eq!(line.text, "MiXeD Straße", "reopened canonical line for {role:?}");
        assert_eq!(line.display_text, "MIXED STRASSE", "reopened display line for {role:?}");
    }
}

#[test]
fn imported_all_caps_and_mixed_case_source_survive_edit_save_and_reopen() {
    let source = b"<FinalDraft DocumentType='Script' Version='6'><Content><Paragraph Type='Character' id='cue-1'><Text>jOhN</Text></Paragraph></Content></FinalDraft>";
    let imported = read_v6(source)
        .unwrap_or_else(|_| unreachable!("import fixture must be valid"))
        .project;
    let mut editing = EditingSession::new(imported)
        .unwrap_or_else(|_| unreachable!("imported fixture must form a session"));
    assert_eq!(editing.project().elements[0].text, "jOhN");
    caret(&mut editing, 0, "jOhN".len());
    editing
        .insert_paste(" McKay")
        .unwrap_or_else(|_| unreachable!("imported text mapping must accept paste"));
    assert_eq!(editing.project().elements[0].text, "jOhN McKay");

    let reopened = round_trip(editing.project());
    assert_eq!(reopened.elements[0].text, "jOhN McKay");
    let line = line_for(&reopened, 0);
    assert_eq!(line.text, "jOhN McKay");
    assert_eq!(line.display_text, "JOHN MCKAY");
}

#[test]
fn imported_fontspec_override_without_allcaps_displays_canonical_case() {
    let source = b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Scene Heading'><FontSpec Font='Courier Final Draft' Size='12' Style='Bold'/></ElementSettings><Content><Paragraph Type='Scene Heading' id='heading-1'><Text>Int. Kitchen</Text></Paragraph></Content></FinalDraft>";
    let imported = read_v6(source)
        .unwrap_or_else(|_| unreachable!("source FontSpec fixture must import"))
        .project;
    let mut editing = EditingSession::new(imported)
        .unwrap_or_else(|_| unreachable!("source FontSpec fixture must form a session"));
    caret(&mut editing, 0, "Int. Kitchen".len());
    editing
        .insert(" - Night")
        .unwrap_or_else(|_| unreachable!("source-mapped edit must preserve case"));
    assert_eq!(editing.project().elements[0].text, "Int. Kitchen - Night");

    let reopened = round_trip(editing.project());
    assert_eq!(reopened.elements[0].text, "Int. Kitchen - Night");
    let line = line_for(&reopened, 0);
    assert!(!line.text_style.all_caps);
    assert_eq!(line.display_text, "Int. Kitchen - Night");
}

#[test]
fn legacy_json_uppercase_policy_field_remains_compatibility_only() {
    let project = Project::from_json(include_str!("fixtures/legacy_project.json"))
        .unwrap_or_else(|_| unreachable!("legacy fixture must remain readable"));
    let text = project
        .to_json()
        .unwrap_or_else(|_| unreachable!("legacy JSON fixture must serialize"));
    assert!(text.contains("\"uppercase_cues\":true"));

    let without_legacy_field = text.replace(",\"uppercase_cues\":true", "");
    assert!(Project::from_json(&without_legacy_field).is_ok());
    let unsupported_legacy_value = text.replace(
        "\"uppercase_cues\":true",
        "\"uppercase_cues\":false",
    );
    assert_eq!(
        Project::from_json(&unsupported_legacy_value),
        Err(scriptcraft_core::ModelError::UnsupportedJsonState)
    );
}

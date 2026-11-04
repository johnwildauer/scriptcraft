use scriptcraft_core::editing::{DerivedWorkSummary, EditingSession, Position, Selection};
use scriptcraft_core::fdx::read_v6;
use scriptcraft_core::fdx_storage::{open, save_session, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem};
use scriptcraft_core::preservation::{PreservedElement, PreservedNode, SourceOwnership};
use scriptcraft_core::{Project, Role, StableId};
use std::fs;
use std::path::PathBuf;

const IMPORTED_SCENE_WITHOUT_CAST_LIST: &[u8] = br#"<FinalDraft DocumentType="Script" Version="6"><Content>
<Paragraph Type="Scene Heading" ID="heading"><Text>INT. ROOM - DAY</Text></Paragraph>
<Paragraph Type="Character" ID="alice"><Text>ALICE</Text></Paragraph>
<Paragraph Type="Character" ID="bob"><Text>BOB</Text></Paragraph>
<Paragraph Type="Action" ID="tail"><Text>END</Text></Paragraph>
<Paragraph Type="General" ID="unrelated"><Text>KEEP THIS SOURCE</Text><VendorPayload Marker="retain"/></Paragraph>
</Content></FinalDraft>"#;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "scriptcraft-v3-wo002-{}",
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

fn managed_paragraph<'a>(project: &'a Project, id: &StableId) -> &'a PreservedElement {
    fn find<'a>(nodes: &'a [PreservedNode], id: &StableId) -> Option<&'a PreservedElement> {
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

    let preservation = project
        .preservation
        .as_ref()
        .unwrap_or_else(|| unreachable!("imported document retains source"));
    preservation
        .sections
        .iter()
        .find_map(|section| find(&section.children, id))
        .unwrap_or_else(|| unreachable!("inserted Cast List has a managed source paragraph"))
}

fn retained_text_runs(project: &Project, id: &StableId) -> Vec<String> {
    managed_paragraph(project, id)
        .children
        .iter()
        .filter_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == "Text" => {
                let mut text = String::new();
                for child in &element.children {
                    match child {
                        PreservedNode::Text(value) | PreservedNode::CData(value) => {
                            text.push_str(value);
                        }
                        _ => unreachable!("managed Cast List Text run contains plain text only"),
                    }
                }
                Some(text)
            }
            PreservedNode::Text(text) | PreservedNode::CData(text) if text.trim().is_empty() => None,
            _ => unreachable!("Cast List source contains only Text runs and formatting whitespace"),
        })
        .collect()
}

fn foreign_paragraph_id(project: &Project, cast_id: &StableId, step: &str) -> String {
    let element = project
        .elements
        .iter()
        .find(|element| element.id.as_str() == cast_id.as_str())
        .unwrap_or_else(|| panic!("{step}: inserted Cast List StableId {} is absent", cast_id.as_str()));
    assert_eq!(element.role, Role::CastList, "{step}: inserted element role");
    element
        .source_provenance
        .as_ref()
        .and_then(|provenance| provenance.foreign_id.clone())
        .unwrap_or_else(|| panic!("{step}: inserted Cast List has no durable foreign paragraph identity"))
}

fn cast_list_by_foreign_paragraph_id<'a>(
    project: &'a Project,
    foreign_id: &str,
    step: &str,
) -> &'a scriptcraft_core::ScreenplayElement {
    project
        .elements
        .iter()
        .find(|element| {
            element.source_provenance.as_ref().and_then(|provenance| provenance.foreign_id.as_deref())
                == Some(foreign_id)
        })
        .unwrap_or_else(|| {
            let candidates = project
                .elements
                .iter()
                .map(|element| {
                    format!(
                        "StableId={} role={:?} foreign_paragraph_id={:?}",
                        element.id.as_str(),
                        element.role,
                        element.source_provenance.as_ref().and_then(|provenance| provenance.foreign_id.as_deref())
                    )
                })
                .collect::<Vec<_>>();
            panic!(
                "{step}: no semantic element for durable foreign paragraph ID {foreign_id:?}; candidates: {candidates:?}"
            )
        })
}

fn retained_text_runs_for_foreign_id(project: &Project, foreign_id: &str, step: &str) -> Vec<String> {
    let element = cast_list_by_foreign_paragraph_id(project, foreign_id, step);
    assert_eq!(element.role, Role::CastList, "{step}: durable paragraph identity resolves to Cast List role");
    retained_text_runs(project, &element.id)
}

fn assert_project_semantic_source_runs(
    project: &Project,
    foreign_id: &str,
    expected: &str,
    step: &str,
) {
    let element = cast_list_by_foreign_paragraph_id(project, foreign_id, step);
    assert_eq!(element.role, Role::CastList, "{step}: semantic Cast List role");
    assert_eq!(element.text, expected);
    let semantic_runs = element
        .runs
        .iter()
        .map(|run| run.text.clone())
        .collect::<Vec<_>>();
    assert_eq!(semantic_runs, retained_text_runs_for_foreign_id(project, foreign_id, step));
    assert_eq!(semantic_runs, vec![expected.to_owned()]);
}

fn assert_semantic_source_runs(session: &EditingSession, foreign_id: &str, expected: &str, step: &str) {
    assert_project_semantic_source_runs(session.project(), foreign_id, expected, step);
}

fn assert_refresh_completed(session: &mut EditingSession, expected_cast: &str, foreign_id: &str, step: &str) {
    let outcome = session
        .refresh_cast_lists_with_outcome()
        .unwrap_or_else(|_| unreachable!("supported inserted Cast List source refreshes"));
    assert_eq!(outcome.requested_scenes, 1);
    assert_eq!(outcome.attempted_scenes, 1);
    assert_eq!(outcome.changed_scenes, 1);
    assert_eq!(outcome.unchanged_scenes, 0);
    assert_eq!(outcome.deferred_scenes, 0);
    assert_eq!(session.pending_derived_work_summary(), DerivedWorkSummary::default());
    assert_semantic_source_runs(session, foreign_id, expected_cast, step);
}

fn replace_character(session: &mut EditingSession, old: &str, new: &str) {
    let element = session
        .project()
        .elements
        .iter()
        .position(|element| element.role == Role::Character && element.text == old)
        .unwrap_or_else(|| unreachable!("target imported Character cue exists"));
    session
        .set_selection(Selection {
            anchor: Position { element, offset: 0 },
            focus: Position {
                element,
                offset: old.len(),
            },
        })
        .unwrap_or_else(|_| unreachable!("whole Character cue selection is valid"));
    session
        .insert(new)
        .unwrap_or_else(|_| unreachable!("canonical Character edit is supported"));
    let pending = session.pending_derived_work_summary();
    assert!(pending.pending && !pending.complete && pending.retryable);
    assert_eq!(pending.affected_scene_count, 1);
    assert_eq!(pending.deferred_scene_count, 0);
    assert_eq!(pending.failed_scene_count, 0);
}

fn assert_native_source_round_trip(
    bytes: &[u8],
    foreign_id: &str,
    expected: &str,
    expected_characters: &[&str],
    step: &str,
) {
    let reopened = read_v6(bytes)
        .unwrap_or_else(|_| unreachable!("saved native FDX reopens"))
        .project;
    let cast = cast_list_by_foreign_paragraph_id(&reopened, foreign_id, step);
    assert_eq!(cast.role, Role::CastList);
    assert_eq!(cast.text, expected);
    assert_eq!(
        cast.runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(),
        vec![expected]
    );
    assert_eq!(
        retained_text_runs_for_foreign_id(&reopened, foreign_id, step),
        vec![expected.to_owned()]
    );
    for character in expected_characters {
        assert!(reopened
            .elements
            .iter()
            .any(|element| element.role == Role::Character && element.text == *character));
    }
    let xml = String::from_utf8(bytes.to_vec())
        .unwrap_or_else(|_| unreachable!("native FDX output is UTF-8"));
    assert!(xml.contains("Marker=\"retain\""));
    assert!(xml.contains("KEEP THIS SOURCE"));
}

#[test]
fn imported_new_cast_list_stays_synchronized_across_edits_save_and_save_as_rebase() {
    let directory = TestDirectory::new();
    let source_path = directory.path("source.fdx");
    fs::write(&source_path, IMPORTED_SCENE_WITHOUT_CAST_LIST)
        .unwrap_or_else(|_| unreachable!("import fixture is written"));
    let filesystem = StdFdxStorageFileSystem;
    let opened = open(&filesystem, &source_path)
        .unwrap_or_else(|_| unreachable!("import fixture opens"));
    let mut session = EditingSession::new(opened.document.project)
        .unwrap_or_else(|_| unreachable!("imported document starts a session"));

    let initial_pending = session.pending_derived_work_summary();
    assert!(initial_pending.pending && initial_pending.complete && initial_pending.retryable);
    let initial = session
        .refresh_cast_lists_with_outcome()
        .unwrap_or_else(|_| unreachable!("initial Complete refresh succeeds"));
    assert_eq!(initial.requested_scenes, 1);
    assert_eq!(initial.attempted_scenes, 1);
    assert_eq!(initial.deferred_scenes, 0);
    assert_eq!(session.pending_derived_work_summary(), DerivedWorkSummary::default());

    let action = session
        .project()
        .elements
        .iter()
        .position(|element| element.role == Role::Action && element.text == "END")
        .unwrap_or_else(|| unreachable!("imported Action insertion point exists"));
    let action_end = session.project().elements[action].text.len();
    session
        .set_selection(Selection::caret(Position {
            element: action,
            offset: action_end,
        }))
        .unwrap_or_else(|_| unreachable!("Action end caret is valid"));
    session
        .enter()
        .unwrap_or_else(|_| unreachable!("Enter inserts a new managed paragraph"));
    let inserted_index = session.selection().focus.element;
    assert!(session.project().elements[inserted_index].text.is_empty());
    session
        .change_role(Role::CastList)
        .unwrap_or_else(|_| unreachable!("inserted paragraph accepts Cast List role"));
    let cast_id = session.project().elements[inserted_index].id.clone();
    let cast_foreign_id = foreign_paragraph_id(session.project(), &cast_id, "inserted Cast List setup");
    assert_eq!(session.project().elements[inserted_index].runs.len(), 0);

    // Creating a Cast List does not itself schedule derived work. The first
    // Character edit schedules the refresh that creates its semantic text and
    // source Text run; later Character edits must remain refreshable.
    replace_character(&mut session, "ALICE", "ALISON");
    assert_refresh_completed(&mut session, "(ALISON, BOB)", &cast_foreign_id, "first repeated refresh");

    replace_character(&mut session, "BOB", "BRIAN");
    assert_refresh_completed(&mut session, "(ALISON, BRIAN)", &cast_foreign_id, "second repeated refresh");

    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    let save_candidate = session
        .prepare_cast_list_save_candidate()
        .unwrap_or_else(|_| unreachable!("Save preflight prepares the synchronized candidate"));
    assert_project_semantic_source_runs(&save_candidate, &cast_foreign_id, "(ALISON, BRIAN)", "Save candidate");
    assert!(session.dirty(), "detached Save preparation does not publish clean state");
    save_session(&filesystem, &mut session, &mut target, FdxSaveMode::Save)
        .unwrap_or_else(|_| unreachable!("Save prepares and publishes the synchronized candidate"));
    assert!(!session.dirty());
    assert_eq!(session.pending_derived_work_summary(), DerivedWorkSummary::default());
    assert_semantic_source_runs(&session, &cast_foreign_id, "(ALISON, BRIAN)", "session after Save rebase");
    let saved = open(&filesystem, &source_path)
        .unwrap_or_else(|_| unreachable!("saved FDX reopens"));
    let saved_bytes = fs::read(&source_path)
        .unwrap_or_else(|_| unreachable!("published Save bytes remain readable"));
    assert_native_source_round_trip(
        &saved_bytes,
        &cast_foreign_id,
        "(ALISON, BRIAN)",
        &["ALISON", "BRIAN"],
        "Save native reopen",
    );
    assert_eq!(
        retained_text_runs_for_foreign_id(&saved.document.project, &cast_foreign_id, "Save reopened project"),
        vec!["(ALISON, BRIAN)".to_owned()]
    );

    replace_character(&mut session, "BRIAN", "BLAKE");
    assert_refresh_completed(&mut session, "(ALISON, BLAKE)", &cast_foreign_id, "post-Save refresh");

    let save_as_path = directory.path("copy.fdx");
    let save_as_candidate = session
        .prepare_cast_list_save_candidate()
        .unwrap_or_else(|_| unreachable!("Save As preflight prepares the synchronized candidate"));
    assert_project_semantic_source_runs(&save_as_candidate, &cast_foreign_id, "(ALISON, BLAKE)", "Save As candidate");
    assert!(session.dirty(), "detached Save As preparation does not publish clean state");
    save_session(
        &filesystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(save_as_path.clone()),
    )
    .unwrap_or_else(|_| unreachable!("Save As prepares and publishes the synchronized candidate"));
    assert!(!session.dirty());
    assert_eq!(session.pending_derived_work_summary(), DerivedWorkSummary::default());
    assert_semantic_source_runs(&session, &cast_foreign_id, "(ALISON, BLAKE)", "session after Save As rebase");
    let copied = open(&filesystem, &save_as_path)
        .unwrap_or_else(|_| unreachable!("Save As FDX reopens"));
    let copied_bytes = fs::read(&save_as_path)
        .unwrap_or_else(|_| unreachable!("published Save As bytes remain readable"));
    assert_native_source_round_trip(
        &copied_bytes,
        &cast_foreign_id,
        "(ALISON, BLAKE)",
        &["ALISON", "BLAKE"],
        "Save As native reopen",
    );
    assert_eq!(
        retained_text_runs_for_foreign_id(&copied.document.project, &cast_foreign_id, "Save As reopened project"),
        vec!["(ALISON, BLAKE)".to_owned()]
    );

    replace_character(&mut session, "ALISON", "ALICE");
    assert_refresh_completed(&mut session, "(ALICE, BLAKE)", &cast_foreign_id, "post-Save-As refresh");
}

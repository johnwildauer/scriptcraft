#![forbid(unsafe_code)]

use scriptcraft_fdx::read_v6;
use scriptcraft_model::{
    BodyOwner, ElementTypeKey, Position, Project, ScreenplayElement, Selection,
};
use scriptcraft_session::editing::{
    EditingSession, ElementDefinitionCandidate, ElementTypeRemapping,
};
use scriptcraft_session::storage::{
    save_session, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem,
};
use std::fs;
use std::path::PathBuf;

const OPEN_FIXTURE: &[u8] =
    include_bytes!("../../scriptcraft-fdx/tests/fixtures/open_element_settings.fdx");
const SAVE_CORRECTION_FDX: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<FinalDraft DocumentType="Script" Version="6">
  <Content>
    <Paragraph Type="Scene Heading" id="derived-save-scene">
      <Text>INT. ROOM - DAY</Text>
      <SceneProperties><SceneArcBeats /></SceneProperties>
    </Paragraph>
    <Paragraph Type="Character" id="derived-save-cue"><Text>ALICE</Text></Paragraph>
    <Paragraph Type="Dialogue" id="derived-save-dialogue"><Text>Hello.</Text></Paragraph>
    <Paragraph Type="Cast List" id="derived-save-cast"><Text>(OLD)</Text></Paragraph>
  </Content>
</FinalDraft>"#;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "scriptcraft-derived-smoke-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&path)
            .unwrap_or_else(|_| unreachable!("unique temporary directory can be created"));
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

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("dynamic derived source uses valid bounded input"))
}

fn key(label: &str) -> ElementTypeKey {
    valid(ElementTypeKey::new(label))
}

fn project_with_body(rows: &[(&str, &str)]) -> Project {
    let mut project = valid(scriptcraft_fdx::new_project());
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    for label in rows
        .iter()
        .map(|(label, _)| *label)
        .chain(["SpeakerCue", "scene heading"])
    {
        if definitions
            .iter()
            .any(|definition| definition.key.as_str() == label)
        {
            continue;
        }
        let mut definition = definitions
            .iter()
            .find(|definition| definition.key.as_str() == "Action")
            .cloned()
            .unwrap_or_else(|| unreachable!("the project contains its base Action definition"));
        definition.key = key(label);
        if label == "SpeakerCue" {
            definition.behavior.paginate_as = Some("Character".to_owned());
        }
        if label == "scene heading" {
            definition.behavior.paginate_as = Some("Scene Heading".to_owned());
        }
        definitions.push(definition);
    }
    valid(project.replace_element_definitions(definitions));
    project.elements = rows
        .iter()
        .enumerate()
        .map(|(index, (label, text))| {
            BodyOwner::shared(valid(ScreenplayElement::new(
                format!("derived-{index}"),
                key(label),
                *text,
            )))
        })
        .collect();
    project
}

fn caret(element: usize, offset: usize) -> Selection {
    Selection::caret(Position { element, offset })
}

fn change_type(session: &mut EditingSession, element: usize, label: &str) {
    valid(session.set_selection(caret(element, 0)));
    valid(session.change_element_type(key(label)));
}

fn session_with_derived_baseline(project: Project) -> EditingSession {
    let mut session = valid(EditingSession::new(project));
    let _ = valid(session.refresh_cast_lists_with_outcome());
    assert!(
        !session.pending_derived_work_summary().pending,
        "derived tests begin from a refreshed baseline"
    );
    session
}

#[test]
fn character_transitions_and_text_edits_invalidate_only_current_owning_scenes() {
    let project = project_with_body(&[
        ("Scene Heading", "INT. FIRST"),
        ("Character", "ALICE"),
        ("Dialogue", "Line one"),
        ("Scene Heading", "INT. SECOND"),
        ("Character", "BOB"),
    ]);
    let mut session = session_with_derived_baseline(project);

    // A capability-removing Type edit affects the cue's current scene only;
    // PaginateAs="Character" on SpeakerCue must not retain cue semantics.
    change_type(&mut session, 1, "SpeakerCue");
    let pending = session.pending_derived_work_summary();
    assert!(!pending.complete);
    assert_eq!(pending.affected_scene_count, 1);

    // An ordinary edit to that now-arbitrary paragraph leaves existing work
    // pending without inventing further semantic invalidation.
    valid(session.set_selection(caret(1, 0)));
    valid(session.insert("X"));
    assert_eq!(
        session.pending_derived_work_summary().affected_scene_count,
        1
    );

    // A Character text edit belongs to the second current scene and merges that
    // scope without converting it to a whole-document refresh.
    valid(session.set_selection(caret(4, 0)));
    valid(session.insert("X"));
    assert_eq!(
        session.pending_derived_work_summary().affected_scene_count,
        2
    );

    let mut reverse = session_with_derived_baseline(project_with_body(&[
        ("Scene Heading", "INT. ONLY"),
        ("SpeakerCue", "ALICE"),
    ]));
    change_type(&mut reverse, 1, "Character");
    assert_eq!(
        reverse.pending_derived_work_summary().affected_scene_count,
        1
    );
}

#[test]
fn heading_capability_transitions_ignore_case_variants_and_paginate_as() {
    let project = project_with_body(&[
        ("Scene Heading", "INT. FIRST"),
        ("Scene Heading", "INT. MIDDLE"),
        ("Character", "ALICE"),
        ("Scene Heading", "INT. FINAL"),
        ("Character", "BOB"),
    ]);
    let mut session = session_with_derived_baseline(project);

    // The lower-case key carries PaginateAs="Scene Heading", but is not a
    // structural boundary. The following cue transfers to the first scene.
    change_type(&mut session, 1, "scene heading");
    assert_eq!(
        session.pending_derived_work_summary().affected_scene_count,
        1
    );

    // Restoring the exact base key creates a new boundary and transfers cue
    // ownership back; the first and restored scenes both require correction.
    change_type(&mut session, 1, "Scene Heading");
    assert_eq!(
        session.pending_derived_work_summary().affected_scene_count,
        2
    );

    let mut heading_text = session_with_derived_baseline(project_with_body(&[
        ("Scene Heading", "INT. TITLE"),
        ("Character", "ALICE"),
    ]));
    valid(heading_text.set_selection(caret(0, 0)));
    valid(heading_text.insert("NEW "));
    assert_eq!(
        heading_text
            .pending_derived_work_summary()
            .affected_scene_count,
        1
    );
}

#[test]
fn split_join_delete_and_registry_remaps_schedule_derived_work() {
    let mut split = session_with_derived_baseline(project_with_body(&[
        ("Scene Heading", "INT. SPLIT"),
        ("Character", "ALICE"),
    ]));
    valid(split.set_selection(caret(0, 4)));
    valid(split.split());
    assert_eq!(split.pending_derived_work_summary().affected_scene_count, 2);

    let mut joined = session_with_derived_baseline(split.project().clone());
    valid(joined.set_selection(caret(0, joined.project().elements[0].text.len())));
    valid(joined.join_with_next());
    assert_eq!(
        joined.pending_derived_work_summary().affected_scene_count,
        1
    );

    let mut deleted = session_with_derived_baseline(project_with_body(&[
        ("Scene Heading", "INT. FIRST"),
        ("Scene Heading", ""),
        ("Character", "ALICE"),
    ]));
    valid(deleted.set_selection(caret(1, 0)));
    valid(deleted.delete());
    assert_eq!(
        deleted.pending_derived_work_summary().affected_scene_count,
        1
    );

    let mut remapped = session_with_derived_baseline(project_with_body(&[
        ("Scene Heading", "INT. ONLY"),
        ("Character", "ALICE"),
    ]));
    let mut definitions = remapped
        .project()
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    definitions.retain(|definition| definition.key.as_str() != "Character");
    let mut candidate = ElementDefinitionCandidate::new(definitions);
    candidate.body_remappings.push(ElementTypeRemapping::new(
        key("Character"),
        key("SpeakerCue"),
    ));
    valid(remapped.replace_element_definitions_and_remap_types(candidate));
    assert!(remapped.pending_derived_work_summary().complete);
}

#[test]
fn unsupported_scene_metadata_defers_without_blocking_the_edit_or_refresh() {
    let source = String::from_utf8_lossy(OPEN_FIXTURE).replace(
        "<SceneProperties><SceneNumber>not-semantic-for-arbitrary-type</SceneNumber></SceneProperties>",
        "<SceneProperties><SceneArcBeats><UnsupportedBeat /></SceneArcBeats></SceneProperties>",
    );
    let project = valid(read_v6(source.as_bytes())).project;
    let heading = project
        .elements
        .iter()
        .position(|element| element.element_type().as_str() == "NoSettingsBody")
        .unwrap_or_else(|| unreachable!("fixture contains its managed arbitrary paragraph"));
    let mut session = session_with_derived_baseline(project);

    change_type(&mut session, heading, "Scene Heading");
    let pending = session.pending_derived_work_summary();
    assert!(pending.pending);
    assert!(!pending.complete);
    assert_eq!(pending.affected_scene_count, 1);
    let correction = valid(session.refresh_cast_lists_with_outcome());
    assert!(correction.deferred);
    let pending = session.pending_derived_work_summary();
    assert!(pending.pending);
    assert_eq!(pending.deferred_scene_count, 1);
    assert!(!pending.retryable);
    assert_eq!(
        session.project().elements[heading].element_type(),
        key("Scene Heading")
    );
}

#[test]
fn pending_capability_invalidation_is_corrected_during_save_preflight() {
    let directory = TestDirectory::new();
    let destination = directory.path("derived-save.fdx");
    let project = valid(read_v6(SAVE_CORRECTION_FDX)).project;
    let cue = project
        .elements
        .iter()
        .position(|element| {
            element
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("derived-save-cue")
        })
        .unwrap_or_else(|| unreachable!("fixture Character cue is managed"));
    let cast = project
        .elements
        .iter()
        .position(|element| {
            element
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("derived-save-cast")
        })
        .unwrap_or_else(|| unreachable!("fixture Cast List is managed"));
    let mut session = valid(EditingSession::new(project));
    valid(session.set_selection(Selection {
        anchor: Position {
            element: cue,
            offset: 0,
        },
        focus: Position {
            element: cue,
            offset: "ALICE".len(),
        },
    }));
    valid(session.insert("BOB"));
    let pending = session.pending_derived_work_summary();
    assert!(pending.pending);
    assert!(pending.complete);

    let mut target = FdxSaveTarget::new_unsaved();
    let outcome = valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(destination),
    ));
    assert!(!outcome.temporary_cleanup_warning);
    assert_eq!(session.project().elements[cast].text, "(BOB)");
    assert!(!session.pending_derived_work_summary().pending);
    assert!(!session.dirty());
}

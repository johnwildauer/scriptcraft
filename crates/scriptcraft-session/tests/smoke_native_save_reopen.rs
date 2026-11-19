#![forbid(unsafe_code)]

use scriptcraft_fdx::read_v6;
use scriptcraft_model::dual_dialogue::{active_dual_dialogue_projection, map_group_identity};
use scriptcraft_model::{ElementDefinition, ElementTypeKey, Position, Project, Selection};
use scriptcraft_session::editing::EditingSession;
use scriptcraft_session::editing::{ElementDefinitionCandidate, ElementTypeRemapping};
use scriptcraft_session::storage::{
    open, save_session, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem,
};
use std::fs;
use std::path::PathBuf;

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("native save/reopen smoke uses valid bounded input"))
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("scriptcraft-native-smoke-{}", uuid::Uuid::new_v4()));
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

#[test]
fn native_save_reopen_public_api_smoke() {
    const BODY_TEXT: &str = "Native smoke — café";

    let directory = TestDirectory::new();
    let destination = directory.path("smoke-native.fdx");
    assert!(!destination.exists());

    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.insert(BODY_TEXT));
    assert_eq!(session.project().elements[0].text, BODY_TEXT);
    let expected_body = session
        .project()
        .elements
        .iter()
        .map(|owner| (owner.element_type(), owner.text.clone()))
        .collect::<Vec<_>>();
    let expected_title_page = session
        .project()
        .title_page
        .iter()
        .map(|paragraph| (paragraph.element_type(), paragraph.text.clone()))
        .collect::<Vec<_>>();
    let expected_direct_paragraph_overrides = session.project().direct_paragraph_overrides.clone();
    let expected_dual_dialogue_paragraph_overrides =
        session.project().dual_dialogue_paragraph_overrides.clone();
    let expected_dual_dialogue_groups = session.project().dual_dialogue_groups.clone();
    let mut target = FdxSaveTarget::new_unsaved();

    let outcome = valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(destination.clone()),
    ));

    assert!(!outcome.temporary_cleanup_warning);
    assert_eq!(target.path(), Some(destination.as_path()));
    assert!(target.baseline().is_some());
    assert!(!session.dirty());
    let saved_body = session
        .project()
        .elements
        .iter()
        .map(|owner| (owner.element_type(), owner.text.clone()))
        .collect::<Vec<_>>();
    let saved_title_page = session
        .project()
        .title_page
        .iter()
        .map(|paragraph| (paragraph.element_type(), paragraph.text.clone()))
        .collect::<Vec<_>>();
    assert_eq!(saved_body, expected_body);
    assert_eq!(saved_title_page, expected_title_page);
    assert_eq!(
        session.project().direct_paragraph_overrides,
        expected_direct_paragraph_overrides
    );
    assert_eq!(
        session.project().dual_dialogue_paragraph_overrides,
        expected_dual_dialogue_paragraph_overrides
    );
    assert_eq!(
        session.project().dual_dialogue_groups,
        expected_dual_dialogue_groups
    );

    let reopened = valid(open(&StdFdxStorageFileSystem, &destination));
    let reopened_project = reopened.document.project;
    assert_eq!(reopened_project.validate(), Ok(()));
    let saved_project = session.project();
    assert_eq!(reopened_project.elements, saved_project.elements);
    assert_eq!(reopened_project.title_page, saved_project.title_page);
    assert_eq!(
        reopened_project.direct_paragraph_overrides,
        saved_project.direct_paragraph_overrides
    );
    assert_eq!(
        reopened_project.dual_dialogue_paragraph_overrides,
        saved_project.dual_dialogue_paragraph_overrides
    );
    assert_eq!(
        reopened_project.dual_dialogue_groups,
        saved_project.dual_dialogue_groups
    );
    assert_eq!(
        reopened_project.element_definitions(),
        saved_project.element_definitions()
    );

    let reopened_session = valid(EditingSession::new(reopened_project));
    assert!(!reopened_session.dirty());
}

#[test]
fn save_rebases_selection_and_history_across_body_title_type_and_definition_changes() {
    const OPEN_FIXTURE: &[u8] =
        include_bytes!("../../scriptcraft-fdx/tests/fixtures/open_element_settings.fdx");

    fn key(label: &str) -> ElementTypeKey {
        ElementTypeKey::new(label).unwrap_or_else(|_| unreachable!("test type key is valid"))
    }
    fn rename(definitions: &mut [ElementDefinition], from: &str, to: &str) {
        let definition = definitions
            .iter_mut()
            .find(|definition| definition.key.as_str() == from)
            .unwrap_or_else(|| unreachable!("fixture includes the remapped definition"));
        definition.key = key(to);
    }

    let project = valid(read_v6(OPEN_FIXTURE)).project;
    let body_index = project
        .elements
        .iter()
        .position(|element| {
            element
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("open-body-lyrics")
        })
        .unwrap_or_else(|| unreachable!("fixture Lyrics paragraph is managed"));
    let title_index = project
        .title_page
        .iter()
        .position(|paragraph| {
            paragraph
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("open-title-content")
        })
        .unwrap_or_else(|| unreachable!("fixture TitlePage Content paragraph is managed"));
    let mut session = valid(EditingSession::new(project));
    let body_selection = Selection::caret(Position {
        element: body_index,
        offset: 3,
    });
    let title_selection = Selection::caret(Position {
        element: title_index,
        offset: 2,
    });
    valid(session.set_selection(body_selection));
    valid(session.set_title_selection(title_selection));

    let mut definitions = session
        .project()
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    rename(&mut definitions, "Lyrics", "Song");
    rename(&mut definitions, "TitleContent", "Credit");
    definitions.reverse();
    let expected_definitions = definitions.clone();
    let mut candidate = ElementDefinitionCandidate::new(definitions);
    candidate
        .body_remappings
        .push(ElementTypeRemapping::new(key("Lyrics"), key("Song")));
    candidate.title_remappings.push(ElementTypeRemapping::new(
        key("TitleContent"),
        key("Credit"),
    ));
    valid(session.replace_element_definitions_and_remap_types(candidate));
    let candidate_body_id = session.project().elements[body_index].id.clone();
    let candidate_title_id = session.project().title_page[title_index].id.clone();
    assert!(session.dirty());
    assert!(session.can_undo());

    let directory = TestDirectory::new();
    let destination = directory.path("changed-types.fdx");
    let mut target = FdxSaveTarget::new_unsaved();
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(destination.clone()),
    ));

    assert_eq!(session.project().elements[body_index].id, candidate_body_id);
    assert_eq!(
        session.project().elements[body_index].element_type(),
        key("Song")
    );
    assert_eq!(
        session.project().title_page[title_index].id,
        candidate_title_id
    );
    assert_eq!(
        session.project().title_page[title_index].element_type(),
        Some(key("Credit"))
    );
    assert_eq!(session.selection(), body_selection);
    assert_eq!(session.title_selection(), title_selection);
    assert_eq!(
        session
            .project()
            .element_definitions()
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        expected_definitions
    );
    assert!(!session.dirty());

    valid(session.undo());
    assert_eq!(
        session.project().elements[body_index].element_type(),
        key("Lyrics")
    );
    assert_eq!(
        session.project().title_page[title_index].element_type(),
        Some(key("TitleContent"))
    );
    assert!(session.dirty());
    valid(session.redo());
    assert_eq!(
        session.project().elements[body_index].element_type(),
        key("Song")
    );
    assert_eq!(
        session.project().title_page[title_index].element_type(),
        Some(key("Credit"))
    );
    assert!(!session.dirty());
}

#[test]
fn save_rebases_inactive_dual_identity_after_a_member_type_change() {
    const ACTIVE_DUAL_FIXTURE: &[u8] =
        include_bytes!("../../scriptcraft-fdx/tests/fixtures/active_dual_dialogue.fdx");

    fn key(label: &str) -> ElementTypeKey {
        ElementTypeKey::new(label).unwrap_or_else(|_| unreachable!("test type key is valid"))
    }

    let project = valid(read_v6(ACTIVE_DUAL_FIXTURE)).project;
    let mut session = valid(EditingSession::new(project));
    let group = session
        .project()
        .dual_dialogue_groups
        .first()
        .unwrap_or_else(|| unreachable!("fixture contains a dual dialogue group"));
    assert!(active_dual_dialogue_projection(session.project(), group).is_some());
    let left_cue = session
        .project()
        .elements
        .iter()
        .position(|element| {
            element
                .source_provenance
                .as_ref()
                .and_then(|source| source.foreign_id.as_deref())
                == Some("active-left-cue")
        })
        .unwrap_or_else(|| unreachable!("fixture left cue is managed"));
    valid(session.set_selection(Selection::caret(Position {
        element: left_cue,
        offset: 2,
    })));
    valid(session.change_element_type(key("Action")));
    let changed_project = session.project().clone();
    let changed_group_id = changed_project.dual_dialogue_groups[0].id.clone();
    assert_eq!(session.project().dual_dialogue_groups.len(), 1);
    assert!(active_dual_dialogue_projection(
        session.project(),
        &session.project().dual_dialogue_groups[0]
    )
    .is_none());

    let directory = TestDirectory::new();
    let destination = directory.path("inactive-dual.fdx");
    let mut target = FdxSaveTarget::new_unsaved();
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(destination),
    ));

    let saved_group = session
        .project()
        .dual_dialogue_groups
        .first()
        .unwrap_or_else(|| unreachable!("save preserves the dual source container"));
    assert_eq!(
        map_group_identity(&changed_project, session.project(), &changed_group_id),
        Some(saved_group.id.clone())
    );
    assert_eq!(
        saved_group.members,
        changed_project.dual_dialogue_groups[0].members
    );
    assert!(active_dual_dialogue_projection(session.project(), saved_group).is_none());
    assert!(!session.dirty());
}

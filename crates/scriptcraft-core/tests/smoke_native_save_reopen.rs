#![forbid(unsafe_code)]

use scriptcraft_core::editing::EditingSession;
use scriptcraft_core::fdx_storage::{
    open, save_session, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem,
};
use scriptcraft_core::Project;
use std::fs;
use std::path::PathBuf;

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("native save/reopen smoke uses valid bounded input"))
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "scriptcraft-native-smoke-{}",
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
        .map(|owner| (owner.role, owner.text.clone()))
        .collect::<Vec<_>>();
    let expected_title_page = session
        .project()
        .title_page
        .iter()
        .map(|paragraph| (paragraph.role, paragraph.text.clone()))
        .collect::<Vec<_>>();
    let expected_direct_paragraph_overrides = session.project().direct_paragraph_overrides.clone();
    let expected_dual_dialogue_paragraph_overrides = session
        .project()
        .dual_dialogue_paragraph_overrides
        .clone();
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
        .map(|owner| (owner.role, owner.text.clone()))
        .collect::<Vec<_>>();
    let saved_title_page = session
        .project()
        .title_page
        .iter()
        .map(|paragraph| (paragraph.role, paragraph.text.clone()))
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

    let reopened_session = valid(EditingSession::new(reopened_project));
    assert!(!reopened_session.dirty());
}

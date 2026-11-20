#![forbid(unsafe_code)]

use scriptcraft_fdx::{instantiate_builtin_template, BuiltinTemplateId};
use scriptcraft_model::{BodyOwner, ElementDefinition, TitlePageParagraph};
use scriptcraft_session::editing::EditingSession;
use scriptcraft_session::storage::{
    open, save_session, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem,
};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

#[track_caller]
fn valid<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("template save/reopen input failed: {error:?}"))
}

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "scriptcraft-template-save-reopen-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&path)
            .unwrap_or_else(|_| unreachable!("unique temporary directory can be created"));
        Self(path)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn full_content(
    project: &scriptcraft_model::Project,
) -> (
    Vec<Arc<BodyOwner>>,
    Vec<TitlePageParagraph>,
    Vec<ElementDefinition>,
) {
    (
        project.elements.clone(),
        project.title_page.clone(),
        project.element_definitions().iter().cloned().collect(),
    )
}

#[test]
fn every_builtin_requires_save_as_then_roundtrips_full_content_and_allows_ordinary_save() {
    for id in [
        BuiltinTemplateId::Screenplay,
        BuiltinTemplateId::Stageplay,
        BuiltinTemplateId::DramatistsGuildMusical,
    ] {
        let directory = TempDirectory::new();
        let existing_destination = directory.file("pre-existing.fdx");
        let existing_bytes = b"pre-existing destination must not be overwritten";
        fs::write(&existing_destination, existing_bytes)
            .unwrap_or_else(|_| unreachable!("temporary destination can be prepared"));
        let destination = directory.file("new-document.fdx");
        let template = valid(instantiate_builtin_template(id));
        let original_template_bytes = template
            .project
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!("built-in import retains original source"))
            .source
            .original_bytes
            .clone();
        let mut session = valid(EditingSession::new(template.project));
        let mut target = FdxSaveTarget::new_unsaved();
        let before_missing_baseline = session.project().clone();

        assert!(save_session(
            &StdFdxStorageFileSystem,
            &mut session,
            &mut target,
            FdxSaveMode::Save,
        )
        .is_err());
        assert_eq!(session.project(), &before_missing_baseline);
        assert!(target.path().is_none());
        assert!(target.baseline().is_none());

        valid(session.insert(" — edited before Save As"));
        let edited_project = session.project().clone();
        let edited_content = full_content(&edited_project);
        let edited_revision = session.revision();
        let edited_mutation_token = session.mutation_token();
        assert!(save_session(
            &StdFdxStorageFileSystem,
            &mut session,
            &mut target,
            FdxSaveMode::SaveAs(existing_destination.clone()),
        )
        .is_err());
        assert_eq!(
            fs::read(&existing_destination).ok().as_deref(),
            Some(existing_bytes.as_slice())
        );
        assert_eq!(session.project(), &edited_project);
        assert_eq!(session.revision(), edited_revision);
        assert_eq!(session.mutation_token(), edited_mutation_token);
        assert!(session.dirty());
        assert!(target.path().is_none());
        assert!(target.baseline().is_none());

        valid(save_session(
            &StdFdxStorageFileSystem,
            &mut session,
            &mut target,
            FdxSaveMode::SaveAs(destination.clone()),
        ));
        assert_eq!(target.path(), Some(destination.as_path()));
        assert!(target.baseline().is_some());
        assert!(!session.dirty());
        assert_eq!(full_content(session.project()), edited_content);

        let saved_bytes = fs::read(&destination)
            .unwrap_or_else(|_| unreachable!("successful Save As publishes an FDX"));
        let saved_source = std::str::from_utf8(&saved_bytes)
            .unwrap_or_else(|_| unreachable!("ordinary FDX output is UTF-8"));
        let root_tag = saved_source
            .split_once("<FinalDraft")
            .and_then(|(_, root)| root.split_once('>'))
            .map(|(root, _)| root)
            .unwrap_or_else(|| unreachable!("ordinary output has a FinalDraft root"));
        assert!(!root_tag.contains("Template="));
        assert!(!root_tag.contains("Template=\"No\""));
        assert!(saved_source.contains("HeaderAndFooter"));
        assert!(saved_source.contains("ScriptNotes"));
        assert_eq!(
            session
                .project()
                .preservation
                .as_ref()
                .map(|state| state.source.original_bytes.as_slice()),
            Some(saved_bytes.as_slice()),
            "successful publication rebases to ordinary marker-free output"
        );
        assert_ne!(saved_bytes, original_template_bytes);
        assert!(std::str::from_utf8(&original_template_bytes)
            .is_ok_and(|source| source.contains("Template=\"Yes\"")));

        let reopened = valid(open(&StdFdxStorageFileSystem, &destination));
        assert_eq!(full_content(&reopened.document.project), edited_content);
        let reopened_source = std::str::from_utf8(
            &reopened
                .document
                .project
                .preservation
                .as_ref()
                .unwrap_or_else(|| unreachable!("reopened FDX retains its saved source"))
                .source
                .original_bytes,
        )
        .unwrap_or_else(|_| unreachable!("saved FDX source is UTF-8"));
        let reopened_root = reopened_source
            .split_once("<FinalDraft")
            .and_then(|(_, root)| root.split_once('>'))
            .map(|(root, _)| root)
            .unwrap_or_else(|| unreachable!("reopened source has a FinalDraft root"));
        assert!(!reopened_root.contains("Template="));
        let mut reopened_session = valid(EditingSession::new(reopened.document.project));
        let mut reopened_target = FdxSaveTarget::from_baseline(reopened.baseline);
        assert!(!reopened_session.dirty());

        valid(reopened_session.insert(" and saved again"));
        valid(save_session(
            &StdFdxStorageFileSystem,
            &mut reopened_session,
            &mut reopened_target,
            FdxSaveMode::Save,
        ));
        assert!(!reopened_session.dirty());
        assert_eq!(reopened_target.path(), Some(destination.as_path()));
        let second_reopen = valid(open(&StdFdxStorageFileSystem, &destination));
        assert_eq!(
            second_reopen.document.project.elements,
            reopened_session.project().elements
        );
        assert_eq!(
            second_reopen.document.project.title_page,
            reopened_session.project().title_page
        );
        assert_eq!(
            second_reopen.document.project.element_definitions(),
            reopened_session.project().element_definitions()
        );
    }
}

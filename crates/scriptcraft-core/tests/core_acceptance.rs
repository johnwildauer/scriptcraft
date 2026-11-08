#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::exports::{export_pdf, export_text};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::fdx_storage::{
    save_session, FdxSaveMode, FdxSaveTarget, FdxStorageError, FdxStorageErrorKind,
    FdxStorageFileSystem, StdFdxStorageFileSystem, TransactionalSaveError,
};
use scriptcraft_core::layout::ScreenplayLayout;
use scriptcraft_core::{Project, Role};
use std::fs;
use std::cell::Cell;
use std::io::Write;
use std::path::{Path, PathBuf};

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("acceptance scenario uses valid bounded input"))
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "scriptcraft-core-acceptance-{}",
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
fn headless_layout_and_noncanonical_exports_are_repeatable_and_non_mutating() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.insert("A quiet room — café."));
    let before = session.clone();

    let first = valid(ScreenplayLayout::build(session.project()));
    let second = valid(ScreenplayLayout::build(session.project()));
    assert_eq!(first, second);
    assert!(!first.pages.is_empty());

    let pdf = valid(export_pdf(session.project()));
    let text = valid(export_text(session.project()));
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert!(text.text.contains("A QUIET ROOM"));
    assert!(text.loss.page_layout_lost);
    assert!(text.loss.roles_lost > 0);
    assert_eq!(session, before);
    assert!(session.dirty());
}

#[test]
fn native_fdx_reopen_preserves_semantics_opaque_source_and_established_ids() {
    let source = br#"<?xml version="1.0" encoding="UTF-8" standalone="no" ?>
<FinalDraft DocumentType="Script" Version="6"><FutureData marker="keep"><Payload>opaque-source</Payload></FutureData><Content><Paragraph Type="Scene Heading" id="scene-id"><Text>INT. ROOM - DAY</Text></Paragraph><Paragraph Type="Action" id="action-id"><Text>Original action</Text></Paragraph></Content></FinalDraft>"#;
    let imported = valid(read_v6(source));
    let mut session = valid(EditingSession::new(imported.project));
    valid(session.set_selection(Selection::caret(Position {
        element: 1,
        offset: "Original action".len(),
    })));
    valid(session.insert(" — déjà vu"));

    let saved = valid(write_v6(session.project()));
    let reopened = valid(read_v6(&saved));
    assert_eq!(reopened.project.elements.len(), 2);
    assert_eq!(reopened.project.elements[0].role, Role::SceneHeading);
    assert_eq!(reopened.project.elements[0].text, "INT. ROOM - DAY");
    assert_eq!(reopened.project.elements[1].role, Role::Action);
    assert_eq!(
        reopened.project.elements[1].text,
        "Original action — déjà vu"
    );
    let saved_xml = String::from_utf8(saved).unwrap_or_else(|_| unreachable!("FDX is UTF-8"));
    assert!(saved_xml.contains("marker=\"keep\""));
    assert!(saved_xml.contains("opaque-source"));
    assert!(saved_xml.contains("id=\"scene-id\""));
    assert!(saved_xml.contains("id=\"action-id\""));

    let saved_again = valid(write_v6(&reopened.project));
    let reopened_again = valid(read_v6(&saved_again));
    assert_eq!(reopened_again.project.elements, reopened.project.elements);
    let saved_again_xml =
        String::from_utf8(saved_again).unwrap_or_else(|_| unreachable!("FDX is UTF-8"));
    assert!(saved_again_xml.contains("id=\"scene-id\""));
    assert!(saved_again_xml.contains("id=\"action-id\""));
    assert!(saved_again_xml.contains("opaque-source"));
}

#[test]
fn failed_save_as_keeps_existing_destination_and_live_unsaved_session() {
    let directory = TestDirectory::new();
    let destination = directory.path("existing.fdx");
    fs::write(&destination, b"prior valid destination")
        .unwrap_or_else(|_| unreachable!("temporary destination can be written"));

    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.insert("unsaved screenplay text"));
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: 5,
    })));
    valid(session.set_title_selection(Selection::caret(Position {
        element: 17,
        offset: 0,
    })));
    valid(session.insert_title("unsaved title text"));
    valid(session.set_title_selection(Selection::caret(Position {
        element: 17,
        offset: "unsaved title text".len(),
    })));
    let before = session.clone();
    let mut target = FdxSaveTarget::new_unsaved();

    let error = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(destination.clone()),
    )
    .err()
    .unwrap_or_else(|| unreachable!("Save As must refuse an existing destination"));

    assert!(matches!(
        error,
        TransactionalSaveError::Storage(ref storage)
            if storage.kind == FdxStorageErrorKind::DestinationExists
    ));
    assert_eq!(
        fs::read(&destination).ok().as_deref(),
        Some(&b"prior valid destination"[..])
    );
    assert_eq!(session, before);
    assert!(session.dirty());
    assert_eq!(session.selection().focus.offset, 5);
    assert_eq!(session.title_selection().focus.offset, "unsaved title text".len());
    assert_eq!(session.project().title_page[17].text, "unsaved title textScript Title");
    assert!(target.path().is_none());
    assert!(target.baseline().is_none());
}

#[test]
fn confirmed_save_as_creates_an_absent_destination_without_changing_source() {
    let directory = TestDirectory::new();
    let source = directory.path("source.fdx");
    let destination = directory.path("new-destination.fdx");
    let source_bytes = valid(write_v6(&valid(Project::new())));
    fs::write(&source, &source_bytes)
        .unwrap_or_else(|_| unreachable!("temporary source can be written"));

    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.insert("new Save As screenplay"));
    let mut target = FdxSaveTarget::new_unsaved();
    let outcome = valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAsOverwrite(destination.clone()),
    ));

    let saved = valid(read_v6(&valid(fs::read(&destination))));
    assert_eq!(saved.project.elements[0].text, "new Save As screenplay");
    assert_eq!(valid(fs::read(&source)), source_bytes);
    assert_eq!(target.path(), Some(destination.as_path()));
    assert!(target.baseline().is_some());
    assert!(!session.dirty());
    assert!(!outcome.temporary_cleanup_warning);
}

#[test]
fn confirmed_save_as_replaces_an_existing_regular_destination() {
    let directory = TestDirectory::new();
    let destination = directory.path("existing.fdx");
    fs::write(&destination, b"prior destination bytes")
        .unwrap_or_else(|_| unreachable!("temporary destination can be written"));

    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.insert("replacement screenplay"));
    let mut target = FdxSaveTarget::new_unsaved();
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAsOverwrite(destination.clone()),
    ));

    let bytes = valid(fs::read(&destination));
    assert_ne!(bytes, b"prior destination bytes");
    let reopened = valid(read_v6(&bytes));
    assert_eq!(reopened.project.elements[0].text, "replacement screenplay");
    assert_eq!(target.path(), Some(destination.as_path()));
    assert_eq!(target.baseline().map(|baseline| baseline.path()), Some(destination.as_path()));
    assert!(!session.dirty());
}

#[cfg(unix)]
#[test]
fn confirmed_save_as_rejects_symlinks_without_changing_target_or_session() {
    let directory = TestDirectory::new();
    let target_file = directory.path("target.fdx");
    let destination = directory.path("link.fdx");
    fs::write(&target_file, b"symlink target remains unchanged")
        .unwrap_or_else(|_| unreachable!("temporary target can be written"));
    std::os::unix::fs::symlink(&target_file, &destination)
        .unwrap_or_else(|_| unreachable!("temporary symlink can be created"));
    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.insert("unsaved"));
    let before = session.clone();
    let mut target = FdxSaveTarget::new_unsaved();

    let error = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAsOverwrite(destination),
    )
    .err()
    .unwrap_or_else(|| unreachable!("confirmed Save As must not follow symlinks"));
    assert!(matches!(
        error,
        TransactionalSaveError::Storage(ref storage)
            if storage.kind == FdxStorageErrorKind::SymlinkDestination
    ));
    assert_eq!(valid(fs::read(&target_file)), b"symlink target remains unchanged");
    assert_eq!(session, before);
    assert!(session.dirty());
    assert!(target.path().is_none());
    assert!(target.baseline().is_none());
}

#[test]
fn confirmed_save_as_rejects_nonregular_and_oversized_destinations() {
    let directory = TestDirectory::new();
    let nonregular = directory.path("directory.fdx");
    fs::create_dir(&nonregular)
        .unwrap_or_else(|_| unreachable!("temporary destination directory can be created"));
    let oversized = directory.path("oversized.fdx");
    fs::File::create(&oversized)
        .and_then(|file| {
            file.set_len((scriptcraft_core::fdx::MAX_SOURCE_BYTES as u64).saturating_add(1))
        })
        .unwrap_or_else(|_| unreachable!("sparse oversized test file can be created"));

    for (destination, expected_kind) in [
        (nonregular, FdxStorageErrorKind::NotRegularFile),
        (oversized, FdxStorageErrorKind::SourceTooLarge),
    ] {
        let mut session = valid(EditingSession::new(valid(Project::new())));
        valid(session.insert("unsaved"));
        let before = session.clone();
        let mut target = FdxSaveTarget::new_unsaved();
        let error = save_session(
            &StdFdxStorageFileSystem,
            &mut session,
            &mut target,
            FdxSaveMode::SaveAsOverwrite(destination),
        )
        .err()
        .unwrap_or_else(|| unreachable!("unsafe destination must be rejected"));
        assert!(matches!(
            error,
            TransactionalSaveError::Storage(ref storage) if storage.kind == expected_kind
        ));
        assert_eq!(session, before);
        assert!(session.dirty());
        assert!(target.path().is_none());
        assert!(target.baseline().is_none());
    }
}

struct MutateDestinationAfterFirstRead {
    mutated: Cell<bool>,
}

impl FdxStorageFileSystem for MutateDestinationAfterFirstRead {
    type TempFile = fs::File;
    type Permissions = fs::Permissions;

    fn read_bounded(&self, path: &Path, limit: usize) -> Result<Vec<u8>, FdxStorageError> {
        let bytes = StdFdxStorageFileSystem.read_bounded(path, limit)?;
        if !self.mutated.replace(true) {
            fs::write(path, b"external concurrent update")
                .unwrap_or_else(|_| unreachable!("test destination can be changed"));
        }
        Ok(bytes)
    }

    fn inspect_destination(
        &self,
        path: &Path,
    ) -> Result<Option<Self::Permissions>, FdxStorageError> {
        StdFdxStorageFileSystem.inspect_destination(path)
    }

    fn create_temp(
        &self,
        destination: &Path,
    ) -> Result<(PathBuf, Self::TempFile), FdxStorageError> {
        StdFdxStorageFileSystem.create_temp(destination)
    }

    fn set_temp_permissions(
        &self,
        temp: &mut Self::TempFile,
        permissions: &Self::Permissions,
    ) -> Result<(), FdxStorageError> {
        StdFdxStorageFileSystem.set_temp_permissions(temp, permissions)
    }

    fn sync_temp(&self, temp: &mut Self::TempFile) -> Result<(), FdxStorageError> {
        StdFdxStorageFileSystem.sync_temp(temp)
    }

    fn replace(&self, temp: &Path, destination: &Path) -> Result<(), FdxStorageError> {
        StdFdxStorageFileSystem.replace(temp, destination)
    }

    fn install_new(&self, temp: &Path, destination: &Path) -> Result<(), FdxStorageError> {
        StdFdxStorageFileSystem.install_new(temp, destination)
    }

    fn remove_temp(&self, temp: &Path) -> Result<(), FdxStorageError> {
        StdFdxStorageFileSystem.remove_temp(temp)
    }
}

#[test]
fn confirmed_save_as_rejects_destination_changed_after_consent_and_rolls_back_session() {
    let directory = TestDirectory::new();
    let destination = directory.path("raced.fdx");
    fs::write(&destination, b"selected destination baseline")
        .unwrap_or_else(|_| unreachable!("temporary destination can be written"));
    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.insert("unsaved screenplay"));
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: 4,
    })));
    valid(session.set_title_selection(Selection::caret(Position {
        element: 17,
        offset: 0,
    })));
    valid(session.insert_title("unsaved title"));
    valid(session.set_title_selection(Selection::caret(Position {
        element: 17,
        offset: 8,
    })));
    let before = session.clone();
    let mut target = FdxSaveTarget::new_unsaved();
    let filesystem = MutateDestinationAfterFirstRead {
        mutated: Cell::new(false),
    };

    let error = save_session(
        &filesystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAsOverwrite(destination.clone()),
    )
    .err()
    .unwrap_or_else(|| unreachable!("external destination change must conflict"));
    assert!(matches!(
        error,
        TransactionalSaveError::Storage(ref storage)
            if storage.kind == FdxStorageErrorKind::Conflict
    ));
    assert_eq!(valid(fs::read(&destination)), b"external concurrent update");
    assert_eq!(session, before);
    assert!(session.dirty());
    assert_eq!(session.selection().focus.offset, 4);
    assert_eq!(session.title_selection().focus.offset, 8);
    assert_eq!(session.project().title_page[17].text, "unsaved titleScript Title");
    assert!(target.path().is_none());
    assert!(target.baseline().is_none());
}

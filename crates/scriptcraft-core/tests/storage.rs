#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::fdx_storage::{
    open, open_source, save_session, FdxSaveMode, FdxSaveTarget, FdxStorageError,
    FdxStorageErrorKind, FdxStorageFileSystem, StdFdxStorageFileSystem, StorageOperation,
    TransactionalSaveError,
};
use scriptcraft_core::preservation::{PreservedElement, PreservedNode};
use scriptcraft_core::Project;
use std::cell::Cell;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const OPAQUE_SOURCE: &[u8] = include_bytes!("fixtures/broad_compatibility.fdx");

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("acceptance scenario uses valid bounded input"))
}

fn preserved_elements(project: &Project) -> Vec<&PreservedElement> {
    fn visit<'a>(nodes: &'a [PreservedNode], output: &mut Vec<&'a PreservedElement>) {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            output.push(element);
            visit(&element.children, output);
        }
    }

    let mut output = Vec::new();
    if let Some(preservation) = &project.preservation {
        for section in &preservation.sections {
            visit(&section.children, &mut output);
        }
    }
    output
}

fn raw_attribute<'a>(element: &'a PreservedElement, local_name: &str) -> Option<&'a str> {
    element.attributes.iter().find(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == local_name
    }).map(|attribute| attribute.value.as_str())
}

fn assert_opaque_fixture_roundtrip(project: &Project) {
    let elements = preserved_elements(project);
    let notes = elements.iter().filter(|element| element.name.local_name == "ScriptNote")
        .copied().collect::<Vec<_>>();
    assert_eq!(notes.len(), 2);
    assert!(notes.iter().all(|note| raw_attribute(note, "ID") == Some("shared-note")));
    assert!(elements.iter().any(|element| {
        element.name.local_name == "ScriptNoteDefinition"
            && raw_attribute(element, "ID") == Some("shared-note")
    }));

    let body = elements.iter().copied().find(|element| {
        element.name.local_name == "Paragraph" && raw_attribute(element, "id") == Some("body-one")
    }).unwrap_or_else(|| unreachable!("synthetic managed owner remains source-backed"));
    assert_eq!(raw_attribute(body, "Bookmark"), Some("body-bookmark"));
    assert!(body.attributes.iter().any(|attribute| {
        attribute.name.local_name == "known-owner"
            && attribute.name.namespace_uri.as_deref() == Some("urn:scriptcraft:fixture:vendor")
            && attribute.value == "kept"
    }));
    let children = body.children.iter().filter_map(|node| match node {
        PreservedNode::Element(element) => Some(element.name.local_name.as_str()),
        _ => None,
    }).collect::<Vec<_>>();
    assert_eq!(children, [
        "BeforeText", "Text", "BetweenText", "Text", "ScriptNote", "ScriptNote",
        "UserParagraphData", "AfterText",
    ]);
    assert!(body.children.iter().any(|node| matches!(node,
        PreservedNode::Element(element) if element.name.local_name == "UserParagraphData"
            && raw_attribute(element, "vendor-data") == Some("retained")
    )));
    assert!(elements.iter().any(|element| {
        element.name.local_name == "FinalDraft"
            && element.attributes.iter().any(|attribute| {
                attribute.name.local_name == "root-flag"
                    && attribute.name.namespace_uri.as_deref()
                        == Some("urn:scriptcraft:fixture:vendor")
                    && attribute.value == "kept"
            })
    }));
    assert!(body.children.iter().any(|node| matches!(node,
        PreservedNode::Element(text) if text.name.local_name == "Text"
            && text.children.iter().any(|child| matches!(child,
                PreservedNode::Element(inline) if inline.name.local_name == "InlineMarkup"
            ))
    )));
}

fn note_subtrees(project: &Project) -> Vec<PreservedElement> {
    preserved_elements(project).into_iter()
        .filter(|element| element.name.local_name == "ScriptNote")
        .cloned()
        .collect()
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
fn opaque_noop_save_save_as_reopen_and_second_cycle_preserve_source_data() {
    let directory = TestDirectory::new();
    let source = directory.path("opaque-source.fdx");
    let save_as_one = directory.path("opaque-save-as-one.fdx");
    let save_as_two = directory.path("opaque-save-as-two.fdx");
    fs::write(&source, OPAQUE_SOURCE)
        .unwrap_or_else(|_| unreachable!("isolated fixture copy can be written"));
    fs::write(&save_as_one, b"prior Save As destination")
        .unwrap_or_else(|_| unreachable!("isolated Save As destination can be seeded"));

    let opened = valid(open(&StdFdxStorageFileSystem, &source));
    let original_project = opened.document.project.clone();
    let original_notes = note_subtrees(&original_project);
    let mut session = valid(EditingSession::new(opened.document.project));
    let mut target = FdxSaveTarget::from_baseline(opened.baseline);

    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    ));
    let saved = valid(read_v6(&valid(fs::read(&source))));
    assert_eq!(saved.project.elements, original_project.elements);
    assert_eq!(saved.project.title_page, original_project.title_page);
    assert_eq!(note_subtrees(&saved.project), original_notes);
    assert_opaque_fixture_roundtrip(&saved.project);
    let saved_source_bytes = valid(fs::read(&source));

    let reopened = valid(open(&StdFdxStorageFileSystem, &source));
    let mut session = valid(EditingSession::new(reopened.document.project));
    let mut target = FdxSaveTarget::from_baseline(reopened.baseline);
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAsOverwrite(save_as_one.clone()),
    ));
    let saved_as_one = valid(open(&StdFdxStorageFileSystem, &save_as_one));
    assert_eq!(saved_as_one.document.project.elements, original_project.elements);
    assert_eq!(saved_as_one.document.project.title_page, original_project.title_page);
    assert_eq!(note_subtrees(&saved_as_one.document.project), original_notes);
    assert_opaque_fixture_roundtrip(&saved_as_one.document.project);
    assert_eq!(valid(fs::read(&source)), saved_source_bytes);

    let saved_as_one_bytes = valid(fs::read(&save_as_one));
    let mut session = valid(EditingSession::new(saved_as_one.document.project));
    let mut target = FdxSaveTarget::from_baseline(saved_as_one.baseline);
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(save_as_two.clone()),
    ));
    let saved_as_two = valid(open(&StdFdxStorageFileSystem, &save_as_two));
    assert_eq!(saved_as_two.document.project.elements, original_project.elements);
    assert_eq!(saved_as_two.document.project.title_page, original_project.title_page);
    assert_eq!(note_subtrees(&saved_as_two.document.project), original_notes);
    assert_opaque_fixture_roundtrip(&saved_as_two.document.project);
    assert_eq!(valid(fs::read(&save_as_one)), saved_as_one_bytes);
}

#[test]
fn opaque_edit_save_rebases_session_without_detaching_owner_source() {
    let directory = TestDirectory::new();
    let source = directory.path("opaque-edited-rebase.fdx");
    fs::write(&source, OPAQUE_SOURCE)
        .unwrap_or_else(|_| unreachable!("isolated fixture copy can be written"));

    let opened = valid(open(&StdFdxStorageFileSystem, &source));
    let original_notes = note_subtrees(&opened.document.project);
    let mut session = valid(EditingSession::new(opened.document.project));
    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    valid(session.set_selection(Selection::caret(Position { element: 0, offset: 0 })));
    valid(session.insert("edited "));
    assert_opaque_fixture_roundtrip(session.project());

    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    ));
    assert!(!session.dirty());
    assert_opaque_fixture_roundtrip(session.project());
    assert_eq!(note_subtrees(session.project()), original_notes);
    let reopened = valid(open(&StdFdxStorageFileSystem, &source));
    assert_opaque_fixture_roundtrip(&reopened.document.project);
    assert_eq!(note_subtrees(&reopened.document.project), original_notes);
}

#[test]
fn opaque_save_as_conflict_preserves_source_destination_and_live_session() {
    let directory = TestDirectory::new();
    let source = directory.path("opaque-conflict-source.fdx");
    let destination = directory.path("opaque-existing-destination.fdx");
    let prior_destination = b"preexisting destination remains unchanged";
    fs::write(&source, OPAQUE_SOURCE)
        .unwrap_or_else(|_| unreachable!("isolated fixture copy can be written"));
    fs::write(&destination, prior_destination)
        .unwrap_or_else(|_| unreachable!("isolated destination can be seeded"));
    let opened = valid(open(&StdFdxStorageFileSystem, &source));
    let mut session = valid(EditingSession::new(opened.document.project));
    let before_session = session.clone();
    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    let before_target = target.clone();

    let error = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(destination.clone()),
    ).err().unwrap_or_else(|| unreachable!("Save As must reject an existing destination"));

    assert!(matches!(error, TransactionalSaveError::Storage(ref storage)
        if storage.kind == FdxStorageErrorKind::DestinationExists));
    assert_eq!(valid(fs::read(&source)), OPAQUE_SOURCE);
    assert_eq!(valid(fs::read(&destination)), prior_destination);
    assert_eq!(session, before_session);
    assert_eq!(target, before_target);
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
    assert_eq!(
        session.title_selection().focus.offset,
        "unsaved title text".len()
    );
    assert_eq!(
        session.project().title_page[17].text,
        "unsaved title textScript Title"
    );
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
fn create_only_save_as_creates_absent_destination_and_preserves_source() {
    let directory = TestDirectory::new();
    let source = directory.path("create-only-source.fdx");
    let destination = directory.path("create-only-new.fdx");
    let source_bytes = valid(write_v6(&valid(Project::new())));
    fs::write(&source, &source_bytes)
        .unwrap_or_else(|_| unreachable!("temporary source can be written"));
    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.insert("create-only Save As"));
    let mut target = FdxSaveTarget::new_unsaved();

    let outcome = valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(destination.clone()),
    ));

    let published = valid(fs::read(&destination));
    assert_eq!(
        valid(read_v6(&published)).project.elements[0].text,
        "create-only Save As"
    );
    assert_eq!(valid(fs::read(&source)), source_bytes);
    assert_eq!(target.path(), Some(destination.as_path()));
    assert_eq!(
        target.baseline().map(|baseline| baseline.byte_count()),
        Some(published.len())
    );
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
    assert_eq!(
        target.baseline().map(|baseline| baseline.path()),
        Some(destination.as_path())
    );
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
    assert_eq!(
        valid(fs::read(&target_file)),
        b"symlink target remains unchanged"
    );
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
    assert_eq!(
        session.project().title_page[17].text,
        "unsaved titleScript Title"
    );
    assert!(target.path().is_none());
    assert!(target.baseline().is_none());
}

#[test]
fn codec_candidate_failure_does_not_create_destination_or_rebase_session() {
    let directory = TestDirectory::new();
    let destination = directory.path("candidate-must-not-publish.fdx");
    let mut project = valid(Project::new());
    project.title_page[0].formatting.leading_regular = Some(false);
    let mut session = valid(EditingSession::new(project));
    let before = session.clone();
    let mut target = FdxSaveTarget::new_unsaved();

    let error = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(destination.clone()),
    )
    .err()
    .unwrap_or_else(|| unreachable!("unsupported FDX candidate fails before publish"));
    assert!(matches!(
        error,
        TransactionalSaveError::Codec(scriptcraft_core::fdx::FdxError::UnsupportedConstruct)
    ));
    assert!(!destination.exists());
    assert_eq!(session, before);
    assert_eq!(target.path(), None);
    assert!(!session.dirty());
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "injected temporary write failure",
        ))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "injected temporary flush failure",
        ))
    }
}

struct FailingWriteFileSystem;

impl FdxStorageFileSystem for FailingWriteFileSystem {
    type TempFile = FailingWriter;
    type Permissions = fs::Permissions;

    fn read_bounded(&self, path: &Path, limit: usize) -> Result<Vec<u8>, FdxStorageError> {
        StdFdxStorageFileSystem.read_bounded(path, limit)
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
        let (path, file) = StdFdxStorageFileSystem.create_temp(destination)?;
        drop(file);
        Ok((path, FailingWriter))
    }

    fn set_temp_permissions(
        &self,
        _temp: &mut Self::TempFile,
        _permissions: &Self::Permissions,
    ) -> Result<(), FdxStorageError> {
        Ok(())
    }

    fn sync_temp(&self, _temp: &mut Self::TempFile) -> Result<(), FdxStorageError> {
        Ok(())
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
fn temporary_write_failure_keeps_disk_session_selection_history_and_target_atomic() {
    let directory = TestDirectory::new();
    let path = directory.path("existing.fdx");
    let initial = valid(write_v6(&valid(Project::new())));
    fs::write(&path, &initial).unwrap_or_else(|_| unreachable!("seed FDX is writable"));
    let opened = valid(open(&StdFdxStorageFileSystem, &path));
    let mut session = valid(EditingSession::new(opened.document.project));
    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    valid(session.insert("unsaved body"));
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: 4,
    })));
    valid(session.set_title_selection(Selection::caret(Position {
        element: 17,
        offset: 0,
    })));
    valid(session.insert_title("unsaved title"));
    let before = session.clone();
    let before_target = target.clone();

    let error = save_session(
        &FailingWriteFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    )
    .err()
    .unwrap_or_else(|| unreachable!("injected temporary write fails"));
    assert!(matches!(error, TransactionalSaveError::Storage(ref storage)
        if storage.kind == FdxStorageErrorKind::Io(StorageOperation::WriteTemporary)));
    assert_eq!(fs::read(&path).ok().as_deref(), Some(initial.as_slice()));
    assert_eq!(session, before);
    assert!(session.dirty());
    assert!(session.can_undo());
    assert_eq!(session.selection(), before.selection());
    assert_eq!(session.title_selection(), before.title_selection());
    assert_eq!(target.path(), before_target.path());
    assert_eq!(target.baseline(), before_target.baseline());
}

#[test]
fn opaque_candidate_write_failure_preserves_source_and_live_session() {
    let directory = TestDirectory::new();
    let path = directory.path("opaque-write-failure.fdx");
    fs::write(&path, OPAQUE_SOURCE)
        .unwrap_or_else(|_| unreachable!("isolated fixture copy can be written"));
    let opened = valid(open(&StdFdxStorageFileSystem, &path));
    let mut session = valid(EditingSession::new(opened.document.project));
    let before_session = session.clone();
    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    let before_target = target.clone();

    let error = save_session(
        &FailingWriteFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    ).err().unwrap_or_else(|| unreachable!("injected candidate write failure must abort publication"));

    assert!(matches!(error, TransactionalSaveError::Storage(ref storage)
        if storage.kind == FdxStorageErrorKind::Io(StorageOperation::WriteTemporary)));
    assert_eq!(valid(fs::read(&path)), OPAQUE_SOURCE);
    assert_eq!(session, before_session);
    assert_eq!(target, before_target);
}

struct FailingSyncFileSystem;

impl FdxStorageFileSystem for FailingSyncFileSystem {
    type TempFile = fs::File;
    type Permissions = fs::Permissions;

    fn read_bounded(&self, path: &Path, limit: usize) -> Result<Vec<u8>, FdxStorageError> {
        StdFdxStorageFileSystem.read_bounded(path, limit)
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

    fn sync_temp(&self, _temp: &mut Self::TempFile) -> Result<(), FdxStorageError> {
        Err(FdxStorageError {
            kind: FdxStorageErrorKind::Io(StorageOperation::SyncTemporary),
            temporary_cleanup_failed: false,
        })
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
fn temporary_sync_failure_rolls_back_disk_session_target_and_staged_file() {
    let directory = TestDirectory::new();
    let path = directory.path("sync-failure.fdx");
    let initial = valid(write_v6(&valid(Project::new())));
    fs::write(&path, &initial).unwrap_or_else(|_| unreachable!("seed FDX is writable"));
    let opened = valid(open(&StdFdxStorageFileSystem, &path));
    let mut session = valid(EditingSession::new(opened.document.project));
    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    valid(session.insert("unsaved body before sync failure"));
    valid(session.set_selection(Selection::caret(Position {
        element: 0,
        offset: 7,
    })));
    let before = session.clone();
    let before_target = target.clone();

    let error = save_session(
        &FailingSyncFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    )
    .err()
    .unwrap_or_else(|| unreachable!("injected sync failure must fail"));

    assert!(matches!(error, TransactionalSaveError::Storage(ref storage)
        if storage.kind == FdxStorageErrorKind::Io(StorageOperation::SyncTemporary)));
    assert_eq!(valid(fs::read(&path)), initial);
    assert_eq!(session, before);
    assert!(session.dirty());
    assert!(session.can_undo());
    assert_eq!(session.selection(), before.selection());
    assert_eq!(target, before_target);
    assert_eq!(
        valid(fs::read_dir(&directory.0)).count(),
        1,
        "staged file is removed after sync failure"
    );
}

#[test]
fn standard_backend_opens_v6_with_exact_source_baseline() {
    let directory = TestDirectory::new();
    let path = directory.path("valid-v6.fdx");
    let bytes = valid(write_v6(&valid(Project::new())));
    fs::write(&path, &bytes).unwrap_or_else(|_| unreachable!("temporary FDX can be written"));

    let opened = valid(open(&StdFdxStorageFileSystem, &path));

    assert_eq!(opened.baseline.path(), path);
    assert_eq!(opened.baseline.byte_count(), bytes.len());
    let source = valid(open_source(&StdFdxStorageFileSystem, &path));
    assert_eq!(source.bytes(), bytes);
}

#[test]
fn standard_backend_reads_version_two_source_for_unchanged_preflight() {
    let directory = TestDirectory::new();
    let path = directory.path("older-version.fdx");
    let v6 = valid(write_v6(&valid(Project::new())));
    let v6_text = valid(String::from_utf8(v6));
    let older_text = v6_text.replacen("Version=\"6\"", "Version=\"2\"", 1);
    assert_ne!(older_text, v6_text, "serialized fixture declares Version 6");
    let older_bytes = older_text.into_bytes();
    fs::write(&path, &older_bytes)
        .unwrap_or_else(|_| unreachable!("temporary older-version FDX can be written"));

    let source = valid(open_source(&StdFdxStorageFileSystem, &path));
    assert_eq!(source.bytes(), older_bytes);
    assert!(matches!(
        scriptcraft_core::fdx::preflight_version(source.bytes()),
        Ok(scriptcraft_core::fdx::FdxVersionStatus::ExplicitNon6 {
            declared,
            direction: scriptcraft_core::fdx::FdxVersionDirection::Older,
        }) if declared == "2"
    ));
}

#[test]
fn standard_backend_enforces_read_limit_at_and_above_boundary() {
    let directory = TestDirectory::new();
    let within_limit = directory.path("within-limit.bin");
    let over_limit = directory.path("over-limit.bin");
    fs::write(&within_limit, b"12345678")
        .unwrap_or_else(|_| unreachable!("small boundary fixture can be written"));
    fs::write(&over_limit, b"123456789")
        .unwrap_or_else(|_| unreachable!("small oversized fixture can be written"));

    assert_eq!(
        valid(StdFdxStorageFileSystem.read_bounded(&within_limit, 8)),
        b"12345678"
    );
    let error = StdFdxStorageFileSystem
        .read_bounded(&over_limit, 8)
        .err()
        .unwrap_or_else(|| unreachable!("limit-plus-one input must be rejected"));
    assert_eq!(error.kind, FdxStorageErrorKind::SourceTooLarge);
}

#[test]
fn standard_backend_rejects_directory_and_oversized_sources_before_open() {
    let directory = TestDirectory::new();
    let folder = directory.path("folder.fdx");
    fs::create_dir(&folder).unwrap_or_else(|_| unreachable!("temporary directory can be created"));
    let exact_limit = directory.path("exact-limit.fdx");
    fs::File::create(&exact_limit)
        .and_then(|file| file.set_len(scriptcraft_core::fdx::MAX_SOURCE_BYTES as u64))
        .unwrap_or_else(|_| unreachable!("exact-limit sparse fixture can be created"));
    let oversized = directory.path("oversized.fdx");
    fs::File::create(&oversized)
        .and_then(|file| {
            file.set_len((scriptcraft_core::fdx::MAX_SOURCE_BYTES as u64).saturating_add(1))
        })
        .unwrap_or_else(|_| unreachable!("sparse oversized fixture can be created"));

    let directory_error = open_source(&StdFdxStorageFileSystem, &folder)
        .err()
        .unwrap_or_else(|| unreachable!("directory source must be rejected"));
    assert_eq!(directory_error.kind, FdxStorageErrorKind::NotRegularFile);
    assert_eq!(
        valid(
            StdFdxStorageFileSystem
                .read_bounded(&exact_limit, scriptcraft_core::fdx::MAX_SOURCE_BYTES)
        )
        .len(),
        scriptcraft_core::fdx::MAX_SOURCE_BYTES
    );
    let oversized_error = open_source(&StdFdxStorageFileSystem, &oversized)
        .err()
        .unwrap_or_else(|| unreachable!("oversized source must be rejected"));
    assert_eq!(oversized_error.kind, FdxStorageErrorKind::SourceTooLarge);
}

#[test]
fn ordinary_save_publishes_valid_fdx_and_updates_target_baseline() {
    let directory = TestDirectory::new();
    let path = directory.path("ordinary-save.fdx");
    let initial = valid(write_v6(&valid(Project::new())));
    fs::write(&path, &initial).unwrap_or_else(|_| unreachable!("seed FDX can be written"));
    let opened = valid(open(&StdFdxStorageFileSystem, &path));
    let mut session = valid(EditingSession::new(opened.document.project));
    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    valid(session.insert("ordinary Windows save"));

    let outcome = valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    ));

    let published = valid(fs::read(&path));
    let reopened = valid(read_v6(&published));
    assert_eq!(reopened.project.elements[0].text, "ordinary Windows save");
    assert_ne!(published, initial);
    assert_eq!(target.path(), Some(path.as_path()));
    assert_eq!(
        target.baseline().map(|baseline| baseline.byte_count()),
        Some(published.len())
    );
    assert_eq!(
        target.baseline().map(|baseline| baseline.path()),
        Some(path.as_path())
    );
    assert!(!session.dirty());
    assert!(!outcome.temporary_cleanup_warning);
}

#[test]
fn ordinary_save_conflict_preserves_external_bytes_and_live_state() {
    let directory = TestDirectory::new();
    let path = directory.path("externally-edited.fdx");
    let initial = valid(write_v6(&valid(Project::new())));
    fs::write(&path, &initial).unwrap_or_else(|_| unreachable!("seed FDX can be written"));
    let opened = valid(open(&StdFdxStorageFileSystem, &path));
    let mut session = valid(EditingSession::new(opened.document.project));
    valid(session.insert("unsaved local edit"));
    let before = session.clone();
    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    let before_target = target.clone();
    let external = b"external edit that must not be overwritten";
    fs::write(&path, external).unwrap_or_else(|_| unreachable!("external edit can be written"));

    let error = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    )
    .err()
    .unwrap_or_else(|| unreachable!("external content change must conflict"));

    assert!(matches!(error, TransactionalSaveError::Storage(ref storage)
        if storage.kind == FdxStorageErrorKind::Conflict));
    assert_eq!(valid(fs::read(&path)), external);
    assert_eq!(session, before);
    assert!(session.dirty());
    assert_eq!(target, before_target);
}

struct CreateDestinationDuringInstall {
    destination: PathBuf,
}

impl FdxStorageFileSystem for CreateDestinationDuringInstall {
    type TempFile = fs::File;
    type Permissions = fs::Permissions;

    fn read_bounded(&self, path: &Path, limit: usize) -> Result<Vec<u8>, FdxStorageError> {
        StdFdxStorageFileSystem.read_bounded(path, limit)
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
        fs::write(&self.destination, b"racing creator owns this file")
            .unwrap_or_else(|_| unreachable!("racing creator can create destination"));
        StdFdxStorageFileSystem.install_new(temp, destination)
    }

    fn remove_temp(&self, temp: &Path) -> Result<(), FdxStorageError> {
        StdFdxStorageFileSystem.remove_temp(temp)
    }
}

#[test]
fn save_as_racing_creator_is_not_overwritten_and_staged_file_is_cleaned() {
    let directory = TestDirectory::new();
    let destination = directory.path("racing-create.fdx");
    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.insert("candidate must not publish"));
    let before = session.clone();
    let mut target = FdxSaveTarget::new_unsaved();
    let filesystem = CreateDestinationDuringInstall {
        destination: destination.clone(),
    };

    let error = save_session(
        &filesystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(destination.clone()),
    )
    .err()
    .unwrap_or_else(|| unreachable!("create-only install must reject racing destination"));

    assert!(matches!(error, TransactionalSaveError::Storage(ref storage)
        if storage.kind == FdxStorageErrorKind::DestinationExists));
    assert_eq!(
        valid(fs::read(&destination)),
        b"racing creator owns this file"
    );
    assert_eq!(session, before);
    assert!(session.dirty());
    assert!(target.path().is_none());
    assert!(target.baseline().is_none());
    assert_eq!(
        valid(fs::read_dir(&directory.0)).count(),
        1,
        "staged file is removed on failed install"
    );
}

struct FailReplacement;

impl FdxStorageFileSystem for FailReplacement {
    type TempFile = fs::File;
    type Permissions = fs::Permissions;

    fn read_bounded(&self, path: &Path, limit: usize) -> Result<Vec<u8>, FdxStorageError> {
        StdFdxStorageFileSystem.read_bounded(path, limit)
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

    fn replace(&self, _temp: &Path, destination: &Path) -> Result<(), FdxStorageError> {
        StdFdxStorageFileSystem.replace(
            &destination.with_file_name("missing-temp-file"),
            destination,
        )
    }

    fn install_new(&self, temp: &Path, destination: &Path) -> Result<(), FdxStorageError> {
        StdFdxStorageFileSystem.install_new(temp, destination)
    }

    fn remove_temp(&self, temp: &Path) -> Result<(), FdxStorageError> {
        StdFdxStorageFileSystem.remove_temp(temp)
    }
}

#[test]
fn failed_native_replacement_preserves_old_destination_and_session() {
    let directory = TestDirectory::new();
    let path = directory.path("replacement-failure.fdx");
    let initial = valid(write_v6(&valid(Project::new())));
    fs::write(&path, &initial).unwrap_or_else(|_| unreachable!("seed FDX can be written"));
    let opened = valid(open(&StdFdxStorageFileSystem, &path));
    let mut session = valid(EditingSession::new(opened.document.project));
    valid(session.insert("unsaved candidate"));
    let before = session.clone();
    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    let before_target = target.clone();

    let error = save_session(
        &FailReplacement,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    )
    .err()
    .unwrap_or_else(|| unreachable!("injected native replacement failure must fail"));

    assert!(matches!(error, TransactionalSaveError::Storage(ref storage)
        if storage.kind == FdxStorageErrorKind::Io(StorageOperation::Replace)));
    assert_eq!(valid(fs::read(&path)), initial);
    assert_eq!(session, before);
    assert!(session.dirty());
    assert_eq!(target, before_target);
    assert_eq!(
        valid(fs::read_dir(&directory.0)).count(),
        1,
        "staged file is removed after replace failure"
    );
}

#[cfg(windows)]
#[test]
fn windows_native_replacement_sharing_violation_rolls_back_and_cleans_staged_file() {
    use std::os::windows::fs::OpenOptionsExt;

    let directory = TestDirectory::new();
    let path = directory.path("sharing-violation.fdx");
    let initial = valid(write_v6(&valid(Project::new())));
    fs::write(&path, &initial).unwrap_or_else(|_| unreachable!("seed FDX is writable"));
    let opened = valid(open(&StdFdxStorageFileSystem, &path));
    let mut session = valid(EditingSession::new(opened.document.project));
    valid(session.insert("unsaved candidate"));
    let before = session.clone();
    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    let before_target = target.clone();

    // Permit the storage preflight/read handles, but deny FILE_SHARE_DELETE so
    // Windows' native replacement rename encounters a real sharing violation.
    let blocker = fs::OpenOptions::new()
        .read(true)
        .share_mode(0x0000_0001 | 0x0000_0002)
        .open(&path)
        .unwrap_or_else(|_| unreachable!("sharing blocker can open the destination"));

    let error = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    )
    .err()
    .unwrap_or_else(|| unreachable!("native replacement must fail while delete sharing is denied"));

    assert!(matches!(error, TransactionalSaveError::Storage(ref storage)
        if storage.kind == FdxStorageErrorKind::Io(StorageOperation::Replace)));
    assert_eq!(valid(fs::read(&path)), initial);
    assert_eq!(session, before);
    assert!(session.dirty());
    assert_eq!(target, before_target);
    assert_eq!(
        valid(fs::read_dir(&directory.0)).count(),
        1,
        "staged file is removed after native replacement failure"
    );
    drop(blocker);
}

#[cfg(windows)]
#[test]
fn windows_file_symlink_is_rejected_without_reading_or_replacing_target() {
    use std::os::windows::fs::symlink_file;

    let directory = TestDirectory::new();
    let target_file = directory.path("real-target.fdx");
    let link = directory.path("file-link.fdx");
    fs::write(&target_file, b"reparse target must remain untouched")
        .unwrap_or_else(|_| unreachable!("symlink target can be written"));
    if let Err(error) = symlink_file(&target_file, &link) {
        eprintln!(
            "SKIP windows_file_symlink_is_rejected_without_reading_or_replacing_target: creating a Windows file symlink failed ({error}); this host may lack Developer Mode or symlink privilege"
        );
        return;
    }

    let bounded_error = StdFdxStorageFileSystem
        .read_bounded(&link, scriptcraft_core::fdx::MAX_SOURCE_BYTES)
        .err()
        .unwrap_or_else(|| unreachable!("bounded read must reject the file reparse point"));
    match bounded_error.kind {
        FdxStorageErrorKind::SymlinkDestination => eprintln!(
            "windows file reparse direct-read outcome: native open succeeded on the reparse object; handle metadata rejected it"
        ),
        FdxStorageErrorKind::Io(StorageOperation::Read) => eprintln!(
            "windows file reparse direct-read outcome: native open rejected the reparse object before handle metadata validation"
        ),
        other => unreachable!("unexpected direct bounded-read result: {other:?}"),
    }

    let open_error = open_source(&StdFdxStorageFileSystem, &link)
        .err()
        .unwrap_or_else(|| unreachable!("file symlink must not be followed for Open"));
    assert_eq!(open_error.kind, FdxStorageErrorKind::SymlinkDestination);

    let mut session = valid(EditingSession::new(valid(Project::new())));
    valid(session.insert("not published through reparse point"));
    let before = session.clone();
    let mut target = FdxSaveTarget::new_unsaved();
    let save_error = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAsOverwrite(link),
    )
    .err()
    .unwrap_or_else(|| {
        unreachable!("confirmed overwrite must reject final-component reparse point")
    });
    assert!(
        matches!(save_error, TransactionalSaveError::Storage(ref storage)
        if storage.kind == FdxStorageErrorKind::SymlinkDestination)
    );
    assert_eq!(
        valid(fs::read(&target_file)),
        b"reparse target must remain untouched"
    );
    assert_eq!(session, before);
    assert!(target.path().is_none());
    assert!(target.baseline().is_none());
}

#[cfg(windows)]
struct SwapToFileSymlinkAfterInspection {
    target: PathBuf,
    swapped: Cell<bool>,
    read_attempted: Cell<bool>,
}

#[cfg(windows)]
impl FdxStorageFileSystem for SwapToFileSymlinkAfterInspection {
    type TempFile = fs::File;
    type Permissions = fs::Permissions;

    fn read_bounded(&self, path: &Path, limit: usize) -> Result<Vec<u8>, FdxStorageError> {
        self.read_attempted.set(true);
        StdFdxStorageFileSystem.read_bounded(path, limit)
    }

    fn inspect_destination(
        &self,
        path: &Path,
    ) -> Result<Option<Self::Permissions>, FdxStorageError> {
        let permissions = StdFdxStorageFileSystem.inspect_destination(path)?;
        if !self.swapped.replace(true) {
            fs::remove_file(path)
                .unwrap_or_else(|_| unreachable!("preflighted regular file can be removed"));
            std::os::windows::fs::symlink_file(&self.target, path)
                .unwrap_or_else(|_| unreachable!("preflighted path can be swapped to a symlink"));
        }
        Ok(permissions)
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

#[cfg(windows)]
#[test]
fn windows_preflight_to_bounded_read_file_symlink_swap_is_rejected() {
    use std::os::windows::fs::symlink_file;

    let directory = TestDirectory::new();
    let target = directory.path("swap-target.bin");
    let source = directory.path("swap-source.fdx");
    let probe = directory.path("symlink-capability-probe");
    fs::write(&target, b"swap target remains unchanged")
        .unwrap_or_else(|_| unreachable!("symlink target can be written"));
    fs::write(&source, b"regular file observed during preflight")
        .unwrap_or_else(|_| unreachable!("preflight source can be written"));
    if let Err(error) = symlink_file(&target, &probe) {
        eprintln!(
            "SKIP windows_preflight_to_bounded_read_file_symlink_swap_is_rejected: creating a Windows file symlink failed ({error}); this host may lack Developer Mode or symlink privilege"
        );
        return;
    }
    fs::remove_file(&probe)
        .unwrap_or_else(|_| unreachable!("symlink capability probe can be removed"));
    let filesystem = SwapToFileSymlinkAfterInspection {
        target: target.clone(),
        swapped: Cell::new(false),
        read_attempted: Cell::new(false),
    };

    let error = open_source(&filesystem, &source)
        .err()
        .unwrap_or_else(|| unreachable!("bounded read must reject the swapped-in reparse point"));

    assert!(filesystem.swapped.get());
    assert!(filesystem.read_attempted.get());
    match error.kind {
        FdxStorageErrorKind::SymlinkDestination => eprintln!(
            "windows preflight/read swap outcome: native open succeeded on the reparse object; handle metadata rejected it"
        ),
        FdxStorageErrorKind::Io(StorageOperation::Read) => eprintln!(
            "windows preflight/read swap outcome: native open rejected the reparse object before handle metadata validation"
        ),
        other => unreachable!("unexpected preflight/read swap result: {other:?}"),
    }
    assert_eq!(valid(fs::read(&target)), b"swap target remains unchanged");
    assert!(valid(fs::symlink_metadata(&source))
        .file_type()
        .is_symlink());
}

#[cfg(windows)]
struct SwapToDirectoryJunctionAfterInspection {
    target: PathBuf,
    swapped: Cell<bool>,
    read_attempted: Cell<bool>,
}

#[cfg(windows)]
impl FdxStorageFileSystem for SwapToDirectoryJunctionAfterInspection {
    type TempFile = fs::File;
    type Permissions = fs::Permissions;

    fn read_bounded(&self, path: &Path, limit: usize) -> Result<Vec<u8>, FdxStorageError> {
        self.read_attempted.set(true);
        StdFdxStorageFileSystem.read_bounded(path, limit)
    }

    fn inspect_destination(
        &self,
        path: &Path,
    ) -> Result<Option<Self::Permissions>, FdxStorageError> {
        let permissions = StdFdxStorageFileSystem.inspect_destination(path)?;
        if !self.swapped.replace(true) {
            fs::remove_file(path).map_err(|_| FdxStorageError {
                kind: FdxStorageErrorKind::Io(StorageOperation::Inspect),
                temporary_cleanup_failed: false,
            })?;
            let result = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(path)
                .arg(&self.target)
                .output();
            if !matches!(result, Ok(output) if output.status.success()) {
                return Err(FdxStorageError {
                    kind: FdxStorageErrorKind::Io(StorageOperation::Inspect),
                    temporary_cleanup_failed: false,
                });
            }
        }
        Ok(permissions)
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

#[cfg(windows)]
#[test]
fn windows_preflight_to_bounded_read_directory_junction_swap_is_rejected() {
    use std::os::windows::fs::MetadataExt;
    use std::process::Command;

    let directory = TestDirectory::new();
    let target = directory.path("swap-junction-target");
    let source = directory.path("swap-junction-source.fdx");
    let probe = directory.path("junction-capability-probe");
    fs::create_dir(&target)
        .unwrap_or_else(|_| unreachable!("junction target directory can be created"));
    let target_file = target.join("contents.bin");
    fs::write(&target_file, b"junction target remains unchanged")
        .unwrap_or_else(|_| unreachable!("junction target content can be written"));
    fs::write(&source, b"regular file observed during preflight")
        .unwrap_or_else(|_| unreachable!("preflight source can be written"));
    let probe_result = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&probe)
        .arg(&target)
        .output();
    match probe_result {
        Ok(output) if output.status.success() => {
            fs::remove_dir(&probe)
                .unwrap_or_else(|_| unreachable!("junction capability probe can be removed"));
        }
        Ok(output) => {
            eprintln!(
                "SKIP windows_preflight_to_bounded_read_directory_junction_swap_is_rejected: cmd mklink /J capability probe failed with status {} ({})",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            );
            return;
        }
        Err(error) => {
            eprintln!(
                "SKIP windows_preflight_to_bounded_read_directory_junction_swap_is_rejected: cmd mklink /J could not run ({error})"
            );
            return;
        }
    }
    let filesystem = SwapToDirectoryJunctionAfterInspection {
        target,
        swapped: Cell::new(false),
        read_attempted: Cell::new(false),
    };

    let error = match open_source(&filesystem, &source) {
        Ok(_) => unreachable!("bounded read must reject the swapped-in directory junction"),
        Err(error) => error,
    };
    if error.kind == FdxStorageErrorKind::Io(StorageOperation::Inspect) {
        eprintln!(
            "SKIP windows_preflight_to_bounded_read_directory_junction_swap_is_rejected: junction could not be created at the source path during the deterministic swap"
        );
        return;
    }

    assert!(filesystem.swapped.get());
    assert!(filesystem.read_attempted.get());
    match error.kind {
        FdxStorageErrorKind::SymlinkDestination => eprintln!(
            "windows preflight/junction swap outcome: native open succeeded on the reparse object; handle metadata rejected it"
        ),
        FdxStorageErrorKind::Io(StorageOperation::Read) => eprintln!(
            "windows preflight/junction swap outcome: native open rejected the junction before handle metadata validation"
        ),
        other => unreachable!("unexpected preflight/junction swap result: {other:?}"),
    }
    assert_ne!(
        valid(fs::symlink_metadata(&source)).file_attributes()
            & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT,
        0,
        "the final component is the junction created after preflight"
    );
    assert_eq!(
        valid(fs::read(&target_file)),
        b"junction target remains unchanged"
    );
    fs::remove_dir(&source)
        .unwrap_or_else(|_| unreachable!("swapped junction can be removed after the test"));
}

#[cfg(windows)]
#[test]
fn windows_directory_junction_direct_bounded_read_is_rejected_when_available() {
    use std::process::Command;

    let directory = TestDirectory::new();
    let target = directory.path("junction-target");
    let junction = directory.path("directory-junction");
    fs::create_dir(&target)
        .unwrap_or_else(|_| unreachable!("junction target directory can be created"));
    let output = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&junction)
        .arg(&target)
        .output();
    match output {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            eprintln!(
                "SKIP windows_directory_junction_direct_bounded_read_is_rejected_when_available: cmd mklink /J failed with status {} ({})",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            );
            return;
        }
        Err(error) => {
            eprintln!(
                "SKIP windows_directory_junction_direct_bounded_read_is_rejected_when_available: cmd mklink /J could not run ({error})"
            );
            return;
        }
    };
    let bounded_error = StdFdxStorageFileSystem
        .read_bounded(&junction, scriptcraft_core::fdx::MAX_SOURCE_BYTES)
        .err()
        .unwrap_or_else(|| unreachable!("bounded read must reject a directory junction"));
    match bounded_error.kind {
        FdxStorageErrorKind::SymlinkDestination => eprintln!(
            "windows directory-junction direct-read outcome: native open succeeded on the reparse object; handle metadata rejected it"
        ),
        FdxStorageErrorKind::Io(StorageOperation::Read) => eprintln!(
            "windows directory-junction direct-read outcome: native open rejected the junction before handle metadata validation"
        ),
        other => unreachable!("unexpected junction direct bounded-read result: {other:?}"),
    }
    fs::remove_dir(&junction)
        .unwrap_or_else(|_| unreachable!("junction link can be removed after direct read"));
}

#[cfg(windows)]
#[test]
fn windows_read_only_target_fails_closed_without_changing_content_or_state() {
    let directory = TestDirectory::new();
    let path = directory.path("read-only.fdx");
    let initial = valid(write_v6(&valid(Project::new())));
    fs::write(&path, &initial).unwrap_or_else(|_| unreachable!("seed FDX can be written"));
    let opened = valid(open(&StdFdxStorageFileSystem, &path));
    let mut session = valid(EditingSession::new(opened.document.project));
    valid(session.insert("attempted write to read-only target"));
    let before = session.clone();
    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    let before_target = target.clone();
    let mut permissions = valid(fs::metadata(&path)).permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&path, permissions)
        .unwrap_or_else(|_| unreachable!("Windows read-only attribute can be set"));

    let error = save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::Save,
    )
    .err()
    .unwrap_or_else(|| unreachable!("Windows read-only destination must fail closed"));

    assert!(matches!(error, TransactionalSaveError::Storage(ref storage)
        if storage.kind == FdxStorageErrorKind::Io(StorageOperation::SetPermissions)));
    assert_eq!(valid(fs::read(&path)), initial);
    assert_eq!(session, before);
    assert!(session.dirty());
    assert_eq!(target, before_target);
    let mut writable = valid(fs::metadata(&path)).permissions();
    writable.set_readonly(false);
    fs::set_permissions(&path, writable)
        .unwrap_or_else(|_| unreachable!("read-only test fixture can be restored for cleanup"));
    assert_eq!(
        valid(fs::read_dir(&directory.0)).count(),
        1,
        "staged file is removed after read-only rejection"
    );
}

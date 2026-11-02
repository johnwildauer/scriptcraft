#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::EditingSession;
use scriptcraft_core::fdx::FdxError;
use scriptcraft_core::fdx_storage::{
    open, save_session, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem,
    TransactionalSaveError,
};
use scriptcraft_core::{Project, RunAttribute, TextRun};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const REFERENCE: &[u8] = include_bytes!("../../../.reference/reference2.fdx");
const EXPECTED_UNSUPPORTED_DIAGNOSTIC: &str =
    "23: FDX contains a construct that cannot be projected safely";
static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "scriptcraft-v4-wo003-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path)
            .unwrap_or_else(|_| unreachable!("temporary test directory is creatable"));
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

fn unsupported_generated_project() -> Project {
    let mut project = Project::new().unwrap_or_else(|_| unreachable!("default project is valid"));
    project.elements[0].text = "safe text".to_owned();
    project.elements[0].runs = vec![TextRun {
        text: "safe text".to_owned(),
        attributes: vec![RunAttribute {
            name: "vendor:style".to_owned(),
            value: "private attribute value".to_owned(),
        }],
    }];
    project
}

fn assert_numeric_codec_failure(error: TransactionalSaveError) {
    assert!(matches!(
        &error,
        TransactionalSaveError::Codec(FdxError::SaveDiagnosticUnsupportedConstruct(23))
    ));
    let detail = error.to_string();
    assert!(detail.ends_with(EXPECTED_UNSUPPORTED_DIAGNOSTIC));
    assert!(detail.contains("23: FDX contains"));
    assert!(!detail.contains("private attribute value"));
}

#[test]
fn generated_writer_diagnostic_is_numbered_for_save_and_save_as_without_publication() {
    let directory = TestDirectory::new();
    let source_path = directory.path("source.fdx");
    fs::write(&source_path, REFERENCE)
        .unwrap_or_else(|_| unreachable!("disposable source fixture can be written"));
    let fs_backend = StdFdxStorageFileSystem;
    let opened = open(&fs_backend, &source_path)
        .unwrap_or_else(|_| unreachable!("checked-in FDX reference opens"));
    let source_before = fs::read(&source_path)
        .unwrap_or_else(|_| unreachable!("source fixture remains readable"));

    let mut session_for_save = EditingSession::new(unsupported_generated_project())
        .unwrap_or_else(|_| unreachable!("diagnostic project initializes"));
    let save_session_before = session_for_save.clone();
    let mut save_target = FdxSaveTarget::from_baseline(opened.baseline.clone());
    let save_target_before = save_target.clone();
    let save_error = match save_session(
        &fs_backend,
        &mut session_for_save,
        &mut save_target,
        FdxSaveMode::Save,
    ) {
        Err(error) => error,
        Ok(_) => unreachable!("unsupported generated run attribute blocks Save"),
    };
    assert_numeric_codec_failure(save_error);
    assert_eq!(session_for_save, save_session_before);
    assert_eq!(save_target, save_target_before);
    assert_eq!(
        fs::read(&source_path).unwrap_or_else(|_| unreachable!("source remains readable")),
        source_before
    );

    let destination = directory.path("copy.fdx");
    let mut session_for_save_as = EditingSession::new(unsupported_generated_project())
        .unwrap_or_else(|_| unreachable!("diagnostic project initializes"));
    let save_as_session_before = session_for_save_as.clone();
    let mut save_as_target = FdxSaveTarget::new_unsaved();
    let save_as_target_before = save_as_target.clone();
    let save_as_error = match save_session(
        &fs_backend,
        &mut session_for_save_as,
        &mut save_as_target,
        FdxSaveMode::SaveAs(destination.clone()),
    ) {
        Err(error) => error,
        Ok(_) => unreachable!("unsupported generated run attribute blocks Save As"),
    };
    assert_numeric_codec_failure(save_as_error);
    assert_eq!(session_for_save_as, save_as_session_before);
    assert_eq!(save_as_target, save_as_target_before);
    assert!(!destination.exists());
    assert_eq!(
        fs::read(&source_path).unwrap_or_else(|_| unreachable!("source remains readable")),
        source_before
    );
}

#[test]
fn reopened_reference_save_save_as_and_second_save_preserve_transaction_controls() {
    let directory = TestDirectory::new();
    let source_path = directory.path("reopened-source.fdx");
    let copy_path = directory.path("reopened-copy.fdx");
    fs::write(&source_path, REFERENCE)
        .unwrap_or_else(|_| unreachable!("disposable source fixture can be written"));
    let fs_backend = StdFdxStorageFileSystem;

    let opened = open(&fs_backend, &source_path)
        .unwrap_or_else(|_| unreachable!("checked-in FDX reference opens"));
    let mut session_for_reopened_save = EditingSession::new(opened.document.project)
        .unwrap_or_else(|_| unreachable!("reopened reference initializes a session"));
    let mut save_target = FdxSaveTarget::from_baseline(opened.baseline);
    save_session(
        &fs_backend,
        &mut session_for_reopened_save,
        &mut save_target,
        FdxSaveMode::Save,
    )
    .unwrap_or_else(|_| unreachable!("unchanged reopened reference Save succeeds"));

    let opened_for_save_as = open(&fs_backend, &source_path)
        .unwrap_or_else(|_| unreachable!("original source remains openable"));
    let mut session_for_reopened_save_as = EditingSession::new(opened_for_save_as.document.project)
        .unwrap_or_else(|_| unreachable!("reopened reference initializes for Save As"));
    let mut save_as_target = FdxSaveTarget::new_unsaved();
    save_session(
        &fs_backend,
        &mut session_for_reopened_save_as,
        &mut save_as_target,
        FdxSaveMode::SaveAs(copy_path.clone()),
    )
    .unwrap_or_else(|_| unreachable!("unchanged reopened reference Save As succeeds"));

    let reopened_copy = open(&fs_backend, &copy_path)
        .unwrap_or_else(|_| unreachable!("Save As output reopens"));
    let mut session_for_second_save = EditingSession::new(reopened_copy.document.project)
        .unwrap_or_else(|_| unreachable!("Save As output initializes a session"));
    let mut second_save_target = FdxSaveTarget::from_baseline(reopened_copy.baseline);
    save_session(
        &fs_backend,
        &mut session_for_second_save,
        &mut second_save_target,
        FdxSaveMode::Save,
    )
    .unwrap_or_else(|_| unreachable!("reopened Save As output saves a second time"));
}

#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::EditingSession;
use scriptcraft_core::fdx::{self, FdxError};
use scriptcraft_core::fdx_storage::{
    open, save_session, FdxSaveMode, FdxSaveTarget, FdxStorageErrorKind,
    StdFdxStorageFileSystem, TransactionalSaveError,
};
use scriptcraft_core::preservation::PreservedNode;
use scriptcraft_core::Project;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const DECLARATION: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\" ?>";
const REFERENCE2: &[u8] = include_bytes!("../../../.reference/reference2.fdx");
const ARC_COPY: &[u8] = include_bytes!("../../../.reference/arc copy.fdx");
const MINIMAL_ROOT: &str = r#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="p1"><Text>Action</Text></Paragraph></Content></FinalDraft>"#;
static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "scriptcraft-v5-wo002-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path)
            .unwrap_or_else(|_| unreachable!("temporary declaration test directory is creatable"));
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

fn fixture(declaration: bool, separator: &str, misc: &str) -> Vec<u8> {
    let mut source = String::new();
    if declaration {
        source.push_str(DECLARATION);
    }
    source.push_str(separator);
    source.push_str(misc);
    source.push_str(MINIMAL_ROOT);
    source.into_bytes()
}

fn declaration_count(bytes: &[u8]) -> usize {
    bytes
        .windows(DECLARATION.len())
        .filter(|window| *window == DECLARATION.as_bytes())
        .count()
}

fn assert_declaration_prefix(bytes: &[u8], retained_prolog: &[u8]) {
    let mut expected = DECLARATION.as_bytes().to_vec();
    expected.extend_from_slice(retained_prolog);
    assert!(bytes.starts_with(&expected));
    assert_eq!(declaration_count(bytes), 1);
}

fn retained_reference_separator(source: &[u8]) -> Vec<u8> {
    let declaration_end = source
        .windows(2)
        .position(|window| window == b"?>")
        .map(|index| index + 2)
        .unwrap_or_else(|| unreachable!("checked-in reference has an XML declaration"));
    let root_start = source
        .windows(b"<FinalDraft".len())
        .position(|window| window == b"<FinalDraft")
        .unwrap_or_else(|| unreachable!("checked-in reference has a FinalDraft root"));
    source[declaration_end..root_start].to_vec()
}

fn inject_source_type_mismatch(project: &mut Project) -> bool {
    fn visit(nodes: &mut [PreservedNode]) -> bool {
        for node in nodes {
            if let PreservedNode::Element(element) = node {
                if element.source_type.is_some() {
                    element.source_type = Some("independent-code30-control".to_owned());
                    return true;
                }
                if visit(&mut element.children) {
                    return true;
                }
            }
        }
        false
    }

    project
        .preservation
        .as_mut()
        .map(|state| state.sections.iter_mut().any(|section| visit(&mut section.children)))
        .unwrap_or(false)
}

#[test]
fn direct_codec_retains_declaration_separator_variants_and_repeated_save_is_fixed_point() {
    let variants = [
        (true, "\n", ""),
        (true, "\n\n\n", ""),
        (true, " \t\t", ""),
        (true, "", ""),
        (false, "\n", ""),
        (false, "", ""),
        (true, " \n", "<!--top-level--><?vendor retained?>\t"),
    ];

    for (has_declaration, separator, misc) in variants {
        let source = fixture(has_declaration, separator, misc);
        let expected_prolog = format!("{separator}{misc}").into_bytes();
        let imported = fdx::read_v6(&source)
            .unwrap_or_else(|_| unreachable!("minimal valid managed Content fixture imports"));
        let first = fdx::write_v6(&imported.project)
            .unwrap_or_else(|_| unreachable!("source-backed whitespace variant serializes strictly"));
        assert_declaration_prefix(&first, &expected_prolog);

        let reopened = fdx::read_v6(&first)
            .unwrap_or_else(|_| unreachable!("first source-backed output reopens"));
        let second = fdx::write_v6(&reopened.project)
            .unwrap_or_else(|_| unreachable!("reopened source-backed output serializes strictly"));
        assert_declaration_prefix(&second, &expected_prolog);
    }
}

#[test]
fn preservation_free_generation_keeps_declaration_plus_one_lf() {
    let project = Project::new().unwrap_or_else(|_| unreachable!("default project is valid"));
    let bytes = fdx::write_v6(&project)
        .unwrap_or_else(|_| unreachable!("new preservation-free project serializes"));
    let mut expected = DECLARATION.as_bytes().to_vec();
    expected.push(b'\n');
    assert!(bytes.starts_with(&expected));
    assert_eq!(declaration_count(&bytes), 1);
}

#[test]
fn independent_preserved_source_mismatch_still_fails_with_code30() {
    let source = fixture(true, "\n", "");
    let mut project = fdx::read_v6(&source)
        .unwrap_or_else(|_| unreachable!("minimal valid managed Content fixture imports"))
        .project;
    assert!(inject_source_type_mismatch(&mut project));

    let error = fdx::write_v6(&project)
        .err()
        .unwrap_or_else(|| unreachable!("strict source-tree mismatch remains rejected"));
    assert!(matches!(error, FdxError::SaveDiagnosticSourceTreeMismatch(_)));
    assert!(error.to_string().starts_with("30: FDX contains a construct"));
}

fn exercise_reference_filesystem_cycle(source: &[u8], stem: &str) {
    let directory = TestDirectory::new();
    let source_path = directory.path(&format!("{stem}-source.fdx"));
    let copy_path = directory.path(&format!("{stem}-copy.fdx"));
    let separator = retained_reference_separator(source);
    fs::write(&source_path, source)
        .unwrap_or_else(|_| unreachable!("disposable checked-in reference can be written"));
    let backend = StdFdxStorageFileSystem;

    let opened = open(&backend, &source_path)
        .unwrap_or_else(|_| unreachable!("checked-in Final Draft reference opens"));
    let mut session = EditingSession::new(opened.document.project)
        .unwrap_or_else(|_| unreachable!("opened reference initializes a session"));
    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    save_session(&backend, &mut session, &mut target, FdxSaveMode::Save)
        .unwrap_or_else(|_| unreachable!("unchanged reference Save succeeds"));
    let first_save = fs::read(&source_path)
        .unwrap_or_else(|_| unreachable!("saved reference remains readable"));
    assert_declaration_prefix(&first_save, &separator);

    let reopened = open(&backend, &source_path)
        .unwrap_or_else(|_| unreachable!("first Save output reopens"));
    let mut second_session = EditingSession::new(reopened.document.project)
        .unwrap_or_else(|_| unreachable!("reopened first Save initializes a session"));
    let mut second_target = FdxSaveTarget::from_baseline(reopened.baseline);
    save_session(&backend, &mut second_session, &mut second_target, FdxSaveMode::Save)
        .unwrap_or_else(|_| unreachable!("repeated reference Save succeeds"));
    let second_save = fs::read(&source_path)
        .unwrap_or_else(|_| unreachable!("repeatedly saved reference remains readable"));
    assert_declaration_prefix(&second_save, &separator);

    let source_before_save_as = second_save;
    let opened_for_save_as = open(&backend, &source_path)
        .unwrap_or_else(|_| unreachable!("reference opens before Save As"));
    let mut save_as_session = EditingSession::new(opened_for_save_as.document.project)
        .unwrap_or_else(|_| unreachable!("reference initializes for Save As"));
    let mut save_as_target = FdxSaveTarget::new_unsaved();
    save_session(
        &backend,
        &mut save_as_session,
        &mut save_as_target,
        FdxSaveMode::SaveAs(copy_path.clone()),
    )
    .unwrap_or_else(|_| unreachable!("unchanged reference Save As succeeds"));
    assert_eq!(
        fs::read(&source_path).unwrap_or_else(|_| unreachable!("Save As source remains readable")),
        source_before_save_as
    );
    let save_as_bytes = fs::read(&copy_path)
        .unwrap_or_else(|_| unreachable!("Save As destination is published"));
    assert_declaration_prefix(&save_as_bytes, &separator);

    let reopened_copy = open(&backend, &copy_path)
        .unwrap_or_else(|_| unreachable!("Save As output reopens"));
    let mut repeated_save_as_session = EditingSession::new(reopened_copy.document.project)
        .unwrap_or_else(|_| unreachable!("Save As output initializes for a repeated Save"));
    let mut repeated_save_as_target = FdxSaveTarget::from_baseline(reopened_copy.baseline);
    save_session(
        &backend,
        &mut repeated_save_as_session,
        &mut repeated_save_as_target,
        FdxSaveMode::Save,
    )
    .unwrap_or_else(|_| unreachable!("reopened Save As output saves again"));
    let repeated_save_as = fs::read(&copy_path)
        .unwrap_or_else(|_| unreachable!("repeated Save As output remains readable"));
    assert_declaration_prefix(&repeated_save_as, &separator);
}

#[test]
fn reference2_and_arc_copy_save_save_as_reopen_and_repeat_without_separator_growth() {
    exercise_reference_filesystem_cycle(REFERENCE2, "reference2");
    exercise_reference_filesystem_cycle(ARC_COPY, "arc-copy");
}

#[test]
fn external_conflict_keeps_destination_live_session_and_target_unchanged() {
    let directory = TestDirectory::new();
    let source_path = directory.path("conflict-source.fdx");
    fs::write(&source_path, REFERENCE2)
        .unwrap_or_else(|_| unreachable!("disposable reference can be written"));
    let backend = StdFdxStorageFileSystem;
    let opened = open(&backend, &source_path)
        .unwrap_or_else(|_| unreachable!("checked-in reference opens"));
    let mut session = EditingSession::new(opened.document.project)
        .unwrap_or_else(|_| unreachable!("opened reference initializes a session"));
    let session_before = session.clone();
    let mut target = FdxSaveTarget::from_baseline(opened.baseline);
    let target_before = target.clone();
    let external_bytes = [REFERENCE2, b"\n"].concat();
    fs::write(&source_path, &external_bytes)
        .unwrap_or_else(|_| unreachable!("external conflict control updates the source"));

    let error = save_session(&backend, &mut session, &mut target, FdxSaveMode::Save)
        .err()
        .unwrap_or_else(|| unreachable!("external source conflict blocks publication"));
    assert!(matches!(
        &error,
        TransactionalSaveError::Storage(storage)
            if storage.kind == FdxStorageErrorKind::Conflict
    ));
    assert_eq!(session, session_before);
    assert_eq!(target, target_before);
    assert_eq!(
        fs::read(&source_path).unwrap_or_else(|_| unreachable!("external source remains readable")),
        external_bytes
    );
}

#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::exports::{export_pdf, export_text};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::fdx_storage::{
    save_session, FdxSaveMode, FdxSaveTarget, FdxStorageErrorKind, StdFdxStorageFileSystem,
    TransactionalSaveError,
};
use scriptcraft_core::layout::ScreenplayLayout;
use scriptcraft_core::{Project, Role};
use std::fs;
use std::path::PathBuf;

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
    assert!(target.path().is_none());
    assert!(target.baseline().is_none());
}

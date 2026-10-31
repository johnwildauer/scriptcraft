#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::exports::{export_pdf, export_text};
use scriptcraft_core::persistence::{
    atomic_save, deserialize, discover_recovery, save_recovery, FileSystem, PersistenceError,
};
use scriptcraft_core::{Project, Role};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct MemFs {
    files: Arc<Mutex<BTreeMap<PathBuf, Vec<u8>>>>,
    fail_rename: bool,
}

impl FileSystem for MemFs {
    fn read(&self, path: &Path) -> Result<Vec<u8>, PersistenceError> {
        self.files
            .lock()
            .map_err(|_| PersistenceError::Io("lock poisoned".into()))?
            .get(path)
            .cloned()
            .ok_or_else(|| PersistenceError::Io("missing file".into()))
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), PersistenceError> {
        self.files
            .lock()
            .map_err(|_| PersistenceError::Io("lock poisoned".into()))?
            .insert(path.to_owned(), bytes.to_vec());
        Ok(())
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), PersistenceError> {
        if self.fail_rename {
            return Err(PersistenceError::Io("injected rename failure".into()));
        }
        let mut files = self
            .files
            .lock()
            .map_err(|_| PersistenceError::Io("lock poisoned".into()))?;
        let bytes = files
            .remove(from)
            .ok_or_else(|| PersistenceError::Io("missing temporary file".into()))?;
        files.insert(to.to_owned(), bytes);
        Ok(())
    }

    fn remove(&self, path: &Path) -> Result<(), PersistenceError> {
        self.files
            .lock()
            .map_err(|_| PersistenceError::Io("lock poisoned".into()))?
            .remove(path);
        Ok(())
    }

    fn list(&self, _directory: &Path) -> Result<Vec<PathBuf>, PersistenceError> {
        Ok(self
            .files
            .lock()
            .map_err(|_| PersistenceError::Io("lock poisoned".into()))?
            .keys()
            .cloned()
            .collect())
    }
}

fn project() -> Project {
    Project::from_json(include_str!("fixtures/legacy_project.json"))
        .unwrap_or_else(|_| unreachable!("fixed fixture must be valid"))
}

#[test]
fn representative_headless_writing_loop_survives_every_boundary() {
    let mut session = EditingSession::new(project())
        .unwrap_or_else(|_| unreachable!("fixed fixture must be valid"));

    // Create and edit a scene heading, then use the explicit scene-heading → action transition.
    session
        .change_role(Role::SceneHeading)
        .unwrap_or_else(|_| unreachable!("role change must be valid"));
    session
        .insert("INT. CAFÉ — NIGHT")
        .unwrap_or_else(|_| unreachable!("fixture insert must be valid"));
    session
        .enter()
        .unwrap_or_else(|_| unreachable!("scene transition must be valid"));
    assert_eq!(session.project().elements[1].role, Role::Action);

    // Exercise character → dialogue and a Unicode edit through the same session boundary.
    session
        .change_role(Role::Character)
        .unwrap_or_else(|_| unreachable!("role change must be valid"));
    session
        .insert("MARA")
        .unwrap_or_else(|_| unreachable!("fixture insert must be valid"));
    session
        .enter()
        .unwrap_or_else(|_| unreachable!("character transition must be valid"));
    assert_eq!(session.project().elements[2].role, Role::Dialogue);
    session
        .insert("こんにちは、世界 🌙")
        .unwrap_or_else(|_| unreachable!("Unicode insert must be valid"));
    assert!(session.dirty());

    let edited = session.project().clone();
    session
        .undo()
        .unwrap_or_else(|_| unreachable!("undo must restore Unicode edit"));
    assert_ne!(session.project(), &edited);
    session
        .redo()
        .unwrap_or_else(|_| unreachable!("redo must restore Unicode edit"));
    assert_eq!(session.project(), &edited);

    // Pagination is structural and repeatable, independent of any desktop window.
    let first_layout = scriptcraft_core::layout::ScreenplayLayout::build(session.project())
        .unwrap_or_else(|_| unreachable!("valid project must lay out"));
    let second_layout = scriptcraft_core::layout::ScreenplayLayout::build(session.project())
        .unwrap_or_else(|_| unreachable!("valid project must lay out"));
    assert_eq!(first_layout, second_layout);
    assert!(!first_layout.pages.is_empty());

    let fs = MemFs::default();
    let destination = Path::new("/native/scriptcraft.json");
    atomic_save(&fs, destination, session.project())
        .unwrap_or_else(|_| unreachable!("native save must succeed"));
    let reopened = deserialize(&fs.read(destination).unwrap_or_default())
        .unwrap_or_else(|_| unreachable!("native reopen must succeed"));
    assert_eq!(reopened, *session.project());

    save_recovery(&fs, Path::new("/native"), session.project(), 99)
        .unwrap_or_else(|_| unreachable!("recovery save must succeed"));
    let candidates = discover_recovery(&fs, Path::new("/native"), 0)
        .unwrap_or_else(|_| unreachable!("recovery discovery must succeed"));
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].project, *session.project());

    // Exports consume the project but must not falsely clean or otherwise mutate the session.
    let before_exports = session.clone();
    let pdf = export_pdf(session.project()).unwrap_or_else(|_| unreachable!("PDF must export"));
    let text = export_text(session.project()).unwrap_or_else(|_| unreachable!("TXT must export"));
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert!(text.text.contains("LOSSY EXPORT"));
    assert_eq!(session, before_exports);
    assert!(session.dirty(), "exports must not clear dirty state");
}

#[test]
fn bounded_hostile_sequences_return_errors_without_state_corruption() {
    let mut session = EditingSession::new(project())
        .unwrap_or_else(|_| unreachable!("fixed fixture must be valid"));
    session
        .change_role(Role::Dialogue)
        .unwrap_or_else(|_| unreachable!("role change must be valid"));
    session
        .insert("é🙂")
        .unwrap_or_else(|_| unreachable!("fixture insert must be valid"));

    for step in 0..64 {
        let before = session.clone();
        let result = match step % 4 {
            0 => session.set_selection(Selection::caret(Position {
                element: 0,
                offset: 1,
            })),
            1 => session.set_selection(Selection::caret(Position {
                element: usize::MAX,
                offset: 0,
            })),
            2 => session.insert(&"x".repeat(scriptcraft_core::MAX_TEXT_BYTES + 1)),
            _ => {
                assert!(deserialize(b"{not-json").is_err());
                continue;
            }
        };
        assert!(result.is_err());
        assert_eq!(session, before);
    }
    assert_eq!(session.project().elements[0].text, "é🙂");
}

#[test]
fn failed_native_replacement_preserves_saved_bytes_and_dirty_state() {
    let fs = MemFs::default();
    let destination = Path::new("/native/scriptcraft.json");
    let old = project();
    atomic_save(&fs, destination, &old)
        .unwrap_or_else(|_| unreachable!("initial native save must succeed"));
    let failing = MemFs {
        files: fs.files.clone(),
        fail_rename: true,
    };
    let mut session = EditingSession::new(old).unwrap_or_else(|_| unreachable!());
    session
        .insert("unsaved")
        .unwrap_or_else(|_| unreachable!("fixture edit must succeed"));
    let before = session.clone();
    assert!(atomic_save(&failing, destination, session.project()).is_err());
    assert_eq!(session, before);
    assert!(session.dirty());
    assert_eq!(
        deserialize(&fs.read(destination).unwrap_or_default()),
        Ok(project())
    );
}

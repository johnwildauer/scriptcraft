#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_core::editing::{EditError, EditingSession, Position, Selection};
use scriptcraft_core::layout::{Caret, LayoutErrorKind, ScreenplayLayout};
use scriptcraft_core::persistence::{atomic_save, deserialize, FileSystem, PersistenceError};
use scriptcraft_core::{LayoutInputs, ModelError, Project, Role, MAX_TEXT_BYTES};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct MemFs {
    files: Arc<Mutex<BTreeMap<PathBuf, Vec<u8>>>>,
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
        Ok(Vec::new())
    }
}

fn project() -> Project {
    let mut project = Project::from_json(include_str!("fixtures/legacy_project.json"))
        .unwrap_or_else(|_| unreachable!("fixed fixture must be valid"));
    project.elements[0].role = Role::Action;
    project
}

fn layout_for(text: &str, lines_per_page: u32) -> ScreenplayLayout {
    let inputs = LayoutInputs {
        lines_per_page,
        ..LayoutInputs::default()
    };
    let mut project = project();
    project.elements[0].role = Role::Action;
    project.elements[0].text = text.into();
    ScreenplayLayout::build_with_inputs(&project, &inputs)
        .unwrap_or_else(|_| unreachable!("fixture must lay out"))
}

#[test]
fn controlled_fixture_dimensions_are_independent_and_bounded() {
    let by_elements = {
        let mut project = project();
        project.elements.clear();
        for index in 0..8 {
            project.elements.push(
                scriptcraft_core::ScreenplayElement::new(
                    format!("element-{index}"),
                    Role::Action,
                    "short",
                )
                .unwrap_or_else(|_| unreachable!("fixed fixture must be valid")),
            );
        }
        ScreenplayLayout::build(&project).unwrap_or_else(|_| unreachable!())
    };
    assert_eq!(by_elements.pages.len(), 1);
    assert_eq!(by_elements.pages[0].lines.len(), 8);

    let by_text_size = layout_for(&"word ".repeat(300), 10);
    assert!(by_text_size.pages.len() > 1);

    let by_wrapped_lines = layout_for(&"x".repeat(55 * 6), 4);
    assert!(by_wrapped_lines.pages.len() > 1);

    let by_pages = layout_for(&"line\n".repeat(20), 2);
    assert!(by_pages.pages.len() >= 10);
    assert!(by_pages.pages.iter().all(|page| !page.lines.is_empty()));
}

#[test]
fn pasted_newline_patterns_preserve_semantics_without_phantom_cr_lines() {
    let lf = layout_for("one\ntwo\nthree", 55);
    let crlf = layout_for("one\r\ntwo\r\nthree", 55);
    let cr = layout_for("one\rtwo\rthree", 55);

    assert_eq!(lf.pages[0].lines.len(), 3);
    assert_eq!(crlf.pages[0].lines.len(), 3);
    assert_eq!(cr.pages[0].lines.len(), 3);
    assert_eq!(
        crlf.pages[0]
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>(),
        vec!["one", "two", "three"]
    );
}

#[test]
fn final_page_caret_and_native_round_trip_survive_long_paste() {
    let text = (0..500)
        .map(|line| format!("Action line {line} with enough words to wrap safely."))
        .collect::<Vec<_>>()
        .join("\r\n");
    assert!(text.len() < MAX_TEXT_BYTES);

    let mut session = EditingSession::new(project())
        .unwrap_or_else(|_| unreachable!("fixed fixture must be valid"));
    session
        .insert(&text)
        .unwrap_or_else(|_| unreachable!("bounded paste must succeed"));
    let end = Position {
        element: 0,
        offset: text.len(),
    };
    session
        .set_selection(Selection::caret(end))
        .unwrap_or_else(|_| unreachable!("final caret must be valid"));

    let layout = ScreenplayLayout::build(session.project())
        .unwrap_or_else(|_| unreachable!("long bounded project must lay out"));
    assert!(layout.pages.len() > 1);
    assert!(layout
        .caret_rect(Caret {
            element: end.element,
            offset: end.offset,
        })
        .is_ok());

    let fs = MemFs::default();
    let path = Path::new("/long-document.json");
    atomic_save(&fs, path, session.project())
        .unwrap_or_else(|_| unreachable!("bounded project must save"));
    let reopened = deserialize(&fs.read(path).unwrap_or_default())
        .unwrap_or_else(|_| unreachable!("saved project must reopen"));
    assert_eq!(reopened, *session.project());
    assert_eq!(reopened.elements[0].text, text);
}

#[test]
fn failed_edit_and_layout_leave_last_valid_state_usable() {
    let mut session = EditingSession::new(project())
        .unwrap_or_else(|_| unreachable!("fixed fixture must be valid"));
    session
        .insert("valid")
        .unwrap_or_else(|_| unreachable!("fixture edit must succeed"));
    let before = session.clone();
    assert_eq!(
        session.insert(&"x".repeat(MAX_TEXT_BYTES)),
        Err(EditError::TextTooLong)
    );
    assert_eq!(session, before);
    session
        .insert(" still usable")
        .unwrap_or_else(|_| unreachable!("last valid session must remain editable"));

    let invalid = LayoutInputs {
        chars_per_line: 0,
        ..LayoutInputs::default()
    };
    assert_eq!(
        ScreenplayLayout::build_with_inputs(session.project(), &invalid),
        Err(scriptcraft_core::layout::LayoutError {
            kind: LayoutErrorKind::InvalidProfile,
        })
    );
    assert_eq!(session.project().elements[0].text, "valid still usable");
}

#[test]
fn model_serialization_keeps_json_bound_even_when_text_is_valid() {
    let mut project = project();
    project.elements = (0..5)
        .map(|index| {
            scriptcraft_core::ScreenplayElement::new(
                format!("oversize-{index}"),
                Role::Action,
                "x".repeat(MAX_TEXT_BYTES - 1),
            )
            .unwrap_or_else(|_| unreachable!())
        })
        .collect();
    assert_eq!(project.to_json(), Err(ModelError::StringTooLong));
}

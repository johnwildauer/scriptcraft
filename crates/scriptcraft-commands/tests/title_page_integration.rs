#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_commands::{
    execute_editor_command, Command, CommandOutcome, EditorSurface, ParagraphAlignmentValue,
    Position, Selection, TitleTextStyle,
};
use scriptcraft_core::editing::{EditingSession, Position as CorePosition, Selection as CoreSelection};
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::fdx_storage::{
    save_session, FdxSaveMode, FdxSaveTarget, StdFdxStorageFileSystem,
};
use scriptcraft_core::preservation::{PreservedNode, SourceOwnership};
use scriptcraft_core::{Project, Role};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("title-page command scenario uses valid bounded input"))
}

fn command_position(element: usize, offset: usize) -> Position {
    Position {
        element: element as u32,
        offset: offset as u32,
    }
}

fn set_selection(
    session: &mut EditingSession,
    surface: EditorSurface,
    anchor: CorePosition,
    focus: CorePosition,
) -> CommandOutcome {
    execute_editor_command(
        session,
        &Command::EditorSelectionSet {
            surface,
            selection: Selection {
                anchor: command_position(anchor.element, anchor.offset),
                focus: command_position(focus.element, focus.offset),
            },
        },
    )
}

fn unique_directory() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!("scriptcraft-title-page-e2e-{}-{nanos}", std::process::id()))
}

#[test]
fn default_executor_mixed_surface_history_save_reopen_and_subsequent_edits() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    assert_eq!(session.project().title_page.len(), 54);
    assert_eq!(session.project().elements[0].role, Role::SceneHeading);
    assert!(session.project().elements[0].text.is_empty());

    assert_eq!(
        set_selection(
            &mut session,
            EditorSurface::TitlePage,
            CorePosition { element: 17, offset: 0 },
            CorePosition { element: 17, offset: 0 },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorTextInsert {
                surface: EditorSurface::TitlePage,
                text: "Draft: ".to_owned(),
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorTextInsert {
                surface: EditorSurface::Body,
                text: "INT. SHARED ROOM - DAY".to_owned(),
            },
        ),
        CommandOutcome::Changed
    );

    let title_text = session.project().title_page[17].text.clone();
    assert_eq!(
        set_selection(
            &mut session,
            EditorSurface::TitlePage,
            CorePosition { element: 17, offset: 0 },
            CorePosition { element: 17, offset: "Draft: ".len() },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorStyleToggle {
                surface: EditorSurface::TitlePage,
                style: TitleTextStyle::Bold,
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorAlignmentSet {
                surface: EditorSurface::TitlePage,
                alignment: ParagraphAlignmentValue::Right,
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        set_selection(
            &mut session,
            EditorSurface::Body,
            CorePosition { element: 0, offset: 5 },
            CorePosition { element: 0, offset: 11 },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorStyleToggle {
                surface: EditorSurface::Body,
                style: TitleTextStyle::Strikethrough,
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorAlignmentSet {
                surface: EditorSurface::Body,
                alignment: ParagraphAlignmentValue::Justified,
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(session.project().title_page[17].text, title_text);
    assert_eq!(session.project().elements[0].role, Role::SceneHeading);

    let edited = session.project().clone();
    for _ in 0..6 {
        assert_eq!(
            execute_editor_command(&mut session, &Command::EditUndo),
            CommandOutcome::Changed
        );
    }
    assert_eq!(session.project().title_page[17].text, "Script Title");
    assert_eq!(session.project().elements[0].text, "");
    for _ in 0..6 {
        assert_eq!(
            execute_editor_command(&mut session, &Command::EditRedo),
            CommandOutcome::Changed
        );
    }
    assert_eq!(session.project(), &edited);

    let directory = unique_directory();
    std::fs::create_dir(&directory)
        .unwrap_or_else(|_| unreachable!("unique integration directory can be created"));
    let path = directory.join("shared-session.fdx");
    let revision_before_save = session.revision();
    let title_selection_before_save = session.title_selection();
    let body_selection_before_save = session.selection();
    let mut target = FdxSaveTarget::new_unsaved();
    valid(save_session(
        &StdFdxStorageFileSystem,
        &mut session,
        &mut target,
        FdxSaveMode::SaveAs(path.clone()),
    ));
    assert_eq!(target.path(), Some(path.as_path()));
    assert!(target.baseline().is_some());
    assert!(!session.dirty());
    assert!(session.revision() > revision_before_save);
    assert_eq!(session.title_selection(), title_selection_before_save);
    assert_eq!(session.selection(), body_selection_before_save);

    let bytes = std::fs::read(&path)
        .unwrap_or_else(|_| unreachable!("successful native save created the destination"));
    let reopened = valid(read_v6(&bytes));
    assert_eq!(reopened.project.title_page[17].text, title_text);
    assert_eq!(reopened.project.elements[0].text, "INT. SHARED ROOM - DAY");
    assert_eq!(reopened.project.title_page[17].formatting.alignment, Some(
        scriptcraft_core::TitlePageAlignment::Right
    ));
    assert!(reopened.project.title_page[17].runs.iter().any(|run| {
        run.attributes.iter().any(|attribute| {
            attribute.name == "Style" && attribute.value.split('+').any(|token| token == "Bold")
        })
    }));
    assert!(reopened.project.elements[0].runs.iter().any(|run| {
        run.attributes.iter().any(|attribute| {
            attribute.name == "Style"
                && attribute.value.split('+').any(|token| token == "Strikethrough")
        })
    }));

    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorTextInsert {
                surface: EditorSurface::TitlePage,
                text: "live".to_owned(),
            },
        ),
        CommandOutcome::Changed
    );
    assert!(session.dirty());
    assert!(session.project().title_page[17].text.contains("live"));
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn reference3_executor_formatting_round_trip_preserves_source_runs_and_direct_overrides() {
    let imported = valid(read_v6(include_bytes!("../../../.reference/reference3.fdx")));
    let mut session = valid(EditingSession::new(imported.project));
    assert_eq!(session.project().elements[1].role, Role::Action);
    assert_eq!(session.project().elements[1].runs.len(), 7);
    let title_index = session
        .project()
        .title_page
        .iter()
        .position(|paragraph| paragraph.text == "Name of First Writer")
        .unwrap_or_else(|| unreachable!("reference3 includes its six-run writer placeholder"));
    assert_eq!(session.project().title_page[title_index].runs.len(), 6);

    assert_eq!(
        set_selection(
            &mut session,
            EditorSurface::Body,
            CorePosition { element: 1, offset: 12 },
            CorePosition { element: 1, offset: 16 },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorStyleToggle {
                surface: EditorSurface::Body,
                style: TitleTextStyle::Strikethrough,
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorAlignmentSet {
                surface: EditorSurface::Body,
                alignment: ParagraphAlignmentValue::Justified,
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        set_selection(
            &mut session,
            EditorSurface::TitlePage,
            CorePosition {
                element: title_index,
                offset: "Name of ".len(),
            },
            CorePosition {
                element: title_index,
                offset: "Name of First".len(),
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorStyleToggle {
                surface: EditorSurface::TitlePage,
                style: TitleTextStyle::Bold,
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorAlignmentSet {
                surface: EditorSurface::TitlePage,
                alignment: ParagraphAlignmentValue::Right,
            },
        ),
        CommandOutcome::Changed
    );

    let saved = valid(write_v6(session.project()));
    let output = String::from_utf8(saved.clone())
        .unwrap_or_else(|_| unreachable!("native FDX output uses UTF-8"));
    assert!(output.contains("<Watermarking"));
    assert!(output.contains("<DynamicLabel"));
    assert!(output.contains("Alignment=\"Full\""));
    assert!(output.contains("Alignment=\"Right\""));
    let reread = valid(read_v6(&saved));
    assert_eq!(reread.project.elements[1].text, session.project().elements[1].text);
    assert_eq!(reread.project.elements[1].runs, session.project().elements[1].runs);
    assert_eq!(
        reread.project.title_page[title_index].runs,
        session.project().title_page[title_index].runs
    );
    assert_eq!(
        reread.project.title_page[title_index].formatting.alignment,
        Some(scriptcraft_core::TitlePageAlignment::Right)
    );
}

fn attach_opaque_child(nodes: &mut [PreservedNode], target_id: &scriptcraft_core::StableId) -> bool {
    for node in nodes {
        let PreservedNode::Element(element) = node else {
            continue;
        };
        if matches!(&element.ownership, SourceOwnership::Managed(id) if id == target_id) {
            if element.children.len() > 1 {
                element.children.insert(1, PreservedNode::Comment("opaque-run-boundary".into()));
                return true;
            }
        }
        if attach_opaque_child(&mut element.children, target_id) {
            return true;
        }
    }
    false
}

#[test]
fn invalid_selection_and_unsupported_source_mapping_reject_without_session_mutation() {
    let mut session = valid(EditingSession::new(valid(Project::new())));
    let before_invalid_selection = session.clone();
    assert!(matches!(
        execute_editor_command(
            &mut session,
            &Command::EditorSelectionSet {
                surface: EditorSurface::TitlePage,
                selection: Selection {
                    anchor: command_position(usize::MAX, 0),
                    focus: command_position(usize::MAX, 0),
                },
            },
        ),
        CommandOutcome::Rejected(_)
    ));
    assert_eq!(session, before_invalid_selection);

    let project = valid(read_v6(include_bytes!("../../../.reference/reference3.fdx"))).project;
    let title_index = project
        .title_page
        .iter()
        .position(|paragraph| paragraph.text == "Name of First Writer")
        .unwrap_or_else(|| unreachable!("reference3 has a mixed-run title paragraph"));
    let title_id = project.title_page[title_index].id.clone();
    let mut project = project;
    let preservation = project
        .preservation
        .as_mut()
        .unwrap_or_else(|| unreachable!("imported reference has preserved source"));
    let attached = preservation
        .sections
        .iter_mut()
        .any(|section| attach_opaque_child(&mut section.children, &title_id));
    assert!(attached);
    preservation.rebuild_indexes();

    let mut source_session = valid(EditingSession::new(project));
    assert_eq!(
        set_selection(
            &mut source_session,
            EditorSurface::TitlePage,
            CorePosition { element: title_index, offset: 0 },
            CorePosition {
                element: title_index,
                offset: "Name of First Writer".len(),
            },
        ),
        CommandOutcome::Changed
    );
    let before_unsupported_mapping = source_session.clone();
    assert!(matches!(
        execute_editor_command(
            &mut source_session,
            &Command::EditorStyleToggle {
                surface: EditorSurface::TitlePage,
                style: TitleTextStyle::Underline,
            },
        ),
        CommandOutcome::Failed(_)
    ));
    assert_eq!(source_session, before_unsupported_mapping);
}

#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_commands::{
    execute_editor_command, execute_editor_command_on_surface, Command, CommandOutcome,
    CommandRejection, EditorSurface, PointerPhase, Position, RoleValue, Selection,
    TitleTextStyle, UnavailableReason, ParagraphAlignmentValue,
};
use scriptcraft_core::editing::{EditError, EditingSession, Position as CorePosition, Selection as CoreSelection};
use scriptcraft_core::{Project, Role, ScreenplayElement, MAX_TEXT_BYTES};

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("executor test fixture and bounded operation are valid"))
}

fn session() -> EditingSession {
    valid(EditingSession::new(valid(Project::new())))
}

fn position(element: u32, offset: u32) -> Position {
    Position { element, offset }
}

fn selection(anchor: Position, focus: Position) -> Selection {
    Selection { anchor, focus }
}

#[test]
fn text_paste_delete_enter_tab_and_role_commands_delegate_semantics() {
    let mut editing = session();
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorTextInsert { text: "INT. CAFÉ — NUIT".into(), surface: EditorSurface::Body },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(execute_editor_command(&mut editing, &Command::EditorParagraphEnter), CommandOutcome::Changed);
    assert_eq!(editing.project().elements[0].role, Role::SceneHeading);
    assert_eq!(editing.project().elements[1].role, Role::Action);

    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorElementChange { role: RoleValue::new(Role::Character), surface: EditorSurface::Body },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorTextPaste { text: "MARA\n(quietly)".into(), surface: EditorSurface::Body },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(editing.project().elements[1].role, Role::Character);
    assert_eq!(editing.project().elements[1].text, "MARA\n(quietly)");

    valid(editing.set_selection(CoreSelection::caret(CorePosition { element: 1, offset: 1 })));
    assert_eq!(execute_editor_command(&mut editing, &Command::EditorDeleteBackward), CommandOutcome::Changed);
    assert_eq!(editing.project().elements[1].text, "ARA\n(quietly)");
    valid(editing.set_selection(CoreSelection::caret(CorePosition { element: 1, offset: 0 })));
    assert_eq!(execute_editor_command(&mut editing, &Command::EditDelete), CommandOutcome::Changed);
    assert_eq!(editing.project().elements[1].text, "RA\n(quietly)");

    let mut expected_project = valid(Project::new());
    scriptcraft_core::BodyOwner::update(&mut expected_project.elements[0], |element| {
        element.role = Role::Action;
    });
    let mut expected = valid(EditingSession::new(expected_project.clone()));
    let mut actual = valid(EditingSession::new(expected_project));
    let expected_before = expected.clone();
    let direct_result = expected.tab();
    let expected_outcome = match direct_result {
        Ok(()) if expected == expected_before => CommandOutcome::NoOp,
        Ok(()) => CommandOutcome::Changed,
        Err(error) => CommandOutcome::Failed(error),
    };
    assert_eq!(
        execute_editor_command(&mut actual, &Command::EditorParagraphTab),
        expected_outcome
    );
    assert_eq!(actual, expected);
}

#[test]
fn selection_and_pointer_navigation_commits_use_semantic_utf8_positions() {
    let mut editing = session();
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorTextInsert { text: "one two".into(), surface: EditorSurface::Body },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorSelectionSet {
                selection: selection(position(0, 0), position(0, 0)),
                surface: EditorSurface::Body,
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorSelectionPointer {
                position: position(0, 1),
                phase: PointerPhase::DoubleClick,
                surface: EditorSurface::Body,
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        editing.selection(),
        CoreSelection {
            anchor: CorePosition { element: 0, offset: 0 },
            focus: CorePosition { element: 0, offset: 3 },
        }
    );
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorSelectionPointer {
                position: position(0, 7),
                phase: PointerPhase::Drag,
                surface: EditorSurface::Body,
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(editing.selection().anchor, CorePosition { element: 0, offset: 0 });
    assert_eq!(editing.selection().focus, CorePosition { element: 0, offset: 7 });
}

#[test]
fn formatting_and_history_are_single_session_semantics() {
    let mut editing = session();
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorTextInsert { text: "styled line".into(), surface: EditorSurface::Body },
        ),
        CommandOutcome::Changed
    );
    valid(editing.set_selection(CoreSelection {
        anchor: CorePosition { element: 0, offset: 0 },
        focus: CorePosition { element: 0, offset: "styled line".len() },
    }));
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorStyleToggle { surface: EditorSurface::Body, style: TitleTextStyle::Bold },
        ),
        CommandOutcome::Changed
    );
    let formatted = editing.project().clone();
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorAlignmentSet { surface: EditorSurface::Body, alignment: ParagraphAlignmentValue::Right },
        ),
        CommandOutcome::Changed
    );
    let aligned = editing.project().clone();
    assert_ne!(formatted, aligned);
    assert_eq!(execute_editor_command(&mut editing, &Command::EditUndo), CommandOutcome::Changed);
    assert_eq!(editing.project(), &formatted);
    assert_eq!(execute_editor_command(&mut editing, &Command::EditUndo), CommandOutcome::Changed);
    assert_ne!(editing.project(), &formatted);
    assert_eq!(execute_editor_command(&mut editing, &Command::EditRedo), CommandOutcome::Changed);
    assert_eq!(editing.project(), &formatted);
    assert_eq!(execute_editor_command(&mut editing, &Command::EditRedo), CommandOutcome::Changed);
    assert_eq!(editing.project(), &aligned);
}

#[test]
fn shared_owner_command_outcomes_restore_history_pointers_and_preserve_noop_identity() {
    let mut editing = session();
    let before = editing.project().clone();
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorTextInsert { text: "command text".into(), surface: EditorSurface::Body },
        ),
        CommandOutcome::Changed
    );
    let inserted = editing.project().clone();
    assert!(!std::sync::Arc::ptr_eq(&before.elements[0], &inserted.elements[0]));
    assert_ne!(before.elements[0].revision(), inserted.elements[0].revision());
    assert_eq!(execute_editor_command(&mut editing, &Command::EditUndo), CommandOutcome::Changed);
    assert!(std::sync::Arc::ptr_eq(&before.elements[0], &editing.project().elements[0]));
    assert_eq!(execute_editor_command(&mut editing, &Command::EditRedo), CommandOutcome::Changed);
    assert!(std::sync::Arc::ptr_eq(&inserted.elements[0], &editing.project().elements[0]));

    let revision = editing.revision();
    let history = editing.can_undo();
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorElementChange {
                role: RoleValue::new(Role::SceneHeading),
                surface: EditorSurface::Body,
            },
        ),
        CommandOutcome::NoOp
    );
    assert!(std::sync::Arc::ptr_eq(&inserted.elements[0], &editing.project().elements[0]));
    assert_eq!(editing.project().elements[0].revision(), inserted.elements[0].revision());
    assert_eq!(editing.revision(), revision);
    assert_eq!(editing.can_undo(), history);
}

#[test]
fn body_and_title_edits_are_isolated_but_share_global_undo_redo_history() {
    let mut editing = session();
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorTextInsert { text: "body text".into(), surface: EditorSurface::Body },
        ),
        CommandOutcome::Changed
    );
    let body_text = editing.project().elements[0].text.clone();
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorTextInsert { text: "title text".into(), surface: EditorSurface::TitlePage },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(editing.project().elements[0].text, body_text);
    assert!(editing.project().title_page[0].text.starts_with("title text"));
    assert_eq!(
        execute_editor_command_on_surface(&mut editing, &Command::EditSelectAll, EditorSurface::TitlePage),
        CommandOutcome::Changed
    );
    assert_eq!(editing.title_selection().anchor, CorePosition { element: 0, offset: 0 });

    assert_eq!(execute_editor_command(&mut editing, &Command::EditUndo), CommandOutcome::Changed);
    assert_eq!(editing.project().title_page[0].text, "");
    assert_eq!(editing.project().elements[0].text, body_text);
    assert_eq!(execute_editor_command(&mut editing, &Command::EditUndo), CommandOutcome::Changed);
    assert_eq!(editing.project().elements[0].text, "");
    assert_eq!(execute_editor_command(&mut editing, &Command::EditRedo), CommandOutcome::Changed);
    assert_eq!(editing.project().elements[0].text, body_text);
    assert_eq!(execute_editor_command(&mut editing, &Command::EditRedo), CommandOutcome::Changed);
    assert!(editing.project().title_page[0].text.starts_with("title text"));
}

fn dual_dialogue_session() -> EditingSession {
    let mut project = valid(Project::new());
    project.elements = [Role::Character, Role::Dialogue, Role::Character, Role::Dialogue]
        .into_iter()
        .enumerate()
        .map(|(index, role)| scriptcraft_core::BodyOwner::shared(valid(ScreenplayElement::new(format!("speaker-{index}"), role, "line"))))
        .collect();
    valid(EditingSession::new(project))
}

#[test]
fn dual_dialogue_toggle_unwrap_and_one_undo_redo_journey() {
    let mut editing = dual_dialogue_session();
    assert_eq!(
        execute_editor_command(&mut editing, &Command::EditorDualDialogueToggle),
        CommandOutcome::Changed
    );
    assert_eq!(editing.project().dual_dialogue_groups.len(), 1);
    let paired = editing.project().clone();

    valid(editing.set_selection(CoreSelection::caret(CorePosition { element: 1, offset: 0 })));
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorElementChange { role: RoleValue::new(Role::Action), surface: EditorSurface::Body },
        ),
        CommandOutcome::Changed
    );
    assert!(editing.project().dual_dialogue_groups.is_empty());
    let unwrapped = editing.project().clone();
    assert_eq!(execute_editor_command(&mut editing, &Command::EditUndo), CommandOutcome::Changed);
    assert_eq!(editing.project(), &paired);
    assert_eq!(execute_editor_command(&mut editing, &Command::EditRedo), CommandOutcome::Changed);
    assert_eq!(editing.project(), &unwrapped);
}

#[test]
fn noops_unavailable_selection_and_history_and_unsupported_effects_do_not_mutate() {
    let mut editing = session();
    let before = editing.clone();
    assert_eq!(execute_editor_command(&mut editing, &Command::EditorParagraphEnter), CommandOutcome::NoOp);
    assert_eq!(editing, before);
    assert!(!editing.can_undo());
    assert_eq!(
        execute_editor_command(&mut editing, &Command::EditUndo),
        CommandOutcome::Unavailable(UnavailableReason::UndoHistory)
    );
    assert_eq!(
        execute_editor_command(&mut editing, &Command::EditRedo),
        CommandOutcome::Unavailable(UnavailableReason::RedoHistory)
    );

    let mut empty_title_project = valid(Project::new());
    empty_title_project.title_page.clear();
    let mut empty_title = valid(EditingSession::new(empty_title_project));
    let empty_before = empty_title.clone();
    assert_eq!(
        execute_editor_command_on_surface(&mut empty_title, &Command::EditSelectAll, EditorSurface::TitlePage),
        CommandOutcome::Unavailable(UnavailableReason::Selection)
    );
    assert_eq!(empty_title, empty_before);

    for command in [
        Command::FilePrint,
        Command::EditorFocus,
        Command::WindowTitlePageOpen,
        Command::EditorElementChange { role: RoleValue::new(Role::Action), surface: EditorSurface::TitlePage },
    ] {
        let state = editing.clone();
        assert_eq!(
            execute_editor_command(&mut editing, &command),
            CommandOutcome::Rejected(CommandRejection::UnsupportedCommand(command.id()))
        );
        assert_eq!(editing, state);
    }
}

#[test]
fn invalid_wire_parameters_utf8_and_core_errors_roll_back_atomically() {
    let mut editing = session();
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorTextInsert { text: "é".into(), surface: EditorSurface::Body },
        ),
        CommandOutcome::Changed
    );
    let before = editing.clone();
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorSelectionSet {
                selection: selection(position(0, 1), position(0, 1)),
                surface: EditorSurface::Body,
            },
        ),
        CommandOutcome::Rejected(CommandRejection::InvalidSelection(
            scriptcraft_commands::SelectionError::InvalidUtf8Boundary,
        ))
    );
    assert_eq!(editing, before);
    assert!(matches!(
        execute_editor_command(
            &mut editing,
            &Command::EditorSelectionSet {
                selection: selection(position(u32::MAX, 0), position(u32::MAX, 0)),
                surface: EditorSurface::Body,
            },
        ),
        CommandOutcome::Rejected(CommandRejection::InvalidSelection(_))
    ));
    assert_eq!(editing, before);

    let oversized = "x".repeat(MAX_TEXT_BYTES + 1);
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorTextInsert { text: oversized, surface: EditorSurface::Body },
        ),
        CommandOutcome::Failed(EditError::TextTooLong)
    );
    assert_eq!(editing, before);

    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorParagraphEnter,
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorSelectionSet {
                selection: selection(position(0, 0), position(1, 0)),
                surface: EditorSurface::Body,
            },
        ),
        CommandOutcome::Changed
    );
    let invalid_role_change = editing.clone();
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorElementChange { role: RoleValue::new(Role::Dialogue), surface: EditorSurface::Body },
        ),
        CommandOutcome::Failed(EditError::InvalidSelection)
    );
    assert_eq!(editing, invalid_role_change);
}

#[test]
fn title_delete_and_enter_commands_report_effective_changes() {
    let mut editing = session();
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorTextInsert { text: "ab".into(), surface: EditorSurface::TitlePage },
        ),
        CommandOutcome::Changed
    );
    valid(editing.set_title_selection(CoreSelection::caret(CorePosition { element: 0, offset: 1 })));
    assert_eq!(
        execute_editor_command_on_surface(&mut editing, &Command::EditorDeleteBackward, EditorSurface::TitlePage),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command_on_surface(&mut editing, &Command::EditDelete, EditorSurface::TitlePage),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command_on_surface(&mut editing, &Command::EditorParagraphEnter, EditorSurface::TitlePage),
        CommandOutcome::Changed
    );
    let before = editing.clone();
    assert_eq!(
        execute_editor_command_on_surface(&mut editing, &Command::EditorParagraphTab, EditorSurface::TitlePage),
        CommandOutcome::NoOp
    );
    assert_noop_public_state(&editing, &before);
}

fn assert_noop_public_state(actual: &EditingSession, expected: &EditingSession) {
    assert_eq!(actual.project(), expected.project());
    assert_eq!(actual.selection(), expected.selection());
    assert_eq!(actual.title_selection(), expected.title_selection());
    assert_eq!(actual.body_typing_style(), expected.body_typing_style());
    assert_eq!(actual.title_typing_style(), expected.title_typing_style());
    assert_eq!(actual.revision(), expected.revision());
    assert_eq!(actual.mutation_token(), expected.mutation_token());
    assert_eq!(actual.dirty(), expected.dirty());
    assert_eq!(actual.can_undo(), expected.can_undo());
    assert_eq!(actual.can_redo(), expected.can_redo());
}

#[test]
fn body_and_title_boundary_noops_preserve_public_state_history_and_revision() {
    let mut editing = session();
    for command in [
        Command::EditorDeleteBackward,
        Command::EditDelete,
        Command::EditorParagraphEnter,
    ] {
        let before = editing.clone();
        assert_eq!(
            execute_editor_command_on_surface(&mut editing, &command, EditorSurface::Body),
            CommandOutcome::NoOp
        );
        assert_noop_public_state(&editing, &before);
    }

    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorTextInsert { text: "boundary".into(), surface: EditorSurface::Body },
        ),
        CommandOutcome::Changed
    );
    valid(editing.set_selection(CoreSelection::caret(CorePosition { element: 0, offset: 0 })));
    let before = editing.clone();
    assert_eq!(
        execute_editor_command_on_surface(&mut editing, &Command::EditorDeleteBackward, EditorSurface::Body),
        CommandOutcome::NoOp
    );
    assert_noop_public_state(&editing, &before);

    let mut editing = session();
    valid(editing.set_title_selection(CoreSelection::caret(CorePosition { element: 0, offset: 0 })));
    let before = editing.clone();
    assert_eq!(
        execute_editor_command_on_surface(&mut editing, &Command::EditorDeleteBackward, EditorSurface::TitlePage),
        CommandOutcome::NoOp
    );
    assert_noop_public_state(&editing, &before);

    let last = editing.project().title_page.len() - 1;
    let offset = editing.project().title_page[last].text.len();
    valid(editing.set_title_selection(CoreSelection::caret(CorePosition { element: last, offset })));
    for command in [Command::EditDelete, Command::EditorParagraphTab] {
        let before = editing.clone();
        assert_eq!(
            execute_editor_command_on_surface(&mut editing, &command, EditorSurface::TitlePage),
            CommandOutcome::NoOp
        );
        assert_noop_public_state(&editing, &before);
    }

    let mut general_project = valid(Project::new());
    scriptcraft_core::BodyOwner::update(&mut general_project.elements[0], |element| {
        element.role = Role::General;
    });
    let mut general = valid(EditingSession::new(general_project));
    let before = general.clone();
    assert_eq!(
        execute_editor_command(&mut general, &Command::EditorParagraphTab),
        CommandOutcome::NoOp
    );
    assert_noop_public_state(&general, &before);
}

#[test]
fn selection_and_typing_style_changes_do_not_require_document_revision_or_history() {
    let mut editing = session();
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorTextInsert { text: "one two".into(), surface: EditorSurface::Body },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorTextInsert { text: "title".into(), surface: EditorSurface::TitlePage },
        ),
        CommandOutcome::Changed
    );
    let project = editing.project().clone();
    let revision = editing.revision();
    let can_undo = editing.can_undo();
    let can_redo = editing.can_redo();

    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorSelectionSet {
                selection: selection(position(0, 0), position(0, 0)),
                surface: EditorSurface::Body,
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorSelectionPointer {
                position: position(0, 0),
                phase: PointerPhase::Press,
                surface: EditorSurface::TitlePage,
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(editing.project(), &project);
    assert_eq!(editing.revision(), revision);
    assert_eq!(editing.can_undo(), can_undo);
    assert_eq!(editing.can_redo(), can_redo);

    for surface in [EditorSurface::Body, EditorSurface::TitlePage] {
        assert_eq!(
            execute_editor_command(
                &mut editing,
                &Command::EditorStyleToggle { surface, style: TitleTextStyle::Bold },
            ),
            CommandOutcome::Changed
        );
        assert_eq!(editing.project(), &project);
        assert_eq!(editing.revision(), revision);
        assert_eq!(editing.can_undo(), can_undo);
        assert_eq!(editing.can_redo(), can_redo);
        assert_eq!(
            execute_editor_command(&mut editing, &Command::EditorStyleClear { surface }),
            CommandOutcome::Changed
        );
        let before = editing.clone();
        assert_eq!(
            execute_editor_command(&mut editing, &Command::EditorStyleClear { surface }),
            CommandOutcome::NoOp
        );
        assert_noop_public_state(&editing, &before);
    }
}

#[test]
fn role_alignment_and_ineligible_dual_dialogue_distinguish_changed_from_noop() {
    let mut editing = session();
    let before = editing.clone();
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorElementChange {
                role: RoleValue::new(Role::SceneHeading),
                surface: EditorSurface::Body,
            },
        ),
        CommandOutcome::NoOp
    );
    assert_noop_public_state(&editing, &before);

    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorElementChange { role: RoleValue::new(Role::Action), surface: EditorSurface::Body },
        ),
        CommandOutcome::Changed
    );
    let before = editing.clone();
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorElementChange { role: RoleValue::new(Role::Action), surface: EditorSurface::Body },
        ),
        CommandOutcome::NoOp
    );
    assert_noop_public_state(&editing, &before);

    for surface in [EditorSurface::Body, EditorSurface::TitlePage] {
        assert_eq!(
            execute_editor_command(
                &mut editing,
                &Command::EditorAlignmentSet { surface, alignment: ParagraphAlignmentValue::Right },
            ),
            CommandOutcome::Changed
        );
        let before = editing.clone();
        assert_eq!(
            execute_editor_command(
                &mut editing,
                &Command::EditorAlignmentSet { surface, alignment: ParagraphAlignmentValue::Right },
            ),
            CommandOutcome::NoOp
        );
        assert_noop_public_state(&editing, &before);
    }

    let before = editing.clone();
    assert_eq!(
        execute_editor_command(&mut editing, &Command::EditorDualDialogueToggle),
        CommandOutcome::NoOp
    );
    assert_noop_public_state(&editing, &before);
    assert_eq!(
        execute_editor_command_on_surface(&mut editing, &Command::EditorDualDialogueToggle, EditorSurface::TitlePage),
        CommandOutcome::Unavailable(UnavailableReason::DualDialogueTarget)
    );
    assert_noop_public_state(&editing, &before);
}

#[test]
fn content_edits_and_history_remain_changed_while_dirty_stays_true() {
    let mut editing = session();
    for command in [
        Command::EditorTextInsert { text: "body".into(), surface: EditorSurface::Body },
        Command::EditorTextPaste { text: " paste".into(), surface: EditorSurface::Body },
        Command::EditorTextInsert { text: "title".into(), surface: EditorSurface::TitlePage },
        Command::EditorTextPaste { text: " paste".into(), surface: EditorSurface::TitlePage },
    ] {
        let revision = editing.revision();
        assert_eq!(execute_editor_command(&mut editing, &command), CommandOutcome::Changed);
        assert!(editing.dirty());
        assert_eq!(editing.revision(), revision + 1);
    }

    let dirty_revision = editing.revision();
    assert_eq!(execute_editor_command(&mut editing, &Command::EditUndo), CommandOutcome::Changed);
    assert!(editing.dirty());
    assert!(editing.can_redo());
    assert_eq!(editing.revision(), dirty_revision + 1);
    assert_eq!(execute_editor_command(&mut editing, &Command::EditRedo), CommandOutcome::Changed);
    assert!(editing.dirty());
    assert_eq!(editing.revision(), dirty_revision + 2);
}

#[test]
fn executor_outcome_detection_has_no_whole_session_snapshot() {
    let source = include_str!("../src/executor.rs");
    assert!(!source.contains("session.clone()"));
    assert!(!source.contains("*session != before"));
    assert!(source.contains("session.mutation_token()"));
}

#[test]
fn ordinary_body_executor_outcomes_survive_move_to_history_publication() {
    let mut editing = session();
    for (command, expected_text) in [
        (
            Command::EditorTextInsert {
                text: "a".into(),
                surface: EditorSurface::Body,
            },
            "a",
        ),
        (
            Command::EditorTextPaste {
                text: "b".into(),
                surface: EditorSurface::Body,
            },
            "ab",
        ),
    ] {
        let revision = editing.revision();
        let token = editing.mutation_token();
        assert_eq!(execute_editor_command(&mut editing, &command), CommandOutcome::Changed);
        assert_eq!(editing.project().elements[0].text, expected_text);
        assert_eq!(editing.revision(), revision + 1);
        assert_eq!(editing.mutation_token(), token + 1);
        assert!(editing.dirty());
    }

    let inserted = editing.project().clone();
    assert_eq!(
        execute_editor_command(&mut editing, &Command::EditorDeleteBackward),
        CommandOutcome::Changed
    );
    assert_eq!(editing.project().elements[0].text, "a");
    let deleted = editing.project().clone();
    assert_eq!(execute_editor_command(&mut editing, &Command::EditDelete), CommandOutcome::NoOp);
    assert_eq!(editing.project(), &deleted);
    assert!(!editing.can_redo());

    assert_eq!(execute_editor_command(&mut editing, &Command::EditUndo), CommandOutcome::Changed);
    assert_eq!(editing.project(), &inserted);
    assert!(editing.dirty());
    assert_eq!(execute_editor_command(&mut editing, &Command::EditRedo), CommandOutcome::Changed);
    assert_eq!(editing.project(), &deleted);
}

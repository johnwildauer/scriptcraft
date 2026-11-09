#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_commands::{EditorSurface, 
    execute_editor_command, validate_descriptor_catalog, Command, CommandId, CommandOutcome,
    CommandRejection, DiagnosticMessage, Position, RoleValue, Selection, UnavailableReason,
    COMMAND_DESCRIPTORS, MAX_DIAGNOSTIC_MESSAGE_BYTES,
};
use scriptcraft_core::editing::{EditError, EditingSession, Position as CorePosition, Selection as CoreSelection};
use scriptcraft_core::{Project, Role, MAX_TEXT_BYTES};

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("executor scenario uses valid bounded input"))
}

fn new_session() -> EditingSession {
    valid(EditingSession::new(valid(Project::new())))
}

#[test]
fn diagnostic_status_is_bounded_catalogued_and_rejected_by_headless_execution() {
    let message = valid(DiagnosticMessage::new("SAVE_FAILED: destination conflict"));
    assert_eq!(CommandId::ALL.len(), 48);
    assert_eq!(COMMAND_DESCRIPTORS.len(), 48);
    assert_eq!(validate_descriptor_catalog(), Ok(()));
    assert_eq!(CommandId::UiStatusSet.as_str(), "ui.status.set");
    assert_eq!(
        DiagnosticMessage::new("x".repeat(MAX_DIAGNOSTIC_MESSAGE_BYTES + 1)).err(),
        Some(scriptcraft_commands::InvalidDiagnosticMessage)
    );
    assert!(DiagnosticMessage::new("two\nlines").is_err());

    let mut session = new_session();
    let before = session.clone();
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::UiStatusSet {
                message: Some(message),
            },
        ),
        CommandOutcome::Rejected(CommandRejection::UnsupportedCommand(CommandId::UiStatusSet))
    );
    assert_eq!(session, before);
}

fn position(element: u32, offset: u32) -> Position {
    Position { element, offset }
}

fn selection(anchor: Position, focus: Position) -> Selection {
    Selection { anchor, focus }
}

fn assert_same_operation(command: Command, use_enter: bool, role: Role) {
    let mut project = valid(Project::new());
    project.elements[0].role = role;
    let initial = valid(EditingSession::new(project));
    let mut expected = initial.clone();
    let mut actual = initial;
    let expected_result = if use_enter {
        expected.enter()
    } else {
        expected.tab()
    };
    let expected_outcome = match expected_result {
        Ok(()) if expected == actual => CommandOutcome::NoOp,
        Ok(()) => CommandOutcome::Changed,
        Err(error) => CommandOutcome::Failed(error),
    };

    assert_eq!(execute_editor_command(&mut actual, &command), expected_outcome);
    assert_eq!(actual, expected);
}

#[test]
fn semantic_editing_preserves_roles_unicode_history_and_dirty_state_through_commands() {
    let mut session = new_session();

    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorTextInsert { surface: EditorSurface::Body, 
                text: "INT. CAFÉ — NUIT".into(),
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(&mut session, &Command::EditorParagraphEnter),
        CommandOutcome::Changed
    );
    assert_eq!(session.project().elements[0].role, Role::SceneHeading);
    assert_eq!(session.project().elements[0].text, "INT. CAFÉ — NUIT");
    assert_eq!(session.project().elements[1].role, Role::Action);

    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorElementChange { surface: EditorSurface::Body, 
                role: RoleValue::new(Role::Character),
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorTextInsert { surface: EditorSurface::Body, 
                text: "MARA".into(),
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(&mut session, &Command::EditorParagraphEnter),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorTextInsert { surface: EditorSurface::Body, 
                text: "こんにちは、🌙".into(),
            },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(session.project().elements[1].role, Role::Character);
    assert_eq!(session.project().elements[2].role, Role::Dialogue);
    assert_eq!(session.project().elements[2].text, "こんにちは、🌙");
    assert!(session.dirty());

    let edited = session.project().clone();
    assert_eq!(
        execute_editor_command(&mut session, &Command::EditUndo),
        CommandOutcome::Changed
    );
    assert_ne!(session.project(), &edited);
    assert!(session.dirty());
    assert_eq!(
        execute_editor_command(&mut session, &Command::EditRedo),
        CommandOutcome::Changed
    );
    assert_eq!(session.project(), &edited);
    assert!(session.dirty());
}

#[test]
fn every_role_parameter_reaches_the_existing_role_change_operation() {
    for role in Role::ALL {
        let mut session = new_session();
        let expected = if role == Role::SceneHeading {
            CommandOutcome::NoOp
        } else {
            CommandOutcome::Changed
        };
        assert_eq!(
            execute_editor_command(
                &mut session,
                &Command::EditorElementChange { surface: EditorSurface::Body, 
                    role: RoleValue::new(role),
                },
            ),
            expected
        );
        assert_eq!(session.project().elements[0].role, role);
    }
}

#[test]
fn enter_and_tab_commands_delegate_the_complete_role_matrix() {
    for role in Role::ALL {
        assert_same_operation(Command::EditorParagraphEnter, true, role);
        assert_same_operation(Command::EditorParagraphTab, false, role);
    }
}

#[test]
fn text_and_paste_are_separate_commands_and_paste_keeps_newlines_in_the_element() {
    let mut typed = new_session();
    let mut pasted = new_session();
    let text = "first line\nsecond line";

    assert_eq!(
        execute_editor_command(
            &mut typed,
            &Command::EditorTextInsert { surface: EditorSurface::Body,  text: text.into() },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut pasted,
            &Command::EditorTextPaste { surface: EditorSurface::Body,  text: text.into() },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(typed.project().elements.len(), 1);
    assert_eq!(pasted.project().elements.len(), 1);
    assert_eq!(typed.project().elements[0].text, text);
    assert_eq!(pasted.project().elements[0].text, text);
}

#[test]
fn backspace_and_forward_delete_delegate_to_distinct_session_operations() {
    let mut backward = new_session();
    let mut forward = new_session();
    for session in [&mut backward, &mut forward] {
        assert_eq!(
            execute_editor_command(
                session,
                &Command::EditorTextInsert { surface: EditorSurface::Body,  text: "AB".into() },
            ),
            CommandOutcome::Changed
        );
        valid(session.set_selection(CoreSelection::caret(CorePosition {
            element: 0,
            offset: 1,
        })));
    }

    assert_eq!(
        execute_editor_command(&mut backward, &Command::EditorDeleteBackward),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(&mut forward, &Command::EditDelete),
        CommandOutcome::Changed
    );
    assert_eq!(backward.project().elements[0].text, "B");
    assert_eq!(forward.project().elements[0].text, "A");
}

#[test]
fn accepted_noops_do_not_create_history_and_empty_history_is_unavailable() {
    let mut session = new_session();
    let before = session.clone();

    assert_eq!(
        execute_editor_command(&mut session, &Command::EditorParagraphEnter),
        CommandOutcome::NoOp
    );
    assert_eq!(session, before);
    assert!(!session.can_undo());
    assert_eq!(
        execute_editor_command(&mut session, &Command::EditUndo),
        CommandOutcome::Unavailable(UnavailableReason::UndoHistory)
    );
    assert_eq!(
        execute_editor_command(&mut session, &Command::EditRedo),
        CommandOutcome::Unavailable(UnavailableReason::RedoHistory)
    );
    assert_eq!(session, before);
}

#[test]
fn invalid_utf8_selection_is_rejected_without_session_mutation() {
    let mut session = new_session();
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorTextInsert { surface: EditorSurface::Body,  text: "é".into() },
        ),
        CommandOutcome::Changed
    );
    let before = session.clone();

    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorSelectionSet { surface: EditorSurface::Body, 
                selection: selection(position(0, 1), position(0, 1)),
            },
        ),
        CommandOutcome::Rejected(CommandRejection::InvalidSelection(
            scriptcraft_commands::SelectionError::InvalidUtf8Boundary,
        ))
    );
    assert_eq!(session, before);
}

#[test]
fn cross_element_role_change_preserves_the_original_core_error_and_session() {
    let mut session = new_session();
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorTextInsert { surface: EditorSurface::Body,  text: "A".into() },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(&mut session, &Command::EditorParagraphEnter),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorSelectionSet { surface: EditorSurface::Body, 
                selection: selection(position(0, 0), position(1, 0)),
            },
        ),
        CommandOutcome::Changed
    );
    let before = session.clone();

    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorElementChange { surface: EditorSurface::Body, 
                role: RoleValue::new(Role::Dialogue),
            },
        ),
        CommandOutcome::Failed(EditError::InvalidSelection)
    );
    assert_eq!(session, before);
}

#[test]
fn failed_oversized_insert_and_non_editor_command_leave_session_unchanged() {
    let mut session = new_session();
    let before = session.clone();
    let oversized = "x".repeat(MAX_TEXT_BYTES + 1);

    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorTextInsert { surface: EditorSurface::Body,  text: oversized },
        ),
        CommandOutcome::Failed(EditError::TextTooLong)
    );
    assert_eq!(session, before);
    assert_eq!(
        execute_editor_command(&mut session, &Command::FileNew),
        CommandOutcome::Rejected(CommandRejection::UnsupportedCommand(
            scriptcraft_commands::CommandId::FileNew,
        ))
    );
    assert_eq!(session, before);
}

#[test]
fn select_all_uses_a_direct_session_selection_update() {
    let mut session = new_session();
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorTextInsert { surface: EditorSurface::Body,  text: "scene".into() },
        ),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(&mut session, &Command::EditorParagraphEnter),
        CommandOutcome::Changed
    );
    assert_eq!(
        execute_editor_command(
            &mut session,
            &Command::EditorTextInsert { surface: EditorSurface::Body,  text: "action".into() },
        ),
        CommandOutcome::Changed
    );

    assert_eq!(
        execute_editor_command(&mut session, &Command::EditSelectAll),
        CommandOutcome::Changed
    );
    assert_eq!(
        session.selection(),
        CoreSelection {
            anchor: CorePosition {
                element: 0,
                offset: 0,
            },
            focus: CorePosition {
                element: 1,
                offset: "action".len(),
            },
        }
    );
}

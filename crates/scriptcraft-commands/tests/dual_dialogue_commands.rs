#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use scriptcraft_commands::{
    descriptor_for, execute_editor_command, execute_editor_command_on_surface,
    validate_descriptor_catalog, AvailabilityPolicy, Command, CommandEnvelope, CommandId,
    CommandOrigin, CommandOutcome, EditorSurface, RoleValue, UnavailableReason, COMMAND_DESCRIPTORS,
    COMMAND_WIRE_VERSION,
};
use scriptcraft_core::editing::{EditingSession, Position, Selection};
use scriptcraft_core::{Project, Role, ScreenplayElement};

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("dual-dialogue command fixture is valid"))
}

fn session_with_toggle_pattern() -> EditingSession {
    let mut project = valid(Project::new());
    project.elements = [
        Role::Character,
        Role::Dialogue,
        Role::Character,
        Role::Dialogue,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, role)| {
        valid(ScreenplayElement::new(format!("command-body-{index}"), role, "line"))
    })
    .collect();
    valid(EditingSession::new(project))
}

#[test]
fn dual_toggle_keeps_its_stable_id_and_version_one_wire_contract() {
    assert_eq!(validate_descriptor_catalog(), Ok(()));
    assert_eq!(CommandId::ALL.len(), 50);
    assert_eq!(COMMAND_DESCRIPTORS.len(), 50);
    assert_eq!(CommandId::ALL[48], CommandId::EditorDualDialogueToggle);
    assert_eq!(CommandId::EditorDualDialogueToggle.as_str(), "editor.dual-dialogue.toggle");
    let descriptor = descriptor_for(CommandId::EditorDualDialogueToggle);
    assert!(descriptor.is_some_and(|descriptor| {
        descriptor.parameters == "none; Body only; host active surface"
            && descriptor.availability == AvailabilityPolicy::RecheckAtExecution
    }));
    assert_eq!(COMMAND_WIRE_VERSION, 1);
    let envelope = CommandEnvelope::new(CommandOrigin::Test, Command::EditorDualDialogueToggle);
    assert_eq!(envelope.version, 1);
    assert_eq!(envelope.command.id(), CommandId::EditorDualDialogueToggle);
}

#[test]
fn body_toggle_enables_disables_and_uses_one_undo_redo_history_step_each_way() {
    let mut editing = session_with_toggle_pattern();
    assert_eq!(
        execute_editor_command(&mut editing, &Command::EditorDualDialogueToggle),
        CommandOutcome::Changed,
    );
    assert_eq!(editing.project().dual_dialogue_groups.len(), 1);
    assert!(editing.can_undo());

    assert_eq!(
        execute_editor_command(&mut editing, &Command::EditUndo),
        CommandOutcome::Changed,
    );
    assert!(editing.project().dual_dialogue_groups.is_empty());
    assert!(editing.can_redo());

    assert_eq!(
        execute_editor_command(&mut editing, &Command::EditRedo),
        CommandOutcome::Changed,
    );
    assert_eq!(editing.project().dual_dialogue_groups.len(), 1);

    assert_eq!(
        execute_editor_command(&mut editing, &Command::EditorDualDialogueToggle),
        CommandOutcome::Changed,
    );
    assert!(editing.project().dual_dialogue_groups.is_empty());
    assert!(editing.can_undo());
}

#[test]
fn ineligible_body_selection_is_a_noop_without_session_or_history_changes() {
    let mut editing = session_with_toggle_pattern();
    valid(editing.set_selection(Selection {
        anchor: Position { element: 0, offset: 0 },
        focus: Position { element: 0, offset: 1 },
    }));
    let before = editing.clone();
    assert_eq!(
        execute_editor_command(&mut editing, &Command::EditorDualDialogueToggle),
        CommandOutcome::NoOp,
    );
    assert_eq!(editing, before);
    assert!(!editing.can_undo());
}

#[test]
fn title_page_toggle_is_typed_unavailable_without_rerouting_or_mutation() {
    let mut editing = session_with_toggle_pattern();
    let before = editing.clone();
    assert_eq!(
        execute_editor_command_on_surface(
            &mut editing,
            &Command::EditorDualDialogueToggle,
            EditorSurface::TitlePage,
        ),
        CommandOutcome::Unavailable(UnavailableReason::DualDialogueTarget),
    );
    assert_eq!(editing, before);
    assert!(!editing.can_undo());
}

#[test]
fn command_remains_compatible_with_retained_groups_unwrap_and_final_right_enter() {
    let mut retained = session_with_toggle_pattern();
    assert_eq!(
        execute_editor_command(&mut retained, &Command::EditorDualDialogueToggle),
        CommandOutcome::Changed,
    );
    valid(retained.set_selection(scriptcraft_core::editing::Selection::caret(Position {
        element: 1,
        offset: 0,
    })));
    assert_eq!(
        execute_editor_command(
            &mut retained,
            &Command::EditorTextInsert {
                text: "edited ".into(),
                surface: EditorSurface::Body,
            },
        ),
        CommandOutcome::Changed,
    );
    assert_eq!(retained.project().dual_dialogue_groups.len(), 1);
    assert_eq!(
        execute_editor_command(&mut retained, &Command::EditorDualDialogueToggle),
        CommandOutcome::Changed,
    );
    assert!(retained.project().dual_dialogue_groups.is_empty());

    let mut unwrapped = session_with_toggle_pattern();
    assert_eq!(
        execute_editor_command(&mut unwrapped, &Command::EditorDualDialogueToggle),
        CommandOutcome::Changed,
    );
    valid(unwrapped.set_selection(scriptcraft_core::editing::Selection::caret(Position {
        element: 1,
        offset: 0,
    })));
    valid(unwrapped.change_role(Role::Action));
    assert!(unwrapped.project().dual_dialogue_groups.is_empty());
    let before_ineligible_toggle = unwrapped.clone();
    assert_eq!(
        execute_editor_command(&mut unwrapped, &Command::EditorDualDialogueToggle),
        CommandOutcome::NoOp,
    );
    assert_eq!(unwrapped, before_ineligible_toggle);

    let mut final_right_enter = session_with_toggle_pattern();
    assert_eq!(
        execute_editor_command(&mut final_right_enter, &Command::EditorDualDialogueToggle),
        CommandOutcome::Changed,
    );
    valid(final_right_enter.set_selection(
        scriptcraft_core::editing::Selection::caret(Position {
            element: 3,
            offset: "line".len(),
        }),
    ));
    assert_eq!(final_right_enter.enter(), Ok(()));
    assert_eq!(final_right_enter.project().dual_dialogue_groups.len(), 1);
    valid(final_right_enter.set_selection(
        scriptcraft_core::editing::Selection::caret(Position {
            element: 3,
            offset: 0,
        }),
    ));
    assert_eq!(
        execute_editor_command(
            &mut final_right_enter,
            &Command::EditorDualDialogueToggle,
        ),
        CommandOutcome::Changed,
    );
    assert!(final_right_enter.project().dual_dialogue_groups.is_empty());
}

#[test]
fn headless_command_journey_rechecks_eligibility_and_keeps_unwrap_in_one_history_step() {
    let mut editing = session_with_toggle_pattern();

    // A collapsed caret on a non-candidate paragraph is still an exact no-op.
    valid(editing.set_selection(Selection::caret(Position {
        element: 1,
        offset: 0,
    })));
    let before_ineligible = editing.clone();
    assert_eq!(
        execute_editor_command(&mut editing, &Command::EditorDualDialogueToggle),
        CommandOutcome::NoOp,
    );
    assert_eq!(editing, before_ineligible);
    assert!(!editing.can_undo());

    valid(editing.set_selection(Selection::caret(Position {
        element: 0,
        offset: 0,
    })));
    let toggle = CommandEnvelope::new(CommandOrigin::Test, Command::EditorDualDialogueToggle);
    assert_eq!(toggle.version, COMMAND_WIRE_VERSION);
    assert_eq!(
        execute_editor_command(&mut editing, &toggle.command),
        CommandOutcome::Changed,
    );
    assert_eq!(editing.project().dual_dialogue_groups.len(), 1);
    let member_ids = editing.project().dual_dialogue_groups[0]
        .member_ids()
        .cloned()
        .collect::<Vec<_>>();

    valid(editing.set_selection(Selection::caret(Position {
        element: 1,
        offset: 0,
    })));
    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorTextInsert {
                text: "edited ".into(),
                surface: EditorSurface::Body,
            },
        ),
        CommandOutcome::Changed,
    );
    assert_eq!(editing.project().elements[1].text, "edited line");
    assert_eq!(editing.project().dual_dialogue_groups[0].member_ids().cloned().collect::<Vec<_>>(), member_ids);

    // A valid Tab edit remains grouped; a subsequent role-breaking command
    // unwraps and edits atomically through the same headless executor.
    valid(editing.set_selection(Selection::caret(Position {
        element: 0,
        offset: "line".len(),
    })));
    assert_eq!(
        execute_editor_command(&mut editing, &Command::EditorParagraphTab),
        CommandOutcome::Changed,
    );
    assert_eq!(editing.project().dual_dialogue_groups.len(), 1);
    let retained = editing.project().clone();

    assert_eq!(
        execute_editor_command(
            &mut editing,
            &Command::EditorElementChange {
                role: RoleValue::new(Role::Action),
                surface: EditorSurface::Body,
            },
        ),
        CommandOutcome::Changed,
    );
    assert!(editing.project().dual_dialogue_groups.is_empty());
    let unwrapped = editing.project().clone();
    assert_eq!(
        execute_editor_command(&mut editing, &Command::EditUndo),
        CommandOutcome::Changed,
    );
    assert_eq!(editing.project(), &retained);
    assert_eq!(
        execute_editor_command(&mut editing, &Command::EditRedo),
        CommandOutcome::Changed,
    );
    assert_eq!(editing.project(), &unwrapped);
}

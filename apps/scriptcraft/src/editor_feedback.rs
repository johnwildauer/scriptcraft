//! Best-effort native feedback for narrowly classified editor rejections.

use scriptcraft_commands::CommandOutcome;
use scriptcraft_core::editing::EditError;

pub(super) trait RejectionSoundSink {
    fn play_rejection_sound(&mut self);
}

/// Route only the dedicated incompatible nonempty paragraph-join failure.
/// The command outcome and its existing diagnostic remain authoritative.
pub(super) fn route_editor_outcome<S: RejectionSoundSink>(outcome: &CommandOutcome, sink: &mut S) -> bool {
    if matches!(outcome, CommandOutcome::Failed(EditError::IncompatibleParagraphJoin)) {
        sink.play_rejection_sound();
        true
    } else {
        false
    }
}

pub(super) struct NativeRejectionSound;

impl RejectionSoundSink for NativeRejectionSound {
    fn play_rejection_sound(&mut self) {
        play_native_rejection_sound();
    }
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
fn play_native_rejection_sound() {
    // SAFETY: AppKit's NSBeep has no pointer or object arguments and is called
    // synchronously from the desktop UI command-dispatch thread.
    unsafe {
        objc2_app_kit::NSBeep();
    }
}

#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
fn play_native_rejection_sound() {
    use windows::Win32::UI::WindowsAndMessaging::{MessageBeep, MB_ICONWARNING};

    // SAFETY: MessageBeep takes only a predefined sound-kind flag; the call is
    // synchronous and its result is intentionally ignored as best effort.
    unsafe {
        let _ = MessageBeep(MB_ICONWARNING.0);
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn play_native_rejection_sound() {}

#[cfg(test)]
mod tests {
    use super::{route_editor_outcome, RejectionSoundSink};
    use scriptcraft_commands::{execute_editor_command, Command, CommandOutcome};
    use scriptcraft_core::editing::{EditError, EditingSession, Position, Selection};
    use scriptcraft_core::{Project, Role, ScreenplayElement};

    #[derive(Default)]
    struct FakeSoundSink {
        requests: usize,
    }

    impl RejectionSoundSink for FakeSoundSink {
        fn play_rejection_sound(&mut self) {
            self.requests += 1;
        }
    }

    fn valid<T, E>(result: Result<T, E>) -> T {
        result.unwrap_or_else(|_| unreachable!("feedback routing fixture is valid"))
    }

    fn session(elements: &[(&str, Role)]) -> EditingSession {
        let mut project = valid(Project::new());
        project.elements = elements
            .iter()
            .enumerate()
            .map(|(index, (text, role))| {
                valid(ScreenplayElement::new(
                    &format!("feedback-{index}"),
                    *role,
                    *text,
                ))
            })
            .collect();
        valid(EditingSession::new(project))
    }

    fn execute_and_route(
        session: &mut EditingSession,
        command: &Command,
        sink: &mut FakeSoundSink,
    ) -> CommandOutcome {
        let outcome = execute_editor_command(session, command);
        route_editor_outcome(&outcome, sink);
        outcome
    }

    #[test]
    fn forward_and_backward_incompatible_nonempty_joins_request_one_sound_each() {
        let mut forward = session(&[("left", Role::Action), ("right", Role::Dialogue)]);
        valid(forward.set_selection(Selection::caret(Position { element: 0, offset: 4 })));
        let mut sink = FakeSoundSink::default();
        assert_eq!(
            execute_and_route(&mut forward, &Command::EditDelete, &mut sink),
            CommandOutcome::Failed(EditError::IncompatibleParagraphJoin)
        );
        assert_eq!(sink.requests, 1);

        let mut backward = session(&[("left", Role::Action), ("right", Role::Dialogue)]);
        valid(backward.set_selection(Selection::caret(Position { element: 1, offset: 0 })));
        assert_eq!(
            execute_and_route(&mut backward, &Command::EditorDeleteBackward, &mut sink),
            CommandOutcome::Failed(EditError::IncompatibleParagraphJoin)
        );
        assert_eq!(sink.requests, 2);
    }

    #[test]
    fn successful_joins_empty_removals_and_final_empty_noops_are_silent() {
        let mut sink = FakeSoundSink::default();

        let mut same_role_forward = session(&[("left", Role::Action), ("right", Role::Action)]);
        valid(same_role_forward.set_selection(Selection::caret(Position { element: 0, offset: 4 })));
        assert_eq!(
            execute_and_route(&mut same_role_forward, &Command::EditDelete, &mut sink),
            CommandOutcome::Changed
        );

        let mut same_role_backward = session(&[("left", Role::Action), ("right", Role::Action)]);
        valid(same_role_backward.set_selection(Selection::caret(Position { element: 1, offset: 0 })));
        assert_eq!(
            execute_and_route(&mut same_role_backward, &Command::EditorDeleteBackward, &mut sink),
            CommandOutcome::Changed
        );

        let mut empty_forward = session(&[("left", Role::Action), ("", Role::Dialogue)]);
        valid(empty_forward.set_selection(Selection::caret(Position { element: 0, offset: 4 })));
        assert_eq!(
            execute_and_route(&mut empty_forward, &Command::EditDelete, &mut sink),
            CommandOutcome::Changed
        );

        let mut empty_backward = session(&[("", Role::Dialogue), ("right", Role::Action)]);
        valid(empty_backward.set_selection(Selection::caret(Position { element: 1, offset: 0 })));
        assert_eq!(
            execute_and_route(&mut empty_backward, &Command::EditorDeleteBackward, &mut sink),
            CommandOutcome::Changed
        );

        let mut final_empty = session(&[("", Role::Action)]);
        assert_eq!(
            execute_and_route(&mut final_empty, &Command::EditDelete, &mut sink),
            CommandOutcome::NoOp
        );
        assert_eq!(sink.requests, 0);
    }

    #[test]
    fn unrelated_failure_and_noncollapsed_selection_deletion_are_silent() {
        let mut sink = FakeSoundSink::default();
        assert!(!route_editor_outcome(
            &CommandOutcome::Failed(EditError::InvalidSelection),
            &mut sink
        ));

        let mut selected = session(&[("left", Role::Action), ("right", Role::Dialogue)]);
        valid(selected.set_selection(Selection {
            anchor: Position { element: 0, offset: 2 },
            focus: Position { element: 1, offset: 2 },
        }));
        assert_eq!(
            execute_and_route(&mut selected, &Command::EditDelete, &mut sink),
            CommandOutcome::Changed
        );
        assert_eq!(sink.requests, 0);
    }
}

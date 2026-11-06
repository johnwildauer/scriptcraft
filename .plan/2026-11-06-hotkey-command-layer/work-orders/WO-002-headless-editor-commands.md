# WO-002 — Headless editor command executor

- **Status:** Implemented; user-owned validation pending
- **Initiative:** [Hotkeys and consolidated command layer v1](../hotkey-command-layer-v1.md)
- **Depends on:** [WO-001](WO-001-command-vocabulary.md)
- **Validation owner:** User; no agent-run checks, reviews, or validators

## Single outcome

Execute every semantic screenplay command through one reusable headless function over the real EditingSession, with no desktop window, clipboard, menu, or shortcut dependency.

## Scope

1. Add an editor-command executor in the command package or a narrow core adapter that accepts a mutable [`EditingSession`](../../../crates/scriptcraft-core/src/editing.rs:232) and a typed semantic command.
2. Map text insert/paste, Backspace, forward Delete, Enter, Tab, Role change, Undo, Redo, Select All, and direct Selection updates to their existing session methods exactly once.
3. Preserve canonical UTF-8, current selection, source preservation, Parenthetical policy, strict boundaries, element IDs, revision, dirty state, bounded history, and rollback. Do not reimplement any transition matrix in the executor.
4. Return a typed command outcome that distinguishes semantic mutation, accepted no-op, unavailable history/selection state, invalid parameters, and existing EditError failure. Preserve the original error as the authoritative failure cause.
5. Reject non-editor commands at this boundary without modifying the session. Desktop effects are handled only by WO-003 and later host adapters.
6. Adapt the semantic scenario in [`core_acceptance.rs`](../../../crates/scriptcraft-core/tests/core_acceptance.rs:45) to exercise the command executor, demonstrating that the same commands used by the GUI can create content, transition roles, change roles, and undo/redo headlessly.
7. Author focused command coverage for all 17 explicit Role parameters, Enter/Tab matrix delegation, text/paste distinction, deletion, no-op history behavior, invalid selections, and failed-edit state equality.

## Out of scope

- No egui event parsing, hotkey matching, clipboard access, pointer geometry, visual-line navigation, file operations, or desktop status text.
- No change to Enter/Tab targets, Role behavior, preservation rules, or public transaction invariants.
- No second screenplay model or test-only mutation path.

## Structural acceptance for user review

- The desktop can later invoke the same executor without supplying a window or egui context.
- Each accepted semantic command calls one existing session operation; no command directly edits Project fields.
- Unsupported desktop commands and invalid editor parameters leave every session observable unchanged.
- The command-based acceptance scenario retains the existing Unicode, Role, dirty-state, and history assertions.

## User-owned checks

- Run applicable workspace checks and the command-based headless acceptance/focused scenarios.
- Compare pre/post session state for no-op, unavailable, invalid UTF-8, non-collapsed cross-element Role change, preservation rejection, and bounded-capacity failures.
- Verify one-step undo/redo for Enter, Tab, element-number-equivalent Role changes, typing, paste, and deletion.

## Durable-document impact

Update the screenplay-core/technical blueprint to identify the headless command executor as the single semantic command adapter. Requirements impact is update for non-GUI reachability; the existing AC-WRITING-001.4a matrices remain unchanged.

## WO-002 implementation handoff — 2026-11-06

- Implemented `execute_editor_command(&mut EditingSession, &Command) -> CommandOutcome` in the UI-independent `scriptcraft-commands` package. Supported commands each delegate to one existing session operation; the executor does not modify `Project` fields or implement an Enter/Tab/Role matrix.
- Outcomes distinguish `Changed`, `NoOp`, `Unavailable` (undo history, redo history, or selection), `Rejected` (unsupported command or invalid Selection parameter), and `Failed(EditError)` with the original core error retained. Undo/Redo availability is checked before the single operation call. Unsupported document, clipboard, view, navigation, and desktop-effect commands are rejected without mutation.
- Select All uses the existing session selection setter over the first-to-last body text range. Direct Selection payloads are document- and UTF-8-boundary-validated before conversion and delegation. Core editing semantics, preservation, transactions, history, revision, dirty tracking, and rollback were not changed.
- Relocated the semantic Unicode/Role/dirty/history acceptance scenario from core integration tests to the command package, avoiding a core-to-command dependency cycle. Added focused scenarios for all 17 Role parameters, Enter/Tab delegation over all 17 roles, text/paste, Backspace/forward Delete, accepted no-op/unavailable history, invalid UTF-8 Selection, cross-element Role failure, oversized insertion rollback, unsupported command rejection, and Select All.
- Exact files changed: `crates/scriptcraft-commands/src/lib.rs`, `crates/scriptcraft-commands/src/executor.rs`, `crates/scriptcraft-commands/tests/editor_executor.rs`, `crates/scriptcraft-core/tests/core_acceptance.rs`, `.forge/blueprints/technical-context.md`, `.forge/requirements/features/screenplay-writing-loop.md`, `.plan/2026-11-06-hotkey-command-layer/work-orders/WO-002-headless-editor-commands.md`, and `.plan/2026-11-06-hotkey-command-layer/hotkey-command-layer-v1.md`.
- Implementation evidence is source-level only: executor arms delegate to existing `EditingSession` methods and complete pre/post session comparison classifies accepted no-op versus mutation; the test source expresses the cases listed above. No test, build, check, Clippy, formatting, manual verification, review, or independent validation was run, and no pass/compilation claim is made.
- Outstanding risks/user-owned validation: confirm compilation and all authored scenarios; verify no-op and unavailable state equality, invalid UTF-8 and out-of-range selections, non-collapsed cross-element Role rejection, preservation rejection and bounded-capacity rollback, and one-step undo/redo for Enter, Tab, Role changes, typing, paste, and deletion. The current core acceptance target no longer contains its moved semantic scenario; the command-crate test target must be included in user validation.
- WO-003 integration contract: import `execute_editor_command`, `CommandOutcome`, `CommandRejection`, and `UnavailableReason` from `scriptcraft_commands`. The application dispatcher should route the listed semantic commands through it and translate outcomes into host availability/status/reveal policy without replacing core errors. `EditorSelectionPointer`, navigation, focus, and all desktop/document/effect commands are intentionally outside this executor and remain for later work orders.

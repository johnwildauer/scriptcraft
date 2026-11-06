# WO-004 — Focused hotkeys and element-number commands

- **Status:** Implemented; user-owned validation pending
- **Initiative:** [Hotkeys and consolidated command layer v1](../hotkey-command-layer-v1.md)
- **Depends on:** [WO-003](WO-003-desktop-command-dispatcher.md)
- **Validation owner:** User; no agent-run checks, reviews, or validators

## Single outcome

Route focused screenplay input through the consolidated dispatcher and enable unmodified Enter/Tab plus all requested Mac and Windows element-number defaults without changing semantic editing behavior.

## Scope

1. Extract the logical shortcut model from [`menu_items.rs`](../../../apps/scriptcraft/src/menu_items.rs:67) into a shared desktop [`shortcuts.rs`](../../../apps/scriptcraft/src/shortcuts.rs) module. Retain Primary, literal Control, Option/Alt, Shift, existing punctuation/letters, and add logical top-row Digit 0 through Digit 9.
2. Keep menu/native shortcut adapters as consumers of that shared model. Do not add element bindings to MENU_CATALOG or the native menu command map; editor-local bindings must not change native macOS menu ownership.
3. Add a deterministic editor hotkey catalog/resolver with a platform parameter for headless coverage. Map unmodified Enter and Tab to their command IDs, and map number chords to `editor.element.change` with the exact Role table in the initiative.
4. On macOS, additional elements use Command + Control + digit. On Windows, they use Control + Shift + digit. Base elements use Primary + digit on both. Require exact modifiers and reject extra Option/Alt, Shift, Control, or Command not present in the binding.
5. In [`dispatch_surface_events()`](../../../apps/scriptcraft/src/script_view.rs:680), translate ordinary Text, OS Paste, and IME Commit into typed editor commands; translate Backspace/Delete and recognized editor keys through the same resolver. Dispatch synchronously in original event order through the consolidated application dispatcher.
6. Preserve IME preedit/commit state, duplicate Text-before-Commit suppression, control-character filtering, key-release handling, key repeat behavior, and native input diagnostics. Suspend hotkey recognition while IME is active or the raw frame contains IME events.
7. Keep the existing preceding-frame surface-focus gate in [`handle_surface_input()`](../../../apps/scriptcraft/src/script_view.rs:770) and the migration-modal exclusion in [`update()`](../../../apps/scriptcraft/src/main.rs:1331). Editor hotkeys do not become global application shortcuts.
8. Use the shared mutation/status/repaint/focus adapter for accepted results. Role-change rejection must leave project, selection, history, dirty state, and revision unchanged.
9. Replace the current modifier-insensitive Enter/Tab key arms with exact unmodified recognition so modified chords do not accidentally invoke paragraph transitions.

## Exact hotkey acceptance matrix

| Binding group | Expected result |
|---|---|
| Enter with no modifiers | Existing EditingSession Enter operation |
| Tab with no modifiers | Existing EditingSession Tab operation |
| Command/Control + 0 through 9 | General, Scene Heading, Action, Character, Parenthetical, Dialogue, Transition, Shot, Cast List, New Act respectively |
| Mac Command + Control + 8/9/0/1/2/3/4 | Sequence, End of Act, Summary, Outline 1, Outline 2, Outline 3, Note respectively |
| Windows Control + Shift + 8/9/0/1/2/3/4 | Sequence, End of Act, Summary, Outline 1, Outline 2, Outline 3, Note respectively |
| Wrong platform extended chord, extra modifier, numpad, key release | No editor command from that event |
| Unfocused surface, active IME, migration modal | No editor command dispatch |
| Same target Role or preserved-source conflict | Existing accepted no-op or transactional rejection; no parallel hotkey policy |

## Out of scope

- No Find/search or any other reference-only command; no numpad aliases, shortcut customization, persisted bindings, command palette, or new menu items.
- No Enter/Tab semantic matrix changes, automatic scene-heading detection, Scene Heading subfield editing, SmartType, or Character-to-Parenthetical policy change.
- No navigation/pointer migration beyond what is needed to leave those existing paths working; WO-005 owns their command consolidation.

## User-owned checks

- Review resolver coverage for both logical platforms independently of the host OS, including all 17 roles, exact modifiers, top-row digits, key release/repeat, and conflict rejection.
- In the running editor, type text followed immediately by Enter/Tab and verify event order, caret position, role, Unicode text, dirty state, and one-step undo/redo.
- Exercise every number binding from a focused editor, including Parenthetical generation and preservation-rejection cases. Confirm unfocused/IME/modal input does not mutate.
- On macOS, verify native menu accelerators still execute exactly once and element chords work without native menu rows. On Windows, verify in-window menu shortcuts and element chords do not double dispatch.
- Run applicable workspace checks and focused headless resolver/command scenarios.

## Durable-document impact

Add the exact platform element-binding and focused-routing acceptance criteria to the writing-loop requirements and the shared shortcut/editor adapter boundary to the technical context. Existing AC-WRITING-001.4a semantics remain unchanged.

## WO-004 implementation handoff — 2026-11-06

- Added `apps/scriptcraft/src/shortcuts.rs` as the shared logical shortcut model and focused-editor resolver. It defines logical top-row Digit 0–9 values and an explicit platform input, a 17-role binding table, exact modifier matching, unmodified Enter/Tab, and focused-key mappings for Backspace/forward Delete. The resolver also requires the event's physical key to match its logical digit so numpad aliases are not introduced.
- Moved `LogicalShortcut` and `ShortcutKey` out of `menu_items.rs`; menu presentation, in-window accelerator matching, and the native key adapter consume the shared definitions. Existing command-only menu matching, disabled-placeholder exclusion, and native-vs-in-window accelerator ownership remain separate from editor-local element bindings. No number binding was added to `MENU_CATALOG` or the native command map.
- `script_view.rs` now turns Text, OS Paste, IME Commit, Backspace/Delete, unmodified Enter/Tab, and recognized platform element chords into typed commands and dispatches each synchronously while iterating the original event list. Existing Text/Commit duplicate suppression, control-character filtering, IME preedit state, key-release handling, repeat-as-pressed-events, native input diagnostics, preceding-frame focus gate, and migration-modal exclusion are retained. Arrow/Home/End navigation still uses its existing path for WO-005.
- `app_commands.rs` routes those new semantic commands through the WO-002 executor and the WO-003 dispatcher mutation/status/reveal/repaint/focus adapter. Enter/Tab still delegate to the existing session methods; each role binding sends `editor.element.change` with a typed Role. Same-role no-op and preservation rejection remain core/executor-owned; no core editing semantics changed.
- Updated the technical-context and writing-loop records with the shared shortcut/editor boundary and the top-row/focused-routing criteria. The detailed contract was already present in AC-WRITING-012; this update clarifies top-row physical-key matching and records the implementation boundary without changing AC-WRITING-001.4a.
- Exact files changed: `apps/scriptcraft/src/shortcuts.rs`, `apps/scriptcraft/src/main.rs`, `apps/scriptcraft/src/menu_items.rs`, `apps/scriptcraft/src/menu.rs`, `apps/scriptcraft/src/macos_menu.rs`, `apps/scriptcraft/src/app_commands.rs`, `apps/scriptcraft/src/script_view.rs`, `.forge/blueprints/technical-context.md`, `.forge/requirements/features/screenplay-writing-loop.md`, this work order, and `.plan/2026-11-06-hotkey-command-layer/hotkey-command-layer-v1.md`. No manifest or lockfile change was needed.
- Source-level implementation evidence: the resolver table contains ten base and seven extended role rows; the editor event loop constructs command invocations and calls the desktop dispatcher inline, not the pending FIFO; the desktop dispatcher forwards text/paste/deletion/paragraph/role commands to `execute_editor_command`; menu adapters still resolve only real command rows. Resolver test cases were authored in `shortcuts.rs` for both explicit platforms, role coverage, modifier rejection, paragraph modifier strictness, and digit-key rejection, but were not executed.
- Risks and limits: no compilation or runtime behavior is established. User-owned review should confirm egui physical-key values for top-row versus keypad input on both OSes, native key-code integration, and exact modifiers on real devices. Forward Delete retains the existing desktop command's fresh target-availability check. Focused routing intentionally does not migrate arrow/Home/End navigation, pointer/selection, or standalone focus actions; WO-005 owns those. Native menu ownership, in-window preview routing, and no-double-dispatch behavior remain user-unverified.
- User-owned validation: review every role mapping independently for Mac and Windows, exact/extra/wrong-platform modifiers, top-row versus numpad, key releases and repeat; exercise Enter/Tab modified and unmodified; type Unicode/text immediately before Enter/Tab and verify event ordering, caret, role, dirty state, and one-step undo/redo; exercise every element binding, Parenthetical generation and transactional preservation rejection; confirm focused/unfocused, IME active/frame, duplicate Text/Commit, and migration-modal gates; verify existing native macOS and Windows menu accelerators still execute once without element rows. Continue all applicable checks and interaction validation under user ownership only.
- WO-005 handoff: preserve the synchronous focused-event path and its focused/migration/IME gates. Migrate only the still-direct Selection, pointer, navigation, and explicit focus operations there; retain current geometry/navigation semantics and route resulting semantic Selection through the existing typed executor/dispatcher boundary. Do not move editor input into the menu FIFO or change Enter/Tab/Role policy.

No tests, builds, checks, Clippy, formatting, manual verification, review, or independent validation were run. Compilation and all behavior remain unverified; user validation remains required.

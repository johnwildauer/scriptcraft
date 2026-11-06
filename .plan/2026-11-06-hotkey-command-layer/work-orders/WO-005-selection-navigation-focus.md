# WO-005 — Selection, layout navigation, pointer, and focus commands

- **Status:** Implemented; user-owned validation pending
- **Initiative:** [Hotkeys and consolidated command layer v1](../hotkey-command-layer-v1.md)
- **Depends on:** [WO-004](WO-004-focused-hotkeys.md)
- **Validation owner:** User; no agent-run checks, reviews, or validators

## Single outcome

Route every existing screenplay selection, caret-navigation, pointer-selection, and editor-focus action through typed commands while retaining the current layout geometry and transient navigation rules.

## Scope

1. Remove remaining direct session calls for Select All and direct Selection changes from desktop command adapters; delegate to the WO-002 headless executor.
2. Convert [`move_horizontal()`](../../../apps/scriptcraft/src/script_view.rs:796), [`move_vertical()`](../../../apps/scriptcraft/src/script_view.rs:853), and [`move_to_edge()`](../../../apps/scriptcraft/src/script_view.rs:941) into command handlers selected by typed direction/edge and extend parameters.
3. Keep layout-aware vertical calculation, current immutable layout input, preferred horizontal goal retention/reset, Shift anchor behavior, short-line clamping, document/page boundaries, and UTF-8 source mapping unchanged. These handlers may read desktop layout but must commit Selection only through the semantic executor.
4. Convert pointer activation into `editor.focus` and `editor.selection.pointer` commands. Pointer coordinates are first mapped through the existing page/layout hit-test adapter to a semantic Position; command payloads never contain egui rectangles or pointer coordinates as screenplay state.
5. Preserve drag-start anchor creation, drag-update extension, release/collapse behavior, title-page exclusion, nearest-page selection, and pointer-driven focus. Pointer phase is typed and bounded; invalid phases/positions reject without mutation.
6. Retain the recorded editor surface ID and request egui focus only in the desktop focus adapter. Command execution targets the current session regardless of transient focus.
7. Preserve conditional reveal, repaint, rendered-revision invalidation, preferred-vertical-goal reset, and native input diagnostics through shared command outcomes.
8. Remove obsolete direct helper paths after all callers use commands; read-only session projections remain allowed for layout, availability, and presentation.

## Out of scope

- No new word/document navigation, Page Up/Down behavior, numpad handling, selection feature, pointer algorithm, or geometry redesign.
- No change to FDX TextState Selection persistence, zoom, scrolling, layout indexes, pagination, or caret blink.
- No direct mutation of Project outside EditingSession.

## Structural acceptance for user review

- Arrow/Home/End key events, pointer click/drag, Select All, and focus activation all produce stable commands and pass through the consolidated dispatcher.
- Layout-aware handlers return a desired semantic Selection; the headless executor remains the only commit path.
- A failed Selection update leaves session and transient drag/navigation state consistent with current behavior.
- No enabled editor interaction retains a parallel direct session-mutation route.

## User-owned checks

- Exercise Left/Right, wrapped-line Up/Down, Home/End, Shift extension, document/page edges, Unicode boundaries, pointer click/drag, and focus loss/regain.
- Verify preferred horizontal goal retention/reset, caret reveal, selection rendering, and one-step editing history behavior after navigation.
- Run applicable focused and workspace checks; inspect source for direct Selection writes outside the headless executor.

## Durable-document impact

Update the desktop input blueprint with typed navigation/pointer/focus commands and the semantic Selection commit boundary. Requirements impact is update for uniform action routing; AC-WRITING-001.4b navigation behavior itself remains unchanged.

## WO-005 implementation handoff — 2026-11-06

- `script_view.rs` now turns Arrow Left/Right, Up/Down, and Home/End into typed navigation commands and dispatches them inline while iterating the original focused event sequence. Existing preceding-frame focus and migration-modal gates, the WO-004 hotkey/IME behavior, and menu accelerator ownership were not moved or redesigned.
- The three layout-aware navigation handlers now compute and return a desired core Selection for typed direction/edge and extend parameters. Existing visual-line layout lookup, immutable layout use, UTF-8/source mapping, selection collapse and Shift anchor rules, short-line clamping, page/document boundaries, and preferred vertical goal handling remain in those algorithms. The desired Selection is encoded and committed only by `execute_editor_command` through `editor.selection.set`; navigation execution does not call `EditingSession::set_selection` directly. Transient vertical-goal changes are restored if command-parameter conversion rejects the selection.
- Pointer interaction retains the existing page/layout hit test, title-page exclusion, nearest body page, pointer-activation focus, and click/drag phase decisions. It issues `editor.focus` before any pointer-selection command. Pointer commands contain only a checked semantic Position and typed Press/Drag/Release phase. The dispatcher validates the position before calculating a desired Selection, applies drag-anchor state only after an accepted executor outcome, and commits the Selection via the same headless executor. Invalid positions reject without changing session or drag/navigation state. The desktop focus adapter alone uses the recorded surface ID and requests egui focus.
- Select All remains the existing typed `edit.select-all` route through the WO-002 executor; explicit `editor.selection.set` is now also dispatched to that executor. Accepted results continue through the shared status/reveal/repaint adapter, with pointer selection retaining its prior status and non-reveal behavior. No layout geometry, Enter/Tab policy, Role policy, core editing algorithm, persistence, or menu ownership was changed.
- Durable updates record synchronous navigation/pointer/focus routing and the semantic Selection boundary in the technical context and AC-WRITING-012.4. AC-WRITING-001.4b navigation behavior is unchanged.
- Exact files changed: `apps/scriptcraft/src/script_view.rs`, `apps/scriptcraft/src/app_commands.rs`, `.forge/blueprints/technical-context.md`, `.forge/requirements/features/screenplay-writing-loop.md`, this work order, and `.plan/2026-11-06-hotkey-command-layer/hotkey-command-layer-v1.md`.
- Source-level evidence only: focused Arrow/Home/End event mapping constructs typed `editor.navigation.*` commands and dispatches synchronously from the event loop; navigation handlers return semantic Selection values; the dispatcher turns those into `editor.selection.set` executor calls. Pointer coordinates are consumed by the existing hit-test adapter, then only a typed Position/phase enters dispatch; the focus command delegates to the recorded-surface adapter. The existing Select All branch continues to invoke the executor. No runtime or compilation evidence is asserted.
- Risks and user-owned validation: no compilation, behavior, or platform event result is established. Exercise left/right selection collapse and Unicode scalar boundaries; wrapped-line Up/Down with retained horizontal goal, short-line clamping, Shift anchor, document/page boundaries and boundary no-op resets; Home/End; pointer press/drag/click/release, title-page exclusion, nearest-page choice, invalid position rejection, focus loss/regain, reveal/repaint, rendered revision, and status behavior. Confirm Select All and all selection updates have one executor commit route; verify Enter/Tab/Role, text/IME order and gates, menu/native accelerator ownership, and modal exclusion remain intact. User owns all review and validation; no tests, builds, checks, Clippy, formatting, manual verification, review, or independent validation were run.

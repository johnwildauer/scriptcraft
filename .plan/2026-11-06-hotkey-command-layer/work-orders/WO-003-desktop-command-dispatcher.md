# WO-003 — Unified desktop dispatcher and existing menu commands

- **Status:** Implemented; user-owned validation pending
- **Initiative:** [Hotkeys and consolidated command layer v1](../hotkey-command-layer-v1.md)
- **Depends on:** [WO-002](WO-002-headless-editor-commands.md)
- **Validation owner:** User; no agent-run checks, reviews, or validators

## Single outcome

Replace the current parameterless desktop command path with the shared typed command vocabulary and one ordered application dispatcher used by menus, native menus, toolbar items, and future input adapters.

## Scope

1. Evolve [`app_commands.rs`](../../../apps/scriptcraft/src/app_commands.rs) into the consolidated desktop dispatcher. Accept a typed command invocation and origin, recompute fresh availability, and return a typed outcome.
2. Rename/generalize the menu-only pending FIFO in [`menu.rs`](../../../apps/scriptcraft/src/menu.rs:177) so it carries command invocations from all deferred producers. Detach each batch and dispatch in activation order as today.
3. Store stable command IDs or parameterless factories in [`menu_items.rs`](../../../apps/scriptcraft/src/menu_items.rs) and [`toolbar_items.rs`](../../../apps/scriptcraft/src/toolbar_items.rs). Materialize typed commands at activation; never serialize or persist the catalog itself as screenplay state.
4. Route existing New, Open request, Close, Save, Save As request, PDF export, Undo, Redo, Cut, Copy, Paste, Delete, and Select All through the dispatcher. Preserve the existing file/export/clipboard adapters temporarily behind that boundary until WO-006 separates direct-path and dialog effects completely.
5. Route toolbar Role selection through `editor.element.change` instead of the direct session call in [`change_role_from_toolbar()`](../../../apps/scriptcraft/src/toolbar.rs:592).
6. Map native macOS item IDs to stable command IDs and enqueue invocations in the existing event handoff. Keep native enablement synchronized from the shared availability projection.
7. Retain exact menu shortcut ownership: native macOS while installed, in-window Windows otherwise. Preserve the Redo Windows alias and exclude every placeholder binding from recognition.
8. Use the common mutation/status/reveal adapter for accepted editor results. Restore recorded editor focus after Edit commands as today; command target remains the application session, not transient popup focus.
9. Recheck modal and selection/history availability at dispatch. A queued command that became unavailable is rejected without mutation.

## Out of scope

- No new hotkeys, editor event migration, view/status/timer changes, or new file/clipboard behavior.
- No functional placeholder, menu redesign, toolbar layout change, or native accelerator ownership change.
- No fix for the pre-existing New dirty-session consent or Close confirmation discrepancies.

## Structural acceptance for user review

- One desktop dispatcher is the only activation path for all existing File/Edit commands and toolbar Role changes.
- Menu clicks, native events, toolbar actions, and owned menu accelerators converge on the same command IDs and availability projection.
- Static catalogs remain presentation-independent and disabled placeholders cannot construct an invocation.
- Deferred commands preserve FIFO order and one physical activation cannot execute twice through native and in-window paths.

## User-owned checks

- Exercise every existing File/Edit command from each visible enabled surface, including macOS dual preview and Windows in-window menus where available.
- Verify no double New/Save/Paste/Undo, correct Redo alias handling, fresh enablement, modal blocking, and editor focus restoration.
- Confirm toolbar Role selection retains the existing transactional status/error/reveal behavior.

## Durable-document impact

Update the desktop application-menu technical context from AppCommand-specific dispatch to the general typed command queue/dispatcher. Requirements impact is update for unified activation and availability; no menu presentation requirement changes.

## WO-003 implementation handoff — 2026-11-06

- Replaced the parameterless `AppCommand` activation path with typed `scriptcraft_commands::Command` invocations carrying `CommandOrigin`. The dispatcher returns a typed desktop result and recomputes modal, history, selection, and delete-target availability at dispatch time.
- The 13 enabled File/Edit catalog rows and toolbar command items now store stable `CommandId` values. Parameterless command materialization occurs at activation; the toolbar Role selector builds `EditorElementChange { role }` at activation. Disabled placeholders remain behavior-less and excluded from shortcut matching, queueing, native command mapping, and dispatch.
- Menu clicks, native macOS item IDs, and owned in-window menu accelerators enqueue typed invocations into the generalized detached FIFO with `Menu`, `NativeMenu`, and `Shortcut` origins respectively. Native IDs map only to stable command IDs, and native enablement uses the same availability projection. Toolbar actions and Role changes use the same dispatcher with `Toolbar` origin.
- Existing File New/Open request/Close/Save/Save As request/PDF export and clipboard adapters remain behind the typed dispatcher. Undo, Redo, Delete, Select All, clipboard Paste insertion, Cut's post-write deletion, and toolbar Role changes delegate their semantic operation to `execute_editor_command`. Changed/no-op results retain existing success status, reveal, focus, and repaint behavior; unavailable/rejected outcomes do not mutate or replace the current status, and `Failed(EditError)` is passed to the existing editor-error display while retaining the original error in the returned typed outcome.
- Effect handlers remain the existing void-returning desktop adapters. Their dispatcher result is explicitly `EffectDispatched`, not a claim of effect success; their existing status/error reporting remains authoritative until later effect-adapter work. New's pre-existing dirty-session consent discrepancy is unchanged. Pointer selection/navigation/focus, direct-path document APIs, and other later-order command families remain outside this work order. Native predefined macOS About/Hide/Hide Others/Show All/Quit services remain OS-owned.
- Durable updates: the technical context now records the implemented typed menu/toolbar dispatcher and distinguishes later adapter work; AC-WRITING-012.4 now identifies typed queued origins and shared toolbar dispatch. No menu presentation or shortcut ownership policy changed.
- Exact files changed: `apps/scriptcraft/Cargo.toml`, `apps/scriptcraft/src/app_commands.rs`, `apps/scriptcraft/src/main.rs`, `apps/scriptcraft/src/menu.rs`, `apps/scriptcraft/src/menu_items.rs`, `apps/scriptcraft/src/macos_menu.rs`, `apps/scriptcraft/src/toolbar.rs`, `apps/scriptcraft/src/toolbar_items.rs`, `.forge/blueprints/technical-context.md`, `.forge/requirements/features/screenplay-writing-loop.md`, this work order, and `.plan/2026-11-06-hotkey-command-layer/hotkey-command-layer-v1.md`. `Cargo.lock` already contained the desktop package's `scriptcraft-commands` dependency edge, so no lockfile edit was needed.
- Evidence is source-level only: catalog behavior now contains `CommandId`s; queue entries carry `Command` plus origin; the dispatcher calls the WO-002 executor for supported semantic commands; toolbar Role no longer calls the session directly; native menu synchronization and dispatch map the same IDs and availability. No tests, build, check, Clippy, formatting, launch, manual verification, review, or independent validation was run. No compilation or behavior pass is claimed.
- User-owned validation remains required: confirm compilation and exercise every enabled File/Edit menu, native menu, in-window shortcut, toolbar command, and toolbar Role action; verify FIFO order and no duplicate activation; verify platform accelerator ownership and the Windows Redo alias; test fresh modal/history/selection availability and focus restoration; inspect status/error/reveal behavior including core failures, clipboard failures, and queued commands that become unavailable. Also review the technical-context/requirement updates and retain the existing New dirty-session discrepancy as a known issue.
- Handoff to WO-004: use `Command`/`CommandOrigin` and `dispatch_command` as the desktop activation boundary. This order does not migrate editor text, IME, Enter/Tab, deletion key events, or navigation; those remain later-order responsibilities. Preserve the established menu accelerator owner and do not move focused editor commands into the deferred FIFO.

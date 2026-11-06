# WO-008 — Durable contracts and user-validation handoff

- **Status:** Implemented; user-owned validation and acceptance pending
- **Initiative:** [Hotkeys and consolidated command layer v1](../hotkey-command-layer-v1.md)
- **Depends on:** [WO-007](WO-007-view-status-timer-commands.md)
- **Validation owner:** User; no agent-run checks, reviews, validators, or manual verification

## Single outcome

Reconcile durable product/architecture records with the completed command and hotkey implementation, prove the consolidation boundary by source inventory, and give the user a complete validation handoff without claiming agent validation.

## Scope

1. Update [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md) with the approved stable command IDs/typed parameters, unified dispatch, headless semantic execution, exact hotkey table, focus/modifier/IME/modal rules, ordering, availability, and preservation requirements.
2. Update [`product-overview.md`](../../../.forge/requirements/product-overview.md) to distinguish the internal command architecture from the still-out-of-scope MCP/network product feature and command palette.
3. Update [`technical-context.md`](../../../.forge/blueprints/technical-context.md) with the new command crate, desktop dispatcher, headless editor executor, shared shortcut registry, queue versus focused-event routing, and host effect adapters.
4. Update [`project-manifest.md`](../../../.forge/project-manifest.md) with dated implementation-submission facts, new workspace member/dependency observations, branch/change evidence supplied by the implementer, and explicit absence of agent validation.
5. Perform a source inventory—not a behavioral validation—of direct session mutation, viewport effects, clipboard access, file dialogs, timer/status state writes, and enabled UI activation. Every remaining direct path must be either a named command executor/effect adapter or a documented read-only/framework-owned boundary.
6. Reconcile the initiative plan and every work order with exact changed paths and implementation handoffs. Preserve prior evidence as historical; do not mark any order accepted or validated without user evidence.
7. Produce a user-owned validation matrix covering command serialization, headless semantics, every menu/toolbar action, both logical hotkey platforms, native macOS and Windows ownership, Unicode/IME, ordering, role preservation, navigation, file/clipboard failures, view/status/timer actions, and save/reopen.
8. Record any residual exception, especially native predefined macOS Application-menu services and framework-owned presentation bookkeeping. No exception may conceal an enabled screenplay mutation outside the command layer.

## Required user-validation scenarios

- **Command wire:** unknown ID, malformed payload, every Role, Unicode text, paths, Selection, origin, and round-trip stability.
- **Headless semantics:** command-based typing/paste/deletion/Enter/Tab/Role/history/Selection, no-op and transactional failure equality.
- **Hotkeys:** all 17 base/extended bindings for Mac and Windows logical platforms, exact modifiers, top-row digits only, focus, IME, modal, key release/repeat, event order, and undo/redo.
- **Desktop integration:** one activation per menu/native/toolbar/shortcut action, availability and focus restoration, no placeholder activation, and no dual-backend double dispatch.
- **Editing/navigation:** pointer click/drag, arrows/Home/End, Shift selection, wrapped lines, Unicode boundaries, reveal, and source preservation.
- **Document/effects:** direct and picker paths, migration choices, clipboard failure ordering, Close, PDF export, failed-save rollback, and native predefined-service exception.
- **View/timer:** zoom/pinch/scroll interaction, appearance, Focus Mode state gates, every timer transition, notifications, and TextState zoom persistence.
- **Workspace:** user-run format, check, build, Clippy, and tests using the repository's recorded command inventory, plus native macOS and Windows interaction where available.

## Acceptance test

Given the completed implementation and durable records, when the user compares every enabled ScriptCraft-owned interaction with the command inventory and runs the validation matrix, then each action has one stable typed command route, semantic operations work headlessly, hotkeys match the approved table, existing safety/behavior is preserved, exceptions are explicit, and every result is recorded without inferring acceptance from implementation submission.

## Out of scope

- No agent execution of the listed checks or scenarios, no independent review/validation artifact, and no acceptance claim.
- No follow-on CLI/MCP implementation, shortcut customization, new hotkey feature, or semantic redesign.
- No destructive Git action, remote operation, commit, merge, or closeout without separate explicit user authorization.

## WO-008 implementation handoff — 2026-11-06

### Durable contract reconciliation

- Updated the writing-loop requirement with the complete 41-ID internal inventory grouped by typed
  payload and execution owner, the exact 17-role platform hotkey matrix, ordering/focus/modifier/IME/
  modal and availability rules, headless execution boundary, effect-result distinction, UTF-8 path
  limitation, and preservation/non-persistence constraints. Existing Enter/Tab semantic matrices
  remain unchanged.
- Updated the product overview to distinguish the internal command contract and desktop dispatcher
  from a user-facing command palette, CLI, MCP/network service, macro UI, or plugin host; all remain
  out of product scope.
- Updated the technical context with the workspace command crate, consolidated state through WO-007,
  direct-action inventory, and host/framework/OS boundaries.
- Updated the manifest with dated implementation-submission facts, manifest/dependency observations,
  branch attribution supplied by the user, and explicit absence of agent validation.
- Updated the initiative status and recorded this WO-008 handoff. Earlier work-order changed-file
  lists remain historical and unchanged; all remain user-unvalidated and none is accepted.

### Source inventory and bypass disposition

This was scoped source/documentation inspection only. It was not independent review, validation,
execution, or evidence of runtime behavior. Inspected routes included desktop dispatch and semantic
executor calls; focused editor event mapping; pointer/navigation and Selection commits; menu/toolbar
activations; status intents; timer commands; document pickers, direct paths, migration, clipboard and
viewport effects; and the framework/native input paths.

The inventory found one source-level semantic Selection write outside the headless executor: Open
restored the imported TextState Selection by calling the replacement session setter directly in
`activate_opened_project()`. This within-scope bypass was routed through the existing typed
`editor.selection.set` command and `execute_editor_command()` before replacing the live session. An
invalid restored selection retains the existing fallback to caret 0,0. The correction does not alter
core editing semantics.

No other enabled ScriptCraft-owned screenplay mutation bypass was identified in the scoped source
inventory. Current action routes converge on the dispatcher; semantic editor changes and Selection
commits use the headless executor. Remaining direct effects/writes are part of named dispatcher or
host adapters: file/storage and fixed-path PDF operations; clipboard; migration resolution; viewport
Close/fullscreen/imported restore; focus requests; zoom/appearance and timer transitions; notification
requests/results; and existing status/error updates. Framework-owned state includes IME preedit,
raw-scroll delivery, pointer geometry, popup placement/rectangles and `just_opened`, focus-mode
observation of native fullscreen, clock reconciliation, repaint/caret/layout publication, and
diagnostics. These are bookkeeping/host observations, not separate enabled screenplay mutation paths.

Explicit residuals:

- Native predefined macOS About, Hide, Hide Others, Show All, and Quit remain OS-owned services. This
  is the documented native boundary; no new unsafe AppKit route was added.
- The typed document-path contract is UTF-8; a native picker path without a UTF-8 representation is
  rejected rather than lossily converted.
- Existing File New dirty-session consent and File Close confirmation discrepancies remain unchanged
  from earlier handoffs and were not expanded or redesigned by WO-008.
- `EffectDispatched` remains routing-only. For desktop effect handlers, their existing status/error
  channel is authoritative; notification OS acceptance/pending is not delivery confirmation.

### Exact changed files and repository facts

Exact WO-008 changed files: `apps/scriptcraft/src/main.rs`,
`.forge/requirements/features/screenplay-writing-loop.md`,
`.forge/requirements/product-overview.md`, `.forge/blueprints/technical-context.md`,
`.forge/project-manifest.md`,
`.plan/2026-11-06-hotkey-command-layer/hotkey-command-layer-v1.md`, this work order, and
[`WO-008-user-validation-handoff.md`](WO-008-user-validation-handoff.md). No package dependency,
workspace membership, or lockfile change was made by WO-008.

Manifest observations read for this handoff: the root Cargo workspace lists the desktop, command, and
core packages; the desktop package depends on local `scriptcraft-commands` and `scriptcraft-core`; the
command package depends on local `scriptcraft-core` and exact Serde 1.0.229 with derive. These are
manifest/source observations, not build or compilation results. Branch attribution is the user-supplied
existing initiative branch `dev/2026-11-06-hotkey-command-layer`; no Git inspection was performed to
independently establish checkout state. No commit or push was made.

### User-owned validation handoff

The complete scenario matrix and blank evidence fields are at
[`WO-008-user-validation-handoff.md`](WO-008-user-validation-handoff.md). It includes command-wire
serialization, headless semantics and rollback, both logical hotkey platforms, native macOS and
Windows/menu ownership, enabled menu/toolbar actions and availability, focus/IME/order, selection and
navigation, document/clipboard/modal/effect failures, view/status/timer/notification behavior,
preservation and save/reopen, and the workspace command inventory from the project manifest. It is a
handoff, not a validation artifact; no scenario result is recorded.

No tests, builds, checks, Clippy, formatting, application launch, manual verification, review, or
independent validation were run. No pass, compilation result, behavioral result, or acceptance is
claimed. User review, validation evidence, and acceptance remain pending.

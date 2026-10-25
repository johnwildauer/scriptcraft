# Work Order: WO-009 — Unified Live Editing Surface

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop editing-surface implementer (one fresh session)
- **Phase/order:** 9 of 10; depends on WO-004, WO-005, and WO-008
- **Initiative:** [`../2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Generated filename:** `WO-009-unified-live-editing-surface.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-009.md`](../validations/validation-WO-009.md) — failed; resolved through the passed lettered fix validation
- **Fixes:** [`WO-009a-preserve-pointer-drag-selection-anchor.md`](WO-009a-preserve-pointer-drag-selection-anchor.md) — passed [`../validations/validation-WO-009a.md`](../validations/validation-WO-009a.md)
- **Implementer session/agent ID:** `code-session-2026-10-25-wo-009`
- **Session exception:** `none`

## Summary

Replace the current split/duplicated desktop editing interaction with one primary
live document editing surface. The surface must render the fixed page layout by
default while accepting direct typing, caret movement, selection, deletion,
backspace, and role-aware edits through the existing `EditingSession` and layout
source-mapping APIs. This is a scope correction to the desktop adapter, not a
claim that the prior WO-005/WO-008 validation covered these behaviors.

## In scope

- One primary document surface with page layout as the default visual
  presentation and a live caret/selection representation.
- Direct typing routed to the active core selection, including replacement,
  deletion, backspace, Enter/split, and caret movement at valid UTF-8 boundaries.
- Selection and hit-testing routed through `ScreenplayLayout` source mappings and
  `EditingSession::set_selection`, `insert`, `delete`, and `backspace`.
- Role-aware editing using the existing role controls and explicit transition
  policy, with undo/redo and visible bounded errors preserved.
- Removal of the duplicate per-element `TextEdit`/text-entry interaction and
  removal of a separate continuous-versus-page-preview editing split. Any
  secondary diagnostic rendering must not be an alternate editing surface or
  second source of truth.
- Focused app tests and manual interaction evidence for direct typing, caret,
  selection, deletion, role transitions, undo/redo, wrapping, pagination, and
  resize/scroll behavior.

## Out of scope

- Changes to semantic model rules, UTF-8 safety, layout algorithms, page profile,
  persistence bytes, recovery rotation, export formats, or core recovery APIs.
- A second editor, a continuous/page-preview mode switch, per-element text-entry
  controls, rich formatting, collaboration, clipboard history, or broad desktop
  polish.
- User-selected file dialogs and named Open/Save/Save As; those belong to WO-010.

## Requirements

- **REQ-WRITING-LOOP-001:** “As an individual screenwriter, I want to create and edit an ordered screenplay with explicit element roles, so that my draft retains screenplay meaning while I write.”
- **AC-WRITING-001.1:** Given a new project, the system shall provide a valid editable starting element and title-page metadata without requiring visual formatting tricks.
- **AC-WRITING-001.2:** Given an element, the system shall preserve its explicit role and canonical UTF-8 text independently of capitalization, spaces, tabs, indentation, or font appearance.
- **AC-WRITING-001.3:** Given a valid selection, the system shall support insertion, deletion, split, join, role change, undo, and redo while preserving valid text boundaries and element order.
- **AC-WRITING-001.4:** Given a role transition such as scene heading → action or character → dialogue, the system shall apply an explicit, testable transition policy and make the operation undoable.
- **AC-WRITING-001.5:** Given malformed or out-of-bound input, the system shall return a bounded error without corrupting the project or session state.
- **REQ-WRITING-LOOP-002:** “As an individual screenwriter, I want continuous writing and fixed-page preview to agree about semantic content and layout inputs, so that I can write comfortably and inspect a predictable page result.”
- **AC-WRITING-002.1:** Given the same project and fixed page profile, the system shall derive stable page structure and source mappings independent of window zoom.
- **AC-WRITING-002.4:** Given a document position and rendered geometry, the system shall support a safe caret/hit-test round trip at valid text boundaries.

## Blueprints

- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) — maintain one shared semantic-to-layout display path; the UI must not introduce a parallel editor buffer or editing mode.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) — the shell owns input, caret/selection rendering, and page presentation while delegating semantic mutations to core.

## Implementation plan

1. Replace the per-element label-plus-text-entry interaction with one page-layout
   document surface that tracks the core selection and paints caret/selection
   state from source mappings.
2. Translate keyboard/IME input, selection, deletion, backspace, Enter, and role
   actions into the existing `EditingSession` APIs; resynchronize geometry after
   every accepted or rejected operation.
3. Remove the continuous/page-preview editing split, make page layout the default
   live presentation, and add focused tests/manual evidence for the full editing
   interaction boundary.

## Files and systems

- **Create:** Focused app interaction tests or small app-local input/rendering helpers as needed.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs), app manifest only if required by the chosen input surface, this WO, execution log, and the desktop/writing-loop blueprints if verified boundary wording changes.
- **Avoid changing:** `crates/scriptcraft-core` semantic/editing/layout algorithms unless a narrowly evidenced adapter contract issue requires it; persistence, recovery, and exports.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo build --workspace`; `cargo test --workspace -- --nocapture`; `cargo clippy --workspace --all-targets -- -D warnings`; focused app interaction tests where feasible.
- **Manual/external checks:** Launch the desktop app and demonstrate one live page-layout surface: type ASCII and Unicode text, move the caret, select/replace/delete, backspace across an element boundary, press Enter for a role transition, change roles, undo/redo, scroll/wrap across pages, and resize without losing selection or text. Record unavailable visual evidence honestly.
- **Acceptance test:** Given a launched app with a new screenplay, when a writer types, selects, deletes, transitions roles, and changes layout geometry in the default page presentation, then the same core project/selection is updated through `EditingSession`, caret/selection remain valid, role semantics and undo/redo are preserved, and no duplicate text-entry surface or alternate page-editing mode exists.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `update`
- **Affected documents/IDs:** `AC-WRITING-001.1`–`001.5`, `AC-WRITING-002.1`, `AC-WRITING-002.4`, [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md)
- **Rationale and evidence:** The requested correction changes how the existing requirements are realized in the desktop shell, while the semantic and layout contracts remain the existing core boundary. The current defect is evidenced by [`main.rs`](../../../apps/scriptcraft/src/main.rs:205-280), where the app renders per-element labels plus a separate selected-element `TextEdit` and switches to a non-editable painted page view.
- **Authorized documentation changes:** Record the single live page-layout surface, caret/selection input ownership, and removal of the duplicate/mode-split UI; do not broaden requirements or add rich editing scope.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document impact classification is accurate against the diff, and authorized documentation updates are applied.
- [x] Execution log is appended, including dispatch and completion session/agent identifiers.
- [x] Work order is `complete` through the passed fresh validation fix chain in [`validation-WO-009a.md`](../validations/validation-WO-009a.md).

## Implementation evidence — 2026-10-25

- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs), this work order, and [`execution-log.md`](../execution-log.md). No core semantic, layout, persistence, recovery, export, or WO-010 files were changed. No commit was made.
- **Surface evidence:** Replaced the selectable per-element summary plus selected multiline `TextEdit` and the Continuous/Page toggle with one page-layout `live_document_surface`. Pages remain the default presentation; the painted page is the only editing surface and owns the visible caret and selection.
- **Input evidence:** Pointer hit testing uses `ScreenplayLayout::hit_test` and selections use `EditingSession::set_selection`. Text, replacement, Delete, Backspace, Enter, horizontal caret movement, undo, redo, role changes, and bounded errors route through the existing `EditingSession`; no parallel text buffer is retained.
- **Focused tests:** App tests cover single-surface initialization, direct core-routed replacement/deletion/backspace, role-aware Enter transition, and source-mapped hit/caret geometry. Existing core and WO-008 tests remain unchanged.
- **Checks:** `cargo fmt --all -- --check` passed; `cargo check --workspace` passed. Full build, test, clippy, and bounded smoke results are recorded in the completion log entry below.
- **Validation boundary:** This is implementer evidence only. The work order is `in-review`; no independent validation or completion claim is made.

## Independent validation — 2026-10-25

- **Validator/session:** Independent fresh-context validator, `validation-session-2026-10-25-wo-009`.
- **Result:** `FAIL`. The single page-layout surface, removal of duplicate editor/mode split, source-mapped caret/hit testing, core-routed typing/replacement/Delete/Backspace/Enter, undo/redo, bounded errors, and all requested format/check/build/test/clippy gates passed. The pointer implementation does not preserve the initial drag anchor when `drag_started()` is handled before `dragged()`, so reliable range selection is not demonstrated.
- **Required next state:** Create and dispatch **WO-009a — preserve pointer drag selection anchor**, then run a fresh validation. No product code was changed during validation.

## Fix-chain resolution — 2026-10-25

- **Validation:** [`validation-WO-009a.md`](../validations/validation-WO-009a.md).
- **Result:** `PASS`. WO-009a corrected the drag-start anchor loss and its fresh validation passed the focused regression plus all required quality gates.
- **Required next state:** WO-009 is `complete` under the validation process because its lettered fix work order passed validation.

# Work Order: WO-009a — Preserve Pointer Drag Selection Anchor

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop editing-surface fix implementer (one fresh session)
- **Phase/order:** Lettered fix for WO-009; depends on WO-009
- **Initiative:** [`../2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Generated filename:** `WO-009a-preserve-pointer-drag-selection-anchor.md`
- **Owner/orchestrator:** Forge orchestrator
- **Original work order:** [`WO-009-unified-live-editing-surface.md`](WO-009-unified-live-editing-surface.md)
- **Failing validation:** [`../validations/validation-WO-009.md`](../validations/validation-WO-009.md)
- **Fixes:** `none`
- **Implementer session/agent ID:** `code-session-2026-10-25-wo-009a`
- **Session exception:** `none`

## Summary

Preserve the pointer position hit at drag start as a stable selection anchor
through subsequent `dragged()` frames. Route each updated focus through the
existing `EditingSession::set_selection()` API so a pointer drag extends from
the initial hit position to the current hit position.

## In scope

- Add app-local transient state for the active pointer drag-start anchor.
- Use that anchor for every subsequent pointer-drag focus update.
- Add one focused regression test covering a multi-character drag range.

## Out of scope

- Persistence, recovery, exports, native paths, and WO-010.
- Changes to core editing, layout, selection semantics, or unrelated UI behavior.
- New editor surfaces, interaction modes, or broad UI polish.

## Requirements

- **WO-009 acceptance:** Selection and hit-testing are routed through
  `ScreenplayLayout` source mappings and `EditingSession::set_selection`.
- **Validation finding:** The pointer-down position must remain a stable drag
  anchor through the gesture, with updated focus routed through
  `EditingSession::set_selection()`.

## Implementation plan

1. Store the hit position from `drag_started()` in transient app state and use
   it rather than the mutable current selection anchor for `dragged()` frames.
2. Add a focused app regression test proving a drag from the first to a later
   character retains the initial anchor.

## Files and systems

- **Create:** This fix work order.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs), this work order, and [`../execution-log.md`](../execution-log.md).
- **Avoid changing:** Persistence, recovery, exports, WO-010, core crates, and unrelated UI behavior.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo build --workspace`; `cargo test --workspace -- --nocapture`; `cargo clippy --workspace --all-targets -- -D warnings`.
- **Manual/external checks:** No new manual UI claim; the bounded native smoke limitation remains recorded honestly.
- **Acceptance test:** Given a pointer hit at an initial character and a later
  `dragged()` hit, when the drag sequence is processed, then the resulting
  selection anchor remains the initial hit and focus is the later hit.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `verify unchanged`
- **Affected documents/IDs:** WO-009 writing-loop interaction criteria only; no durable document contracts.
- **Rationale and evidence:** This is an app-local transient pointer-state fix;
  it changes neither semantic content nor persistence, export, layout, or
  requirements behavior.
- **Authorized documentation changes:** Record implementation and check
  evidence in this fix WO and the execution log only.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document impact classification is accurate; no durable documents require updates.
- [x] Execution log is appended, including dispatch and completion session/agent identifiers.
- [x] Work order is `complete` after the passed fresh validation in [`../validations/validation-WO-009a.md`](../validations/validation-WO-009a.md).

## Implementation evidence — 2026-10-25

- **State transition:** `in-progress → in-review`
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs), this fix work order, and [`execution-log.md`](../execution-log.md). No persistence, recovery, export, core, WO-010, or validation files were changed. No commit was made.
- **Implementation:** Added transient `drag_anchor` state and a small pointer-selection adapter. `drag_started()` records the initial hit and sets the caret; each `dragged()` update uses that stable anchor with the current hit as focus and calls `EditingSession::set_selection()`. Non-drag pointer selection clears the transient anchor and preserves existing caret behavior.
- **Focused regression:** `pointer_drag_preserves_initial_hit_as_selection_anchor` covers a multi-character drag from byte offset 1 to offset 5 and verifies the resulting anchor/focus range.
- **Checks:** `cargo fmt --all -- --check` passed; `cargo check --workspace` passed; `cargo build --workspace` passed; `cargo test --workspace -- --nocapture` passed with 5 app tests, 22 core tests, 3 WO-008 integration tests, and doc-test target at zero failures; `cargo clippy --workspace --all-targets -- -D warnings` passed.
- **Validation boundary:** This is implementer evidence only. No validation artifact was created and no independent validation or `complete` status is claimed.

## Independent validation — 2026-10-25

- **Validator/session:** Independent fresh-context validator, `validation-session-2026-10-25-wo-009a`.
- **Result:** `PASS`. The focused drag regression preserved the initial hit at offset 1 while extending focus to offset 5; all required format/check/build/test/clippy gates passed. The bounded native smoke reached the binary but provided no visual/manual result.
- **Evidence:** [`validation-WO-009a.md`](../validations/validation-WO-009a.md).
- **Required next state:** WO-009a is `complete`; WO-009 is complete through this passed lettered fix chain.

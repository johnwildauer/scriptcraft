# Work Order: WO-003 — Editing Session and Safety

## Metadata

- **Status:** `complete`
- **Assignee:** Editing/session implementer (one fresh session)
- **Phase/order:** 3 of 8; depends on WO-002
- **Initiative:** [`../2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Generated filename:** `WO-003-editing-session.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`validation-WO-003.md`](../validations/validation-WO-003.md) — `PASS`
- **Fixes:** `none`
- **Implementer session/agent ID:** `code-session-2026-10-25-wo-003`
- **Session exception:** `none`

## Summary

Implement the headless transactional editing session over the semantic model. It owns checked positions/selections, explicit role transitions, dirty/revision state, and bounded undo/redo while preserving invariants on rejected operations.

## In scope

- Insert/delete/split/join/role-change, caret and selection checks at UTF-8 boundaries, explicit transition policy for the supported role set, undo/redo, and typed bounded errors.
- Failure-injection and rollback tests for model, selection, history, and dirty-state preservation.

## Out of scope

- Layout/pagination, desktop widgets, filesystem persistence/recovery, exports, collaboration, MCP, planning, and broad formatting.

## Requirements

- **AC-WRITING-001.3:** “Given a valid selection, the system shall support insertion, deletion, split, join, role change, undo, and redo while preserving valid text boundaries and element order.”
- **AC-WRITING-001.4:** “Given a role transition such as scene heading → action or character → dialogue, the system shall apply an explicit, testable transition policy and make the operation undoable.”
- **AC-WRITING-001.5:** “Given malformed or out-of-bound input, the system shall return a bounded error without corrupting the project or session state.”
- **NFR-WRITING-001:** “Production paths should prefer typed errors and bounded inputs over panic-prone handling; a discovered crash or data-loss defect requires a regression test.”
- **NFR-WRITING-002:** “Core editing, layout, and persistence behavior shall be exercisable without a desktop window for deterministic tests.”

## Blueprints

- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) — `EditingSession` owns transactional mutations, selection, dirty, revision, and history.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) — `RoleTransitionPolicy` remains explicit and undoable.

## Implementation plan

1. Implement checked position/selection and semantic commands over WO-002 types.
2. Implement the minimum explicit role-transition matrix and unsupported-transition behavior, preserving undo semantics.
3. Add bounded history and atomic mutation rollback tests, including hostile UTF-8 and out-of-range inputs.

## Files and systems

- **Create:** editing/session/transition modules and headless tests in the reusable crate.
- **Update:** blueprint facts for the concrete transition and session invariants.
- **Avoid changing:** UI event handling, pagination, native file I/O, and export adapters.

## Validation and E2E acceptance tests

- **Automated checks:** Workspace format/check/test; command sequence tests for every required mutation and rollback tests.
- **Manual/external checks:** Review transition table and error behavior against the approved scope.
- **Acceptance test:** Given a valid semantic project and selection, when all required edits and role transitions execute, then order/text/roles remain valid, undo/redo restores exact prior states, and invalid input leaves all session state unchanged.

## Durable-document impact

- **Requirements impact:** `verify unchanged` — implements existing ACs without changing their wording.
- **Blueprint impact:** `update` — record concrete session/transition ownership and safety evidence.
- **Affected documents/IDs:** `AC-WRITING-001.3`–`001.5`, `NFR-WRITING-001`–`002`, [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md)
- **Rationale and evidence:** Session behavior makes the desired transactional editing boundary concrete.
- **Authorized documentation changes:** Add the minimum supported transition matrix and verified session invariants only.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document impact classification is accurate.
- [x] Execution log is appended with dispatch/completion IDs.
- [x] Work order is `complete` after a passed fresh validation.

## Implementation evidence

- **Completion session/agent ID:** `code-session-2026-10-25-wo-003`
- **Changed files:** [`crates/scriptcraft-core/src/lib.rs`](../../../crates/scriptcraft-core/src/lib.rs), [`crates/scriptcraft-core/src/editing.rs`](../../../crates/scriptcraft-core/src/editing.rs), this work order, and the initiative execution log. No commit was made.
- **API/model evidence:** Added the dependency-free [`editing`](../../../crates/scriptcraft-core/src/editing.rs) module with checked UTF-8 [`Position`](../../../crates/scriptcraft-core/src/editing.rs), [`Selection`](../../../crates/scriptcraft-core/src/editing.rs), typed [`EditError`](../../../crates/scriptcraft-core/src/editing.rs), and transactional [`EditingSession`](../../../crates/scriptcraft-core/src/editing.rs). The session supports insertion, deletion, backspace, split, join, direct role changes, explicit Enter transitions, bounded undo/redo, dirty state, and monotonic revision state.
- **Transition evidence:** [`transition`](../../../crates/scriptcraft-core/src/editing.rs) is an explicit finite policy: scene heading → action, action → action, character → dialogue, dialogue → parenthetical, parenthetical → dialogue, transition → scene heading, and note → note. Enter applies the policy transactionally and records an undo state.
- **Safety evidence:** Mutations operate on cloned project/session inputs and validate the candidate project and selection before commit. Invalid elements, non-boundary offsets, reversed selections, oversized text, element limits, unsupported history operations, and unsupported transitions return typed errors without changing project, selection, history, dirty, or revision state.
- **Focused tests:** Ten workspace core tests pass, including Unicode boundary/backspace behavior, explicit Enter role transition, cross-element backspace/join, failed-edit rollback, dirty/revision behavior, existing model invariants, and undo/redo restoration.
- **Validation status:** Implementer checks and independent fresh validation passed; the validation is linked above.

## Independent validation evidence

- **Validator session/agent ID:** `validation-session-2026-10-25-wo-003`
- **Verdict:** `PASS`
- **Evidence:** [`validation-WO-003.md`](../validations/validation-WO-003.md) records fresh-context source inspection, an independent behavioral probe covering UTF-8 boundaries, edits, transitions, rollback, bounded history, undo/redo, dirty/revision semantics, and passing format/check/test/clippy gates.
- **Result:** No hard-stop implementation, scope, policy-gate, or rogue-behavior finding; no lettered fix work order required.

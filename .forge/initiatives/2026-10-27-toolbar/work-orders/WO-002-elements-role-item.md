# Work Order: WO-002 — Elements Role Item

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop toolbar implementer (one fresh session)
- **Phase/order:** 2 of 3; depends on WO-001
- **Initiative:** [`../toolbar-v1.md`](../toolbar-v1.md:1)
- **Generated filename:** `WO-002-elements-role-item.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-002.md`](../validations/validation-WO-002.md)
- **Fixes:** none
- **Implementer session/agent ID:** `code-implementer-toolbar-wo002-2026-10-27-0237`
- **Session exception:** none

## Summary

Implement the central Elements Item as a text dropdown containing the supported screenplay roles. Its displayed label must derive from the current focused element, and a selected role must call the existing transactional session operation so the caret's element changes without bypassing undo/error invariants.

## In scope

- Replace the separate role-controls row with the Elements Item in the toolbar composition.
- Display Scene Heading, Action, Character, Dialogue, Parenthetical, Transition, and Note labels using the existing role mapping.
- Synchronize the selected/displayed value with the current session selection on every relevant render.
- Dispatch role changes through [`EditingSession::change_role()`](../../../../crates/scriptcraft-core/src/editing.rs:1), preserving failure reporting and focus transfer.
- Add tests for role display, each supported role choice, current-caret targeting, and rejected/no-op safety.

## Out of scope

- New roles, role policy changes, Enter/Tab behavior, or core model changes.
- Toolbar spacing/overflow mechanics; WO-003 owns those presentation concerns.
- Persisting selected toolbar state or changing the native project format.

## Requirements

- **REQ-WRITING-LOOP-001 / AC-WRITING-001.2:** “Given an element, the system shall preserve its explicit role and canonical UTF-8 text independently of capitalization, spaces, tabs, indentation, or font appearance.”
- **REQ-WRITING-LOOP-001 / AC-WRITING-001.3:** “Given a valid selection, the system shall support insertion, deletion, split, join, role change, undo, and redo while preserving valid text boundaries and element order.”
- **REQ-WRITING-LOOP-001 / AC-WRITING-001.5:** “Given malformed or out-of-bound input, the system shall return a bounded error without corrupting the project or session state.”

## Blueprints

- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:59) — semantic role changes remain transactional and owned by `EditingSession`.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:35) — the desktop adapts role actions and delegates semantic edits.

## Implementation plan

1. Replace the old role-control rendering with the Elements Item control in [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:269).
2. Derive the selected role from the session focus and route selection changes through the existing mutation/error path.
3. Add focused app/core regression coverage for role synchronization, transactionality, undo, and selection targeting.

## Files and systems

- **Create:** No new production module required unless a bounded toolbar helper is extracted; tests may be added.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), authorized requirement/blueprint sections, this work order, and [`execution-log.md`](../execution-log.md:1).
- **Avoid changing:** role transition policy in [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:1), native schema, persistence, and layout.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`.
- **Manual/external checks:** bounded desktop smoke with the caret in at least Action, Dialogue, and Character elements; native control interaction is unclaimed if unavailable.
- **Acceptance test:** Given the caret is in a Dialogue element, when the Elements dropdown is opened, then Dialogue is displayed and selected; when Action is chosen, then the focused element becomes Action through the session boundary and the display updates without corrupting text or selection.

## Durable-document impact

- **Requirements impact:** `update`
- **Blueprint impact:** `update`
- **Affected documents/IDs:** [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:38), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:59), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:35)
- **Rationale and evidence:** The Elements control exposes an existing semantic capability through a new user-facing path; documents must explicitly connect the control to current focus and transactional role changes.
- **Authorized documentation changes:** Clarify the toolbar role-selection acceptance path and desktop/core ownership only; do not alter role semantics.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document impact classification is accurate against the diff, and authorized documentation updates are applied.
- [x] Execution log is appended, including dispatch and completion session/agent identifiers.
- [x] Work order is `in-review` with recorded evidence; completion comes only from a passed validation.

## Implementation evidence

- **Implementer session/agent ID:** `code-implementer-toolbar-wo002-2026-10-27-0237`
- **Dispatch evidence:** WO-001 passed fresh validation before this session; this session was dispatched for WO-002 only. WO-003 remains undispatched.
- **Changed files:**
  - [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1) — replaced the Elements placeholder with a session-focus-synchronized text dropdown, routed choices through `EditingSession::change_role()` and the existing mutation/error path, and added focused role/display/caret/rejection tests.
  - [`../../../requirements/features/screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:98) — added the authorized Elements role-selection acceptance criterion.
  - [`../../../blueprints/containers/scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58) — recorded desktop/core ownership and transactional Elements dispatch.
  - [`WO-002-elements-role-item.md`](WO-002-elements-role-item.md:1) — recorded this implementation evidence and transitioned to `in-review`.
  - [`../execution-log.md`](../execution-log.md:1) — appended WO-002 dispatch and completion evidence.
- **Validation attempts/results:**
  - Initial `cargo fmt --all -- --check` — FAIL because the newly added tests needed formatter output; no source behavior failure. Remediated with `cargo fmt --all`.
  - `cargo fmt --all -- --check` — PASS after formatter correction.
  - `cargo check --workspace` — PASS.
  - `cargo test --workspace` — PASS; app 38 tests, core 44 unit tests, all workspace integration/doc tests passed with zero failures.
  - `cargo build --workspace` — PASS.
  - `cargo clippy --workspace --all-targets -- -D warnings` — PASS.
  - No fresh validation was performed by this implementer session; the listed commands are implementation-session policy checks, not WO validation.
- **Comparable diff evidence:** `git diff -- apps/scriptcraft/src/main.rs .forge/requirements/features/screenplay-writing-loop.md .forge/blueprints/containers/scriptcraft-desktop.md .forge/initiatives/2026-10-27-toolbar/work-orders/WO-002-elements-role-item.md .forge/initiatives/2026-10-27-toolbar/execution-log.md` is the bounded implementation diff.
- **Scope confirmation:** The seven existing roles are exposed without changing role-transition policy, `EditingSession`, persistence, schema, layout, spacing, overflow, or WO-003 behavior. The former separate role-control presentation was not present after WO-001; the Elements placeholder was replaced in the central toolbar.
- **Durable-document evidence:** Requirements and blueprint updates are limited to the authorized role-selection acceptance path and desktop/core ownership clarification; semantic role meaning and transition policy are unchanged.
- **Next state:** Fresh WO-002 validation is required. Do not mark this work order `complete` from the implementer session.

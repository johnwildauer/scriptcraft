# Work Order: WO-002 — Role Keyboard and Line Navigation

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer after WO-001 validation
- **Phase/order:** `2 of 4`
- **Initiative:** [`../ui-quality-of-life-v1.md`](../ui-quality-of-life-v1.md)
- **Generated filename:** `WO-002-role-keyboard-and-line-navigation.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-002.md`](../validations/validation-WO-002.md)
- **Fixes:** `none`
- **Implementer session/agent ID:** `code-fresh-context-ui-quality-of-life-WO-002-2026-10-25T21:11:33Z`
- **Session exception:** `none`

## Summary

Replace the current role-transition behavior with the requested Final Draft-style Enter and Tab matrices, then make Up/Down operate on wrapped visual lines rather than screenplay elements. The semantic transitions belong in the core; key dispatch and transient preferred vertical position belong at the app/session boundary.

## In scope

- Enter: General/Note→same role, Scene Heading→Action, Action→Action, Character→Dialogue, Dialogue→Action, Parenthetical→Dialogue, Transition→Scene Heading.
- Tab: Scene Heading→Action, Action→Character, blank Character→Transition, Transition→Scene Heading, Dialogue→Parenthetical, Parenthetical→Dialogue.
- Explicit fallback behavior for unlisted Tab roles as resolved by design review, with no accidental literal tab in canonical text.
- Visual-line Up/Down using layout [`LayoutLine`](../../../crates/scriptcraft-core/src/layout.rs:29) source ranges and geometry, including wrapped lines, cross-element/page movement, edge clamping, and Shift extension.
- Unlisted Tab cases are consumed no-ops: they do not insert a literal tab, mutate the model, or create an undo entry.
- The preferred vertical goal persists across successful repeated Up/Down moves and resets on horizontal movement, edits, pointer selection, Home/End, role changes, undo/redo, and a no-op boundary move.

## Out of scope

- Full Final Draft keymap parity, mouse wheel behavior, word-wise movement, grapheme-cluster redesign, or arbitrary tab stops.
- Changing screenplay roles or layout measures.

## Requirements

- **REQ-WRITING-LOOP-001:** “As an individual screenwriter, I want to create and edit an ordered screenplay with explicit element roles, so that my draft retains screenplay meaning while I write.”
- **AC-WRITING-001.3:** “Given a valid selection, the system shall support insertion, deletion, split, join, role change, undo, and redo while preserving valid text boundaries and element order.”
- **AC-WRITING-001.4:** “Given a role transition such as scene heading → action or character → dialogue, the system shall apply an explicit, testable transition policy and make the operation undoable.”
- **AC-WRITING-002.4:** “Given a document position and rendered geometry, the system shall support a safe caret/hit-test round trip at valid text boundaries.”

## Blueprints

- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) — editing owns checked positions, explicit transitions, and transactional undo.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) — the UI translates input to semantic operations and layout supplies source mappings.

## Implementation plan

1. Add separate, explicit Enter and Tab policy functions in [`editing.rs`](../../../crates/scriptcraft-core/src/editing.rs:216), preserving atomic split/role mutation and undo.
2. Add core tests for every requested role, blank/nonblank Character Tab behavior, unspecified fallback, and undo/redo.
3. Add transient preferred vertical goal state in the app/session layer; derive current and target visual lines from the latest layout rather than element indexes.
4. Map the desired horizontal goal to the target line's nearest valid UTF-8 boundary; reset the goal on horizontal movement, edits, pointer selection, and role-changing commands.
5. Add tests for short-line clamping, document first/last line behavior, wrapped lines, page boundaries, Unicode, and Shift+Up/Down.

## Files and systems

- **Create:** core editing/navigation tests and any narrow test helper needed for visual-line fixtures.
- **Update:** [`crates/scriptcraft-core/src/editing.rs`](../../../crates/scriptcraft-core/src/editing.rs), [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs), and possibly [`crates/scriptcraft-core/src/layout.rs`](../../../crates/scriptcraft-core/src/layout.rs) only for a missing source/line query.
- **Avoid changing:** native JSON schema; preferred vertical goal is transient.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`.
- **Manual/external checks:** type/paste a wrapped action and dialogue, move Up/Down repeatedly across page boundaries, test short/long adjacent lines and first/last document lines, and verify Enter/Tab in every role through the running app.
- **Acceptance test:** Given the caret on any requested role or wrapped visual line, when Enter, Tab, Up, or Down is pressed, then the resulting role/caret follows the approved matrix, stays at the nearest intended horizontal line position, clamps at the correct document edge, and remains undoable where the operation mutates content or role.

## Durable-document impact

- **Requirements impact:** `update`
- **Blueprint impact:** `update`
- **Affected documents/IDs:** [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md)
- **Rationale and evidence:** The current requirement names explicit transitions but not the complete matrix or visual-line navigation semantics. The final implementation must add the approved matrix and transient-navigation contract to the durable documents, without persisting view state.
- **Authorized documentation changes:** Add the finalized Enter/Tab matrix, fallback behavior, and navigation invariants with links to the passing validation artifact.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document updates are applied only after the final behavior is known.
- [x] Execution log is appended with dispatch/completion identifiers.
- [x] Work order was `in-review` before fresh validation and became `complete` after the passed validation in [`validation-WO-002.md`](../validations/validation-WO-002.md).

## Implementer completion evidence

- **Changed files:** [`crates/scriptcraft-core/src/editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs), [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs), [`../../../requirements/features/screenplay-writing-loop.md`](../../../../requirements/features/screenplay-writing-loop.md), [`../../../blueprints/components/screenplay-core.md`](../../../../blueprints/components/screenplay-core.md), and this work order plus the initiative execution log.
- **Implementation evidence:** Added separate Enter/Tab policies with transactional undo behavior and consumed no-op fallback; added layout-backed wrapped visual-line movement across elements/pages with preferred horizontal goal and Shift extension; added headless matrix, fallback, wrapping, and selection tests.
- **Validation evidence:** `cargo fmt --all -- --check` passed after formatting; `cargo check --workspace` passed; `cargo test --workspace` passed with 18 app tests, 24 core tests, and 3 E2E tests; `cargo clippy --workspace --all-targets -- -D warnings` passed. One earlier test run failed only because a newly added assertion assumed the wrong boundary position; the test was corrected and the rerun passed. Native desktop interaction was not attempted.
- **Durable-document evidence:** Updated the authorized requirements and core blueprint with the finalized matrix, consumed-no-op fallback, and transient navigation contract.
- **Follow-up:** Fresh validator returned `PASS WITH WARNINGS`; the orchestrator marked this work order `complete`. Native desktop interaction remains an explicitly recorded warning.

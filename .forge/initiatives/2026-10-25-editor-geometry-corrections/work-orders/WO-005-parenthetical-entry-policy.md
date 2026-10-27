# Work Order: WO-005 — Parenthetical Entry Policy

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer after WO-004 validation
- **Phase/order:** `5 of 6`
- **Initiative:** [`../editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md)
- **Validation:** [`../validations/validation-WO-005a-v3.md`](../validations/validation-WO-005a-v3.md)
- **Fixes:** [`WO-005a-invalidate-parenthetical-provenance.md`](WO-005a-invalidate-parenthetical-provenance.md)

## Summary

Make a newly inserted Parenthetical start as `()` with the caret inside, and remove only an empty generated pair when Enter or Tab leaves it.

## In scope

- Define the parenthetical entry boundary for Enter/Tab role transitions and explicit role assignment.
- Insert `()` transactionally and place the selection at the interior UTF-8 boundary.
- Remove an empty generated pair when leaving by Enter/Tab; preserve non-empty or user-authored parentheses.
- Preserve one logical undo step for the user-visible transition and test selection/caret behavior.

## Out of scope

- General bracket matching, rich-text auto-pairing, arbitrary delimiter completion, or changing Dialogue semantics.

## Requirements and blueprints

- **Requirements:** `REQ-WRITING-LOOP-001`, `AC-WRITING-001.3`, `AC-WRITING-001.5` in [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md).
- **Blueprint:** [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md).

## Validation

- Test empty generated pair, typed content, user-authored `()`, partial delimiters, Enter, Tab, undo/redo, paste, and role transitions.
- Verify failed transitions leave the session unchanged and all caret offsets remain valid UTF-8 boundaries.
- Run all Cargo gates.

## Durable-document impact

- **Requirements:** `update` — generated-parenthetical insertion and cleanup are a new user-facing editing contract and must be recorded with the provenance boundary.
- **Blueprint:** `update` with the transactional editing policy.

## Reconciled handoff

- **Applicable requirements (verbatim):** `REQ-WRITING-LOOP-001`: “As an individual screenwriter, I want to create and edit an ordered screenplay with explicit element roles, so that my draft retains screenplay meaning while I write.” `AC-WRITING-001.3`: “Given a valid selection, the system shall support insertion, deletion, split, join, role change, undo, and redo while preserving valid text boundaries and element order.” `AC-WRITING-001.5`: “Given malformed or out-of-bound input, the system shall return a bounded error without corrupting the project or session state.”
- **Exact oracle:** Generated `()` is marked transiently by stable element ID and included in undo/redo snapshots, never serialized. Cleanup occurs only on Enter/Tab when the marked element remains Parenthetical and exactly `()`; save/reopen, pasted/user-authored `()`, partial delimiters, and non-empty text are preserved.
- **Rationale/evidence:** [`ScreenplayElement`](../../../crates/scriptcraft-core/src/lib.rs:107) has no provenance field, so transient session metadata avoids schema changes while protecting user-authored empty pairs.
- **Authorized documentation changes:** Update [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) and [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) with the transient provenance policy; update requirements only if this becomes a frozen criterion.
- **Native evidence boundary:** Core editing/undo/persistence tests and Cargo gates are required; native typing smoke is optional and cannot be claimed from headless tests.

## Validation outcome

- Initial validation [`validation-WO-005-v2.md`](../validations/validation-WO-005-v2.md) identified the genuine provenance-invalidation defect.
- Fix [`WO-005a-invalidate-parenthetical-provenance.md`](WO-005a-invalidate-parenthetical-provenance.md) passed scope-corrected fresh validation [`validation-WO-005a-v3.md`](../validations/validation-WO-005a-v3.md) with warnings.
- State transition: `in-review → complete` through the WO-005a fix chain; no further fix is required.

## Implementation evidence — 2026-10-26

- **Changed files:** [`editing.rs`](../../../crates/scriptcraft-core/src/editing.rs), [`wo005_parenthetical.rs`](../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md), and this work order plus the initiative execution log.
- [`EditingSession`](../../../crates/scriptcraft-core/src/editing.rs) now keeps generated-parenthetical stable IDs in transient session state; snapshots copy that state for undo/redo, while persistence continues to serialize only [`Project`](../../../crates/scriptcraft-core/src/lib.rs) data. Explicit role assignment and Dialogue→Parenthetical Tab create `()` with a byte-offset-one caret. Enter/Tab cleanup is guarded by role, exact text, and provenance.
- Fix-chain correction in [`WO-005a-invalidate-parenthetical-provenance.md`](WO-005a-invalidate-parenthetical-provenance.md): successful candidate edits now invalidate a marked ID when its element record changes, so editing generated `()` away and reconstructing `()` makes the pair user-authored before cleanup. Internal generation, failed-edit rollback, and snapshot provenance remain unchanged.
- Deterministic tests cover Enter and Tab cleanup, explicit role assignment, caret placement, content insertion, partial delimiters, user-authored and pasted `()`, undo/redo provenance restoration, and save/reopen non-persistence. Existing transactional clone/validate behavior continues to protect failed transitions and UTF-8 boundaries.
- **Cargo evidence:** Focused `cargo test -p scriptcraft-core --test wo005_parenthetical` passed with 5 tests. Full required gates are recorded in the execution log below after completion.

# Work Order: WO-005a — Invalidate Parenthetical Provenance on Edit

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer after WO-005 validation failure
- **Phase/order:** `fix for 5 of 6`
- **Initiative:** [`../editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md)
- **Original work order:** [`WO-005-parenthetical-entry-policy.md`](WO-005-parenthetical-entry-policy.md)
- **Failing validation:** [`../validations/validation-WO-005-v2.md`](../validations/validation-WO-005-v2.md)
- **Validation:** [`../validations/validation-WO-005a-v3.md`](../validations/validation-WO-005a-v3.md)

## Summary

Close the WO-005 provenance hard stop: any ordinary edit that changes a generated parenthetical must invalidate its generated marker before later cleanup decisions.

## In scope

- Invalidate transient generated-parenthetical provenance when generated content, role, selection, or element identity changes in a way that makes the marker no longer trustworthy.
- Preserve the existing cleanup condition: only a still-marked element with Parenthetical role and exact `()` may be removed on Enter/Tab.
- Add a regression where generated `()` is edited away and reconstructed as user-authored `()`, proving Enter/Tab preserves it.
- Preserve undo/redo provenance snapshots, save/reopen non-persistence, and all existing WO-005 behavior.

## Out of scope

- New delimiter behavior, persistence schema changes, general provenance systems, or changes to other editing policies.

## Requirements and blueprints

- **Requirements (verbatim):** `AC-WRITING-001.3`: “Given a valid selection, the system shall support insertion, deletion, split, join, role change, undo, and redo while preserving valid text boundaries and element order.” `AC-WRITING-001.5`: “Given malformed or out-of-bound input, the system shall return a bounded error without corrupting the project or session state.”
- **Blueprint:** [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md).

## Implementation guidance

Keep the marker transient and stable-ID keyed. The smallest safe fix may clear the marker on any successful edit affecting the marked element or on role/selection transition that changes the generated pair's provenance. Ensure failed edits do not mutate provenance or session state.

## Validation

- Run all Cargo gates.
- Test generated pair cleanup, edit-away/reconstruct-user-authored pair preservation, partial edits, content insertion, undo/redo, save/reopen, and failed-edit rollback.
- Native typing evidence is optional/unavailable under project policy.

## Implementation evidence — 2026-10-26

- **Changed files:** [`editing.rs`](../../../crates/scriptcraft-core/src/editing.rs), [`wo005_parenthetical.rs`](../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`WO-005-parenthetical-entry-policy.md`](WO-005-parenthetical-entry-policy.md), and [`execution-log.md`](../execution-log.md).
- [`EditingSession::mutate()`](../../../crates/scriptcraft-core/src/editing.rs) now compares the candidate project with the live project and removes transient generated-parenthetical IDs for marked elements whose stable element record changes. Internal generated insertion and cleanup retain their existing explicit marker operations; failed operations still return before candidate state is committed, and undo/redo snapshots remain unchanged.
- Regression [`editing_away_and_reconstructing_generated_pair_makes_it_user_authored()`](../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs) edits generated `()` to `(`, reconstructs `()`, and verifies Enter preserves the reconstructed pair. Existing cleanup, partial/non-empty, user-authored/pasted, undo/redo, save/reopen, and transactional rollback coverage remains in place.
- **Cargo evidence:** Focused `cargo test -p scriptcraft-core --test wo005_parenthetical` passed with 6 tests. The full gate chain passed: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace` (93 tests plus 0 doc tests), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace`, and `git diff --check`. Native typing smoke was not performed and is not inferred from headless evidence.

## Durable-document impact

- **Requirements:** `verify unchanged` — this is a corrective provenance invariant within the approved parenthetical contract.
- **Blueprint:** `update` only to clarify marker invalidation evidence.
- **Rationale/evidence:** [`validation-WO-005-v2.md`](../validations/validation-WO-005-v2.md) found that ordinary edits did not invalidate the generated marker, allowing reconstructed user-authored `()` to be deleted.
- **Authorized documentation changes:** Update the original WO, this fix WO, [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), and [`execution-log.md`](../execution-log.md) with fix-chain evidence.

## Validation outcome

- Fresh scope-corrected validation [`validation-WO-005a-v3.md`](../validations/validation-WO-005a-v3.md) returned `PASS WITH WARNINGS` for the authorized fix boundary.
- The prior validation's extra typed/deleted and caret-position findings were classified as out-of-scope/rogue because they are absent from the approved v2 plan and WO-005a handoff.
- State transition: `in-review → complete`; no additional fix work order is required.

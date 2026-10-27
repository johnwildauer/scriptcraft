# Validation — WO-005a Invalidate Parenthetical Provenance on Edit (v3)

## Result

- **Validation result:** `PASS WITH WARNINGS`
- **Validated work order:** [`WO-005a-invalidate-parenthetical-provenance.md`](../work-orders/WO-005a-invalidate-parenthetical-provenance.md:1)
- **Original work order:** [`WO-005-parenthetical-entry-policy.md`](../work-orders/WO-005-parenthetical-entry-policy.md:1)
- **Prior validation superseded for scope assessment:** [`validation-WO-005a-v2.md`](validation-WO-005a-v2.md:1)
- **Initiative:** [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:1)
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `architect-fresh-context-editor-geometry-corrections-v3-WO-005a-validation-2026-10-26`
- **Session exception:** `none`
- **Work-order state at validation:** `in-review`; this validator did not change the state
- **Fix chain:** `WO-005a`
- **Product source modified by validator:** `no`
- **Existing artifacts modified by validator:** `no`; this new validation artifact is the only file created

## Fresh comparison basis and authorized boundary

I independently read the Forge validation rules in [`05-validation.md`](../../../process/05-validation.md:1), the approved v2 plan and parenthetical oracle in [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:73), the reconciled v2 review in [`design-review-v2.md`](../reviews/design-review-v2.md:146), the original and fix work orders, the prior validations, the v2 execution log, the current editing/model/persistence source, the focused parenthetical tests, the core blueprint, the project manifest, and the available implementation evidence.

The comparison is meaningful. [`WO-005a-invalidate-parenthetical-provenance.md`](../work-orders/WO-005a-invalidate-parenthetical-provenance.md:43) records the changed-file boundary and implementation evidence; [`execution-log.md`](../execution-log.md:88) records the fresh fix dispatch, implementation, focused regression, full Cargo evidence, and `in-review` transition; and the current source and tests contain the reported invalidation behavior.

This validation assesses only the authorized fix boundary:

1. ordinary successful edits invalidate generated-parenthetical provenance when the marked element record changes;
2. editing generated `()` away and reconstructing `()` makes the pair user-authored and preserves it on later Enter/Tab cleanup;
3. untouched generated `()` cleanup remains available;
4. transient stable-ID provenance remains undoable, non-persistent, and transactionally safe;
5. undo/redo, failed rollback, save/reopen, user-authored, pasted, partial, and non-empty preservation remain intact;
6. implementation scope, durable-document impact, rogue behavior, and the required Cargo gates remain aligned.

The approved scope does **not** create a new typed-then-deleted expectation, a new caret-position policy, or a new text-presence interaction contract. The v2 plan says generated `()` is removed when Enter/Tab leaves it and non-empty parentheses remain, while the fix work order explicitly excludes new delimiter behavior and changes to other editing policies at [`WO-005a-invalidate-parenthetical-provenance.md`](../work-orders/WO-005a-invalidate-parenthetical-provenance.md:17). Validation must not anticipate those future obligations under [`05-validation.md`](../../../process/05-validation.md:65).

## Authorized scope versus prior v2 findings

| Prior v2 finding | Scope classification | Fresh v3 disposition |
|---|---|---|
| “Typed/deleted” expectation that deleting typed content should restore the prior empty generated-pair cleanup behavior | **Out of scope / rogue validation finding** | Not an acceptance criterion in the approved v2 plan, [`WO-005-parenthetical-entry-policy.md`](../work-orders/WO-005-parenthetical-entry-policy.md:43), or [`WO-005a-invalidate-parenthetical-provenance.md`](../work-orders/WO-005a-invalidate-parenthetical-provenance.md:17). Under the authorized rule, the first successful edit that changes the marked element invalidates provenance; a later empty `()` is therefore user-authored and preserved. No failure is recorded. |
| Caret-position and text-presence policy requiring special end-of-text, interior-caret, and before-opening-delimiter Enter/Tab behavior | **Out of scope / rogue validation finding** | Not present in the user-approved v2 plan/oracle, original WO, or fix WO. The fix preserves the existing Enter/Tab behavior except for provenance invalidation and does not need a new caret policy. No failure is recorded. |

The prior v2 artifact therefore does not establish additional acceptance criteria. Its findings are retained as historical evidence of scope error only; this v3 artifact does not modify that existing artifact.

## Evidence assessment

| Authorized boundary | Status | Fresh evidence and assessment |
|---|---|---|
| Transient provenance model | **PASS** | [`EditingSession`](../../../../crates/scriptcraft-core/src/editing.rs:61) stores generated stable IDs outside [`ScreenplayElement`](../../../../crates/scriptcraft-core/src/lib.rs:107). [`Snapshot`](../../../../crates/scriptcraft-core/src/editing.rs:53) includes the set, and [`undo()`](../../../../crates/scriptcraft-core/src/editing.rs:328) / [`redo()`](../../../../crates/scriptcraft-core/src/editing.rs:344) restore it. The marker remains transient, stable-ID keyed, undoable, and absent from the semantic persistence model. |
| Ordinary edits invalidate changed generated elements | **PASS** | [`mutate()`](../../../../crates/scriptcraft-core/src/editing.rs:360) clones the live project and prior marker set, runs [`invalidate_changed_generated_parentheticals()`](../../../../crates/scriptcraft-core/src/editing.rs:410), and only commits after candidate validation. The helper compares the before/after stable element records and removes a marker when a marked record changes at [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:420). This covers successful text, role, split/join, and other candidate changes without clearing the marker for internal generated insertion before commit. |
| Edit-away/reconstruct `()` becomes user-authored | **PASS** | [`editing_away_and_reconstructing_generated_pair_makes_it_user_authored()`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:77) changes generated `()` to `(`, reconstructs `()`, presses Enter, and asserts that the pair remains `()` while a Dialogue element is created. This is the exact regression required by the prior authorized hard stop. |
| Untouched generated cleanup remains intact | **PASS** | [`cleanup_generated_parenthetical()`](../../../../crates/scriptcraft-core/src/editing.rs:497) still requires Parenthetical role, exact `()`, and a live generated marker. Existing Dialogue→Parenthetical Tab and explicit role assignment still generate `()` with caret byte offset `1` at [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:267) and [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:307); focused coverage remains at [`wo005_parenthetical.rs`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:23). |
| Undo/redo provenance | **PASS** | Undo and redo restore the project, selection, and generated-ID snapshot together at [`EditingSession::undo()`](../../../../crates/scriptcraft-core/src/editing.rs:328) and [`EditingSession::redo()`](../../../../crates/scriptcraft-core/src/editing.rs:344). The focused regression retains coverage at [`wo005_parenthetical.rs`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:109). |
| Failed rollback and valid UTF-8 boundaries | **PASS by inspected evidence** | [`mutate()`](../../../../crates/scriptcraft-core/src/editing.rs:364) operates on cloned project, selection, and marker state; failed candidate operations return before commit, history recording, or marker replacement. Selection validation remains before commit at [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:380). The fix evidence expressly preserves this boundary at [`WO-005a-invalidate-parenthetical-provenance.md`](../work-orders/WO-005a-invalidate-parenthetical-provenance.md:43). |
| Save/reopen and non-persistence | **PASS** | The semantic project contains no generated marker, while serialization covers only durable project data. The focused test verifies that no generated marker is serialized and that a reopened `()` is treated as user-authored at [`wo005_parenthetical.rs`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:126). A new session through [`replace_with_saved()`](../../../../crates/scriptcraft-core/src/editing.rs:124) starts without transient provenance. |
| User-authored, pasted, partial, and non-empty preservation | **PASS** | Existing focused coverage preserves inserted/pasted user-authored `()`, partial delimiters, and non-empty text at [`wo005_parenthetical.rs`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:59) and [`wo005_parenthetical.rs`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:92). The new reconstruction regression adds the previously missing transition from generated to user-authored content. |
| Scope and rogue behavior | **PASS** | The changed-file record is limited to the transactional editing core, focused test, authorized blueprint/work-order/execution evidence, and the original authorized documentation boundary at [`WO-005a-invalidate-parenthetical-provenance.md`](../work-orders/WO-005a-invalidate-parenthetical-provenance.md:43). No second text buffer, rich-text pairing, arbitrary bracket completion, new role, renderer path, deceptive behavior, backdoor, malicious behavior, or unrelated product area was observed. |
| Durable-document impact | **PASS** | The marker remains transient and no schema or durable provenance field is introduced. The blueprint clarification records successful-edit invalidation and cleanup conditions at [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:72), matching the fix work order's `verify unchanged` requirements classification and authorized blueprint clarification at [`WO-005a-invalidate-parenthetical-provenance.md`](../work-orders/WO-005a-invalidate-parenthetical-provenance.md:50). |

## Required Cargo gates

The fix work order and execution log report passing results for [`cargo fmt --all -- --check`](../../../../Cargo.toml:1), [`cargo check --workspace`](../../../../Cargo.toml:1), [`cargo test --workspace`](../../../../Cargo.toml:1) with 93 tests and 0 doc tests, [`cargo clippy --workspace --all-targets -- -D warnings`](../../../../Cargo.toml:1), [`cargo build --workspace`](../../../../Cargo.toml:1), and [`git diff --check`](../../../../Cargo.toml:1) at [`WO-005a-invalidate-parenthetical-provenance.md`](../work-orders/WO-005a-invalidate-parenthetical-provenance.md:43) and [`execution-log.md`](../execution-log.md:92). The focused [`cargo test -p scriptcraft-core --test wo005_parenthetical`](../../../../Cargo.toml:1) is recorded as passing with six tests.

These gates were not independently rerun in this repository-reading validation context. The recorded results are specific, current, and internally consistent with the inspected source and tests, so they are accepted as evidence with a warning rather than treated as a blocker. The project manifest configures no required manual/external gate at [`project-manifest.md`](../../../project-manifest.md:88). Native typing smoke was not performed and no native success is claimed; it is not a required hard-stop gate under the manifest.

## Warnings and notes

### W-001 — Cargo gates are recorded, not independently rerun

- **Severity:** warning
- **Evidence:** Passing focused and full gate results are recorded in [`execution-log.md`](../execution-log.md:92) and the fix work order, but this validation used repository inspection and artifact-writing evidence rather than command execution.
- **Recommendation:** Preserve the recorded gate evidence and rerun the chain during any later implementation change.
- **Required next state:** Note only; no fix work order is required by this warning.

### W-002 — No standalone branch diff was available

- **Severity:** warning
- **Evidence:** Comparison used the changed-file record, execution log, current source, tests, and durable documents. A standalone Git diff was not available in this inspection context.
- **Recommendation:** Retain the file-level evidence as the comparison basis and attach a comparable diff if one becomes available.
- **Required next state:** Note only; the available evidence is sufficient for meaningful comparison under [`05-validation.md`](../../../process/05-validation.md:20).

## Final verdict and exact next state

- **Implementation alignment:** **Pass.** The authorized provenance fix is implemented: ordinary successful edits that change a marked generated element clear its transient marker; edit-away/reconstruct `()` is preserved as user-authored; untouched generated `()` cleanup remains; undo/redo, failed rollback, save/reopen, user-authored/pasted/partial/non-empty preservation, scope, and durable boundaries remain aligned.
- **Prior v2 extra findings:** **Rejected as acceptance criteria.** The typed/deleted expectation and caret-position/text-presence policy are not authorized by the approved v2 plan, WO-005, or WO-005a. They are out-of-scope/rogue validation findings and do not create hard stops for this fix.
- **Hard-stop findings:** `none`.
- **Fix work order required:** `no`.
- **Exact next state:** The orchestrator may mark [`WO-005a-invalidate-parenthetical-provenance.md`](../work-orders/WO-005a-invalidate-parenthetical-provenance.md:5) complete under the normal Forge closeout process and may then advance the authorized WO-005 fix chain according to initiative policy. Do not dispatch work for the rejected typed/deleted or caret-position findings under WO-005a; any future behavior change must receive a separately approved scope and handoff.

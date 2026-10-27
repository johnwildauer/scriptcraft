# Validation — WO-005a Invalidate Parenthetical Provenance on Edit (v2)

## Result

- **Validation result:** `FAIL`
- **Validated work order:** [`WO-005a-invalidate-parenthetical-provenance.md`](../work-orders/WO-005a-invalidate-parenthetical-provenance.md:1)
- **Original work order:** [`WO-005-parenthetical-entry-policy.md`](../work-orders/WO-005-parenthetical-entry-policy.md:1)
- **Failing validation context:** [`validation-WO-005-v2.md`](validation-WO-005-v2.md:1)
- **Initiative:** [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:1)
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `architect-fresh-context-editor-geometry-corrections-v2-WO-005a-validation-2026-10-26`
- **Session exception:** `none`
- **Work-order state at validation:** `in-review`; this validator did not change the state
- **Fix chain:** `WO-005a`
- **Product source modified by validator:** `no`

## Fresh comparison basis and boundary

I independently read the validation rules in [`05-validation.md`](../../../process/05-validation.md:1), the v2 parenthetical oracle in [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:73), the reconciled v2 review, the original and fix work orders, the failing validation, the v2 execution log, the applicable requirement and core blueprint, the current editing/model/persistence source, the focused parenthetical tests, the format rules, and the project manifest.

The comparison is meaningful. The fix work order is `in-review`; its implementation evidence identifies [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:360), [`wo005_parenthetical.rs`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:77), the core blueprint, the original work order, and the execution log. The execution log records the fix dispatch, implementation, focused test, full gate evidence, and `in-review` transition at [`execution-log.md`](../execution-log.md:88). The current source contains the reported stable-ID marker invalidation and the existing Enter/Tab paths.

This validation assesses only: ordinary-edit invalidation of generated-parenthetical provenance; preservation of a reconstructed user-authored pair; generated cleanup; explicit entry and caret behavior; undo/redo, failed-edit rollback, and save/reopen persistence boundaries; the user-commissioned cursor-position and text-presence behavior; scope and durable impact; rogue behavior; and the required Cargo gates. It does not assess geometry, casing, blink, pointer placement, long-document responsiveness, unrelated persistence, or native visual behavior outside the reported parenthetical interaction.

## Evidence assessment

| Boundary | Status | Fresh evidence and assessment |
|---|---|---|
| Transient provenance boundary | **PASS** | [`EditingSession`](../../../../crates/scriptcraft-core/src/editing.rs:61) keeps generated IDs outside [`ScreenplayElement`](../../../../crates/scriptcraft-core/src/lib.rs:107). [`Snapshot`](../../../../crates/scriptcraft-core/src/editing.rs:53) includes the set, and [`undo()`](../../../../crates/scriptcraft-core/src/editing.rs:328) / [`redo()`](../../../../crates/scriptcraft-core/src/editing.rs:344) restore it. The marker remains transient, stable-ID keyed, undoable, and absent from the semantic persistence model. |
| Ordinary edits invalidate changed generated elements | **PASS** | [`mutate()`](../../../../crates/scriptcraft-core/src/editing.rs:360) clones the live project and prior marker set, then invokes [`invalidate_changed_generated_parentheticals()`](../../../../crates/scriptcraft-core/src/editing.rs:410) before retaining cleanup eligibility. The helper compares the live and candidate stable element records and removes a marker when a marked record changes at [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:420). This covers successful text edits and other successful candidate changes without clearing the marker for the internal generated insertion before commit. |
| Edit-away/reconstruct regression | **PASS** | [`editing_away_and_reconstructing_generated_pair_makes_it_user_authored()`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:77) changes generated `()` to `(`, reconstructs `()`, presses Enter, and asserts the pair remains `()` while a Dialogue element is created. This directly closes [`EGC-V2-WO-005-F-001`](validation-WO-005-v2.md:38). |
| Generated cleanup predicate and existing entry behavior | **PASS** | [`cleanup_generated_parenthetical()`](../../../../crates/scriptcraft-core/src/editing.rs:497) still requires Parenthetical role, exact `()`, and a live generated marker. Dialogue→Parenthetical Tab and explicit role assignment still create `()` with caret byte offset `1` at [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:267) and [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:307); focused coverage remains at [`wo005_parenthetical.rs`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:23). |
| Partial, non-empty, user-authored, pasted, undo/redo, persistence, and rollback behavior | **PASS by inspected evidence** | The focused suite retains initial partial/non-empty and user-authored/pasted preservation at [`wo005_parenthetical.rs`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:59) and [`wo005_parenthetical.rs`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:92), provenance restoration through undo/redo and non-persistence through save/reopen at [`wo005_parenthetical.rs`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:109), and the shared transactional clone/validate boundary at [`mutate()`](../../../../crates/scriptcraft-core/src/editing.rs:364). No failed candidate is committed before marker invalidation or history recording. |
| User-gated typed/deleted behavior | **FAIL as a requested interaction expectation** | The user-gated observation reports that after text is entered inside generated parentheses and subsequently deleted, Tab changes to Dialogue but does not delete the parentheses. The current implementation intentionally clears the marker on the first successful changed record, so later cleanup cannot remove the pair; this is consistent with the original provenance rule that an edited/reconstructed pair becomes user-authored, but it does not satisfy the newly reported expectation that the prior empty-pair behavior should continue. This expectation is not covered by the current fix work order or focused regression and requires an explicit product-policy decision in the next handoff rather than being inferred from the original oracle. |
| User-commissioned text/cursor policy | **FAIL** | The newly commissioned behavior is not implemented or tested. When a Parenthetical contains text, Tab/Enter should preserve the pair and create a new Dialogue line only when the caret is immediately before the final `)`; at other interior positions those commands should be disabled. When the caret is before the opening `(`, Enter should insert an empty Parenthetical above it and move the existing element down. Current [`tab()`](../../../../crates/scriptcraft-core/src/editing.rs:253) unconditionally maps Parenthetical to Dialogue after its cleanup attempt, and current [`enter()`](../../../../crates/scriptcraft-core/src/editing.rs:236) always applies the generic Parenthetical→Dialogue split. No caret-position oracle or regression for these cases exists in [`wo005_parenthetical.rs`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:1). |
| Scope and rogue-behavior check | **PASS for implemented fix; new request requires handoff update** | The implemented change remains confined to the transactional editing core, focused test, authorized blueprint/work-order evidence, and execution log. No second text buffer, rich-text pairing, arbitrary bracket completion, new role, renderer path, or deceptive, backdoor, malicious, nefarious, or otherwise rogue behavior was observed. The commissioned cursor policy is a new editing contract and is currently outside the fix work order's explicit out-of-scope statement for new delimiter behavior at [`WO-005a-invalidate-parenthetical-provenance.md`](../work-orders/WO-005a-invalidate-parenthetical-provenance.md:24); it must be added to an approved follow-on handoff before implementation. |
| Durable-document impact | **PASS WITH NOTE** | The transient marker clarification is recorded in [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:72) and the original work order, with no schema or durable provenance field. The newly commissioned caret-position/text-presence policy would change the durable parenthetical contract and therefore requires corresponding approved updates to the work order, blueprint, and—if promoted beyond the initiative—the writing-loop requirement. This validator did not modify those artifacts. |

## Hard-stop findings

### EGC-V2-WO-005A-F-001 — User-gated typed/deleted parenthetical expectation is unmet

- **Severity:** hard stop; the user-gated behavior is not satisfied.
- **Evidence:** The reported manual interaction is: generated `()` accepts text; after that text is deleted, Tab changes to Dialogue but leaves the parentheses. The current implementation clears the marker on the first successful changed element record through [`invalidate_changed_generated_parentheticals()`](../../../../crates/scriptcraft-core/src/editing.rs:410), so [`cleanup_generated_parenthetical()`](../../../../crates/scriptcraft-core/src/editing.rs:497) correctly refuses to delete the now user-authored pair.
- **Assessment:** This behavior is aligned with the original WO-005a provenance-invalidation oracle, not with the newly reported user-gated expectation that the empty generated-pair behavior continue after typed content is removed. Because the user has explicitly gated the result, the discrepancy cannot be dismissed as a warning. The next implementation handoff must state whether typed-then-deleted content preserves user authorship or restores generated cleanup eligibility, and must add a deterministic regression for that decision.
- **Required correction:** Record the authoritative typed/deleted policy, implement it without weakening edit-away/reconstruct preservation, and add focused Enter/Tab coverage for both the marker transition and resulting text/role behavior.

### EGC-V2-WO-005A-F-002 — Commissioned caret-position parenthetical policy is not implemented

- **Severity:** hard stop; a user-commissioned required behavior is absent.
- **Evidence:** Current [`enter()`](../../../../crates/scriptcraft-core/src/editing.rs:236) performs generic cleanup and then splits at the current caret; current [`tab()`](../../../../crates/scriptcraft-core/src/editing.rs:253) performs generic cleanup and then maps Parenthetical to Dialogue without checking whether the caret is at the end of the inner text. The focused suite has no tests for a text-bearing pair with the caret at the final interior boundary, an interior caret, or a caret before the opening delimiter.
- **Required correction:** Add the commissioned policy to the approved fix handoff: preserve a text-bearing pair and create a new Dialogue line only at the boundary immediately before `)`; consume/no-op Enter and Tab at other interior positions; and make Enter before `(` insert an empty Parenthetical above the current element. Define exact element roles, caret positions, undo/redo behavior, UTF-8 boundary behavior, and failure semantics, then implement and test the policy.
- **Required next state:** Do not mark [`WO-005a-invalidate-parenthetical-provenance.md`](../work-orders/WO-005a-invalidate-parenthetical-provenance.md:5) complete. Because this is a new commissioned contract beyond the current fix's explicit scope, the orchestrator must amend and re-approve the fix handoff or create the next letter-suffixed fix work order before dispatching a fresh implementer.

## Required Cargo gates

The fix work order and execution log report passing results for [`cargo fmt --all -- --check`](../../../../Cargo.toml:1), [`cargo check --workspace`](../../../../Cargo.toml:1), [`cargo test --workspace`](../../../../Cargo.toml:1) with 93 tests and 0 doc tests, [`cargo clippy --workspace --all-targets -- -D warnings`](../../../../Cargo.toml:1), [`cargo build --workspace`](../../../../Cargo.toml:1), and [`git diff --check`](../../../../Cargo.toml:1) at [`WO-005a-invalidate-parenthetical-provenance.md`](../work-orders/WO-005a-invalidate-parenthetical-provenance.md:43) and [`execution-log.md`](../execution-log.md:92). The focused [`cargo test -p scriptcraft-core --test wo005_parenthetical`](../../../../Cargo.toml:1) is recorded as passing with six tests.

These gates were **not independently rerun** in this repository-reading validation context. Under the project manifest, no manual/external gate is configured as a hard-stop policy gate at [`project-manifest.md`](../../../project-manifest.md:88). The recorded Cargo results are therefore accepted as evidence with an explicit rerun limitation; they do not cure either user-gated implementation finding. Native typing smoke was not performed by the implementer and is not claimed here; the typed/deleted observation is recorded as user-gated evidence supplied for this validation, not as native evidence inferred from headless tests.

## Warnings and notes

### W-001 — Cargo gates are recorded, not independently rerun

- **Severity:** warning.
- **Evidence:** Final passing results are recorded in [`execution-log.md`](../execution-log.md:92) and the fix handoff, while this validation context provided repository inspection and artifact-writing tools rather than command execution.
- **Recommendation:** Preserve the recorded results and rerun the full gate chain after the required policy/handoff correction and implementation.
- **Required next state:** Note only; the hard-stop findings independently require follow-on implementation and validation.

### W-002 — No standalone branch diff was available

- **Severity:** warning.
- **Evidence:** Comparison used the changed-file record, execution log, current source, tests, and durable documents. No standalone Git diff output was available in this inspection context.
- **Recommendation:** Attach the comparable branch/file diff to the next implementation handoff if available; do not represent this artifact as a direct patch review.
- **Required next state:** Note only; the file-level evidence was sufficient for meaningful comparison under [`05-validation.md`](../../../process/05-validation.md:20).

## Final verdict and exact next state

- **Implementation alignment:** **Fail.** The original WO-005a provenance hard stop is resolved in source and deterministic regression coverage: changed generated elements lose their marker, reconstructed `()` remains user-authored, and the existing cleanup, undo/redo, persistence, rollback, scope, and durable boundaries remain aligned. However, the user-gated typed/deleted expectation is not satisfied, and the newly commissioned cursor-position/text-presence parenthetical policy is neither in the current fix scope nor implemented.
- **Hard-stop findings:** `EGC-V2-WO-005A-F-001`, `EGC-V2-WO-005A-F-002`.
- **Fix work order required:** `yes` — retain WO-005a in `in-review`; amend/re-approve its scope or create a letter-suffixed follow-on fix before implementation, then obtain a separate fresh validation.
- **Exact next state:** Do not mark WO-005a or WO-005 complete. Record the user-commissioned parenthetical policy and the authoritative typed/deleted provenance rule in the next approved handoff; add deterministic tests for generated cleanup, typed/deleted content, end-of-text caret behavior, interior-caret no-op behavior, before-opening-delimiter Enter behavior, undo/redo, persistence, and failed-edit rollback; dispatch a new implementer; rerun all required Cargo gates; and obtain a new fresh-context validation. Do not validate WO-006 from this result and do not claim unperformed native typing success.

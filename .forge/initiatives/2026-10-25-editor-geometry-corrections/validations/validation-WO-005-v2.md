# Validation — WO-005 Parenthetical Entry Policy (v2)

## Result

- **Validation result:** `FAIL`
- **Validated work order:** [`WO-005-parenthetical-entry-policy.md`](../work-orders/WO-005-parenthetical-entry-policy.md:1)
- **Initiative:** [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:1)
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `architect-fresh-context-editor-geometry-corrections-v2-WO-005-validation-2026-10-26`
- **Session exception:** `none`
- **Work-order state at validation:** `in-review`; this validator did not change the state
- **Fix chain:** `none`
- **Product source modified by validator:** `no`

## Fresh comparison basis and boundary

I independently read the validation process in [`05-validation.md`](../../../process/05-validation.md:1), the authoritative v2 plan and parenthetical oracle in [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:73), the reconciled design review in [`design-review-v2.md`](../reviews/design-review-v2.md:146), the approved work order and implementation handoff, the v2 execution log, the applicable requirement and core blueprint, the current editing/model/persistence source, the focused WO-005 tests, and the available changed-file/equivalent evidence.

The comparison is meaningful. The work order is `in-review`; its changed-file record names [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:1), [`wo005_parenthetical.rs`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:1), and the authorized durable documents; the execution log records the matching dispatch, implementation, focused test, full-gate evidence, and status transition at [`execution-log.md`](../execution-log.md:82); and the current source contains the reported transient set, snapshot, entry, cleanup, persistence, and transactional paths.

This validation assesses only generated-parenthetical provenance, Enter/Tab cleanup, explicit role assignment, caret/UTF-8/undo behavior, user-authored/pasted/partial/save-reopened preservation, scope, durable impact, rogue behavior, and the required Cargo gates. It does not assess geometry, casing, blink, pointer placement, long-document responsiveness, native typing success, or unrelated persistence behavior.

## Evidence assessment

| Boundary | Status | Fresh evidence and assessment |
|---|---|---|
| Transient provenance boundary | **PASS** | [`EditingSession`](../../../../crates/scriptcraft-core/src/editing.rs:61) stores generated IDs outside [`ScreenplayElement`](../../../../crates/scriptcraft-core/src/lib.rs:107) in [`generated_parentheticals`](../../../../crates/scriptcraft-core/src/editing.rs:69). [`Snapshot`](../../../../crates/scriptcraft-core/src/editing.rs:53) copies that set, and undo/redo restores it at [`EditingSession::undo()`](../../../../crates/scriptcraft-core/src/editing.rs:328) and [`EditingSession::redo()`](../../../../crates/scriptcraft-core/src/editing.rs:344). This matches the approved transient, stable-ID, undoable, non-schema provenance design. |
| Explicit role assignment and Tab entry | **PASS** | [`EditingSession::change_role()`](../../../../crates/scriptcraft-core/src/editing.rs:307) and the Dialogue/Parenthetical cases in [`EditingSession::tab()`](../../../../crates/scriptcraft-core/src/editing.rs:253) create `()` only when the resulting Parenthetical text is empty, mark the stable ID, and return caret byte offset `1`. Focused tests cover explicit assignment and Dialogue→Parenthetical Tab at [`wo005_parenthetical.rs`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:23) and [`wo005_parenthetical.rs`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:37). |
| Enter/Tab cleanup predicate | **PASS for the stated predicate** | [`cleanup_generated_parenthetical()`](../../../../crates/scriptcraft-core/src/editing.rs:460) requires Parenthetical role, exact text `()`, and the generated-ID mark before clearing text and removing the mark. [`EditingSession::enter()`](../../../../crates/scriptcraft-core/src/editing.rs:236) and [`EditingSession::tab()`](../../../../crates/scriptcraft-core/src/editing.rs:253) call it on the transactional clone before applying the next role operation. The focused tests establish empty generated cleanup for both Enter and Tab. |
| Caret, UTF-8, transaction, and undo/redo behavior | **PASS** | The generated caret is offset `1`, a valid boundary between ASCII delimiters. The surrounding session validates candidate project/selection before commit in [`EditingSession::mutate()`](../../../../crates/scriptcraft-core/src/editing.rs:360); failed edits therefore do not replace live state. The focused suite checks caret placement and provenance restoration through undo/redo at [`wo005_parenthetical.rs`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:93), while the core editing tests cover invalid UTF-8 offsets and rollback at [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:653) and [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:761). |
| Non-empty, partial, user-authored, and pasted preservation | **FAIL** | Standalone unmarked user-authored/pasted pairs and initially partial/non-empty values are preserved by the focused tests at [`wo005_parenthetical.rs`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:59) and [`wo005_parenthetical.rs`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:76). However, ordinary editing operations do not invalidate provenance: [`insert()`](../../../../crates/scriptcraft-core/src/editing.rs:142), [`delete()`](../../../../crates/scriptcraft-core/src/editing.rs:163), [`backspace()`](../../../../crates/scriptcraft-core/src/editing.rs:190), and [`split()`](../../../../crates/scriptcraft-core/src/editing.rs:226) all leave the generated-ID set unchanged. Consequently, a marked generated `()` can be edited to a partial form and then manually reconstructed as `()`; the stable ID remains marked and the next Enter/Tab deletes the reconstructed user-authored pair. This contradicts the exact oracle that user-authored and partial edits are preserved, and the focused tests do not exercise this edit-away/recreate path. |
| Save/reopen and non-persistence | **PASS** | The semantic model has no provenance field, and [`persistence::serialize()`](../../../../crates/scriptcraft-core/src/persistence.rs:56) serializes only [`Project`](../../../../crates/scriptcraft-core/src/persistence.rs:56). The focused test verifies no `generated` marker is serialized and that a reopened user-authored `()` is preserved at [`wo005_parenthetical.rs`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:93). [`EditingSession::replace_with_saved()`](../../../../crates/scriptcraft-core/src/editing.rs:124) also starts a new session with an empty provenance set. |
| Scope and rogue-behavior check | **PASS** | The changed-file/equivalent evidence is limited to the transactional editing core, one focused core integration test, the core blueprint, the writing-loop requirement, the work order, and execution evidence. No second text buffer, rich-text pairing, arbitrary bracket completion, new role, renderer path, or unrelated product area was observed. No deceptive, backdoor, malicious, nefarious, or otherwise rogue behavior was found. The execution log records WO-006 as undispatched at [`execution-log.md`](../execution-log.md:86). |
| Durable-document impact | **PASS WITH NOTE** | The implementation updates the core blueprint and writing-loop requirement with the transient provenance contract at [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:72) and [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:52), matching the work order's authorized documentation boundary at [`WO-005-parenthetical-entry-policy.md`](../work-orders/WO-005-parenthetical-entry-policy.md:42). The work order retains conditional `Requirements: update` wording, which is the minor handoff warning noted by the reconciled design review at [`design-review-v2.md`](../reviews/design-review-v2.md:168). No schema or durable provenance field was introduced. |

## Hard-stop finding

### EGC-V2-WO-005-F-001 — Provenance survives user edits that reconstruct `()`

- **Severity:** hard stop; required preservation criterion is unmet.
- **Evidence:** The approved oracle requires cleanup only for a marked element that remains Parenthetical and exactly `()`, while explicitly preserving user-authored and partial delimiters at [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:80) and [`WO-005-parenthetical-entry-policy.md`](../work-orders/WO-005-parenthetical-entry-policy.md:44). The implementation removes the mark only during cleanup or when changing away from Parenthetical at [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:314), but normal text edits do not clear it at [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:142), [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:163), and [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:190).
- **Reproducible behavior from source:** Enter a generated Parenthetical, edit its delimiters so the text is partial, then insert/delete the missing delimiter so the final text is again `()`. The stable element ID remains in `generated_parentheticals`; the next Enter/Tab satisfies the cleanup predicate and clears text. The final pair was reconstructed by the user, but is treated as generated.
- **Impact:** This can delete canonical user-authored parentheses and violates the requested partial/user-authored preservation boundary. The five focused tests prove initial unmarked partial/user-authored/pasted cases, but do not prove provenance invalidation after an edit to a marked pair.
- **Required correction:** Invalidate the generated mark on any user text mutation that changes the marked element, while retaining the mark for the internal generated insertion itself and preserving the approved undo/redo snapshot behavior. Add a focused regression that edits a generated pair to partial text, reconstructs `()`, and verifies Enter/Tab preserves it; retain the existing initial user-authored, pasted, non-empty, save/reopen, and cleanup tests. Re-run fresh validation after the fix.
- **Required next state:** Keep [`WO-005-parenthetical-entry-policy.md`](../work-orders/WO-005-parenthetical-entry-policy.md:5) `in-review`; create and dispatch a letter-suffixed fix work order from a new implementer session, then obtain a separate fresh validation artifact. Do not mark WO-005 `complete`.

## Required Cargo gates

The implementer and execution log report passing results for [`cargo fmt --all -- --check`](../../../../Cargo.toml:1), [`cargo check --workspace`](../../../../Cargo.toml:1), [`cargo test --workspace`](../../../../Cargo.toml:1), [`cargo clippy --workspace --all-targets -- -D warnings`](../../../../Cargo.toml:1), [`cargo build --workspace`](../../../../Cargo.toml:1), and [`git diff --check`](../../../../Cargo.toml:1) at [`WO-005-parenthetical-entry-policy.md`](../work-orders/WO-005-parenthetical-entry-policy.md:50) and [`execution-log.md`](../execution-log.md:86). The focused [`cargo test -p scriptcraft-core --test wo005_parenthetical`](../../../../Cargo.toml:1) is also recorded as passing with five tests.

These gates were **not independently rerun** in this tool-only validation context. The recorded results are specific and internally consistent with the inspected source and tests, so they are recorded as passing evidence rather than as a gate blocker. The project manifest configures no required manual/external gate at [`project-manifest.md`](../../../project-manifest.md:88); native typing smoke was not performed and no native success is claimed.

## Warnings and notes

### W-001 — Cargo gates are recorded, not independently rerun

- **Severity:** warning.
- **Evidence:** The final gate results are recorded in [`execution-log.md`](../execution-log.md:86), but this validation context had repository-reading and artifact-writing tools only.
- **Recommendation:** Preserve the recorded results and the explicit independent-rerun limitation.
- **Required next state:** Note only; the hard-stop provenance finding independently requires a fix work order.

### W-002 — No standalone branch diff output was available

- **Severity:** warning.
- **Evidence:** The comparison used the explicit changed-file record, execution log, current source, tests, and durable-document updates. No standalone Git diff result was available through the inspection tools.
- **Recommendation:** Attach the normal comparable diff if the orchestrator later has it; do not represent this validation as a direct patch review.
- **Required next state:** Note only; the available file-level evidence was sufficient for meaningful comparison under [`05-validation.md`](../../../process/05-validation.md:20).

## Final verdict and exact next state

- **Implementation alignment:** **Fail.** The transient provenance design, explicit entry paths, cleanup predicate, UTF-8-safe caret, transactional history, persistence exclusion, scope, and durable-document updates align in the covered cases. The implementation does not preserve a user-authored pair reconstructed after editing a generated pair because provenance remains attached to the stable ID through ordinary text mutations.
- **Hard-stop finding:** `EGC-V2-WO-005-F-001`.
- **Fix work order required:** `yes` — a narrowly scoped letter-suffixed fix within WO-005's editing/provenance boundary.
- **Exact next state:** Keep WO-005 `in-review`; create the letter-suffixed fix work order with the required provenance-invalidation regression, dispatch a new implementer session, rerun the recorded Cargo gates, and obtain a separate fresh validation. Do not complete WO-005, do not validate WO-006 from this result, and do not claim native typing evidence.

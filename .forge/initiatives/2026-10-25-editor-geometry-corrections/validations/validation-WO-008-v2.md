# Validation — WO-008 Dialogue Enter Split Policy (v2)

## Validation metadata

- **Validation type:** `implementation alignment and anti-rogue validation`
- **Status:** `PASS WITH WARNINGS`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `architect-fresh-context-editor-geometry-corrections-WO-008-validation-v2-2026-10-27`
- **Session exception:** `none`
- **Date:** 2026-10-27
- **Initiative:** [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:1)
- **Validated work order:** [`WO-008-dialogue-enter-split-policy.md`](../work-orders/WO-008-dialogue-enter-split-policy.md:1)
- **Work-order state at validation:** `in-review`; this validation does not mark the work order complete.
- **Fix work order required:** `no`
- **Product-code changes:** none
- **Existing-artifact changes:** none; this validation artifact is the only artifact created by this validation.

## Validation boundary and comparison basis

This fresh validation covers only the authorized WO-008 behavior: collapsed
Dialogue Enter at the UTF-8-safe end versus an interior boundary, empty Dialogue,
exact text/role/ID/order/selection/history/dirty/revision behavior, rejected
operations and rollback, preservation of non-Dialogue Enter policy, save/reopen
and deterministic layout impact, scope/rogue behavior, durable-document alignment,
and the required Cargo gates. It does not reopen completed WO-005 or the
WO-007 → WO-007a → WO-007b → WO-007c predecessor chain, and it does not validate
Tab, Parenthetical policy, geometry, performance, or native typing.

The comparison basis was:

- Forge validation rules in [`05-validation.md`](../../../process/05-validation.md:1).
- The final reconciled WO-008 design review in [`design-review-wo-008.md`](../reviews/design-review-wo-008.md:158).
- The approved handoff and implementation evidence in [`WO-008-dialogue-enter-split-policy.md`](../work-orders/WO-008-dialogue-enter-split-policy.md:36).
- The WO-008 dispatch, implementation report, and final gate record in [`execution-log.md`](../execution-log.md:128).
- The amended requirement in [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:46).
- The updated core and feature blueprint clauses in [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:63) and [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:137).
- Current production editing behavior in [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:257), including [`EditingSession::mutate()`](../../../../crates/scriptcraft-core/src/editing.rs:481) and [`split_at()`](../../../../crates/scriptcraft-core/src/editing.rs:754).
- Focused core evidence in [`wo008_dialogue_enter.rs`](../../../../crates/scriptcraft-core/tests/wo008_dialogue_enter.rs:22), app dispatch evidence in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1321), and broader execution evidence in [`wo008_e2e.rs`](../../../../crates/scriptcraft-core/tests/wo008_e2e.rs:77).
- The completed predecessor validation in [`validation-WO-007bc-v2.md`](validation-WO-007bc-v2.md:1).

The implementation evidence, current source, focused tests, durable clauses, and
execution record are sufficiently consistent for a meaningful comparison. No
standalone Git diff was available to this repository-reading validator, but the
changed-file record, bounded source inspection, focused tests, and execution log
provide an adequate equivalent comparison basis.

## Acceptance findings

### PASS-001 — WO008-AC-001: end-boundary Dialogue preserves Action transition

- **Severity:** pass
- **Evidence:** [`dialogue_end_and_empty_enter_keep_action_transition()`](../../../../crates/scriptcraft-core/tests/wo008_dialogue_enter.rs:22) covers both non-empty and empty Dialogue at `text.len()`. It asserts that the original element remains Dialogue, retains its original ID and exact text, the inserted element is an empty Action, and the collapsed selection is element `1`, offset `0`.
- **State evidence:** The focused test asserts revision `1`, dirty state true, and an available undo entry. The production transaction in [`EditingSession::mutate()`](../../../../crates/scriptcraft-core/src/editing.rs:481) records one pre-mutation snapshot and increments revision once when the project/selection changes.
- **Assessment:** The end-boundary oracle is implemented without changing the original element identity or text.
- **Required next state:** None.

### PASS-002 — WO008-AC-002: interior ASCII and Unicode splits preserve Dialogue semantics

- **Severity:** pass
- **Evidence:** [`interior_ascii_and_unicode_enter_split_dialogue_exactly_once()`](../../../../crates/scriptcraft-core/tests/wo008_dialogue_enter.rs:48) checks an ASCII split and a Unicode split at the byte boundary after `前`. It asserts exact prefix/suffix strings, both roles remain Dialogue, the original ID remains on the prefix, the inserted ID is distinct and bounded, and the new selection is collapsed at the suffix element offset `0`.
- **Production evidence:** [`EditingSession::enter()`](../../../../crates/scriptcraft-core/src/editing.rs:257) selects Dialogue when the collapsed position is before `item_text.len()`, while [`split_at()`](../../../../crates/scriptcraft-core/src/editing.rs:754) slices at the supplied byte boundary, retains the original element, creates the bounded collision-checked ID through [`split_id()`](../../../../crates/scriptcraft-core/src/editing.rs:776), and returns the inserted element at offset `0`.
- **Undo/redo evidence:** The focused test snapshots the split result and selection, verifies undo restores the exact original project and original caret, then verifies redo restores the exact split project and split selection. Revisions advance once for the split, once for undo, and once for redo; dirty state returns clean on undo and dirty on redo.
- **Assessment:** Exact byte preservation, stable identity, selection placement, one-step history, and reversible behavior are aligned.
- **Required next state:** None.

### PASS-003 — WO008-AC-003: empty Dialogue and rejected selection/capacity cases

- **Severity:** pass
- **Evidence:** The empty case is explicitly included in [`dialogue_end_and_empty_enter_keep_action_transition()`](../../../../crates/scriptcraft-core/tests/wo008_dialogue_enter.rs:22), proving that zero length is treated as the end boundary. [`rejected_dialogue_enters_leave_every_session_observable_unchanged()`](../../../../crates/scriptcraft-core/tests/wo008_dialogue_enter.rs:123) compares a non-collapsed invalid-selection result and a capacity failure against complete pre-operation session clones.
- **Failure details:** Non-collapsed Enter returns [`EditError::InvalidSelection`](../../../../crates/scriptcraft-core/src/editing.rs:35); a full project returns [`EditError::TooManyElements`](../../../../crates/scriptcraft-core/src/editing.rs:35). Both preserve project, selection, history availability, dirty state, revision, and transient session fields through the clone-first transaction boundary.
- **UTF-8 details:** The same focused test verifies that setting a caret inside the four-byte `🙂` encoding returns [`EditError::InvalidUtf8Boundary`](../../../../crates/scriptcraft-core/src/editing.rs:35) without changing the session. [`validate_position()`](../../../../crates/scriptcraft-core/src/editing.rs:561) uses the element's UTF-8 boundary predicate, and [`mutate()`](../../../../crates/scriptcraft-core/src/editing.rs:481) validates the candidate before committing it.
- **Assessment:** Required rejection and rollback behavior is aligned. The direct invalid-boundary Enter call is not possible through the public selection API because the selection setter rejects it first; the equivalent public failure and production validation paths are covered, with this evidence boundary retained as a warning below rather than treated as an implementation defect.
- **Required next state:** None.

### PASS-004 — WO008-AC-004: non-Dialogue Enter behavior remains unchanged

- **Severity:** pass
- **Evidence:** The existing role matrix in [`enter_matrix_covers_every_role()`](../../../../crates/scriptcraft-core/src/editing.rs:884) still expects Scene Heading→Action, Action→Action, Character→Dialogue, Dialogue→Action at the end, Parenthetical→Dialogue, Transition→Scene Heading, and Note→Note. The WO-008 app regression [`enter_key_dispatch_splits_interior_dialogue_without_role_drift()`](../../../../apps/scriptcraft/src/main.rs:1321) exercises the app's `Key::Enter` route and confirms only the interior Dialogue result changes.
- **Source comparison:** The new branch is limited to the Dialogue role and an interior offset in [`EditingSession::enter()`](../../../../crates/scriptcraft-core/src/editing.rs:321). The existing transition matrix remains in [`enter_transition()`](../../../../crates/scriptcraft-core/src/editing.rs:827), and the app maps Enter through [`key_event()`](../../../../apps/scriptcraft/src/main.rs:726) rather than a second editor path.
- **Predecessor compatibility:** The independently validated WO-007 chain remains `PASS WITH WARNINGS` in [`validation-WO-007bc-v2.md`](validation-WO-007bc-v2.md:122). No current evidence reopens its Parenthetical, strict-boundary, rendered-route, persistence, or delimiter-only contracts.
- **Assessment:** No non-Dialogue role drift, Tab change, Parenthetical-policy change, or predecessor rewrite was observed.
- **Required next state:** None.

### PASS-005 — WO008-AC-005: save/reopen, layout durability, and failed-edit safety

- **Severity:** pass
- **Evidence:** [`dialogue_split_save_reopen_preserves_roles_ids_and_unicode_bytes()`](../../../../crates/scriptcraft-core/tests/wo008_dialogue_enter.rs:170) serializes and deserializes a Unicode Dialogue split and asserts project equality, exact Unicode prefix/suffix bytes, both Dialogue roles, and both stable IDs through project equality. The broader [`representative_headless_writing_loop_survives_every_boundary()`](../../../../crates/scriptcraft-core/tests/wo008_e2e.rs:77) proves deterministic repeated layout construction and native atomic save/reopen for the same session model.
- **Transactional failure evidence:** The focused capacity and invalid-selection tests compare complete session state before and after failure. The shared [`mutate()`](../../../../crates/scriptcraft-core/src/editing.rs:485) does not push history, replace the project, clear redo, increment revision, or recompute dirty state until candidate validation succeeds.
- **Persistence boundary:** The saved project includes semantic text, roles, IDs, and profile inputs; caret, history, and dirty/revision session state are transient as required by [`AC-WRITING-003.1`](../../../requirements/features/screenplay-writing-loop.md:75). No schema or layout geometry change was introduced.
- **Assessment:** Durable semantic impact and deterministic layout/persistence compatibility are aligned. The focused WO-008 save test does not rebuild the layout immediately after its split, but the same project is layout-tested deterministically in the broader E2E evidence and the feature blueprint preserves this contract.
- **Required next state:** None.

## Scope, rogue behavior, and durable-document findings

### PASS-006 — Scope and anti-rogue alignment

- **Severity:** pass
- **Evidence:** The work order limits product changes to the collapsed Dialogue Enter branch, its focused core coverage, optional app dispatch coverage, and the named requirements/blueprint amendments at [`WO-008-dialogue-enter-split-policy.md`](../work-orders/WO-008-dialogue-enter-split-policy.md:66). Current source shows no new setting, second text buffer, persistence schema change, layout replacement, cache change, backdoor, deceptive behavior, or unrelated product feature.
- **Assessment:** The implementation is subordinate to the approved semantic editing boundary. Completed WO-005/WO-007 history was not rewritten or reopened. No clearly unrelated or rogue behavior was found.
- **Required next state:** None.

### PASS-007 — Durable requirements and blueprint impact is correctly realized

- **Severity:** pass
- **Evidence:** The requirement amendment at [`AC-WRITING-001.4a`](../../../requirements/features/screenplay-writing-loop.md:48) explicitly distinguishes end-boundary Dialogue→Action from interior Dialogue→Dialogue and records the transactional refinement immediately below it. The core contract at [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:67) and feature facts at [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:137) match the implemented behavior. The work order correctly classifies both requirements and blueprint impact as `update` and keeps [`format-rules.md`](../format-rules.md:1) unchanged.
- **Assessment:** The durable impact is not contradicted by the implementation. The changed durable clauses are exact, bounded, and compatible with the completed predecessor chain.
- **Required next state:** None.

## Required Cargo gates and evidence warnings

### WARN-001 — Required Cargo gates were recorded, not independently rerun here

- **Severity:** warning
- **Evidence:** The final implementation record reports passing [`cargo fmt --all -- --check`](../../../../Cargo.toml:1), [`cargo check --workspace`](../../../../Cargo.toml:1), [`cargo test --workspace -- --nocapture`](../../../../Cargo.toml:1), [`cargo clippy --workspace --all-targets -- -D warnings`](../../../../Cargo.toml:1), [`cargo build --workspace`](../../../../Cargo.toml:1), and [`git diff --check`](../../../../Cargo.toml:1), including 5 WO-008 integration tests, 3 historical E2E tests, 32 app tests, 44 core unit tests, and zero doc-test failures, at [`execution-log.md`](../execution-log.md:131). This repository-reading session had no command-execution tool, so those results were not rerun independently.
- **Assessment:** The records are specific, current, and internally consistent with the inspected source/tests. Under [`05-validation.md`](../../../process/05-validation.md:23), this is an evidence warning, not a blocker; no project-policy-required native/manual hard-stop gate is configured in [`project-manifest.md`](../../../project-manifest.md:88).
- **Required next state:** Note only; do not create a fix work order on this basis.

### WARN-002 — Native typing smoke remains unavailable and unclaimed

- **Severity:** warning
- **Evidence:** The implementation record explicitly says native typing smoke was not performed and is not inferred at [`execution-log.md`](../execution-log.md:130). The app test is a headless production dispatch test, not native-window evidence.
- **Assessment:** This is optional/unavailable under current project policy and does not invalidate semantic core or rendered event-path evidence.
- **Required next state:** Preserve as an honest follow-up; do not claim native typing passed.

### WARN-003 — Direct invalid-boundary Enter regression is represented indirectly

- **Severity:** warning
- **Evidence:** [`wo008_dialogue_enter.rs`](../../../../crates/scriptcraft-core/tests/wo008_dialogue_enter.rs:143) proves invalid-boundary rejection through [`set_selection()`](../../../../crates/scriptcraft-core/src/editing.rs:136), then compares the unchanged session. It does not invoke [`EditingSession::enter()`](../../../../crates/scriptcraft-core/src/editing.rs:257) with an internally invalid selection, because the public API prevents constructing that state.
- **Assessment:** Production code validates the candidate selection and the capacity/non-collapsed Enter failures exercise the actual Enter transaction. This is an evidence-shape limitation, not a plainly unmet criterion or a reason to fail the implementation.
- **Required next state:** Note only; no fix work order required.

## Final verdict and exact next state

- **Implementation alignment:** **PASS.** WO-008 implements the approved end/interior/empty Dialogue policy, exact UTF-8-safe prefix/suffix preservation, original/new ID behavior, collapsed suffix selection, transactional history/revision/dirty semantics, rollback, app Enter dispatch, save/reopen compatibility, and non-Dialogue preservation. The durable requirement and blueprint clauses match the implementation, and no scope or rogue-behavior finding was identified.
- **Warnings:** Recorded Cargo gates were not independently rerun in this reading-only session; native typing smoke was not performed and is not inferred; invalid-boundary Enter is covered through the public validation boundary rather than an impossible internally-invalid public session; and the focused save test relies on broader E2E evidence for deterministic layout construction.
- **Fix work order required:** `no`.
- **Exact next state:** The orchestrator should mark [`WO-008-dialogue-enter-split-policy.md`](../work-orders/WO-008-dialogue-enter-split-policy.md:5) `complete`, link this validation from the work order and [`execution-log.md`](../execution-log.md:1), preserve all warnings above, and proceed to the initiative's normal closeout/next-work-order transition. Do not claim native typing smoke, do not reopen the completed WO-005/WO-007 predecessor chain, and do not create a fix work order for the recorded evidence warnings.

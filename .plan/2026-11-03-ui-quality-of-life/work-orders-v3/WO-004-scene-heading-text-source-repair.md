# V3-WO-004 — Scene Heading text/source synchronization repair

- **Status:** Corrected narrow source-projection repair validated by the user; focused Scene Heading regression passed 1/0, with broader validation and acceptance pending
- **Initiative:** [`ui-quality-of-life-v3-derived-refresh-and-scene-heading.md`](../ui-quality-of-life-v3-derived-refresh-and-scene-heading.md)
- **Depends on:** [`V3-WO-003`](WO-003-scene-heading-role-cycle-oracle.md); second user-run evidence localizes the failure to whitespace-only direct source children being included in the semantic text projection
- **Validation owner:** User; no agent-run checks, reviews, or validation

## Objective

Repair the source-text projection defect that made Scene Heading insertion compare semantic text against element-only formatting whitespace while preserving strict mismatch detection.

## Scope

1. Use WO-003's second user-run result and exact before-state to select the narrow ownership boundary: direct text-child classification in retained Text-run mutation.
2. Change only that boundary. Keep [`replace_text_element_children()`](../../../crates/scriptcraft-core/src/preservation.rs:2821) and its semantic-old-text/source-segment equality check intact as a detection guard.
3. Preserve canonical UTF-8 text, run attributes, paragraph identity, generated scene attachments, exact selection, and transactional history through the repaired transition.
4. Extend the WO-003 sequence through subsequent typing, clearing/retyping, undo/redo, in-memory FDX serialization/reopen, and the original failing insertion. Add only the cases needed to prove the established branch.
5. Retain StructuralAttachmentConflict for unknown/authored attachments and all existing split/join/role-transition safe-stops not directly implicated by the diagnosed defect.
6. Record the root cause, exact production edit, and authored-but-unrun coverage in this order. Update durable documentation only if the reviewed ownership contract changes.

## Boundaries

- Do not force source text to match semantic text after a mismatch, skip validation, or convert mismatch into silent success.
- Do not implement generated-container deletion; that belongs exclusively to V3-WO-005.
- Do not change Cast List scheduling/correction or zoom behavior.
- Do not rewrite semantic/source content to compensate for an unresolved managed-source target.

## Structural acceptance

- Every operation in the regression maintains matching semantic and retained source text/runs.
- The original TextReplacement failure no longer occurs for the supported sequence.
- Rejected unsupported edits leave the complete prior session unchanged, and native save/reopen retains source-owned data.

## Handoff

V3-WO-005 addresses deletion of recognized generated scene containers as a separate, deterministic ownership change.

## WO-008 closeout disposition

- **Final disposition:** The earlier blocked disposition was superseded by the first user-run failure, but the resulting source-lookup diagnosis and repair were ineffective. A second user run reproduced the same insertion failure; the corrected whitespace-projection diagnosis and repair are recorded below. User validation and acceptance remain pending.
- **Evidence:** [`WO-003 second user-run result`](WO-003-scene-heading-role-cycle-oracle.md#second-user-run-result-and-diagnosis-supersession--2026-11-04) records the repeated failure and corrected before-state. Neither user-run result is a passing test.

## Execution record — 2026-11-04

- **Diagnostic gate/evidence:** The user ran `cargo test -p scriptcraft-core --test v3_wo003_scene_heading_role_cycle -- --nocapture`. `generated_empty_heading_enter_tab_cycle_refresh_and_insert_stay_synchronized` failed at `session.insert("INT. ROOM - DAY")` (reported line 212) with left `Err(Preservation(ManagedTextMismatchAt(TextReplacement)))` and right `Ok(())`. Every preceding per-operation semantic/source synchronization assertion passed, including immediately after restoring the caret to element 1, offset 0. This user-supplied result satisfies WO-004's gate; the agent did not run the reproduction.
- **Before-state and failing operation:** The selected Scene Heading had empty semantic text/runs and its identity-resolved retained direct Text projection was empty. The exact failing operation is insertion at byte offset 0 into that cycled heading, after Enter, Action -> Character -> Transition -> Scene Heading Tab transitions, derived refresh, and the separately guarded generated-attachment role-change probe.
- **Initial diagnosis and first repair (superseded):** The first interpretation attributed the mismatch to a stale managed-paragraph path, and added an ownership check/reindex retry in [`managed_element_mut()`](../../../crates/scriptcraft-core/src/preservation.rs:2269). The user's second run reproduced the same failure at the original insertion. This outcome disproves that inference for the observed failure; the identity safeguard remains defensive but is not the root-cause repair.
- **Transactional and preservation boundaries:** The correction resolves the exact managed source node before existing source mutation. Text/run updates remain in the existing detached transactional candidate; failure still rolls back. Generated SceneProperties/SceneArcBeats are not rewritten or removed by this fix. Existing role-transition, split/join, unknown/authored attachment, and structural deletion safe-stops remain unchanged.
- **Authored-but-unrun coverage:** [`v3_wo003_scene_heading_role_cycle.rs`](../../../crates/scriptcraft-core/tests/v3_wo003_scene_heading_role_cycle.rs) continues the original failing sequence through subsequent typing, undo/redo, whole-text clearing, retyping, undo/redo of retyping, and in-memory FDX serialization/reopen. It checks synchronized semantic/source text and runs after each edit/history step, stable in-session identity, reopened foreign paragraph identity/role/text, and generated SceneProperties/SceneArcBeats retention. The before/after insertion assertions also prove direct formatting whitespace exists and remains. None of this coverage has been run by the agent.
- **Files/interfaces/API:** Current production change is in [`preservation.rs`](../../../crates/scriptcraft-core/src/preservation.rs), private `direct_text_child_indexes()`; the prior private `PreservationState::managed_element_mut()` ownership check is retained. Regression: [`v3_wo003_scene_heading_role_cycle.rs`](../../../crates/scriptcraft-core/tests/v3_wo003_scene_heading_role_cycle.rs). Records: this order, [`WO-003`](WO-003-scene-heading-role-cycle-oracle.md), and the active v3 record. No public API impact.
- **Validation still required:** User must rerun the exact WO-003 command, then validate the extended regression and relevant preservation/serialization coverage, and review the change. No agent-run builds, tests, lint, formatting, implementation checks, reviews, or independent validators were performed. The editor had surfaced unrelated existing diagnostics in unchanged tests; their status remains unknown and they were not investigated.

## Second user-run failure and corrected repair — 2026-11-04

- **Second user result:** After the prior patch and expanded test, the user reran `cargo test -p scriptcraft-core --test v3_wo003_scene_heading_role_cycle -- --nocapture`. It still failed at the same original insertion, now at [`v3_wo003_scene_heading_role_cycle.rs`](../../../crates/scriptcraft-core/tests/v3_wo003_scene_heading_role_cycle.rs:236), with left `Err(Preservation(ManagedTextMismatchAt(TextReplacement)))`, right `Ok(())`. The user explicitly says this is not a later typing/undo failure. No agent-run reproduction was performed.
- **Complete before-state and why the synchronization oracle passed:** The cycled heading had empty semantic text/runs and no nonempty child `<Text>`-element projection. Its generated scene metadata had element-only formatting whitespace represented as direct Text/CData children at the paragraph level. The integration test's [`source_runs()`](../../../crates/scriptcraft-core/tests/v3_wo003_scene_heading_role_cycle.rs:36) deliberately collects only child `<Text>` elements, so its before-state assertions pass while omitting layout whitespace. The failing replacement takes another path: when there are no child `<Text>` elements, [`replace_managed_text()`](../../../crates/scriptcraft-core/src/preservation.rs:1621) used [`direct_text_child_indexes()`](../../../crates/scriptcraft-core/src/preservation.rs:3127), which included all direct Text/CData nodes. Its source concatenation was therefore formatting indentation, not the empty semantic old text.
- **Corrected root cause:** The source/semantic projection disagreement was created by using all direct text leaves as editable semantic content despite the established paragraph contract that whitespace-only direct leaves are element-only formatting. No source target mismatch or earlier semantic/source text divergence is needed to explain the failure. The per-operation check and replacement guard compared different projections: the former only Text-element runs, the latter also direct formatting whitespace.
- **Current narrow production repair:** [`direct_text_child_indexes()`](../../../crates/scriptcraft-core/src/preservation.rs:3127) now excludes whitespace-only direct Text/CData nodes from the semantic text segments, but does not remove, rewrite, or reorder those nodes. If no semantic direct text remains, the existing supported empty-paragraph path creates a child `<Text>` element while retaining surrounding source whitespace. The existing mismatch guard in [`replace_text_element_children()`](../../../crates/scriptcraft-core/src/preservation.rs:2960) is unchanged. The cached-path identity check remains as a separate defense, not as a proven fix.
- **Regression coverage:** The same role-cycle regression now asserts the direct element-only whitespace is present immediately before insertion and preserved immediately after. Existing subsequent typing, clearing/retyping, history, and FDX reopen assertions remain. This coverage and the corrected implementation are authored but unrun.
- **Scope exclusions:** No mismatch guard, attachment policy, Cast List behavior, deletion, zoom, WO-002, WO-005, or WO-007 behavior changed. User owns all builds, tests, lint, formatting, implementation checks, reviews, and validation under [`project-manifest.md`](../../../.forge/project-manifest.md:73).

## User validation result — 2026-11-04

- The user reran `cargo test -p scriptcraft-core --test v3_wo003_scene_heading_role_cycle -- --nocapture` after the direct-whitespace projection repair. `generated_empty_heading_enter_tab_cycle_refresh_and_insert_stay_synchronized` passed: 1 passed, 0 failed. This is user-supplied execution evidence for the corrected repair and its extended typing, history, whitespace-preservation, and FDX reopen assertions; no agent-run validation occurred.

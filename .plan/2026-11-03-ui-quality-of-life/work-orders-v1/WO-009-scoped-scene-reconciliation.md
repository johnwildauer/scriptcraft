# RS-WO-009 — Affected-scene derived reconciliation

- **Status:** Implementation plus WO-010 structural integration submitted 2026-11-03; user review and validation pending; not user-validated
- **Initiative:** [`ui-quality-of-life-v1-render-speed.md`](../ui-quality-of-life-v1-render-speed.md)
- **Depends on:** [`RS-WO-008`](WO-008-revision-gated-derived-refresh.md)
- **Authorization:** Covered by the user's 2026-11-03 authorization for sequential execution of all twelve orders; authorization is not review, validation, or acceptance
- **Validation owner:** User; no agent-run checks or independent validators

## Objective

Remove redundant whole-project snapshots and per-heading whole-preservation-state clones from actual derived refreshes, while retaining detached rollback and per-scene deferral.

This order changes candidate construction, not derived names, arc ownership, Cast List content, or save behavior.

## Implementation steps

1. Trace and remove duplicate original/candidate layers across [`EditingSession::refresh_cast_lists()`](../../../crates/scriptcraft-core/src/editing.rs:227), [`correct_cast_lists()`](../../../crates/scriptcraft-core/src/cast_list.rs:37), and [`reconcile_project_with_outcome()`](../../../crates/scriptcraft-core/src/scene_metadata.rs:39).
2. Retain one detached top-level candidate where required for atomic publication. Use explicit changed/deferred/no-op outcomes instead of cloning an additional original solely for full-project equality.
3. Replace the per-heading complete preservation clone in [`reconcile_project_with_outcome()`](../../../crates/scriptcraft-core/src/scene_metadata.rs:39) with a candidate scoped to the affected managed heading and its generated SceneProperties/SceneArcBeats subtree.
4. Share global identity, reference, and source-layout inputs read-only. Clone/mutate only data owned by the active scene candidate.
5. Publish each supported scene candidate atomically; on unsupported metadata, discard only that scene's attempted candidate and retain the complete original subtree.
6. Avoid complete-project equality and revalidation of untouched source when no candidate changed. Preserve full candidate checks where changed global invariants require them.
7. Keep normalized Character association, uppercase names, nested arc text/runs/IDs, Cast List ordering, formatted-list safe-stops, and nonblocking Save/Save As behavior unchanged.
8. Keep explicit save preparation complete even when no scheduled refresh has run.

## Structural acceptance

- A refresh touching one supported scene does not deep-clone unrelated scene subtrees or original source bytes.
- A no-op refresh performs no full-project equality and does not advance revision.
- Deferred scenes preserve their complete original generated/unsupported subtree while independent supported scenes still update.
- Fatal errors publish no partial live project state.

## Regression coverage to author

Cover single/multiple scenes, changed and unchanged candidates, deferred scenes, nested arc content and IDs, Character add/remove/rename, formatted lists, preservation-free documents, failure atomicity, and explicit save before/after refresh. Add candidate/clone counters and compare complete derived output with the previous implementation. Authored order-specific core cases cover single project-candidate ownership and per-heading candidate counts, preservation-free no-op behavior, fatal later-scene failure atomicity, and exact deferred-heading retention while an independent scene publishes; desktop diagnostic projection assertions cover the numeric counter labels. Existing cases continue to cover nested IDs, role/name changes, formatted lists, history, and save preparation. A previous-implementation complete-output golden was not present, so literal old-versus-new golden comparison remains a user-review limitation rather than a fabricated baseline claim.

## Durable documentation

Document affected-scene candidate ownership in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:152) without weakening any preservation contract. Existing derived-scene requirements remain behaviorally unchanged.

## Implementation record — 2026-11-03

The derived correction path now creates one detached top-level [`Project`](../../../crates/scriptcraft-core/src/lib.rs:464) candidate for atomic publication. Session refresh and Save/Save As preparation share this complete candidate-preparation path, so save correctness does not depend on the WO-008 timer watermark. Cast/scene reconciliation mutates that detached candidate in place, tracks explicit changed/deferred outcomes, validates the complete project only when derived data changed, and does not use whole-project equality for no-op detection. Character projection now scans cue references directly rather than cloning scene body elements.

Imported scene reconciliation shares global identity/reference/source-layout inputs read-only and clones only each managed heading subtree being attempted. Successful heading candidates publish individually; unsupported metadata discards that heading candidate, preserves the complete original subtree and its IDs, and allows independent supported headings to continue. Fatal errors return before the caller publishes its detached top-level candidate. Source-owned opaque data, normalized Character association, uppercase names, nested arc text/runs/IDs, Cast List ordering, formatted-list deferral, and global identity/reference safety remain on the existing path.

Value-free diagnostics expose project-candidate, scene-candidate, and redundant preservation-state clone counts. WO-009 regression coverage was authored for candidate counts, changed/unchanged correction, preservation-free no-op behavior, independent-scene deferral, and failure atomicity; existing derived-scene/Cast List/history/save coverage remains in place. WO-010 adds core-level preservation sharing/detachment counters; they are not desktop-projected. Coverage is authored but unrun. No builds, tests, lint, formatting, manual verification, implementation checks, reviews, or independent validation were performed; user review and validation remain pending. No timing result or user acceptance is claimed.

**Structural integration record:** WO-010 changes preservation storage to immutable `Arc`-backed vectors and payloads, so the detached top-level candidate shares original source bytes and untouched preserved subtrees rather than deep-copying them. Mutable access detaches shared vectors/payloads; managed paragraph mutation follows an immutable index path and detaches only vectors on that path. The existing detached candidate, scene-local publish/defer behavior, fatal-error rollback, and complete save preparation remain unchanged. WO-009 plus this direct WO-010 integration therefore meets the stated no-original-byte/no-unrelated-subtree-copy structural criterion by implementation design. This is not user-reviewed or validated, and no acceptance claim is made. Authored structural assertions remain unrun.

The WO-010 integration points are [`corrected_cast_list_candidate()`](../../../crates/scriptcraft-core/src/cast_list.rs:69), [`reconcile_project_candidate_with_outcome()`](../../../crates/scriptcraft-core/src/scene_metadata.rs:56), and [`prepare_save_candidate()`](../../../crates/scriptcraft-core/src/cast_list.rs:150). The structural criterion is now addressed through the authorized immutable-sharing design, without weakening detached rollback, scene-local deferral, full Save/Save As candidate preparation, or native save publication. WO-011 remains separately authorized and not started; its managed lookup and change-scoped validation work is not included here.

## User validation

Run derived Character/scene, Cast List, history, and native-save suites. Verify supported scenes update, deferred/formatted content remains unchanged, undo/redo remains coherent, and Save/Save As/reopen produces the same derived output as the baseline.

## Handoff

RS-WO-010 applies the same immutable-sharing principle to ordinary editing snapshots so untouched preserved source is not deeply cloned per keystroke.

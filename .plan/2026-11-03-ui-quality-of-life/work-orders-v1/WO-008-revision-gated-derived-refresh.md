# RS-WO-008 — Revision-gated derived refresh scheduling

- **Status:** Implemented 2026-11-03; user review and validation pending; not user-validated
- **Initiative:** [`ui-quality-of-life-v1-render-speed.md`](../ui-quality-of-life-v1-render-speed.md)
- **Depends on:** [`RS-WO-007`](WO-007-indexed-layout-interactions.md)
- **Authorization:** Covered by the user's 2026-11-03 authorization for sequential execution of all twelve orders; authorization is not review, validation, or acceptance
- **Validation owner:** User; no agent-run checks or independent validators

## Objective

Make the one-second derived-refresh timer a no-op unless a new relevant revision requires reconciliation, and prevent scheduled work from unnecessarily delaying queued editor input.

This order changes scheduling only. RS-WO-009 reduces the cost of an actual changed-revision reconciliation.

## Implementation steps

1. Track the last attempted/completed derived-refresh revision and its disposition at the desktop/session boundary used by [`refresh_cast_lists_on_schedule()`](../../../apps/scriptcraft/src/main.rs:245).
2. Return immediately on an unchanged-revision tick before [`EditingSession::refresh_cast_lists()`](../../../crates/scriptcraft-core/src/editing.rs:227) can clone the project.
3. Preserve the existing one-second maximum cadence for changed Character/scene content and perform an initial refresh after open/new-document activation when required.
4. Handle revision changes from edits, undo/redo, refresh itself, Save/Save As rebase, and document replacement without creating a refresh loop or leaving a changed revision indefinitely pending.
5. Place scheduled refresh after already queued editor input has been dispatched, preserving RS-WO-003 same-frame input behavior.
6. Record attempted, skipped, completed, changed, unchanged, and deferred refresh counts through RS-WO-001 diagnostics.
7. Keep Save/Save As candidate preparation independent of timer state so an explicit save never waits for or skips required derived correction.

## Structural acceptance

- An unchanged revision causes zero project/source clones and zero scene reconciliations on timer ticks.
- One changed revision causes at most one scheduled reconciliation until another relevant revision occurs.
- Unchanged attempts do not advance revision or alter dirty state, history, selection, or layout identity.
- Save/Save As always prepares the complete current derived candidate regardless of timer disposition.
- Deferred/error behavior is explicit and does not accidentally suppress all future correction attempts.

## Regression coverage to author

Cover repeated idle ticks, one edit followed by many ticks, refresh followed by undo/redo, save before pending refresh, save-triggered revisions, document replacement, deferred refresh, and preservation-free documents. Use test-only counters to assert clone/reconciliation suppression rather than elapsed time.

## Durable documentation

Record revision-gated scheduling in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:43) and clarify the unchanged one-second derived-update behavior in [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:24).

## Implementation record — 2026-11-03

The desktop scheduler now remembers the session revision successfully refreshed for the active document. It checks the due time first, then compares the current revision before calling the core refresh API; a same-revision due tick increments the skipped counter and returns without entering project cloning or reconciliation. A successful refresh records the post-refresh revision, including when derived data itself advanced the revision. Opened-document activation clears the watermark even when a replacement has the same numeric revision. Ordinary edits, undo, and redo advance the revision and therefore re-arm one scheduled pass. Refresh remains at most once per one-second cadence; input dispatch now runs before scheduled derived work, while the pending migration modal continues to exclude both.

Successful Save/Save As still prepares its complete corrected detached candidate independently of timer state. Only after successful publication/rebase does the app mark the resulting session revision current for the timer. Failed saves do not alter the watermark. Successful refresh outcomes distinguish changed, unchanged, and safely deferred work; a failed attempt records a retryable failed disposition that does not cover the revision and is retried on the next cadence, while deferred source regions are recorded and retried after a later relevant revision rather than repeatedly reconciled while the project is unchanged. Privacy-safe diagnostics now report attempted, completed, skipped, changed, unchanged, and deferred counts; completion denotes successful return, not necessarily a project change.

Authored desktop coverage exercises initial and repeated idle ticks, one refresh per revision, same-update input dispatch before refresh, edit/undo/redo re-arming, replacement with the same starting revision, successful-save correction before a timer tick, and deferred work suppression/re-arming. The WO-002 oracle's unchanged/changed refresh expectations now reflect these gates. Coverage is authored but unrun; no builds, tests, lint, formatting, manual verification, implementation checks, reviews, or independent validation were performed. User review and validation remain pending.

RS-WO-009 remains the next authorized order. It may reduce the work for a genuinely changed-revision reconciliation; this order changes scheduling only and makes no speed or timing claim.

## User validation

Observe idle diagnostics before and after Character edits. Confirm no-op ticks do no work, derived Cast Lists and scene arcs still update within the established cadence, Save before a pending tick is correct, and undo/redo schedules the appropriate refresh.

## Handoff

RS-WO-009 reduces actual changed-revision reconciliation from repeated whole-state clones to affected-scene candidates.

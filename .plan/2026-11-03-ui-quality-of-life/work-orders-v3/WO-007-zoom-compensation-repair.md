# V3-WO-007 — Viewport-centered zoom compensation repair

- **Status:** Implemented — WO-006 isolated the post-layout scroll-state defect and the explicitly authorized agent-run full lifecycle command passed 1/0. Manual desktop evidence, broader validation, and user acceptance remain pending.
- **Initiative:** [`ui-quality-of-life-v3-derived-refresh-and-scene-heading.md`](../ui-quality-of-life-v3-derived-refresh-and-scene-heading.md)
- **Depends on:** [`V3-WO-006`](WO-006-zoom-input-scroll-oracle.md) and an established failing lifecycle stage
- **Validation owner:** User. The user explicitly authorized the exact focused WO-006 lifecycle command, which was executed and passed; all other checks, desktop validation, review, and acceptance remain user-owned.

## Objective

Repair the single established defect that prevents viewport-centered zoom from retaining its captured document anchor through settled scroll geometry.

## Scope

1. Use WO-006's failing stage to choose exactly one owning boundary: capture freshness/geometry, raw-input cancellation, pending-state retention, target calculation, egui scroll request ordering, or one-shot publication.
2. Change only that boundary in [`main.rs`](../../../apps/scriptcraft/src/main.rs) or [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs). Preserve the selected viewport-center policy for native pinch and status controls.
3. Retain constant page gaps and outside-page displacement, current horizontal padding, normal edge clamping, repeated-pinch coalescing, and manual-scroll precedence unless the established defect specifically disproves one of those implementations.
4. Keep caret reveal independent and do not create selection, revision, dirty/history, fixed-layout, persistence, or export changes from zoom.
5. Extend the WO-006 end-to-end scenario through the original failure and one settled follow-up frame. Retain beginning/middle/end, 75–300%, both axes, gap, edge-clamping, and document-replacement coverage only where affected by the repair.
6. Record the root cause, exact production edit, lifecycle-counter changes, and authored-but-unrun coverage here and in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:83).

## Boundaries

- Do not switch to pointer- or caret-anchored zoom, add animation, or change zoom limits/percentages.
- Do not weaken manual-scroll independence or replay compensation every frame.
- Do not change core layout/pagination or native save behavior.
- If WO-006 establishes no failing stage, stop rather than replacing the zoom mechanism speculatively.

## Structural acceptance

- The original complete input sequence preserves page index, page-local point, and outside-page displacement except at clamped edges.
- One zoom request produces at most one compensation, and the following frame does not drift.
- Selection/session/layout/export state remains unchanged and manual scrolling retains its documented precedence.

## Handoff

V3-WO-008 integrates the completed correctness repairs and records user-owned desktop, persistence, and documentation outcomes.

## WO-008 closeout disposition — historical snapshot

- **Disposition at WO-008 authoring:** Blocked; WO-006 had not yet established a failing zoom lifecycle stage. Superseded by the WO-006 user-run failure and the implementation/result below.
- **Evidence requirement at that time:** [`WO-008 user-run evidence matrix`](WO-008-integration-and-closeout.md#user-run-evidence-matrix) records the then-unpopulated complete-lifecycle outcomes. Its manual-desktop evidence fields remain outstanding.

## Implementation and focused execution — 2026-11-04

- **Established root cause:** The WO-006 diagnostic-enhanced native pinch reproduced capture success `1`, cancellation `0`, queued compensation `1`, geometry publication `1`, and anchor consumption `1`. The correctly calculated target was `[862.4, 2379.6]`, on the captured page 1, but the settled `ScrollArea` offset remained `[3.6, 13.2]` with maximum `[444.9, 63855.4]`. Thus the anchor was captured and retained, not canceled; target math and the normal clamp were not the failure. `ui.scroll_to_rect()` did not commit the calculated target into the resulting scroll state, while the lifecycle immediately treated page-geometry allocation as successful application, consumed the pending anchor, and published geometry at the uncorrected offset. The exact failed boundary is post-layout application/publication of the scroll target.
- **Production edit:** Only [`main.rs`](../../../apps/scriptcraft/src/main.rs) changed in production. `live_document_surface()` now holds the target until `ScrollArea` has resolved its inner viewport and content size, computes the centered offset from the resolved viewport center, applies the ordinary `[0, max]` clamp independently on both axes, commits the offset once to the ScrollArea state, and publishes page rectangles translated to the resulting view. This retains page-local scaling while keeping page gaps, outside-page displacement, and padding unscaled; repeated pinch keeps the first pending anchor; manual scroll still cancels before the surface pass; caret reveal remains independent; no layout, session, persistence/save, export, or public API behavior changed.
- **Coverage adjustment:** The original native lifecycle assertions were not weakened. The status-slider scenario now uses a 1,920 px-wide status-only fixture so the actual inline slider is rendered, and clicks the slider rectangle center after a far-right click proved not to change its value. The test still covers the real rendered status control, settled anchor, one-shot stability, and lifecycle counters.
- **Exact authorized agent-run command:** `cargo test -p scriptcraft wo006_full_application_zoom_inputs_preserve_settled_anchor_and_report_lifecycle -- --nocapture` (the only command run in this work order; repeated after bounded corrections).
- **Exact outcomes:** Initial diagnostic-enhanced run: exit `101`, native page-identity failure with the stage counters/target above. First production-repair run: exit `101` after the native branch passed, because the status fixture did not render an inline slider. After widening the status-only fixture: exit `101` because the far-right slider click did not change zoom. Final run after clicking the real slider center: exit `0`; selected test result `1 passed; 0 failed; 0 ignored; 0 measured; 202 filtered out`. Compilation completed with existing warnings. The passing test includes repeated native pinch plus a settled frame, status-slider input plus a settled frame, and same-frame pinch/raw-scroll cancellation. No other command or check was run.
- **Remaining manual evidence:** The exact native desktop gesture/control and platform event shape from the original report, along with actual before/after viewport observations, remain unprovided. Manual desktop verification, separate zoom tests, broader validation, review, and user acceptance remain outstanding; synthetic test success is not presented as desktop acceptance or complete project validation.

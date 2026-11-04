# V4-WO-002 — Deep-scroll prepaint lifecycle

- **Status:** Focused deep-scroll lifecycle and edge checks pass; V4-WO-003 documentation closeout recorded; manual desktop evidence remains
- **Initiative:** [`ui-quality-of-life-v4-deep-scroll-zoom-anchor.md`](../ui-quality-of-life-v4-deep-scroll-zoom-anchor.md)
- **Depends on:** [`V4-WO-001`](WO-001-content-space-anchor-capture.md)
- **Validation owner:** User for desktop review; focused test commands authorized for V4-WO-001/002

## Objective

Prove viewport-centered native and status zoom from genuine nonzero deep-scroll positions while preserving WO-009's pre-paint smoothing, and repair only any residual prepaint geometry defect exposed by that stronger oracle.

## Scope

1. Replace the zero-offset/tall-viewport-only setup in [`zoom_oracle_fixture()`](../../../apps/scriptcraft/src/main.rs:6898) with a deterministic nonzero deep-scroll starting position on an interior page. Do not delete beginning/end coverage; add deep middle coverage independently.
2. Make expected anchors independent of the production capture helper where practical: derive the known starting page/local coordinate from published content rectangles and offset rather than judging success solely with the same function being tested.
3. Exercise native pinch in both directions and the actual status slider from deep scroll. Preserve continuous-frame painted/published equality, first-anchor coalescing, one-shot consumption, normal clamping, and cancellation coverage.
4. Compare the resulting offset against the recipe's page-origin + scaled local point + outside displacement − viewport-relative center equation in [`prepaint_zoom_scroll_offset()`](../../../apps/scriptcraft/src/main.rs:2168). If a residual mismatch remains, repair only the demonstrated geometry/extents/application boundary; do not alter capture again in this order.
5. Retain all current zero-offset and edge cases as supplemental coverage, not substitutes for deep scroll. Record exact command outcomes and remaining manual evidence.

## Boundaries

- Do not start at zero offset merely to avoid capture anomalies or loosen page-identity/clamping assertions.
- Do not switch to pointer anchoring, add animation, change zoom limits, reintroduce post-paint correction, or modify session/layout/save/export state.
- Do not broaden a demonstrated residual into a wholesale scroll rewrite.

## Structural acceptance

- Deep-scroll native and status zoom preserve the original page/local/outside coordinate in both directions except at demonstrated extents.
- Every intermediate painted frame agrees with its committed offset and published rectangles.
- The oracle detects the former page-0/top-anchor failure independently of production capture assumptions.

## Handoff

V4-WO-003 remains pending. Focused synthetic lifecycle and edge acceptance now pass after the coordinate correction recorded below. Do not claim final acceptance from synthetic evidence; manual desktop observations remain outstanding.

## Initial V4-WO-002 execution record — diagnosis superseded by coordinate audit

- **Test changes:** [`zoom_oracle_fixture()`](../../../apps/scriptcraft/src/main.rs:6972) now scrolls to a nonzero deep interior position and settles egui's animated wheel target before beginning zoom. [`independent_published_zoom_anchor()`](../../../apps/scriptcraft/src/main.rs:5636) derives the expected page/local point from the published rectangles normalized into content space with the committed scroll offset, independently of `ViewportPageTracker::capture_center_anchor()`. The lifecycle case covers native pinch-in and pinch-out, six separate continuous pinch frames in both directions, status-slider increase and decrease, painted/published equality, first-anchor coalescing, one-shot consumption, cancellation, and the existing edge-control test remains unchanged.
- **Offset oracle:** Each settled zoom compares the committed offset with `page_origin + scaled_local_point + outside_page_displacement - viewport_center`, clamped to the published content extents. Anchor-oracle failures are accumulated until all native/status/cancellation paths have run, so one mismatch does not prevent later lifecycle evidence from being collected.
- **Primary focused command:** `cargo test -p scriptcraft --bin scriptcraft wo006_full_application_zoom_inputs_preserve_settled_anchor_and_report_lifecycle -- --nocapture`
- **Primary outcome:** Exit code `101`; the test ran one test and failed after executing the lifecycle paths (203 filtered out). The independent deep-start oracle selected page 5/local y `313.5` at nonzero scroll. After native pinch-in to 274%, the published geometry placed the viewport center on page 9/local y `576.7`; actual offset y was `20397.5`, while the independent page-origin recipe for the original page/local point expected y `10935.9` (no extent clamp; max y `63855.4`). The reverse pinch, six continuous pinch frames, and both status-slider directions also reported mismatches. Painted/published geometry and lifecycle-counter assertions completed without aborting; the final anchor-oracle assertion reported the collected residuals.
- **Diagnosis/boundary:** This is not an isolated prepaint extents/application arithmetic discrepancy: the full-app capture path selects a different page/local point than the independent oracle at a genuine nonzero scroll position. The recipe mismatch therefore originates before prepaint compensation. Per this order, no capture code or scrolling behavior was changed; no prepaint production repair was justified without crossing the explicit no-recapture boundary. V4-WO-001's focused unit results remain recorded as 4 passed/0 failed, but do not establish this full-application coordinate contract.
- **Supplemental edge command:** `cargo test -p scriptcraft --bin scriptcraft wo0012_desktop_zoom_keeps_the_viewport_anchor_once_or_clamps_at_an_edge -- --nocapture`
- **Supplemental edge outcome:** Exit code `101`; one test failed (203 filtered out) at the existing post-correction offset-stability assertion for beginning/middle/end and min/max zoom coverage. No edge test was loosened or rewritten.
- **Warnings/scope:** Focused builds emitted the repository's existing compiler warnings. No broad checks, format sweep, or other work order was run. V4-WO-003 was not started.
- **Manual evidence still required:** No desktop observations were made. A user must still manually verify deep-scroll native pinch and status-slider zoom in both directions, continuous-frame behavior, and beginning/end clamping after the capture-coordinate boundary is resolved under an authorized follow-up.

## Corrective coordinate and lifecycle record

- **egui contract/full-frame evidence:** egui 0.29.1 builds its ScrollArea child max rect from `inner_rect.min - state.offset`, and its viewport range from `ZERO + state.offset`. Allocated rectangles are consequently viewport-positioned; adding the committed offset restores their content origins. The independent oracle's rectangle translation was correct. Production capture had been incorrectly changed to omit the matching translation in WO-001.
- **Capture correction:** Restored the offset translation in WO-001. Deep lifecycle capture and independent expected-value geometry now identify the same original page/local point without the oracle calling production capture.
- **Demonstrated prepaint origin defect:** At native zoom 274%, the full-frame page 1 allocated rect was `[(-198.4, 53.5), (1478.4, 2223.6)]` with committed offset `(222.4, 10923.9)` and clip/outer `[(8,52), (1272,1749)]`. Translating the page rect by the offset gives content origin `(24,10977.4)`; the first page allocation translated back to y `52`, exactly the outer origin. Measured page stride was page height `2170.1` + explicit `PAGE_VERTICAL_GAP=12` + egui vertical item spacing `3` = `2185.1`. Prepaint had omitted per-row item spacing and added it as a one-time top inset, producing an anchor offset 12 points too small at page index 5. It now uses `page_height + PAGE_VERTICAL_GAP + item_spacing.y` for every stride and the extent, with no artificial top inset. WO-009 pre-paint smoothing remains unchanged.
- **Edge fixture correction:** The original edge test began zoom with egui's smoothed wheel target still in flight (horizontal offset changed from `866.3` to `1162.1` on the next frame at 300%). Added empty frames to settle the existing wheel input before anchor capture. Beginning/middle/end, min/max zoom, anchor/clamp, and one-shot stability assertions remain intact and were not relaxed.
- **Focused lifecycle command:** `cargo test -p scriptcraft --bin scriptcraft wo006_full_application_zoom_inputs_preserve_settled_anchor_and_report_lifecycle -- --nocapture`
- **Lifecycle outcome:** Exit code `0`; 1 passed, 0 failed, 203 filtered out. Native pinch both ways, all six continuous frames, both status-slider directions, painted/published geometry, lifecycle counters/coalescing/consumption, cancellation, and deep-scroll anchor assertions passed.
- **Focused edge command:** `cargo test -p scriptcraft --bin scriptcraft wo0012_desktop_zoom_keeps_the_viewport_anchor_once_or_clamps_at_an_edge -- --nocapture`
- **Edge outcome:** Exit code `0`; 1 passed, 0 failed, 203 filtered out. Beginning/middle/end and min/max zoom cases pass after deterministic wheel settling.
- **Scope/evidence:** Focused capture tests also pass (4/4, recorded in WO-001). No broad checks or desktop observations were performed. WO-003 was not started; manual desktop behavior remains outstanding.

## V4-WO-003 closeout

- **Final focused results:** `viewport_center_anchor` — 4 passed/0 failed; `wo006_full_application_zoom_inputs_preserve_settled_anchor_and_report_lifecycle` — 1 passed/0 failed; `wo0012_desktop_zoom_keeps_the_viewport_anchor_once_or_clamps_at_an_edge` — 1 passed/0 failed. Exact commands and full focused outcomes are recorded above. No commands were run for the documentation-only closeout.
- **Final diagnosis:** Capture normalizes viewport-positioned allocated page rectangles by the committed scroll offset before comparison to the content-space viewport center. The removed translation was incorrect and has been restored. Separately, prepaint page stride is page height + 12-point explicit page gap + 3-point egui item spacing on every page row; the former one-time top inset was removed. Prepaint applies the clamped target before painting, preserving painted/published geometry agreement and WO-009's smoothing.
- **Remaining evidence:** No desktop observations or acceptance are claimed. The manual evidence matrix remains blank for the user in [`WO-003`](WO-003-desktop-evidence-and-closeout.md).

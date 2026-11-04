# V4-WO-001 — Content-space anchor capture

- **Status:** Corrected coordinate contract implemented; focused anchor tests pass; V4-WO-003 documentation closeout recorded
- **Initiative:** [`ui-quality-of-life-v4-deep-scroll-zoom-anchor.md`](../ui-quality-of-life-v4-deep-scroll-zoom-anchor.md)
- **Depends on:** None
- **Validation owner:** User for desktop review/acceptance; focused test execution authorized for this order

## Objective

Correct viewport-center anchor capture using egui's actual `ScrollArea` coordinate contract: allocated page rectangles are viewport-positioned and must be normalized by the committed offset before comparison with the content-space center point.

## Scope

1. In [`ViewportPageTracker::capture_center_anchor()`](../../../apps/scriptcraft/src/status_bar.rs:623), retain `content_point = viewport_center + snapshot.scroll_offset` and translate each allocated viewport-positioned page rectangle by the same committed offset in containment and nearest-page fallback selection.
2. Replace the incorrect content-space unit fixture with actual viewport-positioned allocations in [`viewport_center_anchor_uses_actual_scroll_offset_and_page_scale()`](../../../apps/scriptcraft/src/status_bar.rs:1789), independently checking page-local position and outside-page displacement.
3. Add a multi-page fixture with a real nonzero vertical offset and a center on a later page. Assert page identity, page-local point, and outside-page displacement independently of the prepaint compensation function.
4. Retain deterministic nearest-page/gap tie-breaking and existing freshness, invalid-geometry, missing-geometry, and scale checks.
5. Record exact source/test changes and focused command outcomes only when separately authorized.

## Boundaries

- Do not change prepaint compensation, zoom percentages, anchor policy, page geometry, or persistence in this order.
- Do not assume allocated rectangles are content-space: verify egui's actual `ScrollArea` child origin and apply matching coordinate transforms.
- Do not use the production capture function as the sole independent expected-value oracle for the new deep-scroll case.

## Structural acceptance

- A nonzero offset selects the actual center page instead of page 0.
- Capture transforms viewport-positioned allocations into content space exactly once; center and page rect receive the same offset.
- Existing gap/edge and freshness behavior remains intact.

## Handoff

V4-WO-002 uses the corrected capture in full native/status prepaint lifecycles beginning from genuine nonzero deep-scroll offsets.

## Initial execution record — diagnosis superseded by coordinate audit

- **Production change:** In `ViewportPageTracker::capture_center_anchor()`, retained `content_point = viewport_center + snapshot.scroll_offset` and removed the extra `rect.translate(snapshot.scroll_offset)` from both the containment search and nearest-page fallback. Allocated page rectangles are compared directly in their existing content coordinate space.
- **Existing expectation corrected:** `viewport_center_anchor_uses_actual_scroll_offset_and_page_scale()` now expects document point `(100, 110)` and outside-page displacement `(10, 0)` for its content point `(230, 260)` against the unchanged page rectangle. This records the actual horizontal overrun rather than concealing it.
- **Independent regression added:** `viewport_center_anchor_selects_later_page_at_nonzero_scroll_offset()` uses two explicitly defined pages, a real vertical offset of `350`, scale `2`, and independently derives center content point `(200, 550)`. It asserts page index `1`, local point `(90, 75)`, and zero outside displacement. It does not use prepaint compensation as its oracle.
- **Preserved coverage:** Existing fresh/stale/mismatched/missing geometry, scale rejection, invalidation, gap tie, and two-axis outside-page checks remain unchanged. Capture still rejects invalid geometry and non-finite/negative offsets.
- **Focused command:** `cargo test -p scriptcraft --bin scriptcraft viewport_center_anchor -- --nocapture`
- **Outcome:** Exit code `0`; 4 passed, 0 failed, 200 filtered out. The build emitted existing warnings in `scriptcraft-core` and `main.rs`/`status_bar.rs`; no warning was treated as a test failure. No broader workspace checks were run.
- **Boundary:** No prepaint compensation or other zoom behavior was changed. WO-002 has not started. Native/status prepaint lifecycle evidence from genuine nonzero deep-scroll positions, including both zoom directions and continuous-frame painted/published equality, remains outstanding for WO-002; manual desktop evidence remains for WO-003.

## Corrective coordinate-contract record

- **Superseding diagnosis:** The initial v4 assumption that allocated page rectangles were content-space was wrong. egui 0.29.1 sets the `ScrollArea` child max-rect origin to `inner_rect.min - state.offset`; allocated page rectangles therefore move with scrolling in viewport/screen coordinates. The viewport clip remains screen-positioned. In full-frame evidence at native zoom 274%, offset `(222.4, 10923.9)`, and clip/outer `[(8, 52), (1272, 1749)]`, page 1's allocated y-origin was `-10871.9`; adding the committed offset restored its content y-origin to `52`, the outer origin.
- **Correct production change:** Restored `rect.translate(snapshot.scroll_offset)` in both containment and nearest-page fallback while keeping `content_point = viewport_center + snapshot.scroll_offset`. The original translation was correct; removing it caused capture and independent expected anchors to diverge. No prepaint change belongs to WO-001.
- **Corrected independent cases:** The offset/scale fixture now uses viewport-positioned rectangles and expects page-local `(75, 80)` plus outside displacement `(30, 0)`. The multi-page fixture uses viewport positions for both pages and continues to assert later page/local `(90, 75)` at offset `(0, 350)`.
- **Focused command after correction:** `cargo test -p scriptcraft --bin scriptcraft viewport_center_anchor -- --nocapture`
- **Outcome:** Exit code `0`; 4 passed, 0 failed, 200 filtered out. The initial execution record above is retained as history only; its content-space conclusion is superseded.

## V4-WO-003 closeout

- egui 0.29.1 positions ScrollArea child allocations at `inner_rect.min - state.offset`. Allocated page rectangles are therefore viewport-positioned, not content-space; capture correctly translates them by the committed offset for containment and nearest-page selection while forming the content-space center as `viewport_center + scroll_offset`.
- WO-001's initial removal of that rectangle translation was incorrect and was restored. The initial execution record remains preserved as superseded history; no production change was made during V4-WO-003.
- Final focused result remains the corrected command above: `cargo test -p scriptcraft --bin scriptcraft viewport_center_anchor -- --nocapture` — exit code `0`, 4 passed, 0 failed, 200 filtered out.
- Manual desktop evidence remains user-owned and unrecorded; see the blank evidence matrix in [`WO-003`](WO-003-desktop-evidence-and-closeout.md).

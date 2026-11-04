# UI Quality of Life v4 — Deep-scroll zoom anchor correctness

**Status:** V4-WO-001 corrected coordinate contract and V4-WO-002 focused lifecycle/edge tests pass; V4-WO-003 documentation closeout is recorded; manual desktop evidence and user acceptance remain pending
**Supersedes as active zoom context:** The unresolved anchor portion of [`ui-quality-of-life-v3-derived-refresh-and-scene-heading.md`](ui-quality-of-life-v3-derived-refresh-and-scene-heading.md), while preserving all v3 history  
**Validation owner:** User for desktop review/acceptance; agent-run commands require explicit authorization  

> **Policy boundary:** The initial v4 research used static source inspection, prior focused command evidence, the user's desktop observations, and [`09-wordcraft-zoom.md`](../../.reference/09-wordcraft-zoom.md). V4-WO-001 was subsequently authorized and changed only anchor capture and its focused unit coverage; its exact command/outcome are recorded below. No prepaint compensation, desktop automation, or broader workspace validation was performed for WO-001. [`project-manifest.md`](../../.forge/project-manifest.md:73) remains authoritative.

> **Corrective diagnosis — supersedes the initial v4 coordinate analysis below:** egui 0.29.1 creates the ScrollArea child max rect at `inner_rect.min - state.offset`, while the viewport range is `ZERO + state.offset`. Allocated page rectangles are viewport-positioned; adding the committed offset restores content coordinates. The original capture translation was correct and its removal in WO-001 was the bug. After restoring it, full-frame geometry exposed a separate prepaint stride defect: vertical item spacing belongs in every page stride, not in a one-time top inset. Both focused lifecycle and edge tests now pass; details and exact outcomes appear in the corrected execution records. WO-003 has not started.

## 1. Current outcome

| Area | Current evidence |
|---|---|
| Cast List refresh | The user's latest desktop check reports that the freezes are gone. WO-002's imported/new-Cast-List semantic-run repair and focused regression are complete. |
| Scene Heading synchronization/deletion | WO-004 and WO-005 focused regressions pass; no v4 work is proposed for these areas. |
| Zoom smoothing | WO-009 removed the user's reported continuous-pinch sputtering/flashing by applying compensation before painting. Preserve this improvement. |
| Zoom anchor | egui 0.29.1 allocations are viewport-positioned, not content-space. WO-001's initial removal of rectangle translation followed an incorrect assumption; the translation is required when comparing against `viewport_center + scroll_offset`. Capture was restored, and WO-002 corrected prepaint row stride to include egui vertical item spacing. Focused capture, deep-scroll lifecycle, continuous-frame, status-control, and edge tests pass; manual desktop evidence remains. |

## 2. Initial v4 capture diagnosis — superseded by the coordinate audit below

[`ViewportPageTracker::capture_center_anchor()`](../../apps/scriptcraft/src/status_bar.rs:623) first computes the viewport-center content point as `viewport_center + snapshot.scroll_offset`. It then calls `rect.translate(snapshot.scroll_offset)` on every allocated page rectangle before containment and nearest-page selection.

The WO-006 diagnostic history established that allocated page rectangles remain in content coordinates after real scrolling: a nonzero offset coexisted with first-page rectangles still beginning at their original content positions. In that representation, the center content point and page rectangles already share one coordinate space. Translating the rectangles by the offset again applies scroll twice and can cancel the offset during page comparison, biasing capture toward page 0 and producing a top-directed compensation.

This mechanism matches the user's report that WO-009 removed flicker but restored top anchoring. The prepaint path now sends that incorrectly captured page-0 anchor through an otherwise smooth compensation path.

## 3. Initial lifecycle interpretation — superseded by the corrective record below

1. [`zoom_oracle_fixture()`](../../apps/scriptcraft/src/main.rs:6898) starts at zero scroll offset and uses a tall viewport to place its center on page 1. At zero offset, translating page rectangles by the offset is a no-op, so the defect is invisible.
2. [`settled_zoom_anchor()`](../../apps/scriptcraft/src/main.rs:5636) calls the same production capture function used by zoom. Expected and actual anchors therefore share the same coordinate error instead of providing an independent check.
3. [`viewport_center_anchor_uses_actual_scroll_offset_and_page_scale()`](../../apps/scriptcraft/src/status_bar.rs:1789) hard-codes the current translate-the-page-rectangles assumption. Its expected value treats allocated rectangles as if they need offset translation, while the full-frame diagnostics showed content-space rectangles. The unit test and production therefore reinforce the same wrong contract.
4. Earlier wheel-based setup produced a real nonzero offset but still captured page 0. The fixture was changed to zero offset/tall viewport to satisfy the non-edge precondition, which allowed the lifecycle to proceed without diagnosing that capture anomaly.

Consequently, the WO-009 pass proves pre-paint consistency for the synthetic zero-offset case, not deep-scroll anchor correctness.

## 4. Wordcraft recipe — adopt the ordering, not product-specific policy

The supplied recipe's essential sequence is capture → update zoom/layout → recompute content geometry → solve scroll → clamp → apply scroll before paint. Wordcraft converts the old viewport/pointer location into a page-local document coordinate, then computes `new_page_origin + anchor_doc * new_page_scale - anchor_screen` and clamps to new content bounds.

ScriptCraft should retain its chosen viewport-center policy, 75–300% range, page gaps/padding, fixed document layout, and persistence semantics. Do not adopt Wordcraft's pointer anchoring, 10–500% range, fit modes, or page-column layout. WO-009 already moved compensation before paint; v4 should correct the coordinate contract and prove it from real nonzero offsets rather than redesign zoom again.

## 5. Target coordinate invariants

- Allocated page rectangles and clip are viewport-positioned; normalize page rectangles by committed offset before comparing with a content-space center point.
- A nonzero vertical offset changes the selected page predictably; it is not applied once to the center and again to page rectangles.
- The captured page-local document point is independent of raw scroll offset except for the page/gap boundary actually outside a page.
- New offset equals new page origin + scaled local point + unscaled outside-page displacement − viewport-relative center, clamped to new content extents.
- Every painted frame uses the same corrected geometry as its published offset, preserving WO-009's smoothing.
- Native pinch and status controls preserve the viewport-center anchor at beginning, middle/deep-scroll, and end positions except demonstrated edge clamping.
- Zoom remains presentation-only: no session revision, selection, dirty/history, fixed-layout, canonical content, save, or export changes.

## 6. Proposed sequential work orders

| Order | Single objective | Main boundary |
|---|---|---|
| [`V4-WO-001`](work-orders-v4/WO-001-content-space-anchor-capture.md) | Match capture and tests to egui's viewport-positioned allocations; normalize once into content space | [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs) and its focused tests |
| [`V4-WO-002`](work-orders-v4/WO-002-deep-scroll-prepaint-lifecycle.md) | Prove and, if necessary, narrowly correct prepaint compensation from actual nonzero deep-scroll offsets for native and status zoom | Full-application lifecycle in [`main.rs`](../../apps/scriptcraft/src/main.rs); no zero-offset-only oracle |
| [`V4-WO-003`](work-orders-v4/WO-003-desktop-evidence-and-closeout.md) | Record focused results, manual continuous-pinch/control observations, and the corrected durable coordinate contract | Documentation and user evidence only; no new behavior |

Execute sequentially. V4-WO-002 must not weaken tests to accommodate a residual prepaint defect; if one appears, isolate whether it is capture, content geometry, clamping, or application and repair only that boundary. V4-WO-003 must not claim desktop acceptance from synthetic passes.

## 7. Evidence still needed

Focused capture, deep-scroll lifecycle, and edge tests now pass. Final acceptance still requires the user's manual desktop observation of continuous pinch and status zoom from a deeply scrolled document, including both zoom directions and edge behavior. V4-WO-003 remains pending; the user authorized focused V4-WO-001/002 commands only.

## 7.1 Initial V4-WO-001 execution record — superseded

- **Initial implementation (incorrect):** Capture stopped translating page rectangles based on the assumption that egui allocations were in content coordinates. The initial unit fixtures modeled that same invalid coordinate space.
- **Corrected implementation/tests:** Restored rectangle translation. `viewport_center_anchor_uses_actual_scroll_offset_and_page_scale()` now models viewport-positioned allocations and expects local `(75, 80)` plus outside displacement `(30, 0)`. The independent later-page case retains page 2/local `(90, 75)` at offset `(0,350)` using viewport-positioned rectangles.
- **Exact focused command:** `cargo test -p scriptcraft --bin scriptcraft viewport_center_anchor -- --nocapture`
- **Initial outcome:** Exit code `0`; 4 passed, 0 failed, 200 filtered out, but the content-space conclusion was incorrect. Historical only.
- **Corrected outcome:** The same focused command passes 4/4 after correcting production geometry and fixtures; see WO-001 corrective record.
- **Scope boundary:** WO-001 did not modify prepaint compensation and did not start WO-002. Deep-scroll prepaint lifecycle evidence and manual desktop evidence remain outstanding.

## 7.2 Initial V4-WO-002 execution record — superseded

- **Changes:** [`wo006_full_application_zoom_inputs_preserve_settled_anchor_and_report_lifecycle()`](../../apps/scriptcraft/src/main.rs:7026) now starts after a real wheel-driven nonzero deep scroll has settled on an interior page. The expected anchor is derived independently by translating published viewport rectangles into content coordinates with the committed offset. The oracle compares the committed offset against the page-origin + scaled local point + outside displacement − viewport center recipe, clamped to the new extents. The test covers native pinch both ways, separate continuous pinch frames in both directions, the actual status slider both ways, painted/published equality, lifecycle counters/coalescing/consumption, and cancellation. The existing beginning/middle/end control remains supplemental.
- **Exact focused lifecycle command:** `cargo test -p scriptcraft --bin scriptcraft wo006_full_application_zoom_inputs_preserve_settled_anchor_and_report_lifecycle -- --nocapture`
- **Initial lifecycle outcome:** Exit code `101`; 1 test failed, 203 filtered out before restoring the capture transform and correcting prepaint row stride.
- **Exact supplemental edge command:** `cargo test -p scriptcraft --bin scriptcraft wo0012_desktop_zoom_keeps_the_viewport_anchor_once_or_clamps_at_an_edge -- --nocapture`
- **Initial edge outcome:** Exit code `101`; 1 test failed because smoothed wheel input was still active across the zoom boundary.
- **Superseded diagnosis:** The independent deep-scroll expectation exposed the incorrect WO-001 change. After restoring capture, a separate prepaint stride mismatch remained and required a narrow WO-002 repair.
- **Scope/evidence:** No broad checks or other work order was run. V4-WO-003 was not started. No manual desktop behavior was observed.

## 7.3 Corrected coordinate contract and focused outcomes

- **egui 0.29.1 contract:** ScrollArea allocations are viewport-positioned because child max rect origin is `inner_rect.min - state.offset`; adding the committed offset restores content coordinates. The clip remains viewport-positioned. The initial diagnosis in sections 2, 3, 7.1, and 7.2 is superseded.
- **Full-frame proof:** At 274% zoom, allocated page 1 rect `[(-198.4, 53.5), (1478.4, 2223.6)]` plus offset `(222.4, 10923.9)` yields content rect origin `(24,10977.4)`. The first allocated page begins at y `-10871.9`; plus offset returns y `52`, exactly the outer/clip top. Page stride is page height `2170.1` + explicit gap `12` + item spacing `3` = `2185.1`. Prepaint now includes item spacing in each row stride and the content extent, and removes the incorrect one-time top inset. WO-009 pre-paint smoothing is unchanged.
- **Edge setup:** The test now settles the existing smoothed wheel input before capture; no deep-scroll, edge, page-identity, clamp, or stability assertion was weakened.
- **Focused outcomes:** `cargo test -p scriptcraft --bin scriptcraft viewport_center_anchor -- --nocapture` — 4 passed; `cargo test -p scriptcraft --bin scriptcraft wo006_full_application_zoom_inputs_preserve_settled_anchor_and_report_lifecycle -- --nocapture` — 1 passed; `cargo test -p scriptcraft --bin scriptcraft wo0012_desktop_zoom_keeps_the_viewport_anchor_once_or_clamps_at_an_edge -- --nocapture` — 1 passed. Existing compiler warnings only; no broad checks.
- **At the time of the WO-002 execution record:** V4-WO-003 had not started. Its documentation closeout is now recorded below; manual desktop observations and user acceptance remain outstanding.

## 7.4 V4-WO-003 documentation closeout

- **Coordinate diagnosis:** egui 0.29.1 positions ScrollArea child allocations at `inner_rect.min - state.offset`; allocated page rectangles are viewport-positioned. Capture translates each rectangle by the committed offset and compares it with the content-space point `viewport_center + scroll_offset`. The original rectangle translation was correct; its initial removal in WO-001 was incorrect and was restored. Earlier contrary explanations remain in the record only as explicitly superseded history.
- **Prepaint diagnosis:** Independent of capture, every vertical page stride includes page height + 12-point explicit page gap + 3-point egui item spacing. The incorrect one-time top inset was removed. The compensated target is clamped to content extents and applied before paint, preserving the same-frame painted/published geometry and WO-009 smoothing.
- **Final focused outcomes:** `viewport_center_anchor` 4/0; `wo006_full_application_zoom_inputs_preserve_settled_anchor_and_report_lifecycle` 1/0; `wo0012_desktop_zoom_keeps_the_viewport_anchor_once_or_clamps_at_an_edge` 1/0. The exact authorized command strings and their outcomes are in the WO-001/WO-002 execution records. No command was run for this documentation closeout.
- **Manual evidence:** No desktop behavior was observed. V4-WO-003 contains the prepared evidence matrix with outcomes intentionally blank for user completion. Desktop acceptance remains unclaimed and user-owned. [`project-manifest.md`](../../.forge/project-manifest.md) is intentionally unchanged and incomplete.

## 8. Non-goals

- No pointer-anchored zoom, fit modes, new zoom range, animation system, or Wordcraft dependency.
- No change to Cast List, Scene Heading, native FDX, fixed pagination, or export behavior.
- No wholesale renderer/scroll rewrite and no return to post-paint correction.
- No invented timings, desktop observations, test outcomes, or user acceptance.

## 9. User-directed closeout — 2026-11-04

The user directed closeout and confirms that the changes from the completed work orders worked: Cast List refresh, Scene Heading synchronization and deletion, and zoom smoothing with deep-scroll anchoring. This records user confirmation only; no new validation commands or results are claimed.

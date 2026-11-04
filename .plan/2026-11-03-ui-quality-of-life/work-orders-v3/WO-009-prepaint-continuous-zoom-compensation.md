# V3-WO-009 — Pre-paint continuous-zoom compensation

- **Status:** Pre-paint smoothing implemented and preserved; its anchor behavior is superseded by the corrected V4 coordinate/stride contract. Original authorized focused synthetic lifecycle passed 1/0 with six consecutive pinch-frame geometry checks; manual desktop evidence, broader validation, and user acceptance remain pending.
- **Initiative:** [`ui-quality-of-life-v3-derived-refresh-and-scene-heading.md`](../ui-quality-of-life-v3-derived-refresh-and-scene-heading.md)
- **Depends on:** [`V3-WO-007`](WO-007-zoom-compensation-repair.md) and the user's continuous-pinch sputtering report
- **Validation owner:** User; the focused lifecycle command was explicitly authorized for this continuation and executed, while all broader checks and desktop evidence remain user-owned

## Objective

Eliminate stale/corrected viewport alternation during continuous pinch by committing each zoom's compensated scroll offset before page geometry is painted, while retaining the settled viewport-center anchor.

## Evidence

- In [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:1423), page painting currently occurs inside the ScrollArea closure before the post-pass correction commits the new offset and translates only the published rectangles. Continuous input can therefore paint stale placement on one frame and corrected placement on another.
- The user's settled zoom is now correct, but continuous zoom visibly sputters/flashes as placement changes and corrects repeatedly.
- The referenced Wordcraft path computes the requested offset, applies [`ScrollArea::scroll_offset()`](../../../craft/wordcraft/crates/ui-egui/src/canvas.rs:1301) before [`show_viewport()`](../../../craft/wordcraft/crates/ui-egui/src/canvas.rs:1308), and paints the corrected view in that same frame. This is an implementation reference, not a requirement to copy its pointer-anchor policy or architecture.

## Scope

1. Extend the existing full-application zoom oracle with a continuous native-pinch sequence that asserts every painted frame's page rectangles and actual scroll offset describe the same corrected view, not merely the final settled anchor. Detect stale/corrected alternation without timing thresholds.
2. Compute the required viewport/content extents and centered clamped offset early enough to supply the corrected offset before page allocation/paint. Preserve normal per-axis clamping and the existing first-anchor pinch coalescing.
3. Apply the corrected offset before the ScrollArea paints its child geometry, using the current egui state/offset mechanism as appropriate. Remove any post-paint state/geometry correction that can publish a view different from the one just painted.
4. Retain one compensation per zoom request, fixed page gaps/outside-page displacement, horizontal padding, manual-scroll precedence, independent caret reveal, document-replacement behavior, and final anchor assertions.
5. Run only explicitly authorized focused checks and record exact outcomes. Update [`technical-context.md`](../../../.forge/blueprints/technical-context.md) with pre-paint ordering and this order with production edits and remaining manual-desktop evidence.

## Boundaries

- Do not change viewport-center policy to pointer/caret anchoring, add zoom animation, change zoom percentages/limits, or alter core layout/pagination.
- Do not replay compensation on frames without a pending zoom request or change session, selection, dirty/history, persistence, save, or export state.
- Do not treat final-frame anchor correctness as proof that intermediate frames are stable.

## Structural acceptance

- Every continuous-pinch frame paints geometry consistent with its committed offset; no stale/corrected alternating placement is submitted.
- Settled behavior and all existing native/status/cancellation assertions remain intact.
- The first anchor is retained across coalesced pinch input and each actual request is consumed once.

## Handoff

After focused validation, record the separate need for manual continuous-pinch desktop observation. Do not expand this order into unrelated rendering optimization or further zoom redesign.

## Implementation and focused execution — 2026-11-04

- **Established rendering defect:** The WO-007 correction committed the centered scroll offset only after the ScrollArea child had already painted. It then translated only the published rectangles, so actual paint and published geometry could describe different placements during intermediate pinch frames even though settled-anchor checks passed.
- **Narrow production change:** [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:1423) now computes the pending anchor's content-space point and clamped target offset before showing the ScrollArea, passes that one-shot offset through `ScrollArea::scroll_offset()`, and paints/allocates page rectangles at the corrected offset in that same frame. It no longer applies a post-paint state/rectangle correction. The prepaint calculation uses the actual outer viewport dimensions (egui overlay scrollbars do not consume the inner extent) and includes the existing vertical item-spacing origin; it retains the first-anchor policy, content geometry, normal independent-axis clamp, manual-scroll precedence, caret reveal behavior, and one-shot publication/consumption.
- **Focused coverage:** The existing [`wo006_full_application_zoom_inputs_preserve_settled_anchor_and_report_lifecycle`](../../../apps/scriptcraft/src/main.rs:6950) scenario now includes six consecutive native pinch frames. Every frame must advance zoom, queue/publish/consume exactly one request, preserve the settled viewport-center anchor, and prove that each page rectangle passed to `paint_page()` is identical to the geometry published for that same frame. Existing repeated-pinch coalescing, rendered status slider, cancellation, settled follow-up, and lifecycle-counter assertions remain enabled; no assertion was weakened.
- **Exact authorized agent-run command and outcomes:** Only `cargo test -p scriptcraft wo006_full_application_zoom_inputs_preserve_settled_anchor_and_report_lifecycle -- --nocapture` was run for WO-009, repeated after bounded corrections. First run failed the existing horizontal page-local anchor assertion because the prepaint extent estimate incorrectly reserved overlay-scrollbar width. Second diagnostic run reproduced that 5-point target-center discrepancy. Third run, after using the actual overlay viewport dimensions, isolated a 3-point content-origin offset from UI vertical item spacing. After including that existing top spacing in the content-space target, the final run exited `0`: `1 passed; 0 failed; 0 ignored; 0 measured; 202 filtered out`. Compilation completed with existing warnings. No other command or broader validation was run.
- **Remaining evidence:** Manual continuous native-pinch observation on desktop, user review, and acceptance remain outstanding. The focused synthetic lifecycle pass is not evidence of desktop gesture/platform behavior or broad project validation.

## V4 anchor-contract supersession note — 2026-11-04

- WO-009's pre-paint timing and continuous-frame smoothing remain preserved: the compensated, clamped offset is supplied before child allocation/paint so the painted and published geometry describe the committed view in each frame.
- The anchor behavior described and validated in this v3 order is superseded by V4-WO-001/002. Under egui 0.29.1, ScrollArea child allocations are viewport-positioned (`inner_rect.min - state.offset`); capture must translate each allocated rectangle by the committed offset to compare it with `viewport_center + scroll_offset` in content coordinates. WO-001's initial removal of that translation was incorrect and was restored.
- V4 additionally corrected prepaint stride to include each page's height, the explicit 12-point page gap, and 3-point egui item spacing, with no one-time top inset. These are v4 corrections; they do not erase the WO-009 implementation/diagnostic history or its original passing 0/1-excluding focused run.
- The v4 focused deep-scroll/status/continuous-frame and edge results are recorded in [`V4-WO-002`](../work-orders-v4/WO-002-deep-scroll-prepaint-lifecycle.md). They do not establish manual desktop behavior or acceptance; see the blank matrix in [`V4-WO-003`](../work-orders-v4/WO-003-desktop-evidence-and-closeout.md). WO-009's original focused lifecycle result remains 1 passed/0 failed as recorded above.

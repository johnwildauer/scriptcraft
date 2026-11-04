# V4-WO-003 — Zoom desktop evidence and closeout

- **Status:** Documentation closeout prepared; manual desktop evidence and user acceptance pending
- **Initiative:** [`ui-quality-of-life-v4-deep-scroll-zoom-anchor.md`](../ui-quality-of-life-v4-deep-scroll-zoom-anchor.md)
- **Depends on:** [`V4-WO-002`](WO-002-deep-scroll-prepaint-lifecycle.md)
- **Validation owner:** User for manual desktop observations, review, and acceptance

## Objective

Align the durable zoom coordinate/ordering description with the validated implementation and record focused plus manual desktop evidence without claiming acceptance prematurely.

## Scope

1. Update the v4 initiative and V4-WO-001/V4-WO-002 records with exact changes, explicitly authorized command outcomes, intermediate failures, and unresolved evidence.
2. Update the zoom section in [`technical-context.md`](../../../.forge/blueprints/technical-context.md) to distinguish content-space page rectangles, viewport-relative center, page-local document coordinates, outside-page displacement, scroll offset, pre-paint application, and clamping. Preserve WO-009's smoothing contract.
3. Mark the relevant WO-009 record as smoothing-complete but anchor-superseded by the v4 outcome; do not erase its passing zero-offset lifecycle evidence or diagnostic history.
4. Prepare a concise manual matrix: deep middle zoom-in/out, continuous pinch, status slider, beginning/end clamping, and simultaneous manual scroll. Record observed behavior only when the user supplies it.
5. Record final user acceptance or remediation and update [`project-manifest.md`](../../../.forge/project-manifest.md) only if separately approved for closeout.

## Boundaries

- No production or test changes and no agent-run commands in this order.
- No claim that synthetic lifecycle passes establish native desktop feel or acceptance.
- No adoption of Wordcraft-specific pointer policy, fit modes, zoom limits, or product behavior.

## Structural acceptance

- Durable documentation states one consistent coordinate contract and pre-paint sequence.
- Every focused result is linked to its exact authorized command; every manual result is user-supplied.
- Any remaining desktop failure has a concrete reproduction and does not get converted into acceptance.

## Handoff

The zoom initiative stops after user disposition. Further rendering work requires a separately authorized scope.

## Documentation closeout — 2026-11-04

- **Scope completed:** Updated the v4 initiative, WO-001, WO-002, WO-009, and [`technical-context.md`](../../../.forge/blueprints/technical-context.md) with the corrected coordinate contract, independent prepaint stride diagnosis, exact focused results, and supersession history. This closeout changed documentation only; no commands were run and no production/test files were changed.
- **Final focused results recorded from the authorized WO-001/WO-002 runs:** `cargo test -p scriptcraft --bin scriptcraft viewport_center_anchor -- --nocapture` — exit 0, 4 passed/0 failed; `cargo test -p scriptcraft --bin scriptcraft wo006_full_application_zoom_inputs_preserve_settled_anchor_and_report_lifecycle -- --nocapture` — exit 0, 1 passed/0 failed; `cargo test -p scriptcraft --bin scriptcraft wo0012_desktop_zoom_keeps_the_viewport_anchor_once_or_clamps_at_an_edge -- --nocapture` — exit 0, 1 passed/0 failed. No broad validation is claimed.
- **Coordinate and stride record:** egui 0.29.1 positions ScrollArea child rectangles at `inner_rect.min - state.offset`. Allocated page rectangles are viewport-positioned; capture translates them by committed offset into content coordinates to compare against `viewport_center + scroll_offset`. The initial translation removal was incorrect and has been restored. Separately, prepaint stride is page height + 12-point explicit gap + 3-point egui item spacing on each row, with no one-time top inset. The target is clamped and applied before paint. WO-009's smoothing remains; its prior anchor-coordinate behavior is superseded by v4.

### Manual desktop evidence matrix — outcomes intentionally blank

Record observed behavior/reproduction notes only after the user performs the desktop check. No desktop observations have been made for this closeout.

| Check | Procedure / evidence to capture | Outcome / user notes |
|---|---|---|
| Deep middle zoom-in | Scroll to a deep interior page; zoom in around viewport center; note starting/ending page-local anchor and whether the view remains stable. | |
| Deep middle zoom-out | From a deep interior position, zoom out around viewport center; note starting/ending page-local anchor and any boundary effects. | |
| Continuous pinch | Pinch continuously in both directions from deep scroll; observe intermediate frames for jumps, flashing, or anchor drift. | |
| Status slider | At deep scroll, change zoom with the status-bar slider in both directions; note anchor retention and final position. | |
| Beginning/end clamping | Repeat zoom near the document beginning and end, including zoom limits; note expected clamping versus unexpected jumps. | |
| Simultaneous manual scroll | Initiate manual scrolling while a zoom compensation is pending/being applied; note which input takes precedence and resulting position. | |

- **Acceptance boundary:** Synthetic focused results do not prove platform gesture behavior or desktop acceptance. Do not mark [`project-manifest.md`](../../../.forge/project-manifest.md) complete from this record; manual outcomes and final acceptance remain user-owned.

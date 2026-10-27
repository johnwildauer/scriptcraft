# Initiative: Toolbar Viewport, Hover, and Final Edge Fit

## Initiative metadata

- **Phase:** `closeout`
- **Status:** `closed`
- **Owner/orchestrator:** Forge orchestrator
- **Created:** 2026-10-27
- **Updated:** 2026-10-27
- **Authoritative version:** `v1`
- **Initiative mode:** `standard`
- **Mode requested by:** `user`
- **Rapid-mode eligibility:** `not eligible` — the work spans native viewport configuration, nested hover hit regions, and final allocation bounds.
- **Parent initiative:** [`../2026-10-27-toolbar-overflow-hover/toolbar-overflow-hover-v1.md`](../2026-10-27-toolbar-overflow-hover/toolbar-overflow-hover-v1.md:1) — closed predecessor; this is a linked follow-up.
- **Base/default branch:** `dev/2026-10-26-editor-geometry-corrections-v2`
- **Initiative branch:** `dev/2026-10-27-toolbar-viewport-hover`
- **Branch policy:** `required`
- **Branch capability/evidence:** `git switch -c dev/2026-10-27-toolbar-viewport-hover` succeeded on 2026-10-27.
- **Validation cadence:** `per work order`
- **Phase transition evidence:** `draft → plan → review` on 2026-10-27 after investigation and explicit request to research/create/implement WOs. `review → execute` on 2026-10-27 after fresh design review PASS and the user's explicit request served as human approval. `execute/validate → closeout` on 2026-10-27 after WO-001, WO-002, and WO-003 each passed separate fresh validation.
- **Direct-translation confirmation:** `requested and confirmed`.

## Outcome

Make the native window chrome transparent/full-content as requested, constrain hover illumination to the icon control plus its small padding rather than the label, and ensure the rightmost toolbar group remains fully inside the actual row bounds.

## Investigation findings

- No current [`ViewportBuilder`](../../apps/scriptcraft/src/main.rs:1) configuration is present in the app entry path; the current [`main()`](../../apps/scriptcraft/src/main.rs:2780) uses the default eframe native viewport.
- [`toolbar_action_button()`](../../apps/scriptcraft/src/main.rs:446) currently allocates/interacts with the entire Item rect, so hover intentionally covers icon and label. The requested behavior requires a separate icon-control response and no event-propagation equivalent is needed; egui responses are independently allocated and only the response used for fill determines hover.
- [`toolbar_item_width()`](../../apps/scriptcraft/src/main.rs:171) estimates labels at a fixed character width, while the row uses [`allocation.row_width`](../../apps/scriptcraft/src/main.rs:545). The remaining edge discrepancy likely comes from actual font metrics, control minimums, or the outer `TopBottomPanel` content/viewport insets not represented by the fixed estimate.

## Work-order plan

1. [`WO-001-transparent-native-viewport.md`](work-orders/WO-001-transparent-native-viewport.md) — configure the requested hidden/transparent/fullsize native viewport.
2. [`WO-002-icon-only-hover-region.md`](work-orders/WO-002-icon-only-hover-region.md) — isolate hover illumination to icon plus small padding while label remains outside.
3. [`WO-003-final-toolbar-edge-budget.md`](work-orders/WO-003-final-toolbar-edge-budget.md) — reconcile actual row/viewport extents and make the rightmost group fully visible.

## Requirements and blueprints

- **Requirements:** [`screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md:90), REQ-WRITING-LOOP-005 / AC-WRITING-005.3 and AC-WRITING-005.5.
- **Blueprints:** [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md:58), desktop viewport and transient toolbar presentation ownership.

## Scope

### In scope

- Native viewport builder flags equivalent to `with_title_hidden(true)`, `with_titlebar_transparent(true)`, and `with_fullsize_content_view(true)`.
- Icon-only hover hit region with explicit small icon padding; label text is not part of hover fill.
- Actual right-edge/row-budget correction with deterministic tests and native smoke evidence where available.

### Out of scope

- New commands, toolbar ordering, core semantics, persistence, exports, page layout, global theme redesign, and platform-specific title-bar pixel claims.

## Design readiness checklist

- [x] Investigation and scope are recorded.
- [x] Three consecutive dependency-ordered work orders are defined.
- [x] Parent initiative, requirements, and blueprint links are recorded.
- [x] Validation cadence and branch evidence are recorded.
- [ ] Fresh design review and human execution approval are pending.

## Human design approval — execution authorized

- **Approval status:** `approved for execution`
- **Approver:** User
- **Approval date:** 2026-10-27
- **Approval scope:** All three consecutive work orders, executed sequentially with fresh validation after each.
- **Approval evidence:** User explicitly requested research, work-order creation, and implementation; fresh design review returned PASS with no blockers.

## Closeout follow-ups

- Native transparent/titlebar, hover, and right-edge visual inspection remains unavailable; repeat on a window-capable environment. Owner: Forge orchestrator; trigger: next native desktop validation session.

## Closeout record

- **Closeout status:** `closed`
- **Closeout date:** 2026-10-27
- **Closeout evidence:** All three work orders passed separate fresh validation; required gates passed; no fix work orders or session exceptions were required. Implementation/closeout commit `7834504`; merge evidence is recorded in the final execution log.

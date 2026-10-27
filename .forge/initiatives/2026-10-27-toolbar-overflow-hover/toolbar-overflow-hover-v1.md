# Initiative: Toolbar Overflow and Hover Correction

## Initiative metadata

- **Phase:** `closeout`
- **Status:** `closed`
- **Owner/orchestrator:** Forge orchestrator
- **Created:** 2026-10-27
- **Updated:** 2026-10-27
- **Authoritative version:** `v1`
- **Initiative mode:** `standard`
- **Mode requested by:** `user`
- **Mode confirmation:** Standard mode; the user requested these corrections as consecutive work orders in the toolbar initiative family.
- **Rapid-mode eligibility:** `not eligible` — the work covers text-aware geometry measurement plus nested widget hover/color behavior.
- **Parent initiative:** [`../2026-10-27-toolbar-final-fit/toolbar-final-fit-v1.md`](../2026-10-27-toolbar-final-fit/toolbar-final-fit-v1.md:1) — closed predecessor; this is a linked follow-up because closed initiatives are terminal under Forge.
- **Base/default branch:** `dev/2026-10-26-editor-geometry-corrections-v2`
- **Initiative branch:** `dev/2026-10-27-toolbar-overflow-hover`
- **Branch policy:** `required`
- **Branch capability/evidence:** `git switch -c dev/2026-10-27-toolbar-overflow-hover` succeeded on 2026-10-27.
- **Validation cadence:** `per work order`
- **Phase transition evidence:** `draft → plan → review` on 2026-10-27 after investigation and the user's explicit request for consecutive WOs. `review → execute` on 2026-10-27 after fresh design review PASS and the user's explicit request served as human approval. `execute/validate → closeout` on 2026-10-27 after WO-001 passed fresh validation and WO-002 passed through the WO-002a fix chain with fresh validation PASS.
- **Direct-translation confirmation:** `requested and confirmed`.

## Outcome

Ensure the Export PDF label is fully inside the toolbar by measuring the complete rendered Item group, make hover illumination apply to the full Item/button surface rather than only glyph pixels, and use `style.widgets.open.weak_bg_fill` for the toolbar chrome color.

## Scope

### In scope

- Measure/allocate the full action Item group including icon control, label width, alignment, and row budgets so Export PDF does not clip.
- Make the outer Item/button surface the hover response while keeping the icon and label visually centered.
- Use the egui open-widget weak background fill as the toolbar frame color source.
- Add deterministic text-aware geometry and hover/style-source tests, plus honest native smoke evidence.

### Out of scope

- New commands, Item ordering, role semantics, persistence, core, exports, page layout, global theme redesign, or native OS title-bar sampling.

## Investigation findings

- [`ToolbarAllocation`](../../apps/scriptcraft/src/main.rs:159) currently stores fixed widths, and [`toolbar_content_width()`](../../apps/scriptcraft/src/main.rs:169) sums them; it does not measure the actual label width.
- [`toolbar_action_button()`](../../apps/scriptcraft/src/main.rs:434) currently creates a narrow icon-only Button plus a sibling label. The Button response therefore cannot illuminate the label/outer group, matching the reported inverse hover behavior.
- The desired full Item surface should be one centered interactive outer response containing the icon and label, or an equivalent allocated response with hover fill applied to the complete group.
- [`toolbar_chrome_color()`](../../apps/scriptcraft/src/main.rs:1367) currently uses `visuals.panel_fill`; the requested replacement is `style.widgets.open.weak_bg_fill`.

## Work-order plan

1. [`WO-001-toolbar-text-aware-fit.md`](work-orders/WO-001-toolbar-text-aware-fit.md) — measure complete Item groups and correct Export PDF right-edge allocation.
2. [`WO-002-toolbar-group-hover-and-color.md`](work-orders/WO-002-toolbar-group-hover-and-color.md) — make the full group hoverable/illuminated and switch toolbar color source.

Fix chain: [`WO-002a-toolbar-hover-gate-fix.md`](work-orders/WO-002a-toolbar-hover-gate-fix.md) addresses the failed WO-002 formatting and Clippy gates only.

The work orders are consecutive: group geometry is established before hover styling is applied to the final group bounds.

## Requirements and blueprints

- **Requirements:** [`screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md:90), REQ-WRITING-LOOP-005 / AC-WRITING-005.3 and AC-WRITING-005.5.
- **Blueprints:** [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md:58), transient toolbar grouping/allocation ownership.

## Design readiness checklist

- [x] User-visible outcome, investigation, scope, and consecutive WO order recorded.
- [x] Parent closed initiative linkage recorded.
- [x] Requirements and blueprint references linked.
- [x] Validation cadence and branch evidence recorded.
- [ ] Fresh design review and human execution approval pending.

## Human design approval — execution authorized

- **Approval status:** `approved for execution`
- **Approver:** User
- **Approval date:** 2026-10-27
- **Approval scope:** Both consecutive work orders, executed sequentially with fresh validation after each.
- **Approval evidence:** User explicitly requested the Export PDF fit, full group hover, and `style.widgets.open.weak_bg_fill` changes; fresh design review returned PASS with no blockers.

## Closeout follow-ups

- Native visual hover/color/right-edge inspection remains unavailable; repeat on a window-capable environment. Owner: Forge orchestrator; trigger: next native desktop validation session.
- Exact native OS pixel equality remains unclaimed; the toolbar uses the requested egui widget color source.

## Closeout record

- **Closeout status:** `closed`
- **Closeout date:** 2026-10-27
- **Closeout evidence:** WO-001 passed fresh validation; WO-002 initially failed only formatting/Clippy gates; WO-002a repaired those gates and fresh validation passed. Git evidence will be recorded in the final execution log.

# Initiative: Toolbar Polish

## Initiative metadata

- **Phase:** `closeout`
- **Status:** `closed`
- **Owner/orchestrator:** Forge orchestrator
- **Created:** 2026-10-27
- **Updated:** 2026-10-27
- **Authoritative version:** `v1`
- **Initiative mode:** `standard`
- **Mode requested by:** `user`
- **Mode confirmation:** Standard mode is the project default; the user requested investigation, work orders, review, and execution without requesting rapid mode.
- **Rapid-mode eligibility:** `not eligible` — the change combines toolbar presentation hierarchy with a layout-allocation defect and requires independent design/validation evidence.
- **Base/default branch:** `dev/2026-10-26-editor-geometry-corrections-v2` — observed clean branch at creation.
- **Initiative branch:** `dev/2026-10-27-toolbar-polish`
- **Branch policy:** `required`
- **Branch naming convention:** `dev/YYYY-MM-DD-short-slug`
- **Branch capability/evidence:** `git switch -c dev/2026-10-27-toolbar-polish` succeeded on 2026-10-27.
- **Reduced-capability fallback or opt-out evidence:** Not applicable.
- **Branch creation failure:** `none`
- **Validation cadence:** `per work order`
- **Phase transition evidence:** `draft → plan → review` on 2026-10-27 after investigation and the user's explicit request to create and run work orders; two dependency-ordered work orders and the design-review artifact are present. `review → execute` on 2026-10-27 after fresh design review PASS and the user's explicit “investigate, create and run the WOs” instruction served as human execution approval. `execute/validate → closeout` on 2026-10-27 after both work orders passed separate fresh validation with no hard-stop findings.
- **Direct-translation confirmation:** `requested and confirmed` — the user's centered labels, Elements label, right-edge correction, and half-padding reduction translate directly into the two work orders.

## Standard artifact locations

- **Authoritative plan:** `toolbar-polish-v1.md`
- **Design review:** [`reviews/design-review.md`](reviews/design-review.md)
- **Validations:** `validations/validation-[work-order-id].md`
- **Execution log:** [`execution-log.md`](execution-log.md)
- **Work orders:** [`work-orders/`](work-orders/)

## Outcome

Make the toolbar's icon/label hierarchy readable and make its allocation mathematically fit within the available top-panel width, including reduced default gaps and a rightmost item that remains fully visible at wide and maximized window sizes.

## Initiative boundary and sizing

- **Parent roadmap or product goal:** Desktop command toolbar requirements in [`../../requirements/features/screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md:90) and the desktop container contract in [`../../blueprints/containers/scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md:58).
- **Initiative type:** `product MVP stage`
- **Single observable outcome:** Every visible action displays its icon above a centered text label, Elements also displays an `Elements` label under its dropdown, and the toolbar allocation plus reduced gaps keep the full default composition inside the available width without clipping the rightmost action.
- **Validation boundary:** Fresh design review, then separate fresh validation for each WO using deterministic allocation/UI tests and bounded native smoke where available; wide/maximized and narrow width cases must be tested.
- **Split assessment:** `keep as one initiative` — both work orders correct one toolbar presentation/allocation boundary and share the same acceptance surface.
- **If split, traceability and dependencies:** Not applicable.

## Scope

### In scope

- Stack action icons above centered text labels, with consistent sizing and the Elements label shown below its text dropdown.
- Investigate and correct the toolbar width calculation so the rendered row accounts for egui inter-item spacing, padding, and the overflow affordance rather than allocating only raw Item widths.
- Reduce the default gap between adjacent normal toolbar Items by half while preserving usable hit targets and equal Flexible Space behavior.
- Add deterministic wide/maximized, exact-fit, narrow, and right-edge regression coverage plus honest native smoke evidence.

### Out of scope

- New toolbar commands, toolbar customization/persistence, title-bar redesign, icon-library replacement, or general theme work.
- Changes to semantic role policy, project persistence, page layout, exports, editor geometry, or overflow product behavior beyond fitting the existing composition.
- Replacing the egui framework or introducing a general responsive-layout system.

## Context and motivation

The current toolbar uses a single horizontal button row. [`toolbar_action_button()`](../../apps/scriptcraft/src/main.rs:408) constructs one inline `RichText` string containing icon and label, while [`elements_toolbar_item()`](../../apps/scriptcraft/src/main.rs:436) renders only the dropdown text. The requested hierarchy requires an explicit vertical icon/label layout.

The allocation helper [`allocate_toolbar()`](../../apps/scriptcraft/src/main.rs:166) treats every non-flexible Item as exactly `TOOLBAR_NORMAL_ITEM_WIDTH` and compares the sum with `ui.available_width()`, but the renderer then places Items in [`ui.horizontal()`](../../apps/scriptcraft/src/main.rs:491), which adds egui spacing between child layouts. Flexible widths also consume all raw remaining width without reserving inter-item spacing or the actual overflow control footprint. This explains why the final item can extend past the visible panel even when the raw allocation appears to fit.

## Investigation findings

- **Observed:** `TOOLBAR_NORMAL_ITEM_WIDTH` is `88.0`, `TOOLBAR_OVERFLOW_WIDTH` is `32.0`, and the allocator counts six normal Items in the default composition while ignoring the horizontal layout gap.
- **Observed:** `ui.horizontal()` and nested `add_sized`/`allocate_ui` calls add framework spacing outside the allocator's width model; the overflow control is appended after the visible allocation.
- **Observed:** Action presentation uses `format!("{} {}", item.icon, item.label)` in one button, and Elements uses a ComboBox without a separate label beneath it.
- **Inferred:** The right-edge clipping is primarily a discrepancy between logical allocation width and actual egui layout width, compounded by no explicit row padding/gap budget.
- **Desired:** Define one shared toolbar geometry contract used by allocation and rendering, with reduced explicit gap/padding and a fully visible right edge.

## Options considered

| Option | Benefits | Costs/risks | Direction |
|---|---|---|---|
| Only reduce the fixed Item width | Quick symptom reduction | Does not account for actual egui spacing; labels can still clip | Rejected |
| Let egui allocate naturally and remove the custom allocator | Less arithmetic | Loses deterministic overflow and flexible-space guarantees | Rejected |
| Reserve framework gap/padding/overflow budgets in a shared geometry contract | Fixes root cause and preserves deterministic overflow | Requires focused geometry tests and renderer alignment | Selected |
| Stack label/icon using separate controls with independent widths | Simple visual prototype | Can drift from allocation widths and break alignment | Rejected; use one measured Item surface |

## Decisions and discussion notes

- The default visible action layout is icon above centered text; Elements displays its selected role dropdown with the centered `Elements` label below.
- The default adjacent-item padding/gap target is half the current effective gap, with the exact numeric value derived from egui spacing and recorded in the work-order evidence.
- Allocation must reserve actual row gap/padding and the overflow affordance before distributing Flexible Space; it must never rely on raw Item-width sums alone.
- Toolbar presentation state remains transient and must not enter native project persistence.

## Requirements and blueprint references

- **Requirements:** [`screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md:90) — REQ-WRITING-LOOP-005, especially AC-WRITING-005.3 and AC-WRITING-005.5; [`product-overview.md`](../../requirements/product-overview.md:54) — dependable desktop writing loop.
- **Blueprints:** [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md:58) — transient toolbar Item and allocation contracts; [`technical-context.md`](../../blueprints/technical-context.md:31) — desktop presentation remains above the headless core.

## Proposed design

Introduce a toolbar geometry/presentation contract that computes the available content width after explicit row padding, inter-item gap, and (when needed) overflow-control reservation. Use the same gap value when painting the row and when deciding fit. Action and Elements Items render a vertically stacked centered surface with icon/text hierarchy; overflow menu entries retain their existing text behavior. Flexible Spaces divide only the remaining measured width.

## Work-order plan

1. [`WO-001-toolbar-label-hierarchy.md`](work-orders/WO-001-toolbar-label-hierarchy.md) — stack icons above centered labels for actions and add the Elements label, with focused presentation tests.
2. [`WO-002-toolbar-width-and-gap-correction.md`](work-orders/WO-002-toolbar-width-and-gap-correction.md) — align allocation with rendered spacing/padding/overflow budgets, halve default gaps, and add wide/right-edge/narrow regressions.

WO-001 precedes WO-002 so geometry work measures the final vertical Item surfaces.

## Risks and decisions

- **Risk:** Vertical labels increase Item height and may affect the top panel's interaction size. **Mitigation:** use a bounded shared Item surface and test hit targets without changing editor layout.
- **Risk:** Reducing gaps too far can make controls visually crowded. **Mitigation:** halve the current default gap only, preserve normal hit width, and validate at wide/narrow widths.
- **Risk:** Fixing only the allocator or only the renderer can recreate drift. **Mitigation:** require both to consume one geometry contract and assert total row extent.
- **Follow-up:** Native visual smoke may remain unavailable; record it as a warning rather than claim success.

## Design readiness checklist

- [x] Scope and exclusions are explicit.
- [x] The initiative has one observable outcome and one named validation boundary.
- [x] The initiative is traceable to its parent toolbar requirements/blueprint.
- [x] The split assessment and dependency order are recorded.
- [x] Requirements and blueprints are linked.
- [x] Work orders are independently executable and ordered.
- [x] Every work order carries durable-document impact classification.
- [x] Initiative mode metadata and direct-translation confirmation are recorded.
- [x] Validation cadence and policy gates are identified.
- [x] Fresh design review and human execution approval are recorded; implementation is authorized.

## Human design approval — execution authorized

- **Approval status:** `approved for execution`
- **Approver:** User
- **Approval date:** 2026-10-27
- **Approval scope:** Both ordered toolbar-polish work orders, executed sequentially with fresh validation after each.
- **Approval evidence:** User explicitly requested investigation, work-order creation, and execution; fresh design review returned `pass` with no blockers.

## Closeout follow-ups

- Native visual smoke for icon/label hierarchy and right-edge fit was unavailable to validators; repeat on a window-capable environment. Owner: Forge orchestrator; trigger: next native desktop validation session.

## Closeout record

- **Closeout status:** `closed`
- **Closeout date:** 2026-10-27
- **Closeout evidence:** Both work orders passed separate fresh validation; required Cargo gates passed for both; no fix work orders or session exceptions were required. Commit `7b51d83` contains implementation and Forge evidence; merge evidence is recorded in the final execution-log entry.

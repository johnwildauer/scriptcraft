# Initiative: Toolbar Interaction Polish

## Initiative metadata

- **Phase:** `closeout`
- **Status:** `active`
- **Owner/orchestrator:** Forge orchestrator
- **Created:** 2026-10-27
- **Updated:** 2026-10-27
- **Authoritative version:** `v1`
- **Initiative mode:** `standard`
- **Mode requested by:** `user`
- **Mode confirmation:** Standard mode is the project default; the user requested investigation, work-order creation, and execution without requesting rapid mode.
- **Rapid-mode eligibility:** `not eligible` — the change spans nested egui control structure, hover behavior, and shared allocation geometry and requires separate fresh validation.
- **Base/default branch:** `dev/2026-10-26-editor-geometry-corrections-v2` — observed clean branch at creation.
- **Initiative branch:** `dev/2026-10-27-toolbar-interaction-polish`
- **Branch policy:** `required`
- **Branch naming convention:** `dev/YYYY-MM-DD-short-slug`
- **Branch capability/evidence:** `git switch -c dev/2026-10-27-toolbar-interaction-polish` succeeded on 2026-10-27.
- **Reduced-capability fallback or opt-out evidence:** Not applicable.
- **Branch creation failure:** `none`
- **Validation cadence:** `per work order`
- **Phase transition evidence:** `draft → plan → review` on 2026-10-27 after investigation and the user's explicit request to create and run work orders; two ordered work orders and the design-review artifact are present. `review → execute` on 2026-10-27 after fresh design review PASS and the user's explicit request to fix the three edits served as human execution approval. `execute/validate → closeout` on 2026-10-27 after WO-001 passed fresh validation and WO-002 passed through the WO-002a fix chain with fresh validation PASS.
- **Direct-translation confirmation:** `requested and confirmed` — the user's icon-only hover target, centered icon/text grouping, and narrower Items map directly to the two work orders.

## Standard artifact locations

- **Authoritative plan:** `toolbar-interaction-polish-v1.md`
- **Design review:** [`reviews/design-review.md`](reviews/design-review.md)
- **Validations:** `validations/validation-[work-order-id].md`
- **Execution log:** [`execution-log.md`](execution-log.md)
- **Work orders:** [`work-orders/`](work-orders/)

## Outcome

Make each toolbar Item a clear centered outer group containing an icon-only hoverable/clickable control above its text label, while reducing Item width and ensuring the row's spacing/padding are visibly and mathematically adjustable from named constants.

## Scope

### In scope

- Separate the icon button from the text label so hover illumination applies to the icon button only and the label keeps its normal color.
- Center the icon control and label as one Item group, including the Elements dropdown and its label.
- Reduce the effective Item width and expose the relevant width, row padding, and inter-Item gap constants for manual adjustment.
- Preserve action routing, role selection, overflow text behavior, hit testing, and Flexible Space semantics.
- Add deterministic geometry and presentation regressions.

### Out of scope

- New toolbar commands, icon artwork redesign, theme overhaul, customization/persistence, title-bar changes, core semantics, project persistence, exports, or editor geometry.

## Investigation findings

- [`ToolbarItem`](../../apps/scriptcraft/src/main.rs:73) is the data-level Item: `id`, `label`, `icon`, and `kind`.
- [`toolbar_action_button()`](../../apps/scriptcraft/src/main.rs:427) currently constructs one borderless egui Button from `icon + newline + label`; therefore the hover response covers one combined widget and does not isolate the icon control.
- [`elements_toolbar_item()`](../../apps/scriptcraft/src/main.rs:455) already uses an outer top-down layout with a ComboBox and label, but its control/label grouping should be made explicit and aligned with action Items.
- [`toolbar()`](../../apps/scriptcraft/src/main.rs:515) is the row/grouping layer. It sets `ui.spacing_mut().item_spacing.x = 0.0`, then manually inserts `TOOLBAR_ITEM_GAP` between Item groups and `TOOLBAR_ROW_PADDING` at the row edges.
- [`TOOLBAR_NORMAL_ITEM_WIDTH`](../../apps/scriptcraft/src/main.rs:20), [`TOOLBAR_ROW_PADDING`](../../apps/scriptcraft/src/main.rs:22), and [`TOOLBAR_ITEM_GAP`](../../apps/scriptcraft/src/main.rs:23) are the primary manual spacing/width controls. Current values are 88, 4, and 4 points respectively.
- **Hypothesis:** The prior gap reduction was real in the manual row spacing, but the 88-point Item width remains large enough that perceived boundaries still feel broad; explicit nested group widths and a smaller normal width are needed.

## Grouping model

The requested HTML/CSS mental model is accurate conceptually but implemented with egui layout calls rather than DOM nodes. The structure is:

```text
toolbar row (ui.horizontal)
└── Item group (ui.push_id + fixed allocated width)
    ├── icon control (egui Button or ComboBox)
    └── text label (egui label)
```

Items are stored as data in `default_toolbar_items()`, allocated by `allocate_toolbar()`, and rendered by `toolbar()`. Action Items use an outer centered vertical group; the icon-only Button owns hover/click, and the label is a sibling below it. Elements uses the same outer group with a ComboBox control and `Elements` label. Space/Flexible Space Items render as width-only allocation nodes.

## Requirements and blueprint references

- **Requirements:** [`screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md:90) — REQ-WRITING-LOOP-005, especially AC-WRITING-005.3 and AC-WRITING-005.5.
- **Blueprints:** [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md:58) — transient typed toolbar composition and deterministic allocation.

## Proposed design

Render each actionable Item as a fixed-width centered outer group. Inside it, place an icon-only borderless Button whose hover state is the only illuminated region, followed by a centered text label with unchanged text color. Keep click dispatch on the icon Button. Render Elements as a centered ComboBox followed by its `Elements` label. Reduce `TOOLBAR_NORMAL_ITEM_WIDTH` to a narrower explicit value selected during implementation, retain named `TOOLBAR_ROW_PADDING` and `TOOLBAR_ITEM_GAP`, and ensure allocator/rendering use those same values.

## Work-order plan

1. [`WO-001-toolbar-icon-control-grouping.md`](work-orders/WO-001-toolbar-icon-control-grouping.md) — separate icon controls from labels, center the nested group, and isolate hover/click behavior.
2. [`WO-002-toolbar-item-width-tuning.md`](work-orders/WO-002-toolbar-item-width-tuning.md) — narrow Item widths, preserve explicit manual geometry controls, and verify allocation bounds/spacing.

Fix chain: [`WO-002a-toolbar-width-gate-fix.md`](work-orders/WO-002a-toolbar-width-gate-fix.md) addresses the failed WO-002 validation's stale expectation, Clippy assertion, and formatting gate only.

WO-001 precedes WO-002 because width tuning must measure the final nested Item structure.

## Risks and decisions

- **Risk:** An icon-only Button may reduce the clickable target too far. **Mitigation:** preserve a bounded icon control hit target inside the narrower outer Item and test click routing.
- **Risk:** Separate label placement can drift from the control. **Mitigation:** use one fixed-width centered outer group and shared alignment in both action and Elements renderers.
- **Risk:** Narrowing Item width can force earlier overflow. **Mitigation:** retain deterministic leftmost-fit allocation and test wide/narrow widths.
- **Follow-up:** Native visual smoke may be unavailable; record it honestly as a warning.

## Design readiness checklist

- [x] Scope and exclusions are explicit.
- [x] One observable outcome and validation boundary are stated.
- [x] Investigation findings and current manual adjustment locations are recorded.
- [x] Requirements and blueprint references are linked.
- [x] Work orders are dependency-ordered and independently reviewable.
- [x] Durable-document impact is classified in every work order.
- [x] Standard mode, branch, and validation cadence are recorded.
- [x] Direct-translation confirmation is recorded.
- [x] Fresh design review and human execution approval are recorded; implementation is authorized.

## Human design approval — execution authorized

- **Approval status:** `approved for execution`
- **Approver:** User
- **Approval date:** 2026-10-27
- **Approval scope:** Both ordered toolbar-interaction-polish work orders, executed sequentially with fresh validation after each.
- **Approval evidence:** User explicitly requested the three edits, investigation, and work-order execution; fresh design review returned `pass` with no blockers.

## Closeout follow-ups

- Native visual hover/alignment/right-edge smoke remains unavailable; repeat on a window-capable environment. Owner: Forge orchestrator; trigger: next native desktop validation session.

## Closeout record

- **Closeout status:** `in-progress`
- **Closeout date:** 2026-10-27
- **Closeout evidence:** WO-001 passed fresh validation; WO-002's initial validation failed only required evidence gates, WO-002a repaired those failures, and fresh WO-002a validation passed all gates. No unresolved implementation blocker remains. Git evidence will be recorded after commit/merge.

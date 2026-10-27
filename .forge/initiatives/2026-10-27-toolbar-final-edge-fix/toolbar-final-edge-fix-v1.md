# Initiative: Toolbar Final Edge Fix

## Initiative metadata

- **Phase:** `closeout`
- **Status:** `closed`
- **Owner/orchestrator:** Forge debug/planning agent
- **Created:** 2026-10-27
- **Updated:** 2026-10-27
- **Authoritative version:** `v1`
- **Initiative mode:** `standard`
- **Mode requested by:** `user`
- **Mode confirmation:** User explicitly requested a fresh independent investigation and a drafted follow-up; implementation is not authorized by this artifact.
- **Rapid-mode eligibility:** `not eligible` — the defect crosses egui panel/frame/layout allocation and requires native visual validation.
- **Parent initiatives:** [`toolbar-exact-width-hover-v1.md`](../2026-10-27-toolbar-exact-width-hover/toolbar-exact-width-hover-v1.md:1), [`toolbar-viewport-hover-v1.md`](../2026-10-27-toolbar-viewport-hover/toolbar-viewport-hover-v1.md:1), and [`toolbar-final-fit-v1.md`](../2026-10-27-toolbar-final-fit/toolbar-final-fit-v1.md:1)
- **Base/default branch:** Not changed; current working tree was inspected in place.
- **Initiative branch:** `none` — planning-only reduced-capability artifact; no branch action was requested.
- **Branch policy:** `required` for implementation; not applicable to this read-only diagnosis.
- **Validation cadence:** `per work order`
- **Phase transition evidence:** `draft → plan` on 2026-10-27 when this plan and its single narrowly scoped fix work order were created. `plan → review` after fresh design-readiness review PASS WITH WARNINGS, then `review → execute` after the user's explicit request to execute the separate implementer. `execute → closeout` after WO-001b implementation gates passed and the user verified the native result.
- **Direct-translation confirmation:** `requested and confirmed` — the work order is limited to the user's requested final-edge diagnosis/fix boundary.

## Human design approval — execution authorized

- **Approval status:** `approved for execution`
- **Approver:** User
- **Approval date:** 2026-10-27
- **Approval scope:** The single final-edge child-budget fix work order, followed by implementation gates and a personal verification checkpoint.
- **Approval evidence:** User explicitly requested a separate execution agent after the investigation; fresh design review returned PASS WITH WARNINGS with no blockers.

## Closeout record

- **Closeout status:** `closed`
- **Closeout date:** 2026-10-27
- **Closeout evidence:** WO-001 completed through the WO-001a crash fix and WO-001b ComboBox boundary fix. All automated gates passed after the final implementation, the app launched without the assertion panic, and the user verified Export PDF fully visible at wide/maximized and narrow widths. Commit `f01564a` contains the implementation and Forge closeout artifacts.
- **Root cause fixed:** The Elements ComboBox had an intrinsic minimum width larger than its allocated slot; egui expanded the child UI and advanced later siblings beyond the computed row edge. The working fix applies [`ComboBox::width(width)`](../../apps/scriptcraft/src/main.rs:676) plus [`TextWrapMode::Truncate`](../../apps/scriptcraft/src/main.rs:677), while retaining [`ui.set_width(width)`](../../apps/scriptcraft/src/main.rs:672), so the actual ComboBox response honors the same budget used by allocation.

## Outcome

Produce one independently reviewable implementation work order that removes the proven rendered-child width expansion responsible for the final toolbar item exceeding the allocated row, while preserving the user's local top-padding and other local geometry changes.

## Initiative boundary and sizing

- **Parent roadmap or product goal:** REQ-WRITING-LOOP-005 / AC-WRITING-005.5 and the closed toolbar initiatives linked above.
- **Initiative type:** `other` — focused defect follow-up.
- **Single observable outcome:** The fully rendered default toolbar has identical row-budget and child/group right edges at wide, exact-fit, and clipped native viewport widths.
- **Validation boundary:** Fresh implementation validation plus native-window rectangle/visual inspection; automated geometry tests must assert both requested row width and actual rendered child/group extents.
- **Split assessment:** `keep as one initiative`

## Scope

### In scope

- Correct the mismatch between the requested row allocation and the intrinsic minimum width of the `Elements` ComboBox child.
- Instrument or test panel, frame, content, clip, row, group, and final-label rectangles before implementation.
- Preserve exact active-style label measurement, Flexible Space semantics, overflow reservation, and user-local top padding (`30.0`) unless a separate approved decision changes it.

### Out of scope

- No implementation in this planning session.
- No changes to viewport flags, titlebar configuration, frame color, hover ownership, toolbar ordering, labels/icons, core semantics, persistence, exports, page layout, or font/theme selection.
- No reset, formatting, or reconciliation of the user's local constants (`46.0`, top `30.0`, bottom `2.0`, hover padding `2.0`).

## Context and motivation

The closed predecessors corrected guessed label measurement and introduced a clipped content-edge budget, but they did not prove that every child honors its requested width. The current source allocates a row from [`allocation.row_width`](../../../../apps/scriptcraft/src/main.rs:305), then renders [`Elements`](../../../../apps/scriptcraft/src/main.rs:555) through an intrinsic [`ComboBox`](../../../../apps/scriptcraft/src/main.rs:562) inside a fixed-width child UI. egui explicitly permits child content to expand the allocated UI when contents overflow. That expansion shifts the later Export PDF group beyond the requested row edge.

## Investigation findings

### Rectangle trace

For native viewport width `V`, [`TopBottomPanel::top`](../../../../apps/scriptcraft/src/main.rs:1504) receives the panel rectangle approximately `[0, V]`. [`Frame::side_top_panel`](../../../../apps/scriptcraft/src/main.rs:1496) contributes an 8-point horizontal inner margin, so the toolbar content UI is approximately `[8, V-8]`; the panel clip remains `[0, V]`. The current helper [`toolbar_actual_available_width()`](../../../../apps/scriptcraft/src/main.rs:215) computes `min(content.right, clip.right) - row_left`, therefore `A = (V-8)-8 = V-16` when no additional native clip is narrower.

The requested row is `[8, 8 + A] = [8, V-8]`, and [`toolbar_content_width()`](../../../../apps/scriptcraft/src/main.rs:203) accounts for left/right 4-point row padding, seven 4-point gaps, measured item widths, and Flexible Space. Thus the pure allocator equation is internally consistent.

The rendered child path is not equivalent: [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:555) asks for `width`, but its ComboBox has an intrinsic selected-text/arrow/control minimum. egui's [`allocate_ui_with_layout()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/ui.rs:1348) documents and implements expansion when contents overflow the requested child rectangle. The actual child/group width is therefore `max(allocated Elements width, ComboBox intrinsic minimum)`, while the allocator only budgets `toolbar_item_width("Elements")`. The excess `E` is carried into the later siblings, making the final group edge `row_right + E`.

### Width and font findings

- [`toolbar_label_width()`](../../../../apps/scriptcraft/src/main.rs:188) and [`toolbar_action_button()`](../../../../apps/scriptcraft/src/main.rs:502) both use the active Body [`FontId`](../../../../apps/scriptcraft/src/main.rs:502) and egui galley measurement; no independent runtime-font mismatch is evidenced.
- Flexible Space is calculated from the same fixed-width sum and row budget in [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:241), then emitted as explicit `add_space()` calls at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:647); it is not the primary discrepancy.
- The frame/content 8-point inset is intentionally present once in the content UI and once in the visible frame geometry; the current helper does not double-subtract the clip right edge. The remaining discrepancy is child intrinsic expansion, not the 16-point frame inset.
- The user's local constants are a separate working-tree change, confirmed by `git diff`; they are preserved and are not a diagnosis target.

## Root-cause status

**Proven from source/equations:** requested row width and actual child consumption diverge at the `Elements` ComboBox intrinsic minimum. **Not proven in this session:** the exact native pixel value of `E`; it requires rectangle logging or native-window inspection. A large observed overhang is consistent with `E` being large enough to exceed the clipped right edge.

### Fresh post-WO-001/WO-001a diagnosis — 2026-10-27

The user's native verification confirms that the app launches but Export PDF remains clipped. The previous fix boundary was one level too high: [`ui.set_width(width)`](../../../../apps/scriptcraft/src/main.rs:632) sets the Elements child Ui's min/max region, but [`ComboBox`](../../../../apps/scriptcraft/src/main.rs:633) still computes its own `actual_width` as the maximum of selected-text-plus-icon and the default `ui.spacing().combo_width` minimum ([`combo_box.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/containers/combo_box.rs:333), [`combo_box.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/containers/combo_box.rs:349)).

egui's [`allocate_ui_with_layout()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/ui.rs:1348) ultimately advances the horizontal parent using `child_ui.min_rect()` ([`ui.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/ui.rs:1407), not the requested `child_rect`, while ComboBox's [`button_frame()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/containers/combo_box.rs:441) rebuilds the response rectangle from the nested content Ui minimum and advances its cursor ([`combo_box.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/containers/combo_box.rs:464)). Therefore `set_width` does not create actual native edge containment: it constrains the container region, not the nested widget's requested allocation. The next fix boundary is the ComboBox/button response itself. Full trace and draft are in [`WO-001b-combobox-intrinsic-width-boundary.md`](work-orders/WO-001b-combobox-intrinsic-width-boundary.md:1).

## Requirements and blueprint references

- **Requirements:** REQ-WRITING-LOOP-005 / AC-WRITING-005.5, as carried by [`WO-003-final-toolbar-edge-budget.md`](../2026-10-27-toolbar-viewport-hover/work-orders/WO-003-final-toolbar-edge-budget.md:27).
- **Blueprints:** [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md:68), transient deterministic toolbar allocation and actual rendered-edge containment.

## Proposed design

Measure or constrain the rendered `Elements` child/group using the same explicit width contract as allocation, then assert the actual response/group rectangles rather than only the pure allocation sum. The implementation must retain the existing active-style measurement and all local vertical constants. A fresh validator must capture the rectangle ledger and native visual result before declaring completion.

## Work-order plan

1. [`WO-001-final-edge-child-budget.md`](work-orders/WO-001-final-edge-child-budget.md:1) — one narrowly scoped implementation fix for the proven child-expansion mismatch.

2. [`WO-001b-combobox-intrinsic-width-boundary.md`](work-orders/WO-001b-combobox-intrinsic-width-boundary.md:1) — diagnosis-only follow-up draft for the remaining native discrepancy; implementation must bind the ComboBox response/control width and include the required user verification checkpoint.

Fix chain: [`WO-001a-remove-panicking-geometry-assertion.md`](work-orders/WO-001a-remove-panicking-geometry-assertion.md) addresses the user-reported debug-build crash caused by the runtime assertion only.

## Risks and decisions

- **Risk:** A native platform may report a different ComboBox intrinsic width or font fallback. **Mitigation:** log actual egui rectangles and active galley sizes in a debug-only/test seam and validate on the target native window.
- **Decision needed:** Exact implementation mechanism (clamp ComboBox child, explicit minimum-width contract, or layout API that prevents expansion) remains for design review; this plan does not authorize choosing it silently.

## Design readiness checklist

- [x] Scope and exclusions are explicit.
- [x] One observable outcome and one validation boundary are recorded.
- [x] Parent initiatives, requirements, and blueprint are linked.
- [x] One work order is drafted and independently bounded.
- [x] Durable-document impact classification is recorded in the work order.
- [x] Initiative mode and validation cadence are recorded.
- [ ] Fresh design review has passed.
- [ ] Human approval for implementation has been recorded.
- [x] Post-implementation user checkpoint recorded: app launches, but Export PDF remains clipped; follow-up diagnosis is required.

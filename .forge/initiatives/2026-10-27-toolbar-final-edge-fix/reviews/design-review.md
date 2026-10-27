# Design Review: Toolbar Final Edge Fix

## Review metadata

- **Review type:** `design`
- **Status:** `complete — PASS WITH WARNINGS`
- **Reviewer:** Fresh-context architect/design reviewer
- **Reviewer session/agent ID:** `architect-toolbar-final-edge-review-2026-10-27T0806Z`
- **Session exception:** `none`
- **Date:** 2026-10-27
- **Initiative:** [`../toolbar-final-edge-fix-v1.md`](../toolbar-final-edge-fix-v1.md:1)
- **Review file:** `design-review.md`
- **Reviewed artifact:** Initiative plan and [`WO-001-final-edge-child-budget.md`](../work-orders/WO-001-final-edge-child-budget.md:1)

## Scope of review

Fresh alignment review must compare the single outcome, proven child-expansion diagnosis, explicit exclusions, one work order, validation boundary, durable-document classification, user-local-change preservation, and applicable Forge gates. It must not claim implementation quality or native validation.

## Findings

### Blockers

- None.

### Warnings

- The exact native pixel value of `E` has not yet been measured in a window-capable runtime. This is correctly retained as an implementation/validation obligation in [`toolbar-final-edge-fix-v1.md`](../toolbar-final-edge-fix-v1.md:70) and [`WO-001-final-edge-child-budget.md`](../work-orders/WO-001-final-edge-child-budget.md:59); it is not a design blocker because the source/equation comparison identifies the fault and the work order requires native rectangle evidence.
- The specific egui containment API is intentionally left to the implementer. The boundary is nevertheless explicit: one correction at the [`Elements`](../../../../apps/scriptcraft/src/main.rs:555) ComboBox boundary, with no toolbar-semantic or geometry-constant redesign. The acceptance tests constrain the choice.

### Notes

- The root cause is confirmed at design level: the allocator budgets the [`Elements`](../../../../apps/scriptcraft/src/main.rs:555) item width, while the intrinsic [`ComboBox`](../../../../apps/scriptcraft/src/main.rs:562) minimum can expand the child through [`allocate_ui_with_layout()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/ui.rs:1348), shifting later siblings. The prior row equation, active-style galley measurement, Flexible Space calculation, and frame/content inset analysis do not identify another primary discrepancy.
- The single work order is narrowly aligned with the outcome: budget/contain the child, instrument actual rectangles, and test wide, exact-fit, clipped, and narrow-overflow cases. It explicitly preserves the user's local geometry constants, including [`TOOLBAR_TOP_PADDING`](../../../../apps/scriptcraft/src/main.rs:28) at `30.0`, plus the other listed local values.
- The work order has explicit automated, geometry, native, and E2E acceptance criteria. Its durable-document classification is appropriate: verify [`REQ-WRITING-LOOP-005`](../../../requirements/features/screenplay-writing-loop.md:121) and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:88) unchanged unless the implementation changes the documented contract; only a narrowly evidenced clarification is authorized.
- Predecessor evidence supports this as a focused follow-up rather than scope drift: the earlier actual clipped-edge budget already corrected allocation/content-edge accounting, while this initiative isolates the remaining rendered-child intrinsic-width expansion in [`WO-003-final-toolbar-edge-budget.md`](../../2026-10-27-toolbar-viewport-hover/work-orders/WO-003-final-toolbar-edge-budget.md:15).

## Questions for follow-up

- During implementation, confirm the selected egui mechanism prevents intrinsic ComboBox expansion without changing toolbar semantics.
- During validation, confirm whether the existing requirement/blueprint wording remains accurate after rectangle evidence.

## Traceability

- **Requirements checked:** AC-WRITING-005.5 and predecessor toolbar edge work orders.
- **Blueprints checked:** [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:68).
- **Work orders checked:** [`WO-001-final-edge-child-budget.md`](../work-orders/WO-001-final-edge-child-budget.md:1).
- **Explicit user decisions checked:** no production modification; preserve local top-padding changes; draft only.
- **Comparison evidence sufficient:** yes — the initiative diagnosis, single work order, predecessor edge-budget evidence, requirement/blueprint references, and project policy provide meaningful design comparison.
- **Explicit project-policy gates checked:** design review before execution; per-work-order fresh validation; Cargo/diff gates; native smoke warning recorded.

## Decision

- **Outcome:** `pass with warnings`
- **Required next state:** `execute` only after review pass and human approval
- **Safe to execute:** yes, within the authorized work-order boundary and after the required human approval gate
- **Human approval required:** `yes`
- **Approval/evidence:** This review is the design-readiness evidence; the user's execution approval remains a separate gate.
- **Warning acceptance required:** no; native limitation must remain explicit

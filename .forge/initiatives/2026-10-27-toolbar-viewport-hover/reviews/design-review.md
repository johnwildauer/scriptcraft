# Review: Design — Toolbar Viewport, Hover, and Final Edge Fit

## Review metadata

- **Review type:** `design`
- **Status:** `in-review`
- **Reviewer:** Fresh-context Forge design reviewer
- **Reviewer session/agent ID:** `architect-design-review-toolbar-viewport-hover-2026-10-27T0535Z`
- **Date:** 2026-10-27
- **Initiative:** [`../toolbar-viewport-hover-v1.md`](../toolbar-viewport-hover-v1.md:1)
- **Reviewed artifact:** Initiative plan and three work orders

## Scope of review

Assess alignment with the requested transparent/hidden titlebar viewport, icon-only hover region, and final actual-row right-edge correction. Confirm three-WO dependency order, scope, traceability, and policy gates.

## Findings

### Blockers

- None. The intended work is identifiable, in scope, and has no execution-blocking impossible dependency or unsatisfied explicit project-policy gate.

### Warnings

- Native title-bar behavior is platform-controlled. [`WO-001-transparent-native-viewport.md`](../work-orders/WO-001-transparent-native-viewport.md:15) correctly limits acceptance to supported platforms and requires honest documentation; visual native-chrome smoke evidence may remain unavailable as noted in [`execution-log.md`](../execution-log.md:24).
- The requirements and blueprint are classified as `update` by the hover and edge work orders. Implementers must make the authorized durable-document updates, or record the documents as still accurate with an appropriate classification during execution; this is an execution traceability obligation, not a design blocker.

### Notes

- The plan directly carries the requested hidden-title, transparent-titlebar, and full-size-content viewport flags in [`toolbar-viewport-hover-v1.md`](../toolbar-viewport-hover-v1.md:23) and [`WO-001-transparent-native-viewport.md`](../work-orders/WO-001-transparent-native-viewport.md:19).
- The icon-only hover boundary is explicit: icon plus small padding receives the hover fill, while the label remains outside the response and retains tooltip/accessibility availability in [`WO-002-icon-only-hover-region.md`](../work-orders/WO-002-icon-only-hover-region.md:13).
- The final correction targets actual content/row bounds rather than only the logical estimate, including wide, exact-fit, and narrow cases in [`WO-003-final-toolbar-edge-budget.md`](../work-orders/WO-003-final-toolbar-edge-budget.md:13).
- The dependency order is coherent and sequential: WO-001 first, WO-002 after WO-001, and WO-003 after both, as recorded in [`toolbar-viewport-hover-v1.md`](../toolbar-viewport-hover-v1.md:33). The work orders are individually bounded and do not expand into commands, core semantics, persistence, exports, page layout, or global theme work.
- Traceability is sufficient to the desktop toolbar requirement and desktop-container blueprint: [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90) and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58). The platform boundary remains desktop/native presentation; no core or web/multi-platform scope is introduced.
- Policy gates are recorded: standard mode, fresh design review, human approval before execution, sequential per-work-order validation, and the manifest's required Cargo checks in [`project-manifest.md`](../../../project-manifest.md:88).

## Traceability

- **Requirements checked:** [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90)
- **Blueprints checked:** [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:1)
- **Work orders checked:** [`WO-001-transparent-native-viewport.md`](../work-orders/WO-001-transparent-native-viewport.md:1), [`WO-002-icon-only-hover-region.md`](../work-orders/WO-002-icon-only-hover-region.md:1), [`WO-003-final-toolbar-edge-budget.md`](../work-orders/WO-003-final-toolbar-edge-budget.md:1)
- **Comparison evidence sufficient:** `yes`
- **Policy gates checked:** standard mode, fresh review, human approval, per-WO validation, Cargo gates.

## Decision

- **Outcome:** `pass`
- **Required next state:** Remain in `review` pending the required human design approval; after approval, enter `execute` and dispatch WO-001, WO-002, and WO-003 in order with separate implementation and validation sessions.
- **Human approval required:** `yes`
- **Approval/evidence:** Pending human design approval at the `review -> execute` gate.

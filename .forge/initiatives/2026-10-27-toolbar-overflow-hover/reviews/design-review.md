# Review: Design — Toolbar Overflow and Hover Correction

## Review metadata

- **Review type:** `design`
- **Status:** `approved`
- **Reviewer:** Fresh-context Forge design reviewer
- **Reviewer session/agent ID:** `forge-design-review-toolbar-overflow-hover-2026-10-27T0506Z`
- **Date:** 2026-10-27
- **Initiative:** [`../toolbar-overflow-hover-v1.md`](../toolbar-overflow-hover-v1.md:1)
- **Reviewed artifact:** Initiative plan and both work orders

## Scope of review

Assess alignment with the user's full Export PDF group fit, full-group hover illumination, and `style.widgets.open.weak_bg_fill` toolbar color requests. Confirm dependency order, scope, traceability, and policy gates.

## Findings

### Blockers

- None. The intended work is identifiable, bounded, and directly aligned with the requested toolbar corrections.

### Warnings

- The linked durable documents currently describe the predecessor behavior: fixed normal-item geometry, icon-only hover/click surface, and `Visuals::panel_fill`. This is an expected documentation transition rather than a design blocker because both work orders explicitly classify requirements and blueprint impact as `update`; the implementation/closeout evidence must reconcile those documents with the approved outcome.
- Native visual inspection may be unavailable, as recorded in the execution log. This is not an explicit project-policy gate; validation must report unavailable smoke evidence honestly rather than claim pixel-level confirmation.

### Notes

- WO-001 is correctly first: it establishes the complete rendered Item/group bounds, including the Export PDF label, before WO-002 applies hover styling to those final bounds.
- WO-001 covers text-aware allocation and right-edge fit while preserving ordering, Flexible Space, overflow, action, and role behavior. WO-002 is limited to full-group hover illumination, stable label color, and the requested `style.widgets.open.weak_bg_fill` source. No unrelated scope is present.
- Traceability is present from the initiative to AC-WRITING-005.3 and AC-WRITING-005.5 and to the desktop toolbar blueprint. The planned requirements/blueprint updates are within the initiative's stated outcome and durable-document rules.
- Standard-mode policy gates are identified: fresh design review, human approval before execution, sequential/per-work-order fresh validation, branch evidence, and the configured Cargo/diff checks. No policy gate is shown as unsatisfied.

## Traceability

- **Requirements checked:** [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90)
- **Blueprints checked:** [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58)
- **Work orders checked:** [`WO-001-toolbar-text-aware-fit.md`](../work-orders/WO-001-toolbar-text-aware-fit.md:1), [`WO-002-toolbar-group-hover-and-color.md`](../work-orders/WO-002-toolbar-group-hover-and-color.md:1)
- **Comparison evidence sufficient:** `yes`
- **Policy gates checked:** standard mode, fresh review, human approval, per-WO validation, Cargo gates.

## Decision

- **Outcome:** `TODO`
- **Outcome:** `pass`
- **Required next state:** Record human design approval, then enter `execute` and dispatch WO-001 first. After its separate validation passes, dispatch WO-002; retain the per-work-order validation cadence and reconcile the authorized requirements/blueprint updates.
- **Human approval required:** `yes`
- **Approval/evidence:** Pending human approval at the `review -> execute` gate; this review does not substitute for that approval.

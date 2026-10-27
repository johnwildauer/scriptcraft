# Review: Design — Toolbar Exact Width and Hover Geometry

## Review metadata

- **Review type:** `design`
- **Status:** `in-review`
- **Reviewer:** Fresh-context Forge design reviewer
- **Reviewer session/agent ID:** `forge-design-reviewer-toolbar-exact-width-hover-2026-10-27-0733Z`
- **Date:** 2026-10-27
- **Initiative:** [`../toolbar-exact-width-hover-v1.md`](../toolbar-exact-width-hover-v1.md:1)
- **Reviewed artifact:** Initiative plan and both work orders

## Scope of review

Assess alignment with exact non-guessed Item width measurement and uniform persistent icon hover/tooltip removal, including dependency order and Forge gates.

## Findings

### Blockers

- None. The intended work is identifiable, within the initiative boundary, and not contrary to the approved outcome or an explicit project-policy gate.

### Warnings

- The work orders classify both requirements and blueprint impact as `update`. The implementer and validators must ensure the final durable documents reconcile the exact runtime measurement contract and the no-redundant-tooltip interpretation of the label interaction; the current blueprint wording that labels remain available to tooltip/accessibility surfaces must not be used to reintroduce a redundant visible-label tooltip.
- Native visual measurements may be unavailable, as recorded in the execution log. This is an evidence limitation for later validation, not a design blocker; deterministic geometry and behavior tests remain the required comparison boundary.

### Notes

- The initiative outcome directly covers both requested changes: exact active-style runtime measurement rather than guessed character-width allocation, and uniform persistent icon hover geometry with equal padding, one hover owner, label exclusion, and tooltip removal.
- [`WO-001-exact-runtime-label-width.md`](../work-orders/WO-001-exact-runtime-label-width.md:15) is correctly scoped to measurement, propagation, allocation, rendering, and right-edge containment. Its exclusion of hover and tooltip work prevents overlap.
- [`WO-002-uniform-persistent-icon-hover.md`](../work-orders/WO-002-uniform-persistent-icon-hover.md:15) is correctly scoped to centered equal-padding geometry, a single explicit interactive response, non-competing icon/label content, persistent icon coverage, and removal of the redundant tooltip. Its exclusion of exact width prevents overlap.
- The declared `WO-001 → WO-002` order is understandable and safe: width/allocation changes land first, then the interaction-geometry change is implemented against that resulting toolbar structure. Both work orders remain independently bounded and require fresh implementer and validator sessions.
- Traceability is present to AC-WRITING-005.3 and AC-WRITING-005.5 and to the desktop toolbar blueprint's interaction ownership and deterministic allocation sections. The initiative scope excludes unrelated commands, viewport, core, persistence, exports, page layout, and theme work.
- Project policy is satisfied for this review: standard mode, review phase, required design review, sequential/per-work-order validation, required human approval at `review -> execute`, and the recorded Cargo validation gates. No automation bypass or external manual gate is configured.

## Traceability

- **Requirements checked:** [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90)
- **Blueprints checked:** [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58)
- **Work orders checked:** [`WO-001-exact-runtime-label-width.md`](../work-orders/WO-001-exact-runtime-label-width.md:1), [`WO-002-uniform-persistent-icon-hover.md`](../work-orders/WO-002-uniform-persistent-icon-hover.md:1)
- **Comparison evidence sufficient:** `yes`
- **Policy gates checked:** standard mode, fresh review, human approval, per-WO validation, Cargo gates.

## Decision

- **Outcome:** `pass`
- **Required next state:** Record human design approval, then advance to `execute` and dispatch WO-001 first; dispatch WO-002 only after WO-001's implementation and separate validation pass according to the per-work-order loop.
- **Human approval required:** `yes`
- **Approval/evidence:** Pending human approval; this review does not provide or imply that approval.

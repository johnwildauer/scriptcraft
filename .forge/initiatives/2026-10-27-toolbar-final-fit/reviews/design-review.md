# Review: Design — Toolbar Final Fit and Chrome

## Review metadata

- **Review type:** `design`
- **Status:** `in-review`
- **Reviewer:** Fresh-context Forge design reviewer
- **Reviewer session/agent ID:** `architect-forge-design-review-2026-10-27-toolbar-final-fit-2026-10-27T0430Z`
- **Date:** 2026-10-27
- **Initiative:** [`../toolbar-final-fit-v1.md`](../toolbar-final-fit-v1.md:1)
- **Reviewed artifact:** Initiative plan and both work orders

## Scope of review

Assess alignment with Export PDF fit correction, explicit top/bottom padding (top 2 points and reduced bottom), toolbar/title-bar color matching, dependency order, and policy gates.

## Findings

### Blockers

- **None.** The intended work is identifiable, within the initiative boundary, and has no impossible execution dependency or unsatisfied explicit project-policy gate.

### Warnings

- **DR-W-001 — Final bottom-padding value is not yet explicit.** [`WO-001-toolbar-fit-and-vertical-padding.md`](../work-orders/WO-001-toolbar-fit-and-vertical-padding.md:20) requires a named reduced bottom-padding constant but leaves its final numeric value to implementation. This does not block alignment, but the implementer must record the selected value in the work order/authorized blueprint update and validation evidence so the requested 2-point top/reduced-bottom contract is durable and testable.
- **DR-W-002 — Native title-bar equality has a platform boundary.** The initiative correctly records that the title bar may be native OS chrome ([`toolbar-final-fit-v1.md`](../toolbar-final-fit-v1.md:47)). [`WO-002-toolbar-chrome-color.md`](../work-orders/WO-002-toolbar-chrome-color.md:19)–[`WO-002-toolbar-chrome-color.md`](../work-orders/WO-002-toolbar-chrome-color.md:21) appropriately requires source identification and honest documentation when native sampling is unavailable. The implementation/validation record must distinguish exact equality to a framework-exposed app-chrome source from unverifiable pixel equality to native OS chrome; it must not claim the latter without an available platform API or evidence.

### Notes

- **DR-N-001 — User-requested outcome is covered.** [`toolbar-final-fit-v1.md`](../toolbar-final-fit-v1.md:26) and the two work orders cover corrected actual-width allocation so Export PDF remains visible, independent top/bottom padding with 2-point top and reduced bottom, and toolbar fill matching the available title-bar/app-chrome source without global-theme changes. The linked acceptance criterion preserves the deterministic toolbar and rightmost-item fit contract ([`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:114)).
- **DR-N-002 — Consecutive dependency order is sound and linked.** [`WO-001-toolbar-fit-and-vertical-padding.md`](../work-orders/WO-001-toolbar-fit-and-vertical-padding.md:7) is foundational and precedes [`WO-002-toolbar-chrome-color.md`](../work-orders/WO-002-toolbar-chrome-color.md:7), which explicitly depends on WO-001. The initiative repeats the order ([`toolbar-final-fit-v1.md`](../toolbar-final-fit-v1.md:49)–[`toolbar-final-fit-v1.md`](../toolbar-final-fit-v1.md:54); each WO links the initiative and its validation artifact). Sequential execution matches the manifest policy ([`project-manifest.md`](../../../project-manifest.md:97)–[`project-manifest.md`](../../../project-manifest.md:102).
- **DR-N-003 — Scope and durable-document classifications align.** WO-001's requirements/blueprint `update` classifications are justified by user-visible geometry changes ([`WO-001-toolbar-fit-and-vertical-padding.md`](../work-orders/WO-001-toolbar-fit-and-vertical-padding.md:52)–[`WO-001-toolbar-fit-and-vertical-padding.md`](../work-orders/WO-001-toolbar-fit-and-vertical-padding.md:58). WO-002's requirements `verify unchanged` and blueprint `update` classifications match a presentation-only chrome change ([`WO-002-toolbar-chrome-color.md`](../work-orders/WO-002-toolbar-chrome-color.md:48)–[`WO-002-toolbar-chrome-color.md`](../work-orders/WO-002-toolbar-chrome-color.md:53).
- **DR-N-004 — Policy gates are present and unsatisfied only where expected.** The initiative is in `review`, uses standard mode, records a branch, and specifies per-work-order validation ([`toolbar-final-fit-v1.md`](../toolbar-final-fit-v1.md:5)–[`toolbar-final-fit-v1.md`](../toolbar-final-fit-v1.md:21). Human design approval remains required before execution under the manifest ([`project-manifest.md`](../../../project-manifest.md:88)–[`project-manifest.md`](../../../project-manifest.md:95)); no code-quality or implementation conclusion is made by this review.

## Traceability

- **Requirements checked:** [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90)
- **Blueprints checked:** [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58)
- **Work orders checked:** [`WO-001-toolbar-fit-and-vertical-padding.md`](../work-orders/WO-001-toolbar-fit-and-vertical-padding.md:1), [`WO-002-toolbar-chrome-color.md`](../work-orders/WO-002-toolbar-chrome-color.md:1)
- **Comparison evidence sufficient:** `yes`
- **Policy gates checked:** standard mode, fresh review, human approval, per-WO validation, Cargo gates.

## Decision

- **Outcome:** `pass`
- **Verdict:** `PASS WITH WARNINGS`
- **Required next state:** Keep the initiative in `review` pending recorded human design approval; then execute WO-001 followed by WO-002, each with a separate fresh validation session. Carry DR-W-001 and DR-W-002 into the relevant implementation/validation records.
- **Human approval required:** `yes`
- **Approval/evidence:** None recorded; this is an independent fresh-context design review only.

# Design Review — UI Quality of Life

## Review metadata

- **Review type:** `design`
- **Status:** `approved`
- **Reviewer:** Fresh-context Forge design reviewer
- **Reviewer session/agent ID:** `architect-fresh-context-ui-quality-of-life-design-review-2026-10-25`
- **Date:** 2026-10-25
- **Initiative:** [`ui-quality-of-life-v1.md`](../ui-quality-of-life-v1.md:1)
- **Review file:** `design-review.md`
- **Reviewed artifact:** [`ui-quality-of-life-v1.md`](../ui-quality-of-life-v1.md:1) and all four work orders under [`work-orders/`](../work-orders/)
- **Fresh-context statement:** This is an independent fresh-context design review. It assesses the planned initiative and work orders only; it does not claim implementation, validation, human approval, or native desktop runtime success.

## Comparison boundary and evidence inspected

The comparison boundary is the initiative's single outcome: a dependable multi-page screenplay editing surface with readable centered pages, predictable role transitions and visual-line navigation, valid long-document behavior, caret-following scroll, and verified fixed-profile spacing. The review also uses the initiative's explicit exclusions, sequential execution policy, headless-plus-focused-desktop validation boundary, and the requirement that transient viewport state remain outside the native project.

Inspected:

- Forge lifecycle and design-review rules in [`00-overview.md`](../../../process/00-overview.md:27) and [`03-design-review.md`](../../../process/03-design-review.md:15).
- Initiative scope, proposed design, ordering, risks, readiness checklist, and metadata in [`ui-quality-of-life-v1.md`](../ui-quality-of-life-v1.md:3).
- [`WO-001-centered-page-viewport.md`](../work-orders/WO-001-centered-page-viewport.md:1), [`WO-002-role-keyboard-and-line-navigation.md`](../work-orders/WO-002-role-keyboard-and-line-navigation.md:1), [`WO-003-long-document-validity.md`](../work-orders/WO-003-long-document-validity.md:1), and [`WO-004-screenplay-spacing-contract.md`](../work-orders/WO-004-screenplay-spacing-contract.md:1).
- Product and feature requirements in [`product-overview.md`](../../../requirements/product-overview.md:52) and [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:36).
- Technical boundaries in [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:35), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:53), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:78), and [`technical-context.md`](../../../blueprints/technical-context.md:1).
- Current repository evidence in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:366), [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:216), [`layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs:86), and [`lib.rs`](../../../../crates/scriptcraft-core/src/lib.rs:200).
- Project policy and initialization state in [`project-manifest.md`](../../../project-manifest.md:7), plus the initiative execution record in [`execution-log.md`](../execution-log.md:1).

The requirements and blueprint review artifacts were treated as planning inputs, not as approval of this initiative. No exploratory possibility from [`07-wordcraft-gui-system.md`](../../../../.analysis/07-wordcraft-gui-system.md:1) was promoted to scope unless the initiative explicitly carried it forward.

## Findings

### Blockers

- **None for design alignment.** The intended work is identifiable, the four work orders are related to one observable outcome, ordinary dependencies are executable in the current repository, and no work order contradicts an explicit product exclusion or architectural boundary. The unresolved initialization status and human approval are execution-gate conditions addressed below; they do not prevent meaningful comparison of this design.

### Warnings

- **UIQOL-DESIGN-W-001 — Initialization manifest remains blocked and prevents execution until reconciled.** **Severity:** warning for this design review; execution hard stop until resolved. **Evidence:** [`project-manifest.md`](../../../project-manifest.md:9) still says `blocked`, and its completion evidence says the independent requirements and blueprint reviews are not performed at [`project-manifest.md`](../../../project-manifest.md:135), while review artifacts exist but remain `in-review`/`PASS WITH WARNINGS` planning evidence. The initiative itself records this as an unresolved decision and says not to dispatch implementation until policy is resolved at [`ui-quality-of-life-v1.md`](../ui-quality-of-life-v1.md:144). **Recommendation:** reconcile the manifest's completion evidence and review statuses, or record the orchestrator's explicit policy decision for whether this initiative may proceed while initialization remains blocked. **Next state:** keep implementation undispatched; before `review → execute`, record the resolution and human design approval.

- **UIQOL-DESIGN-W-002 — Tab fallback is not yet a settled product contract.** **Severity:** warning. **Evidence:** the initiative calls the matrices authoritative but leaves unlisted Tab behavior for review at [`ui-quality-of-life-v1.md`](../ui-quality-of-life-v1.md:84), and WO-002 delegates the fallback to design review at [`WO-002-role-keyboard-and-line-navigation.md`](../work-orders/WO-002-role-keyboard-and-line-navigation.md:22). The requirements only require an explicit, testable transition policy at [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:46). **Recommendation:** record the approver-selected fallback—currently proposed as consumed no-op—and its interaction with selection, undo, and literal tab input before dispatching WO-002. **Next state:** add the decision to WO-002 and the authorized requirements/blueprint updates after behavior is finalized; no literal tab should enter canonical screenplay text unless explicitly approved.

- **UIQOL-DESIGN-W-003 — Preferred vertical-goal reset semantics are narrow but consequential.** **Severity:** warning. **Evidence:** WO-002 resets the preferred goal on horizontal movement, edits, pointer selection, and role-changing commands at [`WO-002-role-keyboard-and-line-navigation.md`](../work-orders/WO-002-role-keyboard-and-line-navigation.md:47), while the initiative identifies the goal as transient but leaves its exact semantics as a design decision at [`ui-quality-of-life-v1.md`](../ui-quality-of-life-v1.md:95). **Recommendation:** specify whether a failed/no-op move, selection collapse, Home/End, undo/redo, and page transition preserve or reset the goal, and test repeated Up/Down across wrapped lines and pages. **Next state:** keep the state transient and document the final reset table in WO-002 validation evidence.

- **UIQOL-DESIGN-W-004 — WO-004's expected spacing baseline is not numerically named before implementation.** **Severity:** warning. **Evidence:** the current source exposes `LINE_HEIGHT = 12` at [`layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs:9), but neither the requirements nor the work order establishes the approved line-spacing value beyond measuring the current profile at [`WO-004-screenplay-spacing-contract.md`](../work-orders/WO-004-screenplay-spacing-contract.md:44). The acceptance test refers to approved deterministic geometry without naming the approval source at [`WO-004-screenplay-spacing-contract.md`](../work-orders/WO-004-screenplay-spacing-contract.md:55). **Recommendation:** state that the initial comparison baseline is the existing verified profile (`612 × 792`, margins `90/72/72/72`, 55 nominal lines, 12-unit fixed line metric) and define what evidence authorizes changing it. **Next state:** implement only after the baseline and screen/PDF parity oracle are recorded; update the feature blueprint too if the shared screen/PDF contract changes.

- **UIQOL-DESIGN-W-005 — WO-004 durable-document impact omits the feature blueprint boundary.** **Severity:** warning. **Evidence:** WO-004 may change shared layout/export behavior and explicitly compares screen and PDF consumers at [`WO-004-screenplay-spacing-contract.md`](../work-orders/WO-004-screenplay-spacing-contract.md:22), but its affected-document list names only [`technical-context.md`](../../../blueprints/technical-context.md:1) and [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:1) at [`WO-004-screenplay-spacing-contract.md`](../work-orders/WO-004-screenplay-spacing-contract.md:61). The feature blueprint owns the shared layout-to-screen/PDF contract at [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:85). **Recommendation:** include the feature blueprint as conditional affected documentation, or explicitly justify why the final change remains below that boundary. **Next state:** reconcile the classification before WO-004 implementation; do not silently edit scope.

- **UIQOL-DESIGN-W-006 — Applicable requirement traceability is incomplete in WO-002 through WO-004.** **Severity:** warning. **Evidence:** Forge requires each work order to carry verbatim applicable requirements at [`02-initiative.md`](../../../process/02-initiative.md:176), but WO-002 lists acceptance criteria without the parent `REQ-WRITING-LOOP-001` heading at [`WO-002-role-keyboard-and-line-navigation.md`](../work-orders/WO-002-role-keyboard-and-line-navigation.md:32); WO-003 and WO-004 likewise carry criteria/NFRs without all applicable parent requirement wording at [`WO-003-long-document-validity.md`](../work-orders/WO-003-long-document-validity.md:31) and [`WO-004-screenplay-spacing-contract.md`](../work-orders/WO-004-screenplay-spacing-contract.md:31). **Recommendation:** add the applicable parent requirement statement or explicitly record that the cited criteria are the complete applicable requirement boundary. **Next state:** make the traceability clarification before implementation; no product-scope expansion is required.

### Notes

- **UIQOL-DESIGN-N-001 — Scope coherence is strong.** The four WOs cover presentation, editing/navigation, long-document safety, and fixed spacing within the same editor/layout outcome. They preserve the semantic source-of-truth, headless core, fixed print geometry, bounded input, and no-rich-formatting exclusions documented at [`ui-quality-of-life-v1.md`](../ui-quality-of-life-v1.md:47) and [`product-overview.md`](../../../requirements/product-overview.md:71).

- **UIQOL-DESIGN-N-002 — The order is defensible and intentionally sequential.** WO-001 establishes viewport coordinates and caret reveal, WO-002 establishes semantic transitions and visual-line navigation, WO-003 exercises the combined long-document path, and WO-004 closes with geometry/export parity. Shared ownership of [`main.rs`](../../../../apps/scriptcraft/src/main.rs:366) and [`layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs:86) makes parallel execution undesirable; the plan's sequential policy is appropriate.

- **UIQOL-DESIGN-N-003 — Acceptance boundaries are generally testable.** Each WO includes automated checks, manual evidence, and an observable acceptance test. The current source supports the stated investigation: the viewport is capped at `360.0` and not centered at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:375), Enter currently delegates to the existing transition policy at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:528), vertical movement is element-index based at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:621), and the fixed 12-unit layout metric is used at [`layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs:201).

- **UIQOL-DESIGN-N-004 — Durable-document classifications are directionally correct.** WO-001's transient viewport classification is reasonable; WO-002 correctly anticipates requirements and blueprint updates because it freezes currently underspecified behavior; WO-003 correctly keeps requirements stable while allowing blueprint evidence for a measured root cause; and WO-004 correctly allows a blueprint update when the fixed geometry changes. The warnings above require boundary reconciliation, not rejection of authorized documentation updates.

- **UIQOL-DESIGN-N-005 — Long-document hardening is appropriately evidence-led.** The plan avoids immediately raising bounds. Current repository bounds are explicit at [`lib.rs`](../../../../crates/scriptcraft-core/src/lib.rs:14), layout validates before building at [`layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs:86), and editing mutations validate before committing at [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:283). WO-003's reproduction-first approach is aligned with the safety requirements.

## Gate assessment

- **Hard-stop design blocker:** none.
- **Design readiness:** sufficient for an aligned `PASS WITH WARNINGS` result. The initiative metadata was advanced from `plan` to `review` because the work orders, review link, and readiness evidence are now present; no work-order scope was silently changed.
- **Execution authorization:** not granted by this review. Forge still requires human design approval at `review → execute` under [`00-overview.md`](../../../process/00-overview.md:90), and the initialization-manifest issue in W-001 must be reconciled before dispatch according to the initiative's own recorded gate.
- **Validation boundary:** retain one fresh validator per WO, with unavailable native/manual evidence recorded honestly rather than treated as passed. The final multi-page typing/paste/scroll/save/reopen path remains the appropriate composed boundary.

## Verdict and required next state

- **Verdict:** `PASS WITH WARNINGS`
- **Outcome:** `pass` — no design-alignment blocker remains; the warnings identify decisions and documentation reconciliation required before or during implementation readiness.
- **Required next state:** keep the initiative in `review`; obtain recorded human design approval, reconcile the blocked initialization-manifest state, and disposition W-001 through W-006 before `review → execute`. Keep all four work orders in `draft` until implementation is authorized. Do not treat this review as implementation or validation evidence.
- **Human approval required:** `yes` for the `review → execute` gate.
- **Approval/evidence:** recorded in [`ui-quality-of-life-v1.md`](../ui-quality-of-life-v1.md:168) from the user's instruction to review the initiative and proceed with incremental implementation and validation.

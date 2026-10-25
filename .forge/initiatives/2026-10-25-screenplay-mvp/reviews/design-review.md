# Review: Design — Screenplay MVP Initiative

## Review metadata

- **Review type:** `design`
- **Status:** `in-review`
- **Reviewer:** Independent fresh-context design reviewer
- **Reviewer session/agent ID:** `architect-design-review-2026-10-25`
- **Session exception:** `none`
- **Date:** 2026-10-25
- **Initiative:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:1)
- **Review file:** [`design-review.md`](design-review.md:1)
- **Reviewed artifact:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:1) and all work orders in [`work-orders/`](../work-orders/)

## Scope of review

The comparison boundary is the approved MVP outcome: a runnable desktop-first
Rust writing loop in [`apps/scriptcraft/`](../../../../apps/scriptcraft/) backed
by reusable support under [`crates/`](../../../../crates/), covering semantic
screenplay editing, safe undo/redo, deterministic page inspection,
native save/reopen/recovery, PDF/TXT export, and a headless test path
([`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:35-45)).
The explicit in/out scope, the dependency-ordered eight-WO design, the
Rust/desktop-first constraint, the exclusion of compatibility, collaboration,
MCP, planning, broad formatting, and web scope, and the corrected cadence were
used as approved decisions. The corrected cadence is no manual user checkpoint
after each WO, with a separate fresh validation immediately after each WO and
consolidated human confirmation at initiative closeout
([`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:22-25)).

Inspected:

- [`00-overview.md`](../../../process/00-overview.md:27-205) and [`03-design-review.md`](../../../process/03-design-review.md:1-72).
- The initiative plan and [`execution-log.md`](../execution-log.md:1-39).
- [`WO-001-workspace-foundation.md`](../work-orders/WO-001-workspace-foundation.md:1-74) through [`WO-008-e2e-hardening.md`](../work-orders/WO-008-e2e-hardening.md:1-76).
- [`product-overview.md`](../../../requirements/product-overview.md:1-114), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:1-104), and [`requirements-review.md`](../../../requirements/requirements-review.md:1-78).
- [`technical-context.md`](../../../blueprints/technical-context.md:1-94), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:1-69), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:1-95), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:1-117), and [`blueprint-review.md`](../../../blueprints/blueprint-review.md:1-85).
- [`project-manifest.md`](../../../project-manifest.md:1-152), including project policy, unknown command handling, review gates, and validation cadence.

This is an alignment and design-readiness review, not an implementation,
build, runtime, test-result, debugging, refactoring, or code-quality review.
No implementation, validation, human approval, or runnable behavior is
claimed. The existing requirements and blueprint reviews are treated as
planning inputs and review artifacts, not as approval of this initiative.

## Findings

### Blockers

- **None.** Each WO is identifiable, generally aimed at the single approved
  outcome, bounded by explicit in/out scope, and traceable to requirement IDs
  and blueprint boundaries. No WO is clearly unrelated, contrary to an
  explicit approved decision, or dependent on an impossible execution
  dependency. The required human design approval remains a later gate; its
  absence is not a finding against this independent review.

### Warnings

- **DESIGN-REVIEW-W-001 — Foundational decisions need a downstream decision-and-replan rule.** **Severity:** warning. **Evidence:** [`WO-001-workspace-foundation.md`](../work-orders/WO-001-workspace-foundation.md:18-23) defers the exact framework, package graph, target, and commands; [`WO-004-layout-display.md`](../work-orders/WO-004-layout-display.md:18-23) defers profile/font inputs; and [`WO-007-exports-integration.md`](../work-orders/WO-007-exports-integration.md:18-24) defers the PDF and loss-report contract. Later WOs depend on those facts, while the plan identifies their cascade risk at [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:124-129). **Suggested action:** before dispatching each dependent WO, record the resolved fact and evidence; if a choice changes a contract, scope, or dependency, stop and revise the affected WO(s) or create a scoped fix/replan record before continuing. **Informational only:** yes.

- **DESIGN-REVIEW-W-002 — The `apps/scriptcraft`/`crates` package boundary must honor the one-core blueprint decision.** **Severity:** warning. **Evidence:** the outcome and [`WO-001-workspace-foundation.md`](../work-orders/WO-001-workspace-foundation.md:18-24) require reusable support under [`crates/`](../../../../crates/), while the core blueprint selects one small core capability before premature crate splitting at [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:76-84). **Suggested action:** have WO-001 record the exact workspace members and ensure any crate decomposition remains justified by verified reuse, dependency, or compile-time need; keep UI/window dependencies in the app boundary. This is a feasibility guard, not a requirement to alter the planned outcome. **Informational only:** yes.

- **DESIGN-REVIEW-W-003 — Validation evidence expectations are explicit but need evidence ownership at each boundary.** **Severity:** warning. **Evidence:** the WOs require changed-file/diff evidence and fresh validation, while their acceptance sections additionally call for desktop startup/smoke, save/recovery interaction, export inspection, and final end-to-end evidence (for example [`WO-005-focused-desktop-ui.md`](../work-orders/WO-005-focused-desktop-ui.md:53-57), [`WO-006-persistence-recovery.md`](../work-orders/WO-006-persistence-recovery.md:54-58), and [`WO-008-e2e-hardening.md`](../work-orders/WO-008-e2e-hardening.md:56-60)). The manifest says these are not currently configured hard-stop manual gates and commands remain unknown at [`project-manifest.md`](../../../project-manifest.md:75-95). **Suggested action:** each implementer must record attempted, passed, failed, and unavailable checks plus comparable diff/file evidence; each fresh validator must distinguish missing optional evidence from a required policy-gate failure and must not infer success. **Informational only:** yes.

- **DESIGN-REVIEW-W-004 — Durable-document impact is mostly sound but not fully consistent in WO-006.** **Severity:** warning. **Evidence:** [`WO-006-persistence-recovery.md`](../work-orders/WO-006-persistence-recovery.md:48-52) authorizes updating the persistence/recovery blueprint and manifest, but its affected-document list names only the requirements and two blueprints at [`WO-006-persistence-recovery.md`](../work-orders/WO-006-persistence-recovery.md:60-66). **Suggested action:** at implementation handoff, either include [`project-manifest.md`](../../../project-manifest.md:1) in the affected durable-document record or state why it is not changed; validate the final classification against the actual diff. Do not update documents merely to satisfy the classification. **Informational only:** yes.

### Notes

- **DESIGN-REVIEW-N-001 — Scope alignment is strong.** **Severity:** note. All eight WOs decompose the approved create → edit → transition → undo/redo → layout → save/reopen/recover → export loop. Their exclusions consistently preserve the explicit non-goals; no WO adds DOCX/FDX, collaboration, MCP, planning, broad formatting, web, or multi-platform polish.

- **DESIGN-REVIEW-N-002 — Dependency ordering is understandable and feasible.** **Severity:** note. The sequence [`WO-001-workspace-foundation.md`](../work-orders/WO-001-workspace-foundation.md:16-24) → [`WO-002-model-schema.md`](../work-orders/WO-002-model-schema.md:16-27) → [`WO-003-editing-session.md`](../work-orders/WO-003-editing-session.md:16-27) → [`WO-004-layout-display.md`](../work-orders/WO-004-layout-display.md:16-27) → [`WO-005-focused-desktop-ui.md`](../work-orders/WO-005-focused-desktop-ui.md:16-27) → [`WO-006-persistence-recovery.md`](../work-orders/WO-006-persistence-recovery.md:16-27) → [`WO-007-exports-integration.md`](../work-orders/WO-007-exports-integration.md:16-27) → [`WO-008-e2e-hardening.md`](../work-orders/WO-008-e2e-hardening.md:16-27) exposes ordinary dependencies, including WO-006's explicit dependency on the UI seam and WO-007's dependency on layout, UI, and persistence.

- **DESIGN-REVIEW-N-003 — Requirement and blueprint traceability is adequate.** **Severity:** note. The WOs cover `REQ-WRITING-LOOP-001` through `REQ-WRITING-LOOP-004`, `AC-WRITING-001.1` through `AC-WRITING-004.3`, and `NFR-WRITING-001` through `NFR-WRITING-003` through explicit per-WO references, while the blueprint references preserve model/session/layout/desktop/persistence/export ownership. The final E2E WO appropriately rechecks the complete set rather than introducing new requirements.

- **DESIGN-REVIEW-N-004 — Cascade risk is recorded and the corrected cadence is retained.** **Severity:** note. The plan explicitly records that workspace, schema, or layout defects may invalidate downstream WOs and may require replanning or fix WOs at [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:124-128). It also correctly states no user checkpoint after each WO while preserving a separate fresh validation immediately after each implementation at [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:22-24) and [`execution-log.md`](../execution-log.md:33-37). This review does not convert that risk into a blocker because the dependencies are executable and the governing process defines the fix-work-order path.

- **DESIGN-REVIEW-N-005 — No implementation or validation evidence exists yet.** **Severity:** note. The plan and execution log correctly state that all WOs are `draft`, no implementer or validator has been dispatched, and no implementation, validation, or approval is claimed at [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:122-129) and [`execution-log.md`](../execution-log.md:18-31).

## Questions for follow-up

- Which exact framework/target and Cargo package graph does WO-001 establish? **Context:** [`project-manifest.md`](../../../project-manifest.md:39-44) and [`WO-001-workspace-foundation.md`](../work-orders/WO-001-workspace-foundation.md:42-44). **Owner or source:** workspace-foundation implementer, with user confirmation if a consequential choice remains. **Blocking:** no for design alignment; blocking for dependent execution if the choice is unavailable.
- What pinned font/profile and minimum TXT loss-report contract are selected before layout/export acceptance is treated as concrete? **Context:** [`WO-004-layout-display.md`](../work-orders/WO-004-layout-display.md:18-23) and [`WO-007-exports-integration.md`](../work-orders/WO-007-exports-integration.md:42-49). **Owner or source:** relevant implementer and technical/product owner. **Blocking:** no for this review; the decisions must be evidenced before their dependent acceptance claims.

## Traceability

- **Requirements checked:** [`product-overview.md`](../../../requirements/product-overview.md:43-95), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:36-97), and [`requirements-review.md`](../../../requirements/requirements-review.md:30-78).
- **Blueprints checked:** [`technical-context.md`](../../../blueprints/technical-context.md:17-87), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:5-62), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:19-88), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:18-110), and [`blueprint-review.md`](../../../blueprints/blueprint-review.md:31-85).
- **Work orders checked:** [`WO-001-workspace-foundation.md`](../work-orders/WO-001-workspace-foundation.md:1-74), [`WO-002-model-schema.md`](../work-orders/WO-002-model-schema.md:1-76), [`WO-003-editing-session.md`](../work-orders/WO-003-editing-session.md:1-76), [`WO-004-layout-display.md`](../work-orders/WO-004-layout-display.md:1-77), [`WO-005-focused-desktop-ui.md`](../work-orders/WO-005-focused-desktop-ui.md:1-75), [`WO-006-persistence-recovery.md`](../work-orders/WO-006-persistence-recovery.md:1-76), [`WO-007-exports-integration.md`](../work-orders/WO-007-exports-integration.md:1-74), and [`WO-008-e2e-hardening.md`](../work-orders/WO-008-e2e-hardening.md:1-76).
- **Explicit user decisions checked:** desktop-first Rust MVP; single dependable screenplay writing-loop outcome; listed in/out scope; standard rather than rapid mode; sequential dependency order; no manual checkpoint after each WO; fresh validation immediately after each WO; consolidated human confirmation/closeout; and no implementation or validation claim during planning.
- **Comparison evidence sufficient:** `yes` — the plan, execution log, all eight WOs, requirements, blueprints, and both independent initialization review artifacts make the intended work and boundaries comparable. No implementation evidence was needed or used for this design alignment decision.
- **Explicit project-policy gates checked:** independent design review before execution; human approval required for `review -> execute`; separate fresh implementer and validator sessions; per-WO validation; comparable implementation evidence before validation; unknown commands are not permission to skip checks; and closeout remains mandatory. No project-policy hard-stop is unsatisfied for this review artifact.

## Decision

- **Outcome:** `pass`
- **Verdict:** `PASS WITH WARNINGS`
- **Required next state:** `execute` only after the required human design approval is separately recorded; until then the initiative remains in `review` and no implementation dispatch is authorized.
- **Human approval required:** `yes`
- **Approval/evidence:** None recorded here. This independent review does not claim human approval, implementation, validation, or closeout.
- **Warning acceptance required:** `no` — warnings do not prevent an aligned review from passing, but their suggested actions and decision evidence remain required during execution handoff and validation.

# Review: Design — Post-MVP UI Scope Correction

## Review metadata

- **Review type:** `design`
- **Status:** `in-review`
- **Reviewer:** Independent fresh-context design reviewer
- **Reviewer session/agent ID:** `architect-design-review-ui-correction-2026-10-25`
- **Session exception:** `none`
- **Date:** 2026-10-25
- **Initiative:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:1)
- **Review file:** [`design-review-ui-correction.md`](design-review-ui-correction.md:1)
- **Reviewed artifact:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:1), [`scope-correction-2026-10-25.md`](../scope-correction-2026-10-25.md:1), [`WO-009-unified-live-editing-surface.md`](../work-orders/WO-009-unified-live-editing-surface.md:1), and [`WO-010-basic-named-persistence-ui.md`](../work-orders/WO-010-basic-named-persistence-ui.md:1)

## Scope of review

The comparison boundary is the user-requested post-MVP correction: establish one
primary live page-layout editing surface with source-mapped caret/selection and
role-aware edits, remove the duplicate text-entry interaction and separate
editing-mode split, then add only basic named Open, Save, and Save As UI while
keeping recovery outside the UI. The explicit correction and exclusions in
[`scope-correction-2026-10-25.md`](../scope-correction-2026-10-25.md:11-55) are the
approved direction used for comparison.

Inspected:

- [`00-overview.md`](../../../process/00-overview.md:27-205), [`03-design-review.md`](../../../process/03-design-review.md:1-72), [`04-work-order.md`](../../../process/04-work-order.md:1-30), and [`05-validation.md`](../../../process/05-validation.md:1-128).
- [`project-manifest.md`](../../../project-manifest.md:1-152), the amended initiative plan [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:1), and [`execution-log.md`](../execution-log.md:253-274).
- [`WO-009-unified-live-editing-surface.md`](../work-orders/WO-009-unified-live-editing-surface.md:1-109) and [`WO-010-basic-named-persistence-ui.md`](../work-orders/WO-010-basic-named-persistence-ui.md:1-95).
- [`product-overview.md`](../../../requirements/product-overview.md:33-95) and [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:36-97).
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:5-81), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:30-75), and [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:18-97).
- Current UI evidence in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:12-380), prior UI validation in [`validation-WO-005.md`](../validations/validation-WO-005.md:13-61), prior composed-loop validation in [`validation-WO-008.md`](../validations/validation-WO-008.md:17-132), and the preserved manual checklist [`pre-closeout-test-checklist.md`](../pre-closeout-test-checklist.md:7-72).

This is an alignment and gate-readiness review, not an implementation,
runtime, test-result, debugging, refactoring, or code-quality review. The
current UI and prior validation artifacts are comparison evidence only. No
implementation, new validation, human approval, or runnable behavior is
claimed here.

## Findings

### Blockers

- **UI-CORRECTION-B-001 — Initiative phase and design-approval gate are internally inconsistent.** **Severity:** blocker. The governing process requires the initiative to reach the `review` phase before formal design review and requires recorded human approval before `review -> execute` ([`03-design-review.md`](../../../process/03-design-review.md:3-13), [`00-overview.md`](../../../process/00-overview.md:90-96)). The amended plan still declares `Phase: plan` ([`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:3-6)), while its transition evidence says `plan → review` was established by the earlier review and later says the correction returned the initiative to `plan` ([`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:24). The design-readiness checklist nevertheless marks formal review and human approval as passed ([`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:135-149)), and the recorded approval is expressly limited to WO-001 ([`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:150-158)). The correction note and execution log correctly say WO-009/WO-010 are draft and not approved for implementation ([`scope-correction-2026-10-25.md`](../scope-correction-2026-10-25.md:5-9), [`execution-log.md`](../execution-log.md:253-263).**
  **Required action:** Reconcile the initiative's current phase, readiness checklist, and approval record so this correction has its own `plan -> review` transition evidence and a separate human approval scope before any WO-009 dispatch. Until that reconciliation and approval occur, retain the initiative in `plan` and do not enter `execute`. **Blocking citation:** explicit Forge review dispatch and `review -> execute` gate in [`03-design-review.md`](../../../process/03-design-review.md:3-13) and [`00-overview.md`](../../../process/00-overview.md:90-96). **Required next state:** `plan`.

### Warnings

- **UI-CORRECTION-W-001 — WO-009's intended single-surface design is clear, but source-mapping acceptance evidence should be made directly observable.** **Severity:** warning. The work order explicitly requires page layout by default, direct typing, caret/selection, source mappings, core-routed mutations, removal of the per-element `TextEdit`, and removal of the continuous/page editing split ([`WO-009-unified-live-editing-surface.md`](../work-orders/WO-009-unified-live-editing-surface.md:18-41)). Its acceptance test also requires valid caret/selection after editing ([`WO-009-unified-live-editing-surface.md`](../work-orders/WO-009-unified-live-editing-surface.md:87-91). The current defect evidence shows why this is necessary: [`continuous_view()`](../../../../apps/scriptcraft/src/main.rs:205-236) renders selectable rows plus a separate multiline editor, while [`page_view()`](../../../../apps/scriptcraft/src/main.rs:238-278) is painted and non-editable, and [`update()`](../../../../apps/scriptcraft/src/main.rs:351-379) exposes a mode switch. **Suggested action:** at implementation handoff, require the focused evidence to demonstrate hit-testing and selection across wrapped/page lines, including UTF-8 boundaries and resize/scroll resynchronization, rather than relying only on source inspection or a broad manual narrative. **Informational only:** yes.

- **UI-CORRECTION-W-002 — WO-010 is narrow, but the host file-selection seam and failed-operation evidence are not yet concrete.** **Severity:** warning. The scope correctly limits the work to a current named path, user-selected Open/Save/Save As, status/error behavior, and existing persistence APIs ([`WO-010-basic-named-persistence-ui.md`](../work-orders/WO-010-basic-named-persistence-ui.md:18-43)). However, the exact native selection boundary is left to implementation, and the acceptance section combines selected-path routing, clean/dirty transitions, failure preservation, and no recovery UI ([`WO-010-basic-named-persistence-ui.md`](../work-orders/WO-010-basic-named-persistence-ui.md:58-77)). **Suggested action:** require evidence for no-path Save behavior, current-path Save behavior, Save As non-mutation of the original, open-replacement failure, and failed-save dirty-state preservation. Keep the chosen dialog mechanism app-local and do not alter the core persistence contract. **Informational only:** yes.

- **UI-CORRECTION-W-003 — Durable-document updates and stale baseline wording need handoff reconciliation.** **Severity:** warning. Both new WOs correctly classify requirements as `verify unchanged` and blueprint behavior as `update` ([`WO-009-unified-live-editing-surface.md`](../work-orders/WO-009-unified-live-editing-surface.md:93-99), [`WO-010-basic-named-persistence-ui.md`](../work-orders/WO-010-basic-named-persistence-ui.md:79-85)). The desktop blueprint still records the old fixed `scriptcraft.json` host behavior ([`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:77-81)), while WO-010 authorizes replacing that with app-owned selected/current path behavior. **Suggested action:** apply only the authorized blueprint correction after implementation and validate its classification against the actual diff; do not change requirements or recovery semantics. **Informational only:** yes.

### Notes

- **UI-CORRECTION-N-001 — WO-009 does consolidate the editing model as requested.** The summary, scope, exclusions, and implementation plan all identify one primary live page-layout document surface, page layout as the default, source-mapped caret/selection, direct core-routed edits, and removal of both the duplicate per-element text-entry interaction and the continuous/page editing split ([`WO-009-unified-live-editing-surface.md`](../work-orders/WO-009-unified-live-editing-surface.md:18-49), [`WO-009-unified-live-editing-surface.md`](../work-orders/WO-009-unified-live-editing-surface.md:69-79)). No second editor or parallel authoritative text buffer is authorized.

- **UI-CORRECTION-N-002 — WO-010 has the correct narrow ownership boundary.** It depends on the existing persistence core and WO-009, adds named path selection and status/error behavior only, explicitly excludes recovery UI and persistence-core changes, and prohibits a second editor or editing mode ([`WO-010-basic-named-persistence-ui.md`](../work-orders/WO-010-basic-named-persistence-ui.md:18-43), [`WO-010-basic-named-persistence-ui.md`](../work-orders/WO-010-basic-named-persistence-ui.md:67-77)).

- **UI-CORRECTION-N-003 — Dependency and order are coherent.** The correction orders WO-009 after layout/UI/E2E evidence and WO-010 after persistence, WO-009, and E2E evidence ([`scope-correction-2026-10-25.md`](../scope-correction-2026-10-25.md:36-48)). This prevents named-file integration from reintroducing the superseded editor split. The dependencies are ordinary and execution-feasible; no impossible dependency was found.

- **UI-CORRECTION-N-004 — Prior WO evidence is preserved and correctly bounded.** [`validation-WO-005.md`](../validations/validation-WO-005.md:35-61) and [`validation-WO-008.md`](../validations/validation-WO-008.md:92-132) remain historical passes for their approved boundaries. They explicitly recorded the old continuous/page UI and unavailable visual/manual evidence; they do not establish the new WO-009 interaction contract. The correction note and execution log preserve that distinction ([`scope-correction-2026-10-25.md`](../scope-correction-2026-10-25.md:50-55), [`execution-log.md`](../execution-log.md:259-263)). No prior evidence needs to be rewritten.

- **UI-CORRECTION-N-005 — The requirements and blueprint direction is aligned.** The existing contracts require semantic edits through [`EditingSession`](../../../../crates/scriptcraft-core/src/editing.rs:59), deterministic layout source mappings through [`ScreenplayLayout`](../../../../crates/scriptcraft-core/src/layout.rs:51), no parallel UI text source, and native persistence separate from recovery ([`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:53-75), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:78-90)). The new WOs realize those existing contracts without broadening requirements.

## Questions for follow-up

- Which exact app-local/native file-selection mechanism will satisfy the WO-010 host boundary? **Context:** [`WO-010-basic-named-persistence-ui.md`](../work-orders/WO-010-basic-named-persistence-ui.md:23-35). **Owner or source:** WO-010 implementer, subject to the existing app-boundary constraint. **Blocking:** no for design alignment; it must be recorded before implementation evidence can demonstrate named-path behavior.

- What exact observable evidence will prove that the caret and selection remain source-mapped after wrapping, pagination, scroll, and resize? **Context:** [`WO-009-unified-live-editing-surface.md`](../work-orders/WO-009-unified-live-editing-surface.md:31-41). **Owner or source:** WO-009 implementer and fresh validator. **Blocking:** no for scope alignment; it is a validation-readiness clarification.

## Traceability

- **Requirements checked:** [`product-overview.md`](../../../requirements/product-overview.md:33-95) and [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:36-97).
- **Blueprints checked:** [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:5-81), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:30-75), and [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:18-97).
- **Work orders checked:** [`WO-009-unified-live-editing-surface.md`](../work-orders/WO-009-unified-live-editing-surface.md:1-109) and [`WO-010-basic-named-persistence-ui.md`](../work-orders/WO-010-basic-named-persistence-ui.md:1-95), with prior boundaries in [`WO-005-focused-desktop-ui.md`](../work-orders/WO-005-focused-desktop-ui.md:1-75) and [`WO-008-e2e-hardening.md`](../work-orders/WO-008-e2e-hardening.md:1-76).
- **Explicit user decisions checked:** one live page-layout editing surface; no duplicate text editor; source-mapped caret/selection; role-aware editing; named Open/Save/Save As only; recovery outside the UI; sequential WO-009 then WO-010 order; preservation of WO-001 through WO-008 evidence; no implementation or new validation claim in the correction planning step.
- **Comparison evidence sufficient:** `yes` for scope, boundaries, dependencies, and acceptance intent. Current UI evidence and prior validation evidence are sufficient to explain the correction. The current phase/approval ledger is not gate-consistent, which is the blocker above.
- **Explicit project-policy gates checked:** formal design review only after the initiative reaches `review`; human approval required before `review -> execute`; separate fresh implementer and validator sessions; per-WO validation; comparable evidence before validation; no configured manual desktop gate ([`project-manifest.md`](../../../project-manifest.md:88-95), [`00-overview.md`](../../../process/00-overview.md:90-116)).

## Decision

- **Outcome:** `blocked`
- **Verdict:** `BLOCKED — ALIGNED DESIGN, GATE LEDGER REQUIRES CORRECTION`
- **Required next state:** `plan` until the amended initiative phase/readiness/approval records are reconciled; then `review` for the corrected initiative gate. No implementation dispatch is authorized by this review.
- **Human approval required:** `yes`
- **Approval/evidence:** None recorded here. The earlier approval is explicitly limited to WO-001 and is not approval for WO-009 or WO-010 ([`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:150-158)).
- **Warning acceptance required:** `no`


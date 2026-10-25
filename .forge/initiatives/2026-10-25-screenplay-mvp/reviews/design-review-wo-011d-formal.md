# Review: Design — Amended Screenplay MVP / WO-011d

## Review metadata

- **Review type:** `design`
- **Status:** `in-review`
- **Reviewer:** Independent fresh-context design reviewer
- **Reviewer session/agent ID:** `architect-formal-design-review-wo-011d-2026-10-25`
- **Session exception:** `none`
- **Date:** 2026-10-25
- **Initiative:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:1)
- **Review file:** [`design-review-wo-011d-formal.md`](design-review-wo-011d-formal.md:1)
- **Prior review preserved:** [`design-review-wo-011d.md`](design-review-wo-011d.md:1)
- **Reviewed artifact:** The reconciled initiative plan and amended work-order set, with [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:1) as the new draft work order.

## Scope of review

The comparison boundary is the amended initiative outcome: a dependable,
desktop-first screenplay writing loop with one live page-layout editing surface,
semantic edits routed through the existing session, deterministic layout/caret
updates, named native persistence UI, and no duplicate editor or hidden text
buffer. The new correction boundary is native macOS keyboard/IME capture
hardening at the existing painted editor boundary. Synthetic/rendered egui
evidence remains explicitly distinct from native macOS proof.

This is a fresh alignment and policy-readiness review. It assesses only outcome
alignment, explicit scope, ordinary dependencies, durable-document
classifications, evidence boundaries, and Forge gate readiness. It does not
assess source quality, implementation completeness, runtime behavior, debugging,
or whether the proposed hardening will solve the native discrepancy.

Inspected:

- [`00-overview.md`](../../../process/00-overview.md:27-205), [`03-design-review.md`](../../../process/03-design-review.md:1-72), and [`project-manifest.md`](../../../project-manifest.md:73-109).
- The reconciled initiative plan [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:1-169), [`execution-log.md`](../execution-log.md:18-28), and prior blocked review [`design-review-wo-011d.md`](design-review-wo-011d.md:39-84).
- [`WO-009-unified-live-editing-surface.md`](../work-orders/WO-009-unified-live-editing-surface.md:1-129), [`WO-010-basic-named-persistence-ui.md`](../work-orders/WO-010-basic-named-persistence-ui.md:1-109), [`WO-011-keyboard-text-input-loop.md`](../work-orders/WO-011-keyboard-text-input-loop.md:1-259), [`WO-011a-retain-page-editor-focus.md`](../work-orders/WO-011a-retain-page-editor-focus.md:1-115), [`WO-011b-wire-rendered-page-editor-focus.md`](../work-orders/WO-011b-wire-rendered-page-editor-focus.md:1-123), [`WO-011c-prove-pointer-rendered-focus.md`](../work-orders/WO-011c-prove-pointer-rendered-focus.md:1-126), and [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:1-169).
- [`keyboard-input-handoff.md`](../keyboard-input-handoff.md:1-114), [`validation-WO-011.md`](../validations/validation-WO-011.md:48-69), [`validation-WO-011a.md`](../validations/validation-WO-011a.md:48-61), [`validation-WO-011b.md`](../validations/validation-WO-011b.md:47-61), [`validation-WO-011c.md`](../validations/validation-WO-011c.md:47-54), and the WO-009/WO-010 execution evidence in [`execution-log.md`](../execution-log.md:300-358).
- [`product-overview.md`](../../../requirements/product-overview.md:52-95), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:38-97), and [`requirements-review.md`](../../../requirements/requirements-review.md:30-78).
- [`technical-context.md`](../../../blueprints/technical-context.md:29-71), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:5-82), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:30-75), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:78-119), and [`blueprint-review.md`](../../../blueprints/blueprint-review.md:31-85).

The earlier blocked review is historical evidence only. Its phase blocker is
re-evaluated against the current ledger, which now records `plan → review`.

## Findings

### Blockers

- **None.** The initiative is now in `review` as recorded in the plan and execution log, so the prior dispatch-phase blocker is resolved for this review. WO-009 and WO-010 are bounded completed correction slices; the WO-011 → WO-011a → WO-011b → WO-011c chain is preserved as completed rendered/synthetic focus evidence; and WO-011d is an identifiable, draft, lettered hardening work order directly aimed at the unresolved native-input discrepancy. No work order is clearly unrelated, contrary to an explicit approved decision, dependent on an impossible ordinary execution dependency, or blocked by an explicit project-policy gate at the design-review stage.

### Warnings

- **WO-011D-W-001 — Native macOS input remains unresolved and is not proven by this review.** **Severity:** warning. The handoff records a native window that can show a caret while accepting no typed text and rejects synthetic/rendered tests as native proof ([`keyboard-input-handoff.md`](../keyboard-input-handoff.md:47-69)). The passed WO-011c validation proves the rendered pointer/focus path only and records visual/window interaction as unavailable ([`validation-WO-011c.md`](../validations/validation-WO-011c.md:27-35)). WO-011d retains a separate observable macOS gate and requires unavailable observations to remain unavailable ([`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:126-143)). **Required action:** preserve this boundary in implementation, validation, and closeout; do not report design review, synthetic tests, or bounded launch as native typing success. Informational only: yes.

- **WO-011D-W-002 — The single-response invariant needs a precise implementation handoff.** **Severity:** warning. The observed basis describes one response per rendered page ([`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:26-34)), while the proposed scope says one stable editor/canvas response and ID own the whole editing surface ([`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:42-55)). The work is identifiable and aligned, but the implementer must make explicit whether the scrollable multi-page surface has one enclosing response or stable per-page responses under one editor boundary. **Required action:** make whole-surface focus ownership, per-page hit-testing, and the rendered regression observable without adding a second editing surface. Informational only: yes.

- **WO-011D-W-003 — Diagnostics require a bounded lifecycle and privacy-safe reconciliation.** **Severity:** warning. WO-011d permits labeled runtime diagnostics for focus and event categories but forbids raw document-text logging and model mutation ([`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:56-63), [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:104-113)). It authorizes a desktop blueprint update only if the verified implementation changes the durable contract ([`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:115-124), [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:145-154)). **Required action:** record whether diagnostics are removed or retained as bounded status instrumentation, ensure no document contents or hidden state are introduced, and reconcile the final blueprint classification against the actual diff. Informational only: yes.

- **WO-011D-W-004 — Native/manual evidence is an acceptance boundary, not implementation authorization.** **Severity:** warning. The work order requires an observable macOS window run, while the manifest says no manual/external gate is currently configured as a project-policy hard stop ([`project-manifest.md`](../../../project-manifest.md:88-95)). The handoff nevertheless keeps the discrepancy as a closeout blocker unless resolved or explicitly accepted by the owner ([`keyboard-input-handoff.md`](../keyboard-input-handoff.md:112-114)). **Required action:** record attempted, passed, failed, or unavailable native observations; if unavailable, retain the held closeout condition rather than infer success. Informational only: yes.

### Notes

- **WO-011D-N-001 — The amended work-order set remains aligned with one outcome.** WO-009 corrects the desktop interaction model to one live page-layout editor, WO-010 adds only named Open/Save/Save As integration while keeping recovery outside the UI, and WO-011d hardens that editor's native input boundary. Their exclusions preserve the explicit non-goals: no compatibility formats, collaboration, MCP, planning, broad formatting, web support, or second editor ([`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:49-65); [`WO-009-unified-live-editing-surface.md`](../work-orders/WO-009-unified-live-editing-surface.md:43-50); [`WO-010-basic-named-persistence-ui.md`](../work-orders/WO-010-basic-named-persistence-ui.md:36-43); [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:65-75)).

- **WO-011D-N-002 — Dependencies are understandable and executable.** WO-009 depends on the existing layout/UI/E2E boundaries; WO-010 depends on persistence, WO-009, and WO-008; WO-011 and its lettered fixes depend on the unified surface and existing core contracts; WO-011d follows the completed WO-011c chain and links the handoff. The chain is ordinary and contains no impossible dependency ([`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:112-127); [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:5-15)).

- **WO-011D-N-003 — Requirements and blueprint alignment is appropriate.** The amended work orders preserve canonical UTF-8 semantics, source-mapped caret/layout behavior, core-owned mutation, and desktop-owned input/focus/IME responsibilities ([`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:38-59); [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:5-10); [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:53-75)). No amended work order expands the product contract or moves semantic authority into the shell.

- **WO-011D-N-004 — Durable-document classifications are directionally correct.** WO-009 and WO-010 use `verify unchanged` for requirements and `update` for their affected desktop/feature blueprint boundary; WO-011d uses `verify unchanged` for requirements and conditional `update` for the desktop/input contract ([`WO-009-unified-live-editing-surface.md`](../work-orders/WO-009-unified-live-editing-surface.md:93-100); [`WO-010-basic-named-persistence-ui.md`](../work-orders/WO-010-basic-named-persistence-ui.md:79-85); [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:145-154)). Final validation must compare declarations with the actual diff; classification is not a requirement to edit a durable document.

- **WO-011D-N-005 — Prior evidence is correctly bounded and preserved.** The WO-011 through WO-011c chain establishes progressively stronger synthetic/rendered focus evidence, and the passed WO-011c validation closes that rendered fix chain. It does not establish native macOS typing. Failed validations remain historical evidence and are not rewritten ([`validation-WO-011.md`](../validations/validation-WO-011.md:48-69); [`validation-WO-011a.md`](../validations/validation-WO-011a.md:48-61); [`validation-WO-011b.md`](../validations/validation-WO-011b.md:47-61); [`validation-WO-011c.md`](../validations/validation-WO-011c.md:47-54)).

## Questions for follow-up

- What exact enclosing response/ID shape will satisfy whole-surface focus ownership while preserving per-page hit testing? **Context:** [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:42-55). **Owner or source:** WO-011d implementer and fresh validator. **Blocking:** no for design alignment; required for an unambiguous implementation handoff.
- Are diagnostics temporary and removed, or retained as bounded status instrumentation? **Context:** [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:56-63). **Owner or source:** implementer/orchestrator, reconciled against the final diff and desktop blueprint. **Blocking:** no for design alignment; required for durable-document and closeout accuracy.

## Traceability

- **Requirements checked:** [`product-overview.md`](../../../requirements/product-overview.md:52-95), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:38-97), and [`requirements-review.md`](../../../requirements/requirements-review.md:30-78).
- **Blueprints checked:** [`technical-context.md`](../../../blueprints/technical-context.md:29-71), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:5-82), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:30-75), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:78-119), and [`blueprint-review.md`](../../../blueprints/blueprint-review.md:31-85).
- **Work orders checked:** [`WO-009-unified-live-editing-surface.md`](../work-orders/WO-009-unified-live-editing-surface.md:1-129), [`WO-010-basic-named-persistence-ui.md`](../work-orders/WO-010-basic-named-persistence-ui.md:1-109), [`WO-011-keyboard-text-input-loop.md`](../work-orders/WO-011-keyboard-text-input-loop.md:1-259), [`WO-011a-retain-page-editor-focus.md`](../work-orders/WO-011a-retain-page-editor-focus.md:1-115), [`WO-011b-wire-rendered-page-editor-focus.md`](../work-orders/WO-011b-wire-rendered-page-editor-focus.md:1-123), [`WO-011c-prove-pointer-rendered-focus.md`](../work-orders/WO-011c-prove-pointer-rendered-focus.md:1-126), and [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:1-169).
- **Explicit user decisions checked:** one page-layout editing surface; no duplicate/hidden editor; preserve the semantic session; basic named persistence only with recovery outside the UI; standard mode; sequential work orders; separate fresh validation; no per-WO manual checkpoint; native macOS behavior remains unresolved; and WO-011d is bounded planning direction until formal review and separate amended approval pass.
- **Comparison evidence sufficient:** `yes` for alignment, scope, dependencies, durable-document intent, and policy readiness. It is not evidence of native runtime success.
- **Explicit project-policy gates checked:** initiative in `review` before formal design review; human approval required before `review → execute`; fresh implementer and validator sessions; per-work-order validation; comparable evidence before validation; honest treatment of unavailable native/manual evidence; and mandatory closeout ([`03-design-review.md`](../../../process/03-design-review.md:3-13); [`00-overview.md`](../../../process/00-overview.md:39-48); [`00-overview.md`](../../../process/00-overview.md:90-116); [`project-manifest.md`](../../../project-manifest.md:88-95)). The phase gate is satisfied for review; the human-approval gate remains pending.

## Decision

- **Outcome:** `pass`
- **Verdict:** `PASS WITH WARNINGS`
- **Amended plan ready for human design approval:** `yes` for the reviewed alignment boundary. This means the work is sufficiently identified and aligned for a separate approval decision; it does not record or imply approval.
- **Required next state:** Remain in `review` pending separate recorded human design approval explicitly covering WO-009/WO-010 and WO-011d. After approval, the orchestrator may record `review → execute` and dispatch a fresh implementer for WO-011d; this review itself does not authorize implementation.
- **Human approval required:** `yes`
- **Approval/evidence:** None recorded here. Historical approval remains limited to WO-001 and is not reused ([`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:160-168)).
- **Warning acceptance required:** `no`; warnings do not prevent this aligned review from passing, but their handoff actions and the native evidence boundary remain mandatory for implementation, validation, and closeout records.
- **Native macOS evidence boundary:** unresolved and preserved. No native typing, Unicode, editing-key, or visible rerender success is claimed by this review, the prior rendered validation chain, or bounded launch evidence.

# Review: Design — WO-011d Native Input Capture Hardening

## Review metadata

- **Review type:** `design`
- **Status:** `in-review`
- **Reviewer:** Independent fresh-context design reviewer
- **Reviewer session/agent ID:** `architect-design-review-wo-011d-2026-10-25`
- **Session exception:** `none`
- **Date:** 2026-10-25
- **Initiative:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:1)
- **Review file:** [`design-review-wo-011d.md`](design-review-wo-011d.md:1)
- **Reviewed artifact:** [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:1)

## Scope of review

The comparison boundary is the amended initiative outcome: a dependable,
desktop-first screenplay writing loop with one live page-layout editing surface,
semantic edits routed through the existing session, deterministic layout/caret
updates, and no duplicate editor or hidden text buffer ([`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:36-65)).
For this work order, the specific approved direction is to harden the unresolved
native macOS keyboard/IME capture boundary with one stable editor/canvas focus
boundary, focused event consumption, bounded diagnostics, and honest native
smoke evidence ([`keyboard-input-handoff.md`](../keyboard-input-handoff.md:72-84)).

This is an alignment and gate-readiness review, not an implementation, runtime,
debugging, code-quality, or validation review. WO-011d remains `draft`; no
implementation, native proof, validation, or amended human approval is claimed.

Inspected:

- [`00-overview.md`](../../../process/00-overview.md:27-205), [`03-design-review.md`](../../../process/03-design-review.md:1-72), and [`project-manifest.md`](../../../project-manifest.md:73-109).
- The amended initiative plan ([`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:1-162)) and [`execution-log.md`](../execution-log.md:349-472).
- [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:1-169), the original [`WO-011-keyboard-text-input-loop.md`](../work-orders/WO-011-keyboard-text-input-loop.md:1-214), and the completed [`WO-011c-prove-pointer-rendered-focus.md`](../work-orders/WO-011c-prove-pointer-rendered-focus.md:1-127).
- [`keyboard-input-handoff.md`](../keyboard-input-handoff.md:1-114), including the unresolved native macOS discrepancy and its planning-only authorization.
- Requirements [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:38-97) and blueprints [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:5-82), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:18-119), and [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:30-75).
- Prior validation chain [`validation-WO-011.md`](../validations/validation-WO-011.md:48-69), [`validation-WO-011a.md`](../validations/validation-WO-011a.md:48-60), [`validation-WO-011b.md`](../validations/validation-WO-011b.md:47-61), and [`validation-WO-011c.md`](../validations/validation-WO-011c.md:47-54).

## Findings

### Blockers

- **WO-011D-B-001 — The formal amended design-review gate is not open and amended human approval is absent.** **Severity:** blocker. Forge requires the initiative to reach the `review` phase before formal design review, and requires recorded human design approval before `review -> execute` ([`03-design-review.md`](../../../process/03-design-review.md:3-13), [`00-overview.md`](../../../process/00-overview.md:90-96)). The initiative currently records `Phase: plan` ([`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:3-6)) and explicitly says that no amended `plan → review` transition has been recorded and amended human approval is pending ([`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:24-31)). The execution log likewise records the initiative as `plan` and says the next action is to record the transition and complete the corrected review gate before approval ([`execution-log.md`](../execution-log.md:265-274)). The only recorded human approval is historical approval for WO-001 only and does not authorize amended work ([`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:153-161)). The handoff explicitly limits the WO-011d authorization to planning direction and preserves design review and amended human approval as prerequisites ([`keyboard-input-handoff.md`](../keyboard-input-handoff.md:72-84)).
  **Required action:** Keep the initiative in `plan`, record a truthful amended `plan → review` transition only when the orchestrator determines the amended plan and work-order set are ready, then obtain separate human approval whose scope explicitly includes WO-011d before implementation dispatch. Do not treat this artifact as authorization to implement. **Blocking citation:** explicit Forge dispatch and approval gates in [`03-design-review.md`](../../../process/03-design-review.md:3-13) and [`00-overview.md`](../../../process/00-overview.md:90-96). **Required next state:** `plan`.

### Warnings

- **WO-011D-W-001 — Native macOS behavior remains an unresolved outcome risk.** **Severity:** warning. The handoff reports that the native window can show a caret while accepting no typed text, despite passing synthetic/rendered-path tests, and expressly says not to treat those tests as native proof ([`keyboard-input-handoff.md`](../keyboard-input-handoff.md:47-69)). The prior WO-011c validation passed the rendered focus chain but recorded bounded launch smoke with no visual/window interaction evidence ([`validation-WO-011c.md`](../validations/validation-WO-011c.md:27-35), [`validation-WO-011c.md`](../validations/validation-WO-011c.md:51-54)). WO-011d correctly retains a separate observable native macOS gate and requires unavailable observations to remain recorded as unavailable ([`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:126-143)). **Required action:** Preserve this discrepancy in all handoffs and later validation/closeout records; do not promote the draft or the existing rendered validation chain into a claim that native typing works. This is not a design-alignment blocker because the proposed work directly addresses the reported discrepancy and identifies an ordinary execution/acceptance path.

- **WO-011D-W-002 — The “one stable response” boundary needs an explicit implementation-time invariant.** **Severity:** warning. The current observed basis describes one response per rendered page with an aggregate focus gate ([`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:28-34)), while the proposed scope calls for one stable editor/canvas response and ID for the whole editing surface ([`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:42-55)). The intended work is identifiable and aligned, but the acceptance wording should make clear whether the whole scrollable multi-page surface has exactly one response or whether a stable response per page is retained beneath one enclosing editor boundary. **Required action:** At implementation handoff, make the single-boundary invariant and its rendered regression observable without introducing a second editing surface. No replan is required for this warning.

- **WO-011D-W-003 — Temporary diagnostics need a bounded lifecycle and documentation reconciliation.** **Severity:** warning. WO-011d authorizes clearly labeled native-input diagnostics and conditionally authorizes a desktop blueprint update if the durable contract changes ([`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:56-63), [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:115-124), [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:145-154)), but does not state whether diagnostics are removed after the native discrepancy is resolved or retained as bounded product status. **Required action:** Record diagnostic retention/removal and any actual blueprint change against the final diff; keep diagnostics from logging raw document text or creating state, as the work order already requires. This is a handoff/validation concern, not a blocker.

### Notes

- **WO-011D-N-001 — The proposed work is generally aligned with the initiative outcome.** The initiative requires a single live page-layout editing surface and the desktop blueprint assigns keyboard/IME ownership to the desktop shell while delegating semantic edits to the core ([`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:49-65), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:5-10), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:35-48)). WO-011d preserves [`EditingSession`](../../../crates/scriptcraft-core/src/editing.rs:59) as the source of truth, keeps the painted surface, and excludes semantic model, layout, persistence, recovery, export, and file-dialog changes ([`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:42-75)). No unrelated feature or explicit user exclusion is introduced.

- **WO-011D-N-002 — The work-order dependency chain is ordinary and executable.** WO-011d is explicitly a lettered hardening work order after WO-011c and links the original WO-011, all prior fixes, the handoff, and the passed WO-011c validation ([`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:5-15)). The prior chain is preserved: WO-011c passed the rendered pointer/focus and later-frame dispatch boundary, while its native visual evidence remained unavailable ([`validation-WO-011c.md`](../validations/validation-WO-011c.md:29-35), [`validation-WO-011c.md`](../validations/validation-WO-011c.md:51-54)). No impossible dependency was found.

- **WO-011D-N-003 — Requirements and blueprint direction are appropriate.** The references to semantic editing, UTF-8 boundaries, deterministic source mappings, and desktop input ownership are consistent with the requirement and blueprint contracts ([`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:38-59), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:78-90), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:53-75)). `verify unchanged` requirements plus a conditional desktop-boundary blueprint update is a reasonable durable-document classification for a UI-boundary hardening work order, provided the final diff supports it ([`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:145-154)).

- **WO-011D-N-004 — Prior validation evidence is correctly bounded.** The WO-011 through WO-011c chain establishes progressively stronger synthetic/rendered focus evidence and closes that fix chain, but it does not establish native macOS typing. The review preserves both facts and does not rewrite or invalidate the historical artifacts ([`validation-WO-011.md`](../validations/validation-WO-011.md:48-69), [`validation-WO-011a.md`](../validations/validation-WO-011a.md:48-60), [`validation-WO-011b.md`](../validations/validation-WO-011b.md:47-61), [`validation-WO-011c.md`](../validations/validation-WO-011c.md:47-54)).

- **WO-011D-N-005 — Work-order status is accurately draft/not implemented.** WO-011d identifies its implementer session as unassigned, marks itself `draft`, and requires a separate fresh implementer followed by a fresh validator ([`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:3-15), [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:136-143)). The initiative plan likewise records that no implementation dispatch is authorized before the amended review and human approval ([`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:121-127)).

## Traceability

- **Requirements checked:** [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:38-97), especially `REQ-WRITING-LOOP-001`, `AC-WRITING-001.2`–`001.5`, `AC-WRITING-002.1`, `AC-WRITING-002.4`, and `NFR-WRITING-002`.
- **Blueprints checked:** [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:5-82), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:18-119), and [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:30-75).
- **Work orders checked:** [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:1-169), [`WO-011-keyboard-text-input-loop.md`](../work-orders/WO-011-keyboard-text-input-loop.md:1-214), and [`WO-011c-prove-pointer-rendered-focus.md`](../work-orders/WO-011c-prove-pointer-rendered-focus.md:1-127).
- **Explicit user decisions checked:** one page-layout editing surface; no duplicate/hidden editor; preserve the semantic session; native macOS behavior remains unresolved; WO-011d is planning direction only; standard mode; sequential work orders; separate fresh validation; and separate amended human approval before implementation.
- **Comparison evidence sufficient:** `yes` for initiative alignment, scope boundaries, dependencies, and durable-document intent. It is not sufficient to claim native runtime success, and this review makes no such claim.
- **Explicit project-policy gates checked:** design review only after `review` phase; human approval before `review → execute`; separate fresh implementer and validator sessions; per-work-order validation; comparable evidence before validation; honest treatment of unavailable native/manual evidence; and mandatory closeout ([`03-design-review.md`](../../../process/03-design-review.md:3-13), [`00-overview.md`](../../../process/00-overview.md:39-48), [`00-overview.md`](../../../process/00-overview.md:90-116), [`project-manifest.md`](../../../project-manifest.md:88-95)). The amended phase/approval gate is unsatisfied.

## Decision

- **Outcome:** `blocked`
- **Verdict:** `BLOCKED — WO-011d IS ALIGNED AS A DRAFT, BUT THE AMENDED DESIGN-REVIEW/APPROVAL GATE IS NOT OPEN`
- **Amended plan ready for human design approval:** `no` in the current ledger state. The WO-011d direction is sufficiently identifiable and bounded for the orchestrator to reconcile the gate, but this review cannot authorize human approval or implementation while the initiative remains in `plan` without a recorded amended `plan → review` transition.
- **Required next state:** `plan`. Reconcile/record the amended `plan → review` transition, then obtain separate human design approval covering WO-011d. Only after a passing amended review and recorded approval may the initiative enter `execute` and dispatch a fresh implementer. Keep WO-011d `draft` and not implemented.
- **Human approval required:** `yes`
- **Approval/evidence:** None for the amended scope. The historical approval is limited to WO-001 ([`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:153-161)).
- **Warning acceptance required:** `no`; warnings do not independently block a passing alignment review, but their actions must remain in the implementation/validation handoff.


# Review: Design — Title-Bar Transition and Elements Alignment v2

## Review metadata

- **Review type:** `design`
- **Status:** `in-review`
- **Reviewer:** Fresh-context Forge design reviewer
- **Reviewer session/agent ID:** `architect-fresh-context-titlebar-transition-and-elements-alignment-v2-design-review-2026-10-27T1906Z`
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Initiative:** [`titlebar-transition-and-elements-alignment-v2.md`](../titlebar-transition-and-elements-alignment-v2.md:1)
- **Review file:** [`design-review-v2.md`](design-review-v2.md:1)
- **Reviewed artifact:** v2 plan, four `V2-WO-*` work orders, and pending validation anchors

## Scope of review

The comparison boundary is the v2 plan's single outcome: an evidence-bounded, implementation-ready direction for transition-time metrics, Elements cursor ownership, and toolbar-bottom surface ownership, with an integrated correction/reconciliation boundary. The user authorized conversion of research into four work orders; that authorization is **not** human design approval and does not authorize implementation.

Inspected:

- [`titlebar-transition-and-elements-alignment-v2.md`](../titlebar-transition-and-elements-alignment-v2.md:1)
- [`V2-WO-001-transition-metric-native-contract.md`](../work-orders/V2-WO-001-transition-metric-native-contract.md:1)
- [`V2-WO-002-elements-cursor-ownership.md`](../work-orders/V2-WO-002-elements-cursor-ownership.md:1)
- [`V2-WO-003-toolbar-bottom-surface-ownership.md`](../work-orders/V2-WO-003-toolbar-bottom-surface-ownership.md:1)
- [`V2-WO-004-integrated-native-validation-and-document-reconciliation.md`](../work-orders/V2-WO-004-integrated-native-validation-and-document-reconciliation.md:1)
- [`validation-V2-WO-001.md`](../validations/validation-V2-WO-001.md:1), [`validation-V2-WO-002.md`](../validations/validation-V2-WO-002.md:1), [`validation-V2-WO-003.md`](../validations/validation-V2-WO-003.md:1), and [`validation-V2-WO-004.md`](../validations/validation-V2-WO-004.md:1)
- [`_index.md`](../../../_index.md:1), [`00-overview.md`](../../../process/00-overview.md:1), [`02-initiative.md`](../../../process/02-initiative.md:1), and [`03-design-review.md`](../../../process/03-design-review.md:1)
- [`project-manifest.md`](../../../project-manifest.md:1), requirements, desktop/feature/technical blueprints, current [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1), and the closed v1 plan/review/validation evidence
- Installed API evidence as recorded in the v2 plan and parent validation artifacts: optional observed viewport state/rectangles, request-style `ViewportCommand::Maximized(bool)`, macOS stale-state limitation, no supported transition flag/timeline, and global egui scale APIs that are not page-only controls ([`titlebar-transition-and-elements-alignment-v2.md`](../titlebar-transition-and-elements-alignment-v2.md:168-178), [`validation-WO-001.md`](../validations/validation-WO-001.md:29-33))

This is an alignment review only. It does not validate implementation, native smoke, source correctness, or the pending v2 validation artifacts, and it does not authorize implementation.

## Findings

### Blockers

- **None.** The research-to-work-order trace is identifiable, all four WOs are related to the stated v2 outcome, the API constraints are carried forward rather than contradicted, and no current project-policy gate makes comparison impossible. The pending validation anchors remain placeholders, as intended.

### Warnings

- **V2-DESIGN-W-001 — Conditional metric freeze is implementable only as an evidence-gated optional branch.** **Severity:** warning. **Evidence:** [`V2-WO-001`](../work-orders/V2-WO-001-transition-metric-native-contract.md:18-31,55-73) correctly rejects `has_requested_resize`, desktop/work-area polling, native animation control, and global `set_pixels_per_point()`, but “reliable transition interval” is not operationally defined. The v2 research establishes that no portable interval/progress API exists ([`titlebar-transition-and-elements-alignment-v2.md`](../titlebar-transition-and-elements-alignment-v2.md:172-178)). **Recommendation:** Make the default outcome explicitly no freeze; require a recorded observable boundary and a page-only metric state machine before enabling a freeze. Define release/recovery and input/IME/native-scale checks as prerequisites, not merely follow-up evidence. **Required next state:** `review`; clarify before implementation dispatch, without inventing a resize flag or global egui zoom lock.

- **V2-DESIGN-W-002 — Elements cursor ownership boundary is measurable but runtime winner arbitration remains conditional.** **Severity:** warning. **Evidence:** [`V2-WO-002`](../work-orders/V2-WO-002-elements-cursor-ownership.md:18-29,43-68) identifies the group, stock ComboBox, decorative label, popup, gap, and document response and requires separate native pointer observations. It safely preserves the stock ComboBox unless evidence requires a narrow wrapper, but a parent cursor request is not guaranteed to override a child/stock response. **Recommendation:** Require the ledger to record the actual winning response or an explicit unavailable result for every region before selecting an overlay/wrapper; prohibit any correction that steals ComboBox activation, keyboard ownership, or popup routing. **Required next state:** `review`; retain the non-sensing-label-first boundary and do not treat deterministic cursor intent tests as native cursor proof.

- **V2-DESIGN-W-003 — Toolbar-bottom ownership has a real correction boundary and correctly forbids blind painting, but native-unresolved results need explicit disposition.** **Severity:** warning. **Evidence:** [`V2-WO-003`](../work-orders/V2-WO-003-toolbar-bottom-surface-ownership.md:18-29,43-68) requires panel/frame/clip/physical-pixel evidence, permits correction only for an app-owned defect, and explicitly refuses paint-over fixes for native/compositor space. Its acceptance criterion permits an explicitly unresolved owner. **Recommendation:** Preserve “no correction” as the required outcome when ownership remains unresolved, and require the record to distinguish app-owned, adjacent-app-fill, device-rounding, and native/compositor results before changing height, padding, or fill. **Required next state:** `review`; native uncertainty is a warning under current policy, not permission to infer ownership.

- **V2-DESIGN-W-004 — Integrated WO-004 is a genuine evidence/correction boundary, not an administrative shortcut.** **Severity:** warning. **Evidence:** [`V2-WO-004`](../work-orders/V2-WO-004-integrated-native-validation-and-document-reconciliation.md:18-29,52-71) requires completed independent WO-001–003 evidence, compares actual diffs, permits source changes only for an evidenced in-scope defect, and retains separate validation and approval boundaries. **Recommendation:** Dispatch it only after the first three WOs are implemented and independently validated; if no defect is evidenced, no source or durable-document change is the correct result. **Required next state:** `review`; retain the explicit prohibition on design-review or human-approval substitution.

- **V2-DESIGN-W-005 — Native acceptance is sufficiently scoped for alignment but not numerically complete for every runtime observation.** **Severity:** warning. **Evidence:** The WOs enumerate per-frame rectangles, metric fields, cursor regions, pixel rows, and deterministic tolerances where applicable ([`V2-WO-001`](../work-orders/V2-WO-001-transition-metric-native-contract.md:68-73), [`V2-WO-002`](../work-orders/V2-WO-002-elements-cursor-ownership.md:63-68), [`V2-WO-003`](../work-orders/V2-WO-003-toolbar-bottom-surface-ownership.md:63-68)). They do not define a universal numeric cursor or native-pixel tolerance, which is reasonable for platform-owned behavior but must not be silently converted into pass claims. **Recommendation:** Record exact observation criteria and an unavailable/unresolved outcome for each native protocol; keep source/API evidence separate from native success. **Required next state:** `review`; this is non-blocking under the design-review policy.

- **V2-DESIGN-W-006 — Branch and phase gates remain pending and must be reconciled before execution.** **Severity:** warning. **Evidence:** The v2 plan records `Phase: plan`, base `master`, and no v2 initiative branch because this authoring session does not create one ([`titlebar-transition-and-elements-alignment-v2.md`](../titlebar-transition-and-elements-alignment-v2.md:3-22)). Project policy requires a local initiative branch when capability is confirmed, and requires human design approval at `review -> execute` ([`project-manifest.md`](../../../project-manifest.md:61-71,88-95)); the Forge process requires the design review plus recorded human approval before execution ([`00-overview.md`](../../../process/00-overview.md:90-96)). **Recommendation:** Before any implementer dispatch, move the initiative through the review gate, verify/create and record the required v2 branch and base, and record separate human design approval. Do not infer v2 execution state from the v1 execution log or closed-parent evidence. **Required next state:** `review`; no implementation dispatch from this artifact.

### Notes

- **V2-DESIGN-N-001 — Research-to-WO traceability is strong.** Investigation A maps to WO-001, Investigation B to WO-002, Investigation C to WO-003, and the cross-cutting measurement/reconciliation boundary maps to WO-004 ([`titlebar-transition-and-elements-alignment-v2.md`](../titlebar-transition-and-elements-alignment-v2.md:208-225)). The four WOs preserve the `128.0`-point Elements baseline and the corrected request-vs-observation, no-global-zoom, no-polling, no-animation-control, and no-blind-painting constraints.

- **V2-DESIGN-N-002 — Dependency order is executable.** WO-001 establishes shared metric/rectangle conventions; WO-002 and WO-003 consume that evidence for distinct ownership paths; WO-004 consumes all three. Sequential execution matches the manifest default and avoids merge ambiguity ([`V2-WO-001`](../work-orders/V2-WO-001-transition-metric-native-contract.md:75-79), [`V2-WO-002`](../work-orders/V2-WO-002-elements-cursor-ownership.md:70-74), [`V2-WO-003`](../work-orders/V2-WO-003-toolbar-bottom-surface-ownership.md:70-74), [`V2-WO-004`](../work-orders/V2-WO-004-integrated-native-validation-and-document-reconciliation.md:73-77)). WO-002 and WO-003 could be parallelized only with an explicit merge strategy; sequential order is safe.

- **V2-DESIGN-N-003 — Durable-document classifications are defensible.** Requirements and blueprint impacts are explicitly classified with rationale and authorized update boundaries in every WO. `verify unchanged` is appropriate by default because the existing requirements/blueprints already express the bounded contracts; an update remains conditional on evidence, especially for a confirmed cursor/surface/metric contract. Final classification must match the actual diff ([`00-overview.md`](../../../process/00-overview.md:141-156)).

## Traceability

- **Requirements checked:** [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90), including AC-WRITING-005.4 through AC-WRITING-005.7.
- **Blueprints checked:** [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:59), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:78), and [`technical-context.md`](../../../blueprints/technical-context.md:31).
- **Parent evidence checked:** closed v1 plan/review, four v1 work orders/validations, current source, and the v1 execution evidence; parent evidence is historical context and is not treated as proof of v2 behavior.
- **API constraints checked:** installed eframe/egui/egui-winit/winit findings recorded in the v2 plan and v1 validation evidence; no unsupported requested-resize flag, transition timeline, animation control, exact restore frame, or page-only global zoom control is carried into the v2 WOs.
- **Pending validation anchors:** all four are filename-reservation placeholders and contain no implementation or validation result; they do not satisfy future validation gates.
- **Comparison evidence sufficient:** `yes` for planning alignment; native smoke is intentionally unavailable and is not upgraded into runtime evidence.
- **Explicit project-policy gates checked:** standard mode, fresh-session separation, sequential/per-WO validation, required branch before implementation, configured Cargo/diff checks, and human approval at `review -> execute`.

## Decision

- **Outcome:** `PASS WITH WARNINGS`
- **Required next state:** `review` — record this review, clarify the evidence-gated no-freeze default and native observation dispositions, then satisfy branch and human-approval gates before any execution dispatch.
- **Human approval required:** `yes`
- **Approval/evidence:** The user approved conversion of v2 research into four work orders. The user explicitly confirmed that this review outcome and that authorization remain separate from human approval to implement. No implementation authorization is granted here.
- **Warning acceptance required:** `no` — warnings are non-blocking under the Forge design-review policy, but their evidence boundaries and required next states remain applicable.


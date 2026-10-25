# Design Review — Editor Geometry Corrections

## Review metadata

- **Review type:** `design`
- **Status:** `PASS WITH WARNINGS`
- **Reviewer:** Fresh-context Forge design reviewer
- **Reviewer session/agent ID:** `architect-fresh-context-editor-geometry-corrections-design-review-2026-10-25`
- **Date:** 2026-10-25
- **Initiative:** [`editor-geometry-corrections-v1.md`](../editor-geometry-corrections-v1.md:1)
- **Review file:** `design-review.md`
- **Reviewed artifacts:** [`editor-geometry-corrections-v1.md`](../editor-geometry-corrections-v1.md:1), [`WO-001-vertical-edge-clamping.md`](../work-orders/WO-001-vertical-edge-clamping.md:1), and [`WO-002-screenplay-width-and-leading.md`](../work-orders/WO-002-screenplay-width-and-leading.md:1)
- **Fresh-context statement:** This is an independent fresh-context design review. I inspected the process rules, project manifest, requirements, blueprints, the closed prior initiative and all of its work-order/validation artifacts, the follow-up initiative and both work orders, and current source evidence. I did not implement code, validate implementation, or silently rewrite the initiative scope.

## Comparison boundary and evidence inspected

The comparison boundary is the initiative's single observable outcome: document-edge Up/Down behavior must reach the true visual-line beginning/end, while fixed screenplay layout must expose intentional effective widths, right-edge behavior, line leading, and role-to-role spacing without changing semantic text or making viewport zoom a print-layout input.

Inspected:

- Forge lifecycle and design-review rules in [`00-overview.md`](../../../process/00-overview.md:27), [`02-initiative.md`](../../../process/02-initiative.md:159), and [`03-design-review.md`](../../../process/03-design-review.md:15).
- Project policy and gate configuration in [`project-manifest.md`](../../../project-manifest.md:7).
- Follow-up plan and work orders in [`editor-geometry-corrections-v1.md`](../editor-geometry-corrections-v1.md:3), [`WO-001-vertical-edge-clamping.md`](../work-orders/WO-001-vertical-edge-clamping.md:1), and [`WO-002-screenplay-width-and-leading.md`](../work-orders/WO-002-screenplay-width-and-leading.md:1).
- The closed predecessor plan, review, execution log, four work orders, and four validations under [`2026-10-25-ui-quality-of-life/`](../../2026-10-25-ui-quality-of-life/).
- Requirements in [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:36).
- Relevant technical boundaries in [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:38), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:78), [`technical-context.md`](../../../blueprints/technical-context.md:52), and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:35).
- Current source evidence in [`move_vertical()`](../../../../apps/scriptcraft/src/main.rs:671), [`PageProfile`](../../../../crates/scriptcraft-core/src/lib.rs:142), [`ScreenplayLayout::make_line()`](../../../../crates/scriptcraft-core/src/layout.rs:192), [`role_width()`](../../../../crates/scriptcraft-core/src/layout.rs:297), and [`LINE_HEIGHT`](../../../../crates/scriptcraft-core/src/layout.rs:11).

The predecessor's spacing validation is relevant evidence, not an approval of this follow-up: it verified the existing twelve-unit line box and the `450`-unit content box, but did not establish a separate role-measure/right-edge contract or an inter-element spacing contract. The predecessor's visual-line validation likewise confirms the current edge behavior was not covered as the requested same-line clamp.

## Diagnostic assessment

### Reported defect 1 — first/last visual-line vertical navigation

**Diagnosis: confirmed.** Current [`move_vertical()`](../../../../apps/scriptcraft/src/main.rs:671) identifies the current visual line from layout source ranges, but when `current_index` has no adjacent line it returns the unchanged selection at lines 697–700 after clearing the preferred goal. That is a boundary no-op, not a same-line edge clamp. The requested semantics are identifiable and compatible with the existing source mapping: Up at the first visual line targets that line's `source.start`; Down at the last visual line targets that line's `source.end`. This remains UTF-8-safe because the layout source range endpoints are valid boundaries.

### Reported defect 2 — screenplay width and vertical geometry

**Diagnosis: materially confirmed, with an important distinction.** The stored default profile has paper width `612`, left margin `90`, and right margin `72`, yielding a `450`-unit content box. With [`CHAR_WIDTH`](../../../../crates/scriptcraft-core/src/layout.rs:10) equal to `7`, the default action/scene-heading/note `55`-cell role measure produces `385` units, leaving `65` units between the role text edge and the content-box right edge. The paper-to-text effective right edge is therefore `137` units, not the stored `72`-unit page margin. This is evidence of a narrower derived role measure; it is not evidence that `PageProfile.margin_right` itself is incorrect.

The current layout also uses one twelve-unit line box for every line and places consecutive lines at twelve-unit y advances in [`ScreenplayLayout::make_line()`](../../../../crates/scriptcraft-core/src/layout.rs:192). It does not expose separate numerical same-element leading or inter-element role spacing. Therefore the spacing complaint is correctly framed as an underspecified geometry contract, but a visual impression alone cannot select the replacement values.

## Findings

### Blockers

- **None for design alignment.** Both defects are identifiable from current source evidence; both work orders are related to the initiative outcome; their ownership and execution order are understandable; and no work order contradicts the explicit exclusions or project policy. The unresolved numeric geometry decision is a design-readiness warning, not an impossible dependency, because WO-002 explicitly requires an approved geometry table before production changes.

### Warnings

#### EGC-DESIGN-W-001 — Edge-clamp acceptance should name the exact current-line oracle

- **Severity:** warning
- **Evidence:** The diagnosis and implementation plan in [`editor-geometry-corrections-v1.md`](../editor-geometry-corrections-v1.md:66) and [`WO-001-vertical-edge-clamping.md`](../work-orders/WO-001-vertical-edge-clamping.md:42) correctly distinguish an edge clamp from a boundary no-op. However, the acceptance wording in [`WO-001-vertical-edge-clamping.md`](../work-orders/WO-001-vertical-edge-clamping.md:56) should make explicit that the current visual line is selected from the layout source range, including a wrapped first or last line, and that only focus moves when Shift is held.
- **Recommendation:** During implementation, make the focused regression table explicit: first visual line/non-edge offset → same line `source.start`; last visual line/non-edge offset → same line `source.end`; wrapped first/last visual line; Unicode endpoints; repeated boundary presses; and Shift anchor preservation. Preserve the existing preferred-goal reset after the clamp.
- **Next state:** Note/warning only; retain WO-001 in `draft` until implementation is authorized. No scope expansion or requirements change is required.

#### EGC-DESIGN-W-002 — Width work must not relabel a role-measure gap as a page-margin defect

- **Severity:** warning
- **Evidence:** [`PageProfile`](../../../../crates/scriptcraft-core/src/lib.rs:142) stores paper and margin inputs, while [`role_width()`](../../../../crates/scriptcraft-core/src/layout.rs:297) derives role widths from content characters, indents, and role measures. The current `450`-unit content width and `385`-unit main-role measure support the initiative's observation, but they do not determine whether the product should fill the content box, retain a narrower screenplay measure, or revise a role measure.
- **Recommendation:** Require WO-002's geometry table to report, separately and numerically, paper bounds, stored margins, content bounds, each role's indent and text measure, the effective role right edge, and the residual gap to both the content edge and paper edge. Select one of the work order's approaches only after that table is reviewed. If a narrower measure is intentional, document it as such rather than changing `margin_right` to compensate.
- **Next state:** Warning; human design approval must cover the selected width interpretation before WO-002 production changes. Requirements may remain `verify unchanged` unless the final user-facing width becomes a newly frozen requirement; the relevant layout blueprints should record any changed durable contract.

#### EGC-DESIGN-W-003 — Leading and inter-element spacing need definitions and a numerical oracle

- **Severity:** warning
- **Evidence:** [`LINE_HEIGHT`](../../../../crates/scriptcraft-core/src/layout.rs:11) is currently `12`, and [`make_line()`](../../../../crates/scriptcraft-core/src/layout.rs:192) assigns that height and a twelve-unit baseline advance to every line. There is no distinct field or record for same-element leading, glyph-to-line-box clearance, or the gap between adjacent elements. [`WO-002-screenplay-width-and-leading.md`](../work-orders/WO-002-screenplay-width-and-leading.md:22) correctly asks for these distinctions, but its acceptance test still depends on an `approved geometry table` without defining the measured quantities that table must contain.
- **Recommendation:** Define terms before implementation without asserting an industry-standard value: `line_box_height`; `baseline_or_top_origin_advance` for consecutive wrapped lines of one element; `same_element_leading = line_advance - glyph_or_box_extent` where the chosen fixed-metric extent is stated; and `inter_element_gap(role_a, role_b) = next_element_first_line_origin - prior_element_last_line_origin - prior_line_box_height`. State whether the current baseline is `12` line-box height, `12` same-element advance, and `0` inter-element gap, or choose different approved values. Test representative same-element wraps separately from each relevant role transition/pair, including page breaks and keep-with-next behavior. Do not call any value industry-standard unless independently measured and explicitly approved as a product decision.
- **Next state:** Warning; record the numeric table and approval in WO-002 before production layout changes. Update the component/technical blueprints, and the feature blueprint if the shared screen/PDF contract changes; update requirements only if the behavior is intentionally frozen as a new user-facing requirement.

#### EGC-DESIGN-W-004 — WO-002 is coherent but has a broad shared-geometry validation boundary

- **Severity:** warning
- **Evidence:** [`WO-002-screenplay-width-and-leading.md`](../work-orders/WO-002-screenplay-width-and-leading.md:22) spans measurement, role widths, all seven roles, screen layout, caret geometry, pagination, PDF transforms, source mappings, and page-capacity regressions. These are all subordinate to the single geometry defect outcome and the work order has a single owner/order, but the acceptance wording in [`WO-002-screenplay-width-and-leading.md`](../work-orders/WO-002-screenplay-width-and-leading.md:62) combines multiple observable assertions behind the approved table.
- **Recommendation:** Keep WO-002 as one work order, but structure its fixture/oracle into independently assertable groups: width/margin separation; same-element wrapped-line geometry; inter-element role-pair spacing; caret/source mapping; pagination/page capacity; and screen/PDF parity. Record which groups are verification-only versus changed-contract tests. Do not split merely for size, and do not broaden it into arbitrary formatting.
- **Next state:** Warning only; the work order is independently testable after the numeric geometry table is approved.

#### EGC-DESIGN-W-005 — Durable-document impact is directionally correct but conditional decisions must be recorded

- **Severity:** warning
- **Evidence:** WO-001 classifies both durable domains as `verify unchanged`, which matches transient navigation behavior. WO-002 classifies requirements as conditional `update`/`verify unchanged` and blueprint impact as `update`, naming [`technical-context.md`](../../../blueprints/technical-context.md:1) and [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:1), with conditional [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:1). This is consistent with the Forge durable-document rules in [`00-overview.md`](../../../process/00-overview.md:141), but the final choice cannot be left implicit after implementation.
- **Recommendation:** At WO-002 completion, compare the final diff with the classification: update the durable technical contract for a changed width/spacing/profile behavior; explicitly verify requirements unchanged if the existing criteria remain sufficient; update the feature blueprint if screen and PDF share changed semantics; and do not add industry-standard claims. Record the exact numeric contract and evidence links in the affected blueprint documents.
- **Next state:** Warning; no scope change. Validation must check the classification against the actual diff.

### Notes

- **EGC-DESIGN-N-001 — Work-order independence is sound.** WO-001 changes only app-level visual-line boundary behavior and explicitly avoids layout/profile files. WO-002 changes shared geometry and is ordered after WO-001 because it consumes the already-stable visual-line/source mapping, but its scope remains independently reviewable and testable. The dependency is conservative rather than execution-blocking.
- **EGC-DESIGN-N-002 — Traceability is adequate.** WO-001 carries [`REQ-WRITING-LOOP-002`](../../../requirements/features/screenplay-writing-loop.md:52) and [`AC-WRITING-002.4`](../../../requirements/features/screenplay-writing-loop.md:61), with desktop/core blueprint references. WO-002 carries [`REQ-WRITING-LOOP-002`](../../../requirements/features/screenplay-writing-loop.md:52), [`AC-WRITING-002.1`](../../../requirements/features/screenplay-writing-loop.md:58), [`AC-WRITING-002.2`](../../../requirements/features/screenplay-writing-loop.md:59), and [`NFR-WRITING-003`](../../../requirements/features/screenplay-writing-loop.md:86), with layout, feature, and technical blueprint references. The cited boundaries are applicable and no unrelated requirement is being used to expand scope.
- **EGC-DESIGN-N-003 — Prior initiative evidence supports, but does not settle, the follow-up.** The predecessor's [`WO-004-screenplay-spacing-contract.md`](../../2026-10-25-ui-quality-of-life/work-orders/WO-004-screenplay-spacing-contract.md:22) established the current `612 × 792`, `72/72/90/72`, `450 × 648`, `55`-line, and twelve-unit baseline. Its tests prove current structural parity, not that a `55 × 7 = 385` role measure is the intended usable width or that zero inter-element gap is the desired screenplay contract.

## Gate assessment

- **Hard-stop design blocker:** none.
- **Design readiness:** sufficient for an aligned `PASS WITH WARNINGS` result. The defects are correctly diagnosed at the level needed to authorize bounded planning; the numeric geometry decision is explicitly reserved for design approval rather than guessed.
- **Execution authorization:** not granted by this review. The project manifest requires human design approval at the `review → execute` gate.
- **Validation boundary:** retain one fresh validator per work order. WO-001 needs focused edge-clamp/source-range regressions. WO-002 needs a recorded geometry table and separate assertions for stored margins, role measures, same-element leading, inter-element spacing, deterministic pagination/source mappings/caret rectangles, and screen/PDF parity. Native visual evidence remains optional under current project policy and must not be claimed if unavailable.

## Verdict and required next state

- **Verdict:** `PASS WITH WARNINGS`
- **Outcome:** `pass` — no design-alignment blocker remains; the warnings identify precision and approval work required before implementation, not unrelated scope or an impossible dependency.
- **Required next state:** Keep the initiative in `review`; obtain recorded human design approval for the complete follow-up, with explicit approval of WO-002's numeric width interpretation and leading/spacing table. Keep both work orders in `draft` until that gate is satisfied. Do not implement around the warnings and do not treat this review as implementation or validation evidence.
- **Human approval required:** `yes`, for the `review → execute` gate under project policy. Approval should cover the two-work-order scope, the first/last visual-line clamp semantics, the distinction between stored right margin and role measure, and the numerical definitions/values for same-element leading and inter-element role spacing. It need not claim industry-standard parity.

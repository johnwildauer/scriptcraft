# Initiative: Editor Geometry Corrections

## Initiative metadata

- **Phase:** `closeout`
- **Status:** `closed`
- **Owner/orchestrator:** Forge orchestrator
- **Created:** 2026-10-25
- **Updated:** 2026-10-25
- **Authoritative version:** `v1`
- **Initiative mode:** `standard`
- **Mode requested by:** `user`
- **Mode confirmation:** Standard mode remains the project default; the user reported concrete follow-up defects from the closed UI Quality of Life initiative.
- **Rapid-mode eligibility:** `not eligible` — the work changes caret navigation and shared screenplay page geometry/spacing contracts.
- **Base/default branch:** `master`
- **Initiative branch:** `dev/2026-10-25-editor-geometry-corrections`
- **Branch policy:** `required`
- **Branch naming convention:** `dev/YYYY-MM-DD-short-slug`
- **Branch capability/evidence:** `git switch -c dev/2026-10-25-editor-geometry-corrections` succeeded on 2026-10-25.
- **Validation cadence:** `per work order`
- **Phase transition evidence:** `draft → plan` on 2026-10-25: the user reported two concrete regressions after the prior initiative closed; this focused plan and two ordered WOs were created. `plan → review` on 2026-10-25: the work orders and fresh-context design review are present; the review returned `PASS WITH WARNINGS` with no hard-stop alignment blocker. `review → execute` on 2026-10-25: the user approved WO-001 and supplied [`format-rules.md`](format-rules.md) as the authoritative reference for WO-002's geometry investigation. `execute → closeout` on 2026-10-25: WO-001 completed after fresh validation; WO-002 completed through the WO-002a/WO-002b fix chain after WO-002 and WO-002a hard-stop validations were resolved by fresh validation.
- **Direct-translation confirmation:** `not needed` — the work orders are a focused translation of the reported boundary-navigation and screenplay-geometry defects, with metric choices intentionally left for design review.

## Standard artifact locations

- **Authoritative plan:** `editor-geometry-corrections-v1.md`
- **Design review:** [`reviews/design-review.md`](reviews/design-review.md)
- **Validations:** `validations/validation-[work-order-id].md`
- **Execution log:** [`execution-log.md`](execution-log.md)
- **Work orders:** [`work-orders/`](work-orders/)

## Outcome

Make vertical caret navigation obey the requested document-editor edge semantics and make the screenplay page geometry visibly match the intended screenplay profile: usable text width, credible right margin, and role-aware leading/spacing that is evident between lines and element types.

## Initiative boundary and sizing

- **Parent roadmap or product goal:** Dependable screenplay writing loop in [`../../requirements/features/screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md).
- **Initiative type:** `product MVP stage`
- **Single observable outcome:** A writer can navigate to the true beginning/end of the document with Up/Down and can visually distinguish the intended screenplay margins, line leading, and role-to-role spacing in screen layout without changing semantic text or making viewport zoom affect print geometry.
- **Validation boundary:** Fresh design review, then separate fresh validation for each WO; headless caret/layout geometry tests plus a focused desktop/rendered-page smoke when window interaction is available.
- **Split assessment:** `keep as one initiative`
- **If split, traceability and dependencies:** Not applicable. Navigation boundary semantics and screenplay geometry are related editor correctness fixes but have distinct implementation/validation boundaries and remain ordered WOs.

## Scope

### In scope

- Correct Up at the first visual line to move the caret to that line's beginning, and Down at the last visual line to move it to that line's end, while retaining ordinary adjacent-line preferred-column behavior.
- Re-measure the relationship between paper width, margins, content width, nominal columns, role measures, and the rendered fixed metric so the effective right margin is not unintentionally oversized.
- Define and implement role-aware vertical geometry/leading, including visible line height and any deliberate inter-element spacing, while preserving deterministic pagination and caret/source mappings.
- Add regression tests that distinguish page margins from role text measures and distinguish same-element line leading from inter-element spacing.

### Out of scope

- Full Final Draft/Word typography parity, arbitrary per-element formatting controls, font substitution databases, or new page profiles without approval.
- Reworking the viewport zoom behavior already delivered by [`ui-quality-of-life-v1.md`](../2026-10-25-ui-quality-of-life/ui-quality-of-life-v1.md).
- Changing Enter/Tab semantics or long-document persistence behavior except where a geometry regression directly affects caret/layout correctness.

## Context and motivation

The closed UI Quality of Life initiative added visual-line navigation and verified the existing fixed metrics, but user smoke testing exposed two shortcomings. In [`move_vertical()`](../../apps/scriptcraft/src/main.rs:671), pressing Up/Down at the document's first/last visual line returns the unchanged selection and clears the preferred goal rather than clamping to the beginning/end of that line. In [`PageProfile`](../../crates/scriptcraft-core/src/lib.rs:142) and [`make_line()`](../../crates/scriptcraft-core/src/layout.rs:192), the nominal 55-column role measure uses only 385 of the 450-unit content width, leaving an effective right-side gap larger than the stored page margin; all lines also use a single 12-unit baseline/box metric with no visible role-aware leading or inter-element spacing.

The prior analysis explicitly treats margins, indents, line spacing, and role rules as profile/layout behavior, and recommends resolving role text boxes before line breaking. The follow-up must therefore measure first and avoid confusing a renderer scale issue with a page-profile or role-measure issue.

## Initial findings and hypotheses

- **Confirmed navigation defect:** the current `target_index` is `None` at the first/last line and returns the existing selection. The requested behavior requires a same-line edge clamp: Up at the first visual line → `source.start`; Down at the last visual line → `source.end`.
- **Likely width defect:** the physical content box is 450 units (`612 - 90 - 72`), but the main role width is capped at 55 cells × 7 units = 385 units. The remaining 65 units are not the stored 72-unit page margin and can make the right edge feel too far away. This is a derived text-measure gap, not proof that the page's `margin_right` field is wrong.
- **Likely spacing defect:** `LINE_HEIGHT` is 12 and the app paints a roughly 10px monospace glyph at the same scaled baseline cadence. There is no role-specific leading or deliberate block spacing in the current layout record, so “space between lines” cannot be observed as a separate contract. The fix must define whether leading is inside each line box, between line boxes, between screenplay elements, or all three.
- **Profile ambiguity:** screenplay conventions commonly use a 1-inch right margin and larger left margin, but the exact ScriptCraft fixed metric, font cell width, and role measures must be selected from measured geometry rather than copied from a generic editor assumption.

## Proposed work-order plan

1. [`WO-001-vertical-edge-clamping.md`](work-orders/WO-001-vertical-edge-clamping.md) — correct first/last visual-line Up/Down edge behavior and add focused navigation regressions.
2. [`WO-002-screenplay-width-and-leading.md`](work-orders/WO-002-screenplay-width-and-leading.md) — measure and correct derived text width/right margin and introduce the approved role-aware leading/spacing contract across screen layout, caret geometry, pagination, and PDF parity.

WO-001 is independent and should execute first because it is a narrow semantic/UI correction. WO-002 depends on the stable visual-line/source mapping and then changes shared layout geometry, so it receives its own design and validation boundary.

## Risks and decisions

- **Risk:** Increasing role text width may alter wrapping/page counts and expose pagination assumptions. **Mitigation:** establish golden fixtures before/after, keep paper/margin inputs explicit, and validate source/caret mappings.
- **Risk:** “Leading” may mean line box height, font glyph height, or blank space between screenplay elements. **Mitigation:** define each term numerically in WO-002 before implementation and test same-element versus inter-element spacing separately.
- **Risk:** Changing profile geometry can invalidate existing native projects or PDF expectations. **Mitigation:** determine whether the change is a derived default, a schema/profile change, or a renderer-only correction; update durable blueprints only at the correct abstraction boundary.
- **Decision resolved:** The user approved the proposed WO-002 oracle from [`format-rules.md`](format-rules.md), including inch-defined margins/role boxes, one 12-unit blank-line-equivalent gap at every element boundary, Courier-family 12-point target metrics, and permission to replace the provisional 12-unit advance if measured font geometry requires it.

### WO-002 design notes — proposed geometry oracle

The design-only amendment in [`WO-002-screenplay-width-and-leading.md`](work-orders/WO-002-screenplay-width-and-leading.md)
now consolidates [`format-rules.md`](format-rules.md) with current source
geometry. The proposed oracle treats layout units as typographic points
(`1/72` inch), retains US Letter at `612 × 792`, targets margins
`108/72/72/72`, and records the target role boxes/edges and spacing semantics
without changing source code or WO-001. It explicitly distinguishes the stored
right margin from a role's text measure and labels the current fixed metrics as
reconciliation evidence rather than approved target behavior.

The approved font assumption is a 12-point Courier-family face, preferring
Courier or Courier New as available on the system. The implementer may choose a
deterministic point inside the approved character range (agent-default: center
the cue at 3.6 inches), notes inherit action geometry, every element boundary
receives the approved blank-line-equivalent gap, and measured installed-font
metrics may replace the provisional 12-unit advance when the tests demonstrate
that need. This amendment authorizes WO-002 implementation after its fresh
dispatch gate.

The approved work also requires [`format-rules.md`](format-rules.md) to remain a
durable, human-readable reference for the format contract and to document the
future project-override seam. Project overrides are not a new UI feature in this
WO, but the persisted profile must not make a future override impossible.

## Scoped human approval — WO-002

- **Approval status:** `approved for execution`
- **Approver:** User
- **Approval date:** 2026-10-25
- **Approval scope:** WO-002 width, margins, role boxes, leading, inter-element spacing, Courier-family target, inch-defined layout conversion, and durable format-rule reference/override seam.
- **Approval evidence:** User approved the proposed oracle and required a document where these rules remain saved and can be referenced by future project overrides.

## Requirements and blueprint references

- **Requirements:** [`screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md) — `REQ-WRITING-LOOP-001`/`AC-WRITING-002.4`, `REQ-WRITING-LOOP-002`/`AC-WRITING-002.1`–`002.2`, and `NFR-WRITING-003`.
- **Blueprints:** [`screenplay-core.md`](../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../blueprints/features/screenplay-writing-loop.md), [`technical-context.md`](../../blueprints/technical-context.md), and [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md).
- **Research:** [`06-screenplay-mvp-project-structure.md`](../../.analysis/06-screenplay-mvp-project-structure.md) and [`02-layout-pagination-rendering.md`](../../.analysis/stage2/02-layout-pagination-rendering.md).
- **User-provided format reference:** [`format-rules.md`](format-rules.md) — 12-point Courier-family target, US Letter 1.5/1.0/1.0/1.0-inch margins, role indents, single spacing within elements, and blank-line separation between elements.

## Design readiness checklist

- [x] Scope and exclusions are explicit.
- [x] The initiative has one observable outcome and one named validation boundary.
- [x] The advisory split assessment is recorded.
- [x] Requirements and blueprints are linked.
- [x] Work orders are independently executable and dependency-ordered.
- [x] Every work order carries a durable-document impact classification.
- [x] Initiative mode metadata and branch decision are recorded.
- [x] Validation cadence and policy gates are identified.
- [ ] Fresh design review has passed.
- [ ] Required human design approval has been obtained.

## Closeout follow-ups

- Native visual interaction remained unavailable in every validator environment; repeat the focused desktop smoke when an interactable window-capable environment is available. Owner: Forge orchestrator.
- The format profile now has a bounded persisted override seam, but an override editor remains future scope. Owner: product/technical owner.

## Closeout record

- **Closeout status:** `closed`
- **Closeout date:** 2026-10-25
- **Closeout evidence:** All authorized work is complete: WO-001 passed fresh validation with warnings; WO-002 passed through WO-002a and WO-002b fix-chain validation; required automated checks passed; no further fix work order is required.

## Scoped human approval — WO-001

- **Approval status:** `approved for execution`
- **Approver:** User
- **Approval date:** 2026-10-25
- **Approval scope:** WO-001 vertical edge clamping only. WO-002 is not yet approved for implementation; its geometry work must use [`format-rules.md`](format-rules.md) as a reference and retain a separate implementation/validation gate.
- **Approval evidence:** User instruction: “Approve WO-001, and for WO-002, use the `format-rules.md` document for reference.”

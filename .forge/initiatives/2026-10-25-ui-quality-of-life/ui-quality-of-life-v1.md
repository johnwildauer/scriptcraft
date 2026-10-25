# Initiative: UI Quality of Life

## Initiative metadata

- **Phase:** `closeout`
- **Status:** `closed`
- **Owner/orchestrator:** Forge orchestrator
- **Created:** 2026-10-25
- **Updated:** 2026-10-25
- **Authoritative version:** `v1`
- **Initiative mode:** `standard`
- **Mode requested by:** `user`
- **Mode confirmation:** Standard mode is the project default; the user requested a new initiative and work orders, not the rapid path.
- **Rapid-mode eligibility:** `not eligible` — the work spans UI presentation, editing semantics, navigation, pagination, persistence safety, and several consequential implementation decisions.
- **Base/default branch:** `master` (current unborn branch)
- **Initiative branch:** `dev/2026-10-25-ui-quality-of-life`
- **Branch policy:** `required`
- **Branch naming convention:** `dev/YYYY-MM-DD-short-slug`
- **Branch capability/evidence:** `git switch -c dev/2026-10-25-ui-quality-of-life` succeeded on 2026-10-25.
- **Reduced-capability fallback or opt-out evidence:** Not applicable.
- **Branch creation failure:** `none`
- **Validation cadence:** `per work order`
- **Phase transition evidence:** `draft → plan` on 2026-10-25: the user asked to formalize the five requested investigations and author their work orders; this plan and the four ordered WOs were created. `plan → review` on 2026-10-25: all four work orders and the fresh-context design review artifact are present. `review → execute` on 2026-10-25: the design review returned `PASS WITH WARNINGS`, the initialization manifest blocker was reconciled, and the user approved proceeding with incremental implementation and validation. `execute/validate → closeout` on 2026-10-25: WO-001 through WO-004 each completed after separate fresh validation with `PASS WITH WARNINGS`; no fix work orders were required.
- **Direct-translation confirmation:** `requested and confirmed` — the work orders directly translate the five user-authorized problem areas, with implementation choices left to design review and validation.

## Standard artifact locations

- **Authoritative plan:** `ui-quality-of-life-v1.md`
- **Design review:** [`reviews/design-review.md`](reviews/design-review.md)
- **Validations:** `validations/validation-[work-order-id].md`
- **Execution log:** [`execution-log.md`](execution-log.md)
- **Work orders:** [`work-orders/`](work-orders/)

## Outcome

Make ScriptCraft's screenplay editing surface feel dependable for ordinary writing: the page is readable and centered at useful scale, role transitions and vertical navigation follow screenplay-editor expectations, long documents remain scrollable and valid, and layout spacing is measured and regression-tested.

## Initiative boundary and sizing

- **Parent roadmap or product goal:** The dependable screenplay writing loop in [`../../requirements/features/screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md), especially `REQ-WRITING-LOOP-001` through `REQ-WRITING-LOOP-003`.
- **Initiative type:** `product MVP stage`
- **Single observable outcome:** A writer can edit a multi-page screenplay in the desktop surface with readable centered pages, predictable Enter/Tab and line navigation, visible caret-following scroll, stable model/pagination behavior, and verified screenplay line spacing.
- **Validation boundary:** Fresh design review, then one fresh validator per WO using headless tests plus focused desktop/manual evidence; the final boundary is a multi-page typing/paste/scroll smoke path with stable save/reopen and structural layout assertions.
- **Split assessment:** `keep as one initiative`
- **If split, traceability and dependencies:** Not applicable. The requested changes share the same editor/layout boundary and are independently reviewable as ordered WOs.

## Scope

### In scope

- Investigate and correct the desktop page viewport's default scale, horizontal centering, resize behavior, vertical scrolling, and caret visibility.
- Implement the requested Final Draft-style Enter transition matrix and the requested Tab transition matrix, including the blank-character condition.
- Implement line-oriented Up/Down navigation over wrapped visual lines with a stable preferred horizontal goal where appropriate, including page boundaries and selection extension.
- Reproduce long pasted/multi-page content through the model, layout, UI scroll, and native JSON validation paths; identify and fix the source of `model (invalid)` rather than masking it.
- Verify default screenplay margins and line spacing against the fixed profile, keeping viewport zoom separate from print pagination.
- Add focused regressions and bounded manual smoke evidence for the above.

### Out of scope

- General WordCraft feature parity, arbitrary rich formatting, font databases, DOCX/FDX interoperability, or new planning features.
- Incremental pagination or performance architecture before a measured long-document failure requires it.
- Persisting caret coordinates, scroll position, zoom, or other transient UI state in the native project.
- Redesigning the entire visual theme, toolbar, file workflow, or desktop framework.

## Context and motivation

The current implementation has a semantic core and a painted egui page surface, but the observed editing behavior is still MVP-level. The page width is capped in the UI and the surface is laid out from the scroll area's left edge; the core Enter policy currently sends Dialogue to Parenthetical; the app's vertical movement changes element index rather than visual line; and long text exercises the bounded model/layout/persistence paths without a dedicated multi-page acceptance fixture. The fixed layout uses integer metrics, so line spacing must be treated as a measurable contract rather than a visual guess.

WordCraft research is used only for boundary and interaction lessons: its canvas separates document layout geometry from viewport pan/zoom, its caret API maps semantic positions to current line geometry, and its vertical navigation retains a page-space horizontal goal. Those ideas inform investigation, but WordCraft source is not current ScriptCraft implementation evidence.

## Exploration scratchpad

### Initial findings

- The desktop page is painted by [`live_document_surface()`](../../apps/scriptcraft/src/main.rs:366), where the page width is currently capped at `360.0` and the scale is derived from that fixed width. Pages are placed sequentially in a vertical scroll area without a centered content column.
- The app currently handles Enter in [`key_event()`](../../apps/scriptcraft/src/main.rs:512), while the core policy in [`transition()`](../../crates/scriptcraft-core/src/editing.rs:427) maps Scene Heading→Action, Action→Action, Character→Dialogue, Dialogue→Parenthetical, Parenthetical→Dialogue, Transition→Scene Heading, and Note→Note. The requested policy changes only Dialogue's Enter target among the existing roles and adds Tab behavior.
- Up/Down currently use element indexes in [`move_vertical()`](../../apps/scriptcraft/src/main.rs:621), so wrapped lines inside one element cannot be addressed. Layout already emits source ranges and rectangles through [`LayoutLine`](../../crates/scriptcraft-core/src/layout.rs:29), which is a suitable source for visual-line movement.
- The core validates projects before layout in [`ScreenplayLayout::build()`](../../crates/scriptcraft-core/src/layout.rs:86) and before serialization in [`Project::to_json()`](../../crates/scriptcraft-core/src/lib.rs:218). It bounds elements/text/JSON with [`MAX_ELEMENTS`](../../crates/scriptcraft-core/src/lib.rs:16), [`MAX_TEXT_BYTES`](../../crates/scriptcraft-core/src/lib.rs:17), and [`MAX_JSON_BYTES`](../../crates/scriptcraft-core/src/lib.rs:15). The initiative must determine whether the reported invalid state is bad data, a UI error path, an overly small limit, or stale/corrupt derived state.
- The fixed layout currently exposes `LINE_HEIGHT = 12` and places each line at `margin_top + line_number * LINE_HEIGHT` in [`ScreenplayLayout::make_line()`](../../crates/scriptcraft-core/src/layout.rs:192). There is no explicit line-spacing field in [`PageProfile`](../../crates/scriptcraft-core/src/lib.rs:142), so the investigation must distinguish desired screenplay line spacing from a mere renderer scale change.
- WordCraft's documented canvas flow keeps page-space layout independent of viewport zoom and maps caret/hit testing through layout. Its vertical navigation uses a retained horizontal goal rather than simply clamping the byte offset in the next paragraph. See [`07-wordcraft-gui-system.md`](../../.analysis/07-wordcraft-gui-system.md) and [`02-layout-pagination-rendering.md`](../../.analysis/stage2/02-layout-pagination-rendering.md).

### Initial thoughts on the five requests

1. **Readable, centered page:** Treat page scale and page placement as UI presentation state. Default scale should be 1.6 times the current effective scale, but it must be clamped to the available viewport and remain independent of print geometry. A centered inner column with minimum horizontal padding should allow a larger window to center the page while a narrow window remains usable. Scrolling should be driven by the caret's current page/line rectangle after edits and key movement, not by saved coordinates.
2. **Enter/Tab transitions:** Put both matrices in core editing policy so headless tests, UI input, and future automation share one semantic contract. Enter should split the current element and assign the target role. Tab should be an explicit operation: it should transition the current element without inserting a tab character, and the Character→Transition rule should inspect whether the character element is blank. The design review should resolve whether non-listed Tab cases are no-ops, cycle through a broader policy, or insert literal text; the initial safe default is no-op unless the requested matrix authorizes a transition.
3. **Long documents and `model (invalid)`:** First reproduce with controlled fixtures at increasing element count, text size, wrapped-line count, page count, and pasted newline patterns. Then trace every validation boundary and error display. The likely fixes are to preserve the last valid model on failed edits, make layout and scroll tolerate many pages, and add regression coverage; increasing limits or changing schema is only justified by evidence and must remain bounded.
4. **Line spacing:** Separate semantic page/profile line spacing from UI zoom. Compare the fixed line height, role keep rules, top/bottom margins, rendered text box, caret height, and PDF output. Add structural assertions for adjacent line baselines and content bounds before changing constants; if the current 12-unit metric is wrong, change the profile/layout contract once and test screen/PDF agreement.
5. **Up/Down by visual line:** Use layout source ranges to find the visual line containing the caret, retain a preferred horizontal column/coordinate during repeated vertical movement, and map that goal into the adjacent visual line even when it belongs to another element or page. Shorter lines clamp to their end; the first/last document line clamp to the start/end as requested. Shift should preserve anchor/focus while movement uses the same visual-line target.

### Questions and hypotheses

- The exact effective current scale and the desired meaning of “1.6x larger” need to be measured from the current window/page geometry rather than assumed to mean a 1.6 constant.
- The UI must decide whether Tab on a non-listed role is consumed as a no-op or remains available as literal insertion; no literal tab is currently part of the semantic screenplay contract.
- The `model (invalid)` report may be a stale app error string rather than a model corruption report; reproduction and error-path instrumentation are required before choosing a fix.
- A preferred vertical navigation goal likely belongs in transient app/session state, not the canonical project JSON.

### Options considered

| Option | Benefits | Costs/risks | Current direction |
|---|---|---|---|
| Make the page itself responsive to viewport width | Simple and readable | Could make print geometry depend on window size and break hit testing | Rejected for print; use a UI transform around fixed layout |
| Add a centered viewport/content column around fixed pages | Preserves fixed layout and supports large windows | Requires careful scroll-to-caret and pointer coordinate conversion | Selected |
| Implement Up/Down by element index with offset clamping | Small code change | Fails on wrapped lines and page boundaries | Rejected |
| Implement Up/Down from layout visual-line source ranges | Correct wrapped-line behavior and shared caret geometry | Needs transient preferred-goal state and layout access | Selected |
| Raise all model limits immediately | May postpone the symptom | Can increase allocation risk and hide the actual invalid-state cause | Rejected pending reproduction |
| Reproduce, validate at every boundary, then adjust one bounded contract if needed | Preserves safety and explains the failure | Requires fixtures and diagnostics | Selected |

### Decisions and discussion notes

- The initiative is standard, sequential, and per-work-order validation applies.
- The requested role matrices are authoritative product direction; design review clarified the unspecified Tab fallback as a consumed no-op and retained the preferred vertical goal as transient.
- WordCraft is a research reference for separation of model/layout/viewport and caret navigation, not a dependency or compatibility target.
- The design-review warnings are accepted as bounded execution follow-ups: W-002 carries the fallback/reset table, W-004 carries the numeric spacing baseline and feature-blueprint boundary, and parent requirement traceability is explicit in the work orders.

### Next investigation or conversation focus

- Dispatch WO-001 to a fresh implementer, then validate it with a separate fresh validator before dispatching WO-002.

## Requirements and blueprint references

- **Requirements:** [`screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md) — `REQ-WRITING-LOOP-001`/`AC-WRITING-001.3`–`001.5`, `REQ-WRITING-LOOP-002`/`AC-WRITING-002.1`–`002.4`, `REQ-WRITING-LOOP-003`/`AC-WRITING-003.1`–`003.3`, and `NFR-WRITING-001`–`003`.
- **Blueprints:** [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md), [`screenplay-core.md`](../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../blueprints/features/screenplay-writing-loop.md), and [`technical-context.md`](../../blueprints/technical-context.md).
- **Research:** [`07-wordcraft-gui-system.md`](../../.analysis/07-wordcraft-gui-system.md), [`02-layout-pagination-rendering.md`](../../.analysis/stage2/02-layout-pagination-rendering.md), and [`05-testing-safety-portability.md`](../../.analysis/stage2/05-testing-safety-portability.md).

## Proposed design

Keep the semantic model and fixed print layout authoritative. Add a presentation layer in the desktop app for default zoom, centered page placement, viewport scrolling, and caret reveal. Add explicit Enter/Tab policy and visual-line navigation at the core/session boundary, with the app translating key events and retaining only transient navigation goals. Make long-document handling an evidence-led hardening pass across validation, layout, error reporting, and scroll behavior. Treat line spacing as a fixed layout/profile input and prove it with structural geometry tests and a focused desktop/PDF comparison.

## Work-order plan

1. [`WO-001-centered-page-viewport.md`](work-orders/WO-001-centered-page-viewport.md) — make the page 1.6x more readable by default, center it responsively, and keep the caret visible while scrolling.
2. [`WO-002-role-keyboard-and-line-navigation.md`](work-orders/WO-002-role-keyboard-and-line-navigation.md) — implement the Enter/Tab matrices and visual-line Up/Down movement with headless and UI regressions.
3. [`WO-003-long-document-validity.md`](work-orders/WO-003-long-document-validity.md) — reproduce and harden multi-page paste/edit/layout/persistence behavior, including the `model (invalid)` path.
4. [`WO-004-screenplay-spacing-contract.md`](work-orders/WO-004-screenplay-spacing-contract.md) — verify margins and line spacing across layout, rendering, caret geometry, and export, changing the fixed contract only when evidence requires it.

Execution is sequential by default. WO-001 establishes viewport geometry used by later desktop smoke. WO-002 depends on stable layout source ranges and should precede long-document caret stress. WO-003 depends on the viewport and keyboard paths for end-to-end reproduction. WO-004 can inspect the same layout code but remains last so its geometry assertions cover any preceding caret/page changes.

## Risks and decisions

- **Risk:** A larger page can become unusable on small windows or cause pointer/caret coordinate drift. **Mitigation:** clamp scale, preserve fixed page units, test resize and hit testing at multiple viewport sizes.
- **Risk:** Tab semantics are underspecified outside the requested transitions. **Mitigation:** default unspecified cases to no-op pending design review; never silently insert tab characters into canonical screenplay text.
- **Risk:** Increasing limits could turn a corruption symptom into an allocation problem. **Mitigation:** reproduce first, preserve bounded validation, add hostile-size tests, and record any limit change as a blueprint impact.
- **Risk:** Visual-line navigation can be wrong if layout source ranges and caret mapping disagree. **Mitigation:** test wrapped Unicode text, cross-element/page movement, caret-hit agreement, and repeated Up/Down goal retention.
- **Decision resolved:** Unlisted Tab cases are consumed no-ops; they do not insert literal tab text, create undo entries, or mutate the model. **Owner/evidence:** Forge orchestrator agent-default accepted under the user's guided approval to proceed; recorded in WO-002.
- **Decision resolved:** Preferred vertical goal is retained across successful repeated Up/Down moves, and reset on horizontal movement, edits, pointer selection, Home/End, role changes, undo/redo, or a no-op boundary move. **Owner/evidence:** Forge orchestrator agent-default recorded in WO-002 for implementation/validation.
- **Decision resolved:** Initialization status was reconciled to `complete` with warning follow-ups in [`project-manifest.md`](../../project-manifest.md:9), removing the design-review execution hard stop.

## Design readiness checklist

- [x] Scope and exclusions are explicit.
- [x] The initiative has one observable outcome and one named validation boundary.
- [x] The advisory split assessment is recorded.
- [x] Requirements and blueprints are linked.
- [x] Work orders are independently executable and dependency-ordered.
- [x] Every work order carries a durable-document impact classification.
- [x] Initiative mode metadata is recorded.
- [x] Branch decision and phase transition evidence are recorded.
- [x] Validation cadence is recorded.
- [x] Applicable validation evidence boundary and policy gates are identified.
- [x] Fresh design review has passed.
- [x] Required human design approval has been obtained — user instruction on 2026-10-25: “let's review these (they look decent to me) and proceed with implementing and validating the execution incrementally”.

## Closeout follow-ups

- Native desktop interaction and visual inspection remained unavailable to implementers/validators; retain this as an accepted warning and repeat the manual smoke when a window-capable validation environment is available. Owner: Forge orchestrator; trigger: next desktop-capable session.
- Add a single combined failed-save/reopen long-document fixture in a future hardening pass; existing save-failure and WO-003 long-document coverage passed independently. Owner: persistence maintainer; trigger: next persistence/recovery initiative.
- Consider measured performance work only after a real long-script benchmark demonstrates a need; incremental pagination remains explicitly out of scope here. Owner: product/technical owner.

## Closeout record

- **Closeout status:** `closed`
- **Closeout date:** 2026-10-25
- **Closeout evidence:** All four work orders are `complete`; each has a separate fresh validation artifact with `PASS WITH WARNINGS`; required automated checks passed across the implementation loop; no fix work orders were required; native visual interaction limitations are explicitly recorded as follow-ups.
- **Git evidence:** The initiative branch was committed and merged into `master`; exact commit and merge results are recorded in the final execution-log entry.

## Human design approval — execution authorized

- **Approval status:** `approved for execution`
- **Approver:** User
- **Approval date:** 2026-10-25
- **Approval scope:** The complete UI Quality of Life initiative and its four ordered work orders, executed incrementally with a fresh validation after each work order.
- **Approval evidence:** User explicitly requested review followed by incremental implementation and validation; the fresh design review returned `PASS WITH WARNINGS` with no design-alignment blocker.

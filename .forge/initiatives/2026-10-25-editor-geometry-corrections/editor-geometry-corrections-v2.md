# Initiative: Editor Geometry and Writing Responsiveness Corrections

## Initiative metadata

- **Phase:** `execute`
- **Status:** `active`
- **Owner/orchestrator:** Forge orchestrator
- **Created:** `2026-10-26`
- **Updated:** `2026-10-26`
- **Authoritative version:** `v2`
- **Predecessor:** [`editor-geometry-corrections-v1.md`](editor-geometry-corrections-v1.md)
- **Initiative mode:** `standard`
- **Mode requested by:** `user`
- **Mode confirmation:** Standard mode remains the project default; this follow-up combines related screenplay geometry, editing affordance, and measured editor responsiveness corrections.
- **Rapid-mode eligibility:** `not eligible` — the work changes shared layout/profile contracts, semantic editing policy, pointer hit testing, and performance-sensitive rendering.
- **Base/default branch:** `master`
- **Initiative branch:** `dev/2026-10-26-editor-geometry-corrections-v2`
- **Branch policy:** `required`
- **Branch capability/evidence:** Initiative branch capability remains to be verified before implementation dispatch.
- **Validation cadence:** `per work order`
- **Phase transition evidence:** `draft → plan` on 2026-10-26: the user reported spacing, font, caret, hit testing, role normalization, parenthetical, and long-document responsiveness defects after v1. `plan → review` on 2026-10-26: six work orders and a fresh design review were created; the first review returned blocked on ledger/handoff precision, and the plan/work orders were reconciled with the decisions below. `review → execute` on 2026-10-26: fresh reconciliation review returned `PASS WITH WARNINGS`, the user approved the reconciled v2 design, all six work orders, exact oracles, and sequential order; branch capability verification remains the execution precondition.
- **Direct-translation confirmation:** User confirmed the proposed defaults on 2026-10-26; implementation remains gated by fresh design review and human approval.

## Standard artifact locations

- **Authoritative plan:** `editor-geometry-corrections-v2.md`
- **Design review:** [`reviews/design-review-v2.md`](reviews/design-review-v2.md)
- **Validations:** `validations/validation-[work-order-id].md`
- **Execution log:** [`execution-log.md`](execution-log.md)
- **Work orders:** [`work-orders/`](work-orders/)

## Outcome

Make the screenplay editor feel like a predictable, responsive writing surface: role blocks use the approved spacing and Courier default, caret blinking and pointer placement remain natural, screenplay casing and parenthetical conventions are applied consistently, and long documents remain measurable and responsive without premature pagination complexity.

## Boundary and validation

- **Parent roadmap/product goal:** Dependable screenplay writing loop in [`screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md).
- **Single observable outcome:** A writer can create a long screenplay using normal paragraph entry, role transitions, pointer clicks, and caret navigation while seeing correct screenplay spacing/casing and no avoidable frame/layout slowdown.
- **Validation boundary:** Fresh v2 design review; one fresh validator per WO; deterministic core tests for layout/editing/normalization; rendered egui tests for blink and nearest pointer placement; performance measurements on controlled multi-page fixtures; native manual smoke when available.
- **Split assessment:** `keep as one initiative` — each WO addresses a different editor symptom, but all share the same fixed profile, editing session, rendered geometry, and responsiveness outcome.

## In scope

- Zero additional boundary gap before new Dialogue and Parenthetical elements; preserve approved inter-element gaps for other role transitions unless the explicit role-pair table says otherwise.
- Change the default persisted profile font family from `Courier New` to `Courier`; preserve bounded validation and future font override compatibility.
- Make the caret blink continuously while the editor response retains focus, using egui repaint/animation state rather than model mutation or a timer thread.
- Map pointer clicks in page blank areas and margins to the nearest rendered line, clamping horizontal position to that line's start/end; clicks above/below the document clamp to the nearest document line.
- Normalize Scene Heading and Character text to uppercase as a project default policy, with a persisted/settings seam that can later support an enable/disable user option.
- Auto-insert `()` when entering a Parenthetical and place the caret between them; remove an empty generated pair on Enter/Tab, but preserve non-empty parentheses.
- Measure frame, layout, paint, and event costs for long documents; add revision/profile-keyed layout caching where measurement demonstrates repeated work; defer incremental pagination until a benchmark proves it is needed.

## Out of scope

- Packaging Courier Draft or another bundled font; font packaging is a later initiative.
- Full typography substitution databases, arbitrary per-element formatting, rich text, collaboration, or a new screenplay role.
- Replacing the deterministic core with a retained widget editor or introducing a second canonical text buffer.
- Incremental pagination/page-frontier caching without benchmark evidence; GPU texture caching or renderer replacement without a measured bottleneck.
- Making uppercase normalization retroactively rewrite existing projects unless the explicit policy implementation requires a migration decision.

## Decisions and research findings

### User-approved defaults

- Dialogue and Parenthetical boundaries have zero extra gap above them; their first line begins at the immediately following line origin under the role-specific spacing table.
- The default profile font family is `Courier`. A later initiative may package Courier Draft or another more appropriate font.
- Focused caret blinking is continuously repaint-driven while editor focus is retained.
- Pointer hit testing uses nearest-line selection with horizontal clamping and document-span vertical clamping.
- Scene Heading and Character casing is canonical edit policy, controlled by a project-default setting seam for a future user option.
- Empty generated `()` is removed when Enter/Tab changes element; non-empty parentheses remain.
- Performance work measures first, then adds revision-keyed layout caching; incremental pagination is a measured follow-up, not an assumption.

### Reconciled implementation oracles

- **Spacing matrix:** For body elements, every transition whose incoming role is Dialogue or Parenthetical has `extra_gap = 0`, including Action, Character, Dialogue, Parenthetical, Note, Scene Heading, and Transition predecessors. Every other body role pair has `extra_gap = boundary_gap`. Same-element wrapped lines always use `line_advance`; title-page elements do not use body boundary gaps. Page-capacity accounting adds one logical boundary unit only when the selected extra gap is nonzero.
- **Courier compatibility:** New projects use `Courier`; existing serialized profiles retain their stored family (`Courier`, `Courier New`, or `Courier Prime`). The field remains required for the current schema; malformed or absent fields remain typed load errors rather than silently rewriting a project. No schema bump or migration is authorized by this WO.
- **Caret blink:** A focused collapsed caret alternates visible/hidden phases on the existing 500 ms animation interval, requests the next repaint only while focused, and resets when focus is lost, selection expands, revision changes, or caret identity changes. Unfocused idle frames must not request blink repaint.
- **Nearest pointer placement:** Body hit testing is document-global across rendered body pages. The nearest line is selected by absolute vertical distance in document/page flow; exact ties prefer the earlier line, then the lower page index. A click left/right of a line clamps to its source start/end; an in-line click uses the existing source-safe hit test. Title-page clicks do not move the body caret. Inter-page gaps compare adjacent body lines; empty pages fall through to the nearest non-empty body page.
- **Casing setting:** Add a persisted bounded `uppercase_cues` project/profile setting, defaulting to `true` for new projects and to `true` when decoding a legacy profile with the field absent. Existing text is not rewritten on open. Insertion, paste, role assignment, and role transitions normalize Scene Heading/Character text when enabled; other roles preserve source casing. Unicode normalization uses Rust `to_uppercase()` and remaps the caret to the transformed prefix byte boundary.
- **Parenthetical provenance:** Generated `()` is tracked as transient session metadata keyed by stable element ID and included in undo/redo snapshots, but is not persisted. It is removed only when the marked element is still a Parenthetical, still exactly `()`, and Enter/Tab leaves it. Save/reopen, user-authored `()`, pasted `()`, partial edits, and non-empty text are never treated as generated.
- **Performance oracle:** Benchmarks warm up three runs and collect ten samples per case, reporting p50/p95 in milliseconds plus frame, layout-build, paint, and repaint-request counts. Fixtures target 1, 5, 15, 50, 120, and 180 pages with edits near beginning/middle/end. A cache change must show a reproducible unchanged-read hit and at least a 30% p95 reduction in the measured bottleneck without introducing >10% p95 regression elsewhere; if no bottleneck meets that threshold, record a no-cache decision. Incremental pagination remains deferred unless downstream pagination is proven dominant.

### Current source findings

- [`place_element()`](../../crates/scriptcraft-core/src/layout.rs:241) currently applies one `boundary_gap` to every adjacent element, including Dialogue and Parenthetical boundaries. The v2 spacing WO must change this through an explicit role-pair policy rather than a renderer special case.
- [`PageProfile`](../../crates/scriptcraft-core/src/lib.rs:154) currently defaults to `Courier New`; [`validate_profile()`](../../crates/scriptcraft-core/src/lib.rs:315) already bounds accepted family names, and PDF font mapping has a separate family mapping in [`exports.rs`](../../crates/scriptcraft-core/src/exports.rs:129).
- [`paint_page()`](../../apps/scriptcraft/src/main.rs:309) requests a repaint every 500 ms while focused for the current blink animation. The v2 caret WO should verify continuous blink semantics and avoid unrelated per-frame work.
- [`caret_from_pointer()`](../../apps/scriptcraft/src/main.rs:238) currently delegates only to [`hit_test()`](../../crates/scriptcraft-core/src/layout.rs:139), which requires the pointer to be inside a line rectangle. Margin/blank-space nearest-line behavior needs a layout-level nearest-line query.
- [`EditingSession::insert()`](../../crates/scriptcraft-core/src/editing.rs:132), [`enter()`](../../crates/scriptcraft-core/src/editing.rs:226), and [`tab()`](../../crates/scriptcraft-core/src/editing.rs:241) are the correct transactional boundaries for casing and parenthetical cleanup.
- The app rebuilds [`ScreenplayLayout::build()`](../../apps/scriptcraft/src/main.rs:410) for the complete project on each rendered surface pass. The analysis recommends paragraph/full-layout caches first and incremental pagination only after a 120–180 page benchmark; see [`02-layout-pagination-rendering.md`](../../.analysis/stage2/02-layout-pagination-rendering.md:157).

### WoodCraft research boundary

The checkout does not contain a `craft/wordcraft` source tree; the available research is the repository's documented analysis of WordCraft. Relevant transferable techniques are: revision-keyed session layout caching, paragraph/layout cache keys that include every metric-affecting input, nearest-line hit testing weighted toward vertical distance, shared layout for render/hit/caret/PDF, and cached page textures/display data for visible rendering. The v2 performance WO will inspect only these documented techniques and measure their fit against ScriptCraft; it will not copy WordCraft's broad document model or UI architecture.

## Proposed work-order plan

1. [`WO-001-role-spacing-and-courier-default.md`](work-orders/WO-001-role-spacing-and-courier-default.md) — encode zero-gap Dialogue/Parenthetical boundaries and change the default profile/PDF font family to Courier.
2. [`WO-002-continuous-caret-blink.md`](work-orders/WO-002-continuous-caret-blink.md) — make focused caret blinking continuous and repaint-driven with rendered-path tests.
3. [`WO-003-nearest-line-pointer-placement.md`](work-orders/WO-003-nearest-line-pointer-placement.md) — support nearest-line and margin/blank-area pointer placement with source-safe clamping.
4. [`WO-004-role-casing-policy.md`](work-orders/WO-004-role-casing-policy.md) — add project-default uppercase normalization for Scene Heading and Character text with a future-option seam.
5. [`WO-005-parenthetical-entry-policy.md`](work-orders/WO-005-parenthetical-entry-policy.md) — insert and clean generated parenthetical delimiters transactionally.
6. [`WO-006-long-document-responsiveness.md`](work-orders/WO-006-long-document-responsiveness.md) — measure frame/layout costs and add only evidence-backed revision-keyed caching.

WO-001 must precede the pointer and performance work because it establishes the final shared geometry and font inputs. WO-004 and WO-005 depend on the existing transactional editing boundary and can follow WO-001. WO-006 executes last so its measurements reflect the settled geometry and editing policies.

## Durable-document impact

- **Requirements:** Verify existing writing-loop requirements first; update only if casing, parenthetical policy, or responsiveness becomes a newly frozen user-facing contract.
- **Blueprints:** Update [`screenplay-core.md`](../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../blueprints/features/screenplay-writing-loop.md), and [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md) for approved role-pair spacing, font default, input policy, and cache ownership.
- **Format reference:** Update [`format-rules.md`](format-rules.md) with the approved zero-gap role pairs and Courier default; do not claim bundled-font parity.

## Risks and mitigations

- **Role spacing changes pagination:** use role-pair geometry tables, page-boundary fixtures, and screen/PDF parity tests.
- **Casing surprises writers or breaks offsets:** normalize only through transactional edits, preserve UTF-8 boundaries, and record the future setting seam without adding its UI yet.
- **Nearest-line hit testing selects an unexpected paragraph:** use vertical-distance-first scoring, deterministic tie-breaking, and caret→click round trips for every role.
- **Blink repaint or frame requests cause load:** distinguish intentional focused blink repaint from unconditional `request_repaint()`, instrument frame/layout/paint counts, and test unfocused idle behavior.
- **Caching returns stale geometry:** key caches by revision and every profile/layout input; add invalidation tests before any optimization is accepted.

## Design readiness checklist

- [x] User defaults and exclusions are recorded.
- [x] One observable outcome and validation boundary are defined.
- [x] Research findings and WoodCraft boundary are recorded.
- [x] Work-order breakdown and dependency order are defined.
- [x] Durable-document impact is classified.
- [x] Fresh v2 design review has been completed and reconciliation evidence is recorded.
- [x] Required human design approval for the reconciled v2 gate has been obtained.

## Scoped human approval — v2

- **Approval status:** `approved for execution`
- **Approver:** User
- **Approval date:** `2026-10-26`
- **Approval scope:** Reconciled v2 design, all six work orders, exact spacing/font/blink/pointer/casing/parenthetical/performance oracles, and sequential execution order.
- **Approval evidence:** User instruction: “I approve the reconciled v2 design, all six work orders, their exact oracles, and the sequential execution order. Proceed to branch verification and implementation dispatch.”

## WO-007 amendment — parenthetical transition and boundary policy

**Amendment status:** `approved for execution after fresh design review`

On 2026-10-27 the user reported a remaining parenthetical failure and commissioned a more precise command policy. This amendment is separate from completed WO-005/005a history and requires its own review/approval cycle.

- Persist `auto_parenthetical` and `strict_transition_boundaries` as `PageProfile` JSON booleans, both defaulting to `true` for new projects and legacy profiles with missing fields. No schema bump is authorized; malformed values remain typed errors. Setting changes are persisted profile changes, survive save/reopen, and are not editing-session text mutations.
- `auto_parenthetical` gates both automatic insertion and generated-pair cleanup. When false, Dialogue→Parenthetical does not insert delimiters, and no generated cleanup occurs; user-authored, loaded, or previously generated-but-now-unmarked empty pairs follow ordinary role transitions and remain text. When true, Dialogue→Parenthetical Tab creates `()` and places the caret between delimiters. Empty Parenthetical Tab/Enter converts to Dialogue and removes both delimiters regardless of caret position around the empty pair.
- Delete on an empty Parenthetical with a collapsed caret before `(` or between delimiters removes the element and moves to the preceding element's end. If it is the only/first element, it becomes an empty Dialogue at index 0, offset 0 to preserve the model invariant. With the caret after `)`, Delete removes only `)` and leaves the caret after `(`. A non-collapsed selection delegates to ordinary deletion and creates no special parenthetical action.
- Accepted text-bearing shapes are `()`, `(text`, `text)`, and `text`. Tab/Enter advances only at the inner-text end: immediately before `)` for `()`/`(text`, at `text.len()` for `text)`/`text`. At the valid boundary, missing opening/closing delimiters are repaired, existing text is preserved, and a new Dialogue is created below. At all other interior positions, Tab/Enter are consumed no-ops with no mutation/history. Other malformed delimiter patterns remain ordinary text.
- Enter at offset 0 before an opening `(` inserts an empty Parenthetical above the current element; with automation on it is generated `()` and caret offset 1, with automation off it is empty text and caret offset 0. The current element remains byte-for-byte unchanged below.
- With `strict_transition_boundaries` enabled, Tab/Enter are disabled/no-op for non-collapsed selections or collapsed carets whose byte offset is not exactly the element's UTF-8-safe `text.len()` in Scene Heading, Character, or Transition. Empty elements therefore qualify at offset 0. Valid end-boundary transitions retain the approved role matrix.
- WO-007 executes after completed WO-005/005a and does not reopen or change completed WO-006 performance work. It requires its own fresh design review, human approval, implementation, and validation cycle.

Authoritative handoff: [`WO-007-parenthetical-transition-and-boundary-policy.md`](work-orders/WO-007-parenthetical-transition-and-boundary-policy.md). Proposed requirements/blueprint impact is `update` because this amendment adds explicit user-facing command and project-setting contracts. WO-005 and WO-005a completion ledger evidence is retained as completed predecessor history; WO-007 is the sole authority for the new command rules.

## WO-008 amendment — Dialogue Enter split policy

On 2026-10-27 the user requested a follow-on correction: Dialogue Enter at the end of the element retains Dialogue→Action behavior, while Enter at an interior caret creates a new Dialogue element so the suffix does not become Action. The authoritative handoff is [`WO-008-dialogue-enter-split-policy.md`](work-orders/WO-008-dialogue-enter-split-policy.md). WO-008 does not reopen WO-005/WO-007 or change Tab, Parenthetical, geometry, or performance behavior; it requires its own fresh design review, approval, implementation, and validation cycle.

### Scoped human approval — WO-008

- **Approval status:** `approved for execution`
- **Approver:** User
- **Approval date:** `2026-10-27`
- **Approval scope:** Reconciled WO-008 handoff, exact end/interior Dialogue Enter matrix, transaction/state assertions, requirements amendment, and blueprint updates.
- **Approval evidence:** User instruction: “Approve WO-008 and its reconciled handoff; dispatch fresh implementation now.”

### Scoped human approval — WO-007 amendment

- **Approval status:** `approved for execution`
- **Approver:** User
- **Approval date:** `2026-10-27`
- **Approval scope:** Reconciled WO-007 amendment and work order, including default-on project settings, exact parenthetical command matrices, strict transition boundaries, persistence compatibility, and validation traceability.
- **Approval evidence:** User instruction: “Approve the reconciled WO-007 amendment and work order; proceed to implementation dispatch after recording approval.”

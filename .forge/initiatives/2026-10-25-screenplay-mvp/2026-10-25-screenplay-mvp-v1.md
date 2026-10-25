# Initiative: Screenplay MVP

## Initiative metadata

- **Phase:** `plan`
- **Status:** `active`
- **Owner/orchestrator:** Forge orchestrator
- **Created:** 2026-10-25
- **Updated:** 2026-10-25
- **Authoritative version:** `v1`
- **Initiative mode:** `standard`
- **Mode requested by:** `user`
- **Mode confirmation:** User explicitly requested the first standard Forge initiative.
- **Rapid-mode eligibility:** `not eligible` — standard mode was explicitly requested and the scope has consequential architecture, persistence, layout, and desktop decisions.
- **Base/default branch:** `master` (current unborn branch; no commit exists)
- **Initiative branch:** `dev/2026-10-25-screenplay-mvp`
- **Branch policy:** `required`
- **Branch naming convention:** `dev/YYYY-MM-DD-short-slug`
- **Branch capability/evidence:** Git is available. `git switch -c dev/2026-10-25-screenplay-mvp` succeeded while the current `master` branch had no commit; the branch was created from the current unborn `master` state. No commit was made.
- **Reduced-capability fallback or opt-out evidence:** Not applicable.
- **Branch creation failure:** `none` — the initial explicit `master` start-point failed because `master` has no commit; the no-start-point branch creation succeeded and is recorded above.
- **Validation cadence:** `per work order`
- **User checkpoint policy:** No manual user checkpoint after each WO, but each WO is validated immediately after its implementation by a separate fresh validation agent; design review still occurs before execution, and a consolidated human confirmation/closeout occurs after all WOs.
- **Phase transition evidence:** `draft → plan` on 2026-10-25: user requested formal creation of the first initiative and dependency-ordered WOs; plan and WO artifacts were created. The original MVP then passed `plan → review` and `review → execute` after the original formal design review and recorded human approval limited to WO-001; WO-001 through WO-008 were subsequently implemented and independently validated, with that approval and evidence preserved as historical records. On 2026-10-25, the user requested a post-MVP scope correction after inspecting the actual UI; the initiative returned from closeout preparation to `plan` to create WO-009 and WO-010. The amended initiative remains in `plan`: the correction's design review is `blocked` for ledger reconciliation, no amended `plan → review` transition has been recorded, and amended human design approval is pending. This correction does not invalidate or rewrite the original WO-001–WO-008 approval or validation evidence.
- **Direct-translation confirmation:** `requested and confirmed` — this initiative and its WOs directly translate the user's approved MVP outcome and exclusions.

## Standard artifact locations

- **Authoritative plan:** `2026-10-25-screenplay-mvp-v1.md`
- **Design review:** [`reviews/design-review-ui-correction.md`](reviews/design-review-ui-correction.md) — amended initiative review is `blocked` pending ledger reconciliation; it does not authorize execution.
- **Validations:** [`validations/`](validations/) — historical validation artifacts for WO-001 through WO-008 are preserved; no validation exists or is claimed for WO-009 or WO-010.
- **Execution log:** [`execution-log.md`](execution-log.md)
- **Work orders:** [`work-orders/`](work-orders/)
- **Scope correction note:** [`scope-correction-2026-10-25.md`](scope-correction-2026-10-25.md)

## Outcome

Deliver a runnable desktop-first Rust MVP writing loop in `apps/scriptcraft`, backed by reusable support layers under `crates`, so one screenwriter can create and edit semantically typed screenplay content, safely undo changes, inspect deterministic pages, save/reopen/recover a native project, export PDF/TXT, and exercise the same behavior through headless tests.

## Initiative boundary and sizing

- **Parent roadmap or product goal:** The first product MVP stage for the dependable screenplay writing loop in [`../../requirements/product-overview.md`](../../requirements/product-overview.md), following the workspace-foundation recommendation in [`../../project-manifest.md`](../../project-manifest.md).
- **Initiative type:** `product MVP stage`
- **Single observable outcome:** A representative screenplay scene completes the create → semantic edit → role transition → undo/redo → deterministic preview → native save/reopen/recovery → PDF/TXT export loop in the runnable desktop app and headless test path.
- **Validation boundary:** Formal pre-execution design review, then one fresh validation agent per implemented WO using changed-file/diff evidence, followed by a consolidated human confirmation and closeout after all WOs.
- **Split assessment:** `keep as one initiative`
- **If split, traceability and dependencies:** Not applicable; the work orders are dependency-ordered slices of one independently reviewable writing-loop outcome.

## Scope

### In scope

- Runnable Cargo/Rust workspace foundation with the desktop binary at `apps/scriptcraft` and reusable support layers under `crates`.
- Semantic screenplay roles: scene heading, action, character, dialogue, parenthetical, transition, and authored note.
- Canonical UTF-8 text, stable IDs, title-page metadata, checked positions, safe editing, role-aware transitions, undo, and redo.
- One shared deterministic fixed-profile layout/display path for continuous writing, page preview, source mappings, and print output.
- Focused desktop-first UI, native versioned JSON atomic save/reopen, bounded recovery candidates, PDF export, deliberately lossy TXT export, and headless tests.

### Out of scope

- DOCX/FDX import or export; collaboration, cloud sync, tracked revisions, comments, or revision comparison.
- MCP, remote control, network automation, or a command catalog as a product feature.
- Beat board, outline/planning, structure-line, production breakdown, scheduling, budgeting, and character databases.
- Broad Word-like formatting, tables, fields, headers/footers, footnotes, shapes, images, equations, arbitrary nested stories, dual dialogue, scene numbering, colored pages, or locked pages.
- Web support, multi-platform polish, pixel-perfect editor parity, font databases, and unmeasured incremental pagination.

## Context and motivation

The checkout contains a populated, reviewed-but-not-approved requirements and blueprint baseline and no product source under the observed app/crate paths. The user-approved goal is to establish the smallest trustworthy desktop writing loop rather than broad document compatibility. The independent requirements and blueprint artifacts are inputs and evidence of planning alignment only; they do not claim design approval, implementation, or validation.

## Exploration scratchpad

### Initial findings

- The manifest records Rust and desktop-first as user-approved, Cargo and the exact UI framework as agent-default/unknown, sequential work orders as the safe default, and per-WO fresh validation as mandatory.
- The requirements baseline provides `REQ-WRITING-LOOP-001` through `REQ-WRITING-LOOP-004`, AC IDs, and NFR IDs. The blueprints define the semantic core, desktop boundary, fixed-profile layout, native persistence, and export boundaries as desired architecture.
- The requirements review and blueprint review both pass with warnings, but neither is a design review for this initiative and neither authorizes implementation.

### Questions and hypotheses

- The workspace-foundation WO must verify the exact Cargo package graph, UI framework isolation, first desktop target, page-profile/font inputs, native extension, and available commands before downstream WOs rely on them.
- The role-transition matrix and export loss-report minimum must be made executable within the relevant WOs without expanding the product boundary.

### Options considered

| Option | Benefits | Costs/risks | Current direction |
|---|---|---|---|
| One large implementation WO | Fewer files | High cascade risk, poor ownership, weak validation boundaries | Rejected |
| Multiple sequential WOs by dependency boundary | Narrow ownership, fresh validation after each slice, clear recovery | More orchestration records | Selected |
| Parallel feature WOs | Potential speed | Current package and contracts are unverified; merge risk | Rejected for now |

### Decisions and discussion notes

- User correction is authoritative: **no manual user checkpoint after each WO, but each WO is validated immediately after its implementation by a separate fresh validation agent; design review still occurs before execution, and a consolidated human confirmation/closeout occurs after all WOs.**
- The dependency cascade risk remains: a workspace, model, or layout contract defect can invalidate downstream WOs. The per-WO fresh validation cadence is retained specifically to surface that risk early.
- No product source code is created by this planning task.

### Next investigation or conversation focus

- Run the independent design review against this plan and all generated WOs; resolve blockers before moving to execution.

## Requirements and blueprint references

- **Requirements:** [`product-overview.md`](../../requirements/product-overview.md); [`screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md), especially `REQ-WRITING-LOOP-001`–`004`, `AC-WRITING-001.1`–`004.3`, and `NFR-WRITING-001`–`003`.
- **Blueprints:** [`technical-context.md`](../../blueprints/technical-context.md); [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md); [`screenplay-core.md`](../../blueprints/components/screenplay-core.md); [`screenplay-writing-loop.md`](../../blueprints/features/screenplay-writing-loop.md).
- **Independent review inputs:** [`requirements-review.md`](../../requirements/requirements-review.md) and [`blueprint-review.md`](../../blueprints/blueprint-review.md), both read as planning evidence; no initiative design approval is claimed.

## Proposed design

Use a one-way dependency graph: workspace/app shell → reusable semantic core/session/layout/persistence/export support, with the desktop shell translating input and rendering results but never owning a parallel authoritative text buffer. The semantic project owns roles, stable IDs, metadata, canonical UTF-8 text, and fixed page-profile inputs. The session owns checked selection, transactional edits, dirty state, and bounded undo/redo. Layout derives deterministic pages and source mappings from semantic data. Persistence serializes versioned native JSON and uses atomic replacement plus explicit bounded recovery candidates. PDF consumes print layout; TXT is explicitly lossy. All core paths remain callable without a desktop window.

## Work-order plan

1. [`WO-001-workspace-foundation.md`](work-orders/WO-001-workspace-foundation.md) — verify and establish the Cargo workspace, package boundaries, app seam, and headless baseline.
2. [`WO-002-model-schema.md`](work-orders/WO-002-model-schema.md) — establish semantic project/schema contracts and role data.
3. [`WO-003-editing-session.md`](work-orders/WO-003-editing-session.md) — implement checked semantic editing, transitions, undo, redo, and safety invariants.
4. [`WO-004-layout-display.md`](work-orders/WO-004-layout-display.md) — implement fixed-profile deterministic layout, pagination, source mappings, and preview data.
5. [`WO-005-focused-desktop-ui.md`](work-orders/WO-005-focused-desktop-ui.md) — connect the focused desktop writing/editor and page-preview flow to the core.
6. [`WO-006-persistence-recovery.md`](work-orders/WO-006-persistence-recovery.md) — implement versioned native JSON save/reopen, atomic failure safety, and recovery candidates.
7. [`WO-007-exports-integration.md`](work-orders/WO-007-exports-integration.md) — add PDF/TXT export and integrate the representative end-to-end loop.
8. [`WO-008-e2e-hardening.md`](work-orders/WO-008-e2e-hardening.md) — harden headless end-to-end behavior, hostile-input regressions, and desktop smoke evidence.
9. [`WO-009-unified-live-editing-surface.md`](work-orders/WO-009-unified-live-editing-surface.md) — correct the desktop interaction model to one primary live page-layout editing surface with caret, direct typing, selection, deletion, and role-aware editing.
10. [`WO-010-basic-named-persistence-ui.md`](work-orders/WO-010-basic-named-persistence-ui.md) — replace hard-coded native `scriptcraft.json` UI behavior with basic user-selected/named Open, Save, and Save As while keeping recovery outside the UI.

Execution is sequential by default. Each WO has one owner and must be implemented in a fresh implementer session, marked `in-review` with changed-file and validation evidence, then immediately assessed by a separate fresh validation agent. WO-001 through WO-008 remain complete with their preserved passed validation artifacts and original approval history. WO-009 and WO-010 are newly created `draft` work orders only; neither is approved, implemented, `in-review`, complete, or validated. The amended initiative is in `plan` with amended design approval pending; no dispatch is authorized until the amended plan reaches `review`, the corrected design review passes, and separate human approval for WO-009/WO-010 is recorded.

## Risks and decisions

- **Risk:** Dependency cascade risk remains: failures in the unverified workspace, schema, or shared layout contracts may require downstream WO replanning or fix WOs. **Mitigation:** strict dependency order, narrow ownership, per-WO fresh validation, and explicit fix-chain handling.
- **Risk:** Font/profile and export contracts are unresolved in the baseline. **Mitigation:** resolve bounded decisions in WOs 001, 004, and 007 and record any authorized durable-document updates.
- **Risk:** Native recovery can be confused with atomic save. **Mitigation:** keep them separate in WO-006 and require failure-injection evidence.
- **Decision needed:** For the amended WO-009/WO-010 scope, complete the corrected `plan → review` gate, obtain a separate required human design approval, and only then consider `review → execute`. **Owner:** orchestrator/user approver. The original approval remains limited to WO-001 and does not authorize WO-009 or WO-010.
- **User-requested scope correction:** The original closeout preparation is superseded for now by the user-requested UI correction recorded in [`scope-correction-2026-10-25.md`](scope-correction-2026-10-25.md). The next gate is planning/design readiness for WO-009 and WO-010; no implementation or validation is authorized by this amendment alone.

## Design readiness checklist

- [x] Scope and exclusions are explicit.
- [x] The initiative has one observable outcome and one named validation boundary.
- [x] The advisory split assessment is recorded.
- [x] Requirements and blueprints are linked.
- [x] Work orders are independently executable and dependency-ordered.
- [x] Every work order carries a durable-document impact classification.
- [x] Initiative mode metadata is recorded.
- [x] Branch decision and phase transition evidence are recorded.
- [x] Validation cadence and user checkpoint policy are recorded.
- [x] Applicable validation evidence boundary and policy gates are identified.
- [x] Original MVP formal design review and WO-001-scoped human approval are preserved as historical evidence for WO-001 through WO-008.
- [ ] Amended WO-009/WO-010 formal design review has passed; the current amended review is blocked pending ledger reconciliation.
- [ ] Separate human design approval for amended WO-009/WO-010 has been obtained.

## Original human design approval — WO-001 only

- **Approval status:** `approved for execution` — historical approval for WO-001 only; not approval of the amended scope.
- **Approver:** User
- **Approval date:** 2026-10-25
- **Approval scope:** Execute WO-001 only, within the approved initiative scope and exclusions; do not implement later work orders or product features.
- **Approval evidence:** User instruction received in the implementation session: “Implement only Forge WO-001...” including authorization to record the human design approval and proceed with the foundation implementation.
- **Review basis:** Independent original design review outcome `PASS WITH WARNINGS` in [`reviews/design-review.md`](reviews/design-review.md:93-100). This record does not establish amended design approval or design readiness for WO-009/WO-010.
- **Warnings preserved as follow-ups:** Resolve and evidence downstream foundational decisions before dependent WOs (DESIGN-REVIEW-W-001); keep the reusable/core package boundary justified and UI/window dependencies at the app boundary (DESIGN-REVIEW-W-002); record attempted/passed/failed/unavailable validation evidence with ownership at each boundary (DESIGN-REVIEW-W-003); and reconcile WO-006's durable-document classification before that WO is implemented (DESIGN-REVIEW-W-004).

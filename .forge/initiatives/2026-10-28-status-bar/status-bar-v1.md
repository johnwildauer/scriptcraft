# Initiative: Status Bar — v1

## Initiative metadata

- **Phase:** `execute`
- **Status:** `active`
- **Owner/orchestrator:** Forge orchestrator
- **Created:** 2026-10-28
- **Updated:** 2026-10-28
- **Authoritative version:** `v1`
- **Initiative mode:** `standard`
- **Mode requested by:** `user`
- **Mode confirmation:** The user explicitly requested a new standard Forge initiative; no rapid path was requested.
- **Rapid-mode eligibility:** `not eligible` — the initiative combines toolbar extraction, new status/page behavior, continuous transient zoom, horizontal navigation and an integrated regression boundary; it follows the standard review/approval and per-WO validation gates.
- **Base/default branch:** `master` at `83621f34f5db17b3060b7c28fce795decd903581` (observed current/base branch at creation).
- **Initiative branch:** `dev/2026-10-28-status-bar`
- **Branch policy:** `required`
- **Branch naming convention:** `dev/YYYY-MM-DD-short-slug` (manifest policy).
- **Branch capability/evidence:** Before creation, `git status --short --branch` returned `## master`; `git status --porcelain=v1 -uall` returned no entries; `git rev-parse HEAD` and `git rev-parse --verify 'master^{commit}'` both returned `83621f34f5db17b3060b7c28fce795decd903581`. The listed local branches did not include `dev/2026-10-28-status-bar`. `git switch -c dev/2026-10-28-status-bar master` succeeded. After creation, `git status --short --branch` returned `## dev/2026-10-28-status-bar`; `git rev-parse HEAD`, `git rev-parse master`, and `git rev-parse dev/2026-10-28-status-bar` all returned `83621f34f5db17b3060b7c28fce795decd903581`; `git status --porcelain=v1 -uall` returned no entries.
- **Reduced-capability fallback or opt-out evidence:** Not applicable; the required local branch was created successfully.
- **Branch creation failure:** `none`
- **Validation cadence:** `per work order` (project default).
- **Phase transition evidence:** Created in `draft` / `active` on 2026-10-28. `draft → plan` on 2026-10-28 after independent repository/process research, recorded convergence, and the user's explicit confirmation to record the direction and create exactly three draft work orders. At that planning handoff the initiative remained in `plan` and no review artifact was claimed. On 2026-10-28, the user's explicit request for the independent design-alignment review authorized `plan → review`; the three linked WOs and review evidence were available. The review at [`reviews/design-review.md`](reviews/design-review.md) independently returned `pass`. The required human approval is recorded separately and verbatim in that review. Therefore `review → execute` passed on 2026-10-28. The subsequent WO-002 validation and fix-loop transitions are recorded in the execution log and in the validation reconciliation below; current phase is `execute`.
- **Direct-translation confirmation:** `requested and explicitly confirmed` — at the convergence checkpoint the user said, “Yes. Record the proposed direction and create the three draft work orders; do not start design review or execution.” This authorizes direct translation into planning artifacts only; it is not design approval, review readiness, implementation authorization, execution, commit, or merge approval.

## Standard artifact locations

- **Authoritative plan:** [`status-bar-v1.md`](status-bar-v1.md)
- **Design review:** [`reviews/design-review.md`](reviews/design-review.md) — completed with outcome `pass`; separate human approval is recorded in its dedicated section.
- **Validations:** [`validations/`](validations/) — [`validation-WO-001.md`](validations/validation-WO-001.md) (`pass`), [`validation-WO-002.md`](validations/validation-WO-002.md) (historical `fail`, preserved), [`validation-WO-002a.md`](validations/validation-WO-002a.md) (`pass` for the fix and full original WO-002 boundary), and [`validation-WO-003.md`](validations/validation-WO-003.md) (`fail` on the stable WO-003 range; actual validator dispatch `01a184cb-6163-710f-b7f3-52d14c89466d` reconciled).
- **Execution log:** [`execution-log.md`](execution-log.md)
- **Work orders:** [`work-orders/`](work-orders/) — WO-001, WO-002, and its fix WO-002a are `complete`; fresh WO-002a validation passed both the fix and full original acceptance. WO-003 is `blocked` after failed fresh validation by actual host dispatch `01a184cb-6163-710f-b7f3-52d14c89466d`; linked fix WO-003a is `in-review` after implementation evidence by actual implementer host dispatch `01a184da-1260-706c-96ec-3c55e3fe5d2c`. Its separate fix commit and committed-range whitespace check remain pending before fresh validation.

## Outcome

Establish a desktop-owned modular toolbar/status presentation and item-based status bar with a viewport-derived visible-page indicator on the left, accurate clean/unsaved and frame/operation feedback in the center, and synchronized transient zoom controls on the right. Preserve existing toolbar/editor/core contracts and fixed screenplay geometry while making the desktop status bar useful at narrow widths and accessible to keyboard users.

## Initiative boundary and sizing

- **Parent roadmap or product goal:** The dependable desktop screenplay-writing loop in [`../../requirements/features/screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md), especially the desktop's status/error presentation and toolbar contract; technical ownership is described by [`../../blueprints/containers/scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md).
- **Initiative type:** `product MVP stage` — desktop status/presentation capability.
- **Single observable outcome:** The desktop has a modular status bar with the requested left/center/right information and synchronized zoom controls, while the toolbar/status presentation no longer depends on inline assembly in the large main entry point.
- **Validation boundary:** First complete the mandatory fresh-context design review and record separate human design approval before execution. Thereafter implement and independently validate each WO in a fresh session, sequentially. The integrated boundary covers preserved toolbar/editor behavior; correct viewport-visible page and failure/state feedback; continuous and preset zoom in both directions; fixed pagination/source-map/PDF invariants; narrow-window navigation; and a real-render/input desktop smoke when available. The project has no configured manual hard-stop gate: unavailable smoke is recorded as unavailable, never as passed, and does not waive explicit automated acceptance criteria or any future configured gate.
- **Split assessment:** `keep as one initiative` — the three sequential WOs share one desktop-chrome outcome and one integration boundary; no fourth integration WO is authorized. Cross-feature regression coverage belongs in WO-003.
- **If split, traceability and dependencies:** No split proposed; the three WOs are sequential stages of this single desktop-chrome outcome and are recorded below.

## Scope

### In scope

- Extract the toolbar into discrete desktop-owned files while preserving existing composition, width/allocation, overflow, action, hover/cursor/focus and behavior contracts.
- Add discrete item-based status-bar files that support noninteractive text, clickable text and clickable icons as capabilities without inventing new actions or icons. Compose left, center and right regions; avoid an unneeded shared generic framework and keep current core contracts.
- Left region: show visible rendered page `N of total`, independent of caret. Select greatest vertical overlap with the actual editor scroll clip; include title page in total; ties prefer earlier page; gaps choose nearest page; failures/stale geometry show unavailable. Keep state current after scroll, resize, zoom, reflow and caret reveal despite footer-before-editor frame order.
- Center region: accurately show current clean/unsaved state (clean does not mean previously saved), cumulative frames, path, operation status and visible errors; retain access to privacy-safe diagnostics, using compact disclosure if needed.
- Right region: one transient page zoom, 75–300% continuous slider rounded/displayed as whole percent, blue fill and circular thumb; clickable menu with exactly `75%`, `90%`, `100%`, `125%`, `150%`, `175%`, `200%`, `250%`, and `300%`; synchronize controls and show intermediate values accurately. `100%` is the existing window-fit baseline, so effective scale may vary on resize. Provide horizontal navigation for wide pages.
- Zoom does not alter pagination, source mapping, PDF geometry, saved content or dirty state; do not persist preferences. Use deterministic bounded narrow-width fallback and keyboard-accessible labels/controls.

### Out of scope

- Application source, tests or durable requirements/blueprint changes in this planning-only task; no design review or execution in this task.
- Persisting zoom preferences; changing native project data/schema, screenplay semantics, fixed pagination, source mapping, PDF behavior, titlebar behavior, or core crate boundary.
- Inventing status-bar actions/icons, toolbar commands, generic frameworks, broad theme redesign, or unrelated editor/layout changes.
- Broad theme redesign, new toolbar commands, or unrelated editor/layout changes.

## Context and motivation

The user wants the toolbar Items and forthcoming footer/status-bar Items modularized out of the large application entry point, and wants most current footer output replaced by a compact, useful status bar. Existing contracts place status/error presentation and desktop controls in the desktop shell, keep toolbar composition desktop-owned, and treat viewport zoom as presentation-only rather than a print-pagination input. Relevant durable context is in [`REQ-WRITING-LOOP-005`](../../requirements/features/screenplay-writing-loop.md), [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md), and [`technical-context.md`](../../blueprints/technical-context.md).

## Exploration scratchpad

This section records repository research and the user-confirmed planning direction. Work-order details remain proposals until fresh-context design review and separate human design approval; nothing here claims implementation.

### Initial findings

- **Observed source:** [`ScriptCraftApp::update()`](../../../apps/scriptcraft/src/main.rs) composes the top toolbar, bottom status panel, and central editor in one 3,860-line application entry point. The toolbar has a typed `ToolbarItem`/action model, allocation helpers, renderer, overflow menu, and command routing, all colocated in that file.
- **Observed footer composition:** The bottom panel renders operation status; path or `(unsaved)`; cumulative `frames`, `events`, `layouts`, cache `hits`, `pages`, and `repaint` counters; a dirty-only yellow unsaved marker; error text; and bounded native-input focus/event diagnostics. `show_error()` retains a detail in `self.error` and a status summary; failed save/open paths preserve session/path state. The request specifically retains clean/unsaved, cumulative frames, path, operation/error visibility, and safe diagnostics; other counters may be compactly disclosed rather than silently lost.
- **Observed toolbar contract:** Toolbar Items are New, Open, Save, Save As, Flexible Space, Elements, Flexible Space, Export PDF. Existing tests exercise ordering, measured widths, flexible-space allocation, narrow overflow, action routing, role choices, hover/cursor ownership, and clipped-edge bounds. Preserve these behaviors through extraction; do not create a generic framework absent a demonstrated need.
- **Observed editor/layout:** `live_document_surface()` builds or retrieves shared `ScreenplayLayout`, computes page presentation metrics per frame, creates actual rendered page rectangles inside `ScrollArea::vertical()`, requests conditional caret reveal against the UI clip, then installs one whole-editor interaction response. `ScreenplayLayout.pages` includes an optional title page as well as at least one body page; title-page lines are excluded from body caret mapping. There is no horizontal navigation path today.
- **Observed frame ordering/currentness risk:** The bottom status panel is composed before the central editor. Its page indicator cannot trust geometry from a prior frame after scroll, resize, scale, reflow, or caret reveal; cached geometry must be validated against current layout/presentation/clip/scroll state and explicitly report unavailable rather than stale while the editor publishes current geometry.
- **Observed presentation baseline:** There is no user zoom state. `presentation_geometry()` derives a fit baseline from a nominal 576-point page width (360 × 1.6), 16-point side padding, and a 240-point minimum; effective scale changes with window width. The app manifest specifies eframe `0.29`; page layout is revision/profile keyed and independent of viewport scale.
- **Observed requirement/blueprint boundary:** [`screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md) has toolbar criteria and fixed-profile zoom-independent pagination, but no status-bar or user-zoom controls acceptance criteria. The desktop blueprint already assigns status/error and toolbar ownership to the desktop and says viewport size/zoom do not redefine print pagination. New user-visible behavior warrants proposed requirements and desktop-blueprint updates during authorized implementation; those durable documents are not changed in this planning task.
- **No implementation or durable-document change** was made as part of this planning session.

### Questions and hypotheses (superseded by the recorded convergence below)

- What exact viewport-derived rule selects `N` when no page is fully visible, multiple pages are visible, or a title page and body page overlap the viewport? The displayed page must not follow the caret.
- Does “frames” mean the existing cumulative frame count specifically, and should other performance counters, native-input diagnostics, path text, and error text remain in this bar or move to another surface?
- Which status-bar items are clickable icons, clickable text, or noninteractive text, and what action/accessible label belongs to each? The user specified interaction classes but not a final per-item assignment.
- Is the slider continuous or restricted to the nine listed menu percentages? If continuous, how is its displayed percentage/menu value rounded or matched to a menu option?
- How should the blue fill and circular thumb behave while dragging, on keyboard interaction, when focus changes, and when the bar is too narrow? The requested visual direction is recorded; exact widget/state behavior remains open.
- Should zoom remain transient for the current window/session, or should it be remembered outside the native screenplay project? No persistence behavior is authorized by this request.
- What extraction boundary is appropriate for toolbar/status Item types, renderers, and app-state adapters, given the desktop shell owns these concerns and the current toolbar is already typed but colocated in `main.rs`?
- How should the regions adapt at narrow window widths without obscuring page state or zoom controls?

### Options considered

| Option | Benefits | Costs/risks | Current direction |
|---|---|---|---|
| Keep toolbar and footer inline in `main.rs` and add more rendering there | Minimal extraction work | Preserves large-entry-point coupling and entangles future items | Rejected; conflicts with explicit modularization direction |
| Extract desktop-owned toolbar and status-bar item files; retain existing app adapters | Clear item and renderer ownership without moving UI concerns into core | Specific module names/trait shapes remain implementation details subject to review | Selected direction; use discrete files and no unneeded shared generic framework |
| Put status models, viewport controls, or zoom state in `scriptcraft-core` | Would make some state headless | Couples semantic core to window/UI presentation | Rejected; keep transient presentation state and viewport geometry in desktop app |
| Use app-wide egui zoom or persist user zoom preferences | Framework shortcut or remembered preference | Scales toolbar/status too, conflicts with page-only presentation and explicit non-persistence boundary | Rejected; own transient page-view zoom, 100% is responsive fit baseline |

### Decisions and discussion notes

- Earlier open questions above are historical scratchpad entries; the following converged decisions supersede them.

### Convergence record — 2026-10-28

- **User authorization (verbatim):** “Yes. Record the proposed direction and create the three draft work orders; do not start design review or execution.” This is explicit direct-translation and WO-creation confirmation only. It is explicitly **not** approval of the design, readiness for review, authorization to implement/execute, or permission to commit or merge.
- **Selected direction:** Extract current toolbar presentation into discrete desktop files; create item-based status-bar files supporting noninteractive text, clickable text, and clickable icons as capabilities, without inventing actions or icon assignments. Keep desktop ownership and current core contracts; do not introduce a generic shared framework without demonstrated need.
- **Left region:** Display `page N of total` from actual rendered page/scroll geometry, independently of caret. Select the page with greatest vertical overlap against the actual editor scroll clip; exact ties select the earlier page. Include the title page among rendered sheets. In gaps choose the nearest page, breaking ties earlier. On layout/geometry failure or stale geometry expose an explicit unavailable state. Reconcile the status-before-editor frame order with currentness checks for scrolling, resizing, zoom, reflow and caret reveal; never present stale page data as current.
- **Center region:** Accurately reflect `EditingSession::dirty()` and current clean state, operation status, path, and cumulative frame count. A clean state means currently clean, not that the project was ever saved. Failures remain visible with access to path, operation status, and safe diagnostics; use a compact disclosure for additional counters/native-input diagnostics if needed, without document text or privacy-sensitive content.
- **Right region:** One transient page-zoom state from 75% through 300%, continuous slider quantized/displayed to whole percentages, filled blue track and circular thumb; adjacent clickable menu contains exactly 75, 90, 100, 125, 150, 175, 200, 250 and 300 percent. Slider and menu are two-way synchronized, intermediate values remain accurate, and 100% means existing responsive window-fit baseline (effective scale may change on resize). Provide horizontal navigation when zoomed pages exceed viewport width. Zoom must not affect pagination, source mapping, PDF geometry, saved content or dirty state. No saved preference.
- **Responsive/accessibility:** Three-region layout has deterministic bounded fallback at narrow widths; preserve access to page/status/error/zoom. Interactive controls have keyboard focus/activation and explicit accessible labels; text labels remain legible and no unrequested icon/action is added.
- **Meaningful reductions:** Keep the one initiative and exactly three sequential WOs: (1) toolbar extraction and preservation only; (2) item-based status bar and viewport page, including all three item interaction capabilities, truthful center state, error/path/diagnostic access, narrow-width fallback and page selection/currentness; (3) synchronized transient zoom plus integrated cross-feature regression. Do not create a fourth cross-feature WO. Do not change core, saved project schema, page profile, export, or unrelated UI.
- **Alternatives rejected:** Inline growth in `main.rs`; shared generic item framework absent demonstrated need; moving desktop state into core; caret-derived page; arbitrary page selection rules; framework-global zoom; persisted zoom; inventing icons/actions; dropping path/error/diagnostic access; using native smoke unavailability to waive automated acceptance.
- **Dependencies:** WO-001 toolbar extraction/preservation precedes WO-002 and WO-003; WO-002 status-item, center-state and viewport-page contracts precede WO-003's synchronized zoom and integrated regression. Work remains sequential because the WOs touch the shared app shell. All require a fresh implementer and a separate fresh validator per manifest policy after future approval.
- **Risks:** egui's status-before-editor composition can expose stale geometry; horizontal navigation may affect editor focus/selection; narrow width can hide status/error affordances; presentation zoom can accidentally influence core layout or global UI scale; extraction can regress toolbar hit/focus/cursor/overflow/action contracts; failure UI can conceal useful diagnostics. WOs require explicit regression coverage for each.
- **Validation boundary/manual status:** Automated tests and all manifest-configured future Cargo checks are required. Real desktop smoke is recommended to observe actual rendering/input at normal and narrow sizes, but is not a configured hard-stop gate. If unavailable, record limitation without claiming a pass; this does not weaken explicit tests or any future project-policy gate.
- **Durable-document impact:** Requirements and desktop-container blueprint changes are planned under the future WOs because new user-visible status/page/zoom contracts are absent from the current requirement. Existing feature/technical/core contracts must be checked and recorded unchanged unless evidence justifies a separately authorized change. No requirement, blueprint, manifest, application source or test is modified now.
- **Phase and gates at planning handoff:** The user explicitly authorized the `draft → plan` transition and three draft WOs. At that time, review, human approval, and execution remained future gates. The later review request and gate results are recorded in the design-review reconciliation below.

### Research evidence and limitations

- Read-only sources reviewed: manifest project policy; initiative process and templates; existing toolbar initiative/WO examples; screenplay-writing requirements; desktop, feature and technical blueprints; current app source, core layout/model/session code and app tests.
- **Fresh Git preflight and final state:** branch `dev/2026-10-28-status-bar`; `HEAD`, `master`, and the initiative branch all resolve to `83621f34f5db17b3060b7c28fce795decd903581`. Before these planning edits, staged and tracked-unstaged diffs were empty; the only untracked paths were the existing status-bar initiative plan/log/placeholders. After drafting, staged and tracked-unstaged diffs remain empty; the untracked status-bar initiative directory contains those artifacts and exactly the three new WOs. No unrelated pre-existing work was found or altered; no Git operation changed branch, index or commit.
- Existing deterministic app tests cover toolbar ordering/allocation/overflow/action and role routing, page-fit geometry, caret reveal, rendered editor interactions, save/open dirty/path behavior, and error paths. There are no current status-bar/page-scroll/user-zoom tests. Their absence motivates focused contracts in the new WOs, not an assertion of existing coverage.
- No source code, requirements, blueprints, manifest or tests were changed; no Cargo validation command was run. Read-only Git inspection was performed. No design review, execution, implementation, validation, commit, or merge has occurred.

### Planning handoff (historical; current gate state is reconciled below)

- At the planning handoff, the three draft work orders were the formal handoff from this converged plan; a later explicit readiness request and approval were still required before review and implementation.

## Requirements and blueprint references

- **Requirements:** [`screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md) — existing toolbar criteria and fixed-layout invariants; status-bar/page/zoom criteria are absent. Each WO identifies authorized future requirement update scope.
- **Blueprints:** [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md) — desktop status/error, toolbar, and viewport presentation ownership; [`screenplay-writing-loop.md`](../../blueprints/features/screenplay-writing-loop.md) — desktop rendering over shared layout; [`technical-context.md`](../../blueprints/technical-context.md) — desktop/core separation and viewport-only presentation scaling.

## Converged proposed direction (review reconciliation recorded below)

Extract toolbar presentation into discrete desktop files and add discrete status-bar item/rendering files without a generic shared framework. Use three regions: viewport-visible page; accurate clean/unsaved, operation/path, cumulative frames and retained errors/diagnostic access; and synchronized transient page zoom with responsive baseline, exact presets, keyboard access and horizontal navigation. Keep all UI state and geometry in the desktop shell. Detailed executable behavior and tests are set out in the three draft WOs. At the planning handoff this was a proposed direction only; the later independent review and human approval are recorded below.

## Work-order plan

The user explicitly confirmed direct translation into exactly three **draft** work orders. They are sequential. At the planning handoff they remained unapproved and not executable pending fresh-context design review and separate human design approval; the later gate result is recorded below.

1. [`WO-001-toolbar-extraction.md`](work-orders/WO-001-toolbar-extraction.md) — extract existing toolbar behavior into a desktop-owned boundary and preserve its existing behavior/contracts only; no status-bar scope.
2. [`WO-002-status-items-and-viewport-page.md`](work-orders/WO-002-status-items-and-viewport-page.md) — create the item-based status bar, all three interaction capabilities, truthful center clean/unsaved and cumulative frames, path/operation/error/diagnostic access and narrow fallback; derive the left-region page identity from actual visible page/clip geometry with deterministic ties/gaps/title/unavailable and geometry freshness. Status: `complete` after the linked fix passed fresh validation over the full original acceptance range.
   - Fix: [`WO-002a-freshness-and-noninteractive-text.md`](work-orders/WO-002a-freshness-and-noninteractive-text.md) — resolved both blockers in [`validation-WO-002.md`](validations/validation-WO-002.md); status `complete`. Implementer host dispatch `01a183b3-777f-7055-b601-e91404620223`; stable fix commit `a73c9de14195fb32ae36dce3ae2bba090c8970cf` directly parented by `4be598803fe19c8f212115ae0f2455afef412fac`. Fresh validation [`validation-WO-002a.md`](validations/validation-WO-002a.md) passed both the fix and full original WO-002 range; actual validator host dispatch ID `01a183d2-d3db-75f9-b653-b8c72106d86b` is reconciled, with no internal session ID available or inferred.
3. [`WO-003-transient-zoom-and-integrated-regression.md`](work-orders/WO-003-transient-zoom-and-integrated-regression.md) — implement transient page zoom/right controls and horizontal navigation; integrate the full toolbar/editor/status/page/zoom/error regression matrix (no fourth WO).

## Risks and decisions

- **Risk:** Caret reveal or frame ordering can leave a stale page indicator. **Mitigation:** currentness key, unavailable state on mismatch and targeted scroll/resize/zoom/reflow/reveal tests in WO-002.
- **Risk:** Narrow layouts or compact diagnostics hide critical state. **Mitigation:** bounded deterministic fallback, persistent operation/error/path access and accessible control labels; exercise narrow and failure cases in WO-002 and the integrated WO-003 matrix.
- **Risk:** Zoom changes page geometry, PDF output, source maps, project dirty state, or unrelated global UI. **Mitigation:** app-owned page-only transient state and explicit unchanged-layout/persistence/PDF regression tests in WO-003.
- **Risk:** Module extraction changes toolbar behavior despite being intended as relocation. **Mitigation:** preserve and exercise existing order, width, overflow execution, action routing, hover/cursor/focus and rendered interaction contracts in WO-001 and WO-003.
- **Risk:** Native smoke may be unavailable. **Mitigation:** distinguish recommended smoke from project policy (no manual hard-stop configured), report unavailable honestly, and retain all automated acceptance/gates.
- **Decision needed:** No product-direction decision is open for these three WOs. Exact Rust module internals and concrete control implementation remain implementation details for design review/implementation and may not contradict this direction.

## Design readiness checklist

- Planning completeness only; this checklist does not claim design-review readiness or approval.
- [x] Scope and exclusions are explicit.
- [x] The initiative has one observable outcome and one named validation boundary.
- [x] The initiative is traceable to its roadmap or parent product goal, including any staged MVP relationship.
- [x] The advisory split assessment is recorded; no split initiative is proposed.
- [x] Requirements and blueprints are linked; the required additive criteria and desktop-blueprint updates are scoped in the draft WOs but are not made in this planning task.
- [x] Three independently reviewable draft work orders are ordered by dependency; this does not mark them approved or ready for execution.
- [x] Every work order carries a proposed durable-document impact classification per [`00-overview.md`](../../process/00-overview.md); actual edits remain future work.
- [x] Initiative mode metadata (mode, requested by, confirmation, eligibility) is recorded.
- [x] Draft creation, branch evidence and the authorized `draft → plan` transition are recorded. At checklist creation no `plan → review` transition was claimed; see current review reconciliation below.
- [x] Validation cadence is recorded.
- [x] Future validation evidence boundary and project-policy/manual gates are identified; desktop smoke is recommended but is not a configured hard-stop gate.
- [x] Separate human design approval was pending at `review → execute` when this checklist was recorded; see current review reconciliation below.

## Design-review reconciliation — 2026-10-28

- **Current gate result:** The independent design-alignment review in [`reviews/design-review.md`](reviews/design-review.md) returned `pass`. It found no blocker; each of the three linked WOs aligns with the converged outcome and explicit decisions.
- **Readiness transition:** `plan → review` was authorized by the user's explicit request for this independent design review after the three linked WOs were prepared. The plan, all three WOs, relevant requirements/blueprints, manifest, and review policy were available for comparison.
- **Human approval and gate:** The exact approval statement is recorded in the review's separate **Explicit human approval** section, not as a finding. The review-pass condition and human-approval condition are both evidenced; `review → execute` is therefore satisfied and the current phase is `execute`.
- **Execution boundary:** No WO was executed or dispatched; WO-001, WO-002, and WO-003 remain `draft`. The execution log was read only and intentionally not edited. No implementation, requirement/blueprint, manifest, branch, index, or commit change was made.
- **Next action:** Dispatch WO-001 to its own fresh implementer session under the standard sequential execute/validate loop; WO-002 and WO-003 remain downstream of their stated dependencies.

## WO-002 failed-validation reconciliation — 2026-10-28

- The fresh validator dispatched as host ID `01a183a1-84ed-71a8-bc37-8608a4b1633a` returned `fail` on stable comparison `fde481db08ba058a1327e2d23abb9b056c2556c3..4be598803fe19c8f212115ae0f2455afef412fac`.
- The two hard-stop findings are: successful New/Open replacement can show a stale page once when the old and new session revisions alias at zero before the editor publishes fresh geometry; and noninteractive status text uses a hover-response label despite the explicit response-free contract. Exact evidence and prior test history remain in [`validations/validation-WO-002.md`](validations/validation-WO-002.md).
- The orchestrator reconciled initiative phase `execute` → `validate` for the validator dispatch and `validate` → `execute` after the failed result/fix creation; initiative status remains `active`. WO-002 transitions `in-review` → `blocked`. The linked fix [`WO-002a-freshness-and-noninteractive-text.md`](work-orders/WO-002a-freshness-and-noninteractive-text.md) was created as `draft`; that historical state is superseded by the later dispatch reconciliation below. WO-003 remains `draft` and undispatched.

## WO-002a implementation-commit reconciliation — 2026-10-28

- Fresh fix implementer host dispatch ID `01a183b3-777f-7055-b601-e91404620223` is reconciled; no internal session/agent ID is inferred. WO-002a remains `in-review`; original WO-002 remains `blocked`; initiative remains `execute` / `active`; WO-003 remains `draft` and undispatched.
- Fix implementation commit `a73c9de14195fb32ae36dce3ae2bba090c8970cf` is directly parented by WO-002 commit `4be598803fe19c8f212115ae0f2455afef412fac`. It contains only the three authorized source paths `apps/scriptcraft/src/main.rs`, `apps/scriptcraft/src/status_bar.rs`, and `apps/scriptcraft/src/toolbar.rs` (all modified). Cached and committed-range whitespace checks passed. The fix remains unvalidated/incomplete pending a fresh independent validator; no WO completion or WO-003 transition is claimed.

## WO-002a dispatch reconciliation — 2026-10-28

- Fresh dedicated fix implementer host dispatch ID `01a183b3-777f-7055-b601-e91404620223` is reconciled from the supplied host identity. No internal session/agent ID is available or inferred.
- WO-002a is `in-review` with implementation evidence; original WO-002 remains `blocked`, initiative remains `execute` / `active`, and WO-003 remains `draft` and undispatched. All required implementation checks are reported passing by the implementer; stable fix commit and independent fresh validation remain pending. No completion transition is claimed. Full details are in the linked fix work order and execution log.

## WO-002a and full WO-002 fresh validation result — 2026-10-28

- Fresh independent validation [`validation-WO-002a.md`](validations/validation-WO-002a.md) returns `pass` on both exact ranges: the WO-002a fix range `4be598803fe19c8f212115ae0f2455afef412fac..a73c9de14195fb32ae36dce3ae2bba090c8970cf` and full original WO-002 range `fde481db08ba058a1327e2d23abb9b056c2556c3..a73c9de14195fb32ae36dce3ae2bba090c8970cf`.
- Both prior blockers pass criterion-level review and regressions; every original WO-002 acceptance criterion and all required commands pass. Native visual/input smoke remains unavailable and optional. The previous failed validation is preserved unchanged.
- At validator return, the host dispatch ID and work-order state reconciliation were pending; no internal session/agent ID was inferred. That result-time snapshot is superseded by the orchestrator reconciliation below. No manual approval is recorded.

## WO-002 validation-result and completion reconciliation — 2026-10-28

- Actual fresh validator host dispatch ID: `01a183d2-d3db-75f9-b653-b8c72106d86b`; no internal session/agent ID is available or inferred.
- [`validation-WO-002a.md`](validations/validation-WO-002a.md) passes both the exact fix range `4be598803fe19c8f212115ae0f2455afef412fac..a73c9de14195fb32ae36dce3ae2bba090c8970cf` and complete original WO-002 range `fde481db08ba058a1327e2d23abb9b056c2556c3..a73c9de14195fb32ae36dce3ae2bba090c8970cf`. It confirms all required gates passed and no blocker remains; the historical failed [`validation-WO-002.md`](validations/validation-WO-002.md) is preserved unchanged.
- WO-002a transitions `in-review` → `complete` and WO-002 transitions `blocked` → `complete` under the validation/fix process. Initiative remains `execute` / `active`; WO-003 remains `draft` and undispatched. Native desktop visual/input smoke is unavailable and is not claimed as passed; it is optional, not a configured hard-stop gate.

## WO-003 failed-validation reconciliation — 2026-10-28

- Fresh independent [`validation-WO-003.md`](validations/validation-WO-003.md) returned `fail` on the stable committed range `a73c9de14195fb32ae36dce3ae2bba090c8970cf..671c7267328b960be10c82be1fa032df895b0874`; the target is a direct child of the base. The finding is the missing required blue value-proportional slider trailing fill; the other inspected acceptance criteria and required automated checks passed. Native smoke remains unavailable/optional.
- Actual validator host dispatch ID `01a184cb-6163-710f-b7f3-52d14c89466d` is reconciled; no internal session/agent ID is available or inferred. The failed validation artifact and committed implementation are preserved.
- WO-003 transitions `in-review` → `blocked`. Created linked fix [`WO-003a-blue-proportional-slider-fill.md`](work-orders/WO-003a-blue-proportional-slider-fill.md) as `draft`, not dispatched. Initiative phase returns to `execute` and status remains `active`.
- Required next flow: dispatch WO-003a to a new fresh implementer; require a separate fix commit and all scoped checks; then dispatch a different fresh validator to assess both the fix commit and full original WO-003 acceptance. Do not complete WO-003 or close the initiative before both boundaries pass.

## WO-003a dispatch reconciliation / implementation-commit preflight — 2026-10-28

- The actual fresh WO-003a implementer host dispatch ID is `01a184da-1260-706c-96ec-3c55e3fe5d2c`; the host provides no internal session/agent ID, and none is inferred. WO-003a remains `in-review`; original WO-003 remains `blocked`; initiative remains `execute` / `active`.
- The implementer reports required focused/workspace checks passing and only `apps/scriptcraft/src/status_bar.rs` changed for product code. These reports and the preserved initial formatting failure/repair are detailed in the fix WO and execution log; this bookkeeping step did not rerun validation.
- Preflight confirmed branch `dev/2026-10-28-status-bar`, `HEAD` `671c7267328b960be10c82be1fa032df895b0874` with direct parent `a73c9de14195fb32ae36dce3ae2bba090c8970cf`, empty index, and only the authorized source plus this initiative's plan/log/WO/failed-validation/fix-WO records in the worktree. No unrelated or unexpected staged change was present.
- The fix commit must be a separate direct child of `671c7267328b960be10c82be1fa032df895b0874`. Its committed-range whitespace check and hash/parent record are pending; neither work order completion nor initiative closure is authorized here.

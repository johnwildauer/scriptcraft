# Initiative: Toolbar

## Initiative metadata

- **Phase:** `closeout`
- **Status:** `active`
- **Owner/orchestrator:** Forge orchestrator
- **Created:** 2026-10-27
- **Updated:** 2026-10-27
- **Authoritative version:** `v1`
- **Initiative mode:** `standard`
- **Mode requested by:** `user`
- **Mode confirmation:** Standard mode is the project default; the user asked to start and formalize a new toolbar initiative without requesting the rapid path.
- **Rapid-mode eligibility:** `not eligible` — the request establishes a reusable toolbar composition model, a role-changing Elements control, flexible/normal spacing, overflow behavior, command routing, and future customization seams; the product and UI contracts require design review before work orders.
- **Base/default branch:** `dev/2026-10-26-editor-geometry-corrections-v2` — observed current branch at initiative creation; the manifest's recorded `master` baseline is stale relative to this checkout and must be reconciled before closeout.
- **Initiative branch:** `dev/2026-10-27-toolbar`
- **Branch policy:** `required`
- **Branch naming convention:** `dev/YYYY-MM-DD-short-slug`
- **Branch capability/evidence:** `git switch -c dev/2026-10-27-toolbar` succeeded on 2026-10-27 from the observed clean current branch.
- **Reduced-capability fallback or opt-out evidence:** Not applicable.
- **Branch creation failure:** `none`
- **Validation cadence:** `per work order`
- **Phase transition evidence:** Initiative created in `draft` on 2026-10-27 from the user's toolbar brief. `draft → plan` and `plan → review` on 2026-10-27 after the user explicitly requested all work orders, review, and implementation start; three dependency-ordered work orders and the design-review artifact were created. `review → execute` on 2026-10-27 after the fresh design review returned PASS with no blockers and the user's explicit request to begin implementation served as human design approval. `execute/validate → closeout` on 2026-10-27 after WO-001, WO-002, and WO-003 each passed separate fresh validation with no hard-stop findings.
- **Direct-translation confirmation:** `requested and confirmed` — the user explicitly requested creation of all work orders from this toolbar brief and asked to proceed through review into implementation.

## Standard artifact locations

- **Authoritative plan:** `toolbar-v1.md`
- **Design review:** [`reviews/design-review.md`](reviews/design-review.md)
- **Validations:** `validations/validation-[work-order-id].md`
- **Execution log:** [`execution-log.md`](execution-log.md)
- **Work orders:** [`work-orders/`](work-orders/)

## Outcome

Give ScriptCraft one formal toolbar surface below the title bar whose contents are modeled as typed Items, routes actionable Items through the session/application command boundary, keeps the current screenplay role visible and changeable through an Elements dropdown, supports fixed and flexible spacing, and establishes an extensible overflow path without prematurely implementing future customization breadth.

## Initiative boundary and sizing

- **Parent roadmap or product goal:** The dependable screenplay writing loop in [`../../requirements/features/screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md), especially the desktop shell's role-aware editing and save/export actions.
- **Initiative type:** `product MVP stage`
- **Single observable outcome:** The desktop app presents the approved default toolbar as a coherent, responsive Item-based toolbar; each visible action works through the existing application/session boundaries, and the Elements item mirrors and changes the focused element role.
- **Validation boundary:** Fresh design review, then one fresh validator per work order using focused Rust tests plus a bounded desktop/render smoke path when available. The final acceptance path exercises the default item order, action dispatch, role synchronization/change, spacing allocation, and overflow menu behavior at narrow and wide widths.
- **Split assessment:** `keep as one initiative` — the Item model, default composition, role control, spacing, and overflow are one toolbar composition boundary and share one visible validation surface.
- **If split, traceability and dependencies:** Not applicable.

## Scope

### In scope

- Formalize a toolbar Item model with a required text label and explicit item kinds for actionable/icon items, the Elements role dropdown, normal Space, and Flexible Space.
- Replace the current mixed convenience-button and role-control presentation with the default order: New, Open, Save, Save As, Flexible Space, Elements, Flexible Space, Export PDF.
- Preserve the existing command behavior for New/Open/Save/Save As/Export PDF while routing toolbar activation through a clear application/session command seam.
- Make action Items visually button-like with icon-first presentation, hover feedback that changes lightness without adding a border, and accessible text labels/tooltips as appropriate to the selected UI framework.
- Keep the Elements display synchronized to the role at the current caret/focus and apply a selected replacement role transactionally through the editing/session layer.
- Define deterministic fixed-width normal spacing and equal-share flexible spacing that consumes remaining toolbar width.
- Define an overflow presentation that retains leftmost fitting Items, places a compact `>>` affordance at the far right, and exposes non-spacing overflow Items in a text menu; nested text-dropdown behavior remains executable through a submenu-like interaction when needed.
- Add focused regression coverage and bounded desktop/manual evidence for layout, dispatch, role synchronization, and narrow-window behavior.

### Out of scope

- Toolbar customization UI, persistence of toolbar arrangements, drag/reorder, user-defined Items, toolbar profiles, or plugin/command registries.
- Adding new screenplay commands beyond the requested default actions and role change behavior.
- Implementing every future icon, menu, export format, or overflow command; the initial default includes only the named Items.
- Changing the canonical project format, persisting toolbar/session presentation state, or moving semantic role ownership into the desktop shell.
- General visual-theme redesign, title-bar redesign, broad responsive-window work, or non-toolbar editor geometry changes.

## Context and motivation

The current desktop surface renders convenience actions directly in the top panel and renders role controls separately through [`role_controls()`](../../apps/scriptcraft/src/main.rs:269). The current toolbar-like block also includes an additional lossy text export action, while the requested default composition centers the role selector between flexible spaces and omits that action for now. A formal Item boundary is needed before adding more commands and customization without duplicating command handling or allowing the UI to become a second semantic editor.

The desktop container owns the window, controls, dialogs, status, and platform services, while [`EditingSession`](../../crates/scriptcraft-core/src/editing.rs:1) owns semantic role changes and selection. The toolbar should therefore remain a desktop composition/presentation concern and send semantic role changes and action commands through existing boundaries.

## Exploration scratchpad

### Initial findings

- The current top panel is assembled inline in [`update()`](../../apps/scriptcraft/src/main.rs:1035) with direct `ui.button(...)` calls for New, Open, Save, Save As, Export PDF, and Export TXT.
- The current role selector is a separate wrapped row in [`role_controls()`](../../apps/scriptcraft/src/main.rs:269), iterating all roles and calling [`EditingSession::change_role()`](../../crates/scriptcraft-core/src/editing.rs:1) on selection.
- Existing desktop state already owns the [`EditingSession`](../../apps/scriptcraft/src/main.rs:54), native path, status/error presentation, and file/export adapters, so the first toolbar initiative can stay in the app shell while preserving semantic ownership in core.
- The current app uses egui and has focused app-level tests, but native visual interaction is not consistently available in validation sessions; automated geometry/dispatch tests and explicit manual-smoke limitations must be recorded rather than assumed.

### Questions and hypotheses

- Which icon source and exact icon glyphs best fit the existing dependency policy? The initial implementation may use framework-native symbols or text fallbacks, but the Item contract should not depend on a particular icon library.
- Should an Item's `Text` be visible, tooltip-only, or menu-only by kind? The requested default implies icon-first actions, while overflow requires text labels; accessibility labels should exist in every presentation.
- What exact width constitutes a normal button, and how should minimum widths behave when the Elements label is longer than the available space?
- Does overflow count only actionable/non-spacing Items, or can the Elements Item overflow with a nested role menu? The prompt requires non-spacing Items in the overflow list and submenu-esque execution for dropdown Items; this needs a deterministic priority rule in design review.
- How should an Elements role change behave if the caret/selection is unavailable or the session rejects the mutation? The existing transactional error/status path should be reused without changing the model on failure.

### Options considered

| Option | Benefits | Costs/risks | Current direction |
|---|---|---|---|
| Keep inline buttons and add more conditionals | Smallest first edit | No reusable Item contract; spacing/overflow become brittle | Rejected |
| Define a desktop-owned typed Item model and renderer | Makes composition, spacing, overflow, and future customization explicit while keeping semantics in the session | Requires careful UI-state and width policy design | Selected |
| Put toolbar Items in the core crate | Headless model could be shared | Couples semantic core to desktop presentation and icon/menu concerns | Rejected |
| Use a generic command registry immediately | Could support future customization | Adds unrequested infrastructure and broadens security/ownership surface | Deferred |

### Decisions and discussion notes

- The initiative is standard, sequential, and uses per-work-order fresh validation under project policy.
- Every toolbar entry is conceptually an `Item`; spacing Items participate in layout but are excluded from the overflow command list.
- Elements is the central semantic Item. Its displayed role derives from the current caret/focus, and selection calls the session role-change operation rather than mutating project data directly.
- The initial default order is authoritative: New, Open, Save, Save As, Flexible Space, Elements, Flexible Space, Export PDF.
- Overflow is planned as a layout/presentation mechanic, not a reason to add toolbar customization or a general command catalog in this initiative.

### Next investigation or conversation focus

- Converge the Item contract, overflow priority/nesting rules, icon/accessibility presentation, and work-order decomposition before creating work-order artifacts.

## Requirements and blueprint references

- **Requirements:** [`product-overview.md`](../../requirements/product-overview.md) — desktop-first writing loop, explicit semantic roles, safe role-aware editing, and PDF export; [`features/screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md) — desktop editing and command-flow acceptance criteria to be mapped to toolbar-specific criteria during convergence.
- **Blueprints:** [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md) — window/input/dialog/status ownership and semantic UI-to-core boundary; [`screenplay-core.md`](../../blueprints/components/screenplay-core.md) — transactional role changes and session ownership; [`technical-context.md`](../../blueprints/technical-context.md) — desktop/core separation and headless validation.

## Proposed design

Add a desktop-owned toolbar composition layer beneath the title bar. Its ordered collection contains typed Items with stable identity, text label, presentation/command metadata, and one of the action, Elements, Space, or Flexible Space kinds. Action Items invoke existing app operations through a single toolbar dispatch path. Elements reads the current focused element role each frame, displays the corresponding label, and invokes the session role-change operation when a new role is selected. The renderer computes fixed widths for regular Items and Space, distributes remaining width equally among Flexible Space Items, and moves the rightmost non-fitting non-spacing Items into a text overflow menu while preserving the left-to-right priority rule. Any nested dropdown Item in overflow remains actionable through a submenu-like child menu.

The implementation should keep toolbar layout state transient. It must not persist icons, order, overflow state, or window measurements in the screenplay project. Durable requirements and the desktop blueprint should be updated by authorized work orders once the converged contracts and observable acceptance criteria are settled.

## Work-order plan

1. [`WO-001-toolbar-item-model-and-default-actions.md`](work-orders/WO-001-toolbar-item-model-and-default-actions.md) — establish the desktop-owned Item model, default composition, action dispatch, labels, and icon-first action presentation.
2. [`WO-002-elements-role-item.md`](work-orders/WO-002-elements-role-item.md) — implement the central Elements dropdown, current-role synchronization, and transactional role-change dispatch.
3. [`WO-003-toolbar-spacing-and-overflow.md`](work-orders/WO-003-toolbar-spacing-and-overflow.md) — implement fixed/flexible spacing, fitting, overflow menu behavior, nested dropdown routing, and responsive regressions.

Execution is sequential. WO-001 establishes the Item and command seams; WO-002 depends on that composition and the existing session selection; WO-003 depends on the complete default Item collection and presentation metadata.

## Risks and decisions

- **Risk:** Overflow behavior can hide important actions or make dropdown Items impossible to use. **Mitigation:** define a deterministic leftmost-fit rule, preserve labels in overflow, and add narrow-width interaction tests including nested dropdown behavior.
- **Risk:** Role display can become stale or toolbar code can bypass semantic transaction rules. **Mitigation:** derive Elements from the current session selection every frame and route changes through [`EditingSession::change_role()`](../../crates/scriptcraft-core/src/editing.rs:1).
- **Risk:** Framework-native icon choices may be unstable or inaccessible. **Mitigation:** require a text label/accessibility name for every Item and keep icon choice behind presentation metadata.
- **Decision needed:** Exact icon source, normal-button width, overflow fitting algorithm, and dropdown nesting policy. **Owner:** Forge design review with guided agent defaults where no user-only decision is required.
- **Decision needed:** Whether the toolbar-specific acceptance criteria should be added to the writing-loop feature requirement or a new toolbar requirement document. **Owner:** initiative planner during convergence; expected durable-document impact is requirements `update` and desktop blueprint `update`.

## Design readiness checklist

- [x] Scope and exclusions are explicit.
- [x] The initiative has one observable outcome and one named validation boundary.
- [x] The initiative is traceable to its roadmap or parent product goal, including any staged MVP relationship.
- [x] The advisory split assessment is recorded; any split initiatives have distinct outcomes, dependencies, and review/closeout boundaries.
- [x] Requirements and blueprints are linked.
- [x] Work orders are independently executable and ordered; work-order artifacts are intentionally deferred during draft exploration.
- [x] Every work order carries a durable-document impact classification per [`00-overview.md`](../../process/00-overview.md).
- [x] Initiative mode metadata (mode, requested by, confirmation, eligibility) is recorded.
- [x] Phase transition evidence and the direct-translation confirmation (when applicable) are recorded.
- [x] Validation cadence is recorded.
- [x] The applicable validation evidence boundary and explicit project-policy/manual gates are identified; optional checks may remain recorded as notes or warnings.
- [x] Required human approval has been obtained; the user's 2026-10-27 instruction to create the work orders, perform review, and begin implementation authorizes execution after the fresh design review passed.

## Human design approval — execution authorized

- **Approval status:** `approved for execution`
- **Approver:** User
- **Approval date:** 2026-10-27
- **Approval scope:** The complete toolbar initiative and its three ordered work orders, executed sequentially with fresh validation after each work order.
- **Approval evidence:** User explicitly requested: “create all the work orders, do the review, and begin implementation”; the fresh design review returned `PASS` with no blockers.

## Closeout follow-ups

- Native hover/click/dropdown interaction was unavailable to implementers and validators; repeat the toolbar visual smoke on a window-capable validation environment. Owner: Forge orchestrator; trigger: next native desktop validation session.
- Reconcile the manifest's recorded default branch (`master`) with the observed branch lineage before any remote/PR-dependent workflow. Owner: Forge orchestrator; trigger: next initiative closeout or remote action.

## Closeout record

- **Closeout status:** `in-progress`
- **Closeout date:** 2026-10-27
- **Closeout evidence:** All three work orders passed separate fresh validation; required Cargo gates passed for every work order; no fix work orders or session exceptions were required. Git commit/merge evidence is recorded in the final execution-log entry.

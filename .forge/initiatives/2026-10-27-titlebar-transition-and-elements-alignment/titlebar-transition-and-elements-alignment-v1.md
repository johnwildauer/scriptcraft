# Initiative: Title-Bar Transition and Elements Alignment

## Initiative metadata

- **Phase:** `closeout`
- **Status:** `active`
- **Owner/orchestrator:** Forge architect/planner
- **Created:** `2026-10-27`
- **Updated:** `2026-10-27`
- **Authoritative version:** `v1`
- **Initiative mode:** `standard`
- **Mode requested by:** `user`
- **Mode confirmation:** The user explicitly requested standard-mode research/planning and draft-only artifacts. No rapid path is requested.
- **Rapid-mode eligibility:** `not eligible` — macOS/eframe behavior, transition rendering, and ComboBox layout remain unresolved.
- **Base/default branch:** `master` per [`project-manifest.md`](../../project-manifest.md:63). The parent closeout records merge `935c162` and clean `master`; this session did not execute a fresh Git command.
- **Initiative branch:** `dev/2026-10-27-titlebar-transition-and-elements-alignment`
- **Branch policy:** `required` when implementation begins and local capability is confirmed
- **Branch naming convention:** `dev/2026-10-27-titlebar-transition-and-elements-alignment`
- **Branch capability/evidence:** On 2026-10-27, `git rev-parse master` returned `5b8eda725f767fbed530c021882b15bec82b6ceb`; `git switch -c dev/2026-10-27-titlebar-transition-and-elements-alignment` succeeded from `master`, and the resulting branch was confirmed with `git branch --show-current`.
- **Reduced-capability fallback or opt-out evidence:** None; use the manifest fallback if branch capability is unavailable.
- **Branch creation failure:** `none recorded`
- **Validation cadence:** `per work order`
- **Phase transition evidence:** `draft → plan` on `2026-10-27`: the user’s approved follow-up prompt explicitly confirmed formalization of the four ordered implementation WOs and their direct translation from the converged decomposition. `plan → execute` on 2026-10-27 after the design review recorded `PASS WITH WARNINGS`, the implementation gate was entered, and the required branch was created from `master`; `execute/validate → closeout` followed four completed work orders and fresh validation evidence. Commit and merge evidence is recorded in the execution log.
- **Direct-translation confirmation:** `confirmed by user on 2026-10-27` — the user approved exactly four ordered WOs: viewport state/native restore, unified titlebar/toolbar transition geometry, closed Elements selected-text centering at the current 128-point baseline, and cross-surface native validation/document reconciliation. This is the required broad-prompt confirmation under [`02-initiative.md`](../../process/02-initiative.md:159).

## Standard artifact locations

- **Authoritative plan:** `titlebar-transition-and-elements-alignment-v1.md`
- **Design review:** [`reviews/design-review.md`](reviews/design-review.md) — not created
- **Validations:** `validations/` — empty marker-only directory; no validation claimed
- **Execution log:** [`execution-log.md`](execution-log.md)
- **Work orders:** [`work-orders/`](work-orders/) — [`WO-001-viewport-state-direction-and-native-restore-contract.md`](work-orders/WO-001-viewport-state-direction-and-native-restore-contract.md), [`WO-002-unified-titlebar-toolbar-surface-and-transition-geometry.md`](work-orders/WO-002-unified-titlebar-toolbar-surface-and-transition-geometry.md), [`WO-003-elements-closed-selected-text-centering.md`](work-orders/WO-003-elements-closed-selected-text-centering.md), [`WO-004-cross-surface-native-validation-and-document-reconciliation.md`](work-orders/WO-004-cross-surface-native-validation-and-document-reconciliation.md)

## Outcome

Produce one evidence-bounded desktop-shell direction in which the title-bar/toolbar is one surface, double-clicking it maximizes from a normal frame and restores from maximized without full-screen, egui content metrics remain stable during the native transition to the extent supported by eframe 0.29/macOS, and the closed Elements selection is centered while opened choices remain left-aligned. This is a reviewed implementation plan, not an implementation or native-behavior claim.

## Initiative boundary and sizing

- **Closed parent:** [`toolbar-and-titlebar-polish-v1.md`](../2026-10-27-toolbar-and-titlebar-polish/toolbar-and-titlebar-polish-v1.md:1), including [`WO-002-custom-title-strip-and-viewport-toggle.md`](../2026-10-27-toolbar-and-titlebar-polish/work-orders/WO-002-custom-title-strip-and-viewport-toggle.md:1) and [`validation-WO-002.md`](../2026-10-27-toolbar-and-titlebar-polish/validations/validation-WO-002.md:1).
- **Relationship:** This is a new follow-up prompted by observed behavior after prior closeout, not a silent revision of the closed initiative. The parent remains historical and closed; current source is the implementation baseline.
- **Parent product goal:** Dependable desktop screenplay-writing loop, especially [`REQ-WRITING-LOOP-005`](../../requirements/features/screenplay-writing-loop.md:90) and [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md:59).
- **Initiative type:** `product MVP stage` / desktop-shell follow-up
- **Single observable outcome:** A window-capable macOS run shows a seamless title/toolbar surface; normal-frame double-click requests maximize/fill and maximized double-click restores the prior user frame without full-screen; transition icon/text metrics meet the agreed measurable tolerance; and the closed Elements control centers only selected text while its popup choices remain left-aligned.
- **Validation boundary:** Fresh design review, then separate per-work-order deterministic tests and identified macOS native smoke recording state, command direction, outer/inner rectangles, metric sizes, seam pixels, hit behavior, and ComboBox closed/open alignment. Missing native evidence remains an explicit limitation.
- **Split assessment:** `keep as one initiative` — all requests share one desktop title-bar/toolbar validation boundary and can be split into independently reviewable work orders.

## Scope

### In scope

- Reconcile the requested double-click state machine with [`title_strip_viewport_command()`](../../apps/scriptcraft/src/main.rs:1671), [`title_strip()`](../../apps/scriptcraft/src/main.rs:1675), egui `ViewportInfo`, `ViewportCommand::Maximized(bool)`, winit, and macOS behavior. Determine whether the observed inversion is stale/unknown state, command timing, backend behavior, or actual direction logic.
- Preserve normal-frame restoration semantics without promising process-persistent frame history or an exact frame where eframe/winit/macOS does not expose one.
- Remove the title-strip/toolbar seam by investigating fill, stroke, margins, panel spacing, full-size-content insets, and ownership; retain reliable double-click coverage and centered title text.
- Investigate whether the custom full-size egui title strip and [`presentation_geometry()`](../../apps/scriptcraft/src/main.rs:1509) width-derived scale cause transition resizing, and define the strongest supported stability guarantee without promising unsupported animation control.
- Preserve and record [`TOOLBAR_ELEMENTS_WIDTH`](../../apps/scriptcraft/src/main.rs:26) at its observed `128.0`-point baseline if still present.
- Center selected text only in the closed Elements control while preserving arrow placement, truncation, the 128-point budget, and left-aligned opened choices; investigate egui selected-text/layout APIs and a custom closed display only if necessary.
- Define deterministic geometry/layout evidence and a bounded native-smoke protocol before creating work orders.

### Out of scope

- Application source, tests, requirements, blueprints, work orders, design review, validation, commits, branch creation, or implementation in this planning session.
- Full-screen, custom window-manager/frame-history persistence, snapping, multi-window behavior, filename/title plumbing, broad toolbar/theme redesign, semantic editing, persistence, export, pagination, or cross-platform decoration parity.
- Promises of animation-duration/interpolation control, pixel-identical content during OS resize, or APIs not exposed by installed eframe/winit.

## Context and motivation

The parent’s [`WO-002-custom-title-strip-and-viewport-toggle.md`](../2026-10-27-toolbar-and-titlebar-polish/work-orders/WO-002-custom-title-strip-and-viewport-toggle.md:18) intentionally bounded restoration because eframe 0.29 exposes optional maximized/outer-rectangle state and a maximize command but no universal application-owned exact prior-frame API. Current source maps known state through the inverse command in [`title_strip_viewport_command()`](../../apps/scriptcraft/src/main.rs:1671), and [`update()`](../../apps/scriptcraft/src/main.rs:1697) paints separate title and toolbar panels. The new user observation says runtime behavior and visual transition do not meet the requested contract.

The source currently records the locally increased Elements width as `128.0` points at [`main.rs`](../../apps/scriptcraft/src/main.rs:26). [`elements_toolbar_item()`](../../apps/scriptcraft/src/main.rs:689) uses stock ComboBox width/truncation/selected-text behavior, so closed and popup alignment need separate measurement. The document renderer derives scale from available width in [`live_document_surface()`](../../apps/scriptcraft/src/main.rs:1080), making transition metric stability an explicit platform question.

## Exploration scratchpad

### Initial findings

- [`native_options()`](../../apps/scriptcraft/src/main.rs:1763) requests hidden title/titlebar and full-size content; [`update()`](../../apps/scriptcraft/src/main.rs:1697) renders a 30-point title panel followed by a separate toolbar panel.
- [`title_strip()`](../../apps/scriptcraft/src/main.rs:1675) owns the full-strip double-click response and centered measured galley. [`title_strip_viewport_command()`](../../apps/scriptcraft/src/main.rs:1671) maps `Some(false)` to `Maximized(true)` and `Some(true)` to `Maximized(false)`, which is logically correct if `ViewportInfo::maximized` is current and truthful. The follow-up must investigate state freshness/backend semantics before changing direction.
- The parent records installed API evidence for [`ViewportInfo`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/data/input.rs:194), [`ViewportCommand::Maximized(bool)`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/viewport.rs:948), and [`egui-winit::update_viewport_info()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:943): optional state/outer geometry and a request command, not a universal restore snapshot or animation-control API. Direct registry reads were unavailable to this planning tool; the recorded parent evidence must be rechecked by implementation/review.
- [`presentation_geometry()`](../../apps/scriptcraft/src/main.rs:1509), [`paint_page()`](../../apps/scriptcraft/src/main.rs:967), and [`renderer_font()`](../../apps/scriptcraft/src/main.rs:1539) multiply display geometry/font size by width-derived scale. Native width animation can therefore change content metrics even though the semantic layout is unchanged.
- [`TOOLBAR_ELEMENTS_WIDTH`](../../apps/scriptcraft/src/main.rs:26) is currently `128.0`; the parent’s earlier target was approximately `92.0`. This plan preserves `128.0` as the current baseline pending fresh measurement.
- [`elements_toolbar_item()`](../../apps/scriptcraft/src/main.rs:689) uses `.width(width)`, `TextWrapMode::Truncate`, stock `.selected_text(...)`, and popup `selectable_value`; opened choices should remain left-aligned while only closed selected text changes.
- Existing durable wording is at [`AC-WRITING-005.5`](../../requirements/features/screenplay-writing-loop.md:115), [`AC-WRITING-005.6`](../../requirements/features/screenplay-writing-loop.md:131), and [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md:100). Update only if evidence changes the durable contract; otherwise explicitly verify unchanged.
- The parent records commit/merge/clean-closeout evidence at [`toolbar-and-titlebar-polish-v1.md`](../2026-10-27-toolbar-and-titlebar-polish/toolbar-and-titlebar-polish-v1.md:174). A fresh Git status/diff command was unavailable here, so no stronger current-cleanliness claim is made.

### Questions and hypotheses

- Does macOS report `ViewportInfo::maximized` reliably during/after native transitions, or can stale/unknown state make a logically correct inverse helper appear reversed?
- Does native unmaximize restore the last user outer frame, and can that be observed without app-owned frame history?
- Is the seam caused by differing fills/strokes, margins, panel insets, or the transition itself?
- Can document/icon/text metrics remain stable while available width changes, or must native titlebar/full-size-content configuration change? What can be guaranteed if the OS owns animation?
- Does egui 0.29 expose selected-text alignment independent of the ComboBox arrow and popup layout?

### Options considered

| Option | Benefits | Costs/risks | Direction |
|---|---|---|---|
| Keep inverse maximize helper and diagnose state/backend | Smallest change; API-semantic | May not explain observed inversion | Investigate first |
| Capture app-owned pre-maximize outer frame | Potential exact restore | Race-prone, optional geometry, platform coupling | Only if proven safe |
| Use native titlebar configuration | Native maximize/restore | May lose centered title/custom hit region | Compare, do not assume |
| Shared title/toolbar paint surface | Removes seam while retaining responses | Must avoid doubled insets/strokes | Preferred visual direction |
| Freeze display scale during transition | Stable metrics | No known animation-progress/disable API | Measure feasibility, bounded guarantee |
| Stock ComboBox alignment hook | Preserves popup/arrow/accessibility | API may not independently align closed text | Preferred if supported |
| Custom closed control plus stock popup | Full closed control | Must preserve keyboard/arrow/truncation | Fallback only |

### Decisions and discussion notes

- This is a new standard follow-up, not a revision of the closed parent.
- “Toggle” is insufficient: the contract explicitly requires normal→maximize/fill and maximized→prior-frame restore without full-screen.
- Native animation control is unresolved; the plan may define metric tolerances and fallback behavior but must not promise suppression of OS animation.
- `128.0` points is the current Elements baseline; any change needs evidence and review.
- No work order or design-review artifact is created in this session.

## Requirements and blueprint references

- **Requirements:** [`REQ-WRITING-LOOP-005`](../../requirements/features/screenplay-writing-loop.md:90), [`AC-WRITING-005.5`](../../requirements/features/screenplay-writing-loop.md:115), [`AC-WRITING-005.6`](../../requirements/features/screenplay-writing-loop.md:131), and [`AC-WRITING-005.7`](../../requirements/features/screenplay-writing-loop.md:137).
- **Blueprints:** [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md:59), [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md:100), [`technical-context.md`](../../blueprints/technical-context.md:29), and [`screenplay-writing-loop.md`](../../blueprints/features/screenplay-writing-loop.md:47).
- **Parent review evidence:** [`design-review.md`](../2026-10-27-toolbar-and-titlebar-polish/reviews/design-review.md:54), especially platform warning [`TTP-DESIGN-W-004`](../2026-10-27-toolbar-and-titlebar-polish/reviews/design-review.md:61).

## Proposed design direction

Keep titlebar/toolbar presentation and viewport commands in the desktop shell. Establish a state machine for normal, maximized, and unknown state; record command direction, backend, and outer-rectangle observations; then choose between the current custom strip, a shared title/toolbar surface, or native titlebar configuration. Treat `Maximized(bool)` as a request and restoration as backend evidence.

Unify adjacent painting through one surface/fill/stroke contract while retaining a full-width title response and centered measured galley. Instrument panel/content/clip rectangles so seam removal does not change title hit coverage or add an inset/height regression.

Measure icon/text bounding boxes and document font/page scale over sampled transition frames. If no transition-progress or animation-disable API exists, define the strongest supported invariant and explicitly report the native limitation.

For Elements, resolve `128.0` points once and preserve control budget/truncation. Prefer an installed egui-supported closed-state alignment mechanism that does not affect popup layout; otherwise isolate a custom closed presentation from the popup and prove arrow, keyboard/open state, truncation, width, and left-aligned choices.

## Likely work-order decomposition

The user confirmed formalization of the following sequential work orders. The filenames are final and preserve one independently reviewable outcome per file:

1. **[`WO-001-viewport-state-direction-and-native-restore-contract.md`](work-orders/WO-001-viewport-state-direction-and-native-restore-contract.md)** — first because all later title-surface work depends on a confirmed known/unknown viewport direction and native restore boundary.
2. **[`WO-002-unified-titlebar-toolbar-surface-and-transition-geometry.md`](work-orders/WO-002-unified-titlebar-toolbar-surface-and-transition-geometry.md)** — second because shared chrome geometry and transition measurements must settle before the Elements control is judged in its final surface.
3. **[`WO-003-elements-closed-selected-text-centering.md`](work-orders/WO-003-elements-closed-selected-text-centering.md)** — third because it changes only the closed selected display while consuming the preserved 128-point allocation and surface geometry.
4. **[`WO-004-cross-surface-native-validation-and-document-reconciliation.md`](work-orders/WO-004-cross-surface-native-validation-and-document-reconciliation.md)** — last because it reconciles the integrated evidence and performs real implementation only if a bounded defect remains; it is not an administrative shortcut.

The direct translation preserves the explicit limitations: no unsupported animation-duration/control promise, no universal exact frame-history claim, no full-screen, and no filename plumbing. The current observed 128-point Elements width is preserved as a baseline, not treated as a change request.

Every future work order must classify requirements/blueprint impact (`update`, `verify unchanged`, or `not applicable`) with evidence and receive a separate fresh validator.

## Risks and unresolved platform/API decisions

- **Risk:** `ViewportInfo::maximized` is stale/unknown on macOS. **Mitigation:** record `Option<bool>`, outer/inner rects, command output, and backend; never infer from prior command alone.
- **Risk:** Native unmaximize does not restore the exact user frame. **Mitigation:** label exact restoration only when observed; do not invent frame history.
- **Risk:** Full-size content and custom panels double-count insets or create seam. **Mitigation:** compare panel/content/clip rectangles and configuration variants.
- **Risk:** Width-derived scale changes during animation. **Mitigation:** define a measurable metric invariant and bounded fallback; exclude unsupported animation-duration/interpolation control.
- **Risk:** Centered selected text collides with arrow/truncation or changes popup alignment. **Mitigation:** measure at `128.0`, test long labels, keyboard/open state, narrow width, and popup choice alignment.
- **Decision needed:** Exact prior outer-frame guarantee versus native restore where observed. **Owner:** user/design reviewer after API/native evidence.
- **Decision needed:** Native titlebar configuration versus custom strip. **Owner:** viewport implementer/validator.
- **Decision needed:** egui 0.29 closed-control technique. **Owner:** Elements implementer after installed-source inspection.
- **Deferred:** Fresh Git status/diff, branch creation, design approval, implementation, and native smoke.

## Design readiness checklist

- [x] Standard mode metadata and draft-only boundary are recorded.
- [x] Closed parent and observed post-closeout follow-up rationale are linked.
- [x] Current source lines, requirements, blueprints, prior work orders, validations, and API evidence are linked.
- [x] One outcome, scope/exclusions, validation boundary, risks, and likely work orders are recorded.
- [x] The `128.0`-point Elements baseline is recorded.
- [x] Unsupported animation-control and exact-restore promises are excluded.
- [x] User confirms convergence and requests formal work-order preparation.
- [x] Work orders exist with durable-document classifications.
- [x] Design review and human approval exist; all four work orders are implemented and freshly validated with native-display warnings recorded.

## Current state and handoff

This initiative is in `closeout` after all four work orders passed fresh validation. Native display/window smoke was unavailable and remains an explicit follow-up rather than a success claim. Commit, merge, and terminal status evidence are recorded in the execution log.

## Closeout record

- **Closeout status:** `pending final commit and merge`
- **Validation outcome:** All four work orders passed fresh validation; native-display limitations remain non-blocking warnings under project policy. No fix work orders were required.
- **Follow-up:** Repeat native macOS smoke for maximize/restore direction, seam pixels, transition-frame metrics, closed/open Elements alignment, cursor, and keyboard behavior when a usable display is available.

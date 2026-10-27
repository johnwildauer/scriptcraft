# Initiative: Toolbar and Title-Bar Polish

## Initiative metadata

- **Phase:** `closeout`
- **Status:** `closed`
- **Owner/orchestrator:** Forge architect/planner
- **Created:** `2026-10-27`
- **Updated:** `2026-10-27`
- **Authoritative version:** `v1`
- **Initiative mode:** `standard`
- **Mode requested by:** `user`
- **Mode confirmation:** The user explicitly requested the standard Forge path and then confirmed direct translation into the four ordered work orders; no rapid path was requested.
- **Rapid-mode eligibility:** `not eligible` — the initiative combines toolbar allocation/presentation, native viewport interaction, pointer affordances, and new-document semantic initialization, with separate implementation and validation boundaries.
- **Base/default branch:** `master` per [`project-manifest.md`](../../project-manifest.md:63); the current checkout branch was not re-verified in this planning-only session.
- **Initiative branch:** `dev/2026-10-27-toolbar-and-titlebar-polish`, created from `master` at `f5b02d3eb58464f091c57667711f3be738e65e56` before WO-001 source changes.
- **Branch policy:** `required` when local branch capability is confirmed.
- **Branch naming convention:** `dev/2026-10-27-toolbar-and-titlebar-polish`
- **Branch capability/evidence:** Local branch creation succeeded before implementation; exact command evidence is recorded in the execution log and WO-001.
- **Reduced-capability fallback or opt-out evidence:** None. If branch capability remains unavailable, the orchestrator must record the manifest's reduced-capability fallback before implementation.
- **Branch creation failure:** `none recorded`
- **Validation cadence:** `per work order`
- **Phase transition evidence:** `draft → plan` was recorded after the four ordered work orders were created; `plan → review` was reviewed with outcome `pass with warnings`; the user-approved initiative was dispatched into `execute` after branch creation; `execute/validate → closeout` followed four completed work orders and fresh validation evidence. Commit and merge evidence is recorded in the execution log.
- **Direct-translation confirmation:** `confirmed by user on 2026-10-27` — after the broad prompt was translated into four independently reviewable implementation work orders, the user explicitly approved the decomposition and the platform-bounded title restoration contract. This satisfies the broad-prompt confirmation requested by [`02-initiative.md`](../../process/02-initiative.md:159).

## Standard artifact locations

- **Authoritative plan:** `toolbar-and-titlebar-polish-v1.md`
- **Design review:** [`reviews/design-review.md`](reviews/design-review.md)
- **Validations:** `validations/validation-[work-order-id].md`
- **Execution log:** [`execution-log.md`](execution-log.md)
- **Work orders:** [`work-orders/`](work-orders/) — [`WO-001-toolbar-geometry-hover-and-elements-width.md`](work-orders/WO-001-toolbar-geometry-hover-and-elements-width.md), [`WO-002-custom-title-strip-and-viewport-toggle.md`](work-orders/WO-002-custom-title-strip-and-viewport-toggle.md), [`WO-003-toolbar-and-document-cursor-ownership.md`](work-orders/WO-003-toolbar-and-document-cursor-ownership.md), [`WO-004-new-document-scene-heading-and-cross-surface-checks.md`](work-orders/WO-004-new-document-scene-heading-and-cross-surface-checks.md)

## Outcome

Make the ScriptCraft desktop writing surface feel like one coherent native-like workspace: the toolbar and custom title strip are visually compact and predictable, title-strip double-click toggles maximization without losing the user's prior frame, pointer affordances distinguish command controls from the document surface, and a new document begins at Scene Heading while existing role progressions remain unchanged.

## Initiative boundary and sizing

- **Parent roadmap or product goal:** The dependable desktop screenplay writing loop in [`product-overview.md`](../../requirements/product-overview.md:52), specifically the desktop command-toolbar requirement [`REQ-WRITING-LOOP-005`](../../requirements/features/screenplay-writing-loop.md:90).
- **Initiative type:** `product MVP stage`
- **Single observable outcome:** In a window-capable run of the current app, a writer sees a compact toolbar/title strip, can double-click the custom title strip to maximize and restore the prior user frame, sees the standard arrow over toolbar/Elements controls and an I-beam over the document surface, and starts a new document on an empty Scene Heading without changing established transitions.
- **Validation boundary:** Fresh design review followed by separate fresh validation per future work order: deterministic egui/model tests for allocation, cursors, viewport-command state, and role initialization; bounded native smoke for title-strip double-click/maximize/restore, title text, toolbar hover geometry, and document cursor. No validation is performed or claimed by this research artifact.
- **Split assessment:** `keep as one initiative` — all six outcomes are user-visible polish of one desktop writing workspace and share the title/toolbar/editor interaction boundary. The intended work orders remain independently reviewable and sequential.
- **If split, traceability and dependencies:** Not applicable. A future split would need to preserve the shared native title-strip dependency and the new-document/cursor validation relationship.

## Scope

### In scope

- Replace broad/boxy toolbar hover presentation with the egui spacing, padding, layout, and response primitives that match the existing centered Item renderers, while preserving click ownership and label accessibility.
- Add per-Item width overrides to the shared allocator and corresponding renderers without double-counting row padding, gaps, overflow reservation, or actual clipped available width. Establish an evidence-based Elements width, with approximately `92.0` points as the initial user-authorized target if runtime measurements support it, so role labels are not broadly truncated.
- Add a dedicated custom title strip, likely approximately `30.0` points high if that remains compatible with the existing vertical geometry, with centered bold `ScriptCraft` placeholder text.
- Restore native-like title-strip double-click behavior: toggle maximized state and restore the prior user frame, using the installed eframe/egui viewport APIs or the smallest compatible stateful adapter.
- Set the standard arrow cursor over toolbar Items and Elements text, and the I-beam cursor over the interactive document editing surface.
- Initialize new documents with one empty Scene Heading and preserve the existing role transition/progression policy for subsequent edits.
- Add or revise the narrowly implicated requirements/desktop blueprint wording only when future implementation evidence changes the durable contract; otherwise explicitly classify documents as verified unchanged.

### Out of scope

- New toolbar commands, toolbar customization or persistence, plugin registries, command catalogs, icon-library replacement, or broad theme redesign.
- Native OS title-bar color/pixel equality, platform-specific decoration guarantees, or a claim that unsupported platforms behave like macOS.
- Changing screenplay role semantics, Enter/Tab matrices, uppercase policy, parenthetical provenance, persistence schema, exports, page geometry, pagination, or editor rendering beyond cursor ownership.
- Filename/session-title plumbing: `ScriptCraft` is only a centered placeholder; active filename display is future work.
- Window snapping, multi-window management, keyboard maximize shortcuts, drag-region redesign beyond the dedicated title strip, or arbitrary frame-history persistence across process restarts.
- Any application-source modification during this planning initiative.

## Context and motivation

Several closed toolbar initiatives established a typed transient toolbar, exact runtime label measurement, icon-only hover ownership, actual clipped-content width budgeting, an explicit app-chrome frame, and hidden-title/full-size-content viewport hints. This follow-up must treat those artifacts as history and current source as the authority, not assume that predecessor plans are complete runtime contracts. The requested six outcomes now connect the remaining visual geometry, title-strip interaction, pointer semantics, and new-document starting-role behavior into one bounded workspace polish pass.

The user also resolved the previously blank “4” clarification by later authorizing outcomes 4–6: pointer cursors, Scene Heading initialization, and the centered `ScriptCraft` title placeholder. This plan records that clarification as resolved rather than reopening it as an unknown.

## Exploration scratchpad

### Initial findings

- **Current toolbar model and allocation:** [`ToolbarItem`](../../apps/scriptcraft/src/main.rs:80) stores stable identity, label, icon, and typed kind. [`default_toolbar_items()`](../../apps/scriptcraft/src/main.rs:148) preserves the approved New/Open/Save/Save As/Flexible Space/Elements/Flexible Space/Export PDF order. [`toolbar_item_width()`](../../apps/scriptcraft/src/main.rs:173) currently applies one normal-width floor plus measured label padding, with Flexible Space and Space special cases; it has no per-item override.
- **Shared width path:** [`allocate_toolbar()`](../../apps/scriptcraft/src/main.rs:347) computes fixed width, row padding, inter-item gaps, flexible-space shares, and overflow reservation. [`toolbar()`](../../apps/scriptcraft/src/main.rs:735) reuses the allocation but manually emits row padding/gaps and passes each width to the renderer. [`toolbar_actual_available_width()`](../../apps/scriptcraft/src/main.rs:215) already intersects the content and clip rectangles, so width overrides must preserve that edge budget rather than bypass it.
- **Elements renderer:** [`elements_toolbar_item()`](../../apps/scriptcraft/src/main.rs:669) constrains the child UI and ComboBox to the passed width and truncates selected text. The current width is still derived from the generic width helper, so a per-item Elements override must be applied at both allocation and rendering boundaries and tested against the existing ComboBox rectangle ledger in [`toolbar_combo_boundary_ledger()`](../../apps/scriptcraft/src/main.rs:246).
- **Current hover/cursor behavior:** [`toolbar_action_button()`](../../apps/scriptcraft/src/main.rs:604) owns an explicit icon-plus-padding response and currently applies `CursorIcon::PointingHand` at [`main.rs`](../../apps/scriptcraft/src/main.rs:609); this does not satisfy the requested standard arrow. [`elements_toolbar_item()`](../../apps/scriptcraft/src/main.rs:669) does not explicitly set a cursor on the control or label. The document surface has one whole-surface response at [`live_document_surface()`](../../apps/scriptcraft/src/main.rs:1118), making it the natural I-beam cursor owner without adding per-page editors.
- **Current title/viewport state:** [`native_options()`](../../apps/scriptcraft/src/main.rs:1682) requests hidden title text, hidden titlebar, and full-size content via installed eframe 0.29 calls at [`main.rs`](../../apps/scriptcraft/src/main.rs:1684). [`update()`](../../apps/scriptcraft/src/main.rs:1620) currently renders only the toolbar top panel, status bottom panel, and central editor; no custom title strip or title-strip pointer response exists.
- **Current document initialization:** [`ScriptCraftApp::new()`](../../apps/scriptcraft/src/main.rs:449) and New dispatch at [`main.rs`](../../apps/scriptcraft/src/main.rs:578) call [`Project::new()`](../../crates/scriptcraft-core/src/lib.rs:221). [`Project::new()`](../../crates/scriptcraft-core/src/lib.rs:222) currently creates one empty `Role::Action` element at [`lib.rs`](../../crates/scriptcraft-core/src/lib.rs:227). The existing role progression is owned by the editing/session layer, including [`EditingSession::change_role()`](../../crates/scriptcraft-core/src/editing.rs:430) and the Enter/Tab policy, so initialization can be changed without redesigning progression.
- **Current test inconsistency:** [`TOOLBAR_NORMAL_ITEM_WIDTH`](../../apps/scriptcraft/src/main.rs:23) is `56.0` in the current source snapshot while [`toolbar_manual_geometry_controls_are_narrow_and_explicit()`](../../apps/scriptcraft/src/main.rs:1932) expects `46.0`. This must be reconciled by a future implementation/validation session rather than silently treating either value as the intended new baseline.
- **History findings:** The closed [`toolbar-interaction-polish-v1.md`](../2026-10-27-toolbar-interaction-polish/toolbar-interaction-polish-v1.md:52), [`toolbar-exact-width-hover-v1.md`](../2026-10-27-toolbar-exact-width-hover/toolbar-exact-width-hover-v1.md:23), [`toolbar-viewport-hover-v1.md`](../2026-10-27-toolbar-viewport-hover/toolbar-viewport-hover-v1.md:23), and [`toolbar-final-fit-v1.md`](../2026-10-27-toolbar-final-fit/toolbar-final-fit-v1.md:24) record predecessor boundaries and repeated native-smoke limitations. The current source, not their closeout claims, remains the implementation baseline.

### Questions and hypotheses

- **Viewport API hypothesis:** eframe/egui can expose maximized state and dispatch a viewport maximize command, but restoring the prior user frame may require capturing a pre-maximize outer position/size from viewport input/output or using platform/native frame behavior. The exact installed 0.29 API and supported macOS semantics must be verified before work-order authorization.
- **Title strip hypothesis:** A separate top panel above the toolbar provides a stable double-click hit region and keeps title text independent from toolbar allocation, but its height changes the available vertical layout and must not be mistaken for toolbar top padding.
- **Cursor hypothesis:** An explicit response-level `.on_hover_cursor(CursorIcon::Default)` for toolbar/Elements and `.on_hover_cursor(CursorIcon::Text)` for the whole document response should be sufficient; native cursor rendering still needs a window-capable smoke check.
- **Elements width hypothesis:** Around `92.0` points should prevent broad truncation for the seven role labels while preserving wide/narrow fit behavior, but active font metrics, ComboBox arrow/padding, and the current row budget must determine the final value.

### Options considered

| Option | Benefits | Costs/risks | Current direction |
|---|---|---|---|
| Keep one generic toolbar width and widen only the row | Small change | Cannot express Elements needs without distorting all Items; likely repeats truncation/overflow defects | Rejected |
| Add an optional per-Item width override consumed by allocator and renderer | Preserves one shared budget and permits an Elements-specific width | Requires explicit default/fallback semantics and regression coverage | Selected |
| Treat the existing hidden native titlebar as the title strip | Fewer egui widgets | No portable custom text/double-click owner and weak restore semantics | Rejected |
| Add a dedicated egui title strip above the toolbar | Stable hit region, placeholder text, and explicit interaction owner | Must reconcile viewport insets and vertical geometry | Selected provisionally |
| Toggle maximized only and let the platform restore size | Minimal viewport command | Does not prove restoration of the prior user frame | Rejected for the requested outcome |
| Rebuild window management or persist frame history | More control | Scope expansion and platform coupling | Rejected |

### Decisions and discussion notes

- The initiative remains `standard`, guided, and research-only. No implementation or design approval is implied by this plan.
- The six user-authorized outcomes are treated as one observable workspace outcome, not six independent initiatives.
- The blank-4 clarification is **resolved** by the user's later tasks 4–6: standard arrow/I-beam cursors, Scene Heading initialization, and centered bold `ScriptCraft` placeholder text.
- The title placeholder must remain exactly a placeholder; it must not read or mutate the active filename.
- Native visual limitations must be reported as unavailable evidence, not converted into success claims.

### Next investigation or conversation focus

- The installed eframe/egui 0.29.1 API is verified: [`ViewportInfo`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/data/input.rs:194) exposes optional `maximized`/`outer_rect`, and [`ViewportCommand::Maximized(bool)`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/viewport.rs:948) exists. It does not establish a universal exact prior-frame restoration API, so WO-002 uses a platform-bounded contract.
- Final Elements width remains an implementation measurement decision. Approximately `92.0` points is a target only; WO-001 must record the active-font/control measurement and narrow-width consequences.

## Requirements and blueprint references

- **Requirements:** [`REQ-WRITING-LOOP-001`](../../requirements/features/screenplay-writing-loop.md:38), especially [`AC-WRITING-001.1`](../../requirements/features/screenplay-writing-loop.md:44) for a valid new starting element and [`AC-WRITING-001.4a`](../../requirements/features/screenplay-writing-loop.md:48) for preserving role progressions; [`REQ-WRITING-LOOP-005`](../../requirements/features/screenplay-writing-loop.md:90), especially [`AC-WRITING-005.3`](../../requirements/features/screenplay-writing-loop.md:104) and [`AC-WRITING-005.5`](../../requirements/features/screenplay-writing-loop.md:115).
- **Blueprints:** [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md:35) for desktop ownership, toolbar composition, viewport presentation, and editor response boundaries; [`screenplay-core.md`](../../blueprints/components/screenplay-core.md:53) for semantic ownership and transactional editing; [`screenplay-writing-loop.md`](../../blueprints/features/screenplay-writing-loop.md:78) for the feature composition; [`technical-context.md`](../../blueprints/technical-context.md:29) for the desktop-shell/core topology.
- **Likely durable impact:** Toolbar/title presentation and pointer behavior will likely require a narrowly scoped update or explicit verification of the desktop blueprint and [`AC-WRITING-005.3`](../../requirements/features/screenplay-writing-loop.md:104). Per-item allocation may clarify [`AC-WRITING-005.5`](../../requirements/features/screenplay-writing-loop.md:115). Scene Heading initialization affects [`AC-WRITING-001.1`](../../requirements/features/screenplay-writing-loop.md:44); existing progression requirements should be verified unchanged. Viewport behavior must remain platform-bounded and should update the desktop blueprint only with evidence-backed API semantics. No core blueprint change is intended unless the initialization ownership is moved into the core contract.

## Proposed design

Keep all presentation and viewport interaction in the desktop shell. Extend the existing typed toolbar metadata with an optional width override or equivalent explicit width policy. Make the allocator, row-width accounting, overflow reservation, and action/Elements renderers consume the same resolved width. Preserve the current ComboBox boundary constraints and add the Elements width only after measuring actual selected-label/control extents.

Render a dedicated title strip above the toolbar using a stable egui response. Paint centered bold `ScriptCraft`, attach double-click detection to that response, and use viewport commands/state to toggle maximized mode while retaining enough pre-maximize frame information to restore the user's prior frame. The implementation must define behavior when viewport state is unavailable or the platform ignores a command; it must not claim portable native behavior.

Attach explicit arrow cursor ownership to toolbar Items and Elements text/control responses, and text/I-beam ownership to the existing whole document response. Change the core's new-project seed role from Action to Scene Heading only if the existing transition tests demonstrate that all subsequent progressions remain unchanged; do not introduce a second role policy in the app.

## Intended work-order breakdown

Work-order files are now created for the `plan` phase; no design-review artifact is created by this authoring session.

1. **[`WO-001-toolbar-geometry-hover-and-elements-width.md`](work-orders/WO-001-toolbar-geometry-hover-and-elements-width.md)** — toolbar geometry/hover and one resolved per-item width contract including Elements; preserves actual clipped width, ComboBox budget, overflow, and deterministic regressions.
2. **[`WO-002-custom-title-strip-and-viewport-toggle.md`](work-orders/WO-002-custom-title-strip-and-viewport-toggle.md)** — custom title strip, centered bold placeholder, double-click maximize toggle, and platform-bounded restoration using verified viewport APIs.
3. **[`WO-003-toolbar-and-document-cursor-ownership.md`](work-orders/WO-003-toolbar-and-document-cursor-ownership.md)** — standard arrow ownership for toolbar/Elements and I-beam ownership for the existing whole-document response without page-level editors.
4. **[`WO-004-new-document-scene-heading-and-cross-surface-checks.md`](work-orders/WO-004-new-document-scene-heading-and-cross-surface-checks.md)** — new-document Scene Heading seed, regression coverage, and explicit final cross-surface/project-policy checks tied to the seed outcome. No separate administrative reconciliation WO is created.

**Decomposition rationale:** The requested six user-visible outcomes consolidate into four independently reviewable implementation boundaries because toolbar geometry/hover and Elements width share one allocator/renderer contract, while title text and viewport interaction share one title-strip response. Cursor ownership is separate from both because it must preserve existing focus/event ownership. Scene initialization remains a core-owned semantic boundary and carries the final cross-surface checks so the research plan's administrative fifth item is not an implementation WO.

## Risks and decisions

- **Risk:** The installed eframe version may not expose enough viewport frame state to restore an exact prior user frame. **Mitigation:** verify the installed API first; define the closest supported restore contract explicitly, or mark the requirement blocked rather than claiming it.
- **Risk:** Adding a title strip can double-count native/full-size-content insets or toolbar vertical padding. **Mitigation:** instrument panel/content/clip rectangles and keep title-strip height separate from toolbar row geometry.
- **Risk:** Per-item widths can reintroduce the prior Elements intrinsic ComboBox overrun. **Mitigation:** resolve width once, pass it to allocation and renderer, retain `ComboBox::width()`/truncate boundary, and assert actual response/group containment.
- **Risk:** A 92-point Elements width can force earlier overflow at narrow widths. **Mitigation:** make it an explicit override, preserve leftmost-fit/far-right overflow policy, and validate wide/exact/narrow cases.
- **Risk:** Cursor APIs may differ between response owner and painted text. **Mitigation:** assign cursor on the owning response, keep decorative labels non-competing, and require native smoke before claiming visible cursor behavior.
- **Risk:** Changing `Project::new()` can unintentionally alter existing role transitions or fixtures. **Mitigation:** add a focused seed-role regression and run the full existing editing matrix; do not change transition logic.
- **Resolved planning decision:** The installed eframe/egui API exposes optional maximized/outer-rectangle state and a maximize command, but no universal exact prior-frame restore API. **Execution contract:** WO-002 must use the safest observable/native-supported restore behavior and must not claim exact restoration beyond the tested platform/backend. **Owner:** WO-002 implementer, reviewed by design review.
- **Decision needed:** Final Elements width and whether approximately 92 points is sufficient under the active style/font. **Owner:** future toolbar work-order implementer, evidenced by galley/control measurements and overflow tests.
- **Deferred:** Native-window visual/cursor smoke is not available in this planning session and is not claimed.

## Design readiness checklist

- [x] Scope and exclusions are explicit.
- [x] The initiative has one observable outcome and one named validation boundary.
- [x] The initiative is traceable to the product goal and relevant requirements/blueprints.
- [x] The split assessment and intended dependency order are recorded.
- [x] Current source/history findings are recorded with file/line links.
- [x] Standard mode metadata and per-work-order validation cadence are recorded.
- [x] Blank-4 clarification is recorded as resolved by later user-authorized tasks 4–6.
- [x] Installed eframe/egui viewport state/command API is verified; exact universal native frame restoration is explicitly bounded rather than claimed.
- [x] Four ordered work-order files exist; no design-review artifact was created.
- [x] Human design approval, implementation, and per-work-order validation are recorded; native-window smoke remains a non-blocking documented warning.

## Current state and handoff

This initiative is in `closeout` after all four work orders passed fresh validation. Native-window smoke was unavailable and remains an explicit follow-up rather than a success claim. Commit, merge, and terminal status evidence are recorded in the execution log.

## Closeout record

- **Closeout status:** `closed`
- **Closeout date:** `2026-10-27`
- **Closeout evidence:** Implementation commit `449f7e6` was created on `dev/2026-10-27-toolbar-and-titlebar-polish`; merge commit `935c162` merged that branch into `master`. The final checkout is clean on `master`.
- **Validation outcome:** WO-001 and WO-002 passed with non-blocking native-smoke warnings; WO-003 and WO-004 passed with the same documented limitation. No fix work orders were required.
- **Follow-up:** Repeat native-window smoke for toolbar hover geometry, title-strip maximize/restore, arrow/I-beam cursor visibility, and new-document presentation on a window-capable macOS environment. Owner: Forge orchestrator; trigger: next native desktop validation session.

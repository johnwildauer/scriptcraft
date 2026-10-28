# Work Order: WO-003 — Transient Zoom and Integrated Regression

## Metadata

- **Status:** `draft`
- **Assignee:** Desktop application implementer (one fresh session; not dispatched)
- **Phase/order:** 3 of 3; depends on WO-001 and WO-002; final integrated validation boundary
- **Initiative:** [`status-bar-v1.md`](../status-bar-v1.md)
- **Generated filename:** [`WO-003-transient-zoom-and-integrated-regression.md`](WO-003-transient-zoom-and-integrated-regression.md)
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`validation-WO-003.md`](../validations/validation-WO-003.md) (future artifact; not created)
- **Fixes:** none
- **Implementer session/agent ID:** Not assigned; no dispatch authorized
- **Session exception:** none

## Summary

Add one transient page-view zoom state, synchronized slider/preset controls in the status bar's right region, and usable horizontal editor navigation for wide pages. Prove that zoom remains presentation-only and integrate the full toolbar/editor/status/page/zoom/error regression matrix in this third and final WO; do not create a fourth integration WO.

## In scope

- Add one authoritative, transient integer-percent page zoom state bounded inclusively from 75 through 300. It is app/window presentation state only; do not serialize it in `Project`, native files, page profile, or preferences.
- Define 100% as the current existing window-fit baseline from [`presentation_geometry()`](../../../../apps/scriptcraft/src/main.rs); the fit baseline is recomputed as the available editor width changes, so effective physical/logical page scale may change on resize while the selected percentage remains accurate.
- Render an apparently continuous slider from 75% to 300%, update in whole-percent increments/rounding, fill the track blue up to the value and show a circular thumb. Its visible/accessibility value accurately reports intermediate integer percentages and both endpoints.
- Render an adjacent clickable percentage control/menu with exactly these nine entries: `75%`, `90%`, `100%`, `125%`, `150%`, `175%`, `200%`, `250%`, `300%`. Selecting a preset updates the same authoritative zoom state; dragging/keyboard-operating the slider updates the displayed percentage/control immediately. Intermediate percentages remain displayed accurately even when not a preset.
- Apply the selected factor only to page/editor presentation geometry and painting. Preserve the existing 100% fit result. Do not call app-global `Context::set_zoom_factor` or otherwise scale toolbar, status bar or global egui controls.
- Support horizontal navigation for pages wider than the editor viewport while retaining vertical multi-page navigation, pointer hit testing, source mapping, editor response identity, keyboard/IME focus and conditional caret reveal. Include both directions as the UI/available width changes.
- Keep the WO-002 visible-page value current across zoom, horizontal/vertical scroll and resize, respecting explicit unavailable-on-stale behavior; never replace it with caret page.
- Provide deterministic bounded narrow-window fallback that keeps status/error/page/zoom functions reachable; do not let the zoom control obscure the toolbar, editor focus or operation errors.
- Add comprehensive cross-feature regression coverage: existing toolbar extraction/preservation; item interactions and status; page selection/currentness; zoom/presets/intermediate values, focus and navigation; save/dirty/error/undo behavior; and fixed layout/source-map/PDF invariants.

## Out of scope

- New toolbar commands, status-bar actions/icons, document semantics, core APIs, changes to saved native schema/page profile, persistent user preferences or broad theme changes.
- Changing fixed pagination, source mapping, PDF print layout, save/open/dirty semantics, editor focus/IME/caret behavior, titlebar or unrelated application surfaces.
- Altering the page-selection rule or weakening WO-001/002 acceptance to accommodate the slider.
- A separate/fourth cross-feature test or integration work order; all integrated cross-feature checks belong here.

## Requirements

Applicable existing requirements (verbatim):

- **REQ-WRITING-LOOP-002 / AC-WRITING-002.1:** “Given the same project and fixed page profile, the system shall derive stable page structure and source mappings independent of window zoom.”
- **REQ-WRITING-LOOP-004 / AC-WRITING-004.1:** “Given a project, PDF export shall use fixed print layout rather than transient viewport layout.”
- **REQ-WRITING-LOOP-003 / AC-WRITING-003.1:** “Given a valid project, the system shall serialize a versioned native representation containing semantic content, stable IDs, metadata, and page-profile inputs, while excluding transient caret/layout/undo state from required content.”
- **NFR-WRITING-001:** “Production paths should prefer typed errors and bounded inputs over panic-prone handling; a discovered crash or data-loss defect requires a regression test.”
- **NFR-WRITING-002:** “Core editing, layout, and persistence behavior shall be exercisable without a desktop window for deterministic tests.”

Initiative-confirmed zoom contract proposed for a new/additive future requirement: page-only transient zoom 75–300%, responsive fit at 100%, exact synchronized nine presets, accurate intermediate values, horizontal navigation, no persistence/dirty-state or fixed page/source/PDF changes. Toolbar extraction/preservation criteria from WO-001 and status-item, viewport-page, error/path/diagnostic, narrow-width and control-accessibility criteria from WO-002 remain in force.

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) — desktop owns transient viewport presentation, status controls, editor scrolling/input and fixed-layout delegation; update only zoom/horizontal-navigation/integration contracts.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) — shared fixed-profile page/source-map/PDF invariants; verify unchanged.
- [`technical-context.md`](../../../blueprints/technical-context.md) — desktop/core boundary and presentation-only scaling; verify unchanged.
- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) — semantic/session/layout/persistence contracts; verify unchanged.

## Implementation plan

1. Re-read approved WO-001/002, current `presentation_geometry()`/`presentation_metrics()`, page renderer, vertical `ScrollArea`, `ScreenplayLayout` cache, caret/hit-test transforms, PDF exporter and dirty/save paths. Confirm the feature stays in the app shell and current eframe/egui APIs before editing.
2. Add one transient percent state initialized to 100; centralize clamping and factor derivation. Ensure slider, preset menu, displayed percentage, page metrics and accessibility value all read/write this same state.
3. Integrate custom page-only scale with the existing 100% fit metric and responsive width; add a two-axis navigation path for overflow pages without changing semantic coordinate units, layout cache keys, project or document input/focus behavior. Keep all forward/inverse pointer/caret geometry transforms consistent.
4. Add deterministic tests for slider value mapping, 75/300 endpoints, representative and every preset, intermediate values, both update directions, accessibility/keyboard and focus retention; geometry/navigation and before/after layout/PDF/save-state invariants; and the full combined preservation matrix from WO-001/002.
5. Update the requirement and desktop blueprint only within the authorized scope, verify all other durable documents unchanged, and run focused plus full configured project checks. The final implementation and validator evidence remain separate sessions.

## Files and systems

- **Create:** Focused app zoom/control/geometry/navigation tests; implementation may use bounded status-bar/editor helpers in the modules established by WO-001.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs) and the approved status-bar module for app state/wiring; editor presentation/scrolling only as required; [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md); [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md).
- **Avoid changing:** `crates/scriptcraft-core/`, `Cargo.toml`/`Cargo.lock`, serialized model/profile, exports/PDF implementation, fixed layout/source maps, and unrelated toolbar/editor behavior.

## Validation and E2E acceptance tests

- **Automated checks:** [`cargo fmt --all -- --check`](../../../../Cargo.toml:1); `cargo test -p scriptcraft zoom_`; `cargo test -p scriptcraft status_bar_`; `cargo test -p scriptcraft visible_page_`; `cargo test -p scriptcraft toolbar_`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`. These are future commands, not results from this planning session.
- **Zoom value/control matrix:** Test min 75, max 300, every integer through the mapping/clamping helper, rounding at fractional slider positions, representative intermediate values not in the preset list, every one of the exact nine presets, menu-to-slider and slider-to-label synchronization, reverse direction, keyboard increments/home/end where supported, accessible name/current value, pointer dragging, and focus retention. Assert preset count/content/order exactly; no added option.
- **Geometry/navigation invariants:** At 100%, match the existing fit geometry at wide, ordinary and narrow widths; on resize keep 100% selected and derive new fit baseline. At 75%, intermediate and 300%, page width/font/rendered coordinates scale coherently and two-axis navigation reaches left/right and top/bottom content. Assert no global toolbar/status scale. Hit testing, caret display/reveal, selection and source ranges map to the same semantic `Position` at different zooms. Fixed layout/page count/source ranges and cache key stay invariant; PDF output/geometry stays fixed; project JSON excludes zoom; zoom-only changes do not change session revision, saved content, `dirty()` or undo/redo state.
- **Integrated regressions:** Re-run preservation tests for toolbar order/width/flexible space/overflow/action/Elements routing/hover/cursor/focus; item-based status capabilities, clean vs unsaved, cumulative frames, path/status/error and privacy-safe diagnostic disclosure; visible page independent of off-screen caret, overlap/tie/gap/title/failure and fresh geometry after scroll/resize/zoom/reflow/reveal; editor pointer/keyboard/IME focus, caret reveal, selection and page interaction; successful/failed save/open/export feedback, failed-operation state preservation, dirty/clean transitions and undo/redo. Repeat status/error accessibility and page/zoom reachability at narrow widths. No failure may be hidden behind a preset, disclosure, overflow or horizontal/vertical scroll state.
- **Manual/external checks:** Recommended bounded `cargo run -p scriptcraft` desktop smoke at normal and narrow sizes. Exercise toolbar overflow and actions; title and body pages; off-screen caret while scrolling; status clean/dirty, save, operation failure/error and diagnostics; slider visual blue track/circular thumb and both directions; all nine presets plus intermediate value; resize at 100%; zoomed wide-page horizontal and vertical navigation; keyboard tab/focus/activation and editing after controls. If no real window/input is available, document that limitation without claiming native pass. Project policy has no configured manual hard-stop; this does not waive automated criteria or a future configured gate.
- **Acceptance test:** Given 100% and current viewport width, when the app lays out, then page presentation equals existing window-fit baseline; given any slider integer 75–300 or one of the exact nine menu presets, when changed, then the same transient state and both controls agree and intermediate values display correctly; given pages wider than the viewport, when navigated, then all page content remains reachable without losing vertical navigation or editor source mapping; and across any zoom/resize/save/error/undo path, fixed pagination/source maps/PDF, saved content, dirty state, toolbar behavior, status errors and editor input/focus remain correct and covered by automated tests.
- **Independent validator boundary:** A separate fresh-context validator must compare the authorized work order/review, current source and comparable diff, all integrated tests and durable-document impact, and independently rerun configured format/check/test/build/Clippy/diff commands. This WO's implementer evidence is not a substitute. The validator records unavailable native smoke honestly and does not claim it passed.

## Durable-document impact

- **Requirements impact:** `update` — selected: add a user-visible transient zoom criterion for range, 100% responsive baseline, exact presets, synchronization, intermediate values, horizontal access and non-effect on document/pagination/PDF to [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md); retain the existing fixed-layout/PDF requirements verbatim.
- **Blueprint impact:** `update` — selected: document desktop-owned one-state page-only zoom, fit baseline, horizontal scrolling and invariants in [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md).
- **Affected documents/IDs:** `REQ-WRITING-LOOP-002 / AC-WRITING-002.1`, `REQ-WRITING-LOOP-004 / AC-WRITING-004.1`, `REQ-WRITING-LOOP-003 / AC-WRITING-003.1` (preserve verbatim); proposed additive zoom criterion under `screenplay-writing-loop.md` with stable ID assigned during implementation/review; desktop zoom/presentation contract in `scriptcraft-desktop.md`.
- **Rationale and evidence:** Existing requirements and feature/technical blueprints already require window-zoom-independent fixed page/source geometry and fixed-layout PDF. Current app has a width-derived fit baseline but no user zoom or horizontal editor navigation; only the desktop container needs new transient presentation behavior. Core/feature/technical contracts remain unchanged if no semantic/layout/persistence/PDF code changes.
- **Authorized documentation changes:** After review and execution authorization only, add the scoped user-visible requirement and desktop contract. Record feature, technical and core blueprints as verified unchanged. Do not change page profile, persistent schema or PDF contract. No durable document is modified now.

## Completion checklist

- [ ] Implementation is complete within scope.
- [ ] Tests/checks and required gates have evidence.
- [ ] Durable-document impact classification is accurate against the diff, and authorized documentation updates are applied.
- [ ] Execution log is appended with dispatch/completion session identifiers; fresh independent validation artifact exists.
- [ ] Work order is `in-review` with evidence; completion comes only from passed validation.

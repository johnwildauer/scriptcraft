# Work Order: WO-002 — Unified Title-Bar/Toolbar Surface and Transition Geometry

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop shell geometry implementer (one fresh session)
- **Phase/order:** 2 of 4; depends on WO-001
- **Initiative:** [`../titlebar-transition-and-elements-alignment-v1.md`](../titlebar-transition-and-elements-alignment-v1.md:1)
- **Generated filename:** `WO-002-unified-titlebar-toolbar-surface-and-transition-geometry.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-002.md`](../validations/validation-WO-002.md:1)
- **Fixes:** none
- **Implementer session/agent ID:** `implementer-fresh-wo002-titlebar-geometry-2026-10-27T1732Z`
- **Session exception:** none

## Summary

Make the custom title strip and toolbar read as one surface while retaining a full-width title hit region, centered title text, and independent toolbar geometry. Investigate whether full-size-content insets and width-derived presentation scale change icon/text/document metrics during native resize, and define only an evidence-backed stability invariant; do not promise animation duration, interpolation, or universal exact frame history.

## In scope

- Reconcile title-strip and toolbar fill, stroke, frame margins, panel spacing, ownership, and seam pixels.
- Preserve title-strip double-click coverage and centered `ScriptCraft` placeholder geometry from WO-001.
- Instrument deterministic panel/content/clip rectangles and title/toolbar heights to prevent inset or vertical-budget regressions.
- Measure icon/text bounds and [`presentation_geometry()`](../../../../apps/scriptcraft/src/main.rs:1509) page/font scale across representative available widths and, where possible, sampled native transition frames.
- Choose the strongest supported content-metric stability behavior, including a bounded fallback if OS-owned resizing changes available width.

## Out of scope

- Animation-duration/interpolation control, disabling or replacing OS animation, full-screen, snapping, native pixel parity, filename plumbing, broad theme redesign, or toolbar command changes.
- Reversing the user’s local [`TOOLBAR_ELEMENTS_WIDTH`](../../../../apps/scriptcraft/src/main.rs:26) change; `128.0` points is the observed baseline and is not a target to undo.
- Elements selected-text alignment, viewport direction logic, semantic editing, persistence, export, and cross-platform decoration guarantees.

## Requirements

Applicable wording copied verbatim from [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90):

- **REQ-WRITING-LOOP-005:** “As an individual screenwriter, I want desktop commands in a consistent toolbar, so common project actions are discoverable without changing the semantic editing model.”
- **AC-WRITING-005.5:** “Given a toolbar width, Space shall consume one deterministic 72-point width, rendered normal Items shall consume their complete group width including label text measured at runtime with the active egui style/font, and Flexible Space Items shall divide the remaining width equally after the shared 4-point horizontal row-padding and 4-point inter-item-gap budget, and a narrow toolbar shall retain the leftmost fitting Items while exposing non-spacing Items through a far-right `>>` text menu. Action entries and Elements role choices in that menu shall remain executable without persisting toolbar layout state; a fully fitting composition shall keep its rightmost Item, including its label, inside the actual clipped toolbar content right edge after panel, frame, and viewport insets are applied. Items with an approved typed width override, including the Elements control, shall use that same resolved width for allocation and rendering without changing the shared padding, gap, flexible-space, overflow, or clipped-edge budget.”
- **AC-WRITING-005.6:** “Given the desktop app is running, a dedicated compact title strip shall display centered bold `ScriptCraft` placeholder text above the toolbar. A double-click shall request maximize/unmaximize only when the viewport's maximized state is known; unavailable or ignored state shall use a bounded no-op/native fallback rather than an unsafe or exact-restoration claim that the platform cannot support.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:59) — the shell owns transient toolbar composition, clipped-edge budgeting, and title-strip presentation.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:100) — full-size-content/titlebar hints and custom title strip are platform-bounded.
- [`technical-context.md`](../../../blueprints/technical-context.md:31) — viewport scale is presentation-only and does not redefine fixed layout.

## Source/API findings

- [`update()`](../../../../apps/scriptcraft/src/main.rs:1697) currently paints separate `title-strip` and `toolbar` top panels.
- [`title_strip_frame()`](../../../../apps/scriptcraft/src/main.rs:1663) and [`toolbar_frame()`](../../../../apps/scriptcraft/src/main.rs:1659) use related but distinct frame helpers, making fill/stroke/margin mismatch a direct seam hypothesis.
- [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:1080) derives scale from available width through [`presentation_geometry()`](../../../../apps/scriptcraft/src/main.rs:1509); native width changes can alter displayed geometry without changing semantic layout.
- [`native_options()`](../../../../apps/scriptcraft/src/main.rs:1763) requests hidden title/titlebar and full-size content; eframe’s installed API cannot promise OS animation control.
- [`TOOLBAR_ELEMENTS_WIDTH`](../../../../apps/scriptcraft/src/main.rs:26) is currently `128.0`, and this WO must preserve it.

## Implementation plan

1. Build a rectangle ledger for title, toolbar, content, clip, and document surfaces and add deterministic assertions for seam-free shared chrome and unchanged hit coverage.
2. Use one explicit surface/fill/stroke contract or the smallest equivalent frame adjustment, retaining separate title response and toolbar row allocation.
3. Measure transition-sensitive icon/text/page metrics at fixed widths and native samples where available; distinguish stable semantic layout from width-derived paint scale.
4. Implement only supported stabilization. If no eframe/winit API controls native animation progress, report that limitation and bound acceptance to measured geometry/tolerance rather than inventing a control promise.

## Files and systems

- **Create:** focused geometry/metric tests only if they isolate pure helpers.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1659) and this WO; blueprint only if verified behavior changes its durable shell contract.
- **Avoid changing:** core layout/persistence/export, viewport command direction, filename/title plumbing, and `TOOLBAR_ELEMENTS_WIDTH` baseline.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; focused geometry/metric tests; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- **Deterministic acceptance:** title and toolbar share the intended chrome surface without a doubled stroke/fill seam; title response remains full-width; title/toolbar heights and toolbar allocation remain bounded; 128-point Elements width and clipped-edge budget remain unchanged; fixed-width metric tests document scale behavior.
- **Manual/native-smoke:** On macOS, record normal/maximized transition samples with outer/inner/content/clip rectangles, title/icon/text bounds, page scale, and seam observations. Record OS-owned animation as a limitation; do not claim universal frame-history or pixel-identical content.
- **Acceptance test:** Given the app at normal and maximized native widths, when the title strip transitions, then title/toolbar chrome remains visually continuous and hit-stable, measured content metrics follow the documented supported invariant, and any OS animation limitation is explicitly recorded.

## Durable-document impact

- **Requirements impact:** `verify unchanged` — [`AC-WRITING-005.5`](../../../requirements/features/screenplay-writing-loop.md:115) and [`AC-WRITING-005.6`](../../../requirements/features/screenplay-writing-loop.md:131) already express clipped allocation and bounded title behavior without an unsupported animation promise.
- **Blueprint impact:** `update` only if implementation changes the durable surface composition; otherwise `verify unchanged` against [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:59).
- **Affected documents/IDs:** REQ-WRITING-LOOP-005 / AC-WRITING-005.5–005.6; desktop shell chrome contract.
- **Rationale and evidence:** Current source confirms separate frame helpers, width-derived presentation scale, and the 128-point baseline. API/source inspection provides no animation-duration or interpolation control.
- **Authorized documentation changes:** If needed, describe the unified app surface and measured platform-bounded metric invariant only; never document exact frame history, full-screen, filename plumbing, or animation control.

## Dependencies, risks, and handoff

- **Dependencies:** WO-001 supplies the viewport state/restore contract; WO-003 must consume the preserved 128-point toolbar baseline.
- **Risks:** full-size-content insets, double-counted panel spacing, width-derived scale changes, and native animation timing can look like regressions even when egui geometry is stable.
- **Handoff:** provide the final surface/geometry ledger and accepted stability limitation to WO-004.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and bounded native evidence have been recorded.
- [x] Durable-document impact classification is accurate against the diff.
- [x] Execution log is appended with fresh dispatch/completion identifiers.
- [x] Work order is `in-review`; `complete` requires fresh validation.

## Implementation evidence

- **Dispatch/completion session:** `implementer-fresh-wo002-titlebar-geometry-2026-10-27T1732Z`; fresh implementer session for WO-002 only. WO-003 and WO-004 were not implemented.
- **Branch/status evidence:** `git status --short --branch` confirmed `dev/2026-10-27-titlebar-transition-and-elements-alignment`; the pre-existing modified [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1) and untracked initiative evidence were preserved. No branch switch or reset was performed.
- **Implementation:** Replaced adjacent title/toolbar panels with one `TopBottomPanel` and one zero-inset shared frame using the existing `Widgets::open::weak_bg_fill`; disabled the panel separator and explicit frame stroke to remove the ownership seam. The title allocation is exactly 30 points and uses the full available width, preserving centered bold `ScriptCraft` and full-width double-click response. Toolbar content remains separately allocated after the title, with existing action routing, cursor behavior, row budget, and `TOOLBAR_ELEMENTS_WIDTH = 128.0` preserved.
- **Geometry/metric evidence:** Added `chrome_geometry()` and deterministic assertions for shared panel ownership, full-width title coverage, contiguous title-to-toolbar boundary, and bounded rectangle tolerance of `0.5` points. Added `presentation_metrics()` as the single runtime metric helper; at widths with enough room for the default 576-point page it preserves page scale/font metrics across representative normal/maximized widths within `0.01` scale/font tolerance. Narrow widths remain width-constrained and are explicitly tested as a bounded limitation.
- **Transition limitation:** Installed eframe 0.29/egui-winit APIs provide viewport requests and optional observed state/rectangles, not animation-progress, duration, interpolation, or pixel-freeze controls. No unsupported animation promise was added. Native macOS window smoke and transition-frame samples were unavailable, so no OS/backend animation or pixel claim is made.
- **Durable documents:** Requirements are `verify unchanged`; existing AC-WRITING-005.5/005.6 already cover toolbar allocation and bounded title behavior. Blueprint impact is `update`: [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:108) now records the durable unified app-surface/frame ownership contract and the same platform-bounded maximize/restore behavior.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:1), this work order, and [`execution-log.md`](../execution-log.md:1). No core crate, requirements file, WO-003, WO-004, or validation artifact was changed.
- **Validation attempts/results:** `cargo fmt --all -- --check`, focused chrome/metric/title-strip tests, `cargo check --workspace`, `cargo test -p scriptcraft` (72 passed), `cargo test --workspace` (72 app tests, 45 core tests, and integration suites passed), `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` all passed. An initial focused command used multiple cargo test filters and was rejected by Cargo argument parsing; the focused tests were rerun as separate valid commands and passed.
- **Comparable diff:** `git diff --stat`, source/blueprint diff inspection, `git status --short --branch`, and `git diff --check` were run. The source diff includes the pre-existing WO-001 baseline reconciliation/state seam plus this WO-002 shared-surface/metric/test change; no WO-003/004 behavior was introduced.
- **Next state:** `complete`; fresh validation passed with warnings. See [`validation-WO-002.md`](../validations/validation-WO-002.md:1).

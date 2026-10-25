# Work Order: WO-001 — Centered Page Viewport

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer after design approval
- **Phase/order:** `1 of 4`
- **Initiative:** [`../ui-quality-of-life-v1.md`](../ui-quality-of-life-v1.md)
- **Generated filename:** `WO-001-centered-page-viewport.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-001.md`](../validations/validation-WO-001.md)
- **Fixes:** `none`
- **Implementer session/agent ID:** `code-fresh-context-ui-quality-of-life-WO-001-2026-10-25T21:06:43Z`
- **Session exception:** `none`

## Summary

Make the live screenplay page substantially more readable without changing fixed print geometry. The desktop surface should default to an effective 1.6x larger page than the current presentation, center pages within the available window, resize safely, and reveal the caret after edits or navigation when it moves outside the viewport.

## In scope

- Refactor the page viewport/layout around [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:366) and [`paint_page()`](../../../apps/scriptcraft/src/main.rs:266).
- Define bounded default scale/zoom behavior and responsive centered page placement.
- Preserve fixed [`ScreenplayLayout`](../../../crates/scriptcraft-core/src/layout.rs:51) coordinates for print and hit testing.
- Add caret-following scroll behavior and resize/pointer regression coverage.

## Out of scope

- User-persisted zoom preferences, page-profile changes, or print pagination changes.
- A second editing buffer or a new UI framework.

## Requirements

- **REQ-WRITING-LOOP-002:** “As an individual screenwriter, I want continuous writing and fixed-page preview to agree about semantic content and layout inputs, so that I can write comfortably and inspect a predictable page result.”
- **AC-WRITING-002.1:** “Given the same project and fixed page profile, the system shall derive stable page structure and source mappings independent of window zoom.”
- **AC-WRITING-002.4:** “Given a document position and rendered geometry, the system shall support a safe caret/hit-test round trip at valid text boundaries.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) — the host owns window, scrolling, zoom, focus, and rendered page interaction.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) — viewport zoom is not a layout input and the UI consumes backend-neutral geometry.

## Implementation plan

1. Measure the current effective page scale and introduce a named, bounded presentation scale that is 1.6x the current default while adapting to narrow viewports.
2. Place pages in a centered inner region with minimum padding and retain the existing vertical scroll path for multi-page documents.
3. Convert pointer coordinates through the centered/scaled page rect and request scroll-to-caret after edits, Enter/Tab, vertical movement, and page changes.
4. Add headless geometry tests where possible and focused desktop/manual evidence at small, normal, and maximized window sizes.

## Files and systems

- **Create:** focused app/UI tests if the current harness supports them.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs), and any narrowly required UI blueprint evidence.
- **Avoid changing:** [`crates/scriptcraft-core/src/layout.rs`](../../../crates/scriptcraft-core/src/layout.rs) print metrics unless a proven coordinate defect requires it.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`.
- **Manual/external checks:** launch with `cargo run -p scriptcraft`; inspect a normal and maximized window; verify page scale, horizontal centering, resize stability, pointer hit testing, and caret visibility after inserting enough text to cross pages.
- **Acceptance test:** Given a multi-page project, when the window is resized from narrow to maximized and the caret is moved or text is inserted near the document end, then pages remain centered and readable, print/layout units remain unchanged, pointer selection lands on the clicked text, and the viewport scrolls far enough to show the caret.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `verify unchanged`
- **Affected documents/IDs:** [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md)
- **Rationale and evidence:** This changes transient viewport presentation only; the existing requirements and desktop boundary already state that zoom/scroll are UI concerns and do not redefine print pagination. Verify those claims against the final diff.
- **Authorized documentation changes:** Append implementation evidence to the initiative plan/execution log; update the desktop blueprint only if its stated focus/scroll contract becomes more precise.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document impact classification is accurate against the diff; requirements and blueprints remain unchanged because the change is transient viewport presentation.
- [x] Execution log is appended with dispatch/completion identifiers.
- [x] Work order was `in-review` before fresh validation and became `complete` after the passed validation in [`validation-WO-001.md`](../validations/validation-WO-001.md).

## Implementer evidence — 2026-10-25

- **Session/agent ID:** `code-fresh-context-ui-quality-of-life-WO-001-2026-10-25T21:06:43Z`
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs) only for implementation and focused tests. Existing unrelated working-tree changes in `.forge/blueprints/blueprint-review.md`, `.forge/project-manifest.md`, and `.forge/requirements/requirements-review.md` were not modified by this session.
- **Implementation:** named presentation geometry keeps fixed layout units and hit testing unchanged, targets `360 × 1.6 = 576` CSS pixels at the default profile, clamps narrow windows with 16-pixel side padding and a 240-pixel minimum, centers each page in the scroll content row, and requests `ScrollArea` caret reveal after successful edits or selection/navigation changes.
- **Focused coverage:** `presentation_geometry_scales_default_page_and_centers_it` covers default scaling and narrow-window centering; the existing rendered-surface focus/input test was adapted to the centered/scaled page geometry.
- **Validation attempts/results:**
  - `cargo fmt --all -- --check`: passed in the final combined run. An earlier check reported only rustfmt suggestions; `cargo fmt --all` applied them.
  - `cargo check --workspace`: passed.
  - `cargo test --workspace`: passed — 16 app tests, 22 core tests, 3 E2E tests, and 0 doc tests failed.
  - `cargo clippy --workspace --all-targets -- -D warnings`: passed.
  - `cargo run -p scriptcraft`: not run; native window/manual smoke evidence remains for the separate validator/manual gate.
- **Bounded API note:** caret reveal uses egui's supported `Ui::scroll_to_rect` within the existing vertical `ScrollArea`; no horizontal scrolling or persisted viewport state was added. Print pagination and core layout coordinates remain unchanged.
- **Comparable diff evidence:** implementation diff is confined to the viewport/presentation and app regression-test paths in [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs); no WO-002/003/004 files or core layout files changed.

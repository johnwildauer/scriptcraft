# Work Order: WO-004 — Screenplay Spacing Contract

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer after WO-003 validation
- **Phase/order:** `4 of 4`
- **Initiative:** [`../ui-quality-of-life-v1.md`](../ui-quality-of-life-v1.md)
- **Generated filename:** `WO-004-screenplay-spacing-contract.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-004.md`](../validations/validation-WO-004.md)
- **Fixes:** `none`
- **Implementer session/agent ID:** `code-fresh-context-ui-quality-of-life-WO-004-2026-10-25T21:19:55Z`
- **Session exception:** `none`

## Summary

Verify that the fixed screenplay page profile applies correct left/right margins and intended vertical line spacing consistently in structural layout, screen rendering, caret geometry, and PDF output. Change the spacing contract only when measurements or requirements demonstrate that the current metric is wrong.

## In scope

- Measure page/content bounds, role indents, adjacent line baselines, line boxes, caret height, and page capacity from [`ScreenplayLayout`](../../../crates/scriptcraft-core/src/layout.rs:51).
- Compare the fixed layout output used by the app and PDF exporter, separating UI scale from line spacing.
- Add regression fixtures for empty lines, wrapped lines, keep-with-next roles, page boundaries, and margin bounds.
- If required, introduce one named profile/layout spacing input with deterministic serialization implications and update durable docs.
- The initial comparison baseline is the existing verified fixed profile: `612 × 792` units, margins top/bottom/left/right `72/72/90/72`, nominal 55 lines per page, and the current 12-unit fixed line metric. A change requires structural evidence showing the baseline is incorrect and screen/PDF parity tests for the replacement.

## Out of scope

- Arbitrary per-paragraph formatting, broad typography controls, font substitution, or viewport zoom changes.

## Requirements

- **REQ-WRITING-LOOP-002:** “As an individual screenwriter, I want continuous writing and fixed-page preview to agree about semantic content and layout inputs, so that I can write comfortably and inspect a predictable page result.”
- **AC-WRITING-002.1:** “Given the same project and fixed page profile, the system shall derive stable page structure and source mappings independent of window zoom.”
- **AC-WRITING-002.2:** “Given role-aware layout, the system shall apply role measures and keep relationships without inserting spaces or tabs into canonical text.”
- **NFR-WRITING-003:** “Native serialization and structural pagination outputs shall be deterministic for the same semantic project/profile inputs.”

## Blueprints

- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) — layout owns fixed profile rules, source mappings, and caret geometry.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) — screen and PDF consume the same semantic/page inputs.

## Implementation plan

1. Establish executable expected geometry from the current approved profile: paper size, margins, role indents, line height, content height, and page capacity.
2. Test adjacent line y positions and caret rectangles across roles, wraps, empty text, keep rules, and page breaks.
3. Compare screen painter transforms and PDF export consumption to ensure neither adds/removes spacing or changes margins.
4. Correct the smallest discrepancy, then update profile serialization/fixtures and blueprint evidence if the durable contract changes.

## Files and systems

- **Create:** layout spacing/margin tests and, if needed, an export parity fixture.
- **Update:** [`crates/scriptcraft-core/src/layout.rs`](../../../crates/scriptcraft-core/src/layout.rs), [`crates/scriptcraft-core/src/exports.rs`](../../../crates/scriptcraft-core/src/exports.rs), and [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs) only where rendering parity requires it.
- **Avoid changing:** canonical text, role transitions, or viewport zoom behavior.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`.
- **Manual/external checks:** inspect a rendered page and exported PDF at a known scale; compare top/bottom/left/right content edges, line baselines, wrapped text, and caret height.
- **Acceptance test:** Given the same project/profile, when layout is rendered on screen and exported, then margins and line spacing match the approved deterministic geometry, role widths/indents remain intact, lines do not overlap, and changing viewport zoom does not change page count or source mappings.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `update`
- **Affected documents/IDs:** [`technical-context.md`](../../../blueprints/technical-context.md), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), conditionally [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) if the shared screen/PDF layout contract changes
- **Rationale and evidence:** The blueprints already name fixed metrics but do not record a verified line-spacing contract. Update only with measured final values and passing structural/export evidence; otherwise record that the existing 12-unit metric was verified.
- **Authorized documentation changes:** Add the measured spacing/margin contract and test evidence; do not broaden the product to arbitrary formatting.

## Completion checklist

- [x] Geometry contract is measured and recorded: the default profile is `612 × 792`, margins top/bottom/left/right `72/72/90/72`, `55` nominal lines, and fixed `12`-unit line boxes. Adjacent baselines, empty/wrapped/keep lines, page resets, role measures, and caret heights match the fixed layout output. The approved metric required no change.
- [x] Implementation is complete within scope: added structural layout, screen-transform, caret, and PDF-transform regression coverage; no production spacing or viewport metric changed.
- [x] Tests/checks and required gates have evidence: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace` (`19` app + `28` core + `5` WO-003 + `3` E2E passed), `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo build --workspace` all passed.
- [x] Durable-document impact classification matches the final diff: requirements verified unchanged; component and technical blueprints record the measured contract. The feature blueprint required no update because shared screen/PDF behavior did not change.
- [x] Execution log is appended with dispatch/completion identifiers.
- [x] Work order was `in-review` before fresh validation and became `complete` after the passed validation in [`validation-WO-004.md`](../validations/validation-WO-004.md).

## WO-004 implementer evidence

- **Dispatch:** Fresh implementer session `code-fresh-context-ui-quality-of-life-WO-004-2026-10-25T21:19:55Z`; WO-003 had passed fresh validation before dispatch. WO-004 was the only assigned scope.
- **Changed files:** [`crates/scriptcraft-core/src/layout.rs`](../../../crates/scriptcraft-core/src/layout.rs), [`crates/scriptcraft-core/src/exports.rs`](../../../crates/scriptcraft-core/src/exports.rs), and [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs); this work order and the linked blueprint evidence were updated.
- **Geometry findings:** layout page bounds are `612 × 792`; content bounds are `(90,72)` with `450 × 648` size; role measures/indents remain fixed; adjacent line boxes and caret heights are `12` units; nominal page capacity remains `55` and resets to the top margin at page boundaries. PDF transforms preserve each layout line's x coordinate and use `paper_height - y - height`. Screen tests confirm zoom scales those fixed coordinates without changing layout/page structure.
- **Metric decision:** no spacing, margin, page-profile, or viewport-zoom production change was justified. Existing fixed metrics are retained and now have regression/export-parity coverage.
- **Validation evidence:** no native desktop interaction was claimed in this implementer session; structural/headless checks above passed. Fresh validation returned `PASS WITH WARNINGS`; the orchestrator marked this work order `complete`.

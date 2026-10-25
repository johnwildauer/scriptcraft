# Work Order: WO-002b — Profile Gap and PDF Font Fix

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer
- **Phase/order:** `fix for WO-002a`
- **Initiative:** [`../editor-geometry-corrections-v1.md`](../editor-geometry-corrections-v1.md)
- **Generated filename:** `WO-002b-profile-gap-and-pdf-font-fix.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-002b.md`](../validations/validation-WO-002b.md)
- **Fixes:** fix chain for [`WO-002a-font-and-profile-rendering-fix.md`](WO-002a-font-and-profile-rendering-fix.md) and failing [`validation-WO-002a.md`](../validations/validation-WO-002a.md)
- **Implementer session/agent ID:** `code-fresh-context-editor-geometry-corrections-WO-002b-2026-10-25T22:37:36Z`
- **Session exception:** `none`

## Summary

Resolve the two remaining hard stops in the approved WO-002 profile contract: preserve arbitrary bounded boundary-gap values without integer quantization loss, and make PDF font selection consume the persisted profile font metadata rather than hardcoding a font name.

## In scope

- Make layout placement preserve the profile's exact bounded boundary gap for alternate profiles, using a geometry representation that does not silently truncate fractional or non-multiple values.
- Make PDF export derive its font resource/name from the persisted profile font contract with a deterministic safe fallback and tests for default and alternate profile metadata.
- Preserve the approved inch/point conversion, role boxes, line advance, source/caret/pagination parity, screen/PDF parity, and bounded serialization.
- Add regression tests specifically targeting non-multiple boundary gaps and alternate profile font metadata.

## Out of scope

- New formatting rules, override UI, changing the approved oracle, or unrelated renderer/navigation behavior.

## Validation findings being fixed

- **WO-002a validation hard stop 1:** [`ScreenplayLayout::place_element()`](../../../crates/scriptcraft-core/src/layout.rs:240) quantizes persisted boundary gaps through integer division, so alternate profile gap values are not faithfully applied.
- **WO-002a validation hard stop 2:** [`export_pdf()`](../../../crates/scriptcraft-core/src/exports.rs:68) hardcodes `/BaseFont /Courier` instead of consuming alternate persisted `font_family` metadata.

## Requirements

- **REQ-WRITING-LOOP-002:** “As an individual screenwriter, I want continuous writing and fixed-page preview to agree about semantic content and layout inputs, so that I can write comfortably and inspect a predictable page result.”
- **AC-WRITING-002.1:** “Given the same project and fixed page profile, the system shall derive stable page structure and source mappings independent of window zoom.”
- **NFR-WRITING-003:** “Native serialization and structural pagination outputs shall be deterministic for the same semantic project/profile inputs.”

## Blueprints

- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) — profile-derived geometry and deterministic layout/persistence.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) — shared screen/PDF geometry boundary.

## Implementation plan

1. Inspect the failing alternate-profile fixtures and choose the smallest bounded representation that preserves profile gaps exactly through layout and serialization.
2. Remove PDF's hardcoded font bypass and add a safe deterministic mapping from allowed persisted font metadata to the PDF resource/base-font output.
3. Add tests for non-multiple gap values, default/alternate font metadata round trips, PDF output differences, and unchanged default geometry.
4. Run the complete workspace checks and document any unavoidable font-format fallback explicitly.

## Files and systems

- **Create:** focused layout and PDF regression tests if needed.
- **Update:** [`crates/scriptcraft-core/src/layout.rs`](../../../crates/scriptcraft-core/src/layout.rs), [`crates/scriptcraft-core/src/exports.rs`](../../../crates/scriptcraft-core/src/exports.rs), and narrowly related profile/reference evidence.
- **Avoid changing:** app navigation, canonical screenplay text, approved margins/role boxes, and override UI.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo build --workspace`; `git diff --check`.
- **Manual/external checks:** inspect generated PDF resource metadata for default and alternate font profiles when available; native window interaction remains optional and must be reported honestly.
- **Acceptance test:** Given a profile with a non-multiple boundary gap and alternate allowed font metadata, when layout and PDF are generated and the profile is serialized/reloaded, then the gap is preserved without truncation, PDF font output follows the profile mapping/fallback, default output remains stable, and all source/caret/page mappings remain valid.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `verify unchanged`
- **Affected documents/IDs:** [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md), and [`format-rules.md`](../format-rules.md) only if fallback semantics change.
- **Rationale and evidence:** This is a fix within the already-approved profile/render/export boundary; no new product capability is authorized.
- **Authorized documentation changes:** Record exact representation and font fallback semantics if they differ from WO-002a's documented contract.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Fresh validation passes this fix work order in [`validation-WO-002b.md`](../validations/validation-WO-002b.md), with `PASS WITH WARNINGS`.
- [x] Original WO-002 is complete only through this passed fix chain.

## Implementer completion evidence — 2026-10-25

- **Implementation:** Layout now carries an exact point-based next-line cursor on each page. Boundary placement adds the persisted `boundary_gap` directly to that cursor instead of deriving whole-line units through integer division; logical page-capacity accounting remains deterministic and source/caret/pagination contracts are unchanged. PDF export now maps persisted allowed font metadata (`Courier`, `Courier New`, and `Courier Prime`) to deterministic PDF resource names, with unknown input safely falling back to `Courier`.
- **Focused regressions:** Added a non-multiple `boundary_gap = 10` / `line_advance = 14` layout assertion proving the exact 24-point boundary-to-boundary origin; added default `Courier New` and alternate `Courier Prime` native-profile round-trip/PDF resource assertions plus unknown-family fallback coverage. Existing default geometry/spacing, source/caret, pagination, and PDF transform tests remain green.
- **Changed files:** [`crates/scriptcraft-core/src/layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs), [`crates/scriptcraft-core/src/exports.rs`](../../../../crates/scriptcraft-core/src/exports.rs), this work order, and [`execution-log.md`](../execution-log.md). The implementation session did not modify navigation, canonical text, profile persistence/schema, or override UI.
- **Exact validation evidence:** `cargo fmt --all -- --check` — passed, exit 0; `cargo check --workspace` — passed, exit 0; `cargo test --workspace` — passed, exit 0 (23 app tests, 34 core unit tests, 5 WO-003 integration tests, 3 WO-008 integration tests, 0 doc-test failures; 65 tests passed, 0 failed); `cargo clippy --workspace --all-targets -- -D warnings` — passed, exit 0; `cargo build --workspace` — passed, exit 0; `git diff --check` — passed, exit 0. `cargo fmt --all` was used as formatting remediation before the final format check.
- **Comparable diff/evidence:** Compared the two hard stops in [`validation-WO-002a.md`](../validations/validation-WO-002a.md) against the focused layout/PDF changes and current shared profile consumers. The approved geometry, 72-point inch boundary, screen/PDF coordinates, source/caret mappings, pagination model, persisted profile fields, and no-override-UI boundary remain in place. Native visual interaction was not attempted in this implementer session; fresh independent validation remains required.

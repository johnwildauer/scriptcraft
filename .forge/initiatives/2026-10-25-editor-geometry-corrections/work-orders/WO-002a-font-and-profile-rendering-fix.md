# Work Order: WO-002a — Font and Profile Rendering Fix

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer
- **Phase/order:** `fix for WO-002`
- **Initiative:** [`../editor-geometry-corrections-v1.md`](../editor-geometry-corrections-v1.md)
- **Generated filename:** `WO-002a-font-and-profile-rendering-fix.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-002a.md`](../validations/validation-WO-002a.md)
- **Fixes:** passed fix chain [`WO-002b-profile-gap-and-pdf-font-fix.md`](WO-002b-profile-gap-and-pdf-font-fix.md) with [`validation-WO-002b.md`](../validations/validation-WO-002b.md); original [`WO-002-screenplay-width-and-leading.md`](WO-002-screenplay-width-and-leading.md)
- **Implementer session/agent ID:** `code-fresh-context-editor-geometry-corrections-WO-002a-2026-10-25T22:20:17Z`
- **Session exception:** `none`

## Summary

Resolve the two hard-stop findings from WO-002 validation without expanding the approved screenplay geometry scope: the renderer must honor the approved 12-point Courier-family target rather than a scaled 10-point UI font, and all persisted profile geometry must drive role boxes/line advancement instead of leaving role definitions hardcoded in layout code.

## In scope

- Replace the renderer's unapproved fixed 10-point monospace base with a clearly documented 12-point Courier-family target/fallback that remains compatible with fixed layout units and viewport scaling.
- Move approved role boxes, line advance, boundary gap, and related geometry into the named/versioned persisted format/profile seam required by [`format-rules.md`](../format-rules.md).
- Make layout, caret geometry, pagination, PDF transforms, and screen rendering consume the same profile-derived values.
- Preserve bounded validation, deterministic serialization, source mappings, and the user-approved geometry oracle.
- Add regressions proving profile changes affect derived geometry consistently without storing UI pixels or per-element formatting.

## Out of scope

- Building a project-override editor or general font-management UI.
- Changing the approved margins/role boxes/spacing oracle, semantic text, Enter/Tab behavior, or vertical navigation.

## Requirements

- **REQ-WRITING-LOOP-002:** “As an individual screenwriter, I want continuous writing and fixed-page preview to agree about semantic content and layout inputs, so that I can write comfortably and inspect a predictable page result.”
- **AC-WRITING-002.1:** “Given the same project and fixed page profile, the system shall derive stable page structure and source mappings independent of window zoom.”
- **AC-WRITING-002.2:** “Given role-aware layout, the system shall apply role measures and keep relationships without inserting spaces or tabs into canonical text.”
- **NFR-WRITING-003:** “Native serialization and structural pagination outputs shall be deterministic for the same semantic project/profile inputs.”

## Validation findings being fixed

- **WO-002 validation hard stop 1:** renderer uses [`FontId::monospace()`](../../../apps/scriptcraft/src/main.rs:326) with a scaled 10-point base rather than the approved 12-point Courier-family target.
- **WO-002 validation hard stop 2:** role boxes remain hardcoded in [`role_box()`](../../../crates/scriptcraft-core/src/layout.rs:311), and a separate app [`LINE_HEIGHT`](../../../apps/scriptcraft/src/main.rs:320) consumer bypasses profile-derived geometry.

## Blueprints

- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) — named/versioned profile owns role geometry and layout metrics.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) — screen and PDF consume one deterministic geometry source.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) — desktop rendering applies viewport transforms without becoming the format source of truth.

## Implementation plan

1. Inspect the WO-002 diff and failing validation evidence; identify the smallest profile/font ownership correction.
2. Define the persisted profile fields needed for approved margins, role boxes, `line_advance`, `boundary_gap`, named format identity/version, and bounded future override compatibility.
3. Route layout, app painting/caret sizing, and PDF conversion through those profile-derived values; keep inches as the human-facing definition and use 72 points/inch at the boundary.
4. Establish a deterministic Courier-family font target/fallback and ensure the renderer's point-size choice is not confused with viewport zoom.
5. Add tests for default profile round trip, an altered bounded profile fixture, screen/PDF/layout parity, role edges, source/caret mapping, and serialization bounds.

## Files and systems

- **Create:** focused profile/font/parity tests if needed.
- **Update:** only the implicated files among [`crates/scriptcraft-core/src/lib.rs`](../../../crates/scriptcraft-core/src/lib.rs), [`crates/scriptcraft-core/src/layout.rs`](../../../crates/scriptcraft-core/src/layout.rs), [`crates/scriptcraft-core/src/exports.rs`](../../../crates/scriptcraft-core/src/exports.rs), [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs), [`format-rules.md`](../format-rules.md), and authorized blueprint evidence.
- **Avoid changing:** canonical text, role transitions, viewport navigation, and unrelated persistence behavior.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo build --workspace`.
- **Manual/external checks:** inspect the renderer/font path and, when available, compare a rendered page and PDF for the approved margins, role edges, leading, and font size; attempt native smoke honestly.
- **Acceptance test:** Given the default named profile and a bounded alternate profile fixture, when layout, screen rendering, caret geometry, and PDF export are produced, then all consume the same persisted role/spacing/font metrics, the default renderer uses the approved 12-point target, no role geometry is hardcoded outside the profile boundary, and viewport zoom changes only presentation scale.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `update`
- **Affected documents/IDs:** [`technical-context.md`](../../../blueprints/technical-context.md), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md), and [`format-rules.md`](../format-rules.md)
- **Rationale and evidence:** The fix addresses implementation defects against the approved WO-002 contract and clarifies the already-authorized persisted format/profile seam. Update durable evidence to match the final diff; do not add override UI.
- **Authorized documentation changes:** Record final profile fields, font fallback/metric choice, and parity-test evidence.

## Completion checklist

- [x] Implementation is complete within the original WO-002 boundary.
- [x] Tests/checks and required gates have evidence.
- [ ] Fresh validation passes this fix work order.
- [ ] Original WO-002 is complete only through this passed fix chain.

## Implementer completion evidence — 2026-10-25

- **Implementation:** `PageProfile` now persists bounded `font_family`, `font_size`, `char_width`, and an explicit seven-role `role_boxes` table alongside the approved margins, line advance, and boundary gap. Layout wrapping, role rectangles, hit testing, carets, pagination, and page capacity consume those profile values. Desktop selection/IME rectangles use `layout.line_height`; the renderer uses the persisted 12-point base with egui's deterministic built-in monospace fallback for an unavailable named Courier face, while viewport scale remains a display transform. PDF text size and transforms consume the shared profile/layout.
- **Focused regressions:** Added default and alternate named-profile deterministic JSON round trips, bounded role-geometry rejection, alternate-profile role/wrap/line/caret/hit-test behavior, and profile-derived PDF font-size/rectangle coverage. Existing source mapping, pagination, viewport-scale, and native editing/persistence regressions remain green.
- **Changed files:** [`lib.rs`](../../../../crates/scriptcraft-core/src/lib.rs), [`layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs), [`exports.rs`](../../../../crates/scriptcraft-core/src/exports.rs), [`main.rs`](../../../../apps/scriptcraft/src/main.rs), [`format-rules.md`](../format-rules.md), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md), and [`technical-context.md`](../../../blueprints/technical-context.md). No WO-001 files or unrelated product behavior changed; no override editor was added.
- **Validation attempts:** An initial `cargo test --workspace` exposed one expected headless-font binding panic after the first named-font attempt and one incorrect new fixture assertion. The renderer was changed to the documented built-in monospace fallback and the fixture corrected; the final complete test run passed: 23 app tests, 32 core unit tests, 5 WO-003 integration tests, 3 WO-008 integration tests, and 0 doc tests.
- **Exact final checks:** `cargo fmt --all -- --check` — passed, exit 0; `cargo check --workspace` — passed, exit 0; `cargo test --workspace` — passed, exit 0 (63 tests, 0 failures, 0 doc tests); `cargo clippy --workspace --all-targets -- -D warnings` — passed, exit 0; `cargo build --workspace` — passed, exit 0; `git diff --check` — passed, exit 0. `cargo fmt --all` was used once as the required remediation after the initial format-check diff.
- **Comparable diff/evidence:** Current source and focused tests were compared against [`validation-WO-002.md`](../validations/validation-WO-002.md); both hard stops are addressed within the approved WO-002 geometry/profile boundary. Native interactive smoke remains unavailable and is not claimed as evidence; fresh independent validation is still required.

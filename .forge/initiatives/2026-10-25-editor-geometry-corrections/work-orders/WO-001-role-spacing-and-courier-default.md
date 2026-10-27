# Work Order: WO-001 — Role Spacing and Courier Default

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer after v2 design approval
- **Phase/order:** `1 of 6`
- **Initiative:** [`../editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md)
- **Validation:** [`../validations/validation-WO-001-v2.md`](../validations/validation-WO-001-v2.md)

## Summary

Correct role-pair vertical spacing and make `Courier` the default persisted profile font without packaging a new font.

## In scope

- Define a role-pair spacing table where Dialogue and Parenthetical boundaries add zero extra gap above the new element.
- Preserve explicit spacing for other boundaries and same-element line advance.
- Change the default profile family to `Courier`, retain bounded accepted-family validation, and update PDF mapping/tests.
- Keep screen, caret, pagination, and PDF geometry sourced from the same profile/layout values.

## Out of scope

- Font packaging, font fallback databases, arbitrary formatting, or a schema migration unless required by existing profile compatibility.

## Requirements and blueprints

- **Requirements:** `REQ-WRITING-LOOP-002`, `AC-WRITING-002.1`, `AC-WRITING-002.2`, `NFR-WRITING-003` in [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md).
- **Blueprints:** [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md).

## Implementation guidance

1. Replace the global boundary-gap assumption with an explicit role-pair policy in layout.
2. Ensure pagination capacity accounts for the selected gap without creating phantom lines.
3. Update profile defaults and PDF family mapping while preserving accepted explicit project families.
4. Add structural tests for Action→Dialogue, Dialogue→Parenthetical, Parenthetical→Dialogue, Character→Dialogue, and representative other pairs, plus profile JSON/PDF round trips.

## Files and systems

- **Update:** [`crates/scriptcraft-core/src/layout.rs`](../../../crates/scriptcraft-core/src/layout.rs), [`crates/scriptcraft-core/src/lib.rs`](../../../crates/scriptcraft-core/src/lib.rs), [`crates/scriptcraft-core/src/exports.rs`](../../../crates/scriptcraft-core/src/exports.rs), and [`format-rules.md`](../format-rules.md).
- **Avoid changing:** canonical text, role transitions, viewport zoom, and pointer behavior.

## Validation

- Run all project Cargo gates.
- Assert exact role-pair y-origin differences and page-boundary behavior.
- Assert new projects default to `Courier` and explicit supported profiles round-trip unchanged.
- Assert PDF emits the matching Courier base font.

## Durable-document impact

- **Requirements:** `verify unchanged` unless the final role-pair policy is promoted to a new requirement.
- **Blueprint:** `update` with the role-pair table and Courier default.

## Reconciled handoff

- **Applicable requirements (verbatim):** `REQ-WRITING-LOOP-002`: “As an individual screenwriter, I want continuous writing and fixed-page preview to agree about semantic content and layout inputs, so that I can write comfortably and inspect a predictable page result.” `AC-WRITING-002.1`: “Given the same project and fixed page profile, the system shall derive stable page structure and source mappings independent of window zoom.” `AC-WRITING-002.2`: “Given role-aware layout, the system shall apply role measures and keep relationships without inserting spaces or tabs into canonical text.” `NFR-WRITING-003`: “Native serialization and structural pagination outputs shall be deterministic for the same semantic project/profile inputs.”
- **Exact oracle:** Incoming Dialogue/Parenthetical always has zero extra boundary gap for every body predecessor; all other incoming roles use `boundary_gap`; title-page boundaries are excluded. New projects default to `Courier`; stored existing families remain unchanged and malformed/absent fields remain typed errors.
- **Rationale/evidence:** Current [`place_element()`](../../../crates/scriptcraft-core/src/layout.rs:241) applies one gap to every pair, and [`PageProfile::default()`](../../../crates/scriptcraft-core/src/lib.rs:174) uses `Courier New`; the v2 plan matrix resolves both without changing canonical text.
- **Authorized documentation changes:** Update [`format-rules.md`](../format-rules.md), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md), and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) only if the desktop font contract is recorded there; update this WO with measured table/test evidence.
- **Native evidence boundary:** Headless layout/profile/PDF checks are required. Native visual inspection is optional under project policy and must be recorded as unavailable rather than inferred from rendered tests.

## Implementation evidence — 2026-10-26

- **Status decision:** Fresh validation [`validation-WO-001-v2.md`](../validations/validation-WO-001-v2.md) returned `PASS WITH WARNINGS`; no fix work order is required, so this work order transitions to `complete`.
- **Role-pair table implemented:** For every body predecessor (`Scene Heading`, `Action`, `Character`, `Dialogue`, `Parenthetical`, `Transition`, and `Note`), incoming `Dialogue` and `Parenthetical` use `extra_gap = 0`. Every other incoming body role uses `PageProfile.boundary_gap`, including same-role and `Note` transitions. Wrapped lines use `line_advance`; title-page boundaries remain excluded. Page capacity increments the logical boundary unit only when the selected gap is nonzero.
- **Font compatibility implemented:** `PageProfile::default()` now selects `Courier` for new projects. JSON decoding still requires the stored `font_family` field, accepts `Courier`, `Courier New`, and `Courier Prime`, preserves explicit stored values, and returns typed missing/wrong-type errors rather than migrating or rewriting them. PDF mapping continues to consume the persisted family, with `Courier` producing `/BaseFont /Courier`.
- **Deterministic evidence:** Added exhaustive incoming Dialogue/Parenthetical spacing coverage, representative nonzero-gap pairs, zero-gap page-capacity coverage, default/support-family round trips, malformed/absent font-field errors, updated PDF default-family assertions, and adjusted existing geometry fixtures to the approved oracle.
- **Required gates:** `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace` (26 app tests, 40 core unit tests, 2 WO-002 integration tests, 5 WO-003 integration tests, 3 WO-008 integration tests, 0 doc tests), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace`, and `git diff --check` all passed. Initial formatting and stale assertions were remediated before the final passing run.
- **Implementation files owned by this WO:** [`crates/scriptcraft-core/src/layout.rs`](../../../crates/scriptcraft-core/src/layout.rs), [`crates/scriptcraft-core/src/lib.rs`](../../../crates/scriptcraft-core/src/lib.rs), [`crates/scriptcraft-core/src/exports.rs`](../../../crates/scriptcraft-core/src/exports.rs), and the affected default-font assertion in [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs). The working tree also contains pre-existing changes in [`crates/scriptcraft-core/src/editing.rs`](../../../crates/scriptcraft-core/src/editing.rs); those were not modified as part of this WO.
- **Authorized durable docs updated:** [`format-rules.md`](../format-rules.md), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md), and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) now state the role-pair policy and Courier default without claiming bundled-font parity. Requirements were verified unchanged.
- **Follow-ups:** Fresh independent validation remains required, including native/manual evidence classification as unavailable unless performed. Later work orders remain unauthorized and unimplemented by this dispatch.

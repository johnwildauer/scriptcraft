# Work Order: WO-003 — Nearest-Line Pointer Placement

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer after WO-002 validation
- **Phase/order:** `3 of 6`
- **Initiative:** [`../editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md)
- **Validation:** [`../validations/validation-WO-003-v2.md`](../validations/validation-WO-003-v2.md)

## Summary

Make clicks in line whitespace and page margins place the caret at the nearest rendered line, clamping horizontal position to that line's start or end.

## In scope

- Add a nearest-line layout hit-test path based on vertical distance first, then horizontal distance/tie order.
- Map left-of-line clicks to line start, right-of-line clicks to line end, and in-line clicks through existing source-safe hit testing.
- Map clicks above/below the document to the nearest available line; preserve title-page and editor-surface ownership rules.
- Add caret↔pointer round-trip tests for all screenplay roles, wrapped lines, margins, blank areas, and page boundaries.

## Out of scope

- New pointer gestures, persistent viewport state, selection semantics redesign, or canonical text changes.

## Requirements and blueprints

- **Requirements:** `AC-WRITING-002.4`, `NFR-WRITING-002` in [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md).
- **Blueprints:** [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md).

## Implementation guidance

Extend the backend-neutral layout query rather than guessing character offsets in [`caret_from_pointer()`](../../../apps/scriptcraft/src/main.rs:238). Preserve UTF-8 boundary guarantees and use one geometry source for hit testing and caret rectangles.

## Validation

- Test nearest line selection above, within, and below line rectangles.
- Test left/right margin clamping and wrapped-line source ranges.
- Exercise the production rendered surface with pointer events and run all Cargo gates.

## Durable-document impact

- **Requirements:** `verify unchanged`.
- **Blueprint:** `update` to record nearest-line hit-testing policy.

## Reconciled handoff

- **Applicable requirements (verbatim):** `AC-WRITING-002.4`: “Given a document position and rendered geometry, the system shall support a safe caret/hit-test round trip at valid text boundaries.” `NFR-WRITING-002`: “Core editing, layout, and persistence behavior shall be exercisable without a desktop window for deterministic tests.”
- **Boundary matrix:** Body pages are one document-global query domain; exact vertical ties prefer the earlier line, then the lower page index. Left/right margin clicks clamp to line start/end. Inter-page gaps compare adjacent body lines. Title-page clicks do not move body selection; empty pages use nearest non-empty body page; clicks above/below clamp to first/last body line. In-line x uses existing source-safe hit testing.
- **Rationale/evidence:** [`hit_test()`](../../../crates/scriptcraft-core/src/layout.rs:139) currently returns `None` outside line rectangles; nearest-line behavior belongs in layout so render, caret, and hit testing share source ranges.
- **Authorized documentation changes:** Update [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) with the query contract and this WO with the boundary matrix/test evidence.
- **Native evidence boundary:** Headless hit/caret round trips and rendered pointer tests are required; native pointer smoke is optional and unavailable evidence must be recorded honestly.

## Validation outcome

- Fresh validation [`validation-WO-003-v2.md`](../validations/validation-WO-003-v2.md) returned `PASS WITH WARNINGS`.
- Warnings cover unavailable optional native pointer smoke, no dedicated empty non-title-page fixture, and gates recorded rather than independently rerun in the inspection context.
- State transition: `in-review → complete`; no fix work order is required.

## Implementation evidence — 2026-10-26

- **Changed files:** [`layout.rs`](../../../crates/scriptcraft-core/src/layout.rs), [`main.rs`](../../../apps/scriptcraft/src/main.rs), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md), and this work order plus the v2 execution log.
- [`ScreenplayLayout::nearest_line_hit_test()`](../../../crates/scriptcraft-core/src/layout.rs:158) now queries all rendered body lines in document flow, skips title pages, uses deterministic vertical/document-order ties, clamps horizontal coordinates, and retains source-byte/UTF-8-safe in-line mapping.
- [`ScriptCraftApp::caret_from_pointer()`](../../../apps/scriptcraft/src/main.rs:243) now sends signed page-local coordinates to the shared query. [`ScriptCraftApp::pointer_body_page()`](../../../apps/scriptcraft/src/main.rs:263) resolves rendered page margins and inter-page gaps while preserving title-page ownership.
- Deterministic headless regressions cover every body role, left/right margins, UTF-8 source ranges, caret→pointer round trips, above/below document clamps, exact inter-page ties, and title-page exclusion. The rendered editor regression covers a pointer click in page blank/margin space and confirms end-of-line placement.
- **Gate evidence:** `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace`, and `git diff --check` all passed. Native pointer smoke was not performed and is recorded as unavailable/optional under project policy.
- **Follow-ups:** Fresh independent WO-003 validation remains required. No WO-004+ behavior was implemented. Native/manual pointer smoke remains optional and unclaimed.

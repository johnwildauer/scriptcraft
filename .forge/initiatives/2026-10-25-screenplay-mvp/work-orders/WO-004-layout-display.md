# Work Order: WO-004 — Deterministic Layout and Display

## Metadata

- **Status:** `complete`
- **Assignee:** Layout/display implementer (one fresh session)
- **Phase/order:** 4 of 8; depends on WO-003
- **Initiative:** [`../2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Generated filename:** `WO-004-layout-display.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-004.md`](../validations/validation-WO-004.md)
- **Fixes:** `none`
- **Implementer session/agent ID:** `code-session-2026-10-25-wo-004`
- **Session exception:** `none`

## Summary

Build the shared fixed-profile layout/display path that derives continuous-writing and page-preview data from the semantic project. Pagination must be deterministic, role-aware, safely bounded, and provide source mappings for later UI/PDF consumers.

## In scope

- Pinned/recorded initial page profile and font metrics, role measures, wrapping, page breaks, oversized-element escape hatch, source mappings, and caret/hit-test boundary data.
- Structural pagination fixtures and determinism tests independent of viewport zoom.

## Out of scope

- Desktop rendering widgets, native save/recovery, PDF file generation, TXT export, incremental pagination, broad formatting, or platform polish.

## Requirements

- **REQ-WRITING-LOOP-002:** “As an individual screenwriter, I want continuous writing and fixed-page preview to agree about semantic content and layout inputs, so that I can write comfortably and inspect a predictable page result.”
- **AC-WRITING-002.1:** “Given the same project and fixed page profile, the system shall derive stable page structure and source mappings independent of window zoom.”
- **AC-WRITING-002.2:** “Given role-aware layout, the system shall apply role measures and keep relationships without inserting spaces or tabs into canonical text.”
- **AC-WRITING-002.3:** “Given long text or an oversized element, the system shall wrap or split it safely without an infinite loop or unbounded blank-page behavior.”
- **AC-WRITING-002.4:** “Given a document position and rendered geometry, the system shall support a safe caret/hit-test round trip at valid text boundaries.”
- **NFR-WRITING-003:** “Native serialization and structural pagination outputs shall be deterministic for the same semantic project/profile inputs.”

## Blueprints

- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) — `ScreenplayLayout` reads the valid project and emits temporary geometry.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) — one layout path feeds display and print consumers.

## Implementation plan

1. Select and record bounded profile/font inputs; implement role-aware measurements and canonical-text-preserving wrapping.
2. Implement deterministic page and source-mapping output with bounded oversized-element handling.
3. Add structural, zoom-independent, long-input, and caret/hit-test tests.

## Files and systems

- **Create:** layout/profile/display modules and fixtures under the reusable crate.
- **Update:** blueprint profile/metric facts and feature layout contract if resolved.
- **Avoid changing:** UI framework, persistence, exports, and semantic editing rules.

## Validation and E2E acceptance tests

- **Automated checks:** Workspace format/check/test; repeated pagination equality, wrapping bounds, source-map, and hit-test fixtures.
- **Manual/external checks:** Inspect representative page preview data for role spacing without canonical text mutation.
- **Acceptance test:** Given identical project/profile inputs at different viewport zoom values, when layout runs repeatedly, then page structure/mappings are identical, role measures are applied, and oversized text terminates safely.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `update`
- **Affected documents/IDs:** `REQ-WRITING-LOOP-002`, `AC-WRITING-002.1`–`002.4`, `NFR-WRITING-003`, [`technical-context.md`](../../../blueprints/technical-context.md)
- **Rationale and evidence:** This resolves provisional profile/layout facts and creates the shared display contract.
- **Authorized documentation changes:** Record selected bounded profile/font inputs, layout output shape, and evidence only.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document impact classification is accurate.
- [x] Execution log is appended with dispatch/completion IDs.
- [x] Work order reached `in-review`; completion is supported by the passed fresh validation below.

## Implementation evidence

- **Completion session/agent ID:** `code-session-2026-10-25-wo-004`
- **Changed files:** [`crates/scriptcraft-core/src/layout.rs`](../../../crates/scriptcraft-core/src/layout.rs), [`crates/scriptcraft-core/src/lib.rs`](../../../crates/scriptcraft-core/src/lib.rs), this work order, the authorized layout blueprints, and the initiative execution log. No commit was made.
- **Layout contract:** Added dependency-free [`ScreenplayLayout`](../../../crates/scriptcraft-core/src/layout.rs:51) with integer geometry, pinned `ScriptCraft Fixed Metrics`, seven-unit character cells, twelve-unit lines, fixed US-letter profile bounds, margins, role indents/measures, canonical-text source byte ranges, page assignment, optional separate title page, basic keep-with-next, bounded wrapping, and oversized-element page splitting.
- **Interaction contract:** Added backend-neutral [`hit_test`](../../../crates/scriptcraft-core/src/layout.rs:120) and [`caret_rect`](../../../crates/scriptcraft-core/src/layout.rs:141) mappings that only return valid UTF-8 caret boundaries represented by the derived line geometry.
- **Focused tests:** Five structural tests cover wrapping/source ranges, page bounds/margins/role boxes, title-page separation and keep rules, long-element termination without blank pages, and repeated-layout equality plus hit/caret mapping.
- **Commands and exact results:** `cargo fmt --all` — passed; `cargo fmt --all -- --check` — passed; `cargo check --workspace` — passed; `cargo test --workspace` — passed (15 core tests, 0 failures; app/doc tests also passed); `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- **Validation status:** Independent validation passed; see [`validation-WO-004.md`](../validations/validation-WO-004.md). WO-004 is `complete`.

## Independent validation

- **Verdict:** `PASS`
- **Evidence:** [`validation-WO-004.md`](../validations/validation-WO-004.md) records fresh-context source/model inspection, an independent structural probe covering deterministic wrapping, source ranges, role boxes, bounds, page/title separation, keep rules, oversized termination, hit/caret mapping, repeatability, and passing format/check/test/clippy gates.
- **Result:** No hard-stop implementation, scope, policy-gate, or rogue-behavior finding; no lettered fix work order required.

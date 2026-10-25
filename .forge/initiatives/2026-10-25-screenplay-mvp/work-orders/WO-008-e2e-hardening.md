# Work Order: WO-008 — End-to-End Hardening

## Metadata

- **Status:** `complete`
- **Assignee:** MVP hardening implementer (one fresh session)
- **Phase/order:** 8 of 8; depends on WO-001 through WO-007
- **Initiative:** [`../2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Generated filename:** `WO-008-e2e-hardening.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-008.md`](../validations/validation-WO-008.md)
- **Fixes:** `none`
- **Implementer session/agent ID:** `code-session-2026-10-25-wo-008`
- **Session exception:** `none`

## Summary

Prove and harden the complete headless writing loop and focused desktop smoke path after all feature seams exist. This is a bounded integration/regression WO, not permission to add new product scope.

## In scope

- One representative scene E2E fixture covering create, roles, edit, transition, undo/redo, paginate/preview, save/reopen/recover, PDF/TXT export, hostile input, and failure safety.
- Regression fixes within existing approved boundaries, command documentation/evidence, and final source/blueprint consistency checks.

## Out of scope

- New features, broad formatting, DOCX/FDX, collaboration, MCP, beat board/outline, web/multi-platform polish, or redesign of prior contracts.

## Requirements

- **REQ-WRITING-LOOP-001:** “As an individual screenwriter, I want to create and edit an ordered screenplay with explicit element roles, so that my draft retains screenplay meaning while I write.”
- **REQ-WRITING-LOOP-002:** “As an individual screenwriter, I want continuous writing and fixed-page preview to agree about semantic content and layout inputs, so that I can write comfortably and inspect a predictable page result.”
- **REQ-WRITING-LOOP-003:** “As an individual screenwriter, I want to save, reopen, and recover my project without losing the last valid version, so that an interrupted session does not destroy my draft.”
- **REQ-WRITING-LOOP-004:** “As an individual screenwriter, I want to export a reviewable PDF and a plainly lossy text copy, so that I can inspect or share output without confusing exports with the native source.”
- **NFR-WRITING-001:** “Production paths should prefer typed errors and bounded inputs over panic-prone handling; a discovered crash or data-loss defect requires a regression test.”
- **NFR-WRITING-002:** “Core editing, layout, and persistence behavior shall be exercisable without a desktop window for deterministic tests.”
- **NFR-WRITING-003:** “Native serialization and structural pagination outputs shall be deterministic for the same semantic project/profile inputs.”

## Blueprints

- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) — verify the composed semantic → layout → persistence/export flow.
- [`technical-context.md`](../../../blueprints/technical-context.md) — verify final topology, commands, and evidence boundaries.

## Implementation plan

1. Add one deterministic headless end-to-end test and failure/hostile-input regression cases using the same operations as the UI.
2. Run verified format/check/test/build and desktop smoke commands; fix only defects within existing WO boundaries.
3. Record final evidence, unresolved non-blocking follow-ups, and documentation consistency for closeout; do not claim closeout itself.

## Files and systems

- **Create:** E2E fixtures/tests and narrowly scoped regression tests.
- **Update:** command/evidence documentation and durable docs only when actual behavior differs from their authorized descriptions.
- **Avoid changing:** product scope, prior contracts, excluded formats/features, and Forge status beyond this WO evidence.

## Validation and E2E acceptance tests

- **Automated checks:** All verified project format/check/test/build commands plus the representative headless E2E and failure-injection suites.
- **Manual/external checks:** Fresh desktop smoke: launch, write/edit/undo, preview, save/reopen/recover, export PDF/TXT; preserve logs/screenshots as project evidence if policy requires.
- **Acceptance test:** Given a clean runnable workspace, when the representative scene loop runs headlessly and through the focused desktop path, then semantic content survives every boundary, pagination/export are deterministic, failure paths preserve valid state, and no excluded feature is introduced.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `verify unchanged` — update only if the implementation reveals a plainly inaccurate contract.
- **Affected documents/IDs:** All `REQ-WRITING-LOOP-001`–`004`, `NFR-WRITING-001`–`003`, and all relevant blueprints.
- **Rationale and evidence:** This WO compares the implemented system against already approved boundaries; any actual discrepancy must be documented rather than silently ignored.
- **Authorized documentation changes:** Correct factual evidence links or inaccuracies found by the E2E comparison; no new behavior.

## Completion checklist

- [x] Implementation is complete within scope: one bounded headless E2E integration fixture, hostile-input sequence checks, failed-save state-preservation coverage, and removal of the accidental root [`o007_probe.rs`](../../../o007_probe.rs) artifact.
- [x] Tests/checks and required gates have evidence: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo build --workspace`, `cargo test --workspace -- --nocapture` (2 app tests, 22 core unit tests, 3 WO-008 integration tests, doc tests with zero failures), and `cargo clippy --workspace --all-targets -- -D warnings` passed.
- [x] Durable-document impact classification is accurate: requirements and blueprints verified unchanged; this WO adds regression evidence only and introduces no product scope.
- [x] Execution log is appended with dispatch/completion IDs.
- [x] Work order passed fresh independent validation and is `complete`; see [`validation-WO-008.md`](../validations/validation-WO-008.md).

## Implementation evidence

- [`crates/scriptcraft-core/tests/wo008_e2e.rs`](../../../crates/scriptcraft-core/tests/wo008_e2e.rs) exercises the complete headless loop: project creation, scene-heading/action and character/dialogue transitions, Unicode edit, undo/redo, repeated deterministic pagination, native save/reopen, explicit recovery candidate discovery, PDF/TXT exports, dirty-state preservation after export, atomic-save failure preservation, and bounded hostile sequences.
- The hostile sequence test repeats 64 bounded invalid operations (invalid UTF-8 position, out-of-range element, oversized insertion, malformed native JSON) and asserts typed failure/state preservation without production panics.
- The bounded app smoke command `cargo run -p scriptcraft` successfully built and reached `target/debug/scriptcraft`; no visible window or UI interaction could be observed in this terminal-only environment. The process was terminated after the bounded attempt, so visual/manual success is not claimed.
- Temporary root artifact [`o007_probe.rs`](../../../o007_probe.rs) was removed; no lettered fix WO was required because no production defect was discovered.

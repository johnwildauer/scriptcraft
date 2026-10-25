# Work Order: WO-007 — PDF/TXT Exports and Integration

## Metadata

- **Status:** `complete`
- **Assignee:** Export/integration implementer (one fresh session)
- **Phase/order:** 7 of 8; depends on WO-004, WO-005, and WO-006
- **Initiative:** [`../2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Generated filename:** `WO-007-exports-integration.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-007.md`](../validations/validation-WO-007.md)
- **Fixes:** `none`
- **Implementer session/agent ID:** `code-session-2026-10-25-wo-007`
- **Session exception:** `none`

## Summary

Add explicit PDF and deliberately lossy TXT exports using the shared print/layout boundary, then connect them to the native project without changing source identity or clean state. This WO closes the product-level integration gap before final hardening.

## In scope

- Fixed print-layout PDF export, plain-text export with explicit loss communication, export error handling, desktop export actions, and representative integration fixtures.

## Out of scope

- DOCX/FDX, rich/broad formatting, export as save/source, collaboration, network/MCP, planning features, web, and independent pagination implementations.

## Requirements

- **REQ-WRITING-LOOP-004:** “As an individual screenwriter, I want to export a reviewable PDF and a plainly lossy text copy, so that I can inspect or share output without confusing exports with the native source.”
- **AC-WRITING-004.1:** “Given a project, PDF export shall use fixed print layout rather than transient viewport layout.”
- **AC-WRITING-004.2:** “Given a PDF or plain-text export, the system shall not silently replace the native source of truth or falsely mark the native project clean.”
- **AC-WRITING-004.3:** “Given a lossy plain-text export, the result shall communicate that roles, IDs, and page layout are not preserved as a lossless round trip.”

## Blueprints

- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) — `ExportLossBoundary` consumes fixed print layout and reports loss.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) — shell exposes output actions and status without owning source data.

## Implementation plan

1. Implement PDF adapter over WO-004 print layout and record chosen bounded library/output contract.
2. Implement TXT adapter and explicit loss status/report; keep native project/path/dirty state untouched.
3. Connect export actions/errors and test save/edit/export integration.

## Files and systems

- **Create:** export adapters, loss report/status types, fixtures, and app actions/tests.
- **Update:** export blueprint/requirements only if the authorized minimum loss contract or technical choice is resolved.
- **Avoid changing:** native schema, semantic roles, pagination algorithm, and excluded compatibility formats.

## Validation and E2E acceptance tests

- **Automated checks:** Workspace format/check/test/build; fixed-layout PDF determinism, TXT loss-report, failure, and dirty-state tests.
- **Manual/external checks:** Export a representative scene to PDF/TXT and inspect page/layout and explicit loss messaging.
- **Acceptance test:** Given a saved or dirty project, when PDF and TXT export run, then PDF uses the shared fixed print layout, TXT communicates role/ID/layout loss, and neither export changes native source or falsely cleans the session.

## Durable-document impact

- **Requirements impact:** `verify unchanged` — existing export contract is implemented, not broadened.
- **Blueprint impact:** `update` — record adapter/output and loss-report facts.
- **Affected documents/IDs:** `REQ-WRITING-LOOP-004`, `AC-WRITING-004.1`–`004.3`, [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md)
- **Rationale and evidence:** Export library/contract was intentionally open and must be recorded after implementation choice.
- **Authorized documentation changes:** Record chosen adapter and minimum loss communication only; no compatibility scope.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document impact classification is accurate.
- [x] Execution log is appended with dispatch/completion IDs.
- [x] Work order reached `in-review`; completion followed a passed fresh validation.

## Implementation evidence

- Added a dependency-free bounded PDF 1.4 writer in `crates/scriptcraft-core/src/exports.rs`. It consumes `ScreenplayLayout` pages/lines, preserves fixed profile page count/coordinates, emits a deterministic signature/xref/trailer, and is explicitly non-canonical.
- Added `export_text` with a deterministic plain-text copy and `LossReport` reporting role count, stable-ID count, page-layout loss, and an in-band loss message. Both adapters borrow `Project` only and therefore cannot change native source, session revision, or dirty/clean state.
- Added focused tests for deterministic text/loss reporting, deterministic PDF signature/page/content output, and failure isolation. Added only the minimum `Export PDF` and `Export TXT (lossy)` host actions; no FDX/DOCX, MCP, or later hardening was added.
- Changed files: `crates/scriptcraft-core/src/exports.rs`, `crates/scriptcraft-core/src/lib.rs`, `apps/scriptcraft/src/main.rs`, this work order, the feature blueprint, and the execution log. No commit was made.
- Checks passed: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo build --workspace`, `cargo test --workspace -- --nocapture` (2 app tests, 22 core tests, 0 failures, doc tests passed), and `cargo clippy --workspace --all-targets -- -D warnings`.
- Independent validation was pending at implementation completion and is recorded below.

## Independent validation

- **Validator session/agent ID:** `validation-session-2026-10-25-wo-007`
- **Validation artifact:** [`validation-WO-007.md`](../validations/validation-WO-007.md)
- **Verdict:** `PASS`
- **Evidence:** Fresh source inspection and independent probes verified deterministic fixed-layout PDF bytes, PDF signature/EOF/xref/page-count structure, representative layout content, explicit lossy TXT role/ID/page-layout reporting, successful-export dirty-state preservation, failed-export session isolation, and desktop action routing. Fresh `fmt`, `check`, `build`, `test`, and `clippy` gates passed.
- **Result:** No hard-stop implementation, scope, policy-gate, or rogue-behavior finding; no lettered fix work order is required. WO-007 is complete.

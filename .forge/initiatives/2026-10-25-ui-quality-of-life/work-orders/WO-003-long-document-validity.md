# Work Order: WO-003 — Long-Document Validity

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer after WO-002 validation
- **Phase/order:** `3 of 4`
- **Initiative:** [`../ui-quality-of-life-v1.md`](../ui-quality-of-life-v1.md)
- **Generated filename:** `WO-003-long-document-validity.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-003.md`](../validations/validation-WO-003.md)
- **Fixes:** `none`
- **Implementer session/agent ID:** `code-fresh-context-ui-quality-of-life-WO-003-2026-10-25T21:15:17Z`
- **Session exception:** `none`

## Summary

Reproduce the reported failure after several pasted pages and make the editor safe for bounded multi-page documents. The work must identify whether `model (invalid)` comes from malformed input, stale layout/session state, a limit, an unsafe error path, or a UI presentation bug before changing contracts.

## In scope

- Controlled long-text and multi-element fixtures covering pasted newlines, long wrapped elements, page crossings, undo/redo, save/reopen, and recovery discovery.
- Trace and improve error reporting around [`Project::validate()`](../../../crates/scriptcraft-core/src/lib.rs:200), [`ScreenplayLayout::build()`](../../../crates/scriptcraft-core/src/layout.rs:86), serialization/deserialization, and app error presentation.
- Ensure failed edits/layout/save/open leave the last valid project/session usable and the caret/scroll path recoverable.
- Adjust a bounded limit or parsing/layout algorithm only when reproduction proves it is the root cause, with hostile-input regression tests.

## Out of scope

- Unbounded input, removing safety limits, incremental pagination optimization, schema migration unrelated to the reproduced defect, or redesigning recovery UX.

## Requirements

- **REQ-WRITING-LOOP-001:** “As an individual screenwriter, I want to create and edit an ordered screenplay with explicit element roles, so that my draft retains screenplay meaning while I write.”
- **REQ-WRITING-LOOP-002:** “As an individual screenwriter, I want continuous writing and fixed-page preview to agree about semantic content and layout inputs, so that I can write comfortably and inspect a predictable page result.”
- **REQ-WRITING-LOOP-003:** “As an individual screenwriter, I want to save, reopen, and recover my project without losing the last valid version, so that an interrupted session does not destroy my draft.”
- **AC-WRITING-001.5:** “Given malformed or out-of-bound input, the system shall return a bounded error without corrupting the project or session state.”
- **AC-WRITING-002.3:** “Given long text or an oversized element, the system shall wrap or split it safely without an infinite loop or unbounded blank-page behavior.”
- **AC-WRITING-003.1:** “Given a valid project, the system shall serialize a versioned native representation containing semantic content, stable IDs, metadata, and page-profile inputs, while excluding transient caret/layout/undo state from required content.”
- **NFR-WRITING-001:** “Production paths should prefer typed errors and bounded inputs over panic-prone handling; a discovered crash or data-loss defect requires a regression test.”

## Blueprints

- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) — validation, bounded persistence, oversized-element escape hatch, and rollback are core responsibilities.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) — failed open/save/layout operations must not silently replace valid state.

## Implementation plan

1. Add deterministic fixtures that increase page count and paste size separately, recording the first failing boundary and exact error.
2. Exercise model validation, layout construction, event dispatch, serialization, and reopen after every successful and failed operation.
3. Fix the smallest root cause while retaining explicit bounds; make errors specific enough to distinguish invalid model, layout, persistence, and viewport failure.
4. Add a multi-page end-to-end test that reaches the final page, moves the caret there, saves, reopens, and compares semantic content/page structure.

## Files and systems

- **Create:** long-document integration/fixture tests, likely under [`crates/scriptcraft-core/tests/`](../../../crates/scriptcraft-core/tests/).
- **Update:** only the implicated files among [`lib.rs`](../../../crates/scriptcraft-core/src/lib.rs:200), [`editing.rs`](../../../crates/scriptcraft-core/src/editing.rs:283), [`layout.rs`](../../../crates/scriptcraft-core/src/layout.rs:86), [`persistence.rs`](../../../crates/scriptcraft-core/src/persistence.rs:56), and [`main.rs`](../../../apps/scriptcraft/src/main.rs:157).
- **Avoid changing:** limits and schema unless the reproduction proves they are the defect and the change remains bounded.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo build --workspace`.
- **Manual/external checks:** paste several pages of representative lorem ipsum and screenplay-like text, observe the final caret/page, save/reopen, and confirm no `model (invalid)` state or unusable window; record resource/latency observations without making an unmeasured performance promise.
- **Acceptance test:** Given a bounded multi-page paste and subsequent edits, when the document is laid out, scrolled to the caret, saved, and reopened, then the project remains valid or reports a typed recoverable error, no partial/corrupt model is displayed, the final page is reachable, and semantic content survives round trip.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `update`
- **Affected documents/IDs:** [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md)
- **Rationale and evidence:** The safety and boundedness requirements already cover the outcome, but the blueprint must record any discovered validation/limit/oversized-element rule and its evidence. If no contract changes, explicitly record that the existing bounds were sufficient.
- **Authorized documentation changes:** Record the reproduced root cause, retained/changed bounds, and validation fixture links; do not claim arbitrary long-script support beyond measured bounded coverage.

## Completion checklist

- [x] Root cause is reproduced and documented.
- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document impact classification matches the final diff.
- [x] Execution log is appended with dispatch/completion identifiers.
- [x] Work order was `in-review` before fresh validation and became `complete` after the passed validation in [`validation-WO-003.md`](../validations/validation-WO-003.md).

## Implementation evidence — fresh implementer session

- **Session/agent ID:** `code-fresh-context-ui-quality-of-life-WO-003-2026-10-25T21:15:17Z`
- **Changed files:** [`crates/scriptcraft-core/src/layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs), [`crates/scriptcraft-core/src/lib.rs`](../../../../crates/scriptcraft-core/src/lib.rs), [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs), [`crates/scriptcraft-core/tests/wo003_long_document.rs`](../../../../crates/scriptcraft-core/tests/wo003_long_document.rs), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md), and this work order plus [`execution-log.md`](../execution-log.md).
- **Reproduction/root cause:** Controlled fixtures separated element count, encoded text size, wrapped-line/page count, and pasted newline patterns. Element/text/page fixtures stayed valid within existing bounds and reached the final page/caret. LF, CRLF, and CR fixtures showed that layout's prior LF-only split passed `\r` into the wrapper, creating a phantom carriage-return line for CRLF paste and inflating page structure. Separately, valid in-memory model content could encode beyond `MAX_JSON_BYTES` through `Project::to_json()` even though persistence later rejected it. The app layout failure branch painted a raw label without entering the standard recoverable error presentation.
- **Smallest fixes:** layout-only CR/LF delimiter handling; enforce the existing JSON bound at `Project::to_json()`; route layout failures through the unchanged-document app error path. No limits were raised and schema version `1` was not changed.
- **Regression coverage:** [`wo003_long_document.rs`](../../../../crates/scriptcraft-core/tests/wo003_long_document.rs) covers independent fixture dimensions, LF/CRLF/CR semantics, bounded multi-page paste/final caret/native round trip, failed edit rollback, invalid layout preservation, and model JSON bound enforcement.
- **Validation results:** `cargo fmt --all -- --check` passed; `cargo check --workspace` passed; `cargo test --workspace` passed (18 app + 24 core unit + 5 WO-003 + 3 E2E, 50 total); `cargo clippy --workspace --all-targets -- -D warnings` passed; `cargo build --workspace` passed. Native desktop interaction was not available in this implementer session and is not claimed as passed.
- **Comparable diff evidence:** implementation and fixture changes are limited to the files listed above; existing WO-001/WO-002 changes were pre-existing in the fresh session baseline. Fresh validation returned `PASS WITH WARNINGS`; the orchestrator marked this work order `complete`.

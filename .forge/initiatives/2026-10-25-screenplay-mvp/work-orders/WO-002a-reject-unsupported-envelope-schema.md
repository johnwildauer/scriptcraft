# Work Order: WO-002a — Reject Unsupported Envelope Schema

## Metadata

- **Status:** `complete`
- **Assignee:** Model/schema fix implementer (one fresh session)
- **Phase/order:** Fix for WO-002; depends on WO-002 validation failure
- **Initiative:** [`../2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Generated filename:** `WO-002a-reject-unsupported-envelope-schema.md`
- **Owner/orchestrator:** Forge orchestrator
- **Original work order:** [`WO-002-model-schema.md`](WO-002-model-schema.md)
- **Failing validation:** [`../validations/validation-WO-002.md`](../validations/validation-WO-002.md)
- **Implementer session/agent ID:** `code-session-2026-10-25-wo-002a`
- **Session exception:** `none`

## Summary

Correct the WO-002 decoder hard-stop so an unsupported root native envelope
`schema_version` is rejected with the existing typed schema error instead of
being normalized to the current version.

## Fix scope

- Validate the root envelope `schema_version` against
  `CURRENT_SCHEMA_VERSION` during project decoding.
- Add one focused regression test proving a valid project with only the root
  envelope version changed to an unsupported newer value returns
  `ModelError::InvalidSchemaVersion`.
- Record implementation and command evidence in this work order and the
  execution log.

## Out of scope

- Any WO-003 or later behavior.
- Changes to the schema contract, error taxonomy, serialization format, or
  unrelated model validation.
- Refactoring, persistence, UI, editing, pagination, exports, or speculative
  compatibility behavior.

## Required fix

The independent validation found that [`Project::from_json()`](../../../crates/scriptcraft-core/src/lib.rs:217)
accepted a root envelope version of `999` and reconstructed the current version.
The decoder must inspect the root `schema_version` before accepting `project` and
return `ModelError::InvalidSchemaVersion` for unsupported values.

## Files and systems

- **Update:** [`crates/scriptcraft-core/src/lib.rs`](../../../crates/scriptcraft-core/src/lib.rs)
  with the minimal decoder check and regression test.
- **Update:** this work order and the initiative execution log with evidence.
- **Avoid changing:** all later work-order areas and durable requirements or
  blueprint wording.

## Validation and acceptance

- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- Regression test rejects a newer root envelope schema version with
  `ModelError::InvalidSchemaVersion`.

## Durable-document impact

- **Requirements impact:** `verify unchanged` — this fixes enforcement of the
  existing versioned-schema behavior identified by WO-002 validation.
- **Blueprint impact:** `verify unchanged` — no model ownership or contract
  change is introduced.
- **Rationale:** The fix is limited to enforcing the already-established outer
  envelope version boundary.

## Completion checklist

- [x] Minimal implementation and regression test are complete within scope.
- [x] Required checks have evidence.
- [x] Execution log records dispatch/completion and changed files.
- [x] Work order is `in-review`; fresh validation remains required.

## Implementation and validation evidence

- **Completion session/agent ID:** `code-session-2026-10-25-wo-002a`
- **Changed files:** [`crates/scriptcraft-core/src/lib.rs`](../../../crates/scriptcraft-core/src/lib.rs), this work order, and the initiative execution log. No commit was made.
- **Implementation evidence:** The decoder now reads and checks the root envelope
  `schema_version` before decoding `project`, returning
  `ModelError::InvalidSchemaVersion` for unsupported values. A focused regression
  test changes only the outer version to `999` and asserts the typed error.
- **Commands and exact results:** `cargo fmt --all` and
  `cargo fmt --all -- --check` passed; `cargo check --workspace` passed;
  `cargo test --workspace` passed (5 core tests, 0 failures; app and doc tests
  also passed); `cargo clippy --workspace --all-targets -- -D warnings` passed.
- **Independent validation:** [`validation-WO-002a.md`](../validations/validation-WO-002a.md)
  — `pass`; the fresh validator independently confirmed the malformed
  outer-schema regression and all required format/check/test/clippy commands.

## Independent validation result

- **Validation session/agent ID:** `validation-session-2026-10-25-wo-002a`
- **Validation artifact:** [`validation-WO-002a.md`](../validations/validation-WO-002a.md)
- **Result:** `pass` — unsupported root envelope schema versions now return
  `ModelError::InvalidSchemaVersion`; all required independent checks passed.
- **Required next state:** `complete`; the original [`WO-002-model-schema.md`](WO-002-model-schema.md)
  may close with this fix chain as evidence.

# Validation: WO-002a — Reject Unsupported Envelope Schema

## Validation metadata

- **Review type:** `validation`
- **Status:** `complete`
- **Validator:** Independent fresh-context validation agent
- **Validator session/agent ID:** `validation-session-2026-10-25-wo-002a`
- **Session exception:** `none`
- **Date:** 2026-10-25
- **Initiative:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Validation file:** `validation-WO-002a.md`
- **Validated work order:** [`WO-002a-reject-unsupported-envelope-schema.md`](../work-orders/WO-002a-reject-unsupported-envelope-schema.md)
- **Original work order:** [`WO-002-model-schema.md`](../work-orders/WO-002-model-schema.md)
- **Prior failing validation:** [`validation-WO-002.md`](validation-WO-002.md)
- **Compared commit/diff:** No commit exists; the repository is an unborn, fully uncommitted branch, so path-specific `git diff` output is empty. Comparison used the fix work order, prior failing validation, execution log, current source/test implementation, and fresh command results. The reported fix files are limited to [`lib.rs`](../../../crates/scriptcraft-core/src/lib.rs), the fix work order, and the execution log.

## Authorized scope and comparison basis

The fix work order authorizes one decoder boundary correction and one focused
regression test: reject an unsupported root native-envelope `schema_version`
with `ModelError::InvalidSchemaVersion`, without changing the schema contract,
error taxonomy, serialization format, or later work-order behavior.

The implementation was inspected at [`decode_project()`](../../../crates/scriptcraft-core/src/lib.rs:510), where the root version is read and rejected before the
`project` object is decoded, and at the focused test
[`unsupported_outer_schema_version_is_a_typed_error()`](../../../crates/scriptcraft-core/src/lib.rs:573), which changes only the first/root version
from `1` to `999` and asserts the typed error.

## Validation checklist

- Reported implementation, changed-file record, prior validation, execution
  evidence, and current source/test behavior support meaningful comparison:
  **pass**.
- Changed files and behavior are authorized by [`WO-002a-reject-unsupported-envelope-schema.md`](../work-orders/WO-002a-reject-unsupported-envelope-schema.md): **pass**. The change is limited to the existing reusable decoder and a focused unit test.
- No clearly deceptive, backdoor, malicious, nefarious, or otherwise rogue
  behavior was found: **pass**. No network, process, filesystem, hidden-state,
  UI, or unrelated product behavior was introduced.
- Explicit acceptance criteria and required checks: **pass**. The malformed
  outer-schema regression and all four required workspace checks passed.
- Durable-document impact classification: **pass**. The fix enforces the
  already-established version boundary and does not change requirements or the
  blueprint contract.

## Independent acceptance evidence

All checks below were run independently in this fresh validation session.

| Check | Result | Evidence |
|---|---|---|
| Malformed outer-schema regression | PASS | `cargo test -p scriptcraft-core tests::unsupported_outer_schema_version_is_a_typed_error -- --exact`; 1 test passed, 0 failed. |
| `cargo fmt --all -- --check` | PASS | Exit code 0. |
| `cargo check --workspace` | PASS | Exit code 0. |
| `cargo test --workspace` | PASS | Exit code 0; 5 core tests passed, 0 failed; app unit tests and doc tests passed. |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | Exit code 0. |

The regression test proves that a valid project whose only changed field is the
root envelope version `999` returns `Err(ModelError::InvalidSchemaVersion)`.
The workspace test run independently confirms the complete five-test core suite
including that regression.

## Diagnostic hypothesis log

Possible sources considered before narrowing the validation:

1. The fix was absent or applied to an unrelated decoder path.
2. The root version check existed but accepted unsupported values.
3. The focused test changed an inner project version rather than the outer one.
4. The typed error assertion was too weak or tested a different error path.
5. Formatting, workspace compilation, test, or lint regressions were introduced.
6. The fix changed serialization or later-WO behavior outside scope.

Source inspection narrowed the relevant risks to the root check and the focused
test. The root field is validated against `CURRENT_SCHEMA_VERSION` before the
project payload is accepted, and the regression mutates only the first serialized
root field. The independent commands then ruled out the remaining command-suite
and scope risks.

## Findings

### Blockers

None.

### Warnings

- **Uncommitted comparison evidence** — Severity: warning. Evidence: the branch
  has no commits and `git diff` is empty because all repository paths are
  untracked. Recommendation: retain the explicit changed-file and source
  comparison evidence; establish normal commit evidence before a validation that
  requires commit-to-commit comparison. Required next state: none for this fix.

### Notes

- The prior hard-stop from [`validation-WO-002.md`](validation-WO-002.md) is
  directly covered by the new root-envelope check and focused regression.
- No later work-order behavior was inspected or assessed.

## Decision

- **Outcome:** `pass`
- **Required next state:** [`WO-002a-reject-unsupported-envelope-schema.md`](../work-orders/WO-002a-reject-unsupported-envelope-schema.md) → `complete`; [`WO-002-model-schema.md`](../work-orders/WO-002-model-schema.md) → `complete` with this fix chain as evidence.
- **Fix work order required:** `no`

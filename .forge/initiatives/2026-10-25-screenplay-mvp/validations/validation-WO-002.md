# Validation: WO-002 — Semantic Model and Schema

## Validation metadata

- **Review type:** `validation`
- **Status:** `complete`
- **Validator:** Independent fresh-context validation agent
- **Validator session/agent ID:** `validation-session-2026-10-25-wo-002`
- **Session exception:** `none`
- **Date:** 2026-10-25
- **Initiative:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Validation file:** `validation-WO-002.md`
- **Validated work order:** [`WO-002-model-schema.md`](../work-orders/WO-002-model-schema.md)
- **Compared commit/diff:** No commit exists and the repository is still uncommitted, so `git diff -- crates/scriptcraft-core/src/lib.rs` was empty. Comparison used the WO-002 changed-file record, execution log, authorized requirements/blueprints, current implementation in [`lib.rs`](../../../crates/scriptcraft-core/src/lib.rs), and independent command/probe output. The reported changed files were the implementation, WO-002, and the execution log; no unrelated product files were found in the status output.
- **Fix chain:** Required but not implemented: `WO-002a-reject-unsupported-envelope-schema`.

## Authorized scope

WO-002 authorizes the reusable dependency-free semantic project model and versioned
schema: project/title metadata, ordered title-page and screenplay elements, stable
opaque IDs, seven explicit roles, page-profile inputs, bounded typed validation,
deterministic serialization, and malformed-input coverage. It excludes editing
sessions, pagination, UI, filesystem persistence, and exports. The comparison also
assessed the explicit acceptance criteria for a valid new project, role and canonical
UTF-8 preservation, typed bounded errors, and absence of transient UI/session state.

The implementation is aligned in file and behavioral scope, but the schema-version
acceptance boundary has one hard-stop defect described below.

## Validation checklist

- The reported work, changed-file record, execution evidence, and implementation
  support a meaningful comparison: **pass**. The implementation is present and the
  work order/execution log identify the same core source file and focused tests.
- Changed files and behavior are authorized by WO-002: **pass**. The implementation
  contains only the reusable semantic model/parser/tests; no later product area or
  rogue behavior was introduced.
- No clearly deceptive, backdoor, malicious, nefarious, or otherwise rogue behavior
  was found: **pass**. The inspected code is dependency-free and has no network,
  process, filesystem, hidden-state, or UI behavior.
- Explicit acceptance criteria and required project-policy/manual gates: **fail**.
  New-project, role, Unicode, deterministic round-trip, malformed-field, and bound
  checks passed, but an unsupported outer schema version is accepted instead of
  returning a typed schema error.
- Durable-document impact is not otherwise contradicted: **pass with the schema
  version exception**. The implementation establishes the claimed model facts, but
  the versioned envelope contract is not enforced at decode.

## Independent acceptance evidence

All commands and the targeted probe were run independently in this fresh validation
session. The probe was compiled from stdin and emitted no repository source file.

| Check | Result | Evidence |
|---|---|---|
| `cargo fmt --all -- --check` | PASS | Exit code 0. |
| `cargo check --workspace` | PASS | Exit code 0. |
| `cargo test --workspace` | PASS | Exit code 0; 4 core tests passed, 0 failed; app and doc tests passed. |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | Exit code 0. |
| `cargo test -p scriptcraft-core new_project_has_valid_start` | PASS | New-project invariant test passed. |
| `cargo test -p scriptcraft-core deterministic_unicode_round_trip` | PASS | Unicode, deterministic JSON, and UTF-8 boundary test passed. |
| `cargo test -p scriptcraft-core all_roles_encode` | PASS | All seven role variants round-tripped. |
| `cargo test -p scriptcraft-core hostile_bounds_are_typed_errors_without_panics` | PASS | Oversized JSON and empty-ID hostile inputs returned typed errors. |
| `cargo test -p scriptcraft-core --doc` | PASS | 0 doc tests, no failures. |
| Fresh malformed/schema probe | **FAIL** | Baseline decode, inner schema `999`, missing-field, and Unicode cases behaved as expected; changing only the outer envelope `schema_version` to `999` returned `Ok(Project { schema_version: 1, ... })` rather than `Err(ModelError::InvalidSchemaVersion)`. |

## Diagnostic hypothesis log

The independent investigation considered these possible sources before narrowing the
finding:

1. Formatting/configuration drift.
2. Workspace compilation or dependency-boundary drift.
3. Role/ordering/ID round-trip loss.
4. Unicode parsing or invalid text-boundary handling.
5. Malformed JSON and bounded-input error handling.
6. Inner project schema-version validation.
7. Outer native envelope schema-version validation.

The first five sources were checked by the command suite, focused tests, source
inspection, and the probe and did not produce a failure. The two most likely schema
sources were then isolated: the probe rejected an inner project version of `999` with
`InvalidSchemaVersion`, but accepted an outer envelope version of `999` and silently
returned a current-version project. Source inspection confirms [`decode_project()`](../../../crates/scriptcraft-core/src/lib.rs:510)
reads the inner `project.schema_version` but never reads or validates the root
`schema_version`; [`NativeProjectEnvelope::from_json()`](../../../crates/scriptcraft-core/src/lib.rs:248)
also reconstructs the current version unconditionally.

No product code was changed and no fix was attempted. Confirmation before a fix is
not applicable to this validation-only dispatch; the required implementation fix is
recorded for a new implementer session.

## Findings

### Blockers

- **Unsupported outer schema version accepted** — Severity: **blocker**. Evidence:
  the fresh probe changed the serialized root `schema_version` from `1` to `999`
  while leaving the project payload valid, and [`Project::from_json()`](../../../crates/scriptcraft-core/src/lib.rs:217)
  returned `Ok(Project { schema_version: 1, ... })`. The decoder obtains the root
  object only to select `project` and does not validate its schema version. This
  violates the versioned-schema contract and can silently reinterpret a future
  envelope as the current schema. Recommendation: in a new fix session, decode and
  validate the root envelope schema version before accepting the project, preserving
  typed `ModelError::InvalidSchemaVersion` behavior and adding a regression test.
  Required next state: `in-progress` under `WO-002a-reject-unsupported-envelope-schema`.

### Warnings

- **Uncommitted comparison evidence** — Severity: warning. Evidence: the repository
  has no commit and the implementation diff is empty. Recommendation: retain the
  changed-file record and establish a normal commit before validations that require
  commit-to-commit evidence. Required next state: `in-review` pending the fix chain.

### Notes

- Manual serialized-fixture inspection was performed against the encoder in
  [`encode_envelope()`](../../../crates/scriptcraft-core/src/lib.rs:318): it contains
  semantic metadata, profile inputs, IDs, roles, and text, with no caret, layout,
  undo, or UI fields.
- The passing focused tests and source inspection support the claimed new-project,
  seven-role, canonical Unicode, deterministic serialization, and typed-boundary
  behavior. No later-WO obligations were assessed as missing.

## Decision

- **Outcome:** `fail`
- **Required next state:** `in-review` until the lettered fix work order passes fresh validation
- **Fix work order required:** `yes: WO-002a-reject-unsupported-envelope-schema` (name specified; fix work-order artifact not created under the validation-only instruction)

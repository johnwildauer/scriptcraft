# Work Order: WO-002 — Semantic Model and Schema

## Metadata

- **Status:** `complete`
- **Assignee:** Model/schema implementer (one fresh session)
- **Phase/order:** 2 of 8; depends on WO-001
- **Initiative:** [`../2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Generated filename:** `WO-002-model-schema.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-002.md`](../validations/validation-WO-002.md) — `fail`; resolved by passed fix validation [`../validations/validation-WO-002a.md`](../validations/validation-WO-002a.md)
- **Fixes:** [`WO-002a-reject-unsupported-envelope-schema.md`](WO-002a-reject-unsupported-envelope-schema.md) — `complete`; passed fresh validation
- **Implementer session/agent ID:** `code-session-2026-10-25-wo-002`
- **Session exception:** `none`

## Summary

Define the durable semantic screenplay project and versioned schema in reusable Rust support layers. The model must carry explicit roles, canonical UTF-8 text, stable IDs, title metadata, and page-profile inputs without transient session/UI state.

## In scope

- Project metadata, title-page metadata, ordered elements, stable opaque IDs, seven semantic roles, schema version, and bounded invariants.
- Typed validation and deterministic serialization primitives sufficient for later persistence.

## Out of scope

- Editing commands/session history, pagination, desktop UI, filesystem atomicity/recovery, PDF/TXT, and all excluded product areas.

## Requirements

- **REQ-WRITING-LOOP-001:** “As an individual screenwriter, I want to create and edit an ordered screenplay with explicit element roles, so that my draft retains screenplay meaning while I write.”
- **AC-WRITING-001.1:** “Given a new project, the system shall provide a valid editable starting element and title-page metadata without requiring visual formatting tricks.”
- **AC-WRITING-001.2:** “Given an element, the system shall preserve its explicit role and canonical UTF-8 text independently of capitalization, spaces, tabs, indentation, or font appearance.”
- **NFR-WRITING-001:** “Production paths should prefer typed errors and bounded inputs over panic-prone handling; a discovered crash or data-loss defect requires a regression test.”
- **NFR-WRITING-003:** “Native serialization and structural pagination outputs shall be deterministic for the same semantic project/profile inputs.”

## Blueprints

- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) — `ProjectModel` owns semantic durable data and validation.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) — `Project` and `ScreenplayElement` contract.

## Implementation plan

1. Add typed model/schema types, role enum, stable IDs, metadata, page-profile input placeholder, and new-project constructor.
2. Add bounded validation and deterministic encode/decode-ready representation while excluding caret/layout/undo/UI state.
3. Add headless model fixtures covering all roles, UTF-8, identity, ordering, and malformed input errors.

## Files and systems

- **Create:** model/schema modules and unit tests under the WO-001 reusable crate boundary.
- **Update:** blueprint references only if implementation resolves a documented contract detail.
- **Avoid changing:** desktop app, layout engine, persistence filesystem, and requirement wording.

## Validation and E2E acceptance tests

- **Automated checks:** Workspace format/check/test; model round-trip/determinism and malformed-input tests.
- **Manual/external checks:** Inspect serialized test fixtures to confirm transient state is absent.
- **Acceptance test:** Given a new project and each supported role, when the model is encoded/decoded, then roles, canonical UTF-8 text, IDs, metadata, and profile inputs remain equivalent and invalid bounds return typed errors without mutation.

## Durable-document impact

- **Requirements impact:** `verify unchanged` — behavior is an implementation of existing semantic requirements.
- **Blueprint impact:** `update` — record concrete schema/model ownership and verified types where the provisional blueprint becomes factual.
- **Affected documents/IDs:** `REQ-WRITING-LOOP-001`, `AC-WRITING-001.1`, `AC-WRITING-001.2`, [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md)
- **Rationale and evidence:** Model implementation establishes durable semantic contracts previously marked desired.
- **Authorized documentation changes:** Update only concrete model/schema facts and evidence; do not expand role scope.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document impact classification is accurate.
- [x] Execution log is appended with dispatch/completion IDs.
- [x] Work order is `in-review`; completion requires a passed fresh validation.

## Implementation and validation evidence

- **Completion session/agent ID:** `code-session-2026-10-25-wo-002`
- **Changed files:** [`crates/scriptcraft-core/src/lib.rs`](../../../crates/scriptcraft-core/src/lib.rs), this work order, and the initiative execution log. No commit was made.
- **Model evidence:** Added dependency-free `Project`, `NativeProjectEnvelope`, `Metadata`, `PageProfile`, `ScreenplayElement`, `StableId`, and the seven explicit `Role` variants. The model provides a valid new-project action element, stable opaque IDs, title-page data, fixed profile inputs, bounded structural/text validation, Unicode-safe byte-boundary checks, deterministic native JSON encoding, and bounded typed decoding errors. Serialization contains no caret, layout, undo, or UI state.
- **Focused tests:** Four core unit tests cover valid new-project invariants, all roles and round-trip identity, deterministic Unicode JSON round-trip and UTF-8 boundaries, and hostile input bounds/typed errors.
- **Commands and exact results:** `cargo fmt --all` — passed; `cargo fmt --all -- --check` — passed; `cargo check --workspace` — passed; `cargo test --workspace` — passed (4 core tests, 0 failures; app/doc tests also passed); `cargo clippy --workspace --all-targets -- -D warnings` — passed. An initial test run exposed only an incorrect test offset assertion; the focused test was corrected and the complete command sequence then passed.
- **Manual/external validation:** The initial independent fresh validation is recorded in [`validation-WO-002.md`](../validations/validation-WO-002.md) and found the outer-envelope schema-version defect. The lettered fix and passed fresh validation are recorded in [`WO-002a-reject-unsupported-envelope-schema.md`](WO-002a-reject-unsupported-envelope-schema.md) and [`validation-WO-002a.md`](../validations/validation-WO-002a.md), respectively.

## Independent validation result

- **Initial validation session/agent ID:** `validation-session-2026-10-25-wo-002`
- **Initial validation artifact:** [`validation-WO-002.md`](../validations/validation-WO-002.md)
- **Initial result:** `fail` — the decoder accepted an unsupported outer envelope schema version.
- **Fix validation session/agent ID:** `validation-session-2026-10-25-wo-002a`
- **Fix validation artifact:** [`validation-WO-002a.md`](../validations/validation-WO-002a.md)
- **Overall result:** `complete` — the lettered fix passed fresh validation and closes the original blocker.

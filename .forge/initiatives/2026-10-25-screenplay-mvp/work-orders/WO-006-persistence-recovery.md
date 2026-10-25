# Work Order: WO-006 — Native Persistence and Recovery

## Metadata

- **Status:** `complete`
- **Assignee:** Persistence/recovery implementer (one fresh session)
- **Phase/order:** 6 of 8; depends on WO-002 and WO-003; consumes UI seam from WO-005
- **Initiative:** [`../2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Generated filename:** `WO-006-persistence-recovery.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-006.md`](../validations/validation-WO-006.md)
- **Fixes:** `none`
- **Implementer session/agent ID:** `code-session-2026-10-25-wo-006`
- **Session exception:** `none`

## Summary

Implement the versioned native JSON source of truth and its safe save/reopen/recovery behavior. Separate pure schema/bytes logic from host filesystem services, use atomic replacement, preserve the prior valid destination on failure, and offer bounded recovery candidates without silent overwrite.

## In scope

- Versioned native JSON schema, bounded parse/validation, reopen semantic equivalence, atomic sibling-temp replacement, dirty-state semantics, and bounded recovery snapshots with provenance/freshness/candidate-offer behavior.

## Out of scope

- DOCX/FDX, cloud/collaboration, encryption/authentication, exports, UI redesign, and recovery as an automatic silent replacement.

## Requirements

- **REQ-WRITING-LOOP-003:** “As an individual screenwriter, I want to save, reopen, and recover my project without losing the last valid version, so that an interrupted session does not destroy my draft.”
- **AC-WRITING-003.1:** “Given a valid project, the system shall serialize a versioned native representation containing semantic content, stable IDs, metadata, and page-profile inputs, while excluding transient caret/layout/undo state from required content.”
- **AC-WRITING-003.2:** “Given a save preparation or replacement failure, the system shall preserve the prior valid destination and retain an unsaved/dirty indication.”
- **AC-WRITING-003.3:** “Given a successfully saved native project, reopening it shall preserve semantic roles, text, stable IDs, metadata, and reproducible pagination inputs.”
- **AC-WRITING-003.4:** “Given a newer valid bounded recovery snapshot after interruption, the system shall offer it as a candidate and shall not silently overwrite the named project.”
- **NFR-WRITING-001:** “Production paths should prefer typed errors and bounded inputs over panic-prone handling; a discovered crash or data-loss defect requires a regression test.”

## Blueprints

- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) — `NativePersistence` owns schema, migrations, atomic save, and bounded candidates.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) — host owns filesystem/path services while core contracts bytes and status.

## Implementation plan

1. Implement versioned JSON encoding/decoding, bounds, migration hook, and semantic round-trip comparison.
2. Implement injected file operations, atomic save/replacement, dirty-state behavior, and failure injection.
3. Implement bounded recovery rotation/provenance and explicit candidate offer/accept/discard flow; connect app open/save affordances.

## Files and systems

- **Create:** persistence/schema adapter, filesystem abstraction, recovery manager, fixtures, and failure tests.
- **Update:** persistence/recovery blueprint and manifest with verified extension/contract facts.
- **Avoid changing:** layout algorithm, export formats, excluded compatibility, and transient-state serialization.

## Validation and E2E acceptance tests

- **Automated checks:** Workspace format/check/test; deterministic JSON, reopen equivalence, replacement failure, dirty-state, rotation, and recovery candidate tests.
- **Manual/external checks:** Save, reopen, interrupt/recover, and decline recovery in the desktop smoke path; verify named project is not silently overwritten.
- **Acceptance test:** Given a valid edited project and injected prepare/replace failures, when native save/reopen/recovery runs, then the prior valid file survives failures, semantic fields round-trip, and newer recovery is offered explicitly.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `update`
- **Affected documents/IDs:** `REQ-WRITING-LOOP-003`, `AC-WRITING-003.1`–`003.4`, [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md)
- **Rationale and evidence:** Persistence and recovery ownership was a reviewed warning and becomes concrete here.
- **Authorized documentation changes:** Record extension, schema version, filesystem boundary, bounds, and recovery provenance/rotation facts only.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document impact classification is accurate.
- [x] Execution log is appended with dispatch/completion IDs.
- [x] Work order passed fresh validation and is `complete`.

## Implementation evidence

- **Session/agent ID:** `code-session-2026-10-25-wo-006`
- **Changed files:** [`crates/scriptcraft-core/src/persistence.rs`](../../../crates/scriptcraft-core/src/persistence.rs), [`crates/scriptcraft-core/src/editing.rs`](../../../crates/scriptcraft-core/src/editing.rs), [`crates/scriptcraft-core/src/lib.rs`](../../../crates/scriptcraft-core/src/lib.rs), [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs), this work order, the authorized persistence blueprints, and the execution log. No commit was made.
- **Implementation:** Added dependency-free bounded native JSON byte serialization/deserialization, injected [`FileSystem`](../../../crates/scriptcraft-core/src/persistence.rs) operations, sibling-temp atomic replacement, explicit session clean/load hooks, bounded revision-named recovery rotation and corruption-skipping candidate discovery. The app now has minimum host-owned Open/Save/Recovery affordances; recovery remains an explicit offer and never silently replaces the named project. PDF/TXT export was not implemented.
- **Tests:** Injected filesystem tests cover semantic round trip, malformed/unknown schema rejection, failed replacement preserving the destination, recovery rotation/corruption handling, and serialization not mutating session state.
- **Checks:** `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo build --workspace`, `cargo test --workspace -- --nocapture` (2 app tests, 19 core tests, 0 failures), and `cargo clippy --workspace --all-targets -- -D warnings` passed. Independent validation was not performed in this implementation session.
- **Status:** `complete`; fresh independent validation is recorded below.

## Independent validation evidence

- **Validator session/agent ID:** `validation-session-2026-10-25-wo-006`
- **Validation artifact:** [`validation-WO-006.md`](../validations/validation-WO-006.md)
- **Verdict:** `PASS`; standalone filesystem/session probe and all requested
  format/check/build/test/clippy gates passed. No hard-stop finding or lettered
  fix work order is required. A stale UI status label and unavailable visual
  smoke path remain warnings only.

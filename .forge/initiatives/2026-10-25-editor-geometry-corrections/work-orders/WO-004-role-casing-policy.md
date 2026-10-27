# Work Order: WO-004 — Role Casing Policy

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer after WO-003 validation
- **Phase/order:** `4 of 6`
- **Initiative:** [`../editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md)
- **Validation:** [`../validations/validation-WO-004-v2.md`](../validations/validation-WO-004-v2.md)

## Summary

Normalize Scene Heading and Character text to uppercase as a project-default editing policy while preserving a future persisted enable/disable setting seam.

## In scope

- Add a bounded project/profile policy field or equivalent default seam for role casing, selecting enabled by default.
- Apply normalization transactionally on insertion, paste, role assignment, and role-changing transitions where the resulting role is Scene Heading or Character.
- Preserve UTF-8 validity, selection offsets, undo/redo, save/reopen, and existing text for other roles.
- Define whether normalization is Unicode-aware and test the selected bounded behavior explicitly.

## Out of scope

- User-facing settings UI, locale-specific title casing, changing existing files on open without a migration decision, or formatting-based uppercase rendering.

## Requirements and blueprints

- **Requirements:** `REQ-WRITING-LOOP-001`, `AC-WRITING-001.3`, `NFR-WRITING-001` in [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md).
- **Blueprints:** [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md).

## Validation

- Test insertion, paste, role assignment, Enter/Tab transitions, Unicode/casing boundaries, undo/redo, and persistence round trips.
- Prove non-cue roles preserve source casing.
- Run all Cargo gates.

## Durable-document impact

- **Requirements:** `update` — the enabled-by-default canonical casing policy is a new user-facing editing contract and must be recorded with its compatibility boundary.
- **Blueprint:** `update` with the policy seam and canonical-text rule.

## Reconciled handoff

- **Applicable requirements (verbatim):** `REQ-WRITING-LOOP-001`: “As an individual screenwriter, I want to create and edit an ordered screenplay with explicit element roles, so that my draft retains screenplay meaning while I write.” `AC-WRITING-001.2`: “Given an element, the system shall preserve its explicit role and canonical UTF-8 text independently of capitalization, spaces, tabs, indentation, or font appearance.” `AC-WRITING-001.3`: “Given a valid selection, the system shall support insertion, deletion, split, join, role change, undo, and redo while preserving valid text boundaries and element order.” `NFR-WRITING-001`: “Production paths should prefer typed errors and bounded inputs over panic-prone handling; a discovered crash or data-loss defect requires a regression test.”
- **Exact oracle:** Persist `uppercase_cues = true` in the profile/settings seam for new projects; legacy missing field decodes as true; existing text is not rewritten on open. When enabled, insertion/paste/role assignment/role transitions normalize only Scene Heading and Character text using Rust Unicode uppercase, remapping caret to the transformed prefix boundary. Other roles preserve source casing.
- **Rationale/evidence:** The existing requirement protects canonical UTF-8 text, so this is a deliberate product-policy refinement that must update requirements if retained as a frozen behavior; Unicode expansion and selection mapping are explicit test obligations.
- **Authorized documentation changes:** Update [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md), and requirements only if the policy is promoted beyond this initiative; record compatibility and tests in this WO.
- **Native evidence boundary:** Core tests, persistence round trips, and Cargo gates are required; no native visual gate is required for canonical casing.

## Validation outcome

- Fresh validation [`validation-WO-004-v2.md`](../validations/validation-WO-004-v2.md) returned `PASS WITH WARNINGS`.
- Warnings cover gates recorded rather than independently rerun in the inspection context and unavailable standalone diff evidence; no fix work order is required.
- State transition: `in-review → complete`.

## Implementation evidence — 2026-10-26

- **Changed implementation files:** [`crates/scriptcraft-core/src/lib.rs`](../../../../crates/scriptcraft-core/src/lib.rs), [`crates/scriptcraft-core/src/editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs), and [`crates/scriptcraft-core/tests/wo004_role_casing.rs`](../../../../crates/scriptcraft-core/tests/wo004_role_casing.rs).
- **Policy seam:** `PageProfile.uppercase_cues` is bounded as a boolean, defaults to `true` for new projects, is serialized in native schema v1, and decodes a missing legacy field as `true`; malformed present values return typed model errors.
- **Editing behavior:** Existing text is untouched during decode/open. Transactional insertion/paste, role assignment, Tab role transitions, and Enter-created cue elements normalize only Scene Heading and Character text when enabled. Rust `char::to_uppercase()` is used, and both caret/selection endpoints are remapped at transformed-prefix UTF-8 boundaries. Other roles and disabled policy preserve source casing.
- **Transactional invariants:** Normalization runs inside the existing clone/validate/history boundary, so failed edits leave project, selection, dirty state, revision, undo, and redo state unchanged; successful casing edits are undoable and redoable.
- **Deterministic evidence:** [`wo004_role_casing.rs`](../../../../crates/scriptcraft-core/tests/wo004_role_casing.rs) covers ASCII insertion/paste, Unicode expansion and caret mapping, role assignment/Tab transition, undo/redo, disabled/non-cue preservation, legacy decode, no-open-rewrite, and save/reopen. Existing unit/persistence tests cover deterministic JSON and typed malformed-input behavior.
- **Durable documentation:** [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) records the canonical editing policy and persistence seam. [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md) retains the semantic UTF-8/role requirement and records the approved cue-policy refinement in its traceability notes; no schema migration or UI setting was added.
- **Cargo evidence:** `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace` (82 tests plus 0 doc tests), `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo build --workspace` passed. Native visual evidence is not applicable to canonical casing and is not claimed.
- **Handoff:** WO-004 is `in-review`; fresh independent validation remains required. WO-005 and WO-006 were not implemented.

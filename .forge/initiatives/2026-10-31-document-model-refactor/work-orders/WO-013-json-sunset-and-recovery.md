# Work Order: WO-013 — Retire JSON native paths and reconcile recovery

- **Status:** `implementation recorded; user review and validation pending` — depends on the WO-012 desktop cutover at `521774d2719cf68c9309411ecb1d40248d822474`, which is itself unvalidated.

## Outcome
Remove obsolete native JSON dependencies without offering legacy migration or unsafe recovery.

## Scope
Inventory and remove or isolate JSON codec, filters, tests, diagnostics and recovery dependencies following FDX cutover. Retain recovery only if it captures complete model and preservation state safely; otherwise disable it explicitly without treating old JSON artifacts as migration.

## Relevant references
- [User JSON decision](../document-model-refactor-v1.md#user-responses-to-the-readiness-questions)
- [Recovery code](../../../../crates/scriptcraft-core/src/persistence.rs)

## Implementation

- Desktop Open filters only FDX and rejects JSON by bounded content detection before any JSON decode; the file and active session remain untouched. JSON destinations are rejected on native save. The defensive FDX-save guard for an already-live JSON-origin session remains, because its retired identity/profile state has no FDX mapping.
- Desktop diagnostics now report only live model validity and FDX serialization availability; they do not attempt a legacy JSON projection.
- The legacy JSON disk writer and snapshot recovery entry points now return explicit retired/unavailable errors without filesystem reads, writes, rotation, cleanup or deletion. Recovery discovery does not offer the old JSON snapshots as candidates. Existing user JSON and `.scriptcraft-recovery-*.json` files remain untouched and are not interpreted as FDX or as complete recovery.
- The bounded JSON codec and compatibility fixture uses remain in the core for existing internal compatibility tests and are documented as non-native legacy helpers; they are not called by desktop Open, Save, diagnostics or recovery. This is a code-level compatibility residue, not a supported migration/import promise.
- Regression coverage was adjusted to remove JSON native-save/recovery expectations and assert that retired persistence preserves existing artifacts. Existing fixture files were retained.

## Acceptance criteria
- Native dialogs, save and diagnostics no longer depend on JSON; legacy JSON is rejected with a clear bounded error rather than misidentified as FDX.
- Recovery cannot restore a partial document as though it were complete; legacy recovery discovery/writes are disabled without touching artifacts; no user data files are deleted.
- Regression tests were updated, but no tests, builds, lint, formatting checks, manual verification, reviews or other validation were run, per repository-wide user direction.

## Useful checks
- Search all JSON/recovery references; tests for old snapshot handling, safe FDX recovery if retained, diagnostics privacy, and workspace checks.

## Dependency and handoff
Depends on WO-012, which is recorded as implemented but has not been user-reviewed or validated. WO-013's JSON codec compatibility residue, lack of replacement recovery, and inability to restore old snapshots are handed to WO-014/user review. Implementation changes are intended for one dedicated commit; no checks are to be run by the agent under current repository policy.

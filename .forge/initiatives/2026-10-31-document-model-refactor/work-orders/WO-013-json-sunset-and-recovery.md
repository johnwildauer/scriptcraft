# Work Order: WO-013 — Retire JSON native paths and reconcile recovery

## Outcome
Remove obsolete native JSON dependencies without offering legacy migration or unsafe recovery.

## Scope
Inventory and remove or isolate JSON codec, filters, tests, diagnostics and recovery dependencies following FDX cutover. Retain recovery only if it captures complete model and preservation state safely; otherwise disable it explicitly without treating old JSON artifacts as migration.

## Relevant references
- [User JSON decision](../document-model-refactor-v1.md#user-responses-to-the-readiness-questions)
- [Recovery code](../../../../crates/scriptcraft-core/src/persistence.rs)

## Implementation
No legacy import or JSON writing promise. Make remaining internal-only uses explicit and harmless or remove them; never parse a legacy snapshot as a complete FDX recovery. Preserve unrelated existing user files and do not delete recovery artifacts. If replacement recovery is retained, preserve source payload, anchors, identities and dirty semantics and apply safe error handling.

## Acceptance criteria
- Native dialogs, save and diagnostics no longer depend on JSON; legacy JSON is rejected with a clear bounded error rather than misidentified as FDX.
- Recovery cannot restore a partial document as though it were complete; no user data files are deleted. Workspace builds and regression tests reflect the sunset.

## Useful checks
- Search all JSON/recovery references; tests for old snapshot handling, safe FDX recovery if retained, diagnostics privacy, and workspace checks.

## Dependency and handoff
Depends on WO-012. One dedicated commit and fresh validation; hand remaining recovery limitations to WO-014.

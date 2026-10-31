# Work Order: WO-002 — Directed content model

## Outcome
Represent all directed screenplay content types and identity invariants in the headless model.

## Scope
Add distinct roles for New Act, End of Act, Cast List, General, Shot, Sequence, Outline 1–3 and Summary alongside existing roles. Extend role-indexed profile data, validation and stable ID allocation, including title/body collisions. Preserve existing session and native JSON behavior as transitional scaffolding, not as the target format.

## Relevant references
- [Content requirements](../../../requirements/features/screenplay-writing-loop.md#fdx-document-content-and-preservation)
- [Current model](../../../../crates/scriptcraft-core/src/lib.rs)

## Implementation
Use WO-001 defaults and add bounded structures for ordered runs and source provenance needed by subsequent orders; retain a compatible plain-text projection while editing clients are migrated. No importer or native cutover here. Existing JSON callers remain buildable; scaffold new variants without making a JSON migration promise.

## Acceptance criteria
- All 17 roles remain distinct and validated; IDs cannot collide across relevant application domains; source case and text are not normalized merely by constructing a model.
- Existing editing and layout clients compile; existing roles and policies remain behaviorally unchanged.
- Transitional JSON save rejects any richer document it cannot faithfully represent; it never silently discards the new state. Test this guard before exposing any new model operation.

## Useful checks
- Unit tests for roles, limits, identity collisions, Unicode and prior defaults; run applicable workspace tests and build.

## Dependency and handoff
Depends on WO-001. One dedicated commit, fresh validation; expose the documented model contract for WO-003–WO-007.

# Work Order: WO-009 — Scene cast derivation and save candidate

## Outcome
Implement periodic scene cast-list updates and detached save-time correction.

## Scope
Collect Character elements after each heading until the next or end, deduplicate names and sort alphabetically. Preserve imported list text on import; subsequent managed-list updates may silently replace direct edits or mismatches. Use a documented bounded default for collation, whitespace, list ownership, multiple lists and refresh scheduling, consistent with the user's decisions.

## Relevant references
- [Cast decisions](../document-model-refactor-v1.md#final-user-responses-and-remaining-implementation-risks)
- [Cast acceptance](../../../requirements/features/screenplay-writing-loop.md#fdx-document-content-and-preservation)

## Implementation
Schedule roughly one-second updates or an equivalent bounded trigger; do not modify global Cast, SmartType or board records. Provide an API to check and silently correct a detached candidate for both Save and Save As. Integrate refresh with undo/redo, selection, pending edits and dirty state; no background undo noise. Publication of a saved candidate belongs to WO-012.

## Acceptance criteria
- Import is as-is; periodic update and candidate correction reflect scene boundaries, all cues and final scene. Direct edits can be corrected silently without mismatch warnings.
- Candidate preparation does not mutate the live session; deterministic correction handles pending refresh and is not a verify-only operation.

## Useful checks
- Duplicate/case/Unicode cues, empty names, multiple scenes/lists, boundaries, direct edits, role changes, insert/delete/reorder, timer races and undo/redo tests.

## Dependency and handoff
Depends on WO-004 and WO-006. One dedicated commit and fresh validation; provide corrected candidate and scheduling APIs to WO-012.

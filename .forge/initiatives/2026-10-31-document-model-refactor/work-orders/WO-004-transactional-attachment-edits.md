# Work Order: WO-004 — Transactional edits and retained attachments

## Outcome
Make edits, snapshots and undo/redo safe for the richer model and its source attachments.

## Scope
Implement and document anchor transformations and identity behavior for insert, delete, replace, split, join, role changes and cross-element edits. Include clean/dirty state and preservation state in session transactions. Keep board data independent and title-page editing deferred.

## Relevant references
- [Edit and user independence decisions](../document-model-refactor-v1.md#final-user-responses-and-remaining-implementation-risks)
- [Editing session](../../../../crates/scriptcraft-core/src/editing.rs)

## Implementation
Apply the evidence-based WO-001 edit matrix to known runs and attachments. Stop or escalate unsupported unrelated attachment intersections, not ordinary script edits merely because of script/board links. Investigate destructive scene edits with authored nested data; if no preservation solution satisfies both requirements, surface the specific blocker. Ensure failed operations preserve selection, undo, source and dirty state.

## Acceptance criteria
- UTF-8-safe edits preserve or correctly move understood anchors and IDs; no hidden payload is silently deleted or duplicated.
- Undo/redo restores semantic and source state together; a safe-stop changes nothing. Script edits never rewrite board objects and cross-domain orphaning has no warning or block.

## Useful checks
- Test boundaries, Unicode expansion, unknown inline intersections, split/join and role changes, nested beats, board independence, undo/redo and failed transaction rollback.

## Dependency and handoff
Depends on WO-002 and WO-003. One dedicated commit and fresh validation; hand off transaction APIs to codec, cast and desktop orders.

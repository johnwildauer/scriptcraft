# Work Order: WO-012 — Native FDX Save and Save As cutover

## Outcome
Make FDX the active desktop native format with transactional live/saved cast agreement.

## Scope
Wire new documents, Open, Save and Save As to the FDX model, codec and atomic storage; use FDX extension/filter and privacy-safe diagnostics. Enable WO-011 open in the same commit. Export behavior remains noncanonical.

## Relevant references
- [Save and cast decisions](../document-model-refactor-v1.md#final-user-responses-and-remaining-implementation-risks)
- [Native acceptance](../../../requirements/features/screenplay-writing-loop.md#fdx-document-content-and-preservation)

## Implementation
For either save command, stage the live document with pending edits, silently correct candidate cast lists, serialize as-is, validate and atomically replace; only then publish the same candidate, preservation baseline, path and clean state. On failure retain all prior live state and destination. Permit explicit in-place older conversion; no open-time overwrite. Update diagnostics without dumping XML, paths or image data. Do not route Fountain here.

## Acceptance criteria
- New/open/edit/save/reopen and Save As use Version 6 FDX; source is untouched by Save As and non-6 source changes only on explicit successful Save.
- Successful save makes live and serialized cast values agree; failures and conflicts publish neither correction nor path/clean-state changes. Pending timer and undo behavior remain coherent.

## Useful checks
- Desktop/headless integration for Save and Save As, periodic race, direct cast edits, failed serialization/write/conflict, output version and reopen; applicable project checks.

## Dependency and handoff
Depends on WO-005, WO-007–WO-011. One dedicated commit and fresh validation; native path is now FDX and subsequent cleanup may remove transitional JSON support.

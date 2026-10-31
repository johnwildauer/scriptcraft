# Work Order: WO-012 — Native FDX Save and Save As cutover

- **Status:** `blocked` — the user directed deprecation of non-FDX legacy document state. WO-012a–g must first retire that state safely, preserve actual FDX data, prove structural editing and prepare transactional publication. No private extension or sidecar is required; do not activate an unsafe cutover or silently discard live transitional data.

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
- Successful save makes live and serialized cast values agree; failures and conflicts publish neither correction nor path/clean-baseline changes. Pending timer and undo behavior remain coherent.
- For Save and Save As, failure is reported without a success signal: an initially modified or unsaved session remains unsaved, and an initially clean session remains clean only relative to its existing baseline, not the attempted destination. Preserve pending edits and the prior destination, session and path; no failed candidate correction is published.

## Useful checks
- Desktop/headless integration for Save and Save As, initially clean and modified sessions, pending edits and periodic race, direct cast edits, failed preparation/serialization/write/conflict/replacement, failure reporting, destination/path/baseline preservation, output version and reopen; applicable project checks.

## Dependency and handoff
Depends on WO-005, WO-007–WO-011 and all inserted WO-012a–g, in their stated order. One dedicated commit and recorded implementer checks; the initiative-specific user direction prohibits dispatching further independent validation agents. Publish only a fully prepared, re-readable candidate and baseline after successful storage; retain selection/history and failure invariants specified by WO-012g. Native path becomes FDX only on completion; subsequent cleanup may remove transitional JSON support.

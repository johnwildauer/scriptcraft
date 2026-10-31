# Work Order: WO-012 — Native FDX Save and Save As cutover

- **Status:** `implementation recorded; user-owned review and validation not performed`.

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

## User-owned verification handoff
The user owns all review and validation. No tests, builds, lint, formatting checks, manual verification, implementation checks, or agent reviews were run for this work order. Suggested user verification remains: desktop/headless Save and Save As for clean and modified sessions, pending edits and cast correction, failure/conflict and post-publication cleanup outcomes, destination/path/baseline preservation, output version/reopen, JSON-origin refusal, and applicable project checks.

## Dependency and handoff
Depends on WO-005, WO-007–WO-011 and all inserted WO-012a–g, in their stated order. One dedicated commit when Git permits; user owns review and validation. Publish only a fully prepared, re-readable candidate and baseline after successful storage; retain selection/history and failure invariants specified by WO-012g. Native path becomes FDX for new and FDX-origin sessions; transitional JSON-origin state is not silently migrated or discarded. WO-013 remains responsible for subsequent JSON sunset policy.

## Implementation record — 2026-10-31

- Desktop Open now activates the staged content-detected FDX/legacy-JSON flow. New documents and Save As use `.fdx`; FDX Save and Save As call the prepared transaction, which publishes cast correction, rebased preservation baseline, path, and clean session only after successful storage. Save As retains no-overwrite behavior; post-install temporary cleanup failure is surfaced as a warning while retaining successful publication.
- Existing JSON-origin sessions retain their original JSON Save path where the legacy adapter can still serialize them. FDX Save As from a JSON-origin session is explicitly refused because retired app-owned fields have no FDX mapping; the live session and existing source file remain unchanged. This is a bounded no-migration limitation, not silent conversion. FDX data is refused under a `.json` name, and native Save As requires `.fdx`.
- Added desktop test cases for native Version 6 save, Save As/source isolation and session publication, Save As destination refusal, external modification conflict, new-document FDX save, and JSON-origin state refusal. These are authored tests only and were not executed.
- The storage layer's documented same-directory replacement/Save As behavior, final external-writer race, platform-specific atomicity and durability limits, and metadata/ACL limitations remain. No stronger atomicity or cross-editor compatibility is claimed. Older/newer version warning/import behavior remains as implemented; title, board, image, unsupported FDX source, script/board independence, and cast correction continue to rely on their prerequisite implementations and limitations, not newly verified claims.
- **Unperformed verification:** no tests (including the authored tests), build, lint, format check, `git diff --check`, manual desktop/Final Draft verification, independent review, validator dispatch, or other validation was performed for WO-012. The user must perform all review and validation. No Fountain, WO-013, or WO-014 work was performed.

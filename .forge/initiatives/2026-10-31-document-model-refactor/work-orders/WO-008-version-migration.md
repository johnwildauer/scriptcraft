# Work Order: WO-008 — Non-6 version warning and conversion

## Outcome
Provide bounded version detection and the directed non-6 importer path without mutating files.

## Scope
Separate preflight detection from import so the desktop can show a warning before attempting conversion. For older documents attempt the Version 6 importer and retain only title and content, with all convertible elements and General fallback for unrecognized older types. For newer declarations attempt the same importer best effort without claiming a supported migration.

## Relevant references
- [Final version decisions](../document-model-refactor-v1.md#final-user-responses-and-remaining-implementation-risks)
- [Version acceptance](../../../requirements/features/screenplay-writing-loop.md#fdx-document-content-and-preservation)

## Implementation
Use secure bounded version preflight; distinguish explicit numeric non-6 from absent, duplicate and malformed declarations. Return a staged candidate or a typed incompatible failure; no partial salvage. Do not silently equate changing the root attribute with migration. Identify unproven newer-section handling and safe-stop if it cannot be represented safely.

## Acceptance criteria
- Every explicit non-6 value requests warning/consent before the importer attempt; cancel performs no conversion. Older missing content or unconvertible element fails the whole candidate, not a partial import.
- No path writes or session replacement occur during import. Version 6 unknown types follow preservation rules, not the older General fallback.

## Useful checks
- Older/newer/6, absent/invalid/duplicate version, malformed XML, unknown convertible/unconvertible elements, missing content, cancellation and source immutability tests.

## Dependency and handoff
Depends on WO-006 and WO-007. One dedicated commit and fresh validation; expose typed warning and candidate outcomes to WO-011.

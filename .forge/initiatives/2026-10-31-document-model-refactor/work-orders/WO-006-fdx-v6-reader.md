# Work Order: WO-006 — Bounded FDX Version 6 reader

## Outcome
Decode Version 6 FDX into the semantic document and preservation layer in a dedicated headless codec file.

## Scope
Implement secure bounded XML parsing and Version 6 deserialization, including 17 roles, runs, title page, unknown content, source ordering, IDs and board/image relationships. This is an isolated codec API, not desktop native Open.

## Relevant references
- [Version and safety requirements](../../../requirements/features/screenplay-writing-loop.md#fdx-document-content-and-preservation)
- [FDX evidence](../document-model-refactor-v1.md#fdx-compatibility-and-preservation-investigation--2026-10-31)

## Implementation
Use the WO-001 selected parser and budgets; disable DTD/external entities, file/network resolution and image-path access. Preserve mixed children, namespaces, attributes and unknown types without forcing them to General. Import cast-list text as-is. Preserve title-page structure, never infer metadata from position; detect malformed or ambiguous identity/reference cases with safe-stop. Cross-domain orphaning is silent and does not alter board internals.

## Acceptance criteria
- Version 6 safe fixtures deserialize with ordered known and opaque data and source text intact; unsupported unrepresentable constructs fail explicitly rather than truncate.
- Invalid encodings, malformed XML, unsafe entities and resource-limit excess produce bounded privacy-safe errors without session or filesystem mutation.

## Useful checks
- Synthetic multi-run, Unicode, mixed-content, namespace, title, board/image, unknown type and ID fixtures; adversarial XML and limit tests. Do not expose original image bytes or paths.

## Dependency and handoff
Depends on WO-001–WO-003. One dedicated commit and fresh validation; reader output is consumed by WO-007 and WO-008.

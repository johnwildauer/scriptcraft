# Work Order: WO-007 — FDX Version 6 writer

## Outcome
Serialize new and imported documents to validated Version 6 FDX through the dedicated codec.

## Scope
Reconcile managed semantic fields with preserved ordered source, runs, title-page structure, scene attachments, unknown XML and board/item/image data. Support a privacy-safe verified-by-tests new-document template and explicit ScriptCraft field mapping.

## Relevant references
- [FDX architecture](../../../blueprints/technical-context.md#fdx-model-and-ownership-contracts)
- [Preservation requirements](../../../requirements/features/screenplay-writing-loop.md#fdx-document-content-and-preservation)

## Implementation
Use WO-001 evidence and WO-004 transformations. Emit only Version 6; validate XML characters, escaping, bounds, IDs, supported references and ownership before returning complete bytes. Document semantic versus lexical guarantees. Do not claim regenerated foreign data or universal Final Draft validity. If app-specific fields lack a proven durable home or source reconciliation destroys authored content, stop and escalate rather than emit lossy success.

## Acceptance criteria
- New, no-op and safely edited Version 6 documents round-trip all supported semantic and preservation data; title page is preserved, not reconstructed from metadata.
- Source order, unknown attributes/children and board internal relationships survive to the documented granularity; allowed script/board orphaning does not warn or block.
- Failure returns no partial output and mutates no session or source.

## Useful checks
- Semantic round-trip tests with synthetic image digest, 17 roles, styles, namespaces, unknown nodes and escaped Unicode; malformed mappings and invalid characters. Distinguish these checks from actual Final Draft tests.

## Dependency and handoff
Depends on WO-004 and WO-006. One dedicated commit and fresh validation; deliver a pure codec API for save integration.

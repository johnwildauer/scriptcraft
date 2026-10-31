# Work Order: WO-003 — Source preservation and identity

## Outcome
Introduce ordered, bounded source-preservation state with explicit ownership and stable anchors.

## Scope
Model opaque sections, attributes, unknown types and inline mixed content, title structure, nested scene attachments and separate board/item/image graphs. Distinguish foreign and internal IDs, definitions and references, and source order. Preserve source independently of filesystem path; no XML codec or UI cutover.

## Relevant references
- [Preservation decisions](../document-model-refactor-v1.md#final-user-responses-and-remaining-implementation-risks)
- [Architecture](../../../blueprints/technical-context.md#fdx-model-and-ownership-contracts)

## Implementation
Apply WO-001's representation and budgets; record what is semantically versus lexically retained and explicitly model unknown dependency confidence. Board and script are independent; allow their links to be orphaned silently without destroying board entities or internal board/image links. Reject ambiguous internal IDs or unsafe representation with typed diagnostics. Never resolve local paths.

## Acceptance criteria
- Ordered unknown content and title/board payloads survive cloning and equality without requiring source path; internal identity collision and dangling-reference cases have deterministic bounded outcomes.
- Cross-domain links do not enforce script/board coupling; unrelated authored payload is not silently deleted.
- Existing native behavior remains buildable behind transitional adapters.

## Useful checks
- Synthetic tests for anchors, unknown interleaving, duplicate IDs, board/item/image links, title order, resource limits and privacy-safe diagnostics.

## Dependency and handoff
Depends on WO-001 and WO-002. One dedicated commit and fresh validation; expose ownership and reference APIs to editing and codec orders.

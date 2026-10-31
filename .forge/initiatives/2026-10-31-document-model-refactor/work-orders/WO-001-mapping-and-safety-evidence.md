# Work Order: WO-001 — FDX mapping and safety evidence

## Outcome
Produce an implementable, evidence-backed mapping and preservation contract without changing native behavior.

## Scope
Inventory every current project, profile, identity and editing-policy field, all 17 directed roles, title, unknown content, nested scenes and board/item/image data. Determine candidate FDX homes and source ownership, identity domains, resource limits and a minimal new-document template. Compare the two references without printing embedded payloads or private paths. Evaluate a maintained namespace-aware XML implementation, license, security, ordering and lexical limitations under the core's unsafe-code policy.

## Relevant references
- [Initiative decisions and risks](../document-model-refactor-v1.md#work-order-readiness-review--2026-10-31)
- [FDX architecture](../../../blueprints/technical-context.md#fdx-model-and-ownership-contracts)

## Implementation
Record mapping evidence, confidence, semantic versus lexical preservation, ID policy, supported encodings, configurable tested bounds, role layout/transition defaults and unsafe-edit matrix in a concise initiative artifact. Compare interoperable mappings, private extensions and sidecars without assuming any survive another editor. Produce small synthetic privacy-safe fixtures; never copy embedded source image data or paths. If a required field has no safe durable home, unknown XML cannot be reconciled, or authored nested scene data cannot survive a directed edit without violating board independence, stop and escalate the concrete case rather than weakening criteria.

## Acceptance criteria
- Every field and relevant source section has an owner, proposed mapping and evidence or explicit blocker; unsupported newer-version behavior and unknown dependencies are not presented as solved.
- Fixtures cover observed structures without private source content; no Final Draft compatibility claim is made from them alone.
- Native JSON behavior and build remain unchanged.

## Useful checks
- Inspect sanitized fixtures and mapping for completeness; run applicable project checks. Record any available real-app evidence separately with version and platform.

## Dependency and handoff
First implementation order after document review and design approval. One dedicated commit and fresh independent validation. Hand the mapping, fixtures and explicit blockers to WO-002 through WO-014.

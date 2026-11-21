# Feature-Area Blueprint: Area Name

> This is the technical counterpart to the linked requirements area. Group related capabilities when they share technical composition; this need not be one file per small feature. Describe the target as established structure, not as a report of what implementation currently exists.

## Scope and requirements

- **Requirements:** [`../../requirements/features/feature-slug.md`](../../requirements/features/feature-slug.md)
- **Technical context:** [`../technical-context.md`](../technical-context.md)
- **Area summary:** `TODO: Explain the target technical path from user intent to system behavior.`

## Shared composition

`TODO: Link the relevant component and container blueprints, and explain how this area uses them.`

```component
name: AreaCapability
container: Container Name
responsibilities:
  - TODO: required responsibility
```

## Capability realization

### Capability name

- **Requirements:** `REQ-AREA-001`, `AC-AREA-001`
- **Composition and flow:** `TODO: Describe the components, data flow, and boundaries that satisfy these requirements.`
- **Contracts:** `TODO: State capability-specific invariants, interfaces, and failure behavior as needed.`

`TODO: Add a section for another capability only when it adds useful technical detail; use stable requirement IDs for traceability.`

## Decisions

`TODO: Record only decisions that materially clarify the area architecture, with rationale and consequences.`

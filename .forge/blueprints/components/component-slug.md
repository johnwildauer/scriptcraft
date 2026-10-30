# Component Blueprint: Capability Name

> A component is a reusable capability composed of runtime components. It may span containers and remains focused on a coherent technical responsibility. This document describes desired structure and contracts in established-state voice, whether or not implementation exists yet.

## Capability

`TODO: Describe what this capability provides, its consumers, and the key data or contracts it owns.`

## Composition

```component
name: ComponentName
container: Container Name
responsibilities:
  - TODO: required responsibility
```

`TODO: Add another block or a short relationship description only when needed to explain ownership, direction, or data crossing a boundary.`

## Contracts

- **Invariants and failure behavior:** `TODO: state required ordering, consistency, retry, security, or reliability rules as applicable.`
- **Interfaces:** `TODO: describe required APIs, events, file formats, or composition contracts as applicable.`

## Decisions

`TODO: Record only decisions that materially clarify this capability's target structure, with rationale and consequences.`

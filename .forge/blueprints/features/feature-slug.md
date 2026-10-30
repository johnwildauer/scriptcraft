# Feature Blueprint: Feature Name

> This is the technical counterpart to [`feature-slug.md`](../../requirements/features/feature-slug.md). It explains how shared capabilities and feature-specific components satisfy the feature requirements.
>
> This blueprint describes the desired technical realization. Evidence about current code is context, not initiative progress; approval does not claim that the code already matches the target.

## Feature Summary

`TODO: Summarize the technical path from user intent to system behavior.`

## Document status

- **Status:** `draft`
- **Owner:** `TODO`
- **Last updated:** `TODO`
- **Requirements:** [`../../requirements/features/feature-slug.md`](../../requirements/features/feature-slug.md)

## Component Blueprint Composition

`TODO: Name the shared component blueprints and explain how this feature configures and wires them. Use #ComponentName for runtime components and @Document Name for system documents.`

## Feature-Specific Components

```component
name: FeatureSpecificComponent
container: Container Name
responsibilities:
	- TODO
```

`TODO: Add relationship paragraphs where direction or data flow is not obvious.`

## Models and contracts

```model
name: FeatureModel
store: TODO
description: TODO
fields:
	- id: TODO
constraints:
	- TODO
```

### Key Contracts

- `TODO: feature-specific invariants and reliability behavior.`

### Integration Contracts

- `TODO: feature-specific APIs, events, and composition expectations.`

## Requirement alignment

- `REQ-FEATURE-001` → `TODO: components and flow satisfying this requirement`

## Architecture Decision Records

### ADR-001: Decision title

**Context:** `TODO`

**Decision:** `TODO`

**Consequences:** `TODO`

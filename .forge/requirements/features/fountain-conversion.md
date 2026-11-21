# Requirements Area: Fountain Conversion

> This document specifies desired user-facing behavior for a cohesive area. Describe requirements in established-state voice, even when implementation is future work. Group related capabilities into sections and keep stable IDs so each requirement and acceptance criterion remains independently traceable. Do not add lifecycle status, implementation evidence, progress, or a backlog here.

## Area purpose

Fountain is a secondary interchange format for creating a new native screenplay or producing a separate text-based copy. It does not replace native FDX as the saved source of truth.

## Capability groups

### Import and export

- **REQ-WRITING-LOOP-010: Fountain conversion.** Fountain import shall use a fixed documented conversion protocol to create a new unsaved native document; export shall produce a separate lossy copy without changing the native path or clean state. Import retains as much parseable writer data as the supported protocol permits. Potential conversion loss does not require a warning; fatal failures are reported. Fountain is not the native format.

## Related documents

- **Product overview:** [`../product-overview.md`](../product-overview.md)
- **Technical blueprint:** [`../../blueprints/technical-context.md`](../../blueprints/technical-context.md)

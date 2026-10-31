# Work Order: WO-012a — Reconcile the FDX-native durable contract

- **Status:** `draft / not started`; insertion before WO-012.

## Outcome and scope
Update only the durable requirements and architecture affected by retiring ScriptCraft-only persistent document state. Follow the [assessment and field dispositions](../document-model-refactor-v1.md#fdx-native-legacy-state-deprecation-assessment-and-handoff--2026-10-31); preserve historical decisions and review artifacts.

## Components and implementation
- Revise [product overview](../../../requirements/product-overview.md), [writing loop](../../../requirements/features/screenplay-writing-loop.md), and [technical context](../../../blueprints/technical-context.md). Keep stable requirement IDs and specify FDX-owned source metadata, paragraph identities, title structure and document settings versus transient internal IDs, schema, layout inputs and editing policies.
- Replace requirements for persisted app-specific metadata, profile, policy and IDs with explicit source-preservation, runtime deterministic layout and safe-save criteria. Distinguish repeatable layout for the same effective inputs from exact fidelity to every FDX file. State nondefault legacy config/IDs can reset; no legacy JSON migration, extension or sidecar is required.
- Keep all 17 roles, supported/source-owned FDX fields, unknown content, title, board independence, version, cast, atomic failure, export and recovery boundaries. Do not claim unsupported real-app mappings or remove historical review warnings. Note the intentional target/code lag once in the initiative handoff.

## Acceptance and checks
- Every conflict called out in the field matrix and AC-WRITING-001.1/001.2/001.4a, 003.1/003.3 and blueprint ownership is resolved without weakening source-data safety. No new product feature or invented FDX mapping is introduced.
- Run `git diff --check`; search the three documents for stale requirements that app-specific profile/metadata/policies must persist. Record exact results as implementer checks, not review or independent validation.

## Risks and dependency
Depends on WO-001 evidence and WO-002–011 implementation history; first of the inserted sequence. Owner checks applicable approval/external gates before execution; no reviewer or independent validator is dispatched under this task. One attributable documentation commit. Risk: accidentally deleting genuine FDX or user-facing pagination guarantees; retain those explicitly. Hands contract to WO-012b.

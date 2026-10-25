# Initiative: Short Slug

> Copy this directory to `.forge/initiatives/YYYY-MM-DD-short-slug/`, rename this file to `short-slug-v1.md`, and replace `initiative-slug`/`short-slug` placeholders in copied links. Keep superseded plan versions for history, but identify only one version as authoritative. The source template directory and its descriptive filename are not renamed.

## Initiative metadata

- **Phase:** `draft` | `plan` | `review` | `execute` | `validate` | `closeout` — selected: `draft`
- **Status:** `active` | `blocked` | `closed` — selected: `active` <!-- when blocked, record the reason, decision needed, owner, and next action in Risks and decisions; `closed` is the terminal status, set when closeout completes -->
- **Owner/orchestrator:** `TODO`
- **Created:** `TODO`
- **Updated:** `TODO`
- **Authoritative version:** `v1`
- **Initiative mode:** `standard` | `rapid` — selected: `standard`
- **Mode requested by:** `user` | `orchestrator recommendation` | `project policy` — selected: `TODO`
- **Mode confirmation:** `TODO` <!-- explicit user confirmation or approval evidence; `rapid` is invalid without it -->
- **Rapid-mode eligibility:** `confirmed` | `not eligible` | `overridden by user` — selected: `TODO`
- **Base/default branch:** `TODO`
- **Initiative branch:** `TODO` <!-- branch name, or `none` in reduced-capability/opt-out mode -->
- **Branch policy:** `required` | `optional` | `user opt-out` — selected: `TODO`
- **Branch naming convention:** `dev/YYYY-MM-DD-short-slug` or recorded override: `TODO`
- **Branch capability/evidence:** `TODO`
- **Reduced-capability fallback or opt-out evidence:** `TODO` <!-- include explicit no-branch review notice when applicable -->
- **Branch creation failure:** `none` | `recorded below` — `TODO`
- **Validation cadence:** `per work order` | `consolidated` | `custom` — selected: `per work order` <!-- `consolidated` requires explicit user request and a cascade-failure warning when applicable -->
- **Phase transition evidence:** `TODO` <!-- for each transition (draft → plan, plan → review, review → execute, each execute → validate and validate → execute in the per-work-order loop, validate → closeout) and the terminal status flip (closeout → `closed`), record the trigger, date, and evidence -->
- **Direct-translation confirmation:** `not needed` | `requested and confirmed` — evidence: `TODO` <!-- required when the initial prompt was broad but the work orders are a direct translation of the user's stated authorization -->

## Standard artifact locations

- **Authoritative plan:** `[initiative-slug]-v1.md`
- **Design review:** [`reviews/design-review.md`](reviews/design-review.md)
- **Validations:** `validations/validation-[work-order-id].md`
- **Execution log:** [`execution-log.md`](execution-log.md)
- **Work orders:** [`work-orders/`](work-orders/)

## Outcome

`TODO: What valuable, observable outcome will this initiative deliver?`

## Initiative boundary and sizing

- **Parent roadmap or product goal:** `TODO: link the roadmap or broader goal; identify the staged MVP step if applicable`
- **Initiative type:** `workspace foundation` | `product MVP stage` | `A/B experiment` | `plausibility branch` | `other` — selected: `TODO`
- **Single observable outcome:** `TODO: state the one outcome that makes this initiative complete`
- **Validation boundary:** `TODO: name the independent review, test, demonstration, or manual check that determines whether the outcome is met`
- **Split assessment:** `keep as one initiative` | `split into linked initiatives` — decision: `TODO`
- **If split, traceability and dependencies:** `TODO: record resulting initiative slugs, dependency order, and how each remains independently reviewable`

Use the sizing guidance in [`02-initiative.md`](../../process/02-initiative.md). This is advisory: a coherent initiative may remain one initiative regardless of duration or work-order count. Consider splitting when outcomes are unrelated, there is no single validation boundary, several independent product decisions are required, or future work is being treated as present.

## Scope

### In scope

- `TODO`

### Out of scope

- `TODO`

## Context and motivation

`TODO: Explain the problem and link the relevant durable documents.`

## Exploration scratchpad

Use this section during the open-ended planning loop. Keep observations,
research, alternatives, and user decisions distinct from settled scope. This
section may be incomplete while the initiative is being explored.

### Initial findings

- `TODO: repository observations, evidence, and relevant existing behavior`

### Questions and hypotheses

- `TODO: unresolved question or hypothesis to investigate`

### Options considered

| Option | Benefits | Costs/risks | Current direction |
|---|---|---|---|
| `TODO` | `TODO` | `TODO` | `TODO` |

### Decisions and discussion notes

- `TODO: record material decisions, rationale, and user guidance`

### Next investigation or conversation focus

- `TODO: leave open-ended during exploration; replace with a convergence action when ready`

## Requirements and blueprint references

- **Requirements:** `TODO: relative links and IDs`
- **Blueprints:** `TODO: relative links`

## Proposed design

`TODO: Describe the intended approach, key flows, and architectural decisions without turning this into a line-by-line implementation plan.`

## Work-order plan

Work orders are not created during initial exploration. Once the user confirms
the initiative is ready for formal design preparation, create the generated
work-order files here and replace this note with the ordered plan.

_No work orders created yet._

## Risks and decisions

- **Risk:** `TODO` — **Mitigation:** `TODO`
- **Decision needed:** `TODO` — **Owner:** `TODO`

## Design readiness checklist

- [ ] Scope and exclusions are explicit.
- [ ] The initiative has one observable outcome and one named validation boundary.
- [ ] The initiative is traceable to its roadmap or parent product goal, including any staged MVP relationship.
- [ ] The advisory split assessment is recorded; any split initiatives have distinct outcomes, dependencies, and review/closeout boundaries.
- [ ] Requirements and blueprints are linked.
- [ ] Work orders are independently executable and ordered; additional detail that is plausibly subordinate to the agreed outcome is expected and is not drift by itself.
- [ ] Every work order carries a durable-document impact classification per [`00-overview.md`](../../process/00-overview.md).
- [ ] Initiative mode metadata (mode, requested by, confirmation, eligibility) is recorded.
- [ ] Phase transition evidence and the direct-translation confirmation (when applicable) are recorded.
- [ ] Validation cadence is recorded.
- [ ] The applicable validation evidence boundary and explicit project-policy/manual gates are identified; optional checks may remain recorded as notes or warnings.
- [ ] Required human approval has been obtained or is pending at the documented gate.

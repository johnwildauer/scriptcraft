# Initiative: Short Slug

> Copy this directory to `.forge/initiatives/YYYY-MM-DD-short-slug/`, rename this file to `short-slug-v1.md`, and replace `initiative-slug`/`short-slug` placeholders in copied links. Keep superseded plan versions for history, but identify only one version as authoritative. The source template directory and its descriptive filename are not renamed.

## Initiative metadata

- **Current activity (optional):** `explore` | `plan` | `review` | `implement` | `validate` | `closeout` — `TODO` <!-- descriptive convention; follow the user's requested path without a redundant phase ledger -->
- **Status (optional):** `active` | `blocked` | `closed` — `TODO` <!-- descriptive convention; if blocked, record the actual blocker, owner, and next action -->
- **Owner/orchestrator:** `TODO`
- **Created:** `TODO`
- **Updated:** `TODO`
- **Authoritative version:** `v1`
- **Initiative mode:** `standard` | `rapid` — selected: `standard`
- **Mode requested by:** `user` | `orchestrator recommendation` | `project policy` — selected: `TODO`
- **Mode confirmation:** `TODO` <!-- the user's explicit rapid-mode request itself is sufficient evidence; do not require redundant confirmation -->
- **Rapid-mode eligibility:** `confirmed` | `not eligible` | `overridden by user` — selected: `TODO`
- **Base/default branch:** `TODO`
- **Initiative branch:** `TODO` <!-- branch name, or `none` in reduced-capability/opt-out mode -->
- **Branch policy:** `required` | `optional` | `user opt-out` — selected: `TODO`
- **Branch naming convention:** `dev/YYYY-MM-DD-short-slug` or recorded override: `TODO`
- **Branch capability/evidence:** `TODO`
- **Git usable for per-WO commit and hash review at initiative start:** `yes` | `no` — evidence: `TODO` <!-- `no` permits only the documented skipped design-review/validation outcomes; a later failure when `yes` is blocked -->
- **Reduced-capability fallback or opt-out evidence:** `TODO` <!-- include explicit no-branch review notice when applicable -->
- **Branch creation failure:** `none` | `recorded below` — `TODO`
- **Validation cadence:** `per work order` | `consolidated` | `custom` — selected: `per work order` <!-- `consolidated` requires explicit user request and a cascade-failure warning when applicable -->
- **Material approvals/gates:** `TODO` <!-- record applicable user-approved gates and evidence; drafting work orders is not approval to execute -->

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

Work orders may be drafted directly from plans or feature requests when the user
asks. Drafting does not itself approve execution. List distinct work-order IDs
and outcomes, with concise dependencies and handoffs. Retain material decisions,
blockers, and evidence rather than a redundant phase ledger.

When a meaningful durable-document update should precede implementation, list a
narrow document-only work order before its dependent implementation work order(s)
and identify the dependency IDs. This is advisory, not mandatory; omit it when
there is no durable impact or the change is below the relevant abstraction
level. Record any declared document/code lag once in the execution handoff, not
in requirements or blueprints. See [`02-initiative.md`](../../process/02-initiative.md).

_No work orders created yet._

## Risks and decisions

- **Risk:** `TODO` — **Mitigation:** `TODO`
- **Decision needed:** `TODO` — **Owner:** `TODO`

## Design readiness prompts

Use these as proportionate drafting/review prompts, not as mandatory fields or
phase-transition gates. Follow the user's actual request and retain only
applicable project-policy and external requirements.

- [ ] Scope and exclusions are explicit.
- [ ] The initiative has one observable outcome and one named validation boundary.
- [ ] The initiative is traceable to its roadmap or parent product goal, including any staged MVP relationship.
- [ ] The advisory split assessment is recorded; any split initiatives have distinct outcomes, dependencies, and review/closeout boundaries.
- [ ] Requirements and blueprints are linked.
- [ ] Work orders are independently executable and ordered; additional detail that is plausibly subordinate to the agreed outcome is expected and is not drift by itself.
- [ ] Relevant durable-document impacts are recorded proportionately per [`00-overview.md`](../../process/00-overview.md).
- [ ] Initiative mode metadata (mode, requested by, confirmation, eligibility) is recorded.
- [ ] Applicable material approvals/gates and evidence are recorded; no redundant phase or direct-translation confirmation is required.
- [ ] Validation cadence is recorded.
- [ ] The applicable validation evidence boundary and explicit project-policy/manual gates are identified; optional checks may remain recorded as notes or warnings.
- [ ] Required human approval has been obtained or is pending at the documented gate.

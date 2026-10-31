# Software Forge

`.forge/` is the portable, Markdown-native workflow for agent-assisted software development. It is the source of truth for the workflow itself, the durable understanding of a host repository, and the record of bounded implementation initiatives.

## Start here

1. Use [`process/00-overview.md`](process/00-overview.md) as default routing guidance when relevant to the user's task.
2. Check for [`project-manifest.md`](project-manifest.md) at this directory's root:
   - **Missing, or not `complete`:** by default, initialize the repository using [`process/01-init-new.md`](process/01-init-new.md) for a new or empty repository or [`process/01-init-existing.md`](process/01-init-existing.md) for an existing repository. Follow the user's actual task instead when initialization is not needed or the user directs another appropriate path; pre-filled intake answers are optional context, not required homework.
   - **`complete`:** initialization is done. The normal starting point for a new initiative is [`process/02-initiative.md`](process/02-initiative.md), subject to the user's actual request.
3. On resume, use [`project-manifest.md`](project-manifest.md) status and revision history when relevant; do not let a routine read or status label block a direct user task.

Use process documents as default workflow guidance, and read the relevant document when it helps with the current task. The user's actual instructions take precedence over advisory lifecycle labels and defaults. Do not require exact phase wording or redundant confirmation when asked to proceed directly from plans or feature requests to drafting work orders. Drafting work orders is distinct from approving or executing them.

At the user's explicit direction, adapt and record project-policy changes in the appropriate durable project document; an orchestrator may dispatch an architect or coding agent to adapt host-project process documents. Respect applicable external rules and checks: do not silently discard or weaken them, and communicate genuine uncertainty or blockers. Process documents are swappable and may be updated or replaced without re-initializing the repository. Preserve existing project-specific policy unless the user directs a change, record that change, and keep material decisions traceable without rote synchronization.

### Authoritative host-project policy

Generic Forge process steps are defaults. An explicit, current repository policy in [`project-manifest.md`](project-manifest.md#authoritative-internal-review-and-validation-policy) overrides conflicting agent workflow requirements across initiatives and process documents. ScriptCraft's current policy makes all review and validation user-owned and disables agent-run internal reviews, validators, checks, and other validation. This changes workflow ownership only: in-product safety behavior and applicable external rules remain in force. Surface genuine conflicts; do not imply an external obligation was waived.

Initiatives normally follow exploration, planning, design review and approval, implementation, per-work-order validation, and closeout. This is guidance, not a rigid transition machine: phases and statuses describe progress, and the user may direct an appropriate path. **Standard** is the default; **Rapid** is an explicitly user-requested compressed path for small, well-understood changes that takes the initial prompt as a complete draft. See [`process/00-overview.md`](process/00-overview.md) and [`process/02-initiative.md`](process/02-initiative.md). Rapid mode and other user-directed paths retain applicable external rules; the user may explicitly adapt and record project-policy defaults.

## Directory map

| Directory | Purpose |
|---|---|
| [`process/`](process/) | Agent instructions for initialization, planning, design review, implementation, validation, and closeout. Swappable; contains no project-specific state. |
| [`project-manifest.md`](project-manifest.md) | Durable project state: intake answers, involvement level, repository baseline, working agreement, project policy, roadmap, and revision history. |
| [`requirements/`](requirements/) | Durable product intent, user-facing capabilities, and testable acceptance criteria. |
| [`blueprints/`](blueprints/) | Durable technical context, architecture, component behavior, contracts, and feature composition. |
| [`initiatives/`](initiatives/) | Bounded change sets containing plans, work orders, reviews, validations, and execution history. |

## Core boundaries

- **Requirements** describe durable desired product behavior: what users and stakeholders need and why. Cohesive area files may group related capabilities and retain stable requirement/acceptance IDs; they do not prescribe internal implementation or carry lifecycle status, implementation evidence, progress, or history.
- **Blueprints** describe the durable desired technical architecture and contracts in established-state voice. The technical context is a navigable target map, not a current-state inventory; useful intended paths may precede code. Approval describes the target, not proof that current code implements it.
- **Initiatives** coordinate a specific change and record its plan, execution, status, history, and any temporary gap between desired documents and code. They do not replace or carry initiative progress in requirements or blueprints.

When a planned change needs a durable requirements or blueprint update, a narrow documentation-first work order before related implementation is a useful default, not a requirement. Link dependent work orders and record expected document/code lag once in the initiative execution handoff. Use this only where there is meaningful durable impact; if no relevant durable change is needed, do not create a documentation work order. Existing code may lag target documents until dependent implementation is complete. Initialization reviews remain required by default and their findings belong in separate concise review artifacts; review outcomes do not require status fields in requirements or blueprints.

## Process order

Initialization establishes the repository baseline in [`project-manifest.md`](project-manifest.md). Each later initiative normally follows this order, adapted to the user's actual instructions and applicable project or external requirements:

1. Explore and plan the initiative, then draft the requested work orders in [`process/02-initiative.md`](process/02-initiative.md).
2. Review the proposed design and obtain the default human approval using [`process/03-design-review.md`](process/03-design-review.md).
3. Implement approved work orders using [`process/04-work-order.md`](process/04-work-order.md).
4. Validate each completed work order using [`process/05-validation.md`](process/05-validation.md) only as permitted by the authoritative project policy; generic agent validation is not mandatory where that policy overrides it.
5. Close out using [`process/06-closeout.md`](process/06-closeout.md), recording material outcomes and follow-ups.

When Git is usable for commit and hash review, each work order closes with one
dedicated single-parent commit; its exact hash is passed to validation. Only
Git-unavailable-at-initiative-start permits design review and work-order
validation to be recorded as `skipped: Git unavailable`; requirements reviews
remain separate. See [`process/00-overview.md`](process/00-overview.md).

Labels such as `draft`, `plan`, `review`, `execute`, and `validate` are descriptive conventions, not prerequisites for responding to a direct user request. Retain material decisions, blockers, evidence, and user-approved gates; avoid redundant phase-ledger updates. The project-specific policy defaults are maintained in [`project-manifest.md`](project-manifest.md); a user-directed adaptation may be recorded there or in another appropriate durable project document, with a clear reference where needed. Applicable external rules remain in force.

# Software Forge

`.forge/` is the portable, Markdown-native workflow for agent-assisted software development. It is the source of truth for the workflow itself, the durable understanding of a host repository, and the record of bounded implementation initiatives.

## Start here

1. Read [`process/00-overview.md`](process/00-overview.md).
2. Check for [`project-manifest.md`](project-manifest.md) at this directory's root:
   - **Missing, or not `complete`:** this repository has not been initialized. Briefly introduce the workflow to the user in plain language — what will be set up and that independent checks run in the background — then follow [`process/01-init-new.md`](process/01-init-new.md) for a new or empty repository or [`process/01-init-existing.md`](process/01-init-existing.md) for an existing repository, completing the manifest together during guided intake. Pre-filled intake answers are optional context, not required homework.
   - **`complete`:** initialization is done. Use [`process/02-initiative.md`](process/02-initiative.md) to begin a new initiative.
3. On resume at any later time, read [`project-manifest.md`](project-manifest.md) first and continue from its status and revision history.

Do not edit workflow artifacts until you have read the process document governing the current step. Do not rely on temporary files outside `.forge/` when operating in an initialized host repository.

Process documents are swappable: they may be updated or replaced wholesale without re-initializing the repository. [`project-manifest.md`](project-manifest.md) is the durable contract — when a process document requires a field the manifest lacks, ask one targeted question and record the answer with a revision-history entry.

Initiatives run in one of two modes. **Standard** is the default and follows the full process order below. **Rapid** is an explicitly user-requested compressed path for small, well-understood changes that takes the initial prompt as a complete draft; see the initiative-mode policy in [`process/00-overview.md`](process/00-overview.md) and the rapid path in [`process/02-initiative.md`](process/02-initiative.md).

## Directory map

| Directory | Purpose |
|---|---|
| [`process/`](process/) | Agent instructions for initialization, planning, design review, implementation, validation, and closeout. Swappable; contains no project-specific state. |
| [`project-manifest.md`](project-manifest.md) | Durable project state: intake answers, involvement level, repository baseline, working agreement, project policy, roadmap, and revision history. |
| [`requirements/`](requirements/) | Durable product intent, user-facing capabilities, and testable acceptance criteria. |
| [`blueprints/`](blueprints/) | Durable technical context, architecture, component behavior, contracts, and feature composition. |
| [`initiatives/`](initiatives/) | Bounded change sets containing plans, work orders, reviews, validations, and execution history. |

## Core boundaries

- **Requirements** describe what users and stakeholders need and why. They should not prescribe internal implementation.
- **Blueprints** describe how the system is structured and how it satisfies requirements.
- **Initiatives** coordinate a specific change. They are not a replacement for the durable requirements or blueprints.

## Process order

Initialization establishes the repository baseline in [`project-manifest.md`](project-manifest.md). Each later initiative follows this order:

1. Draft and plan the initiative in [`process/02-initiative.md`](process/02-initiative.md) (`draft`, `plan`).
2. Review the proposed design with [`process/03-design-review.md`](process/03-design-review.md) (`review`).
3. Execute work orders using [`process/04-work-order.md`](process/04-work-order.md) (`execute`).
4. Validate each completed work order using [`process/05-validation.md`](process/05-validation.md) (`validate`).
5. Close the initiative using [`process/06-closeout.md`](process/06-closeout.md) (`closeout`).

The project-specific policy is maintained in the Project policy section of [`project-manifest.md`](project-manifest.md). It may add stricter gates or commands, but later process documents must not silently weaken it.

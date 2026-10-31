# Initialize an Existing Repository

## When to use

Use this when the user requests a baseline or repository inspection shows an established codebase without useful shared project context. Initialization is optional. Start with the user's request and actual repository; read [`00-overview.md`](00-overview.md) and [`../project-manifest.md`](../project-manifest.md).

The manifest may be an unfilled template. Placeholders and blank fields are not initialization status: use populated content, repository evidence, and explicit user direction to understand what is already established and whether baseline work is useful. Never route from a lifecycle field or require a replacement gate.

## Intake and repository inspection

Inspect relevant repository instructions, directory structure, languages, frameworks, entry points, package/build configuration, tests, documentation, and deployment or generated-file conventions. Inspect Git state and working changes where available so unrelated user work is preserved. Record only useful findings; do not produce an exhaustive file inventory.

Read existing project documentation first. Ask only questions needed to clarify project purpose, intended users and outcomes, important constraints/non-goals, relevant preferences, or policy not evident from the repository. Separate **observed** facts (repository or command evidence), **inferred** interpretations, and **desired** future behavior. Note unknowns rather than presenting assumptions as facts.

Respect user direction and applicable external or host-repository rules. Do not overwrite unrelated work, broaden scope, or run commands with side effects without authorization. Record material facts and decisions in [`../project-manifest.md`](../project-manifest.md) or the relevant durable document.

## Establish the baseline

1. Summarize the repository's purpose, structure, important technologies, and known setup/build/test/run commands in the manifest. Record uncertainty plainly; do not claim a command works unless that has been established.
2. Create or update [`../requirements/product-overview.md`](../requirements/product-overview.md) to describe desired product behavior, users, outcomes, and scope in established-state voice—not implementation status.
3. Group meaningful desired behavior under [`../requirements/features/`](../requirements/features/) with stable requirement and acceptance-criterion IDs. Keep requirements focused and testable; avoid duplicating a code inventory.
4. Describe the desired technical system in [`../blueprints/technical-context.md`](../blueprints/technical-context.md), using the repository to ground useful paths and boundaries. Add only the grouped container, component, and feature blueprints that clarify target structure or contracts. Blueprints are target state, not a current-state inventory or implementation progress record.
5. Record relevant project policy in the manifest, including known commands, constraints, applicable human/external requirements, and important unknowns. Note meaningful gaps between desired behavior and observed implementation for a future initiative when useful.
6. Summarize what was established and, if useful, recommend a bounded first initiative addressing a meaningful baseline gap. Do not require a roadmap or invent work to fill one.

Initialization establishes context, not validation or approval. Repository inspection may note existing checks and their documented purpose; it is not work-order validation. Validation occurs only after a discrete work order is complete and is performed by one coding validator following [`05-validation.md`](05-validation.md).

## Handoff

Summarize the observed baseline, durable documents updated, and remaining uncertainty. For planned work, use [`02-initiative.md`](02-initiative.md). Initialization does not authorize implementation unless requested.

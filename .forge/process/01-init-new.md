# Initialize a New Repository

## When to use

Use this when the user requests a baseline or repository inspection shows there is no reliable product/codebase baseline. Initialization is optional, not a mandatory first step. Start with the user's request and actual repository contents; inspect [`00-overview.md`](00-overview.md) and [`../project-manifest.md`](../project-manifest.md).

The manifest may still be the unfilled template. Treat placeholders and blank fields as a template, not evidence of completed initialization. Infer what is already established from meaningful populated content, the repository, and explicit user direction. Do not replace this distinction with a lifecycle flag or use a missing field as a routing gate.

## Intake and inspection

Inspect the repository root and relevant files, available tools, applicable repository instructions, and existing user changes. Ask only what is needed to understand the intended project: what it should do and for whom, the first useful outcome, constraints or non-goals, any known technology preferences, and any user-specific policy or external requirements. Reuse information already in the repository or manifest; record unknowns rather than guessing. Keep the conversation brief and in plain language.

Respect applicable external rules and preserve unrelated user work. Distinguish observed facts from inferred choices and desired behavior. Record material facts and decisions in the manifest or the relevant durable document; do not treat initialization as authorization to implement product work.

## Establish the baseline

1. Record the project purpose, intended users, outcomes, constraints, and material preferences in [`../project-manifest.md`](../project-manifest.md).
2. Create or update [`../requirements/product-overview.md`](../requirements/product-overview.md) with the desired product scope, outcomes, and explicit MVP boundary. Distinguish a runnable workspace foundation from the product MVP where useful.
3. Group desired behavior into cohesive documents under [`../requirements/features/`](../requirements/features/). Use stable requirement and acceptance-criterion IDs and testable outcomes; avoid a file per trivial feature.
4. Describe the intended technical structure and contracts in [`../blueprints/technical-context.md`](../blueprints/technical-context.md) and only the grouped container, component, and feature blueprints needed to make the next initiative understandable. Write blueprints as desired established state, not a current-state inventory or implementation log.
5. Record relevant project policy in the manifest: known commands and tools, constraints, applicable human or external requirements, and important unknowns. Do not impose a large questionnaire or require optional settings, a roadmap, or speculative architecture.
6. Summarize the baseline and, if useful, recommend a small first initiative with an observable outcome. Link the relevant requirements and blueprints; let the user request or defer further planning.

Initialization establishes context, not validation or approval. Repository inspection may record what is present and what commands are documented; it is not work-order validation. Validation occurs only after a discrete work order is complete and is performed by one coding validator following [`05-validation.md`](05-validation.md).

## Handoff

Summarize what was recorded, what remains unknown, and any suggested next step. For planned work, use [`02-initiative.md`](02-initiative.md). Initialization can stop when the user has the baseline they need; do not begin implementation unless requested.

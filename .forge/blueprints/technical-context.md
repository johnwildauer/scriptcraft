# Technical Context

> This is a high-level navigable map of the project's desired structure and runtime context. Describe the target as established structure, including useful paths that do not exist yet; a path in this map is not evidence that code exists. Record a discovered mismatch between this target and the repository in the relevant initiative record, not here.

## Project map

Choose the level of detail that helps someone navigate:

- For a project with roughly fewer than 20 meaningful files, list important files individually with their path and role.
- For a larger project, map major folders and important entry points, explaining each role. Include specific files only when they are key signposts.
- Include intended future paths when useful. Exclude generated, vendor, dependency, build-output, and secret files. Do not produce an exhaustive low-value inventory.

| Area | Path | Role in the target structure |
|---|---|---|
| `TODO: area or entry point` | `TODO: desired path` | `TODO: responsibility and navigation hint` |

## Runtime and delivery

`TODO: Describe how the target system runs, is built, deployed, or distributed.`

## Technology and boundaries

`TODO: Summarize the target languages, frameworks, runtimes, and major dependency boundaries. Link each independently runnable or deployable boundary to its container blueprint.`

- [`containers/container-slug.md`](containers/container-slug.md) — `TODO: target boundary and role`

## Cross-cutting design

- **Configuration and secrets handling:** `TODO: describe the intended mechanism; never include secret values.`
- **Identity and access:** `TODO: describe only when relevant.`
- **Errors, reliability, and observability:** `TODO: summarize relevant target conventions.`
- **Persistence and data evolution:** `TODO: describe only when relevant.`
- **Verification approach:** `TODO: identify the checks that support the target behavior.`

## Architectural constraints

`TODO: Record durable constraints that shape the target structure. Put initiative-specific questions, gaps, and follow-up actions in initiative records.`

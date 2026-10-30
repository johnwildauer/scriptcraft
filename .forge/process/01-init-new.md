# Initialize a New Repository

## Purpose

Use this process when the host repository is empty or has no reliable product/codebase baseline. The goal is not to generate a large speculative architecture or an immediate product MVP; it is to establish enough shared context to start a workspace foundation safely and, when useful, plan the next product stages.

## Read first

Read [`00-overview.md`](00-overview.md), then open the project manifest at
[`../project-manifest.md`](../project-manifest.md). Initialization is
agent-guided: you conduct a conversational intake with the user and complete
the manifest together. Users may pre-fill the manifest's Project intake
section, but that is optional, and its absence must never block
initialization. Treat pre-filled intake answers as user-provided context, not
repository evidence; technical choices remain provisional until
implementation validates them.

Initialization steps below are the default path. Follow explicit user direction
to adapt project policy, record the decision and rationale in the manifest
revision history or another appropriate durable project document, and retain
applicable external repository rules. A user may request process-document
customization by an architect or coding agent.

## Guided intake

Before asking anything structured, briefly introduce the workflow in plain
language: what the forge will set up, that independent checks run in the
background, and where the user will personally make decisions. Keep jargon out
of the conversation — use plain language, define any term the moment it must
appear, and never require the user to know workflow vocabulary (gates,
cadence, blueprints). Translate their answers into the manifest fields
yourself.

Conduct the intake in stages:

1. **Open intent.** Start with an open prompt — what the user wants to
   create, or keep creating, and who it is for. Listen for goals, users,
   constraints, and context before asking anything structured.
2. **Involvement gauge.** Determine how much the user wants to participate in
   technical choices — gating states, language choices, linting methods,
   workspace defaults — versus wanting working results with the diligence
   handled in the background. Ask directly when the signals are unclear, and
   record the `guided` | `collaborative` | `direct` level and how it was
    decided in the manifest. The level changes who supplies the answers. Record
    fields relevant to the current project; default gates and review steps apply
    unless the user explicitly adapts project policy, and applicable external
    requirements remain in force.
3. **Branch by level.**
   - `guided`: propose a short, plain-language defaults summary — what you
     will set up and how the work will be independently checked — and record
     the user's approval. Ask only genuinely user-only questions (goals,
     priorities, taste).
   - `collaborative`: ask grouped questions about the consequential choices,
     such as language or framework when undetermined and where the user wants
     checkpoints, and select defaults for the rest.
   - `direct`: conduct the full grouped questionnaire below.
4. **Recap and record.** Summarize what was configured in plain language,
   complete the manifest's intake, involvement, preference, baseline, and
   policy sections, and mark agent-selected choices as `agent-default` with
   rationale.

### Full questionnaire (`direct` level)

Conduct a short, conversational intake and record the answers in the canonical
[`../project-manifest.md`](../project-manifest.md). Ask grouped questions
rather than presenting a rigid form. Preserve `unknown` answers when a
technical choice is not known, mark the choice provisional, and assign a
decision owner and next action.

Capture, at minimum:

- repository state (`new`, `existing`, or `uncertain`) and evidence;
- primary goal, users/stakeholders, expected first outcome, explicit non-goals, and tool-use intent;
- preferred language, framework, runtime, package/build tool, and target platform;
- working preferences, including speed, learning, safety, documentation depth, and review strictness;
- rapid-mode preference as a working preference only: whether the user generally wants to be offered the rapid initiative path for small, well-understood changes; a preference never makes rapid mode the default and never replaces the explicit per-initiative request required by [`00-overview.md`](00-overview.md);
- Git installation/repository state, current/default branch, local branch capability, remote expectation, initiative branch policy and default initiative branch naming convention, and reduced-capability fallback;
- required human, external/manual, and automated gates;
- the ideal user/agent development cycle: where the user expects to manually gate the process (after design review, at validation, at closeout, or none), including any manual verification or compilation requirement that gates validation;
- review configuration: design review requirement, validation cadence (`per work order` | `consolidated` | `custom`), and requested depth;
- known, unknown, or unavailable commands; and
- unresolved decisions with owners and next actions, plus the recommended first initiative; and
- whether to plan only the first initiative or a staged roadmap toward the product MVP. If a staged roadmap is requested, capture either the user's intended total number of initiatives or an orchestrator-proposed sequence for user approval. The count is the total number of stages, including the mandatory workspace foundation, and must be at least one.

On resume, update the same manifest, preserve prior answers, and append a dated
revision entry explaining every changed answer.

## Git and branch capability check

Before recommending the first initiative, establish the repository's branch
capability and record the result in
[`../project-manifest.md`](../project-manifest.md).
Use available local inspection commands to determine whether Git is installed,
whether the repository is initialized, the current and default/base branch, and
whether a local initiative branch can be created. Record commands as known,
unknown, or unavailable; do not treat an unavailable check as evidence that a
capability exists. Determine whether a remote is required, available, or
intentionally irrelevant; a remote is not required merely because Git is used.

The default policy for a Git repository with local branch capability is a
dedicated initiative branch named `dev/YYYY-MM-DD-short-slug`, unless the
project manifest or host repository policy specifies another convention.
The Forge must not assume that its own name belongs in the branch namespace,
and branches created under a previous `forge/` default remain valid historical
evidence. A project may override the naming convention or choose `optional`,
but initialization must record the selected policy and the limitation. The user
may opt out for an initiative; record the opt-out, base branch, and alternate
file/diff evidence.

Initialize the **Default initiative branch naming convention** field in
[`../project-manifest.md`](../project-manifest.md) to
`dev/YYYY-MM-DD-short-slug` unless the user or host repository policy records a
different convention.

If Git is unavailable, the repository is not initialized, or local branch
creation is unavailable, label the workflow **reduced capability**. Preserve
file and comparable diff evidence where possible, explicitly state that no
branch-level review occurred, strongly recommend installing and initializing
Git locally where practical, and record the fallback owner and next action.

Record Git usability for work-order commits and exact-hash review separately
from local branch capability. If commits and hash review are unavailable at
initiative start, design review and work-order validation are recorded as
`skipped: Git unavailable` with no commit hash; requirements reviews and other
required safety gates remain. A later failure in a repository recorded as Git
usable is blocked, not skipped. Lack of branch creation alone does not disable
per-work-order commits or hash-based review.

## Procedure

1. **Establish context.** Confirm the repository root, available tools, constraints, target platform, and what “empty” means. Record unknowns rather than inventing facts.
2. **Elicit product intent.** Collaborate with the user on the problem, target users, desired outcome, product MVP boundary, non-goals, and success measures. Distinguish the product MVP from the workspace MVP described below.
3. **Choose the initial roadmap shape.** Ask whether the user wants one first initiative or a sequence toward the product MVP. Accept a count of one or greater, or let the orchestrator propose a provisional sequence for approval. Record initiative sequencing in the project manifest, not in product requirements. A count is a planning target, not a commitment to create or execute every stage immediately. If the user does not want later stages preplanned, record that the roadmap remains open for replanning after the workspace foundation.
4. **Draft product overview.** Populate [`../requirements/product-overview.md`](../requirements/product-overview.md) with desired product scope and outcomes in established-state voice, clearly distinguishing workspace MVP from product MVP. Keep initiative sequencing and roadmap/policy metadata in the project manifest.
5. **Group initial requirements.** Create cohesive area documents under [`../requirements/features/`](../requirements/features/) using stable requirement and acceptance-criterion IDs (for example separate UI, document-model, and CLI areas). Within an area, subgroup related capabilities where useful—for example, toolbar, status bar, windows, and menus within UI. Do not require one file per small feature.
6. **Review requirements (default).** Route a fresh-context reviewer using the requirements-review row in [`00-overview.md`](00-overview.md). Record `requirements-review.md` (or the configured equivalent) as a separate concise review artifact and resolve blockers before continuing unless the user explicitly adapts project policy and external rules permit it; do not treat this as a design review or store review status in requirements.
7. **Choose a technical baseline.** Based on requirements and user constraints, describe the desired runtime, technology, dependency, and validation structure in [`../blueprints/technical-context.md`](../blueprints/technical-context.md). Use intended paths and established-state voice, including paths that do not exist yet.
8. **Create grouped blueprints.** Add only the container, component, and feature-area blueprints needed to make the next initiative implementable. Group related capabilities where they share technical composition; do not create one blueprint per small feature or store lifecycle status or implementation evidence in blueprint content.
9. **Review blueprints (default).** Spawn a fresh-context reviewer to compare the blueprints with requirements and the stated technical constraints. Resolve blockers unless the user explicitly adapts project policy and external rules permit it.
10. **Configure policy.** Populate the Project policy section of [`../project-manifest.md`](../project-manifest.md) — commands, gates, tools, parallelism, and recovery rules — with verified facts, recording unknowns with owners. Do not place project-specific policy in process documents: they are swappable, and the manifest is the durable contract.
11. **Finalize initialization.** Set the manifest to `complete` only when completion evidence links the product overview, technical context, policy, reviews, branch capability decision, and first-initiative recommendation/roadmap. If a required decision or validation is unavailable, set it to `blocked` and record the blocking reason, decision needed, owner, and next action. Do not write individual product initiatives or begin implementation in this initialization step.

## Workspace-to-product roadmap

Initiative 1 for every new codebase is the **workspace MVP**: the smallest runnable and verifiable environment that confirms the selected (or intentionally provisional) language/toolset and development loop. It establishes only the agreed starter structure, setup instructions, a baseline check, and a smoke or manual verification path. Examples include a minimal Pixi Python app, a Next.js application with its selected backend boundary, or a UE5 plugin with its build/editor setup. It is not the product feature MVP.

The **product MVP** is the smallest useful product outcome for its intended users. Intermediate initiatives may establish application structure, navigation and footer, componentization, styling/CSS, or other independently valuable foundations before later initiatives deliver product behavior. Do not silently choose a toolset when it is unknown: include a short choice/discovery step in the workspace initiative and keep the decision provisional until resolved.

For each proposed stage, record a slug, purpose, dependency, observable outcome, validation boundary, and relationship to the product MVP in [`../project-manifest.md`](../project-manifest.md). Each stage must be independently reviewable. Preserve the option to stop after the workspace MVP, choose fewer or more later stages, or revise the roadmap after each completed initiative. Do not create placeholder initiatives merely to satisfy a count.

For example, a four-stage web-app plan can remain bite-sized while moving toward
the product MVP:

| Stage | Example scope | Distinct validation boundary |
|---|---|---|
| 1 | Workspace setup and toolchain baseline | A clean setup, baseline check, and smoke path run successfully. |
| 2 | Application structure, navigation, footer, and componentization | The agreed routes render through the component structure with a manual navigation check. |
| 3 | Styling/CSS foundation | The agreed structural screens satisfy the documented styling checks at the target viewport(s). |
| 4 | First product behavior | The selected user flow satisfies its requirements and acceptance criteria without treating later behavior as present. |

The example is illustrative, not a required architecture or fixed number of
stages.

## Required completion evidence

- Product overview exists and has an explicit MVP boundary.
- Each agreed initial capability has testable requirements in a cohesive area document; product overview contains no feature backlog.
- Technical context is a navigable desired-state map and necessary grouped blueprints describe target structure and contracts.
- Applicable requirements and blueprint review findings are recorded and resolved, accepted, or covered by a recorded user-directed policy adaptation.
- Project policy is populated in the project manifest or its unknowns are explicitly assigned.
- A workspace MVP is recommended as the first initiative, with rationale, scope, expected validation, and a linked configurable roadmap or explicit open-replanning decision.
- [`../project-manifest.md`](../project-manifest.md) is `complete` or `blocked`; a complete manifest links the generated product overview, technical context, policy, applicable review findings or recorded adaptations, and first-initiative handoff.

## Handoff

Tell the user or orchestrator that initialization is complete and point to [`02-initiative.md`](02-initiative.md). If a required decision remains unresolved, mark initialization `blocked` and record the decision needed instead of starting implementation.

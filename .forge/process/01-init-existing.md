# Initialize an Existing Repository

## Purpose

Use this process when `.forge/` is added to a repository containing source code, configuration, documentation, or an existing product. The goal is to create a trustworthy baseline of what exists before proposing changes.

## Read first

Read [`00-overview.md`](00-overview.md), then open the project manifest at
[`../project-manifest.md`](../project-manifest.md). Initialization is
agent-guided: you conduct a conversational intake with the user and complete
the manifest together. Users may pre-fill the manifest's Project intake
section, but that is optional, and its absence must never block
initialization. Treat pre-filled intake answers as user-provided context, not
repository evidence. Classify repository findings as **observed**
(directly supported by files or executed commands), **inferred** (reasonable
interpretation), or **desired** (requested future behavior).

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

Conduct the same short, conversational intake used for new repositories and
record it in the canonical [`../project-manifest.md`](../project-manifest.md).
Ask grouped questions and preserve the distinction between observed facts,
inferred choices, desired outcomes, and provisional answers. Preserve
`unknown` for technical choices that are not known; record the choice as
provisional with a decision owner and next action.

Capture, at minimum:

- repository state (`new`, `existing`, or `uncertain`) and inventory evidence;
- primary goal, users/stakeholders, expected first outcome, explicit non-goals, and tool-use intent;
- preferred language, framework, runtime, package/build tool, and target platform;
- working preferences, including speed, learning, safety, documentation depth, and review strictness;
- rapid-mode preference as a working preference only: whether the user generally wants to be offered the rapid initiative path for small, well-understood changes; a preference never makes rapid mode the default and never replaces the explicit per-initiative request required by [`00-overview.md`](00-overview.md);
- Git installation/repository state, current/default branch, local branch capability, remote expectation, initiative branch policy and default initiative branch naming convention, and reduced-capability fallback;
- required human, external/manual, and automated gates;
- the ideal user/agent development cycle: where the user expects to manually gate the process (after design review, at validation, at closeout, or none), including any manual verification or compilation requirement that gates validation;
- review configuration: design review requirement, validation cadence (`per work order` | `consolidated` | `custom`), and requested depth;
- known, unknown, or unavailable commands; and
- unresolved decisions with owners and next actions, plus the recommended first initiative.

On resume, update the same manifest, preserve prior answers, and append a dated
revision entry explaining every changed answer.

## Git and branch capability check

During the repository inventory, establish and record in
[`../project-manifest.md`](../project-manifest.md) whether Git is
installed and initialized, the current and default/base branch, whether a
local initiative branch can be created, and whether a remote is required,
available, or intentionally irrelevant. Record the evidence or check result;
an unknown or unavailable command is not evidence of capability.

For a Git repository with local branch capability, use a dedicated initiative
branch by default, named `dev/YYYY-MM-DD-short-slug`, unless the project
manifest or host repository policy specifies another convention. The Forge must
not assume that its own name belongs in the branch namespace, and branches
created under a previous `forge/` default remain valid historical evidence. The
user may select `optional` branching or opt out for an initiative, but the
initiative must record the limitation, base branch, and alternate file/diff
evidence. A remote is never required solely to use local branches.

Initialize the **Default initiative branch naming convention** field in
[`../project-manifest.md`](../project-manifest.md) to
`dev/YYYY-MM-DD-short-slug` unless the user or host repository policy records a
different convention.

If Git, repository initialization, or local branch creation is unavailable,
label the workflow **reduced capability**. Preserve file and comparable diff
evidence where possible, state explicitly that branch-level review did not
occur, strongly recommend installing and initializing Git locally where
practical, and record the fallback owner and next action.

Record Git usability for work-order commits and exact-hash review separately
from local branch capability. If commits and hash review are unavailable at
initiative start, design review and work-order validation are recorded as
`skipped: Git unavailable` with no commit hash; requirements reviews and other
required safety gates remain. A later failure in a repository recorded as Git
usable is blocked, not skipped. Lack of branch creation alone does not disable
per-work-order commits or hash-based review.

## Procedure

1. **Inventory the repository.** Identify languages, frameworks, package managers, entry points, deployment files, tests, generated files, and important directories.
2. **Determine runnable state.** Locate documented commands, install dependencies when permitted, run the safest available checks, and record failures verbatim. Do not assume a failing or missing command is a green gate.
3. **Recover product intent.** Read existing documentation and ask only the questions needed to distinguish intended behavior from current behavior.
4. **Draft product overview.** Populate [`../requirements/product-overview.md`](../requirements/product-overview.md) with stakeholder/business context and desired product outcomes in established-state voice. Keep initiative sequencing and roadmap/policy metadata in the project manifest.
5. **Group feature requirements.** Create or update cohesive area documents under [`../requirements/features/`](../requirements/features/), using stable requirement and acceptance-criterion IDs. An area can group related capabilities into sections; do not require one file per small feature. Specify the intended target, not a status report of observed implementation.
6. **Review requirements (default).** Route a fresh-context requirements reviewer using [`00-overview.md`](00-overview.md). Record `requirements-review.md` (or the configured equivalent) as a separate concise review artifact; identify ambiguity, unsupported claims, conflicts, and missing acceptance criteria, and resolve blockers before blueprinting unless the user explicitly adapts project policy and external rules permit it. Do not treat this as a design review or add status fields to requirements.
7. **Map the desired technical system.** Populate [`../blueprints/technical-context.md`](../blueprints/technical-context.md) as a navigable target-state map, informed by repository structure. For roughly fewer than 20 meaningful project files, map important files individually; for larger projects, map major folders and important entry points. Include useful intended paths before code exists and explain each role. Exclude generated, vendor, dependency, build-output, and secret files; do not make an exhaustive low-value inventory. Add container, component, and feature-area blueprints with links to relevant paths or symbols where useful.
8. **Review blueprints (default).** Spawn a fresh-context reviewer to check that the desired technical structure aligns with requirements and that its boundaries and contracts are clear. Keep the review findings separate from blueprint content. Carry each discovered, relevant target/repository mismatch into the recommended first initiative's plan or handoff (or another applicable initiative record), not as implementation status or a backlog in the blueprints.
9. **Configure policy.** Populate the Project policy section of [`../project-manifest.md`](../project-manifest.md) — commands, gates, tools, parallelism, and recovery rules — with verified facts, recording unknowns with owners. Do not place project-specific policy in process documents: they are swappable, and the manifest is the durable contract.
10. **Finalize initialization.** If the repository is broken, prioritize restoring a runnable baseline. Otherwise recommend the smallest valuable change that exercises the documented workflow. Set the manifest to `complete` only with linked completion evidence, including the branch capability decision; if the repository cannot be safely understood or validated, set it to `blocked` with the blocking reason, decision needed, owner, and next action.

## Required completion evidence

- Repository inventory and runnable-state evidence are recorded in the project manifest or relevant initiative record; do not put implementation evidence in requirements or blueprints.
- Requirements and blueprints state the desired target. Any discovered mismatch with the repository is recorded in the relevant initiative record, not as status, progress, or a backlog in durable specifications.
- Applicable requirements and blueprint reviews have independent findings, or the user's policy adaptation is recorded.
- Project policy in the project manifest contains verified commands where available and named owners for unknowns.
- The first initiative addresses the highest-value baseline gap or delivers a small vertical slice.
- [`../project-manifest.md`](../project-manifest.md) is `complete` or `blocked`; a complete manifest links the generated product overview, technical context, policy, applicable review findings or recorded adaptations, and first-initiative handoff.

## Handoff

Point the orchestrator to [`02-initiative.md`](02-initiative.md). If the repository cannot be safely understood or validated, mark initialization `blocked` and recommend a discovery or recovery initiative rather than guessing.

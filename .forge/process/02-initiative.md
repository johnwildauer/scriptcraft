# Plan and Design an Initiative

## Purpose

This process turns a request into a useful plan and, when requested, appropriately
scoped work-order drafts. The normal pattern is exploration, planning, review and
approval, implementation, per-work-order validation, and closeout, but users may
direct an appropriate path without obeying exact phase labels. Drafting work
orders does not approve or authorize their implementation. When Git is usable,
implementation normally follows a passing
[`03-design-review.md`](03-design-review.md) and any required human design
approval. If Git was unavailable for commit and hash review at initiative start,
the documented exception records design review and work-order validation as
skipped; requirements reviews and other applicable safety gates remain.

The initiative may be described as `draft` while being explored. The user guides
the direction; the agent investigates, records material findings, and responds
to the user's current request. Do not manufacture work orders, but when asked to
draft them from plans or feature requests, do so without requiring a phase-label
transition or redundant confirmation.

## Define the initiative boundary

An initiative is a coherent sprint/branch of work with one observable outcome, a small set of related work orders, and a finishable review and closeout boundary. It is a planning boundary, not a duration, staffing, or story-point estimate: an initiative may take longer than one day when its outcome remains coherent, and it need not constitute a complete product release. An initiative may also represent an A/B experiment, a plausibility branch, a workspace foundation, or one staged step toward an MVP. Staged initiatives must link each step to the roadmap or parent product goal while remaining independently reviewable.

Treat the following as advisory signals to split an initiative into smaller initiatives before design review:

- it combines multiple unrelated observable outcomes;
- it has no single validation boundary at which the stated outcome can be assessed;
- it requires several independent product decisions that could change the direction of separate work;
- it combines structure or componentization, styling, and unrelated product behavior without a clear shared outcome; or
- review would require treating future work, unapproved scope, or a later stage as if it were already present.

When a split is useful, preserve traceability by recording the roadmap or parent product goal, the dependency between the resulting initiatives, and each initiative's distinct outcome and validation boundary. Do not split solely because an initiative is large in duration or contains more than a fixed number of work orders. Keep a reasonable larger initiative intact when it has one coherent outcome and one reviewable validation boundary.

## Create the initiative

1. Use [`00-overview.md`](00-overview.md), relevant requirements/blueprints, and the initiative template as practical guidance; follow the user's actual request and applicable external/project requirements.
2. Create or update a dated, short-slug initiative directory under [`../initiatives/`](../initiatives/). Copy the plan template and, when useful, execution-log template. Work orders may be drafted directly when the user asks; they need not wait for exact phase wording. Create review artifacts when ready for the default design review. Replace applicable placeholders. `standard` is the default; `rapid` requires explicit user request and eligibility. Do not rename the source template directory.
   The resulting layout is:

   ```text
   .forge/initiatives/YYYY-MM-DD-short-slug/
     short-slug-v1.md
     execution-log.md
     reviews/
     validations/
     work-orders/
   ```

    Keep the plural `reviews/` and `validations/` directories, but they may remain empty during the draft phase. Requirements and design reviews belong in `reviews/`, named `requirements-review.md` or `design-review.md`; validation artifacts belong in `validations/`, named `validation-[work-order-id].md`. Keep each artifact inside its applicable directory.
    Update copied links so they target generated artifacts rather than source
    template names.
3. Resolve the branch decision before dispatching any implementation work. Read the
   branch policy in the project manifest, then:
   - when Git and local branches are available and policy is `required` or
     `optional`, create the dedicated branch by default using
     `dev/YYYY-MM-DD-short-slug` (or the convention recorded in the
     project manifest or host repository policy), based on the recorded
     current/default branch. Branch creation is a git action requiring an
     execution-capable session: when the orchestrator session cannot run git
     commands, dispatch it to an execution-capable coding session or supply
     the exact commands to the user, and record the resulting evidence before
     dispatching implementation. The Forge must not assume that its own name
     belongs in the branch namespace; branches created under a previous
     `forge/` default remain valid historical evidence;
   - when the user opts out or policy permits no branch, record the opt-out,
     base branch, and the comparable file/diff evidence that will replace
     branch evidence;
   - when Git is unusable for commit and hash review at initiative start, mark
     the initiative as reduced capability, record the no-branch notice and Git
     recommendation, and use named file/diff evidence; design review and
     work-order validation may be recorded as `skipped: Git unavailable` only
     under the explicit exception in [`00-overview.md`](00-overview.md);
   - when Git works but local branch creation is unavailable or disallowed,
     record the no-branch limitation and fallback evidence. This alone does not
     skip design review or hash-based validation; per-WO commits remain required
     when Git is usable.

   Record the base branch, initiative branch (or `none`), policy, opt-out, and
   evidence in the initiative metadata. A remote is not required unless the
   recorded project policy says it is.

   Independently record whether Git can commit and review exact hashes at
   initiative start. A later transient command or commit failure after a `yes`
   decision is blocked, not retroactively treated as Git unavailable. Keep
   branch creation capability distinct from commit/hash capability.

   If a branch is required and creation fails, apply the `blocked` overlay to
   the initiative, record the failed operation and evidence, name the owner
   and next action, and do not dispatch implementation. If policy is optional,
   log the failure and
   continue only with an explicitly named reduced-capability fallback; never
   imply that branch-level review occurred.
4. Record the branch decision in the initiative plan before implementation is
   dispatched. Start the execution log for execution-phase events when the
   first work order is approved or implementation begins, including the branch
   decision and any failure or fallback evidence relevant to execution. Do not
   use the execution log as a transcript of exploratory design discussion.

## Initiative modes and the rapid path

Initiatives run in `standard` mode by default. `rapid` mode is valid only when
the user explicitly requests it with language such as "use the rapid initiative
path" or "fast track this"; an agent may recommend rapid mode but may not
silently select it. Eligibility conditions, the insufficient-information rule,
and the mode metadata fields are defined in [`00-overview.md`](00-overview.md)
and recorded in the initiative template.

Rapid mode takes the initial prompt as a complete draft and compresses
exploration. It does not imply design approval or execution. Retain a useful
initiative plan, explicit scope, work orders, proportionate durable-document
impact decisions, default design review/approval, per-work-order validation,
and closeout unless the user explicitly adapts project policy and applicable
external rules permit it. A single work order still gets its own artifact; each
distinct task gets its own work-order file.

If the user requests rapid mode but has not supplied enough information to
complete the authorized tasks, prompt the user to fill the gaps before creating
or executing work orders. Never silently guess, expand scope, or downgrade the
request into an incomplete implementation.

## Exploration and convergence loop

The initiative plan is the working scratchpad for the planning conversation.
The orchestrator may dispatch one or more architect agents to collaborate with
the user, but the user remains the primary guide. During exploration, the agent
may inspect the repository, research options, compare approaches, and record
findings directly in the initiative plan. Useful planning material includes:

- observed repository findings and relevant evidence;
- hypotheses, assumptions, and unresolved questions;
- candidate approaches and their pros and cons;
- decisions made with the user and alternatives rejected;
- possible scope boundaries, risks, and durable-document impacts; and
- research or inspection needed before committing to a direction.

Keep exploratory notes distinct from settled scope and authoritative design.
Append material conclusions to the plan without rewriting history, and update
the initiative plan version when a material design decision changes. The
execution log is not required for every planning exchange; use it for
execution-phase state transitions, implementation evidence, failures, and
follow-up.

The agent should remain in exploration or convergence when the user is asking
questions, testing an idea, or supplying direction. If the user asks what to
do next or says they do not know where to focus, the agent may suggest a small
set of evidence-based next steps. Otherwise, do not proactively steer the
conversation toward work-order creation, design review, or implementation.

When the user indicates that the direction is settled, or asks to formalize the
plan, move to **convergence**. Confirm the following in the initiative plan:

- one observable initiative outcome;
- explicit in-scope and out-of-scope boundaries;
- the selected approach and important rejected alternatives;
- relevant requirements and blueprints;
- the validation boundary and success measures;
- risks, dependencies, and unresolved decisions; and
- the intended work-order breakdown.

Do not treat a filled template or an agent's confidence as a convergence signal.
If the user has not indicated readiness, continue the open-ended planning loop.

## Prepare work-order drafts

Create work orders when the user asks to draft them from a plan or feature set;
do not require exact phase labels or redundant confirmation. If the requested
scope is unclear or materially conflicting, clarify the actual blocker. Work
orders are drafts for subsequent review and approval by default, not permission
to begin implementation. The orchestrator should:

1. preserve the intended outcome and explicit boundaries to the extent known;
2. capture material decisions, exclusions, dependencies, and actual blockers;
3. create the work-order drafts requested; and
4. prepare default design review and approval separately before execution.

Copy the work-order template for each requested, distinct task. Each work order
must be intentionally narrow: one agent, one context window, quickly completed,
and one independently understandable and verifiable outcome. Include explicit
scope, relevant requirements and blueprint references, general code location,
needed implementation guidance, tests/checks, and a concise handoff. Include
durable-document impact when relevant and proportionate per
[`00-overview.md`](00-overview.md). Additional detail plausibly subordinate to
the outcome is not drift by itself.

For multiple distinct task groups, use IDs such as `WO-001a`, `WO-001b`,
`WO-001c`, then `WO-002a`, `WO-002b`. For a single throughline, use `WO-001`,
`WO-002`, and so on. Every ID names a distinct work order, not merely a group.
Fixes use collision-free suffixes such as `WO-001a-fix-01` or
`WO-001-fix-01`, incrementing as needed; validation artifacts use the same ID.
Older IDs remain valid history. The orchestrator handles dependency ordering,
verification, and relevant project checks; implementers assume dependencies
have been prepared. Parallel work should have non-overlapping ownership or a
documented merge strategy.

## Advisory documentation-first sequencing

When a task group needs a meaningful durable requirements or blueprint change, a useful default is a small document-only work order first, followed by its dependent implementation work order(s). Set dependency IDs explicitly in each affected work order; for many separate groups, sequence only related documents and implementation. In a focused initiative, use the same narrow split when appropriate. Do not create a documentation work order when there is no durable impact or the change is below the relevant abstraction level. The documentation work order defines the target before code changes; dependent implementers treat those documents as desired state even when current code lags. Record expected lag and dependent IDs once in the initiative execution handoff, not in requirements or blueprints. A document-only work order's commit description must identify it as documentation-first/pre-implementation and name its dependent implementation work-order IDs. If cancellation or changed direction makes the target obsolete, record the outcome in the initiative and arrange an authorized documentation correction rather than silently reverting durable documents.

## Design readiness and transition

When the plan and work orders are ready for the default review, confirm that the
intended work can be identified and compared with the outcome, and identify
applicable project-policy gates. Do not require phase labels or an extra
confirmation when the user has already asked directly for the work. Do not
turn ordinary omissions, optional detail, evidence gaps, or architecture and
quality preferences into readiness blockers. Copy the review template and
dispatch [`03-design-review.md`](03-design-review.md) as the default independent
review, distinct from drafting and execution. This is a fresh-context
alignment review of the converged plan and work orders, not a validation of
implemented work; requirements concerns that are not implementation defects
route to requirements review only when that review is explicitly required. If
Git was unavailable for commit and hash review at initiative start, record the
design review as `skipped: Git unavailable` instead of dispatching it, subject
to applicable external rules and project policy.

## Rework and approval

If the default design review finds a material issue, record the finding and
revise the relevant plan or work orders; do not implement around an applicable
failed design gate. By default, implementation follows a passed review and
required human design approval, distinct from work-order drafting. Record
material decisions and evidence without requiring exact phase labels, per-phase
ledger entries, or a fixed transition sequence. Follow an explicit user-directed
policy adaptation when external rules permit it; see [`00-overview.md`](00-overview.md).

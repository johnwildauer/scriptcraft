# Plan and Design an Initiative

## Purpose

This process opens a durable, user-led space for exploring a change and, when
the direction is clear, turns that exploration into one bounded initiative and
an ordered set of executable work orders. Creating an initiative does not mean
that its scope, design, or implementation plan is settled. Implementation
begins only after [`03-design-review.md`](03-design-review.md) passes and the
required human design approval is recorded.

The initiative begins in the `draft` phase. The user guides
the direction of the conversation; the agent investigates, records useful
findings in the initiative plan, and responds to the user's current question.
The agent must not manufacture work orders or advance the initiative merely
because a template contains those possible later artifacts.

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

1. Read [`00-overview.md`](00-overview.md), the relevant requirements and blueprints, and the initiative template at [`../initiatives/YYYY-MM-DD-initiative-template/`](../initiatives/YYYY-MM-DD-initiative-template/).
2. Create a dated, short-slug directory under [`../initiatives/`](../initiatives/). Copy the initiative plan template and rename it to `[initiative-slug]-v1.md`. Copy `execution-log-template.md` to `execution-log.md` for use when work-order execution begins. Do not create work-order or review artifacts yet; copy and rename those templates only when the initiative has converged and is being prepared for design review. Replace initiative-slug, title, and other applicable placeholders in the copied documents. Record the initiative mode metadata (`Initiative mode`, `Mode requested by`, `Mode confirmation`, `Rapid-mode eligibility`) per [`00-overview.md`](00-overview.md): `standard` is the default, and `rapid` requires the user's explicit request and confirmed eligibility. Do not rename the source template directory.
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
   Update copied links so they target the generated `[initiative-slug]-v1.md`,
   generated work orders, and these review filenames rather than the source
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
   - when Git or branch capability is unavailable, mark the initiative as
     reduced capability, record the no-branch review notice and Git
     recommendation, and use the named fallback evidence.

   Record the base branch, initiative branch (or `none`), policy, opt-out, and
   evidence in the initiative metadata. A remote is not required unless the
   recorded project policy says it is.

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

Rapid mode removes the default gates around the draft phase: the initial
initiative prompt is taken as a complete draft, ready for planning (creating
work orders) and execution/validation. The exploration/convergence loop is
replaced by a single confirmation that the prompt-as-draft is accurate, and the
design-readiness confirmation replaces the full design-review gate. It must
never omit the initiative plan, explicit selected and excluded scope,
work-order artifacts, durable-document impact decisions, the per-work-order
validation loop with separate sessions, or closeout and follow-up
recording. The transition procedure in
[Prepare work orders after convergence](#prepare-work-orders-after-convergence)
still applies in compressed form: the initiative moves from `draft` to
`plan` when work orders are created, and to `review` when all review artifacts
are ready. A rapid initiative with one work order still has a work-order
artifact; a rapid initiative with multiple independent changes still gives each
change its own work-order file and implementer session.

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

## Prepare work orders after convergence

Create work orders only after convergence and an explicit request or transition
to formal design preparation. When the user asks to create work orders, the
orchestrator must:

1. confirm that the initiative has one outcome and explicit boundaries;
2. summarize the work-order plan and any meaningful reductions or decisions
   made during convergence;
3. ask for confirmation when the initial prompt was broad but the resulting
   work orders are a direct translation of the user's stated authorization, and
   record the confirmation in the initiative plan;
4. set the initiative to `plan` and record the transition evidence in the
   initiative plan;
5. create the work orders required by the converged plan; and
6. set the initiative to `review` only when all review artifacts are ready.

At that point, copy the work-order templates and create only the work orders
needed by the agreed design. Each work order must have one clear owner and
outcome, explicit in/out-of-scope boundaries, verbatim applicable requirements,
blueprint references, implementation guidance, files/systems likely to change,
validation steps, and a durable-document impact classification per
[`00-overview.md`](00-overview.md). Additional detail and granularity that is
plausibly subordinate to the agreed outcome is expected and is not drift by
itself.

Work orders should be independently reviewable and ordered by dependency. Sequential execution is the default. Parallel work requires non-overlapping ownership or a documented merge strategy in the initiative plan.

## Design readiness and transition

Before requesting review, confirm that the user has asked to formalize the
initiative or has explicitly agreed that it is ready for design review. Then
confirm that the plan has no known hard-stop blocker, the intended work can be
identified and compared with the outcome, work orders exist with their expected
validation boundary, and applicable project-policy gates are identified. Do not
turn ordinary omissions, optional detail, evidence gaps, or architecture and
quality preferences into readiness blockers; additional work-order detail is
expected. Copy the review template, set the initiative to `review`, and
dispatch [`03-design-review.md`](03-design-review.md). This is a fresh-context
alignment review of the converged plan and work orders, not a validation of
implemented work; requirements concerns that are not implementation defects
route to requirements review only when that review is explicitly required.

## Rework and approval

If design review fails, record the findings and return the initiative to
`draft` when the direction itself is unsettled, or to `plan` when only the work
orders need revision, and revise the plan/work orders. Do not implement around a
failed design gate. When the review passes and the required human design
approval is recorded, the initiative passes the `review -> execute` gate and
implementation begins in `execute` per the orchestrator execution loop in
[`00-overview.md`](00-overview.md).

# Plan an Initiative

## Purpose

Use this guidance to turn a request into a bounded, useful plan and discrete work
orders when appropriate. Follow the user's direction and applicable project or
external requirements. No extra approval gate or persisted phase/status progression
is part of the default workflow. The orchestrator manages progress in context;
initiative documents are kept up to date rather than used as a persisted
progress/status system.

## Plan the outcome and scope

An initiative groups related work around an observable outcome. Keep the plan
proportionate: state what should change, what is in and out of scope, relevant
context, important decisions or open questions, and any useful requirements or
blueprint links. Split unrelated outcomes or independently directed efforts when
that makes the work clearer; do not split merely to satisfy a size, duration, or
work-order-count rule.

Explore the repository or options as useful. Record material findings, candidate
approaches, rationale, and decisions in the initiative plan. Keep unresolved
questions visible when they affect the requested direction; do not turn ordinary
uncertainty or missing optional detail into a gate. The user remains the guide,
and the orchestrator handles dependencies, actual issues, and decisions for the
user.

## Create an initiative and plan work

Copy the initiative template to a dated, short-slug directory under
[`../initiatives/`](../initiatives/), replacing placeholders and updating links
to copied artifacts. Use the execution-log template when useful. Create only the
work-order and validation artifacts needed for the work.

Create or draft work orders in response to the request; no redundant confirmation
is required. If the requested scope
has a material conflict or blocker, raise it to the orchestrator. Give each
distinct task a bounded, independently understandable work order with a clear
outcome, relevant scope and references, useful implementation guidance, and
appropriate checks. Note dependencies in the initiative plan where useful. IDs
such as `WO-001` and `WO-002` identify individual work orders. The orchestrator
coordinates dependencies and any parallel ownership.

When a meaningful requirements or blueprint change should precede implementation,
a focused document work order followed by dependent implementation work orders
can be useful. This is advisory: do not create documentation work merely to keep
documents synchronized when there is no durable impact. Requirements and
blueprints describe desired state, not initiative progress or implementation
evidence. Raise relevant mismatches to the orchestrator without broadening scope.

## Execution and evidence

Preserve unrelated user changes and isolate attributable work. The implementer
commits safely attributable work-order changes before validation when Git is
available. After each completed discrete work order, the orchestrator dispatches
exactly one fresh coding validator using the prompt in [`05-validation.md`](05-validation.md).
The validator records its finding in a separate commit, not in the implementation
commit. If Git is unavailable, no attributable diff exists, or safe attribution is
not possible, still dispatch one validator and report the evidence limitation
without claiming alignment or inventing a commit. The validator does not decide
dependencies or resolve actual issues; the orchestrator handles those, user
direction, and applicable external requirements. See
[`00-overview.md`](00-overview.md) and the relevant work-order and validation
guidance for execution details.

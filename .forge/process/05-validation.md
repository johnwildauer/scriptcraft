# Validation

## Purpose

Validation is a lightweight implementation-alignment and anti-rogue check. It
verifies that the reported work happened, that the changed files and relevant
diff are generally the kind of files and behavior authorized by the work order,
and that no clearly unrelated, deceptive, backdoor, malicious, or nefarious
behavior was introduced. It is not a general quality gate or a second design
review. Projects that require manual verification or compilation (C++,
game-engine codebases, device targets) configure **user gates** at validation;
those gates are hard stops only when project policy makes them required.

## Dispatch preconditions

The orchestrator must make these artifacts available before dispatch:

- the work order is `in-review` and reached that state after implementation;
- the approved work order and initiative context;
- the execution log, changed-file record, and validation attempts/results;
- a comparable branch diff, file diff, or equivalent implementation evidence.

The validator records missing, incomplete, failed, or unavailable evidence as a
note or warning unless it prevents any meaningful comparison. If no meaningful
comparison is possible, validation is `blocked`; do not infer implementation
from a plan, conversation, or unchanged files. A required project-policy/manual
gate that is unavailable or failed is also a hard stop.

## Dispatch checklist

Before dispatching a validation, the orchestrator must confirm:

- a separate fresh-context validation session is confirmed for this validation;
- the role and work-order/validation identifier are recorded in the execution log with the session/agent identifier;
- prior work from the same initiative is supplied as artifacts, not assumed conversational memory; and
- any exception to session separation is explicitly approved by the user and logged with reduced-confidence evidence.

## Role

Act as an independent, fresh-context validator in a separate validation session
after a work order is implemented. Load the approved work order, initiative
context, execution log, changed-file record, validation attempts, and relevant
diff. Verify the report against those artifacts and assess only implementation
alignment and rogue behavior. Report findings; do not silently modify the
implementation. Additional detail is expected when it is plausibly subordinate
to the work order. Do not anticipate future tests or judge unimplemented design
obligations.

## Validation checklist

- The reported implementation, changed-file record, validation attempts, and
  relevant diff are sufficiently consistent to support a meaningful comparison.
- The changed files and diff are generally the kind of files and behavior the
  work order authorizes; only clearly unrelated scope is a blocker. Detail that
  is plausibly subordinate to the work order is aligned, not scope creep.
- No clearly deceptive, backdoor, malicious, nefarious, or otherwise rogue
  behavior was introduced.
- Any explicit acceptance criterion or project-policy/manual gate that is
  applicable and required is addressed. A required gate may be a hard stop;
  optional or unavailable checks are recorded without automatic failure.
- The work order's durable-document impact classification is not plainly
  contradicted by the implementation; minor evidence or documentation gaps are
  notes unless they prevent comparison.

Do not fail validation for debugging opportunities, cleanup, refactoring,
maintainability, style, architecture preferences, unrequired test/lint/build
gaps, unrequested security hardening, compatibility or persistence concerns, or
suggestions for a better implementation. Do not use validation to anticipate
future tests or design obligations.

## Outcome

Create `validation-[work-order-id].md` under the initiative's `validations/`
directory from [`../initiatives/YYYY-MM-DD-initiative-template/validations/validation-template.md`](../initiatives/YYYY-MM-DD-initiative-template/validations/validation-template.md).
Link it from the work order and execution log. Record the comparison basis and
every finding with severity, evidence, recommendation, and required next state.
Warnings and notes do not automatically create fix work orders.

- **Pass:** the implementation is aligned; the orchestrator sets the work order
  to `complete` and proceeds. Notes or warnings may remain in the artifact and
  do not require fix work.
- **Fail:** use only for a genuine hard-stop implementation finding: clearly
  unrelated or rogue behavior, a plainly unmet required criterion, or a failed
  required policy/manual gate. The orchestrator creates a fix work order (below)
  only when a concrete implementation fix is required.
- **Blocked:** use when meaningful comparison is impossible or a required gate
  cannot be completed; record the blocker and stop until it is resolved.

Do not route design preferences, quality suggestions, or design deficiencies to
fix work. They are notes, or follow the separate design-review process when
explicitly required by project policy.
Human approval is not required after every validation by default; follow the
configured validation cadence and project policy, and record any bypass.

## Fix work orders

When a validation has a genuine hard-stop finding requiring implementation
changes, the orchestrator creates a **fix work order** rather than reopening the
original:

- **Naming:** letter-suffixed revision of the original — `WO-001a-short-slug.md`,
  then `WO-001b-short-slug.md`, and so on.
- **Content:** links the original work order and the failing validation
  (`validation-WO-001.md`), carries the validator's findings and required fixes,
  and states the fix scope within the original's boundaries.
- **Dispatch:** a **new** implementer session — the fresh-session invariant
  applies to fixes — followed by a fresh validation session
  (`validation-WO-001a.md`).
- **Completion:** the original work order is set to `complete` only when one of
  its fix work orders passes validation, with the fix chain linked as evidence.

## Validation cadence

The default is the **per-work-order loop**: the orchestrator dispatches a fresh
implementer session, receives completion, dispatches a fresh validation session,
and receives the result — repeated for each work order in dependency order (see
the execution loop in [`00-overview.md`](00-overview.md)). The initiative's
phase alternates `execute` and `validate` with each dispatch.

**Consolidated validation is opt-in only.** Validation after all work orders are
implemented happens solely at the user's behest, when the user judges the work
orders discrete enough that cascade failure is low risk and explicitly requests
it. The agent must warn the user when it has high confidence that cascade
failure is probable without per-work-order validation. Even when consolidated,
validations run per work order from separate fresh validation sessions. Even in
rapid mode, validation occurs per work order from a different validation agent;
rapid mode changes the draft phase, not the validation loop.

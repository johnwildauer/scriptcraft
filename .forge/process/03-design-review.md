# Design Review

## Dispatch conditions

Do not dispatch this review during the open-ended planning conversation. The
initiative must have completed its exploration/convergence loop, passed the
`draft -> plan` exit gate, moved to `plan` with its work orders created, and
reached the `review` phase: the user or authorized orchestrator must have
explicitly requested formal design preparation or agreed that the plan is ready
for review. The initiative plan must identify one outcome, scope boundaries, a
proposed approach, validation boundary, and the work orders created from that
agreed design. Dispatch this review to a separate fresh-context session per the
dispatch invariants in [`00-overview.md`](00-overview.md).

## Role

Act as a fresh-context alignment reviewer. Read the initiative, all generated
work orders, relevant requirements and blueprints, and [`00-overview.md`](00-overview.md).
Do not implement or silently rewrite the design under review. Check only that
the work orders are generally aimed at the initiative outcome and are not
clearly unrelated to it or contrary to an explicit approved user decision.
Additional detail and granularity in work orders is expected; it is not drift
by itself. Do not reconstruct discarded exploratory possibilities as omissions
or invent requirements the work-order plan does not carry forward. Do not
anticipate tests, debug, refactor, clean up, improve architecture, or assess
implementation-level concerns. If the initiative is still exploratory, lacks
generated work orders, or has no explicit readiness signal, return it to
planning rather than treating the missing artifacts as alignment findings.

## Authority: block versus warn

The reviewer may block the initiative only for a finding that is clearly:

- unrelated to or out of scope for the initiative outcome;
- in direct contradiction of an explicit approved user decision;
- dependent on an execution-blocking impossible dependency;
- impossible to understand because the intended work cannot be identified; or
- blocked by an explicit project-policy gate.

Ordinary omissions, risks, suggestions, missing optional detail, quality
improvements, and evidence gaps are notes or warnings and must not block. An
evidence gap becomes blocking only when it prevents meaningful comparison of
the work orders with the initiative outcome. Explicit project-policy gates
remain hard stops. A warning does not require owner acceptance for an aligned
review to pass.

Do not treat any of the following as drift or a blocker: reasonable reduction
from exploratory possibilities to executable scope, additional work-order
detail, authorized durable-document updates, or sub-blueprint implementation
changes whose parent blueprint remains accurate.

## Review questions

1. What initiative outcome and explicit approved user decisions are being used as the comparison boundary?
2. Is each work order generally aimed at that outcome?
3. Is any work order clearly unrelated, out of scope, or contrary to an explicit approved user decision?
4. Can the intended work and its ordinary execution dependencies be identified, or is an execution-blocking impossible dependency present?
5. Is an explicit project-policy gate unsatisfied?
6. Are missing artifacts or evidence preventing meaningful comparison, or are they merely ordinary notes or warnings?

## Output

Create a review using [`../initiatives/YYYY-MM-DD-initiative-template/reviews/review-template.md`](../initiatives/YYYY-MM-DD-initiative-template/reviews/review-template.md). Classify findings as blockers, warnings, or notes and include evidence and required action. Every blocker must cite the converged plan, requirement, policy rule, or evidence that makes it blocking.

The outcome is `pass` when no blocker remains; warnings do not require
acceptance and do not prevent an aligned result from passing. A failed review
returns the initiative to `plan` and must be logged; when the direction itself
is unsettled, it returns to `draft`. A review that finds the initiative
premature returns to `draft`; it does not authorize work-order execution. A
passing review, together with any separately required recorded human design
approval, is the `review -> execute` gate per [`00-overview.md`](00-overview.md).

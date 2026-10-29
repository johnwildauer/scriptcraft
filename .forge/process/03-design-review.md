# Design Review

## Dispatch

Dispatch only when the initiative is in `review`, its plan has converged, the
work orders derived from that plan exist, and the user or authorized orchestrator
has confirmed design readiness. Use a separate fresh-context review session per
the dispatch invariants in [`00-overview.md`](00-overview.md).

## Scope and verdict

Compare the initiative's agreed direction with its work-order plan. Use only the
initiative and its work orders. Consult a requirement, blueprint, or other
source only when needed to establish explicit direction. Do not audit the
repository, investigate unrelated concerns, or explore exhaustively.

Start from **pass**. **Fail only when direct evidence shows that the work-order
plan is plainly unrelated to the initiative's agreed direction.** Minor
differences, ambiguity, omissions, added detail, or reasonable implementation
decomposition are pass. Do not assess quality, feasibility, policy compliance,
or preferences. If an artifact needed for comparison is absent or unavailable,
return **blocked**, not fail, and identify only the missing evidence.

Project-policy requirements and human approvals are separate orchestrator-owned
gates; they are not design-review criteria. The orchestrator enforces and records
those gates independently of this alignment verdict.

The reviewer returns a short `pass`, `fail`, or `blocked` result with the direct
comparison basis and, only for fail or blocked, concise evidence. Use the review
template only for basic artifact metadata and this verdict; omit its unrelated
checklists, warnings, notes, and follow-up prompts. Do not spawn or delegate
agents, investigate further, propose fixes or new work orders, implement changes,
give unsolicited advice, or perform follow-up review.

## Artifact and transition

Create `design-review.md` in the initiative's `reviews/` directory using
[`../initiatives/YYYY-MM-DD-initiative-template/reviews/review-template.md`](../initiatives/YYYY-MM-DD-initiative-template/reviews/review-template.md)
as a format reference, not an expanded review mandate. Record only the initiative
and work orders compared, the verdict, and concrete evidence for any fail or
blocked result. Do not perform checklist items or assess policy/human gates;
those are recorded by the orchestrator separately. Link the review from the
initiative plan and execution log.

- **Pass:** the alignment check passes. The orchestrator advances from `review`
  to `execute` only when the separately required human design approval is also
  recorded.
- **Fail:** the orchestrator returns the initiative to `plan` for direction
  alignment; if the initiative direction itself is unsettled, it returns to
  `draft`.
- **Blocked:** the orchestrator records the blocker and stops until the missing
  comparison evidence is available, setting the initiative status to `blocked`
  as required. Do not treat blocked as fail.

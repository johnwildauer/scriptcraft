# Design Review

## Dispatch

By default, dispatch a fresh independent reviewer with the agreed initiative plan,
the work orders explicitly listed for review, and the concise review template.
Include narrowly targeted excerpts only when needed to understand a cited
requirement or blueprint. The user's direction to draft work orders is sufficient
to draft them; status wording is not an additional dispatch gate. The template
also supports initialization requirements reviews: in that use, provide the
requirements draft and relevant review assignment instead of initiative work
orders.

Before dispatching an initiative design review, check the initiative's recorded
Git capability. If Git was unavailable for commit and hash review at initiative
start, do not dispatch this stage: record `skipped: Git unavailable`, state that
no commit hash exists, and continue only as applicable external rules and project
policy allow. This exception applies only to initiative design review, not
requirements reviews, initialization blueprint reviews, other required safety
gates, or a direct user instruction to conduct a review. If Git was recorded
usable and a command later fails, treat the review as blocked rather than
retroactively skipped.

When dispatched for a **requirements review**, assess whether the supplied
product/feature intent, scope, users, and acceptance criteria are clear, coherent,
and testable. Do not assess implementation or expand the assignment into a design
review. Use the same concise artifact format, identify the requirements document
as the reviewed evidence, and report `pass` when no blocker remains, `fail` for a
concrete material issue, or `blocked` when required comparison material is
missing. Warnings remain distinct from blockers and require acceptance only
where applicable project policy says so. Initialization guidance makes this a
default review; a user may explicitly adapt project policy where external rules
permit.

## Scope and verdict

In initiative-design mode, compare the agreed initiative plan with its selected
work orders for meaningful alignment, using just those files and the concise
review template.
Use a narrowly targeted requirement or blueprint excerpt only if needed to
understand that alignment. Do not police semantics, phase labels, speculative
concerns, optional detail, or lengthy lists of negative behavioral prompts; do
not perform repository hunts.

In initiative-design mode, report **pass** when the work orders meaningfully
align with the agreed plan and the supplied evidence supports comparison. Report
**fail** for a significant, evidence-backed mismatch between the plan and the
selected work orders. Report **blocked** when required comparison material is
missing or unavailable; identify the missing input rather than infer a mismatch.
Minor differences, useful decomposition, and subordinate detail do not by
themselves prevent a pass. Requirements-review verdicts follow the requirements
criteria above.

Project-policy checks and human approvals are separate orchestrator-owned gates,
not review criteria. The orchestrator enforces and records them independently.

Return the outcome, date, documents/evidence reviewed, and a two-to-three-sentence
plain-language summary in the template. For fail or blocked, put direct mismatch
evidence or the missing input in the summary. Do not spawn agents, investigate
beyond the supplied comparison material, implement changes, or perform follow-up
review.

## Artifact and transition

For an initiative design review, create `design-review.md` in the initiative's
`reviews/` directory. For an initialization requirements review, create
`requirements-review.md` in the configured location. Use
[`../initiatives/YYYY-MM-DD-initiative-template/reviews/review-template.md`](../initiatives/YYYY-MM-DD-initiative-template/reviews/review-template.md)
as the concise artifact. The orchestrator links it from the initiative plan and
execution log when applicable, and records any separate policy or approval
evidence in the execution log.

- **Pass:** design alignment is supported; for requirements review, no blocker
  remains. The orchestrator proceeds only after any separately required human
  design approval is recorded.
- **Fail:** the orchestrator resolves the specific plan-to-work-order mismatch
  before implementation by default.
- **Blocked:** the orchestrator obtains the named missing comparison input.
  Blocked is not fail. A Git failure after Git was recorded usable is blocked.
- **Skipped:** the orchestrator may record `skipped: Git unavailable` only when
  Git was unavailable for commit and hash review at initiative start. No commit
  hash exists; this is neither pass nor approval. Requirements review is not
  skipped by this exception.

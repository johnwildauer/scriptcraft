# Validation

## Dispatch

The orchestrator dispatches one fresh-context validation session per work order
after implementation, when the work order is `in-review`. Provide the approved
initiative and work order plus the committed diff or equivalent implementation
evidence needed for comparison. The execution log or other context may identify
those artifacts; do not require unrelated material. If the initiative, approved
work order, or comparison evidence is absent or unavailable, return **blocked**
and name the missing input; do not infer implementation or fail speculatively.

## Scope and verdict

Compare the implementation diff/evidence with the initiative direction and the
approved work order. Use those as the minimum comparison basis. Consult other
documents only when needed to establish explicit authorization. Do not perform a
repository audit, exhaustive exploration, or general quality review.

Start from **pass**. **Fail only when direct evidence shows that the authored
implementation is grossly unrelated or rogue relative to the authorized
initiative and work order.** A large or complex change, many changed files,
reasonable stack-wide implementation details, minor differences, omissions, or
ambiguous alignment are not failures. Do not fail for style, quality, security,
architecture, testing, acceptance-criterion, or documentation opinions or
deficiencies unless they directly establish grossly unrelated or rogue work.
If reasonable alignment remains plausible, pass.

Project-policy requirements and human approvals/manual gates are separate
orchestrator-owned requirements, not validation criteria. The orchestrator
checks, records, and enforces them independently; their status does not change
the alignment verdict or expand this review.

Return only a short `pass`, `fail`, or `blocked` result and the required artifact.
State the comparison basis and cite concise, direct evidence for fail or the
missing evidence for blocked. Use the validation template only for basic
artifact metadata and this verdict; skip its acceptance-criterion, policy-gate,
quality, warning, note, and other checklist prompts. Do not spawn or delegate
agents, investigate further, fix or implement anything, create work orders, give
unsolicited advice, or perform follow-up review.

## Artifact and transition

Create `validation-[work-order-id].md` in the initiative's `validations/`
directory using
[`../initiatives/YYYY-MM-DD-initiative-template/validations/validation-template.md`](../initiatives/YYYY-MM-DD-initiative-template/validations/validation-template.md)
as a format reference, not an expanded validation mandate. Record only the work
order, initiative, compared commit/diff or evidence, verdict, and direct evidence
for any fail or blocked result. Do not perform its checklists or assess policy or
human gates; those are recorded by the orchestrator separately. Link the
validation from the work order and execution log.

- **Pass:** the orchestrator sets the work order to `complete` and proceeds,
  subject to any separately required project or human gates at their own
  workflow transition.
- **Fail:** only a concrete grossly unrelated or rogue implementation finding
  qualifies. The orchestrator returns the work order for correction and creates
  a letter-suffixed fix work order (for example, `WO-001a-short-slug`) when a
  fix is required; the fix receives a new implementation and validation session.
  Keep the original work order incomplete until a fix passes, and link the fix
  chain as evidence.
- **Blocked:** the orchestrator records the missing comparison evidence, marks
  the work blocked, and stops until it is available. Blocked is not fail.

The default cadence is per work order in dependency order, alternating
`execute` and `validate`; consolidated validation is opt-in only at the user's
explicit request. Each work order still receives its own fresh validation
session. The orchestrator owns all dispatches, state transitions, and separate
policy or human gates.

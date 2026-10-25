# Validation: WO-001 — Short Title

## Validation metadata

- **Review type:** `validation`
- **Status:** `in-review`
- **Validator:** `TODO`
- **Validator session/agent ID:** `TODO` <!-- fresh-context session per the dispatch invariants in [`../../../process/00-overview.md`](../../../process/00-overview.md) -->
- **Session exception:** `none` | `approved and logged` — evidence: `TODO`
- **Date:** `TODO`
- **Initiative:** [`../initiative-slug-v1.md`](../initiative-slug-v1.md) <!-- replace initiative-slug when copying -->
- **Validation file:** `validation-WO-001.md` <!-- replace WO-001 with the validated work order's ID; fix validations append the letter suffix, e.g. `validation-WO-001a.md` -->
- **Validated work order:** [`../work-orders/WO-001-short-title.md`](../work-orders/WO-001-short-title.md) <!-- replace placeholders when copying -->
- **Compared commit/diff:** `TODO`
- **Fix chain:** `none` | `TODO` <!-- for fix validations only: link the original work order, the failing validation, and the fix work order this validation covers -->

Validation files belong in this initiative's `validations/` directory, named
`validation-[work-order-id].md`. Requirements and design reviews are separate
artifacts and live in `reviews/`; use
[`../reviews/review-template.md`](../reviews/review-template.md) for those.

## Authorized scope

`TODO: State the authorized work-order scope and any explicit acceptance or
project-policy/manual gates assessed. State the comparison basis (changed-file
record and relevant diff). Do not assess unimplemented design obligations,
future tests, or future scope.`

## Validation checklist

- The reported work, changed-file record, validation attempts, and relevant diff
  support a meaningful comparison.
- Changed files and diff behavior are generally authorized by the work order;
  plausibly subordinate detail is not unrelated scope.
- No clearly deceptive, backdoor, malicious, nefarious, or otherwise rogue
  behavior was introduced.
- Explicit acceptance criteria and required project-policy/manual gates are
  addressed. Optional or unavailable checks are recorded as notes/warnings.
- Durable-document impact is not plainly contradicted by the implementation.

Do not fail for debugging opportunities, cleanup, refactoring, maintainability,
style, architecture preferences, unrequired test/lint/build gaps, unrequested
security hardening, compatibility/persistence concerns, or better-implementation
suggestions. Do not anticipate future tests or judge unimplemented obligations.

## Findings

### Blockers

Each blocker must cite evidence that makes it blocking. Blockers are limited to
an impossible meaningful comparison, clearly unrelated or rogue behavior, a
plainly unmet required criterion, or a failed/unavailable required policy gate.
Do not infer implementation from a plan, conversation, or unchanged files.

- `None or TODO: finding — Severity: blocker; Evidence: TODO; Recommendation: TODO; Required next state: in-progress | blocked`

### Warnings

- Use for incomplete or imperfect evidence, unavailable optional checks, or
  minor alignment uncertainty that does not prevent comparison. Warnings do not
  create fix work orders.
- `None or TODO: finding — Severity: warning; Evidence: TODO; Recommendation: TODO; Required next state: complete`

### Notes

- Use for recorded context, non-blocking gaps, and suggestions. Notes do not
  create fix work orders.
- `None or TODO: finding — Severity: note; Evidence: TODO; Recommendation: TODO; Required next state: complete`

## Decision

- **Outcome:** `pass | fail | blocked` <!-- pass means aligned, including any recorded warnings or notes; fail and blocked are reserved for defined hard-stop cases -->
- **Required next state:** `TODO` <!-- pass → complete; fail → fix work order only for a genuine hard-stop finding; blocked → blocked -->
- **Fix work order required:** `no` | `yes: WO-001a-short-slug` <!-- warnings and notes do not require a fix; use yes only for a genuine hard-stop implementation finding; letter-suffixed revision per [`../../../process/05-validation.md`](../../../process/05-validation.md) -->

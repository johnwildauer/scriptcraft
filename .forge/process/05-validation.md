# Validation

## Dispatch

By default, the orchestrator dispatches a fresh independent validator for each
work order after its dedicated commit is finalized and before later work can
contaminate the evidence. Provide only the single assigned work order, its exact
full commit hash, and this validation template. For a document-only work order,
provide only the assigned documents needed to compare its edits with the work
order; do not inspect application source, search for semantic mismatches with
code, or block because dependent implementation is outstanding. For an
implementation work order, provide only relevant desired-state document
references or excerpts so the validator can compare the assigned code outcome
with its target. Do not provide a broad branch diff or moving `HEAD` as the
comparison basis. The orchestrator owns ordering, dependency preparation, and
separate policy, test, or human gates.

Review the exact commit with:

```sh
git show --format=fuller --stat --patch --find-renames <hash>
```

An exact-hash equivalent is acceptable. Confirm the object exists and is reachable
from a ref (for example, `git branch --all --contains <hash>` returns a branch),
and inspect its parents (`git show -s --format=%P <hash>`): it must have exactly
one parent. Root commits, merge commits, missing/unreachable hashes, and commits
that include unrelated changes or multiple work orders are not acceptable default
WO commits. Compare the actual commit patch with the assigned work order and
record the hash reviewed in the validation artifact. Do not substitute the latest
branch commit for the supplied hash.

Git availability is determined and recorded at initiative start. If Git was
unavailable for commit and hash review then, the orchestrator records design
review and work-order validation as `skipped: Git unavailable`, states that no
commit hash exists, and may supply available implementation notes/diff without
claiming independent review. This exception does not skip requirements reviews
or unrelated required safety gates, and does not override a direct user
instruction to retain a review/gate; continuation is subject to applicable
external rules and project policy. A transient Git command or commit failure
after Git was established as usable is **blocked**, not a retroactive skip. An
explicit no-op has no fabricated commit: the orchestrator records `no-change`
and decides whether the validator should assess the absence against the work
order or record the no-change validation policy.

## Scope and verdict

Compare the exact supplied commit patch with the assigned work order's explicit
outcome and scope. Start from a positive alignment check: report whether the
changes deliver that outcome and are meaningfully consistent with the work order.
For document-only work, assess only the assigned document edits and the exact
work-order commit; the absence of dependent code is expected and is not a
blocker. For implementation work, use relevant supplied requirements/blueprints
as desired state, even if legacy code outside the assigned scope lags. Do not
audit unrelated code, investigate semantic consistency beyond the assigned
comparison, or penalize a declared document-first gap. Consult additional
documentation only when necessary to understand an explicit outcome.

Report **pass** when the dedicated commit meaningfully aligns with the assigned
work order. Report **fail** only for a significant, evidence-backed mismatch or
an unmet explicit work-order outcome. Report **blocked** when the hash/commit is
missing, unreachable, not a dedicated single-parent WO commit, or the evidence is
insufficient for meaningful comparison. Minor optional issues need not derail
alignment; do not fail for preferences or speculative concerns.

Return `pass`, `fail`, or `blocked` in the validation template and send it to the
orchestrator. Only the orchestrator may record `skipped: Git unavailable`, and
only in the defined no-Git case. In the two-to-three-sentence summary, state the
work order and exact hash reviewed; cite direct evidence for fail or name the
missing evidence for blocked. Do not spawn agents, change the implementation, or
perform follow-up review. Applicable project-policy, test, and human gates remain
separately orchestrator-owned.

## Artifact and transition

Create `validation-[work-order-id].md` in the initiative's `validations/`
directory using
[`../initiatives/YYYY-MM-DD-initiative-template/validations/validation-template.md`](../initiatives/YYYY-MM-DD-initiative-template/validations/validation-template.md)
as the concise artifact. The orchestrator links it from the work order and
execution log, and records separate gates and session identifiers in the
execution log.

- **Pass:** the orchestrator sets the work order to `complete` after applicable
  separate gates are satisfied.
- **Fail:** the orchestrator addresses the significant mismatch or unmet
  explicit outcome, creating a collision-free fix work order such as
  `WO-001a-fix-01` or `WO-001-fix-01`. Name its validation artifact with the same
  ID, for example `validation-WO-001a-fix-01.md` or
  `validation-WO-001-fix-01.md`; increment the fix number as needed. Keep the
  original work order incomplete until its correction passes. Existing historic
  IDs remain valid.
- **Blocked:** the orchestrator records the missing evidence or attribution
  reason and stops until resolved. A usable-repository commit failure cannot be
  converted to skipped. Blocked is not fail.

- **Skipped:** the orchestrator may record `skipped: Git unavailable` only when
  Git was unavailable for commit and hash review at initiative start. No hash
  exists; this is not pass or approval. Requirements review is unaffected.

The default cadence is fresh validation per work order in dependency order,
before later work contaminates its evidence. A user-authorized alternative still
requires attributable per-work-order evidence. The orchestrator owns dispatch,
state transitions, predecessor verification, and separate policy or human gates.

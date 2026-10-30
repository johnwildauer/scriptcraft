# Closeout

## Role

Closeout is an orchestrator-executed wrap-up procedure, not a review. It may run
in the orchestrator's own session; the dispatch invariants in
[`00-overview.md`](00-overview.md) bar the orchestrator from work orders and
reviews, and closeout is neither. Verify the preconditions from recorded
evidence rather than re-reviewing the work, then execute the closure actions in
order. No review artifact is created.

Closeout is orchestrator-owned, but its git actions are not necessarily
orchestrator-executed: the commit and merge require an execution-capable
session. When the orchestrator session cannot run git commands, it dispatches
those actions to an execution-capable coding session or supplies the exact
commands to the user, then records the resulting evidence (commit hash, merge
result) in the final execution-log entry. This is a capability routing rule,
not a fresh-context requirement: the operations are mechanical, and
independence is guaranteed upstream by validation. Precondition verification,
follow-up recording, the final log entry, and the status flip are
documentation actions and need no such capability.

## Preconditions

Verify all of the following from recorded evidence before closing:

- every work order is `complete` or explicitly tabled;
- all validations, gates, and approvals have recorded evidence; `skipped: Git
  unavailable` design-review/validation records are acceptable only when Git was
  unavailable for commit and hash review at initiative start and applicable
  external rules/project policy allow continuation. A skipped check is never a
  pass or approval, and requirements review is not covered by this exception;
- session separation held per the dispatch invariants in
  [`00-overview.md`](00-overview.md), or each exception was explicitly approved
  by the user and logged with reduced-confidence evidence; and
- intended initiative changes and any remaining worktree changes have been
  attributed; there are no unresolved conflicts or incomplete rollback or
  migration actions. Unrelated or pre-existing user changes are not accidental
  initiative files and must be preserved, not swept into closeout.

If a precondition is unmet, stay in `closeout` with status `blocked`, or return
to the phase owning the missing evidence. Record the unmet precondition, its
owner, and the next action.

## Closure actions

Execute in order:

1. Record deferred work as follow-up items with rationale and discoverable
   ownership (work order, initiative, or future initiative).
2. Append the final execution-log entry, recording the closeout session/agent
   identifier and the evidence for each precondition.
3. Keep dedicated work-order commits intact; do not fold, squash, amend, or
   replace them with an initiative or closeout commit. If authorized initiative-
   level changes remain outside those commits, commit only those separately and
   safely. Do not stage or commit unrelated, pre-existing, or untracked user work
   merely to make the tree clean. Prefer isolated worktrees; otherwise verify
   status/diff, stage a whole path only when all its changes are authorized, use
   scoped hunks for mixed files, and inspect the cached diff.
   Never use `git add -A` blindly. If changes cannot be isolated from existing
   edits, stop as `blocked`. A usable-repository commit failure blocks closeout;
   reduced-capability mode with Git unavailable at initiative start records the
   fallback evidence and explicit no-hash notice instead of claiming commit or
   review evidence. This action requires an execution-capable session — dispatch
   it to a coding agent with terminal access or hand exact, scoped commands to
   the user when the orchestrator cannot run git.
4. Merge the initiative branch into the recorded base branch — the default —
   unless the user or host repository policy instructs otherwise (for example, a
   pull-request-based flow). Verify the branch contains only changes authorized
   for this initiative before merging; preserve and do not integrate unrelated
   user work. Preserve the individual work-order commits through integration;
   do not replace them with a merge or aggregate commit. No-branch and
   reduced-capability modes skip this action with the recorded notice. Like the
   commit, this action requires an execution-capable session; record the merge
   result as evidence.
5. Set the initiative status to `closed`. Closing is a **status flip**, not a
   phase advance.

## Outcome

The final execution-log entry plus the status flip are the closeout record; no
separate closeout artifact exists. Do not delete completed initiatives by
default. Keep them in place with status metadata so future agents can trace
decisions and prior work. For partial or canceled initiatives, record what was
completed, tabled, or made obsolete. If a documentation-first change is no longer
the intended target, arrange an authorized documentation correction; do not
silently revert or delete durable requirements/blueprints, and do not leave them
contradictory with a changed target.

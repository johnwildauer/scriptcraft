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
- all validations, gates, and approvals have recorded evidence;
- session separation held per the dispatch invariants in
  [`00-overview.md`](00-overview.md), or each exception was explicitly approved
  by the user and logged with reduced-confidence evidence; and
- the final state contains no accidental files, unresolved conflicts,
  untracked changes, or incomplete rollback or migration work.

If a precondition is unmet, stay in `closeout` with status `blocked`, or return
to the phase owning the missing evidence. Record the unmet precondition, its
owner, and the next action.

## Closure actions

Execute in order:

1. Record deferred work as follow-up items with rationale and discoverable
   ownership (work order, initiative, or future initiative).
2. Append the final execution-log entry, recording the closeout session/agent
   identifier and the evidence for each precondition.
3. Ensure all changes are committed to Git. This is the default when Git is
   available; reduced-capability mode instead records the fallback evidence and
   the explicit notice that branch-level review and commit evidence did not
   occur. This action requires an execution-capable session — dispatch it to a
   coding agent with terminal access or hand the exact commands to the user
   when the orchestrator cannot run git.
4. Merge the initiative branch into the recorded base branch — the default —
   unless the user or host repository policy instructs otherwise (for example, a
   pull-request-based flow). No-branch and reduced-capability modes skip this
   action with the recorded notice. Like the commit, this action requires an
   execution-capable session; record the merge result as evidence.
5. Set the initiative status to `closed`. Closing is a **status flip**, not a
   phase advance.

## Outcome

The final execution-log entry plus the status flip are the closeout record; no
separate closeout artifact exists. Do not delete completed initiatives by
default. Keep them in place with status metadata so future agents can trace
decisions and prior work.

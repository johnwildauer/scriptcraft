# Execute a Work Order

Implement the assigned outcome within its scope, using relevant references and
the orchestrator's handoff. The orchestrator manages progress and ordering in
context and treats initiative documents as current. Raise genuinely missing
information, conflicts, or relevant mismatches to the orchestrator rather than
expanding the assignment.

Follow applicable user instructions and external rules. Make the requested
document or implementation changes and run useful checks for the task. Report
the outcome, changed files, check results, and any genuine issue to the
orchestrator.

When Git is available and changes can be safely attributed, commit this work
order's changes before validator dispatch. Preserve unrelated edits and exclude
them from the work-order commit. Report the implementation commit/diff to the
orchestrator so the validator can inspect that evidence before its separate finding
commit.

If Git is unavailable, there is no attributable change, or safe isolation is not
possible, report that plainly. The orchestrator still dispatches exactly one
validator for the completed work order; no one may claim alignment without
evidence or manufacture an implementation or finding commit.

For every completed discrete document or implementation work order, the
orchestrator dispatches exactly one fresh coding validator using the same prompt
in [`05-validation.md`](05-validation.md).

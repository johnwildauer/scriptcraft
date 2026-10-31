# Execute a Work Order

Implement the assigned outcome within its scope, using relevant references and
the orchestrator's handoff. The orchestrator manages progress and ordering in
context and treats initiative documents as current. Raise genuinely missing
information, conflicts, or relevant mismatches to the orchestrator rather than
expanding the assignment.

Follow applicable user instructions and external rules. Make the requested
document or implementation changes. Run useful checks only where permitted by
the authoritative project policy; report the outcome, changed files, any checks
actually performed by an authorized party, and genuine issues to the orchestrator.

When Git is available and changes can be safely attributed, the default is to
commit this work order's changes before validator dispatch. Follow the authoritative
project policy if it disables agent validation or assigns review/validation to the
user. Preserve unrelated edits and exclude them from any work-order commit. Report
the implementation commit/diff to the orchestrator where applicable.

If Git is unavailable, there is no attributable change, or safe isolation is not
possible, report that plainly. Generic Forge guidance calls for one validator
even then, subject to the authoritative project policy; no one may claim
alignment without evidence or manufacture an implementation or finding commit.

By default, for every completed discrete document or implementation work order,
the orchestrator dispatches exactly one fresh coding validator using the same
prompt in [`05-validation.md`](05-validation.md), unless the authoritative project
policy overrides that default.

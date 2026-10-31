# Closeout

Closeout is a brief wrap-up of the requested work, guided by the initiative plan
and work orders in [`02-initiative.md`](02-initiative.md). Follow the user's
directions and applicable project or external requirements; no extra approval gate
is required by this workflow.

## Record material outcomes

Summarize the actual outcome and record material decisions, unresolved issues, or
useful follow-ups in the initiative documents when helpful. Keep the record
proportionate: do not add routine activity logs or require a separate closeout
artifact. For partial or canceled work, distinguish what was completed from what
remains or was dropped. Do not silently undo durable changes or describe
unperformed actions as complete.

For each completed work order, the orchestrator dispatches exactly one fresh coding
validator using [`05-validation.md`](05-validation.md). The implementation commit
precedes validation when Git is available; the finding is committed separately
when Git permits. If evidence is missing, record the limitation rather than
claiming validation. Do not add gates beyond applicable user, project, or external
requirements.

## Git and integration

Before Git actions, inspect the working-tree status and relevant diffs, identify
which changes belong to the requested work, and preserve unrelated or pre-existing
user changes. Stage only intended changes and inspect the staged diff before
committing; never stage everything merely to clean the tree. If changes cannot be
safely isolated, stop and report the issue rather than risking unrelated work.

Commit and integrate changes only as directed by the user and the repository's
workflow. Before merging, identify the intended base and integration path, and
confirm the changes to be integrated are authorized. Use the project's required
pull-request or review process where applicable; do not perform an automatic merge
or assume a merge is required. If the path is unclear or a conflict arises, report
it to the orchestrator for direction. Record material commit, merge, or follow-up
information when useful, without imposing a rigid evidence or handoff format.

# Execute a Work Order

## Role and scope

Act as the implementer for one assigned work order. By default, the orchestrator
dispatches a fresh implementer session with only the assigned work order, its
    relevant target requirements and blueprint references or excerpts, necessary handoff
information, and relevant checks or commands. Use other documents only when
needed to complete the task. You may inspect the specific code you change. Do not
load previous work orders or execution logs, or independently re-verify
predecessors: the orchestrator owns ordering and predecessor verification,
prepares dependencies, and passes the information needed for this assignment.
Do not expand scope without the required work-order update and approval.

## Procedure

1. Use the orchestrator's handoff and prepared assignment. If required
   information or a dependency is visibly missing, stop before relying on
   assumptions; report the specific missing item and needed action so the
   orchestrator can supply it or mark the work blocked.
2. Follow the work-order kind. For a document-only work order, make only the
   assigned durable-document edits; do not implement or assess dependent code.
   For implementation work, treat supplied requirements and blueprints as the
   target desired state even when legacy/current code lags. Inspect relevant code
   and implement the smallest coherent change within scope. If normal assigned
   work reveals a meaningful, relevant mismatch outside the assigned scope,
   report it to the orchestrator for initiative-log recording and user
   communication; do not audit broadly or expand scope. Do not block unless the
   mismatch directly prevents the assigned outcome or safe implementation.
3. Follow the work order's durable-document impact classification, applying
   authorized updates and recording proportionate evidence for documents marked
   `verify unchanged` or `not applicable`.
4. Run the relevant checks and commands supplied for the task; report results,
   including failures or unavailable checks.
5. If the orchestrator confirms Git is usable for this repository, close out with
   one dedicated ordinary commit containing only this work order's changes. Prefer
   an isolated worktree. In a shared worktree, inspect `git status` and the full
   diff. Stage a whole path only when all its changes are assigned here; for a
   mixed file, stage only demonstrably scoped hunks. Inspect the cached diff and
   verify no unrelated or pre-existing content is included. Never use `git add -A`
   blindly, and never stage/commit unrelated user changes. If a target file has
   pre-existing edits, distinguish the assigned patch demonstrably or stop as
   blocked; do not rewrite, discard, or stash user work. Parallel work orders
   require separately attributable worktrees or serialized commits. Do not use
   root or merge commits; the work-order commit must have exactly one parent. A
   commit failure when Git was usable at initiative start blocks normal closeout.
   For a document-only work order, make the commit message explicitly state
   that the documentation precedes implementation and name the dependent
   implementation work-order IDs.
   If the work order made no changes, do not create an empty commit: report
   `no-change` for the orchestrator to decide the validation disposition.
6. Report completion with a changed-file list (including created, deleted, and
   renamed files), checks and results, concise outcome evidence, a compact
   handoff, and the exact full commit hash. The commit hash must be the dedicated
   commit for this work order. When Git was unavailable at initiative start,
   report that no hash exists and provide available notes/diff; do not claim
   independent review. The orchestrator records dispatch/completion identifiers,
   hash or explicit skip/no-change, and validation readiness. Mark the work
    `in-review` only after assigned work and evidence are ready; for
    implementation work, this means code is implemented and awaiting validation.

When Git was usable at initiative start, the orchestrator passes this work order
and its exact dedicated commit hash to a fresh validator per
[`05-validation.md`](05-validation.md). Validation, not the implementer,
determines completion; do not mark the work order `complete` yourself. If Git was
unavailable at initiative start, the orchestrator records validation as
`skipped: Git unavailable` without dispatching a validator or representing the
skip as a pass. Requirements reviews and other applicable safety gates remain
separate. A later Git/commit failure in a repository recorded as usable is
blocked, not skipped.

## Blocking conditions

Stop and report the exact issue when required input or a dependency is visibly
missing, instructions materially conflict, a required task check cannot run, a
manual gate is unavailable, or safe implementation would exceed scope. The
orchestrator records blocked status and the next action when appropriate; do not
silently infer missing prerequisite work.

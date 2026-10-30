# Execution Log

> This log is primarily for work-order execution. Do not use it as a transcript
> of every exploratory planning exchange; keep early findings and discussion in
> the initiative plan's exploration scratchpad. Append entries chronologically.
> Never rewrite prior entries to conceal failures, retries, or superseded
> decisions.

## Initiative

- **Initiative:** `TODO`
- **Initiative mode:** `standard` | `rapid` — selected: `standard`
- **Current phase (optional, descriptive):** `draft` | `plan` | `review` | `execute` | `validate` | `closeout` — `TODO`
- **Current status:** `active` | `blocked` | `closed` — selected: `active`
- **Orchestrator:** `TODO`
- **Orchestrator session/agent ID:** `TODO`

## Desired-state documentation handoff (if applicable)

Record a declared document-first gap once: target document(s), documentation
work-order ID(s), dependent implementation work-order ID(s), and the expected
lag until code catches up. Communicate this expected lag once to the user in
plain language. Do not copy initiative status into requirements or blueprints. Record
an unexpected, relevant mismatch found during assigned work as an execution
event for the orchestrator and user; do not turn it into a broad audit.

## Execution log entries

### YYYY-MM-DD HH:MM — Execution event

- **Actor/role:** `TODO`
- **Session/agent ID:** `TODO` <!-- record for every dispatch and completion event -->
- **Material state change (optional):** `TODO: from → to; omit redundant phase/status flips`
- **Work order/review:** `TODO`
- **Action:** `TODO`
- **Evidence:** `TODO: files, checks, exact WO commit hash, or explicit skipped/no-change record; include validation/review artifact link`
- **Result:** `TODO`
- **Next action/owner:** `TODO`

No execution entries are required while the initiative is still in `draft`. Start
the log when a work order is approved or implementation begins, and record the
branch decision and relevant execution evidence then. During the default
per-work-order loop, record material dispatch and completion events — implementer
dispatch, readiness for validation, validation dispatch, and result — with the
session/agent identifier for each. Use status labels only when they convey useful
state; do not maintain a redundant phase-transition ledger.

For each work order, record the exact full dedicated commit hash reported by the
implementer and the validation artifact/outcome. If Git was unavailable for
commit and hash review at initiative start, record design-review and validation
as `skipped: Git unavailable`, state that no commit hash exists, and link any
available implementation notes/diff; never label skipped checks passed. Record
`no-change` when a WO has no changes and the orchestrator's explicit validation
disposition. A usable-repository commit/command failure is blocked, not skipped.
Implementers preserve unrelated work and stage only scoped changes; see
[`../../process/04-work-order.md`](../../process/04-work-order.md) and
[`../../process/05-validation.md`](../../process/05-validation.md).

## Session separation exceptions

Record each exception to the dispatch invariants in
[`../../process/00-overview.md`](../../process/00-overview.md): the combined
work orders or reviews, the user's explicit authorization, and the
reduced-confidence evidence for the affected work.

## Unresolved and deferred work

- `None or TODO: item, rationale, owner, and suggested future initiative`

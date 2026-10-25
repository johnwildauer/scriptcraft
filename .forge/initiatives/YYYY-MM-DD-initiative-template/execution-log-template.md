# Execution Log

> This log is primarily for work-order execution. Do not use it as a transcript
> of every exploratory planning exchange; keep early findings and discussion in
> the initiative plan's exploration scratchpad. Append entries chronologically.
> Never rewrite prior entries to conceal failures, retries, or superseded
> decisions.

## Initiative

- **Initiative:** `TODO`
- **Initiative mode:** `standard` | `rapid` — selected: `standard`
- **Current phase:** `draft` | `plan` | `review` | `execute` | `validate` | `closeout` — selected: `draft`
- **Current status:** `active` | `blocked` | `closed` — selected: `active`
- **Orchestrator:** `TODO`
- **Orchestrator session/agent ID:** `TODO`

## Execution log entries

### YYYY-MM-DD HH:MM — Execution event

- **Actor/role:** `TODO`
- **Session/agent ID:** `TODO` <!-- record for every dispatch and completion event -->
- **State transition:** `TODO: from → to`
- **Work order/review:** `TODO`
- **Action:** `TODO`
- **Evidence:** `TODO: files, commands, commits, or review links`
- **Result:** `TODO`
- **Next action/owner:** `TODO`

No execution entries are required while the initiative is still in `draft`. Start
the log when a work order is approved or implementation begins, and record the
branch decision and relevant execution evidence then. During the per-work-order
loop, record each dispatch and completion event — the implementer dispatch, the
work order reaching `in-review`, the validation dispatch, and the validation
result — with the session/agent identifier for each.

## Session separation exceptions

Record each exception to the dispatch invariants in
[`../../process/00-overview.md`](../../process/00-overview.md): the combined
work orders or reviews, the user's explicit authorization, and the
reduced-confidence evidence for the affected work.

## Unresolved and deferred work

- `None or TODO: item, rationale, owner, and suggested future initiative`

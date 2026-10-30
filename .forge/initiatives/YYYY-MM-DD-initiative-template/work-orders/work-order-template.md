# Work Order: WO-001 — Short Title

<!-- One distinct work order should fit one agent and one context window, be quickly completed, and deliver one independently understandable and verifiable outcome. IDs name individual work orders. For multiple task groups, use WO-001a, WO-001b, WO-001c, then WO-002a, WO-002b; for a single throughline, use WO-001, WO-002, etc. Preserve older IDs as history. -->

## Metadata

- **Status:** `draft`
- **Work-order kind:** `document-only` | `implementation` | `mixed` — selected: `TODO`
- **Assignee:** `TODO`
- **Order/dependencies:** `TODO` <!-- orchestration owns ordering and prepares dependencies before dispatch -->
- **Initiative:** [`../initiative-slug-v1.md`](../initiative-slug-v1.md) <!-- replace initiative-slug when copying -->
- **Generated filename:** `WO-001-short-title.md` <!-- replace with this work order's generated filename -->
- **Owner/orchestrator:** `TODO`
- **Validation:** [`../validations/validation-WO-001.md`](../validations/validation-WO-001.md) <!-- replace WO-001 with this work order's ID when copying -->
- **Fixes:** `none` | `TODO` <!-- fixes use collision-free IDs such as WO-001a-fix-01 or WO-001-fix-01, incrementing for further fixes; link original work order and failing validation; validation ID matches fix ID -->
- **Implementer session/agent ID:** `TODO` <!-- separate fresh implementer session per the dispatch invariants in [`../../../process/00-overview.md`](../../../process/00-overview.md) -->
- **Dedicated work-order commit:** `TODO: exact full hash | skipped: Git unavailable | no-change` <!-- one ordinary single-parent commit scoped only to this work order when Git is usable -->
- **Documentation-first relationship:** `none` | `docs precede implementation` — document WO ID: `TODO`; dependent implementation WO IDs: `TODO`
- **Commit description (document-only):** `docs-first (pre-implementation): [dependent WO IDs] — [document outcome]` <!-- explicitly identify the docs-first relationship and dependent work orders -->
- **Session exception:** `none` | `approved and logged` — evidence: `TODO`

## Summary

`TODO: State one independently understandable, verifiable outcome in 2–3 concise sentences.`

## In scope

- `TODO: responsibilities owned by this work order`

## Out of scope

- `TODO: adjacent or deferred work`

## Requirements

Identify applicable requirements and acceptance criteria; link the source and include only the relevant desired-state excerpt needed by the assigned agent. Target documents may intentionally predate implementation code.

- `TODO: REQ/AC IDs, source link, and relevant excerpt`

## Blueprints

- `TODO: blueprint link and relevant excerpt` — `TODO: concise implementation context`

## Implementation plan

1. `TODO: file/system and key change`
2. `TODO: file/system and key change`

Keep the plan focused on general code location and necessary changes; avoid incidental implementation detail.

## Files and systems

- **Create:** `TODO`
- **Update:** `TODO`
- **Avoid changing:** `TODO`

## Validation and E2E acceptance tests

- **Automated checks:** `TODO: relevant test/check commands only`
- **Manual/external checks:** `TODO: relevant checks or explicitly none`
- **Acceptance test:** Given `TODO`, when `TODO`, then `TODO`.

## Durable-document impact

Complete proportionately when the work plausibly affects durable product or technical intent. For impacts below those documents' abstraction level, use `not applicable` with a brief rationale; do not perform rote synchronization. When this is a document-only work order, state that the durable target is being updated before implementation and link dependent implementation work-order IDs.

- **Requirements impact:** `update` | `verify unchanged` | `not applicable` — selected: `TODO`
- **Blueprint impact:** `update` | `verify unchanged` | `not applicable` — selected: `TODO`
- **Affected documents/IDs:** `TODO`
- **Rationale and evidence:** `TODO` <!-- concise rationale and relevant evidence; no exhaustive search required -->
- **Authorized documentation changes:** `TODO`

## Completion checklist

- [ ] Assigned work is complete within scope.
- [ ] Tests/checks and required gates have evidence.
- [ ] Durable-document impact classification is accurate against the diff, and authorized documentation updates are applied.
- [ ] Execution log is appended, including dispatch/completion session IDs and exact WO commit hash, or the defined Git-unavailable/no-change record.
- [ ] Implementation evidence and readiness for validation are recorded (commonly `in-review`); independent validation receives this WO and exact hash, or records the explicit no-Git/no-change disposition.

## Handoff

`TODO: concise dependencies or context the orchestrator must provide; implementers assume ordering and prerequisite verification are prepared. Include only necessary commands and information for this work order.`

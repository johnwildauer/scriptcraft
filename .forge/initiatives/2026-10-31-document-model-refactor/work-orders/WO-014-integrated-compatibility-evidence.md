# Work Order: WO-014 — Integrated FDX compatibility evidence

- **Status:** `not started` — depends on WO-012 and WO-013; no implementation or validation is claimed.

## Outcome
Validate the complete FDX migration against the directed behavior and record bounded compatibility evidence.

## Scope
Exercise the integrated model, edits, codec, versions, cast, desktop persistence, failure safety and JSON sunset. Prepare permission-safe real-app verification instructions and record actual results only when an identifiable Final Draft installation is available.

## Relevant references
- [Verification implications](../document-model-refactor-v1.md#verification-implications-for-future-work-orders)
- [Project checks](../../../project-manifest.md#commands)

## Implementation
Create targeted end-to-end regression fixtures and an evidence matrix covering all 17 roles; title structure; unknown inline and section data; nested scene attachments; board/item/image links and silent cross-domain orphaning; Version 6 no-op and edits; older and newer warnings; cast synchronization and failure rollback; and security limits. Use synthetic images and no private paths. Document manual open/inspect/save/reopen actions, app/OS versions and fixture provenance. No real-app availability is presumed.

## Acceptance criteria
- Automated and desktop scenarios cover the full primary contract and report precise failures or blockers; all applicable format, lint, test and build checks are recorded.
- Evidence clearly separates synthetic semantic checks from actual external-app results, notes untested versions and unknown foreign links, and makes no blanket interoperability claim. If Final Draft is unavailable, record manual verification as outstanding and do not claim it passed.
- No Fountain implementation is introduced; unresolved blocker cases and doc/code gaps are handed to the orchestrator.

## Useful checks
- Run project policy commands, targeted integration tests and applicable manual desktop checks; record exact outcomes and environment. Verify no sensitive payload or private path in artifacts.

## Dependency and handoff
Depends on WO-012 and WO-013 and their validated prerequisites. One dedicated commit and fresh validation; provide evidence and unresolved limitations for initiative closeout.

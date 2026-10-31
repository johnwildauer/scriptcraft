# Work Order: WO-014 — Integrated FDX compatibility evidence

- **Status:** `user-owned validation handoff prepared; evidence and acceptance pending` — depends on WO-012 and WO-013; this documentation does not validate either implementation or complete WO-014's original evidence requirement.

## Outcome
Provide the user with a concrete end-to-end validation plan for the directed FDX behavior and record bounded compatibility evidence only when the user performs and supplies it. The user's repository-wide policy makes all review and validation user-owned; agents must not run checks, perform manual verification, conduct reviews, or claim results.

## Scope
Prepare a concrete user-owned handoff for integrated model/edit/codec/version/cast/persistence/failure/JSON-sunset scenarios, reproducible commands, safe fixture provenance, real-app compatibility steps and result recording. No tests or fixtures are added by this work; users should use synthetic, permission-safe fixtures for validation. Record actual results only when the user supplies them and an identifiable Final Draft installation is available.

## Relevant references
- [Verification implications](../document-model-refactor-v1.md#verification-implications-for-future-work-orders)
- [Project checks](../../../project-manifest.md#commands)
- [User-owned validation handoff](WO-014-user-validation-handoff.md)

## Implementation
No agent-side tests, fixtures, reviews or checks are authorized. The user handoff specifies scenarios and commands for the user to execute, and safe fixture/provenance rules. No real-app availability is presumed. Preserve historical evidence separately from new user evidence; do not infer a result from implementation commits or authored tests.

## Acceptance criteria
- The user has performed and recorded the applicable automated, desktop and manual scenarios and applicable checks. **Pending:** an agent cannot satisfy this result criterion under repository policy; preparing the handoff does not complete it.
- User evidence clearly separates synthetic semantic results from actual external-app results, notes untested versions and unknown foreign links, and makes no blanket interoperability claim. If Final Draft is unavailable, manual verification stays outstanding and is not reported as passed.
- No Fountain implementation is introduced; unresolved blocker cases and doc/code gaps are handed to the orchestrator. This documentation-only task introduced no Fountain work.

## Useful checks
- User-owned commands, scenario sequence, real-app limitations and evidence template are in the [validation handoff](WO-014-user-validation-handoff.md). Agents must not run them or perform any other validation.

## Dependency and handoff
Depends on WO-012 and WO-013, and transitively WO-012a–g and earlier implementation prerequisites. The implementation records for WO-012 (`521774d2719cf68c9309411ecb1d40248d822474`) and WO-013 (`2c9bc8b72b6f7a386d1c190d3527e81bfb3280c5`) are not validation evidence; neither has been validated. User review and all validation remain pending. Preserve the actual lack of independent validation for WO-002–011 and do not treat this handoff or historical checks as acceptance. No agent checks, reviews, validator dispatch, or real-app experimentation are permitted under current policy.

## User-owned handoff preparation record — 2026-10-31

- The linked handoff is a plan, not a validation artifact. It contains scenario IDs, recommended user-run commands, synthetic fixture guidance, real-app steps, limitations and a results template.
- **Not performed by the agent:** tests, builds, lint, formatting checks, diff checks, manual/UI verification, review, external-app experimentation or any other validation. No scenario is claimed to pass.
- **Still pending:** user review and validation of WO-012 and WO-013; user execution and recording of WO-014 scenarios and checks; actual Final Draft evidence if available; and final WO-014 acceptance. Any acceptance evidence that an agent cannot supply remains explicitly pending user validation.

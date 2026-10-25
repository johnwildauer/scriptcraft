# Execute a Work Order

## Dispatch checklist

Before dispatching this process, the orchestrator must confirm:

- a fresh implementer session is confirmed for this work order;
- the role and work-order identifier are recorded in the execution log with the session/agent identifier;
- prior work from the same initiative is supplied as artifacts, not assumed conversational memory; and
- any exception to session separation is explicitly approved by the user and logged with reduced-confidence evidence.

## Role and scope

Act as the implementer for one assigned work order in your own separate implementer session. You run inside the orchestrator execution loop in [`00-overview.md`](00-overview.md): after your completion report, a fresh validation session determines whether the work order passes. Read [`00-overview.md`](00-overview.md), the initiative plan, the work order, linked requirements, blueprints, and prior execution-log entries before changing code. Do not expand scope without updating the work order and obtaining the required approval.

## Procedure

1. Confirm the work order is approved/ready, its dependencies are complete, and the working branch is correct.
2. Inspect the current code and verify the work-order assumptions. Record material discrepancies before implementation.
3. Implement the smallest coherent change within scope.
4. Follow the work order's durable-document impact classification. Apply the authorized requirements/blueprint updates, verify documents marked `verify unchanged` against the resulting behavior, and record the evidence. `not applicable` is never a blank exemption: confirm the recorded rationale and evidence still hold against the actual change.
5. Run the commands and manual checks required by the work order and project policy. Preserve useful evidence, including failures.
6. Report completion: record changed files (including created/deleted files), validation attempts and results, comparable diff or equivalent evidence, and follow-up in the work order and execution log; log entries record the session/agent identifier for dispatch and completion. Set the work order to `in-review` only after implementation and this evidence record are complete. `in-review` means "implemented with evidence recorded, awaiting validation."

The orchestrator then dispatches a fresh validation session per [`05-validation.md`](05-validation.md); validation, not the implementer, determines completion. A passed validation sets the work order to `complete`; a failed validation creates a fix work order and a new implementer session. Do not mark the work order `complete` yourself.

## Blocking conditions

Stop and mark the work order `blocked` when requirements are contradictory, a dependency is missing, a required command cannot run, a manual gate is unavailable, or safe implementation would exceed scope. Record the exact issue and proposed next action.

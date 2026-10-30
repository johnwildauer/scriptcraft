# Software Forge Process Overview

## Purpose

This directory contains default guidance agents use to initialize a repository, plan an initiative, execute work orders, validate changes, and close out the resulting branch. Use relevant durable requirements and blueprints when they inform the task; do not require unrelated documents or a blanket process-document read for a focused assignment. Leave an auditable record of material decisions and evidence.

The installed workflow begins at [`../_index.md`](../_index.md). This file defines shared vocabulary and default guidance; lifecycle labels and defaults are descriptive, not rigid prerequisites. The user's actual instructions lead. At explicit user direction, project policy may be adapted and the change recorded in the appropriate durable project document. Applicable external rules and checks must not be silently discarded or weakened; communicate actual uncertainty and blockers. An orchestrator may dispatch an architect or coding agent to adapt host-project process documents when the user requests it.

## Artifact boundaries

- [`../requirements/`](../requirements/) is the durable product definition: what the system should do, for whom, and why.
- [`../blueprints/`](../blueprints/) is the durable technical definition: how the system is organized and how behavior is realized.
- [`../initiatives/`](../initiatives/) contains bounded work. An initiative may update requirements or blueprints only when its work orders explicitly authorize those updates.

Requirements and blueprints express the product/system's durable **desired state**, not the progress or history of a particular initiative or work order. A requirement or blueprint may describe a target that current code does not yet implement; approval of that target is not an implementation claim. Keep initiative status, temporary gaps, execution history, and cancellation outcomes in the initiative and its execution log, not in durable requirements or blueprints.

When an upcoming change has a meaningful durable-document impact, a narrow documentation-first work order before dependent implementation is a useful normal suggestion. It is never compulsory: omit it when there is no durable impact or the change is below the documents' abstraction level. Link dependent implementation work orders. Record a declared document/code gap once in the initiative execution handoff and communicate it once to the user in plain language; this expected lag is not a validation blocker. Implementation agents treat supplied requirements/blueprints as target instructions even when existing code lags. If normal assigned work reveals a meaningful, relevant, unexpected mismatch elsewhere, report it to the orchestrator for initiative-log recording and user communication; do not audit broadly or expand scope. Do not silently revert durable docs if work is canceled or becomes obsolete: record the outcome in the initiative and arrange an authorized documentation correction if the intended target changed.

Temporary planning notes, agent conversations, and scratchpads are inputs to the workflow, not durable sources of truth. When a decision becomes relevant to future work, record it in the appropriate durable document or initiative log.

## Shared lifecycle

### Initialization

Use [`01-init-new.md`](01-init-new.md) for a new or empty repository and [`01-init-existing.md`](01-init-existing.md) for a repository with existing code or artifacts. Both paths use [`../project-manifest.md`](../project-manifest.md) as the default durable record and establish a product overview, requirements/blueprints baseline, project policy, applicable independent review findings, and a recommended first initiative. When the manifest is missing or not `complete`, the first agent normally guides the user through initialization; when it is `complete`, agents skip initialization and begin new work at [`02-initiative.md`](02-initiative.md), unless the user directs another permitted task.

### Initiative execution

The normal pattern is exploration, planning and work-order drafting, design review and approval, implementation, per-work-order validation, and closeout. Use [`02-initiative.md`](02-initiative.md) to turn a requested change into a scoped initiative and work orders. The user may direct a suitable shortcut or change of sequence; do not demand exact phase labels or redundant confirmation when asked to proceed directly from plans/features to drafting work orders. Drafting is not approval or execution. Keep default design review, approval, and independent per-work-order validation distinct from work-order authoring. Adapt and record project-policy changes only at explicit user direction, while retaining applicable external rules.

### Review routing

Route each review by the artifact and evidence it is authorized to assess.

| Review | Reviews | May run when | Must not assess | Required output |
|---|---|---|---|---|
| Requirements review | Product and feature intent, scope, users, and acceptance criteria | A requirements draft exists and is ready for independent review | Implementation completeness or unauthorized code | `requirements-review.md` in the initiative's `reviews/` directory, or the configured requirements-review location when no initiative exists yet |
| Design review | The initiative plan and listed work orders | The plan and work orders are ready for the default review, or the user requests review | Code or implementation obligations not authorized by the plan | `design-review.md` in the initiative's `reviews/` directory |
| Validation | One completed work order | The orchestrator has prepared the assigned work order and relevant per-WO evidence, unless a user-approved policy adaptation changes this default | Unimplemented design obligations or future scope | `validation-[work-order-id].md` in the initiative's `validations/` directory |

Requirements review is distinct from design review: it determines whether intent is clear, coherent, and testable before technical planning relies on it. Its outcome is `pass` only when no blocker remains; warnings require the configured owner/approver to accept them. A failed or blocked requirements review returns the requirements artifact to `draft` or `blocked` with a recorded next action and does not authorize design work.

When Git is usable for the repository at initiative start, each implementer creates one ordinary, non-merge commit for its assigned work order at closeout. The commit has exactly one parent, contains only that work order's changes, and its exact full hash is reported to the orchestrator. A root commit or merge commit is not a safe default work-order commit. The orchestrator passes that hash with the single assigned work order to the independent validator; validators inspect the passed hash, never moving `HEAD` or a broad branch diff. Initiative/branch and eventual closeout commits remain separate from work-order commits.

Prefer a separately attributable worktree per work order when this can be done without moving or disturbing user changes. On a shared worktree, inspect `git status` and the full diff before staging. Stage a whole path only when all its changes belong to this work order; for mixed files, stage only demonstrably scoped hunks. Inspect the cached diff and verify that neither pre-existing nor unrelated content is included. Never use `git add -A` blindly or commit unrelated/pre-existing user changes. If a target file already contains edits and the assigned patch cannot be demonstrated separately, isolate safely or stop as blocked. Parallel work orders require separately attributable worktrees or serialized commits; do not mix multiple work orders in a commit. A commit failure in a repository where Git was usable at initiative start blocks normal implementation closeout; it does not authorize a Git-unavailable bypass.

Determine Git usability for commit and hash review at initiative start and record it in the initiative and execution log. If Git is unavailable or the repository cannot support commits and hash review at that point, explicitly record design review and each work-order validation as `skipped: Git unavailable`, state that no commit hash exists, and provide available implementation notes/diff without claiming independent review. This exception does not override explicit user instructions or applicable policy requiring a review/gate; requirements reviews and unrelated safety gates still apply. A later transient Git/commit failure in a repository recorded as usable is blocked, not retroactively skipped. A no-op work order gets no fabricated empty commit: the orchestrator records `no-change` and explicitly decides whether the validator should assess the absence against the work order or record the no-change validation policy.

The orchestrator also prepares:

1. implementation is complete and readiness for validation is clear (commonly marked `in-review`);
2. the implementer recorded all changed files, including created/deleted files;
3. validation attempts and results, including unavailable or failed checks, are recorded; and
4. the exact full work-order commit hash is recorded and reachable, or the explicit no-Git/no-change disposition is recorded.

Status vocabulary is descriptive: do not block solely because an artifact lacks the literal `in-review` label if the work and evidence are ready. The validator assesses only the assigned work order against its dedicated commit. The orchestrator separately handles prior-WO ordering, dependency preparation, relevant project checks, and approvals. Incomplete optional evidence is a note or warning unless it prevents meaningful comparison; an unavailable required external or project-policy gate is a hard stop. Design preferences and other non-blocking deficiencies remain notes unless policy explicitly routes them to review.

Dispatch context stays narrow. A design reviewer receives the initiative plan, selected work orders, and review template. For a requirements review, provide the requirements draft and concise review assignment instead. An implementer receives its work order, only the relevant target requirements/blueprint references or excerpts, the orchestrator's necessary handoff, and commands needed for the task; the orchestrator prepares dependencies and owns ordering and predecessor verification. A validator receives exactly one work order, its exact full commit hash, and the validation template; include only documents needed for that work-order comparison. For a document-only work order, validate its exact commit and assigned document edits against the work order without inspecting application source or blocking on dependent code not yet implemented. For implementation work, compare the exact commit with the assigned outcome and relevant desired-state document excerpts; do not audit unrelated code or penalize declared expected lag. The validator runs `git show --format=fuller --stat --patch --find-renames <hash>` (or an exact-hash equivalent), confirms the commit exists and is reachable, has exactly one parent, and is scoped only to that work order, then records the reviewed hash. A root, merge, multi-work-order, missing, or unscoped commit is blocked or failed as appropriate; it is not replaced by inspecting the latest branch commit.

Review and validation artifacts record only the outcome, date, reviewed documents/evidence (including the exact commit hash for validation), and a 2–3 sentence plain-language summary. Outcomes are `pass`, `fail`, `blocked`, or `skipped: Git unavailable` only in the defined no-Git case. Other gate, approval, session, commit hash, and orchestration tracking belongs in the execution log. Link the artifact from the relevant initiative/work order and execution log where applicable.

## Status vocabulary

These statuses are useful descriptive conventions; use them where helpful, or follow user-directed project vocabulary. Project or external rules may impose additional requirements:

| Status | Meaning |
|---|---|
| `draft` | The artifact is being authored and is not ready for execution. |
| `in-review` | A reviewer or human is evaluating the artifact. |
| `approved` | Required approval for the current gate has passed. |
| `in-progress` | Work is actively being performed. |
| `blocked` | Safe progress requires a decision, fix, external action, or manual gate. |
| `complete` | The artifact's scoped work and required validation are complete. |

Do not imply completion without the evidence or gates that apply. Record material evidence and gate outcomes, but do not require a rote ledger of every phase or duplicate tracking already kept in the execution log. `closed` may describe terminal initiative status; work-order `in-review` conventionally means implemented with evidence and awaiting validation.

## Initiative lifecycle

The statuses above apply to artifacts as descriptive conventions. Initiatives may use **phases** to summarize current activity and **status** to summarize condition. These labels are not a mandatory state machine; use the vocabulary when it helps, and follow the user's requested path. Gates that remain applicable should be recorded with evidence, without redundant phase bookkeeping.

```text
Typical path: explore -> plan/draft work orders -> design review/approval -> implement <-> validate per work order -> closeout
```

- Common phase labels are `draft`, `plan`, `review`, `execute`, `validate`, and `closeout`; common statuses are `active`, `blocked`, and `closed`. They may be adapted to user direction. Record material blockers and their owner/next action, and retain evidence for approvals and decisions that still apply.
- The normal implementation cadence alternates implementation and independent validation per work order. The user may request another suitable cadence, subject to applicable project and external requirements.

### Phase definitions

| Phase | Entered when | Primary activity | Primary artifacts | Exits when | Process doc |
|---|---|---|---|---|---|
| Explore/plan | A change is requested | Explore, plan, and draft appropriately scoped work orders | Initiative plan and work orders | User asks to proceed; no exact phase label or redundant confirmation required | [`02-initiative.md`](02-initiative.md) |
| Design review | Work orders are drafted and ready | Default independent design review and human approval | `design-review.md` and approval evidence | Review and applicable approval pass before implementation by default | [`03-design-review.md`](03-design-review.md) |
| Implement | Approved work is ready, or the user directs a permitted path | Implement each assigned work order | Implementation evidence per work order | Work order has outcome evidence for validation | [`04-work-order.md`](04-work-order.md) |
| Validate | A work order is implemented | Default fresh independent validation per work order | `validations/validation-[work-order-id].md` | Work-order validation and applicable gates are complete; hard-stop findings are resolved | [`05-validation.md`](05-validation.md) |
| Closeout | Initiative work is complete | Record outcomes and follow-ups; perform applicable closeout actions | Follow-up and execution records | Applicable closure actions complete | [`06-closeout.md`](06-closeout.md) |

This mapping is guidance, not a required sequence of exact labels. Drafting work orders does not itself approve or authorize their execution.

### Gates

The normal gates remain clear, even when the user chooses a different appropriate path. Record material approvals and evidence; do not create redundant gate confirmations.

- **Work-order drafting:** proceed when the user requests it; do not require exact phase names or a second confirmation for a direct translation of their plans/features.
- **Design review -> implementation:** by default when Git is usable, independent design review passes and required human design approval is recorded. Only initiative design review may be recorded `skipped: Git unavailable` when Git was unusable at initiative start; this does not skip requirements or other required reviews. Drafting work orders is not execution approval.
- **Validation -> closeout:** by default, work orders are validated and any required project-policy approval is recorded.

### Orchestrator execution loop (default, all modes including rapid)

By default, for each work order in dependency order:

1. dispatch a fresh implementer session ([`04-work-order.md`](04-work-order.md));
2. receive the completion report — scoped changed files, checks, and the exact full dedicated commit hash — and record readiness for validation;
3. when Git was usable at initiative start, pass that one work order and hash to a fresh validation session ([`05-validation.md`](05-validation.md)); when the recorded no-Git exception applies, record validation as `skipped: Git unavailable` and do not dispatch a validator;
4. receive the validation result:
    - **pass** (including recorded warnings or notes) — set the work order to `complete`; proceed to the next work order;
    - **fail with a genuine hard-stop finding** — create a fix work order and loop; or
    - **blocked** — record the blocker, apply the `blocked` status, and stop; or
    - **skipped: Git unavailable** — record the absence of a commit hash and independent validation, and continue only when applicable policy and external rules permit.

The execution log records material dispatch/completion events and session/agent identifiers. Do not maintain a redundant per-phase ledger. With permitted parallelism, the orchestrator prepares dependencies and records enough ownership/evidence to keep work distinguishable.

**Fix work orders:** when a correction is required, assign a collision-free distinct ID, such as `WO-001a-fix-01` or `WO-001-fix-01`, incrementing the fix number for further fixes and using the corresponding validation ID. Link the original work order and finding; keep the fix within authorized scope. A non-blocking design preference remains a note unless policy says otherwise. Preserve older IDs as valid history.

**Validation cadence:** fresh independent validation per work order is the default when Git is usable. A user-approved policy adaptation may change that default, subject to external rules. The Git-unavailable exception is defined below. The orchestrator handles dependencies, ordering, and relevant project checks/approvals separately from the validator's assigned comparison. Rapid mode compresses planning only; it does not silently remove review or validation defaults.

**Git-unavailable exception:** only when Git capability for commit and hash review was unavailable at initiative start may design review and work-order validation be recorded as `skipped: Git unavailable`. Do not label these outcomes passed or approved; no commit hash exists. Requirements reviews are distinct and are not skipped by this exception. Closeout may accept the recorded skipped review/validation only in this defined case; see [`06-closeout.md`](06-closeout.md).

**Closeout** ([`06-closeout.md`](06-closeout.md)) is an orchestrator-executed procedure, not a review, and may run in the orchestrator's session. Its verification, follow-ups, final log entry, and status flip are documentation actions; the commit and merge are git operations that require an execution-capable session — when the orchestrator cannot run git commands, it dispatches those actions to an execution-capable coding session or supplies exact scoped commands to the user, then records the resulting evidence in the final log entry (a capability routing rule, not a fresh-context requirement). It verifies its preconditions from recorded evidence, records follow-ups, and commits only authorized initiative-level changes not already committed per work order. It never folds, squashes, or replaces dedicated work-order commits with an initiative/closeout commit. It preserves unrelated user work and blocks if intended changes cannot be isolated safely. It merges the initiative branch into the recorded base branch by default unless the user or host repository policy instructs otherwise; no-branch and reduced-capability modes record the appropriate notice. It then sets the initiative status to `closed`.

Record material decisions, blockers, approvals, and evidence in the initiative plan; use the execution log for material execution events. Do not maintain a ledger of every phase transition or status flip. [`02-initiative.md`](02-initiative.md) provides default planning guidance.

## Initiative modes

Initiatives run in `standard` by default. `rapid` is an explicitly user-requested compressed path for small, well-understood changes. Both follow user instructions and applicable external/project requirements; labels describe activity rather than enforce transitions.

- `rapid` is valid only when the user explicitly requests it, using language such as "use the rapid initiative path" or "fast track this." An agent may recommend rapid mode but may not silently select it.
- Rapid-mode eligibility requires a narrow observable outcome, understood implementation area, no unresolved decision likely to redirect work, bounded surface, practical validation path, and no unaddressed high-risk or external gate. A user request to draft work orders directly is sufficient direction; do not require a redundant direct-translation confirmation.
- If eligibility is unclear, use the standard path. The orchestrator must record any unmet eligibility condition and require standard review for the affected risk.
- If the user requests rapid mode but has not supplied enough information to complete the authorized tasks, prompt the user to fill the gaps before creating or executing work orders. Never silently guess, expand scope, or downgrade the request into an incomplete implementation.

Rapid mode treats the initial prompt as a complete draft when the user has explicitly requested rapid mode. It compresses exploration; it does not imply approval or execution. Retain the initiative plan, explicit scope, work orders, proportionate durable-document impact decisions, applicable default design review/approval, per-work-order validation, and closeout unless the user explicitly adapts project policy and external rules permit it. The defined Git-unavailable exception is not a general rapid-mode bypass.

## User involvement levels

The project manifest records an involvement level describing how much the user wants to participate in technical choices. The level changes who supplies answers, not which applicable external rules are followed. Keep default review, approval, and validation unless the user directs a project-policy adaptation and records it.

- `guided` — the user states goals in plain language; the agent selects technical defaults and gates on the user's delegated behalf, explains them in plain language, and records them as `agent-default` with rationale.
- `collaborative` (default when signals are unclear) — the agent asks about the consequential choices, such as language or framework when undetermined and where the user wants checkpoints, and selects defaults for the rest.
- `direct` — the user wants hands-on control of specifics; the agent asks the full grouped questionnaire in the applicable initialization process document.

The level is a working preference, like rapid mode: it shapes how much the user asks versus decides. At explicit user direction, project policy may be adapted and recorded; do not silently weaken applicable external rules. Guided intake applies the level during initialization; see [`01-init-new.md`](01-init-new.md) and [`01-init-existing.md`](01-init-existing.md).

## Durable-document impact

For a work order with a plausible durable-document impact, record a proportionate classification:

- **Requirements impact** and **Blueprint impact** as relevant: `update`, `verify unchanged`, or `not applicable`;
- affected documents/IDs and a concise rationale; identify authorized updates where needed.

Rules:

- A work order that changes user-facing behavior normally updates or explicitly verifies the relevant requirements.
- A work order that changes architecture, contracts, runtime topology, or durable technical behavior normally updates or explicitly verifies the relevant blueprints.
- When a required durable update is best made before code, the orchestrator may create a narrow document-only work order and make implementation work orders depend on it. This is advisory sequencing, not a gate for changes without meaningful durable impact.
- A work order may legitimately use `not applicable` for either domain when the change is below that document's abstraction level or the durable document already accurately describes the result.
- Use `not applicable` when the change is below the document's abstraction level or has no plausible effect; a brief rationale is sufficient. Do not require exhaustive document synchronization.
- Reviewers assess whether the classification is correct. They never reject a work order merely because it updates a requirements or blueprint document.

[`04-work-order.md`](04-work-order.md) and the work-order template provide a place to record relevant impact; validate proportionately against the actual work, without rote synchronization.

## Default roles

- **Orchestrator:** owns the initiative state, dispatches agents, enforces gates, and maintains the execution log. Git operations in initiative branch creation and closeout require an execution-capable session; when the orchestrator session cannot run git commands, it dispatches or delegates those actions with recorded evidence.
- **Architect/planner:** collaborates on intent, constraints, design, and work-order decomposition.
- **Implementer:** coding agent that executes one assigned work order and records the resulting changes.
- **Reviewer:** evaluates an artifact or implementation with fresh context and reports findings without silently changing the reviewed work.
- **Human approver:** resolves decisions and approvals required by the project policy.

A reviewer must not approve its own implementation work.

### Dispatch invariants

Fresh-session separation is the default for independent review and validation, not an arbitrary label gate. An explicit user-approved policy adaptation may change default staffing where external rules allow:

- By default, each narrow work order is assigned to one implementer session.
- By default, each validation is dispatched to a separate fresh-context validation session.
- Design review and validation use fresh independent sessions by default.
- The orchestrator normally prepares context and dispatches work rather than implementing or reviewing in its own session; an explicit user-approved adaptation may change this default where external rules permit.
- Combining distinct work orders into one agent session is not the default; follow explicit user direction when permitted, and retain distinct outcomes, evidence, and attribution.

For parallel work, dispatch one session per work order and preserve the existing ownership and merge constraints. The execution log records the session/agent identifier for each dispatch and completion event.

## Default gate policy

The default is human approval after design review, before implementation. This is distinct from drafting work orders. Human approval is not required after every validation unless project policy says so; independent per-work-order validation remains the default. Closeout follows [`06-closeout.md`](06-closeout.md) and host policy. At explicit user direction, adapt and record project-policy defaults as appropriate; do not require rote duplication in the manifest when a concise durable decision record suffices.

User-directed changes to project policy must be recorded with the decision and rationale in the appropriate durable document. Do not silently bypass external rules; where an external rule is unclear or conflicts with the request, state the uncertainty and stop as needed.

## Project-specific policy

The Project policy section of [`../project-manifest.md`](../project-manifest.md) is the default durable home for project-specific policy. A user-directed adaptation may be recorded in another appropriate durable project document, with a clear link or reference where needed to avoid conflicting copies. Process documents and initiatives use these records but need not repeat them wholesale.

Until the manifest is populated, use the defaults above and treat unknown commands or gates as unknown—not as permission to skip validation.

### Failure and recovery

When a review or command reports a failure, the orchestrator must record the exact finding, affected artifact, attempted remediation, evidence, decision-maker, and next state in the initiative execution log. A failed applicable project-policy gate blocks completion until it is fixed or the authorized user explicitly changes the policy and records the change. External rules must not be silently waived or treated as tabled; report uncertainty or block when they cannot be met. For design review and validation, only a defined hard-stop finding blocks or creates fix work; ordinary warnings, notes, and optional-evidence gaps do not.

## Authoring rules shared by all processes

1. Use the role-specific context above: do not require a plan/process-document read for implementers or validators when their assigned task evidence is sufficient.
2. Prefer evidence from the repository over assumptions. Label observed, inferred, desired, and provisional information where relevant.
3. Keep scope explicit. Every work order must state what is in scope and out of scope.
4. Preserve requirements/blueprint traceability when relevant to the work, and link implementation evidence back to its work order without redundant documentation.
5. Make the smallest durable documentation update that accurately reflects the resulting system.
6. Append execution events; do not rewrite history to hide failed attempts or discarded decisions.
7. Stop and mark the work `blocked` when required information or validation is unavailable.

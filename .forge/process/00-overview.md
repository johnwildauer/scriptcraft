# Software Forge Process Overview

## Purpose

This directory contains the instructions that agents use to initialize a repository, plan an initiative, execute work orders, validate changes, and close out the resulting branch. The workflow is documentation-driven: agents must read the durable requirements and blueprints before proposing or implementing changes, and must leave an auditable record of decisions and evidence.

The installed workflow begins at [`../_index.md`](../_index.md). This file defines the shared vocabulary, lifecycle, defaults, and project-specific policy that all other process documents inherit.

## Artifact boundaries

- [`../requirements/`](../requirements/) is the durable product definition: what the system should do, for whom, and why.
- [`../blueprints/`](../blueprints/) is the durable technical definition: how the system is organized and how behavior is realized.
- [`../initiatives/`](../initiatives/) contains bounded work. An initiative may update requirements or blueprints only when its work orders explicitly authorize those updates.

Temporary planning notes, agent conversations, and scratchpads are inputs to the workflow, not durable sources of truth. When a decision becomes relevant to future work, record it in the appropriate durable document or initiative log.

## Shared lifecycle

### Initialization

Use [`01-init-new.md`](01-init-new.md) for a new or empty repository and [`01-init-existing.md`](01-init-existing.md) for a repository with existing code or artifacts. Both paths use [`../project-manifest.md`](../project-manifest.md) as the durable record and produce a product overview, requirements/blueprints baseline, project policy, independent validation findings, and a recommended first initiative. When the manifest is missing or not `complete`, the first agent guides the user through initialization; when it is `complete`, agents skip initialization and begin new work at [`02-initiative.md`](02-initiative.md).

### Initiative execution

Use [`02-initiative.md`](02-initiative.md) to turn a requested change into a scoped initiative and ordered work orders. Validate the proposed design with [`03-design-review.md`](03-design-review.md), implement each work order with [`04-work-order.md`](04-work-order.md), validate each completed work order with [`05-validation.md`](05-validation.md), and close the initiative with [`06-closeout.md`](06-closeout.md).

### Review routing

Route each review by the artifact and evidence it is authorized to assess.

| Review | Reviews | May run when | Must not assess | Required output |
|---|---|---|---|---|
| Requirements review | Product and feature intent, scope, users, and acceptance criteria | A requirements draft exists and is ready for independent review | Implementation completeness or unauthorized code | `requirements-review.md` in the initiative's `reviews/` directory, or the configured requirements-review location when no initiative exists yet |
| Design review | The initiative plan and its work orders | The initiative is in the `review` phase and design-readiness checks pass | Code or implementation obligations not authorized by the plan | `design-review.md` in the initiative's `reviews/` directory |
| Validation | One completed work order | All validation preconditions below are satisfied | Unimplemented design obligations or future scope | `validation-[work-order-id].md` in the initiative's `validations/` directory |

Requirements review is distinct from design review: it determines whether intent is clear, coherent, and testable before technical planning relies on it. Its outcome is `pass` only when no blocker remains; warnings require the configured owner/approver to accept them. A failed or blocked requirements review returns the requirements artifact to `draft` or `blocked` with a recorded next action and does not authorize design work.

Before dispatching validation, the orchestrator must verify that:

1. the work order is marked `in-review` after implementation;
2. the implementer recorded all changed files, including created/deleted files;
3. validation attempts and results, including unavailable or failed checks, are recorded; and
4. a comparable branch diff, file diff, or equivalent evidence exists.

If the work order is not marked `in-review` or the evidence package cannot be made available, do not dispatch the validator. Otherwise, dispatch with the available evidence: incomplete or unavailable optional evidence is a note or warning unless it prevents meaningful comparison, while an unavailable required project-policy gate is a hard stop. The validator assesses only authorized work-order alignment, clearly rogue or unrelated behavior, explicit acceptance criteria, and applicable required policy gates. Design preferences and other non-blocking deficiencies remain notes unless project policy explicitly routes them to design review; an implementation defect against an approved design is assessed only within the work order's authorized boundary.

Every review records its result in the initiative's `reviews/` directory (requirements and design reviews) or `validations/` directory (validations) and links it from the reviewed artifact and execution log. Each finding includes severity, evidence, recommendation, and required next state.

## Status vocabulary

Use these statuses unless the project policy defines a stricter compatible vocabulary:

| Status | Meaning |
|---|---|
| `draft` | The artifact is being authored and is not ready for execution. |
| `in-review` | A reviewer or human is evaluating the artifact. |
| `approved` | Required approval for the current gate has passed. |
| `in-progress` | Work is actively being performed. |
| `blocked` | Safe progress requires a decision, fix, external action, or manual gate. |
| `complete` | The artifact's scoped work and required validation are complete. |

Do not mark an artifact `complete` merely because an agent stopped working. Record the evidence and gate outcome that support the status transition. `closed` is not an artifact status: it is the terminal initiative status only. For work orders, `in-review` means "implemented with evidence recorded, awaiting validation."

## Initiative lifecycle

The statuses above apply to artifacts: work orders, reviews, requirements, and blueprints. Initiatives use a two-field model: **phases** are activity words naming the work currently in progress, and **status** is a condition word describing the initiative as a whole. Gates are transition criteria recorded with evidence, not states, and they keep the approval vocabulary.

```text
Phase:  draft -> plan -> review -> execute <-> validate -> closeout
Status: active ......... blocked overlay ......... closed terminal
```

- **Phase** (`draft`, `plan`, `review`, `execute`, `validate`, `closeout`) names exactly one value: the current activity. `execute` and `validate` alternate per work order in the default loop. There is no per-phase status ledger: past phases are evidenced by transition evidence, and future phases are implied by the fixed sequence.
- **Status** (`active`, `blocked`, `closed`) describes the initiative as a whole. `blocked` is valid in any phase and must be cleared before the next forward transition. `closed` is the terminal status, set when closeout completes; the final transition is a **status flip**, not a phase advance — which is why `closed` is a status word and not a phase.

### Phase definitions

| Phase | Entered when | Primary activity | Primary artifacts | Exits when | Process doc |
|---|---|---|---|---|---|
| `draft` | The initiative is created | Explore and converge with the user; author the initiative plan | Initiative plan, exploration scratchpad | Convergence checklist met; user asks for work orders; direct-translation confirmation when applicable | [`02-initiative.md`](02-initiative.md) |
| `plan` | The draft exit gate passes | Create work orders from the converged plan | Work-order files | All required work orders exist and review artifacts are ready | [`02-initiative.md`](02-initiative.md) |
| `review` | The plan exit gate passes | Independent design review of the converged plan and its work orders | `design-review.md` | Review passes and human design approval is recorded | [`03-design-review.md`](03-design-review.md) |
| `execute` | The review exit gate passes; re-entered from `validate` whenever another work order (or fix) is dispatched | Implement work orders in separate implementer sessions, in dependency order | Implementation evidence per work order | All work orders are complete (default loop) or implemented with evidence (consolidated cadence) | [`04-work-order.md`](04-work-order.md) |
| `validate` | A work order is implemented with evidence (default loop) or all are implemented (consolidated) | Fresh-context validation per work order — compare the reported work with available implementation evidence, identify clearly unrelated or rogue behavior, and assess explicit required criteria or policy gates | `validations/validation-[work-order-id].md` | All work orders complete and any approval or policy gate required by project policy is recorded → `closeout`; a hard-stop validation finding creates a fix work order and returns to `execute` | [`05-validation.md`](05-validation.md) |
| `closeout` | The validate exit gate passes | Verify preconditions from recorded evidence; record follow-ups, final log entry, commit, merge — git actions require an execution-capable session | Follow-up entries, final log entry, commit/merge record | Closure actions complete; status set to `closed` | [`06-closeout.md`](06-closeout.md) |

The mapping is 1:1 — every phase after planning has exactly one process doc.

### Gates

Gates name what must be true to move on. They are recorded with evidence and keep the approval vocabulary.

- **`draft -> plan`:** the convergence checklist is met and the user asks for work orders.
- **`review -> execute`:** the design review passes and the required human design approval is recorded. This is the default gate; human approval is not required after every validation unless project policy says so.
- **`validate -> closeout`:** all work orders are complete and any approval of the result required by project policy is recorded.

### Orchestrator execution loop (default, all modes including rapid)

For each work order in dependency order:

1. dispatch a fresh implementer session ([`04-work-order.md`](04-work-order.md));
2. receive the completion report — changed files, validation attempts, diff evidence — and set the work order to `in-review`;
3. dispatch a fresh validation session ([`05-validation.md`](05-validation.md));
4. receive the validation result:
    - **pass** (including recorded warnings or notes) — set the work order to `complete`; proceed to the next work order;
    - **fail with a genuine hard-stop finding** — create a fix work order and loop; or
    - **blocked** — record the blocker, apply the `blocked` status, and stop.

The initiative's phase alternates `execute` and `validate` with each dispatch; the execution log records each transition with the session/agent identifier. With permitted parallelism, multiple work orders may be in flight; record transitions per dispatch and completion event.

**Fix work orders:** a failed validation triggers the orchestrator to create a letter-suffixed fix work order — `WO-001a-short-slug.md`, then `WO-001b-short-slug.md` — linking the original work order and the failing validation, carrying the validator's findings and required fixes, and scoped within the original's boundaries. Dispatch fixes to a new implementer session (the fresh-session invariant applies), followed by a fresh validation session. The original work order is set to `complete` only when one of its fixes passes validation, with the fix chain linked as evidence. A design preference or other non-blocking design deficiency remains a note, or follows a separate design-review route only when project policy explicitly requires it; a genuine implementation defect within the approved boundary routes to a fix work order. [`05-validation.md`](05-validation.md) defines the convention.

**Validation cadence:** `per work order` is the default (the interleaved loop above). `consolidated` validation — implementing all work orders in `execute`, then validating each in `validate`, still one fresh validation session per work order — is opt-in only at the user's explicit request, and the agent must warn the user when it has high confidence that cascade failure is probable without per-work-order validation. Project policy may define custom grouping. Even in rapid mode, validation occurs per work order from a different validation agent; rapid mode changes the draft phase, not the validation loop.

**Closeout** ([`06-closeout.md`](06-closeout.md)) is an orchestrator-executed procedure, not a review, and may run in the orchestrator's session. Its verification, follow-ups, final log entry, and status flip are documentation actions; the commit and merge, however, are git operations that require an execution-capable session — when the orchestrator cannot run git commands, it dispatches those actions to an execution-capable coding session or supplies the exact commands to the user, then records the resulting evidence in the final log entry (a capability routing rule, not a fresh-context requirement). It verifies its preconditions from recorded evidence, then records follow-ups, appends the final execution-log entry, ensures all changes are committed to Git (the default when Git is available; reduced-capability mode records fallback evidence and the explicit notice instead), merges the initiative branch into the recorded base branch (the default, unless the user or host repository policy instructs otherwise; no-branch and reduced-capability modes skip with the recorded notice), and sets the initiative status to `closed`.

Every phase transition and the terminal status flip are recorded with evidence in the initiative plan; the execution log records execution-phase events. [`02-initiative.md`](02-initiative.md) defines the transition procedures.

## Initiative modes

Initiatives run in one of two modes. `standard` is the default and follows the full lifecycle above. `rapid` is a compressed path for small, well-understood changes and is explicitly opt-in.

- `rapid` is valid only when the user explicitly requests it, using language such as "use the rapid initiative path" or "fast track this." An agent may recommend rapid mode but may not silently select it.
- Rapid-mode eligibility requires all of the following: one narrow observable outcome; a known implementation pattern or sufficiently understood repository area; no unresolved product or architecture decision that could redirect the work; a bounded file/system surface; a practical validation path; no migration, security-sensitive, destructive, or high-risk external gate unless the user explicitly reclassifies the work as standard; and explicit confirmation when the initial prompt is broad enough that it could be translated directly into work orders.
- If eligibility is unclear, use the standard path. The orchestrator must record any unmet eligibility condition and require standard review for the affected risk.
- If the user requests rapid mode but has not supplied enough information to complete the authorized tasks, prompt the user to fill the gaps before creating or executing work orders. Never silently guess, expand scope, or downgrade the request into an incomplete implementation.

Rapid mode removes the default gates around the draft phase: the initial initiative prompt is taken as a complete draft, ready for planning (creating work orders) and execution/validation. The exploration/convergence loop is replaced by a single confirmation that the prompt-as-draft is accurate, and the design-readiness confirmation replaces the full design-review gate. Rapid mode never omits the initiative plan, explicit selected and excluded scope, work-order artifacts, durable-document impact decisions, the per-work-order validation loop with separate sessions, or closeout and follow-up recording.

## User involvement levels

The project manifest records an involvement level describing how much the user wants to participate in technical choices. The level changes who supplies the answers, not what gets recorded: every manifest field is completed at every level, and all gates, reviews, and validations still run.

- `guided` — the user states goals in plain language; the agent selects technical defaults and gates on the user's delegated behalf, explains them in plain language, and records them as `agent-default` with rationale.
- `collaborative` (default when signals are unclear) — the agent asks about the consequential choices, such as language or framework when undetermined and where the user wants checkpoints, and selects defaults for the rest.
- `direct` — the user wants hands-on control of specifics; the agent asks the full grouped questionnaire in the applicable initialization process document.

The level is a working preference, like the rapid-mode preference: it shapes how much the agent asks versus decides, and it may be revised at any time through the manifest's revision history. Agents never weaken gates, reviews, or validation because the user is less involved. Guided intake applies the level during initialization; see [`01-init-new.md`](01-init-new.md) and [`01-init-existing.md`](01-init-existing.md).

## Durable-document impact

Every work order classifies its impact on durable documents:

- **Requirements impact** and **Blueprint impact**, each `update`, `verify unchanged`, or `not applicable`;
- the affected documents/IDs, the rationale and evidence for the classification, and the authorized documentation changes.

Rules:

- A work order that changes user-facing behavior normally updates or explicitly verifies the relevant requirements.
- A work order that changes architecture, contracts, runtime topology, or durable technical behavior normally updates or explicitly verifies the relevant blueprints.
- A work order may legitimately use `not applicable` for either domain when the change is below that document's abstraction level or the durable document already accurately describes the result.
- `not applicable` is never a blank exemption: it requires a rationale and evidence that the document was checked.
- Reviewers assess whether the classification is correct. They never reject a work order merely because it updates a requirements or blueprint document.

[`04-work-order.md`](04-work-order.md) requires the classification; [`05-validation.md`](05-validation.md) compares the final documentation impact to the actual diff and requires correction only when the classification or update is inaccurate.

## Default roles

- **Orchestrator:** owns the initiative state, dispatches agents, enforces gates, and maintains the execution log. Git operations in initiative branch creation and closeout require an execution-capable session; when the orchestrator session cannot run git commands, it dispatches or delegates those actions with recorded evidence.
- **Architect/planner:** collaborates on intent, constraints, design, and work-order decomposition.
- **Implementer:** executes one assigned work order and records the resulting changes.
- **Reviewer:** evaluates an artifact or implementation with fresh context and reports findings without silently changing the reviewed work.
- **Human approver:** resolves decisions and approvals required by the project policy.

A reviewer must not approve its own implementation work.

### Dispatch invariants

Fresh-session separation is required, not optional staffing:

- Each work order is dispatched to a separate implementer session.
- Each validation is dispatched to a separate fresh-context validation session.
- The design review and each validation are separate fresh-context sessions.
- The orchestrator may prepare context and dispatch work, but must not execute a work order or a review in its own session.
- Combining work orders into one agent session is prohibited unless the user explicitly authorizes an emergency exception, which must be recorded with reduced-confidence evidence.

For parallel work, dispatch one session per work order and preserve the existing ownership and merge constraints. The execution log records the session/agent identifier for each dispatch and completion event.

## Default gate policy

The default is human approval after design review; it is the `review -> execute` gate. Human approval is not required after every validation unless the project policy says so; validations still run according to the configured validation cadence, which defaults to one validation per work order. Closeout is mandatory. Closeout commits all changes to Git and merges the initiative branch by default, deferring to user instruction and host repository policy per [`06-closeout.md`](06-closeout.md). These defaults must be recorded in the [`../project-manifest.md`](../project-manifest.md) when no project-specific configuration exists.

An automated bypass is valid only when it is documented in this policy section or in an initiative-specific decision that is stricter than the policy. Every bypass must identify the policy basis, approver or standing authority, and validation evidence.

## Project-specific policy

All project-specific policy — repository and branch facts, commands, gates, tools, parallelism, and recovery rules — lives in the Project policy section of [`../project-manifest.md`](../project-manifest.md). Initialization populates it; process documents and initiatives read it and must not duplicate it here.

Until the manifest is populated, use the defaults above and treat unknown commands or gates as unknown—not as permission to skip validation.

### Failure and recovery

When a review or command reports a failure, the orchestrator must record the exact finding, affected artifact, attempted remediation, evidence, decision-maker, and next state in the initiative execution log. A failed explicit project-policy gate blocks completion until it is fixed, explicitly waived under the project policy in the manifest, or tabled as follow-up work. For design review and validation, only a defined hard-stop finding blocks or creates fix work; ordinary warnings, notes, and optional-evidence gaps do not.

## Authoring rules shared by all processes

1. Read the relevant process file, initiative artifacts, requirements, and blueprints before acting.
2. Prefer evidence from the repository over assumptions. Label observed, inferred, desired, and provisional information where relevant.
3. Keep scope explicit. Every work order must state what is in scope and out of scope.
4. Preserve traceability from work order to requirements/blueprints and from implementation back to the work order.
5. Make the smallest durable documentation update that accurately reflects the resulting system.
6. Append execution events; do not rewrite history to hide failed attempts or discarded decisions.
7. Stop and mark the work `blocked` when required information or validation is unavailable.

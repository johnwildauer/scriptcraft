# Project Manifest

This is ScriptCraft's durable project-specific contract. It records project intent,
policy, unresolved decisions, and a dated repository baseline without treating
research notes or implementation observations as product mandates.

## Metadata

- **Status:** `complete` — initialization reviews and historical baseline evidence are linked; review warnings remain follow-ups.
- **Path type:** `existing`
- **Initialized on:** `2026-10-25`
- **Last revised:** `2026-11-04`
- **Record owner:** `Forge initialization agent`

## Project intake

- **Project:** ScriptCraft
- **What is being created and for whom:** A desktop-first Rust screenplay-writing app for individual screenwriters.
- **Problem/opportunity:** Individual writers need a dependable place to draft screenplay content without losing semantic structure, edits, page flow, or saved work.
- **Primary goal:** Establish a dependable screenplay-writing loop: create, edit, transition roles, paginate, save, reopen, and export useful MVP outputs.
- **First-pass success:** A writer can complete a representative scene through a headless-capable core and focused desktop UI, with deterministic pagination and safe, no-data-loss save/reopen behavior.
- **Explicit non-goals:** DOCX compatibility, collaboration, MCP/control infrastructure, planning or beat-board editing features, broad Word-like formatting, and web/multi-platform polish. FDX is the native document direction; preservation of unsupported beat-board data is distinct from editing those features and does not promise blanket Final Draft compatibility.
- **Planning preference:** Replan after each initiative; do not treat the historical first-initiative handoff as a standing roadmap.
- **User checkpoint preference:** Guided involvement; all review and validation are completed by the user under the repository-wide policy below. Do not dispatch agent reviewers or validators or run agent-side checks.
- **Working style:** Guided and safety-conscious, with concise explanations for consequential defaults; the user handles all review and validation.
- **Technology/constraints:** Rust and desktop-first are user-approved. Current package/dependency observations are recorded below; framework approval, product-supported OS, packaging, and exact desktop UI policy remain open unless separately decided.
- **Additional context:** `.analysis/` is a research/proposal source, not an authorization to copy every recommendation as an implementation command.

## Involvement level

- **Level:** `guided`
- **Decided by:** `asked directly` — user explicitly approved guided involvement.
- **Interpretation:** The agent may select bounded technical defaults, but must label them `agent-default`, preserve unknowns, and never weaken product safety behavior or applicable external obligations. The user owns all review and validation.

## Technical preferences

States use `confirmed`, `provisional`, `agent-default`, or `unknown`. Repository observations do not by themselves establish product approval.

- **Language:** Rust — state: `confirmed`
- **Framework:** Product approval/status `unknown`. The desktop package currently declares `eframe` 0.29 and `rfd` 0.15 (observed 2026-10-30); dependency presence is not a claim of user approval or a future product mandate.
- **Runtime:** Current runtime behavior is not re-verified here; the desktop and reusable-core packages are present in the workspace as recorded below.
- **Package/build tool:** Cargo workspace — observed 2026-10-30 in root `Cargo.toml`, with members `apps/scriptcraft` and `crates/scriptcraft-core`.
- **Target platform:** Desktop-first — state: `confirmed`. Toolchain host reported by `rustc -vV` on 2026-10-30: `aarch64-apple-darwin`; this is not a product support or distribution commitment.
- **Technical constraints/preferences:** Keep the core independent of windowing APIs where practical; preserve semantic screenplay roles, canonical UTF-8 text, deterministic page geometry, atomic native saves, save-failure safety, and headless testing. These remain desired MVP constraints, not claims that all behavior is currently implemented or verified. Snapshot recovery is not a product promise; no replacement recovery capability is implied.

## Working agreement

- **How the user wants to work:** Dependable results with technical diligence handled by agents; avoid unnecessary scope and checkpoints.
- **Preferred communication/decision style:** Guided; explain consequential defaults and record rationale, ask only user-only questions.
- **Ideal user/agent development cycle:** Initiative-level planning/design confirmation and consolidated closeout confirmation when Forge permits; no manual checkpoint after each work order.
- **Validation cadence:** `per work order` — every work order gets a separate fresh validation agent immediately after implementation. No manual confirmation is required after each validation by default.
- **Rapid-mode preference:** `standard unless requested` — rapid must be explicitly requested per initiative.
- **Review configuration:**
  - Design review required: `yes` by default under Forge policy; initiative-specific user direction may set the appropriate path.
  - Validation cadence: `per work order`.
  - Requested review depth: independent alignment review for requirements/blueprints and fresh-context implementation validation; do not claim a review or validation without its evidence artifact.

## Repository baseline

- **Repository state and evidence:** `existing`. On 2026-10-30, the workspace contains the root Cargo manifest, `apps/scriptcraft/`, `crates/scriptcraft-core/`, and source files beneath both packages. The root manifest lists those two workspace members; the desktop package manifest declares `eframe`, `rfd`, and the local core package. These are checkout observations, not claims that product requirements are fully implemented or that checks passed.
- **Git installed/repository initialized:** `yes` — Git status and history commands succeeded on 2026-10-30.
- **Current branch:** `dev/2026-10-30-documentation-cleanup` observed on 2026-10-30; this is a dated checkout fact, not the default branch.
- **Default/base branch:** `master` — recorded in the 2026-10-30 initiative plan as the branch from which the initiative branch was created.
- **Local initiative branches available:** `yes` — creation and switch to the initiative branch from `master` succeeded, as recorded in the initiative plan on 2026-10-30.
- **Remote:** No remotes were configured in `git remote -v` on 2026-10-30. This may change; re-check before remote-dependent actions.
- **Initiative branch policy:** `required` when local branch capability is available; record the reduced-capability fallback if it is not.
- **Default initiative branch naming convention:** `dev/YYYY-MM-DD-short-slug` — no override.
- **Commit/pull-request policy:** Git commit and merge are the default closeout actions when capability and policy permit; exact host PR policy is unknown and must be discovered before a PR-based closeout.
- **Reduced-capability fallback:** If branch capability is unavailable, use comparable file/diff evidence and record an explicit no-branch notice; owner is the orchestrator, who must verify or restore local branch capability before implementation where possible.

## Project policy

### Authoritative internal review and validation policy

**Repository-wide user direction — effective 2026-10-31:** Internal agent review and validation are disabled for this repository. The user performs all review and validation. Agents must not dispatch or conduct design, requirements, blueprint, implementation, or other reviews; dispatch independent validators; or run tests, builds, lint, formatting checks, manual verification, implementation checks, or other validation. This applies to every initiative and supersedes the former initiative-only no-independent-validator exception and all active contrary review, validation, and implementation-check requirements in this manifest, Forge process guidance, and initiative/work-order records. Preserve those prior instructions, findings, and recorded outcomes as historical evidence; do not treat them as current authorization or as newly verified results.

This workflow policy does not remove, weaken, or authorize bypassing in-product safety checks or safeguards. Applicable external rules and obligations remain in force and are not waived by this policy. If a genuine external requirement conflicts with user-owned review/validation or this no-agent-run direction, surface it as a blocker; do not claim the requirement was waived or substitute an agent result. This policy update itself is not evidence that any check passed.

### Commands

The commands below are the project's recorded check inventory and historical
results, not authorization for agent-run checks under the authoritative policy
above. The user performs any review and validation. Discover applicable external
gates rather than assuming none; do not treat them as waived. The historical
results below come from the foundation validation dated 2026-10-25 and are not
fresh results for a later checkout.

- **Install/setup:** No separate setup command is recorded; prerequisites and setup requirements must be discovered when needed.
- **Format:** `cargo fmt --all -- --check` — passed in the historical foundation validation dated 2026-10-25.
- **Lint/static analysis:** `cargo clippy --workspace --all-targets -- -D warnings` — passed in the historical foundation validation dated 2026-10-25.
- **Unit/integration tests:** `cargo test --workspace` — passed in the historical foundation validation dated 2026-10-25 (1 core unit test at that time).
- **Build/package:** `cargo build --workspace` and `cargo check --workspace` — passed in the historical foundation validation dated 2026-10-25.
- **Run/manual verification:** `cargo run -p scriptcraft` — the historical foundation validation dated 2026-10-25 recorded headless output `scriptcraft foundation ready`; this is not a current run result or a visible-window check.
- **Historical evidence:** [`screenplay MVP foundation validation`](initiatives/2026-10-25-screenplay-mvp/validations/validation-WO-001.md) records those command outcomes and its warnings. No command result is represented as freshly verified by this manifest revision.

### Current initiative evidence — 2026-11-04

For the bounded build-cleanup initiative, the user explicitly authorized coding
agents to run formatting, compilation, build, Clippy, and test commands. This is
an initiative-specific exception to the repository-wide no-agent-check policy,
not a blanket revocation of that policy for later initiatives.

The final agent-run inventory passed `cargo fmt --all -- --check`,
`cargo check --workspace`, `cargo build --workspace`,
`cargo clippy --workspace --all-targets -- -D warnings`, and
`cargo test --workspace`. The workspace test result is four passed and zero
failed. Under explicit user direction, the historical work-order suite was
classified obsolete by default and replaced by four consolidated headless
acceptance scenarios in [`core_acceptance.rs`](../crates/scriptcraft-core/tests/core_acceptance.rs:1).
The retired suite's failures were not repaired and are not represented as
resolved product defects. Exact initial/final outcomes and consolidation
boundaries are recorded in the [`build-hygiene report`](../.plan/2026-11-04-build-cleanup/build-hygiene-report.md).

### Gates and intervention

- **Design approval:** Human approval required by default at `review -> execute`, subject to explicit user direction and applicable external rules.
- **Validation cadence:** User performs all review and validation repository-wide. No agent validator or agent-run check is permitted under the authoritative policy above.
- **Historical initiative-specific exception — document-model-refactor (2026-10-31; superseded):** The prior user direction was: “Do not dispatch further independent validation agents for this initiative; record this policy change and continue with the implementation checks only.” It superseded the former per-work-order validator cadence only for that initiative and allowed implementation checks. The newer repository-wide direction supersedes that exception and its implementation-check allowance. Preserve its history, including the existing WO-001 finding and the recorded absence of WO-002–WO-011 validation artifacts; do not dispatch reviewers/validators or run checks under the former exception.
- **Manual/external gates:** No additional manual/external gate was recorded as a project-wide policy; this is not a waiver. Discover and record any applicable gate and surface genuine conflicts before treating it as optional or complete.
- **Automation bypasses:** none.
- **Network or destructive-action restrictions:** Network access is not required solely by project policy. Avoid destructive filesystem or Git actions without explicit authorization and recorded evidence.
- **User confirmation:** The user completes all review and validation. Existing user approval and closeout requirements remain applicable; this policy does not itself satisfy or waive them.

### Tools and parallelism

- **Required tools/MCP servers:** none.
- **Optional tools/MCP servers:** repository inspection tools; no MCP product/control feature is in MVP scope.
- **Default parallelism:** sequential work orders.
- **Permitted parallelism:** unknown; prefer sequential until ownership and merge evidence are verified.

### Failure and recovery

Record exact command/review failures, affected artifact, attempted remediation,
evidence, decision-maker, and next state in the applicable initiative log. A
required unknown command is not a permission to skip validation. Stop and mark
the artifact blocked when safe comparison or a required gate is unavailable.

## Decisions and handoff

- **Unresolved decisions and follow-ups:**
  - Product approval of the current desktop framework/dependency choice, supported desktop OS, and packaging/distribution policy — owner: product/technical owner; trigger: before those choices are treated as durable product commitments; next action: confirm scope and record the decision with evidence.
  - PDF implementation — owner: product/technical owner; trigger: export design; next action: decide and record after the core model boundary is validated. Native projects use FDX.
  - Remote/PR policy — owner: orchestrator; trigger: before remote-dependent or PR-based closeout; next action: discover host expectations and recheck configured remotes.
  - Requirements and blueprint review warnings — owners: requirements/blueprint owners with orchestrator coordination; trigger: before affected intent or contracts are treated as fully resolved; next action: preserve the independent findings and record each warning's disposition. The initialization reviews exist and are linked below; their warnings are not represented as resolved.
- **Historical first-initiative handoff:** The 2026-10-25 `workspace-foundation` recommendation led into the actual screenplay MVP initiative. See the [`screenplay MVP plan`](initiatives/2026-10-25-screenplay-mvp/2026-10-25-screenplay-mvp-v1.md) and its [`foundation validation`](initiatives/2026-10-25-screenplay-mvp/validations/validation-WO-001.md). This is historical traceability, not a new roadmap recommendation or a claim that all product behavior is complete.

## Roadmap

- **Roadmap mode:** `open-ended replan`
- **Historical handoff:** The initial workspace-foundation recommendation is recorded in the actual screenplay MVP plan linked above. No future initiative count or sequence is approved here.
- **Roadmap revision rule:** Revise the roadmap after each initiative while preserving prior decisions and rationale.

## Completion evidence

- Product overview: [`requirements/product-overview.md`](requirements/product-overview.md)
- Technical context: [`blueprints/technical-context.md`](blueprints/technical-context.md)
- Project policy: this manifest's Project policy section
- Historical first-initiative plan: [`2026-10-25-screenplay-mvp-v1.md`](initiatives/2026-10-25-screenplay-mvp/2026-10-25-screenplay-mvp-v1.md)
- Requirements review findings: [`requirements-review.md`](requirements/requirements-review.md) — independent fresh-context review completed with warnings; warning dispositions remain follow-ups.
- Blueprint review findings: [`blueprint-review.md`](blueprints/blueprint-review.md) — independent fresh-context review completed with warnings; warning dispositions remain follow-ups.
- Historical foundation validation: [`validation-WO-001.md`](initiatives/2026-10-25-screenplay-mvp/validations/validation-WO-001.md) records the exact 2026-10-25 checks and warnings; it is not evidence of fresh checks for this revision.
- Initialization completion evidence consists of the linked independent review artifacts and historical validation; the recorded warnings remain follow-ups rather than missing-review claims.

## Scope revision — 2026-10-31

The user directed FDX-based native documents and preservation of unsupported FDX content, replacing the former FDX non-goal; this does not authorize beat-board editing or claim universal compatibility. Fountain conversion is secondary to completing the FDX model. See the [document-model initiative](initiatives/2026-10-31-document-model-refactor/document-model-refactor-v1.md) for decisions, implementation lag and outstanding review gates. Project workflow policy is unchanged.

The historical requirements and blueprint review artifacts referenced above are absent from the inspected checkout. Their detailed findings and dispositions have not been re-verified; fresh independent alignment reviews remain required for the FDX scope revision.

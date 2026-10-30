# Project Manifest

This is the durable project-specific contract for ScriptCraft. It records
user-approved defaults, durable policy, and a dated repository baseline; it does
not treat research notes or historical command results as current verification.

## Metadata

- **Status:** `complete` — initialization reviews and baseline command evidence are recorded; review warnings remain follow-ups
- **Path type:** `existing`
- **Initialized on:** `2026-10-25`
- **Last revised:** `2026-10-30`
- **Record owner:** `Forge initialization agent`

## Project intake

- **Project:** ScriptCraft
- **What is being created and for whom:** A desktop-first Rust screenplay-writing app for individual screenwriters.
- **Problem/opportunity:** Individual writers need a dependable place to draft screenplay content without losing semantic structure, edits, page flow, or saved work.
- **Primary goal:** Establish a dependable screenplay writing loop: create, edit, role transitions, paginate, save, reopen, and export useful MVP outputs.
- **First-pass success:** A writer can complete a representative scene through a headless-capable core and focused desktop UI, with deterministic pagination and safe native saving, including failure safety.
- **Explicit non-goals:** DOCX/FDX compatibility, collaboration, MCP/control infrastructure, planning or beat-board features, broad Word-like formatting, and web/multi-platform polish.
- **Planning preference:** The initial recommendation was for one first initiative only; replan after the workspace foundation and product baseline are proven. That recommendation is now historical; its outcome is recorded below.
- **User checkpoint preference:** Guided involvement; no manual checkpoint after each work order. User confirmation is consolidated at initiative-level/closeout if Forge permits, while every work order still receives a separate fresh validation agent immediately after implementation.
- **Working style:** Guided, safety-conscious, concise explanations for consequential defaults, with independent checks handled in the background.
- **Technology/constraints:** Rust and desktop-first are user-approved. Specific supported operating systems, packaging, and long-term UI framework policy remain unknown unless established by later evidence and decision.
- **Additional context:** `.analysis/` is a research/proposal source, not an authorization to copy every recommendation as an implementation command.

## Involvement level

- **Level:** `guided`
- **Decided by:** `asked directly` — user explicitly approved guided involvement.
- **Interpretation:** The agent may select bounded technical defaults, but must label them `agent-default`, preserve unknowns, and never weaken review or validation gates.

## Technical preferences

States use `confirmed`, `provisional`, `agent-default`, or `unknown`.

- **Language:** Rust — state: `confirmed`
- **Desktop framework:** Project-level approval as a permanent choice is `unknown`. The app manifest listed `eframe` and `rfd` on 2026-10-30; this is a dated repository observation, not a new project mandate.
- **Runtime/package structure:** A Cargo workspace with `scriptcraft` and `scriptcraft-core` packages was present on 2026-10-30. The core package is a path dependency of the app; this observation does not imply a separate runnable container.
- **Package/build tool:** Cargo workspace — state: `confirmed`; evidence: root `Cargo.toml` and the workspace package manifests.
- **Target platform:** Desktop-first — state: `confirmed`. A foundation validation recorded `aarch64-apple-darwin` as its host on 2026-10-25; that dated host observation does not establish a broader supported-platform commitment.
- **Technical constraints/preferences:** Keep the core independent of windowing APIs where practical; preserve semantic screenplay roles, canonical UTF-8 text, deterministic page geometry, atomic native saves, and headless testing. Native-save failure safety is project policy. No recovery-snapshot capability is a desired product behavior; this decision does not assert that existing implementation code has been removed.

## Working agreement

- **How the user wants to work:** Dependable results with technical diligence handled by agents; avoid unnecessary scope and checkpoints.
- **Preferred communication/decision style:** Guided; explain consequential defaults and record rationale, ask only user-only questions.
- **Ideal user/agent development cycle:** Initiative-level planning/design confirmation and consolidated closeout confirmation when Forge permits; no manual checkpoint after each WO.
- **Validation cadence:** `per work order` — every WO gets a separate fresh validation agent immediately after implementation. No manual confirmation is required after each validation by default.
- **Rapid-mode preference:** `standard unless requested` — the current and first initiative mode is standard; rapid must be explicitly requested per initiative.
- **Review configuration:**
  - Design review required: `yes` — default Forge policy.
  - Validation cadence: `per work order`.
  - Requested review depth: independent alignment review for requirements/blueprints and fresh-context implementation validation; do not claim either has happened until artifacts exist.

## Repository baseline

The following is a dated observation, not a continuously verified status report.

- **Observation date:** `2026-10-30`.
- **Repository state:** Existing Git repository with a populated Rust workspace, `.forge/`, `.analysis/`, `apps/scriptcraft/`, and `crates/scriptcraft-core/`; root `Cargo.toml` names both workspace packages.
- **Git installed/repository initialized:** `yes` — Git status, log, branch, and remote inspection succeeded during this observation.
- **Observed branch/commit:** `dev/2026-10-30-documentation-cleanup` at `548034b701929b3da109a2c8aae5578d710d7538`; `master` also pointed at that commit at observation. Local branch creation/switch capability was exercised for this initiative.
- **Default/base branch:** `master` — retained project convention and observed branch reference; recheck before branch-dependent work.
- **Local initiative branches available:** confirmed for the 2026-10-30 initiative by successful local branch creation and switch; future capability remains subject to the current checkout.
- **Remote:** no configured remote was listed by `git remote -v` on 2026-10-30. This does not establish any host pull-request policy.
- **Observed app dependencies:** `apps/scriptcraft/Cargo.toml` listed `eframe`, `rfd`, and the local `scriptcraft-core` package on 2026-10-30. Dependency presence is not a user-approved long-term framework mandate.
- **Initiative branch policy:** `required` when local branch capability is available; otherwise record the reduced-capability fallback.
- **Default initiative branch naming convention:** `dev/YYYY-MM-DD-short-slug` — no override.
- **Commit/pull-request policy:** Git commit and merge are the default closeout actions when capability and policy permit; host PR policy is unknown and must be discovered before a PR-based closeout.
- **Reduced-capability fallback:** Use comparable file/diff evidence and record an explicit no-branch notice; owner is the orchestrator; next action is to restore or verify local branch capability before implementation.
- **Branch/remote decision owner and next action:** Orchestrator; verify local branch capability before dispatch when needed, and determine remote/PR policy before any remote-dependent action.

## Project policy

### Commands

The following command names are project check policy, not fresh results. Confirm applicable setup and required checks in the active checkout before relying on them.

- **Install/setup:** Current setup requirements are `unknown`; inspect the workspace manifests, lockfile, and environment before checks. Do not assume the workspace is dependency-free or that setup is unnecessary.
- **Format:** `cargo fmt --all -- --check`.
- **Lint/static analysis:** `cargo clippy --workspace --all-targets -- -D warnings`.
- **Unit/integration tests:** `cargo test --workspace`.
- **Build/package:** `cargo build --workspace` and `cargo check --workspace`.
- **Run/manual verification:** `cargo run -p scriptcraft`; determine any required desktop/manual verification for the actual change and environment before execution. No result is implied here.
- **Historical command evidence:** The initial foundation validation records the results and context of its 2025-10-25 command runs; see the linked validation under Completion evidence. Those results are historical, not current check results.

### Gates and intervention

- **Design approval:** human approval required by default at `review -> execute`.
- **Validation cadence:** `per work order`; a fresh independent validation agent follows each implemented work order.
- **Manual/external gates:** applicability is `unknown` until checked for the initiative and environment. Discover and record required desktop/manual or external gates before execution; do not infer that none apply.
- **Automation bypasses:** none.
- **Network or destructive-action restrictions:** No remote was configured in the 2026-10-30 observation; do not require network access for local work. Avoid destructive filesystem or Git actions without explicit authorization and recorded evidence.
- **User confirmation:** consolidated at initiative-level/closeout if Forge permits; this does not bypass design review, per-WO validation, or closeout evidence.

### Tools and parallelism

- **Required tools/MCP servers:** none.
- **Optional tools/MCP servers:** repository inspection tools; no MCP product/control feature is in MVP scope.
- **Default parallelism:** sequential work orders.
- **Permitted parallelism:** unknown; prefer sequential until ownership and merge evidence are verified.

### Failure and recovery

Record exact command/review failures, affected artifact, attempted remediation,
evidence, decision-maker, and next state in the applicable initiative log. A
required unknown command is not permission to skip validation. Stop and mark the
artifact blocked when safe comparison or a required gate is unavailable.

## Decisions and handoff

- **Unresolved decisions:**
  - Exact native project format/extension and remaining export-policy decisions — owner: product/technical owner; trigger: persistence/export changes; next action: verify the current decision and implementation evidence before changing policy.
  - Supported operating systems, long-term framework policy, and packaging — owner: product/technical owner; trigger: when support or delivery commitments are required; next action: decide only within that scope, distinguishing current dependencies from approved policy.
  - Remote and host PR policy — owner: orchestrator; trigger: before remote-dependent work or PR-based closeout; next action: inspect the configured remote and applicable host policy.
  - Initialization requirements/blueprint review warnings — owner: orchestrator; trigger: when the affected findings are reconciled; next action: retain and disposition the linked review follow-ups without claiming them resolved prematurely.
- **Recovery-snapshot intent:** The user directed that recovery snapshots not be part of desired product behavior. Native-save, atomic-replacement, and failure-safety policy remain; this decision does not claim existing implementation code was removed.
- **Historical first-initiative handoff:** The [`Screenplay MVP initiative`](initiatives/2026-10-25-screenplay-mvp/2026-10-25-screenplay-mvp-v1.md) records the executed first-initiative path, including the workspace-foundation work order; its [`foundation validation`](initiatives/2026-10-25-screenplay-mvp/validations/validation-WO-001.md) records independent validation. This is historical evidence, not a pending recommendation or a newly approved roadmap.

## Roadmap

- **Roadmap mode:** `open-ended replan`.
- **Future initiative count/sequence:** No future count or sequence is approved in this manifest.
- **Historical approval:** The initial workspace-foundation recommendation was user-approved on `2026-10-25`; its execution and validation are linked in Decisions and handoff.
- **Roadmap revision rule:** revise after each initiative while preserving prior decisions and rationale.

## Completion evidence

- Product overview: [`requirements/product-overview.md`](requirements/product-overview.md)
- Technical context: [`blueprints/technical-context.md`](blueprints/technical-context.md)
- Project policy: this manifest's Project policy section
- Historical first-initiative handoff: [`Screenplay MVP plan`](initiatives/2026-10-25-screenplay-mvp/2026-10-25-screenplay-mvp-v1.md)
- Historical foundation validation: [`WO-001 validation`](initiatives/2026-10-25-screenplay-mvp/validations/validation-WO-001.md) — records the 2025-10-25 checks and host observation; not a current result.
- Requirements review findings: [`requirements-review.md`](requirements/requirements-review.md) — independent fresh-context review completed with warnings; warning dispositions remain follow-ups.
- Blueprint review findings: [`blueprint-review.md`](blueprints/blueprint-review.md) — independent fresh-context review completed with warnings; warning dispositions remain follow-ups.
- Completion evidence: Initialization review artifacts and historical foundation command evidence are linked above; remaining warnings are preserved as follow-up decisions rather than represented as missing reviews.

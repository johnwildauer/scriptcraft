# Project Manifest

This is the durable project-specific contract for ScriptCraft. It records the
initial baseline and the user-approved defaults without treating research notes
as implemented behavior.

## Metadata

- **Status:** `blocked` — durable baseline populated; initialization review gates remain outstanding
- **Path type:** `existing`
- **Initialized on:** `2026-10-25`
- **Last revised:** `2026-10-25`
- **Record owner:** `Forge initialization agent`

## Project intake

- **Project:** ScriptCraft
- **What is being created and for whom:** A desktop-first Rust screenplay-writing app for individual screenwriters.
- **Problem/opportunity:** Individual writers need a dependable place to draft screenplay content without losing semantic structure, edits, page flow, or saved work.
- **Primary goal:** Establish a dependable screenplay writing loop: create, edit, role transitions, paginate, save, reopen, recover, and export the useful MVP outputs.
- **First-pass success:** A writer can complete a representative scene through a headless-capable core and focused desktop UI, with deterministic pagination and no-data-loss save/recovery behavior.
- **Explicit non-goals:** DOCX/FDX compatibility, collaboration, MCP/control infrastructure, planning or beat-board features, broad Word-like formatting, and web/multi-platform polish.
- **Planning preference:** First initiative recommendation only; replan after the workspace foundation and product baseline are proven.
- **User checkpoint preference:** Guided involvement; no manual checkpoint after each work order. User confirmation is consolidated at initiative-level/closeout if Forge permits, while every work order still receives a separate fresh validation agent immediately after implementation.
- **Working style:** Guided, safety-conscious, concise explanations for consequential defaults, with independent checks handled in the background.
- **Technology/constraints:** Rust and desktop-first are user-approved. Other tool, framework, runtime, packaging, and exact desktop UI choices remain unknown until repository discovery and the workspace foundation.
- **Additional context:** `.analysis/` is a research/proposal source, not an authorization to copy every recommendation as an implementation command.

## Involvement level

- **Level:** `guided`
- **Decided by:** `asked directly` — user explicitly approved guided involvement.
- **Interpretation:** The agent may select bounded technical defaults, but must label them `agent-default`, preserve unknowns, and never weaken review or validation gates.

## Technical preferences

States use `confirmed`, `provisional`, `agent-default`, or `unknown`.

- **Language:** Rust — state: `confirmed`
- **Framework:** none selected in WO-001 — state: `agent-default`; rationale: smallest dependency-free foundation seam; visible UI framework remains a later-WO decision
- **Runtime:** native Rust binary/headless seam — state: `verified` for foundation; rationale: desktop-first boundary with deterministic no-window checks
- **Package/build tool:** Cargo workspace — state: `verified`; evidence: root `Cargo.toml`, members `apps/scriptcraft` and `crates/scriptcraft-core`
- **Target platform:** desktop-first, native first target `aarch64-apple-darwin` — state: `confirmed` for desktop / `verified` for first target; evidence: `rustc -vV`
- **Technical constraints/preferences:** Keep the core independent of windowing APIs where practical; preserve semantic screenplay roles, canonical UTF-8 text, deterministic page geometry, atomic native saves, bounded recovery, and headless testing. These are desired MVP constraints derived from `.analysis`, not current implementation claims.

## Working agreement

- **How the user wants to work:** Dependable results with technical diligence handled by agents; avoid unnecessary scope and checkpoints.
- **Preferred communication/decision style:** Guided; explain consequential defaults and record rationale, ask only user-only questions.
- **Ideal user/agent development cycle:** Initiative-level planning/design confirmation and consolidated closeout confirmation when Forge permits; no manual checkpoint after each WO.
- **Validation cadence:** `per work order` — every WO gets a separate fresh validation agent immediately after implementation. No manual confirmation is required after each validation by default.
- **Rapid-mode preference:** `standard unless requested` — the current and first initiative mode is standard; rapid must be explicitly requested per initiative.
- **Review configuration:**
  - Design review required: `yes` — default Forge policy; no design review has happened in this initialization.
  - Validation cadence: `per work order`.
  - Requested review depth: independent alignment review for requirements/blueprints and fresh-context implementation validation; do not claim either has happened until artifacts exist.

## Repository baseline

- **Repository state and evidence:** `existing` — observed `.analysis/`, `.forge/`, `apps/`, and `crates/` directories; `apps/scriptcraft/` and `crates/` contained no files through workspace inventory at initialization. Existing analysis documents describe a proposed Rust architecture, but are not source-code evidence.
- **Git installed:** `yes` — evidence: successful prior initialization evidence and command result on 2026-10-25.
- **Git repository initialized:** `yes` — evidence: `git status --short --branch` returned a repository state.
- **Current branch:** `master` — evidence: Git command result.
- **Default/base branch:** `master` — evidence: user-approved prior successful initialization and current branch result; no commit exists yet.
- **Local initiative branches available:** `unknown` — not separately verified; discovery: attempt a local branch creation during the first initiative, or run a non-destructive branch capability check in an execution-capable session.
- **Remote:** `unknown` / no remote currently known — evidence: `git remote -v` returned no entries; discovery: re-check before any remote-dependent action.
- **Initiative branch policy:** `required` when local branch capability is confirmed; otherwise reduced-capability fallback must be recorded.
- **Default initiative branch naming convention:** `dev/YYYY-MM-DD-short-slug` — no override.
- **Commit/pull-request policy:** Git commit and merge are the default closeout actions when capability and policy permit; exact host PR policy is unknown and must be discovered before a PR-based closeout.
- **Reduced-capability fallback:** Use comparable file/diff evidence and record an explicit no-branch notice; owner is the orchestrator; next action is to restore or verify local branch capability before implementation.
- **Branch capability decision owner/next action:** Orchestrator; verify local branch creation before dispatching the first implementation WO.

## Project policy

### Commands

Commands are intentionally unknown unless verified in this checkout. The source
analysis mentions common Cargo commands, but that is not execution evidence.

- **Install/setup:** `not required` — the dependency-free workspace builds with the installed Rust toolchain.
- **Format:** `cargo fmt --all -- --check` — verified passed after formatter correction.
- **Lint/static analysis:** `cargo clippy --workspace --all-targets -- -D warnings` — verified passed.
- **Unit/integration tests:** `cargo test --workspace` — verified passed; 1 core unit test passed.
- **Build/package:** `cargo build --workspace` and `cargo check --workspace` — verified passed.
- **Run/manual verification:** `cargo run -p scriptcraft` — verified passed with `scriptcraft foundation ready`; visible-window smoke unavailable because no UI framework is selected.
- **Command evidence note:** WO-001 records exact output/results in its work-order artifact and the initiative execution log.

### Gates and intervention

- **Design approval:** human approval required by default at `review -> execute`.
- **Validation cadence:** `per work order`; fresh validation agent immediately after each implementation.
- **Manual/external gates:** none configured yet; any required desktop smoke/manual check must be discovered and recorded before it becomes a hard-stop policy gate.
- **Automation bypasses:** none.
- **Network or destructive-action restrictions:** No remote is known; do not require network access. Avoid destructive filesystem or Git actions without explicit authorization and recorded evidence.
- **User confirmation:** consolidated at initiative-level/closeout if Forge permits; this does not bypass design review, per-WO validation, or closeout evidence.

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

- **Unresolved decisions:**
  - Desktop OS/framework/package layout — owner: workspace-foundation implementer with user confirmation if a consequential choice remains; trigger: before first implementation; next action: inspect repository and verify a runnable baseline.
  - Exact native project extension and PDF implementation — owner: product/technical owner; trigger: persistence/export design; next action: select and record after core model boundary is validated.
  - Local branch capability and remote/PR policy — owner: orchestrator; trigger: first initiative; next action: verify before dispatch.
  - Requirements/blueprint independent reviews — owner: orchestrator; trigger: before initialization can be marked complete under the process; next action: dispatch fresh-context reviews without claiming results in this baseline.
- **Recommended first initiative:** `workspace-foundation` — establish the smallest runnable/verifiable Rust desktop workspace and headless development loop, then prove one end-to-end screenplay writing slice. Rationale: current source/package baseline is not verified, and the durable product baseline must be exercised before feature expansion.
- **Recommended scope:** repository discovery, chosen manifest/package structure, baseline checks, minimal core/app seam, and one smoke path; do not add DOCX/FDX, collaboration, MCP, planning, broad formatting, or web polish.
- **Validation boundary:** verified baseline command(s), headless semantic smoke path, and a desktop startup/manual path once the actual app entry point exists. Exact commands are currently unknown.
- **Next initiative or roadmap handoff:** after the foundation, replan product MVP slices from `REQ-WRITING-LOOP-001`; do not create an initiative directory or work orders as part of this initialization.

## Roadmap

- **Roadmap mode:** `open-ended replan`
- **Planned initiative count:** `1` planning target only (workspace foundation); later product stages are intentionally not pre-created.
- **Count/sequence approved by:** user-approved first-initiative recommendation; later sequence not approved.
- **Approval date or status:** `2026-10-25`; recommendation only, no initiative created.
- **Roadmap revision rule:** revise after each initiative while preserving prior decisions and rationale.

| Stage | Slug | Purpose | Dependency | Observable outcome | Validation boundary | Product MVP relationship | Status |
|---|---|---|---|---|---|---|---|
| 1 | `workspace-foundation` | Establish runnable Rust desktop/headless workspace and development loop | none | A verified starter workspace can build/test/run its smoke path | Verified commands plus headless and desktop smoke evidence | Workspace foundation enabling the product MVP | recommended; not created |

## Completion evidence

- Product overview: [`requirements/product-overview.md`](requirements/product-overview.md)
- Technical context: [`blueprints/technical-context.md`](blueprints/technical-context.md)
- Project policy: this manifest's Project policy section
- First-initiative recommendation and roadmap: this manifest's Decisions and handoff/Roadmap sections
- Requirements review findings: **not performed in this initialization; no review is claimed; fresh-context review remains a prerequisite before status can become `complete`**
- Blueprint review findings: **not performed in this initialization; no review is claimed; fresh-context review remains a prerequisite before status can become `complete`**
- Validation evidence and gate outcomes: **initialization validation not performed; Git evidence is recorded above; command and product smoke evidence remain placeholders for the first initiative**
- Completion evidence placeholder: `[add requirements-review and blueprint-review links here when independently produced]`

## Revision history

| Date | Changed answer | Previous answer | New answer | Why | Owner |
|---|---|---|---|---|---|
| 2026-10-25 | Initialization baseline | Template TODOs | Existing ScriptCraft baseline with guided Rust desktop screenplay defaults, standard initiative mode, per-WO fresh validation, Git `master`, no known remote, and explicit command unknowns | User-approved initialization inputs and repository inventory | Forge initialization agent |
| 2026-10-25 | Initialization status | `draft` | `blocked` pending independent requirements/blueprint reviews and verified non-Git command evidence | Do not claim reviews or validation that have not happened | Forge initialization agent |

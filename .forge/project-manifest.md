# Project Manifest

This is ScriptCraft's durable project-specific contract. It records project intent,
policy, unresolved decisions, and a dated repository baseline without treating
research notes or implementation observations as product mandates.

## Metadata

- **Status:** `complete` — initialization reviews and historical baseline evidence are linked; review warnings remain follow-ups.
- **Path type:** `existing`
- **Initialized on:** `2026-10-25`
- **Last revised:** `2026-11-07`
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
- **Framework:** User-approved for UI Icons v1 on 2026-11-04: desktop migration to eframe/egui 0.34 and egui_material_icons 0.6.0 with filled and outline features. The implementation manifest now requests that framework/icon set; Cargo lockfile resolution selected eframe 0.34.3. This is scoped initiative approval, not a permanent framework mandate. rfd remains unchanged.
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

### UI Icons WO-001 implementation evidence — 2026-11-04

The user-authorized UI Icons initiative is executing on the shared local branch
`dev/2026-11-04-ui-icons`, created from the observed `master` checkout after
preserving its pre-existing tracked and untracked work. WO-001 updates the
desktop framework/icon dependencies and adapts desktop-only eframe/egui APIs;
the lockfile was generated by dependency resolution, which selected 437 packages,
including eframe 0.34.3 and egui_material_icons 0.6.0. No check, build, test,
lint, formatting command, manual verification, agent review, or validation was
run. All review and validation remain user-owned under the authoritative policy
above. Dependency resolution is setup evidence only and is not compilation or
behavior evidence.

### Native Material icon font WO-001 implementation submission — 2026-11-05

The native Material icon-font work is on the observed initiative branch
`dev/2026-11-05-toolbar-item-model`; the pre-existing approved change to the
initiative plan was preserved. WO-001 adds the pinned Material Symbols Rounded
variable TTF and Apache-2.0 license record, then registers outlined and filled
weight-200 faces additively after Courier Prime. The asset provenance and SHA-256
are recorded in [`fonts/MaterialSymbols/README.md`](../fonts/MaterialSymbols/README.md).
Implementation submission is not build or runtime evidence. No agent check,
build, test, lint, formatting, manual verification, review, or validation was run;
user review and validation remain pending.

### Native Material icon font WO-002 implementation submission — 2026-11-05

WO-002 adds typed weight, optical-size, and grade selections with named defaults
and optional per-icon overrides in the desktop toolbar catalog. The app registers
all seven standard weights at the default optical-size/grade pair and only adds
separate faces for non-default optical-size/grade combinations explicitly used
by catalog icons. Toolbar, overflow, and status icons now share the app-owned
rich-text adapter, with status icons retaining the defaults. These source and
documentation changes are submitted for user review; no agent build, check, test,
lint, formatter, manual verification, review, or validation was run. User review
and validation remain pending.

The subsequent toolbar-presentation follow-up separates Elements role-option text
sizing from the general toolbar item text setting and adds independent left/top/
right/bottom padding for toolbar icon hover/click rectangles. The user-owned
source values were preserved; the current Elements item and role-option sizes are
11 and 14 points, with 12 points of horizontal role-row padding per side. egui's
overflow menu button still requires symmetric padding, so its per-axis value is
derived from opposing-edge averages.
These changes and their durable requirement/blueprint updates are submitted for
user review. The central Elements ComboBox also replaces its default triangle
with the outlined Material Unfold More glyph through egui's custom icon painter,
while retaining framework-owned selection and popup behavior; the icon reference
records this assignment. No agent validation was run.

### Appearance theme consolidation WO-001/WO-002 implementation submission — 2026-11-05

The approved appearance plan is being implemented sequentially in the desktop
presentation layer. WO-001 adds `apps/scriptcraft/src/theme.rs` as the central
owner of transient System/Day/Night/Midnight resolution, app semantic palettes,
and egui visuals generation. Toolbar, Script Workspace, and status surfaces now
consume separate palette groups; existing item/icon and status-local foreground
override precedence is retained. WO-002 applies estimated Day/Night/Midnight
neutral values from the supplied references, retains existing selection, caret,
IME, hidden-text, warning, error, generic-selection, and text-cursor values, and
adds explicit page-selection-boundary painting. The zoom slider rail is scoped
to its status palette while its trailing fill remains tied to egui selection.
The exact token table and work-order handoff are recorded in
[`appearance-theme-consolidation.md`](../.plan/2026-11-05-appearance-theme-consolidation/appearance-theme-consolidation.md).

Implementation is submitted for user review. No agent build, check, test, lint,
formatting command, manual verification, review, contrast check, or validation
was run. Screenshot RGB values are initial visual estimates and remain pending
side-by-side user calibration; implementation, compilation, rendering, and
accessibility outcomes are not claimed as verified or accepted.

### Toolbar-to-Script View residual-gap WO-012 implementation submission — 2026-11-06

On the existing `dev/2026-11-06-script-view-boundary` branch, WO-012 changes the
closed Elements toolbar composition so vertical item spacing is suppressed before
the ComboBox allocation, then restored inside its popup before role-row creation.
This prevents the explicit caption allocations from inheriting the ComboBox's
post-widget cursor spacing. The current explicit toolbar bottom padding was
preserved as directed; the durable requirement and blueprint now allow explicitly
owned trailing padding while disallowing implicit toolbar gaps and child-response
overflow. Source reasoning predicts the toolbar response will remain within its
configured panel boundary, but no runtime result is claimed.

At the user's later explicit request, a coding agent ran only
[`cargo check --workspace`](../.plan/2026-11-06-toolbar-script-view-gap/work-orders/WO-012-elements-spacing-overflow.md:52).
It exited successfully (code 0; reported completion in 0.22 seconds) and emitted
four warnings: future-incompatible `float_literal_f32_fallback` at
[`script_view.rs`](../apps/scriptcraft/src/script_view.rs:239), and dead-code
warnings for `Size20`/`Size48` at [`material_font.rs`](../apps/scriptcraft/src/material_font.rs:60),
`Minus50`/`Plus200` at [`material_font.rs`](../apps/scriptcraft/src/material_font.rs:85),
and `Filled` at [`toolbar_items.rs`](../apps/scriptcraft/src/toolbar_items.rs:25).
No deprecation warning was reported. No tests, Clippy, formatting, additional
build/check, or manual runtime verification was run, and the coding agent made no
source edits. This is a one-off user-authorized exception, not a change to the
repository-wide user-owned review/validation policy. User review, warning
disposition, and runtime visual acceptance remain pending. Findings and the full
warning summary are recorded in
[`toolbar-script-view-gap-findings.md`](../.plan/2026-11-06-toolbar-script-view-gap/toolbar-script-view-gap-findings.md)
and [`WO-012-elements-spacing-overflow.md`](../.plan/2026-11-06-toolbar-script-view-gap/work-orders/WO-012-elements-spacing-overflow.md).

### Menu system WO-001–WO-010 implementation submission — 2026-11-06

The menu-system work was submitted sequentially on the existing local branch
`dev/2026-11-06-menu-system`; source inspection for WO-010 found that branch checked
out with the initiative's existing modified and untracked work present. No new
branch or commit was created by WO-010. The desktop manifest records direct
`arboard = "3.7.0"`; the macOS-target manifest records
`muda = { version = "0.17", default-features = false }`, and the existing lockfile
resolves `muda` to 0.17.2 and includes both dependencies for ScriptCraft. These are
dependency/lockfile facts, not compilation or runtime evidence.

WO-001–WO-009 added the shared desktop command dispatcher, canonical clipboard
adapter, declarative File/Edit catalog, in-window renderer and source preview
switch, macOS native menu/event adapter, and shared queue/accelerator arbitration.
WO-010 adds durable requirement and technical-context records and reconciles the
initiative/work-order handoffs with that source. The switch remains enabled;
macOS is configured for native plus in-window preview, while Windows uses the
in-window menu. Native macOS is the sole accelerator owner while installed; the
`muda` 0.17.2 event handler feeds an app-drained ID inbox because handler
registration suppresses `MenuEvent::receiver()` delivery. Initial menu metrics,
canonical clipboard behavior, module boundaries, and known limitations are
recorded in the linked initiative plan and technical context.

This is implementation submission only. No build, check, test, lint, formatting,
application launch, manual verification, review, or independent validator was run
for this initiative under repository-wide user-owned validation policy. No
automated desktop-test target was added, and no prior work order is marked
validated or accepted. User evidence and acceptance remain pending, including
macOS native/dual-preview and Windows in-window behavior, command/shortcut and
clipboard/focus/modal interactions, layout/appearance and viewport-scale review,
and any desired workspace checks. The known pre-existing New dirty-session
consent discrepancy is not claimed fixed. Automatic production OS-only switching
and disabling the preview remain deferred to a separately approved change.

The user subsequently reported E0282 at `MenuEvent::set_event_handler` in
[`macos_menu.rs`](../apps/scriptcraft/src/macos_menu.rs), because the closure
parameter type was unknown at `event.id`. The callback was narrowly annotated as
`move |event: MenuEvent|` using the imported type from `muda 0.17.2`. No eframe/
egui version or dependency changed: eframe remains 0.34, locked at 0.34.3. This
records the user-provided compiler evidence and source correction only; no build
or other validation was run, and compilation is not claimed to pass.

### Menu-system WO-011–WO-012 implementation submissions — 2026-11-06

WO-011 and WO-012 were implemented sequentially on the existing
`dev/2026-11-06-menu-system` branch after explicit user authorization; existing
work was preserved and no branch or commit was created. The desktop package's
macOS-target dependencies are `objc2 = 0.6.5`, `objc2-app-kit = 0.3.2`, and
`objc2-foundation = 0.3.2` with the narrow feature sets recorded in
[`apps/scriptcraft/Cargo.toml`](../apps/scriptcraft/Cargo.toml). They match the
objc2 generation used by `muda = "0.17"`, locked at 0.17.2. The lockfile records
the direct ScriptCraft dependency edges. These dependencies were resolved/fetched
as WO-011 setup evidence only; WO-012 adds no dependencies, and no compilation is
claimed. Eframe/egui remain unchanged at the already recorded versions.

The native adapter uses AppKit system-symbol images for File New `plus`, Open
`arrow.up.right`, Close `xmark`, Save `square.and.arrow.down`, Save As
`square.and.arrow.down.on.square`, and Edit Undo `arrow.uturn.backward`, Redo
`arrow.uturn.forward`, Cut `scissors`, Copy `doc.on.doc`, Paste
`doc.on.clipboard`. Save As PDF, Delete, and Select All have no native images.
Images are requested as 18-point templates; the macOS-only adapter retains the
installed root and resolved item references on the main thread for the native
controller lifetime. The sole unsafe operation is the locally allowed retain of
muda's borrowed root AppKit handle; no raw Objective-C message sends or broader
unsafe allowance were added. Missing symbols or item paths are optional and emit
at most ten bounded static catalog-ID/symbol warnings during installation. Muda
continues to own labels, enablement, commands, events, and item identity; no
per-frame image assignment or hierarchy lookup is introduced. The egui preview
remains enabled and its Material icon assignments are unchanged.

Both work orders are implementation submissions only, not accepted or validated.
No build, check, test, lint, formatting command, launch, manual verification,
review, or independent validator was run under repository policy; no automated
desktop tests were added. Compilation, target OS symbol availability, native menu
resolution, visual Light/Dark and disabled-state appearance, startup fallback,
command/shortcut/wakeup behavior, and in-window preview interaction remain
unverified and pending user evidence. Historical WO-011 notes about the former
limited named-image path and the then-unstarted WO-012 describe their original
handoff state; the completed SF Symbols mapping above supersedes those interim
implementation facts. User review and acceptance remain pending.

### Menu-system WO-013 complete catalog implementation submission — 2026-11-06

WO-013 extends the existing menu-system checkout on `dev/2026-11-06-menu-system`;
the user-directed existing branch was preserved and no branch or commit was created.
The shared desktop catalog now carries ten ordered reference menus and distinguishes
real `AppCommand` rows from disabled placeholders without adding no-op commands.
The existing thirteen File/Edit command rows retain their mapped behavior and icon/
shortcut metadata; 132 other rows are disabled placeholders: 99 ordinary rows, 30
disclosure-only rows, two statically checked rows, and one disabled Help Search label.
All 33 newly cataloged placeholder shortcuts are display/native-accelerator metadata
only and are excluded from egui raw-input matching, the native ID-to-command map,
the pending command queue, and dispatch. Placeholder icons remain unassigned. The
native Application menu remains native-only and separate from the ten shared menus.

No dependencies, lockfile entries, core behavior, toolbar/status behavior, or production
preview policy changed. Existing egui menu height/popup bounds, icon/label/shortcut
alignment, hover switching, and enabled File/Edit behavior were preserved by source
adaptation; these are not runtime or visual verification claims. Repository policy
prohibited and no agent build, check, test, lint, formatting command, launch, manual
verification, review, or independent validator was run. Both backend compilation and
all native/in-window rendering, accessibility, disabled activation, and shortcut
behavior remain user-unverified. This implementation submission is not acceptance.

The user subsequently explicitly authorized Git closeout for the complete initiative:
commit all current changes on `dev/2026-11-06-menu-system` and merge that branch back
to `master`. Preserve all current work, including untracked initiative/reference files,
and use ordinary non-destructive Git behavior only; do not force, rewrite history, use
a remote, or create a pull request. If master has diverged or conflicts arise, stop
without overriding either side. The resulting implementation commit, merge form, and
final master HEAD are evidenced by Git history after the operations; this authorization
record does not invent a hash or claim the operations have already completed. This
closeout does not waive the repository-wide user-owned validation policy: no agent
build, check, test, lint, formatting, launch, review, or validator is authorized here,
and implementation submission remains unaccepted and unvalidated.

### Hotkeys and consolidated command layer WO-001–WO-008 implementation submission — 2026-11-06

The user-directed initiative handoff identifies the existing local branch as
`dev/2026-11-06-hotkey-command-layer`; this is user-supplied branch attribution, not a
fresh Git observation. WO-008 reconciles the internal command architecture and its user-owned
validation handoff. The root workspace manifest lists `apps/scriptcraft`,
`crates/scriptcraft-commands`, and `crates/scriptcraft-core`. The desktop manifest depends on the
local command and core packages; `scriptcraft-commands` depends on local `scriptcraft-core` and exact
`serde = 1.0.229` with the `derive` feature. The lockfile/dependency facts recorded by WO-001 are
historical setup observations; WO-008 made no dependency, workspace, or lockfile change.

At that earlier hotkey-command-layer submission, WO-001–WO-007 supplied a 41-ID UI-independent typed command contract, headless editor executor,
desktop dispatcher, focused hotkeys, selection/navigation/focus routing, document/clipboard/modal/
viewport effects, and view/status/appearance/focus-mode/timer/notification routing. WO-008 updates the
product overview, writing-loop requirements and technical context and records a source inventory. One
document-open TextState Selection restoration path that directly called the replacement session setter
was routed through the existing `editor.selection.set` command and headless executor; invalid restored
selection retains the existing caret-0,0 fallback. The inventory found no other enabled
ScriptCraft-owned screenplay mutation path outside the dispatcher/executor. Native predefined macOS
About/Hide/Hide Others/Show All/Quit remain OS-owned; picker paths that cannot be represented as UTF-8
remain rejected. Existing File New dirty-session consent and File Close confirmation discrepancies
remain outside scope. These are source/documentation findings, not execution or behavioral evidence.

Exact WO-008 changed paths are `apps/scriptcraft/src/main.rs`,
`.forge/requirements/features/screenplay-writing-loop.md`,
`.forge/requirements/product-overview.md`, `.forge/blueprints/technical-context.md`, this manifest,
`.plan/2026-11-06-hotkey-command-layer/hotkey-command-layer-v1.md`,
`.plan/2026-11-06-hotkey-command-layer/work-orders/WO-008-durable-contracts-and-validation.md`, and
`.plan/2026-11-06-hotkey-command-layer/work-orders/WO-008-user-validation-handoff.md`. Earlier
work-order changed-file lists remain in their historical handoffs.

No agent tests, builds, checks, Clippy, formatting, launch, manual verification, review, or independent
validation were run. Compilation, serialization, runtime/platform behavior, and every validation
scenario remain unverified. User review, validation, and acceptance remain pending; the detailed
scenario matrix is the [WO-008 user-validation handoff](../.plan/2026-11-06-hotkey-command-layer/work-orders/WO-008-user-validation-handoff.md).
No commit or push was made.

### UI indicators and platform shell WO-001 implementation submission — 2026-11-07

WO-001 was submitted on the local branch `dev/2026-11-06-ui-indicators-platform-shell`, created from
the observed `master` checkout at `20496cd` (`Merge hotkey command layer`). Before branch creation,
the working tree had no tracked modifications or staged changes; the sole untracked path was the
user-provided `.plan/2026-11-06-ui-indicators-platform-shell/` assessment directory. The destination
branch did not exist, and ordinary `git switch -c` preserved that directory and all checkout content.
No reset, stash, overwrite, commit, merge, or destructive Git operation was used.

WO-001 adds a pure final-filename title projection with the `Untitled Screenplay` fallback, startup
title configuration, a deduplicated `ViewportCommand::Title` adapter effect, and focused projection
tests. Existing successful New/Open/Save/Save As path transitions are the sole title source; their
failure/cancellation paths retain the active path and therefore the displayed title. Native window
decorations and OS-owned title placement remain unchanged. No persistence format, dependency, or
other work-order behavior changed. The initiative assessment, durable title requirement, technical
context, source changes, authored tests, exact changed-file list, and remaining user-owned lifecycle
acceptance scenarios are recorded in the [WO-001 implementation handoff](../.plan/2026-11-06-ui-indicators-platform-shell/work-orders/WO-001-dynamic-native-document-title-handoff.md).

This is implementation submission only. No tests, build, check, lint, formatting command, application
launch, manual verification, review, or independent validation was run; compilation and all native OS
title behavior remain unverified. User review, validation, and acceptance remain pending. WO-002 through
WO-007 have not been implemented by this submission. No commit or merge was made.

### UI indicators and platform shell WO-002 implementation submission — 2026-11-07

WO-002 was submitted on the already checked-out branch
`dev/2026-11-06-ui-indicators-platform-shell`, confirmed by the initial status inspection. The tracked
WO-001 source/manifest/requirements/technical-context edits and the pre-existing untracked initiative
assessment were preserved as the starting state. Work built on those files without switching branches,
stashing, resetting, committing, merging, or overwriting existing work.

The typed `scriptcraft-commands` catalog and descriptor list now contain 42 IDs, including
`ui.status.set`. Its optional `DiagnosticMessage` payload is a nonempty single line, limited to 512
UTF-8 bytes; the same validator is used by construction and Serde deserialization. No dependency or
lockfile change was made. The headless executor rejects this desktop-only effect. Desktop posts and
clears dispatch typed InternalFollowUp commands through the existing dispatcher, remain enabled
during the migration modal, and preserve FIFO order with a reentrancy guard. The command handler alone
writes the diagnostic field. Routine operation/development center messages were removed and the
interim center projection is Ready pending WO-004 live hints. Retained errors/warnings now appear only
inside the existing Diagnostics disclosure. Diagnostics is still off by default; its disabled state
intentionally hides errors and creates no Diagnostics row, body, focus target, or accessibility node.
Save failure categories/details and unchanged-document semantics remain privacy-safe; no document
content or path is posted. The existing WO-001 native-title work and migration modal warnings remain
in place.

Exact WO-002 changed paths are `crates/scriptcraft-commands/src/lib.rs`,
`crates/scriptcraft-commands/tests/editor_executor.rs`, `apps/scriptcraft/src/app_commands.rs`,
`apps/scriptcraft/src/main.rs`, `apps/scriptcraft/src/script_view.rs`,
`apps/scriptcraft/src/status_bar.rs`, `.forge/requirements/features/screenplay-writing-loop.md`,
`.forge/blueprints/technical-context.md`, this manifest,
`.plan/2026-11-06-ui-indicators-platform-shell/ui-indicators-platform-shell-v1.md`, and
`.plan/2026-11-06-ui-indicators-platform-shell/work-orders/WO-002-diagnostic-status-command-handoff.md`.
The focused catalog/bounds/headless, desktop set/replace/clear/order/modal, and Ready/default-gate
tests were authored but not run. No build, check, test, lint, formatting command, launch, manual
verification, review, independent validator, or other validation was performed. Compilation,
serialization round trips, Diagnostics-enabled rendering, save/open/clipboard failures, and runtime
behavior remain unverified; user review, validation, and acceptance remain pending. The explicit
handoff records coverage limits and acceptance scenarios. WO-003–WO-007 remain pending. No commit or
merge was made.

### UI indicators and platform shell WO-003 implementation submission — 2026-11-07

WO-003 was submitted on the already checked-out `dev/2026-11-06-ui-indicators-platform-shell`
branch, preserving the unvalidated WO-001/WO-002 submissions and all existing tracked/untracked
work. No branch switch, stash, reset, overwrite, commit, or merge was performed. Core editing now
exposes `EditingSession::paragraph_key_targets()`, returning `ParagraphKeyTargets` with typed Enter
and Tab `ParagraphKeyTarget::{Target(Role), NoOp, Unavailable}` outcomes. Both real Enter/Tab edits
and this nonmutating projection use the same shared selection/policy/generated-Parenthetical
resolver; mutation, preservation, capacity, history, dirty-state, and revision behavior remain in
the existing transactional edit path. Same-role Tab remains a displayable Target even though its
real command is a consumed no-op. No desktop behavior, dependency, lockfile, or command catalog
changed.

Exact WO-003 changed paths are `crates/scriptcraft-core/src/editing.rs`,
`crates/scriptcraft-core/tests/paragraph_key_projection.rs`,
`.forge/requirements/features/screenplay-writing-loop.md`,
`.forge/blueprints/technical-context.md`, this manifest,
`.plan/2026-11-06-ui-indicators-platform-shell/ui-indicators-platform-shell-v1.md`, and
`.plan/2026-11-06-ui-indicators-platform-shell/work-orders/WO-003-shared-paragraph-key-target-projection-handoff.md`.
Focused coverage was authored for all 17 roles with empty/nonempty text, strict and relaxed
boundaries, UTF-8 byte offsets, noncollapsed selection, Dialogue interior/end, Character blank-only
Tab, same-role outcomes, and Parenthetical delimiters, repair, generated provenance, and disabled
automatic Parentheticals, with real-command comparisons where meaningful. Tests were not run. No
build, check, test, lint, formatting command, launch, manual verification, review, independent
validator, or other validation was performed. Compilation and all runtime/editing behavior remain
unverified; user review, validation, and acceptance remain pending. Later WO-004 must consume this
API rather than reproduce its transition rules. WO-004–WO-007 remain pending; no commit or merge was
made.

### UI indicators and platform shell WO-004 implementation submission — 2026-11-07

WO-004 was submitted on the existing user-designated `dev/2026-11-06-ui-indicators-platform-shell`
branch, preserving WO-001–WO-003 and all tracked/untracked work. No branch switch, stash, reset,
overwrite, commit, or merge was performed. No dependency, lockfile, core editing behavior, command
catalog, or Enter/Tab/element command routing changed.

The desktop status center now projects Ready, shared-core paragraph-key targets at collapsed element
ends, or held element-choice lists derived from the shared shortcut binding table. Exact host chords,
focus, selection, IME/current-frame IME, modal state, release restoration, role labels, and modifier-only
repaint are wired into the projection. Modifier lists use a bounded measured font-fit policy and retain
full accessible text while clipping if their minimum-size layout exceeds available center width. The
migration modal retains only a noninteractive Ready status footer; its existing other application
panels and controls remain suppressed. WO-002's Diagnostics-off source gate remains unchanged.

Exact WO-004 changed paths are `apps/scriptcraft/src/status_bar.rs`,
`apps/scriptcraft/src/shortcuts.rs`, `apps/scriptcraft/src/main.rs`,
`.forge/requirements/features/screenplay-writing-loop.md`,
`.forge/blueprints/technical-context.md`, this manifest,
`.plan/2026-11-06-ui-indicators-platform-shell/ui-indicators-platform-shell-v1.md`, and
`.plan/2026-11-06-ui-indicators-platform-shell/work-orders/WO-004-live-cursor-modifier-guidance-handoff.md`.
Focused source tests were authored for paragraph end/empty/interior and all roles, chord/list mapping
on both platforms, exact modifiers, focus/IME/modal/selection/release, modifier-only transitions, and
status-list fitting/geometry. They were not run. No build, check, test, lint, formatting command,
launch, manual verification, review, independent validator, or other validation was performed.
Compilation, native modifier delivery, rendering/accessibility, modal footer presentation, and all
runtime behavior remain unverified; user review, validation, and acceptance remain pending. The
implementation handoff records the visual-width limitation and user-owned scenarios. WO-005–WO-007
remain pending; no commit or merge was made.

### UI indicators and platform shell WO-005 implementation submission — 2026-11-07

WO-005 was implemented on the already checked-out `dev/2026-11-06-ui-indicators-platform-shell`
branch. The existing WO-001–WO-004 source/document submissions, other uncommitted edits, and untracked
initiative contents were preserved. No branch switch, reset, stash, overwrite, commit, or merge was
performed; no dependency or lockfile changed.

This current work-order submission supersedes earlier UI-menu evidence in this manifest that described
macOS dual preview and the source-only preview switch; those earlier entries remain historical records
of the prior implementation state, not current menu policy.

The source-only dual-preview switch has been replaced by the app-owned `RuntimeMenuPolicy` in
`apps/scriptcraft/src/menu.rs`. `MenuPlatformChoice::{Auto, MacOS, Windows}` and the pure
`RuntimeMenuPolicy::resolve(host, choice)` seam are ready for a later CLI adapter but do not parse CLI
arguments. Auto selects native macOS only, in-window only on Windows, and no menu backend on other
targets. A Windows-style in-window choice is policy-supported on any target; native macOS choice is
rejected off macOS. The selected policy is stored in `ScriptCraftApp` and gates native construction,
in-window panel composition, native event draining and availability synchronization, and raw-input
accelerator extraction. Exactly one backend owns menu accelerators when active; in-window matching
continues to use host shortcut semantics. macOS native installation failure is contextualized and
aborts startup rather than silently presenting an incomplete or multiply-owned menu UI. Shared
catalog, placeholders, command FIFO/dispatcher, availability, focus behavior, and unrelated host
keyboard/platform behavior were not intentionally changed. Unsupported targets do not compile or
construct native menu modules.

Exact WO-005 changed paths are `apps/scriptcraft/src/menu.rs`, `apps/scriptcraft/src/main.rs`,
`.forge/requirements/features/screenplay-writing-loop.md`, `.forge/blueprints/technical-context.md`,
this manifest, `.plan/2026-11-06-ui-indicators-platform-shell/ui-indicators-platform-shell-v1.md`, and
`.plan/2026-11-06-ui-indicators-platform-shell/work-orders/WO-005-runtime-os-default-menu-policy-handoff.md`.
Focused source tests were authored for host defaults, explicit supported/unsupported requests, sole
renderer selection, and accelerator ownership. They were not run. No tests, build, check, lint,
formatting command, launch, manual verification, review, independent validator, or other validation
was performed. Compilation, native installation, runtime menu event/shortcut behavior, rendering,
focus restoration, and native startup-error presentation remain unverified; user review, validation,
and acceptance are pending. Exact API notes, coverage boundaries, risks, and user scenarios are in
the WO-005 implementation handoff. WO-006 and WO-007 remained pending at the time of that submission.
No commit or merge was made.

### UI indicators and platform shell WO-006 implementation submission — 2026-11-07

WO-006 is submitted on the user-designated existing branch
`dev/2026-11-06-ui-indicators-platform-shell`, preserving the pre-existing WO-001–WO-005 and all
tracked/untracked work. No branch switch, stash, reset, overwrite, commit, or merge was performed; no
dependency or lockfile changed.

The desktop startup parses `--menu-platform=auto|macos|windows` using the standard library and resolves
the WO-005 host policy before eframe launch or native-menu construction. The selection is process-only.
Malformed, duplicate, unknown, positional, non-UTF-8, and unsupported native-macOS choices produce
usage errors and exit before launch. On macOS the Windows override selects only the in-window egui
renderer and its accelerator owner; it does not install the native menu controller. The host is not
spoofed: editor shortcut/status labels, menu input modifier resolution, document-point conversion,
native scale, file dialogs, clipboard, notifications, window controls, and document behavior remain
host-specific. Windows-style rendering is a development simulation, not OS emulation; its `Ctrl`
annotation does not change macOS primary input from Command. Existing Auto behavior and supported
other-target Windows-style in-window selection remain as specified. No parser dependency, settings UI,
or persisted field was added.

Exact WO-006 changed paths are `apps/scriptcraft/src/main.rs`,
`.forge/requirements/features/screenplay-writing-loop.md`, `.forge/blueprints/technical-context.md`,
this manifest, `.plan/2026-11-06-ui-indicators-platform-shell/ui-indicators-platform-shell-v1.md`, and
`.plan/2026-11-06-ui-indicators-platform-shell/work-orders/WO-006-cli-menu-platform-development-override-handoff.md`.
The main source authors focused argument parser, host-policy rejection, renderer selection, and unique
accelerator-owner cases; policy/ownership cases also remain in `menu.rs`. All are authored but unrun.
No build, check, test, lint, formatter, launch, manual verification, review, or independent validation
was performed. Compilation, real process exit output, native/in-window rendering, keyboard delivery,
and all runtime/platform interactions remain unverified. User review, validation, and acceptance remain
pending. The handoff records usage, deterministic failures, host-modifier limitations, coverage
boundaries, and user-owned acceptance scenarios. WO-007 remains pending; no commit or merge was made.

### UI indicators and platform shell WO-007 documentation submission — 2026-11-07

WO-007 reconciles the durable product overview, writing-loop requirements, technical context, initiative
record, and manifest against the WO-001–WO-006 implementation handoffs. Those six orders remain
implementation submissions only: **none is accepted or validated**, all reported authored tests remain
unrun, and user review/validation/acceptance are pending. The 42-ID total is the earlier 41-command
catalog plus WO-002's `ui.status.set`; there is no dependency or lockfile change reported. The center is
Ready/cursor/modifier guidance only. Diagnostics is intentionally disabled by default, so errors assigned
there are hidden when the gate is off. Runtime menu defaults are native-only on macOS, in-window-only on
Windows, and no backend on other targets; the transient CLI override and macOS Windows-style Ctrl
annotations do not change Command-based host accelerators or other host OS integrations. Native title
placement remains OS-owned without a cross-platform centering guarantee.

The reported initiative branch is `dev/2026-11-06-ui-indicators-platform-shell`. WO-007 performed no
branch switch, stash, reset, overwrite, commit, merge, test, build, check, lint, formatting command,
launch, manual verification, agent review, or validation. The branch/worktree facts are inherited from
the prior handoffs; no fresh Git inventory is claimed. Existing tracked and untracked work is to remain
preserved. No source, tests, dependency, or lockfile was changed by WO-007.

Exact WO-007 changed paths are `.forge/requirements/product-overview.md`,
`.forge/requirements/features/screenplay-writing-loop.md`, `.forge/blueprints/technical-context.md`,
`.forge/project-manifest.md`, `.plan/2026-11-06-ui-indicators-platform-shell/ui-indicators-platform-shell-v1.md`,
`.plan/2026-11-06-ui-indicators-platform-shell/work-orders/WO-007-durable-contracts-and-user-validation-handoff.md`,
and `.plan/2026-11-06-ui-indicators-platform-shell/work-orders/WO-007-user-validation-handoff.md`.
The consolidated handoff leaves workspace format/check/build/Clippy/test commands for the user to run and
includes macOS/Windows lifecycle, error/diagnostics visibility, status/input, menu/CLI/accelerator, and
regression scenarios. WO-002's lack of concrete Serde codec round-trip coverage, all unverified runtime
behavior, possible narrow-width modifier-list clipping, and native installation error presentation are
explicit pending evidence/risks, not invented outcomes. No validation or acceptance is claimed; no
commit or merge was made.

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

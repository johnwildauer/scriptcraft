# Work Order: WO-001 — Toolbar Extraction

## Metadata

- **Status:** `in-review`
- **Assignee:** Fresh dedicated desktop application implementer session
- **Phase/order:** 1 of 3; foundational; no predecessor; WO-002 and WO-003 depend on the extracted toolbar boundary and preserved behavior
- **Initiative:** [`status-bar-v1.md`](../status-bar-v1.md)
- **Generated filename:** [`WO-001-toolbar-extraction.md`](WO-001-toolbar-extraction.md)
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`validation-WO-001.md`](../validations/validation-WO-001.md) (future artifact; not created)
- **Fixes:** none
- **Implementer host subtask/dispatch ID:** `01a182ea-f5f2-76bf-8374-d98053293286` (actual host dispatch identifier for the fresh implementer subtask; this is not an internal session/agent ID, which is unavailable).
- **Session exception:** none

## Execution evidence

### Dispatch and start — 2026-10-28 14:31 UTC

- **Actor/role:** Fresh dedicated WO-001 implementer (this session).
- **Host subtask/dispatch ID:** `01a182ea-f5f2-76bf-8374-d98053293286` (host-provided identifier; no internal session/agent ID is claimed).
- **State transition:** `draft` → `in-progress`.
- **Authorization:** User explicitly authorized execution, commit, and light independent verification without manual per-WO approvals; this dispatch is limited to WO-001.
- **Preflight:** Rechecked `git status --short --branch`, `git rev-parse HEAD`, and `git branch --show-current`: `dev/2026-10-28-status-bar`, HEAD `83621f34f5db17b3060b7c28fce795decd903581`, and only `.forge/initiatives/2026-10-28-status-bar/` untracked. No unrelated work was present; no staging, commit, reset, stash, or branch change was performed.
- **Execution boundary:** Implement toolbar extraction and preservation tests only. WO-002/WO-003 are not started. Requirements and the desktop blueprint remain `verify unchanged` and will be checked against the final diff.
- **Dispatch-time result:** Implementation started; checks and changed-file inventory were to be recorded at completion. The native desktop smoke is recommended, not a configured hard-stop gate; availability/result would be recorded honestly. See the completion section below for the final result.

### Implementation completion and validation handoff — 2026-10-28 14:37 UTC

- **Actor/role:** Fresh dedicated WO-001 implementer (this session).
- **Host subtask/dispatch ID:** `01a182ea-f5f2-76bf-8374-d98053293286` (host-provided identifier; distinct from an internal session/agent ID, which is unavailable).
- **State transition:** `in-progress` → `in-review`; initiative remains `execute` / `active` until the orchestrator dispatches the separate validator.
- **Implementation:** Extracted the typed toolbar model, default composition, allocation/measurement/overflow policy, geometry and presentation helpers, and `ScriptCraftApp` toolbar rendering/action/role integration into the desktop-owned [`toolbar.rs`](../../../../apps/scriptcraft/src/toolbar.rs). [`main.rs`](../../../../apps/scriptcraft/src/main.rs) retains app state and top-panel composition. Existing focused toolbar tests were retained and exercise the extracted boundary.
- **Changed files (complete):**
  - `apps/scriptcraft/src/toolbar.rs` — created; extracted toolbar implementation.
  - `apps/scriptcraft/src/main.rs` — toolbar module declaration/import and app wiring; non-toolbar diagnostics/reveal state remain in the app module.
  - `.forge/initiatives/2026-10-28-status-bar/work-orders/WO-001-toolbar-extraction.md` — dispatch and completion evidence.
  - `.forge/initiatives/2026-10-28-status-bar/execution-log.md` — dispatch and completion events.
- **Acceptance coverage:** Existing deterministic tests passed for default typed order and metadata; measured label widths, fixed overrides and 72-point Space; equal flexible-space distribution; wide/exact/narrow allocations, overflow and execution; action/session and transactional role routing/rejection preservation; hover/cursor/focus ownership; Elements label/control boundaries; and clipped-row/right-edge geometry. No status-bar, page indicator, zoom, core, schema, or unrelated behavior was added.
- **Durable documents:** Requirements impact `verify unchanged`; screenplay toolbar requirements were checked against the extracted code and retained tests. Blueprint impact `verify unchanged`; desktop toolbar ownership remains accurate. Neither requirements nor blueprint files changed.
- **Commands and actual results:**

  | Command | Result |
  |---|---|
  | `cargo fmt --all` | Exit 0; formatter applied. |
  | `cargo fmt --all -- --check` | Exit 0; passed. |
  | `cargo test -p scriptcraft toolbar_` | Exit 0; 28 passed, 0 failed, 49 filtered out. Passed before and after the warning cleanup. |
  | `cargo check --workspace` (first run) | Exit 0, but emitted three dead-code warnings for the intentionally retained non-default typed `Space` model/constructor/width. These were narrowly documented with `allow(dead_code)` and retained tests. |
  | `cargo check --workspace` (rerun) | Exit 0; passed with no warnings. |
  | `cargo test --workspace` | Exit 0; all 159 tests passed, 0 failed (77 app tests, 45 core unit tests, 37 core integration tests; doc tests: 0). |
  | `cargo build --workspace` | Exit 0; passed. |
  | `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0; passed. |
  | `git diff --check` | Exit 0; passed for tracked changes. |
  | Whitespace scan of untracked changed source/evidence files | Passed; three files scanned, no trailing spaces/tabs. Initiative artifacts were not staged. |

- **Limitations:** Recommended native desktop visual/input smoke at normal and narrow widths was unavailable in this tool surface; it is not a configured project hard-stop gate and is not claimed as passed. No independent validation artifact exists yet; the separate fresh validator remains the next workflow step.
- **Git/evidence boundary at implementation completion:** Branch `dev/2026-10-28-status-bar`, HEAD `83621f34f5db17b3060b7c28fce795decd903581`, tracked modification only to `main.rs`, and untracked new `toolbar.rs` plus the pre-existing nine-file initiative artifact set. Within that set only this WO and `execution-log.md` were modified. The later orchestrator commit-preflight inventory and exact candidate paths are recorded in the execution log; no unrelated paths were present.
- **Known check note:** The first successful `cargo check --workspace` emitted the three warnings above; the warning cleanup was limited to preserving the non-default Space model intentionally. The clean check, full tests, build, strict clippy, formatter check, and diff check all passed afterward. No required gate failed or remains unavailable.
- **Next action/owner:** Orchestrator stages/commits only the explicitly authorized WO-001 candidate set as separately authorized, then dispatches a fresh independent WO-001 validator. Do not begin WO-002 before validation passes.

## Summary

Extract the existing toolbar presentation into discrete desktop-owned files, preserving its present behavior and contracts. This work order is toolbar extraction and preservation only; all status-bar behavior and viewport-page presentation belong to WO-002.

## In scope

- Extract the current toolbar model, renderer/allocation helpers, and app integration from [`main.rs`](../../../../apps/scriptcraft/src/main.rs) into a small desktop module/file boundary (for example, `toolbar.rs`) without changing user-visible behavior. Keep the existing typed model and current app command/session adapters.
- Preserve existing toolbar composition, width/allocation, overflow, action/role routing, hover/cursor/focus, and clipped-edge behavior without adding commands or changing user-visible behavior.
- Add or retain focused tests for toolbar order, measured widths/overrides, flexible-space allocation, narrow overflow and execution, action/session routing, role choices, hover/cursor/focus ownership, and clipped-edge bounds.

## Out of scope

- Any status-bar item/model/renderer, status state or presentation, or viewport-page behavior (WO-002).
- Zoom state/widgets, page scaling, horizontal editor navigation or saved preferences (WO-003).
- Adding toolbar commands, status actions/icons, or changing command, edit, save/open/export, role, hover, cursor, focus, overflow or width behavior.
- Changes to core model/session/layout, native schema/project data, page profile, fixed pagination/source mapping, PDF/export geometry, titlebar, theme-wide styling, or unrelated editor behavior.
- Shared generic toolbar/status framework absent a demonstrated concrete need.
- Editing durable requirements/blueprints in this planning task; existing toolbar requirements and desktop ownership will be verified unchanged during later approved execution unless review-authorized evidence justifies otherwise.

## Requirements

Applicable existing requirements (verbatim):

- **REQ-WRITING-LOOP-005 / AC-WRITING-005.1:** “Given the desktop app is running, the toolbar shall present the typed default Item order New, Open, Save, Save As, Flexible Space, Elements, Flexible Space, and Export PDF.”
- **REQ-WRITING-LOOP-005 / AC-WRITING-005.2:** “Given an actionable toolbar Item is activated, the desktop shall route it through existing application/session adapters and preserve command safety, status, persistence, and fixed-layout PDF behavior.”
- **REQ-WRITING-LOOP-005 / AC-WRITING-005.3:** “Action Items shall use an icon-first, borderless icon control with a small explicit padding region as the hover/click surface. The centered text label beneath it remains outside that response, retains its normal text color, remains available for accessibility use, and does not add a redundant visible-label tooltip.”
- **REQ-WRITING-LOOP-005 / AC-WRITING-005.4:** “Given the current caret/focus is within a screenplay element, the Elements Item shall display that element's explicit role using the seven supported labels Scene Heading, Action, Character, Dialogue, Parenthetical, Transition, and Note; choosing a label shall apply the role through the existing transactional session operation and preserve the document on rejection.”
- **REQ-WRITING-LOOP-005 / AC-WRITING-005.5:** “Given a toolbar width, Space shall consume one deterministic 72-point width, rendered normal Items shall consume their complete group width including label text measured at runtime with the active egui style/font, and Flexible Space Items shall divide the remaining width equally after the shared 4-point horizontal row-padding and 4-point inter-item-gap budget, and a narrow toolbar shall retain the leftmost fitting Items while exposing non-spacing Items through a far-right `>>` text menu. Action entries and Elements role choices in that menu shall remain executable without persisting toolbar layout state; a fully fitting composition shall keep its rightmost Item, including its label, inside the actual clipped toolbar content right edge after panel, frame, and viewport insets are applied. Items with an approved typed width override, including the Elements control, shall use that same resolved width for allocation and rendering without changing the shared padding, gap, flexible-space, overflow, or clipped-edge budget.”
- **REQ-WRITING-LOOP-005 / AC-WRITING-005.7:** “Toolbar action controls and the Elements control/label surface shall request the standard arrow cursor, while the existing whole-document editing response shall request the text/I-beam cursor. Decorative labels and painted page rectangles shall not create competing cursor owners.”
- **NFR-WRITING-001:** “Production paths should prefer typed errors and bounded inputs over panic-prone handling; a discovered crash or data-loss defect requires a regression test.”
- **NFR-WRITING-002:** “Core editing, layout, and persistence behavior shall be exercisable without a desktop window for deterministic tests.”

No new user-visible criterion is introduced by this extraction-only WO. Preserve applicable existing toolbar requirements verbatim; all new status/page requirements are owned by WO-002.

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) — desktop owns toolbar composition and app adapters; verify existing ownership remains accurate and make no module-specific blueprint change for this extraction.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) — shared semantic/layout boundary; verify unchanged.
- [`technical-context.md`](../../../blueprints/technical-context.md) — desktop/core dependency direction; verify unchanged.

## Implementation plan

1. Before editing, compare the approved plan, existing toolbar tests and current source. Extract the toolbar implementation into a discrete desktop file/module and retain the same public/private adapter boundary; leave [`main.rs`](../../../../apps/scriptcraft/src/main.rs) responsible for app state and wiring rather than duplicating policy.
2. Preserve the existing toolbar rendering, allocation, overflow, action/role, hover/cursor/focus, and clipped-edge contracts without adding status-bar responsibilities or changing user-visible behavior.
3. Add deterministic tests for all named toolbar preservation contracts and run the focused plus configured workspace checks.
4. Verify existing toolbar requirements and desktop ownership; do not update requirements or blueprints as part of this relocation-only change. Record exactly which files changed and preserve a comparable diff.

## Files and systems

- **Create:** A discrete toolbar source file (proposed name `apps/scriptcraft/src/toolbar.rs`; final name may follow review-approved Rust conventions); focused app tests may remain in the current test module or move with helpers.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs) for toolbar module declaration/app wiring and focused toolbar tests.
- **Avoid changing:** `crates/scriptcraft-core/`, native data/schema, layout/PDF/export code, Cargo dependencies/lockfile, unrelated app surfaces, and the manifest.

## Validation and E2E acceptance tests

- **Automated checks:** [`cargo fmt --all -- --check`](../../../../Cargo.toml:1); `cargo test -p scriptcraft toolbar_`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`. These are future commands, not results from this planning session.
- **Focused matrix:** Existing toolbar tests still pass for default order, width measurement/overrides, wide/exact/narrow allocation, flexible spacing and clipped-edge bounds; overflow action/Elements behavior; toolbar command/session routing; hover region/fill; arrow/text cursor ownership; role selection and rejected-operation preservation. No status-bar behavior or test belongs to this WO.
- **Manual/external checks:** Recommended bounded `cargo run -p scriptcraft` smoke at normal and narrow window sizes. Observe extracted toolbar visuals, overflow, hover/cursor/focus and actions. If no visible window/input is available, record the limitation as unavailable, not passed. Per manifest this is not a configured hard-stop gate.
- **Acceptance test:** Given the existing toolbar and app/session contracts, when the toolbar is rendered from its extracted desktop file at wide and narrow widths, then ordering, widths, overflow execution, action/role routing, hover/cursor/focus, and clipped-edge behavior match existing tests, with no user-visible toolbar behavior change. Status-bar and viewport-page acceptance is deferred to WO-002.
- **Independent validator boundary:** After future implementation, a separate fresh-context validator compares only this WO, the approved initiative/review, implementation evidence/diff, durable-document changes, and fresh command results. Do not self-validate; native smoke absence is recorded under project policy and never converted to a pass.

## Durable-document impact

- **Requirements impact:** `verify unchanged` — selected: preserve `REQ-WRITING-LOOP-005 / AC-WRITING-005.1–005.5, AC-WRITING-005.7` verbatim; no new user-visible behavior or requirement is introduced by extraction.
- **Blueprint impact:** `verify unchanged` — selected: existing desktop ownership of toolbar composition is already documented; do not add module-specific implementation details.
- **Affected documents/IDs:** [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md), `REQ-WRITING-LOOP-005 / AC-WRITING-005.1–005.5, AC-WRITING-005.7` (preserve verbatim); [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) (verify existing toolbar ownership).
- **Rationale and evidence:** This is a relocation-only work order. Existing toolbar behavior and criteria are retained; new item-based status and viewport-page user-visible criteria are assigned to WO-002. The desktop blueprint already assigns toolbar composition to the desktop shell, so no durable contract change is needed.
- **Authorized documentation changes:** After review and execution authorization, verify both listed documents against the scoped diff and record them unchanged. No durable document is modified by this WO.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document impact classification is accurate against the diff; requirements and blueprints were verified unchanged.
- [x] Execution log has dispatch/completion entries; the actual host subtask/dispatch ID is reconciled. The separate fresh validation artifact has not yet been created.
- [x] Work order is `in-review` with evidence; completion comes only from passed validation.

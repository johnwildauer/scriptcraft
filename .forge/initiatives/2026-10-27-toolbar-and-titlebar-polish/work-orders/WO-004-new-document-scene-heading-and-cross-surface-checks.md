# Work Order: WO-004 — New Document Scene Heading and Cross-Surface Checks

## Metadata

- **Status:** `complete`
- **Assignee:** Core/document initialization implementer (one fresh session)
- **Phase/order:** 4 of 4; final implementation order; depends on WO-001, WO-002, and WO-003
- **Initiative:** [`../toolbar-and-titlebar-polish-v1.md`](../toolbar-and-titlebar-polish-v1.md:1)
- **Generated filename:** `WO-004-new-document-scene-heading-and-cross-surface-checks.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-004.md`](../validations/validation-WO-004.md)
- **Fixes:** none
- **Implementer session/agent ID:** assigned at dispatch; must be a fresh session
- **Session exception:** none

## Summary

Make every new project and toolbar New action begin with exactly one empty Scene Heading while preserving the established semantic role progression for later edits. The final implementation evidence also performs bounded cross-surface checks for the initiative's toolbar/title strip/cursor/seed outcome and records project-policy gate results without becoming a separate administrative reconciliation WO.

## In scope

- Change the semantic new-project seed owned by [`Project::new()`](../../../../crates/scriptcraft-core/src/lib.rs:221) from one empty Action to one empty Scene Heading.
- Verify the app startup path [`ScriptCraftApp::new()`](../../../../apps/scriptcraft/src/main.rs:449) and toolbar New dispatch [`dispatch_toolbar_action()`](../../../../apps/scriptcraft/src/main.rs:578) both inherit the core seed and do not create a second app policy.
- Add focused core and app regression coverage for seed role, element count, empty text, initial selection validity, and New behavior.
- Run existing Enter/Tab role progression matrices and representative end-to-end editing coverage to prove no transition policy changed.
- Verify existing opened/persisted projects are not rewritten merely by opening them; avoid schema or persistence changes.
- In the same final WO evidence package, perform explicit wide/narrow/maximized/native cross-surface checks for the preceding WOs and run all project-policy gates. These checks validate the already authorized implementation boundaries; they do not authorize new behavior.

## Out of scope

- Rewriting Enter/Tab policy, role-transition algorithms, uppercase/parenthetical behavior, persistence schema, exports, layout geometry, editor rendering, title-strip implementation, toolbar allocation, or cursor implementation.
- A separate administrative acceptance/reconciliation work order, broad redesign, or unsupported native claims when a window/backend is unavailable.

## Requirements

Applicable wording copied verbatim from [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:38):

- **REQ-WRITING-LOOP-001:** “As an individual screenwriter, I want to create and edit an ordered screenplay with explicit element roles, so that my draft retains screenplay meaning while I write.”
- **AC-WRITING-001.1:** “Given a new project, the system shall provide a valid editable starting element and title-page metadata without requiring visual formatting tricks.”
- **AC-WRITING-001.4:** “Given a role transition such as scene heading → action or character → dialogue, the system shall apply an explicit, testable transition policy and make the operation undoable.”
- **AC-WRITING-001.4a:** “Enter shall use the approved matrix: General/Note→same role, Scene Heading→Action, Action→Action, Character→Dialogue, Dialogue→Action at the collapsed UTF-8-safe end of the Dialogue, Dialogue→Dialogue when the collapsed caret is at an interior UTF-8 boundary, Parenthetical→Dialogue, and Transition→Scene Heading. Tab shall use Scene Heading→Action, Action→Character, blank Character→Transition, Transition→Scene Heading, Dialogue→Parenthetical, and Parenthetical→Dialogue; other Tab cases are consumed no-ops with no literal tab, mutation, or undo entry.”

Applicable desktop wording copied verbatim from [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90):

- **AC-WRITING-005.1:** “Given the desktop app is running, the toolbar shall present the typed default Item order New, Open, Save, Save As, Flexible Space, Elements, Flexible Space, and Export PDF.”
- **AC-WRITING-005.2:** “Given an actionable toolbar Item is activated, the desktop shall route it through existing application/session adapters and preserve command safety, status, persistence, and fixed-layout PDF behavior.”

## Blueprints

- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:19) — `ProjectModel` owns ordered semantic elements and explicit roles; the app must not duplicate seed policy.
- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:53) — `ProjectModel` is the durable semantic source and `EditingSession` changes it transactionally.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:39) — the desktop shell is not the source of truth for screenplay text and delegates semantic edits to core.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:49) — `Project` stores ordered semantic screenplay elements with explicit roles.

## Current implementation findings

- [`Project::new()`](../../../../crates/scriptcraft-core/src/lib.rs:221) currently creates `ScreenplayElement::new("element-1", Role::Action, "")` at line 227.
- [`EditingSession::new()`](../../../../crates/scriptcraft-core/src/editing.rs:73) initializes a valid caret at element 0, offset 0, so the seed-role change can remain within the current semantic boundary.
- [`ScriptCraftApp::new()`](../../../../apps/scriptcraft/src/main.rs:449) and toolbar New dispatch both call [`Project::new()`](../../../../apps/scriptcraft/src/main.rs:451), so no second role policy is needed.
- Existing transition behavior is implemented by [`EditingSession::enter()`](../../../../crates/scriptcraft-core/src/editing.rs:257), [`EditingSession::tab()`](../../../../crates/scriptcraft-core/src/editing.rs:331), and [`EditingSession::change_role()`](../../../../crates/scriptcraft-core/src/editing.rs:428); the representative [`wo008_e2e.rs`](../../../../crates/scriptcraft-core/tests/wo008_e2e.rs:77) covers scene-heading/action and character/dialogue progression.

## Implementation plan

1. Change only the `Project::new()` starting role and add focused unit/integration assertions for one empty Scene Heading and valid initial selection.
2. Add an app-level New-path regression if needed, proving startup and toolbar New obtain the same core seed and reset path/session state as before.
3. Run the existing editing tests plus representative Enter/Tab/E2E checks; add only regressions attributable to the seed boundary.
4. Verify native open/reopen fixtures preserve existing roles and that no persistence schema or stored project rewrite is introduced.
5. Perform final bounded cross-surface checks: toolbar wide/narrow geometry and hover, custom title strip/text/toggle behavior, arrow/I-beam ownership, and Scene Heading seed; then run `cargo` policy gates and record unavailable native evidence honestly.

## Files and systems

- **Create:** Focused core/app regression tests only if existing test modules do not provide the required coverage.
- **Update:** [`crates/scriptcraft-core/src/lib.rs`](../../../../crates/scriptcraft-core/src/lib.rs:221), [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:449) only if app coverage requires it, relevant tests, this work order, and authorized requirements/blueprint wording.
- **Avoid changing:** [`crates/scriptcraft-core/src/editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:1) transition logic unless a regression proves the seed change requires a minimal compatibility fix; no title/toolbar implementation belongs here.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; focused seed and transition tests; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- **Deterministic acceptance:** `Project::new()` yields one element with `Role::SceneHeading`, empty text, stable valid identity, and a valid caret at element 0/offset 0. Scene Heading Enter still creates Action; the existing approved Enter/Tab matrix passes; existing project parse/open fixtures retain stored roles.
- **Cross-surface acceptance:** In deterministic app tests or a bounded native run, confirm toolbar default order and actual clipped-edge fit, icon/control hover ownership, title text and state-command behavior, standard arrow/I-beam response ownership, and New seed. This is implementation evidence attached to this outcome, not an administrative WO.
- **Manual/external checks:** If native UI is available, exercise wide, narrow, and maximized/restore states and record OS/backend/rect/cursor observations. If unavailable, record each unavailable observation separately and do not convert it into a pass claim.
- **Acceptance test:** Given a fresh app startup or toolbar New action, when the project is created, then the session contains exactly one empty Scene Heading with a valid initial caret; when the writer uses the existing Enter/Tab transitions, then the approved progression remains unchanged; when final cross-surface checks run, then only evidence-backed outcomes are accepted.

## Durable-document impact

- **Requirements impact:** `update` — AC-WRITING-001.1 needs the explicit Scene Heading seed contract, while AC-WRITING-001.4/001.4a and desktop command criteria are verified unchanged by regression.
- **Blueprint impact:** `update` — the core and feature blueprints should state that new `Project` instances seed one empty Scene Heading while `EditingSession` continues to own transition policy; desktop blueprint should be checked for unchanged delegation.
- **Affected documents/IDs:** [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:38) / REQ-WRITING-LOOP-001 / AC-WRITING-001.1 / AC-WRITING-001.4 / AC-WRITING-001.4a; [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:19); [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:49); [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:39).
- **Rationale and evidence:** The current seed is in [`Project::new()`](../../../../crates/scriptcraft-core/src/lib.rs:221), while transition ownership is already in [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:257). The final WO contains explicit cross-surface and full-gate checks so no administrative acceptance WO is needed.
- **Authorized documentation changes:** Record the Scene Heading seed and verification of unchanged transition/delegation contracts; record only evidence-backed cross-surface limitations. Do not revise unrelated role, persistence, or layout contracts.

## Dependencies, risks, and handoff

- **Dependencies:** WO-001, WO-002, and WO-003 must be implemented and independently validated before this final implementation WO's cross-surface evidence can be collected. The seed code itself depends only on the existing core boundary.
- **Risks:** Tests or fixtures may assume Action as the initial role; changing transition logic to compensate would broaden scope. Mitigate by updating only seed-specific expectations and proving the existing matrix.
- **Handoff:** The orchestrator must dispatch fresh validation for this WO and must not treat its implementation-session cross-surface checks as validation or design approval.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document impact classification is accurate against the diff, and authorized documentation updates are applied.
- [x] Execution log is appended with dispatch and completion session/agent identifiers.
- [x] Fresh validation passed with warnings; work order is `complete`. See [`validation-WO-004.md`](../validations/validation-WO-004.md:1).

## Implementation evidence

- **Dispatch/completion session:** `implementer-wo004-scene-heading-2026-10-27T1605Z`; fresh final implementer session for WO-004 only. No prior toolbar/title-strip/cursor implementation was redesigned.
- **Branch/status evidence:** verified before changes with `git status --short --branch` and `git branch --show-current`; branch is `dev/2026-10-27-toolbar-and-titlebar-polish`. Existing WO-001/WO-002/WO-003 source, durable-document changes, and Forge evidence were preserved.
- **Changed files:** [`crates/scriptcraft-core/src/lib.rs`](../../../../crates/scriptcraft-core/src/lib.rs:221), [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:470), [`crates/scriptcraft-core/tests/wo002_paste_safety.rs`](../../../../crates/scriptcraft-core/tests/wo002_paste_safety.rs:7), [`crates/scriptcraft-core/tests/wo003_long_document.rs`](../../../../crates/scriptcraft-core/tests/wo003_long_document.rs:60), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:44), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:63), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:80), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:39), this work order, and [`execution-log.md`](../execution-log.md). The app source also retains the already-validated WO-001/WO-002/WO-003 baseline; this WO-specific change is seed-regression assertions and updated seed-dependent fixture expectations.
- **Implementation:** [`Project::new()`](../../../../crates/scriptcraft-core/src/lib.rs:222) now creates exactly one empty [`Role::SceneHeading`](../../../../crates/scriptcraft-core/src/lib.rs:72) element with stable ID `element-1`. [`EditingSession::new()`](../../../../crates/scriptcraft-core/src/editing.rs:74) remains the unchanged owner of the valid element-0/offset-0 initial selection. [`ScriptCraftApp::new()`](../../../../apps/scriptcraft/src/main.rs:470) and [`dispatch_toolbar_action()`](../../../../apps/scriptcraft/src/main.rs:598) still delegate to the core constructor; no second app seed policy or New/open persistence change was added.
- **Focused regression evidence:** Core tests assert one element, stable identity, empty text, Scene Heading role, valid model, and valid initial caret. App tests assert both startup and toolbar New produce the same one-empty-Scene-Heading seed, valid caret, reset status/path behavior, and preserved existing New routing. Existing Enter/Tab matrix and representative E2E tests remain passing.
- **Fixture reconciliation:** WO-002 paste-safety and WO-003 long-document fixtures now explicitly select `Role::Action` where their assertions exercise non-cue casing/paste/layout behavior; this keeps those tests focused on their prior contract rather than coupling them to the new-project seed.
- **Bounded cross-surface evidence:** `cargo test -p scriptcraft toolbar_` passed 26 tests covering approved order, wide/exact/narrow allocation, clipped-edge containment, hover geometry, and New routing. `cargo test -p scriptcraft title_strip` passed 2 tests covering 30-point geometry, centered placeholder, and known/unknown maximize command selection. `cargo test -p scriptcraft cursor_` passed 2 tests covering arrow control ownership and text document ownership. The combined app suite passed 69 tests. These deterministic checks confirm prior implementation boundaries without changing them.
- **Transition/delegation/persistence evidence:** `cargo test -p scriptcraft-core enter_matrix_covers_every_role` and `tab_matrix_is_undoable_and_unspecified_roles_are_consumed_noops` passed; representative [`wo008_e2e.rs`](../../../../crates/scriptcraft-core/tests/wo008_e2e.rs:78) passed, including scene-heading/action progression and save/reopen. The workspace suite passed 45 core unit tests, all existing integration tests, and 69 app tests. Opened stored projects remain parsed/replaced through the existing path; no persistence schema or rewrite behavior changed.
- **Required policy checks:** `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` all passed after formatter correction. The first format check failed only on new assertion layout; `cargo fmt --all` corrected it. The first full core test attempt exposed seed-dependent fixture assumptions in WO-002/WO-003; explicit Action fixture setup corrected those tests, and the complete required sequence then passed.
- **Comparable diff evidence:** `git status --short --branch`, `git diff --stat`, `git diff --check`, and targeted source/durable-document diff inspection were run. The implementation delta is limited to the core seed, focused app/core regressions, seed-dependent fixture setup/expectations, and the authorized Scene Heading/delegation/unchanged-boundary wording. No validation artifact was created.
- **Native smoke availability:** `cargo run -p scriptcraft` reached `target/debug/scriptcraft` but remained running without a window-capable observation in this session and was terminated. No native OS/backend, wide/narrow/maximized/restore, hover-pixel, visible cursor, title-strip, or native New observation is claimed. Native evidence remains unavailable/non-required under [`project-manifest.md`](../../../project-manifest.md:88).
- **Next state:** `in-review`; do not create [`validation-WO-004.md`](../validations/validation-WO-004.md) in this implementer session and do not mark this work order `complete`. A fresh validator must assess WO-004.

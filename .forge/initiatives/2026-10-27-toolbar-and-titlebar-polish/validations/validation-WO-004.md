# Validation: WO-004 — New Document Scene Heading and Cross-Surface Checks

## Validation metadata

- **Review type:** `validation`
- **Status:** `in-review`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `validator-fresh-context-wo004-scene-heading-2026-10-27T1608Z`
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Initiative:** [`toolbar-and-titlebar-polish-v1.md`](../toolbar-and-titlebar-polish-v1.md:1)
- **Validation file:** `validation-WO-004.md`
- **Validated work order:** [`WO-004-new-document-scene-heading-and-cross-surface-checks.md`](../work-orders/WO-004-new-document-scene-heading-and-cross-surface-checks.md:1)
- **Compared commit/diff:** Working-tree diff on `dev/2026-10-27-toolbar-and-titlebar-polish`, against recorded base `master` commit `f5b02d3eb58464f091c57667711f3be738e65e56`; implementation remains uncommitted, so `git diff master...HEAD` has no committed implementation delta and the comparable evidence is the working-tree status, stat, and tracked diff.
- **Fix chain:** `none`

## Authorized scope

This validation assesses WO-004 only: the core-owned `Project::new()` seed,
startup and toolbar New delegation, the unchanged Enter/Tab progression and
representative E2E behavior, opened/persisted-project role preservation, the
bounded cross-surface checks for the already validated toolbar/title-strip/
cursor boundaries, and the authorized requirements/blueprint and project-policy
checks. WO-001, WO-002, and WO-003 are treated as validated baselines, not
reimplemented future scope. Native smoke is optional under project policy and is
reported as a warning when unavailable. No source or work-order implementation
was modified during validation.

The comparison basis was the `in-review` work order, its changed-file record and
implementation evidence, the execution log, initiative plan and design review,
prior WO validations, requirements, blueprints, current branch status, and the
working-tree diff for [`lib.rs`](../../../../crates/scriptcraft-core/src/lib.rs:221),
[`main.rs`](../../../../apps/scriptcraft/src/main.rs:470), the two seed-dependent
fixtures, and the authorized durable-document wording.

## Validation checklist

- **Evidence/preconditions: PASS.** WO-004 is `in-review`; the fresh implementer, changed files, command attempts/results, native-smoke limitation, and comparable branch evidence are recorded in the work order and execution log. The current branch and working-tree diff permit meaningful comparison.
- **Boundary/comparability: PASS.** The tracked diff contains the core seed, app seed-dependent assertions, fixture isolation updates, and the authorized requirements/blueprint wording, alongside the already validated prior-surface baseline. No unrelated persistence schema, export, transition algorithm, or rogue behavior was observed.
- **Core seed and identity: PASS.** [`Project::new()`](../../../../crates/scriptcraft-core/src/lib.rs:222) constructs exactly one empty [`Role::SceneHeading`](../../../../crates/scriptcraft-core/src/lib.rs:72) element with stable ID `element-1`; model validation succeeds. [`EditingSession::new()`](../../../../crates/scriptcraft-core/src/editing.rs:73) produces the valid element-0/byte-offset-0 caret, and the focused core regression asserts the complete contract.
- **Startup/New delegation: PASS.** [`ScriptCraftApp::new()`](../../../../apps/scriptcraft/src/main.rs:470) and [`dispatch_toolbar_action()`](../../../../apps/scriptcraft/src/main.rs:598) both construct through [`Project::new()`](../../../../crates/scriptcraft-core/src/lib.rs:222) and [`EditingSession::new()`](../../../../crates/scriptcraft-core/src/editing.rs:73). App tests cover startup and toolbar New, including one element, empty text, Scene Heading role, valid caret, status/path reset, and no second app seed policy.
- **Progression and E2E: PASS.** The approved Enter matrix and Tab matrix pass without changes to [`EditingSession::enter()`](../../../../crates/scriptcraft-core/src/editing.rs:257) or [`EditingSession::tab()`](../../../../crates/scriptcraft-core/src/editing.rs:333). The representative [`wo008_e2e.rs`](../../../../crates/scriptcraft-core/tests/wo008_e2e.rs:78) passes scene-heading/action progression, editing, persistence round-trip, recovery, and export assertions.
- **Opened/persisted roles: PASS.** Existing open/reopen paths continue to parse and replace through [`open_native_at()`](../../../../apps/scriptcraft/src/main.rs:552) and [`replace_with_saved()`](../../../../crates/scriptcraft-core/src/editing.rs:566) without a persistence-schema or open-time rewrite. The workspace persistence and role-casing regression suites pass, including the explicit legacy/open-without-rewrite coverage.
- **Bounded cross-surface checks: PASS.** The current deterministic tests retain prior WO-001 wide/exact/narrow allocation, clipped-edge, toolbar-order, and hover behavior; WO-002 title-strip geometry, centered placeholder, and known/unknown viewport-command behavior; and WO-003 standard-arrow/I-beam ownership. Prior validations independently pass those boundaries, and the current combined app suite passes 69 tests. No prior toolbar/title-strip/text/viewport-command/cursor ownership implementation was changed by the WO-004-specific source hunks.
- **Durable-document alignment: PASS.** The requirements update records the exact Scene Heading starting-element contract while retaining the approved transition and desktop criteria. The core, feature, and desktop blueprints record core seed ownership, session caret ownership, startup/New delegation, and unchanged opened-project/Enter/Tab boundaries. The `update/update` classification in WO-004 matches these evidence-backed changes; no unrelated durable contract was revised.
- **Project-policy checks: PASS.** All configured Cargo and diff checks passed. No required native/manual gate is configured in [`project-manifest.md`](../../../project-manifest.md:88).
- **Rogue behavior: PASS.** No deceptive, backdoor, malicious, nefarious, or otherwise unrelated behavior was observed.

## Findings

### Blockers

- None. No plainly unmet authorized criterion, unrelated implementation, required-policy failure, or comparison blocker was found.

### Warnings

- **WO004-VAL-W-001 — Native smoke unavailable.** **Severity:** warning. **Evidence:** [`WO-004`](../work-orders/WO-004-new-document-scene-heading-and-cross-surface-checks.md:118) and the execution log record that `cargo run -p scriptcraft` reached the binary but no window-capable OS/backend observation was available; the run was terminated. Native title-strip maximize/restore, visual hover, visible cursor, and native New observations therefore remain unclaimed. The manifest does not configure native smoke as a required manual gate ([`project-manifest.md`](../../../project-manifest.md:88-95)). **Recommendation:** Preserve the per-surface unavailability record and do not convert deterministic response/state tests into native visual claims. **Required next state:** `complete`.

### Notes

- **WO004-VAL-N-001 — Working-tree implementation is uncommitted.** **Severity:** note. **Evidence:** `git diff master...HEAD` has no committed implementation delta, while status/stat and the tracked diff provide the expected comparable evidence. **Recommendation:** Commit through the normal Forge closeout path; this does not prevent validation. **Required next state:** `complete`.
- **WO004-VAL-N-002 — Diagnostic source review.** **Severity:** note. The plausible sources considered were (1) an incorrect seed role/count/identity, (2) invalid initial caret construction, (3) duplicated app-side New policy, (4) changed Enter/Tab matrices, (5) opened-project rewriting, (6) seed-dependent fixture drift, (7) regression in prior toolbar/title-strip/cursor surfaces, (8) durable-document scope mismatch, and (9) missing policy evidence. Source/diff comparison, focused regressions, prior validations, full workspace checks, persistence/E2E evidence, and policy commands reduce these to no hard-stop finding; only native visual observation remains unavailable. **Recommendation:** No implementation change. **Required next state:** `complete`.

## Exact commands and results

| Command | Result |
|---|---|
| `git status --short --branch` | Exit 0; branch `dev/2026-10-27-toolbar-and-titlebar-polish`; 8 tracked files modified and the initiative evidence directory untracked. |
| `git diff --stat` | Exit 0; 8 tracked files, 291 insertions and 40 deletions. The broad app diff includes the previously validated WO-001/WO-002/WO-003 baseline; WO-004-specific hunks were isolated during comparison. |
| `git diff --check` | Exit 0; no whitespace errors. |
| `cargo test -p scriptcraft-core new_project -- --nocapture` | Exit 0; 2 seed tests passed, 0 failed. |
| `cargo test -p scriptcraft toolbar_ -- --nocapture` | Exit 0; 26 toolbar/geometry/hover/New-routing tests passed, 0 failed. |
| `cargo test -p scriptcraft title_strip -- --nocapture` | Exit 0; 2 title-strip tests passed, 0 failed. |
| `cargo test -p scriptcraft cursor_ -- --nocapture` | Exit 0; 2 cursor-ownership tests passed, 0 failed. |
| `cargo test -p scriptcraft new_dispatch -- --nocapture` | Exit 0; 1 toolbar New reset/seed test passed, 0 failed. |
| `cargo test -p scriptcraft-core enter_matrix_covers_every_role -- --nocapture` | Exit 0; approved Enter matrix test passed. |
| `cargo test -p scriptcraft-core tab_matrix_is_undoable_and_unspecified_roles_are_consumed_noops -- --nocapture` | Exit 0; approved Tab matrix/no-op test passed. |
| `cargo test -p scriptcraft-core --test wo008_e2e -- --nocapture` | Exit 0; 3 representative E2E/persistence/recovery/export tests passed. |
| `cargo fmt --all -- --check` | Exit 0. |
| `cargo check --workspace` | Exit 0. |
| `cargo test --workspace` | Exit 0; 45 core unit tests, all core integration suites, and 69 app tests passed. |
| `cargo build --workspace` | Exit 0. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0. |
| `git diff master...HEAD` | Exit 0; no committed delta because implementation remains uncommitted; working-tree diff used for comparison. |
| `cargo run -p scriptcraft` | Binary reached, but no window-capable native observation was available; process was terminated and no native visual claim is made. Non-required manual limitation. |

## Diff/file comparison

The WO-004-specific implementation delta is confined to:

- [`crates/scriptcraft-core/src/lib.rs`](../../../../crates/scriptcraft-core/src/lib.rs:221): the `Project::new()` seed changes from Action to one empty Scene Heading, with focused identity/model/caret regressions.
- [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:470): startup/New assertions and seed-dependent expectations; startup and toolbar New continue to delegate to the core constructor and session constructor. No toolbar allocation, title-strip, viewport-command, cursor-owner, persistence, or transition implementation hunk was added.
- [`wo002_paste_safety.rs`](../../../../crates/scriptcraft-core/tests/wo002_paste_safety.rs:7) and [`wo003_long_document.rs`](../../../../crates/scriptcraft-core/tests/wo003_long_document.rs:60): explicit Action fixture setup preserves those tests' prior non-cue/paste/layout contracts rather than coupling them to the new-project seed.
- [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:44), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:63), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:80), and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:39): authorized Scene Heading seed, caret, delegation, and unchanged-boundary wording only.

The broader [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1) and desktop-blueprint diff contains the already validated WO-001/WO-002/WO-003 baseline. The prior validation artifacts confirm those changes remain aligned; current focused cross-surface tests confirm they remain passing. No future scope was assessed.

## Decision

- **Outcome:** `pass`
- **Required next state:** `complete`
- **Fix work order required:** `no`
- **WO-004 disposition:** WO-004 should become `complete`; no letter-suffixed fix work order is warranted. Native smoke is a non-blocking warning under project policy and does not prevent meaningful deterministic comparison.
- **Implementation/design claim boundary:** This artifact validates WO-004 only. It does not reopen or revalidate future scope, and it does not claim native visual behavior unavailable in this checkout.

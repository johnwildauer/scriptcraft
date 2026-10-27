# Validation: WO-001 — Viewport State Direction and Native Restore Contract

## Validation metadata

- **Review type:** `validation`
- **Status:** `in-review`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `validator-fresh-context-wo001-viewport-2026-10-27T1731Z`
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Initiative:** [`titlebar-transition-and-elements-alignment-v1.md`](../titlebar-transition-and-elements-alignment-v1.md:1)
- **Validation file:** `validation-WO-001.md`
- **Validated work order:** [`WO-001-viewport-state-direction-and-native-restore-contract.md`](../work-orders/WO-001-viewport-state-direction-and-native-restore-contract.md:1)
- **Compared commit/diff:** Working-tree diff on `dev/2026-10-27-titlebar-transition-and-elements-alignment`, against the recorded `master` base `5b8eda725f767fbed530c021882b15bec82b6ceb`; implementation is uncommitted, so the working-tree status/stat/source diff and `git diff --check` are the comparable evidence.
- **Fix chain:** `none`

## Authorized scope

This validation assesses [`WO-001-viewport-state-direction-and-native-restore-contract.md`](../work-orders/WO-001-viewport-state-direction-and-native-restore-contract.md:1) only: classification of observed normal, maximized, and unknown viewport state; normal-to-maximize/fill and maximized-to-native-restore command direction; safe unknown-state no-op; installed eframe/egui API alignment; deterministic state/command tests; preservation of the non-full-screen and backend-bounded restoration contract; and the configured project-policy checks.

The comparison basis is the `in-review` work order, its implementation evidence, the follow-up execution log, initiative plan/design review, closed-parent [`WO-002-custom-title-strip-and-viewport-toggle.md`](../../2026-10-27-toolbar-and-titlebar-polish/work-orders/WO-002-custom-title-strip-and-viewport-toggle.md:1) and [`validation-WO-002.md`](../../2026-10-27-toolbar-and-titlebar-polish/validations/validation-WO-002.md:1), installed egui/egui-winit 0.29.1 sources, current branch status, and the working-tree diff for [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1671).

Title text and strip geometry, toolbar geometry/actions, Elements rendering/width, cursor behavior, durable-document wording, and WO-002/003/004 implementation are outside this validation's authorized change boundary. The diff shows the already-present `128.0` Elements and zero-top-padding baseline reconciliations; the WO-001 implementation seam did not alter those runtime behaviors, and no new durable-document change was introduced.

## Validation checklist

- **Evidence/preconditions: PASS.** [`WO-001-viewport-state-direction-and-native-restore-contract.md`](../work-orders/WO-001-viewport-state-direction-and-native-restore-contract.md:5) is `in-review`; changed files, implementation attempts/results, native-smoke limitation, session identity, and comparable diff evidence are recorded. Meaningful comparison is possible.
- **Boundary/comparability: PASS.** The WO-specific source change is the state enum/classifier, command mapping, and deterministic assertions at [`viewport_maximized_state()`](../../../../apps/scriptcraft/src/main.rs:1678) and [`title_strip_viewport_command()`](../../../../apps/scriptcraft/src/main.rs:1686). The unrelated baseline constants/tests are explicitly preserved rather than assessed as WO-001 behavior.
- **Normal state direction: PASS.** `Some(false)` classifies as `Normal` and requests [`ViewportCommand::Maximized(true)`](../../../../apps/scriptcraft/src/main.rs:1688), which is maximize/fill rather than full-screen.
- **Maximized state direction: PASS.** `Some(true)` classifies as `Maximized` and requests [`ViewportCommand::Maximized(false)`](../../../../apps/scriptcraft/src/main.rs:1689), delegating restore to the native/backend path.
- **Unknown state safety: PASS.** `None` classifies as `Unknown` and returns no command at [`title_strip_viewport_command()`](../../../../apps/scriptcraft/src/main.rs:1686); the caller sends a viewport command only when the result is `Some` at [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:1708).
- **Installed API semantics: PASS.** Installed [`ViewportInfo::maximized`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/data/input.rs:227) is optional observed state, [`ViewportInfo::outer_rect`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/data/input.rs:218) is optional geometry, and [`ViewportCommand::Maximized(bool)`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/viewport.rs:1006) is a maximize/unmaximize request. Installed [`update_viewport_info()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:943) avoids repeated runtime macOS maximize queries after initialization, supporting the unknown/stale-state boundary.
- **Restore and safety claim boundary: PASS.** No [`ViewportCommand::Fullscreen(bool)`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/viewport.rs:1008) request, application-owned frame snapshot/history, unsafe prior-command inference, or universal exact-restore claim was introduced. Native unmaximize remains explicitly backend-bounded.
- **Deterministic acceptance tests: PASS.** [`title_strip_viewport_toggle_is_safe_for_known_and_unknown_state()`](../../../../apps/scriptcraft/src/main.rs:1849) asserts classification and command selection for `Some(false)`, `Some(true)`, and `None`; independently rerun focused tests passed 2/2.
- **Scope preservation: PASS.** No new title text/strip geometry, toolbar, Elements, cursor, core-crate, WO-002/003/004, requirements, or blueprint implementation change is attributable to this WO. Existing baseline test expectations were reconciled to the already-present runtime constants, as recorded in the implementation evidence.
- **Rogue behavior: PASS.** No deceptive, backdoor, malicious, nefarious, or otherwise unrelated behavior was observed.
- **Durable-document impact: PASS.** The work order classifies requirements and blueprint impact as `verify unchanged`; the bounded known-state/unknown-state/native-restore contract already exists in the cited requirements and desktop blueprint, and no wording change was needed.
- **Project-policy checks: PASS.** All configured automated checks passed. Native macOS window smoke was not available, but no such manual gate is required by [`project-manifest.md`](../../../project-manifest.md:88-95), and its absence does not prevent deterministic comparison.

## Findings

### Blockers

- None. No plainly unmet authorized criterion, unrelated or rogue implementation, required-policy failure, or comparison blocker was found.

### Warnings

- **WO001-VAL-W-001 — Native macOS maximize/restore smoke unavailable.** **Severity:** warning. **Evidence:** [`execution-log.md`](../execution-log.md:28) and the work-order evidence record no window-capable run, OS/backend observation, outer/inner rectangle comparison, command response, or native restore result. The installed API exposes optional state/rectangles and the macOS update path can leave maximize state stale after initialization. **Recommendation:** Preserve the native/backend limitation; do not claim observed maximize, unmaximize, frame restoration, or exact prior-frame behavior. Record those observations only when a window-capable macOS run becomes available. **Required next state:** `complete`.

### Notes

- **WO001-VAL-N-001 — Working-tree implementation is uncommitted.** **Severity:** note. **Evidence:** current branch status shows the expected modified [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1671) and untracked follow-up Forge evidence; the recorded base/diff comparison is therefore a working-tree comparison. **Recommendation:** Commit through normal Forge closeout. **Required next state:** `complete`.
- **WO001-VAL-N-002 — Diagnostic source review.** **Severity:** note. Candidate sources considered were stale/unknown macOS state, inverted boolean interpretation, request-vs-observed API confusion, unsafe `None` toggling, app-owned frame guessing, full-screen substitution, test-only coverage gaps, and unrelated title/toolbar/Elements/cursor changes. Source inspection, installed API evidence, deterministic tests, diff comparison, and policy checks reduce these to no hard-stop finding; only optional native comparison remains unavailable. **Recommendation:** No implementation change. **Required next state:** `complete`.

## Exact checks and results

| Check | Result |
|---|---|
| `git status --short --branch` | Exit 0; branch `dev/2026-10-27-titlebar-transition-and-elements-alignment`; [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1671) modified and follow-up Forge directory untracked. |
| `git diff --stat` | Exit 0; 1 tracked source file, 45 changed lines; WO-specific behavior is limited to the viewport state seam/tests, with recorded pre-existing baseline reconciliations also present. |
| `git diff --check` | Exit 0; no whitespace errors. |
| `cargo fmt --all -- --check` | Exit 0. |
| `cargo test -p scriptcraft title_strip` | Exit 0; 2 focused tests passed, 0 failed. |
| `cargo check --workspace` | Exit 0. |
| `cargo test --workspace` | Exit 0; 69 app tests, 45 core tests, and integration suites passed. |
| `cargo build --workspace` | Exit 0. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0. |
| Installed API inspection | Exit 0; optional `maximized`/`inner_rect`/`outer_rect`, request-style `Maximized(bool)`, and the macOS runtime-query limitation confirmed from installed 0.29.1 sources. |
| Native macOS window smoke | Unavailable; warning only because project policy configures no required native/manual gate. |

## Changed-file and diff comparison

The current source diff contains the existing `TOOLBAR_ELEMENTS_WIDTH` and toolbar-padding baseline reconciliations, plus the WO-001 enum/classifier, explicit command mapping, and test assertions. The implementation evidence identifies the baseline values as pre-existing and records that title text, strip geometry, toolbar runtime behavior, Elements runtime behavior, cursor ownership, core crates, and durable documents were preserved. The WO-001-specific diff is aligned with the authorized file/system boundary in [`WO-001-viewport-state-direction-and-native-restore-contract.md`](../work-orders/WO-001-viewport-state-direction-and-native-restore-contract.md:63-67). No WO-002/003/004 implementation was assessed or claimed.

## Decision

- **Outcome:** `pass`
- **Required next state:** `complete`
- **Fix work order required:** `no`
- **Recommendation:** Set WO-001 to `complete`. Do not create a letter-suffixed fix work order; the missing native macOS smoke is a non-blocking warning under project policy and does not prevent meaningful deterministic comparison. Preserve the warning and the bounded native restore claim for later native-capable validation.
- **Implementation/design claim boundary:** This artifact validates WO-001 alignment only. It does not validate or claim completion of WO-002, WO-003, or WO-004.

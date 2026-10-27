# Validation: V2-WO-001 — Transition Metric and Native Contract

## Validation metadata

- **Review type:** `validation`
- **Status:** `in-review`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `validator-fresh-context-v2-wo001-transition-metrics-2026-10-27T1910Z`
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Initiative:** [`titlebar-transition-and-elements-alignment-v2.md`](../titlebar-transition-and-elements-alignment-v2.md:1)
- **Validation file:** `validation-V2-WO-001.md`
- **Validated work order:** [`V2-WO-001-transition-metric-native-contract.md`](../work-orders/V2-WO-001-transition-metric-native-contract.md:1)
- **Compared commit/diff:** Working-tree comparison on `dev/2026-10-27-titlebar-transition-and-elements-alignment-v2` against the recorded `master` base `e77d6c4ee424403c5ab779d372745a51e4cafd4e`; tracked implementation changes are [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1) and [`execution-log.md`](../execution-log.md:1), with the v2 Forge artifacts untracked as recorded by `git status`.
- **Fix chain:** `none`

## Authorized scope

This validation assesses [`V2-WO-001-transition-metric-native-contract.md`](../work-orders/V2-WO-001-transition-metric-native-contract.md:1) only: bounded viewport/egui/presentation diagnostics; known normal, maximized, and unknown state-to-command behavior; the evidence-gated page-metric policy; installed eframe/egui/winit API alignment; preservation of the v1 toolbar, `128.0`-point Elements, cursor, and unified-chrome behavior; durable-document classification; and the configured project-policy checks. V2-WO-002, V2-WO-003, and V2-WO-004 are excluded.

The comparison basis is the `in-review` work order, its implementation evidence, the v2 plan/design review, the v2 execution-log entry, the closed v1 parent validation evidence, requirements/blueprints, current branch status and diff, [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1), and installed eframe/egui/egui-winit/winit 0.29.1/0.30.13 sources. No source or work-order change was made by this validation.

## Validation checklist

- **Evidence/preconditions: PASS.** The work order is explicitly `in-review` at [`V2-WO-001-transition-metric-native-contract.md`](../work-orders/V2-WO-001-transition-metric-native-contract.md:5); changed files, implementation attempts/results, native-smoke limitation, session identity, and comparable diff evidence are recorded. Meaningful comparison is possible.
- **Authorized behavior: PASS.** The source diff is bounded to diagnostics, their observation points, the live-width policy seam, command observation, and focused tests in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:69).
- **Rogue behavior: PASS.** No deceptive, backdoor, malicious, nefarious, or unrelated behavior was observed.
- **Required checks: PASS.** All configured automated checks were independently rerun successfully. Native macOS display/smoke is not a configured required gate.
- **Durable documents: PASS.** The `verify unchanged` requirements/blueprint classification matches the diff: no requirements or blueprint file changed, and the existing bounded viewport contract remains accurate.

## Exact checks and results

| Check | Result |
|---|---|
| Work-order status | PASS; [`V2-WO-001-transition-metric-native-contract.md`](../work-orders/V2-WO-001-transition-metric-native-contract.md:5) is `in-review`. |
| `git status --short --branch` | Exit 0; required v2 branch confirmed; [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1) and [`execution-log.md`](../execution-log.md:1) modified; v2 planning/validation artifacts untracked. |
| `git diff --stat` | Exit 0; 222 source lines plus 13 execution-log lines in the tracked working-tree diff. |
| `git diff --check` | Exit 0; no whitespace errors. |
| `cargo fmt --all -- --check` | Exit 0. |
| `cargo test -p scriptcraft presentation_metrics -- --nocapture` | Exit 0; 2 focused tests passed. |
| `cargo test -p scriptcraft title_strip -- --nocapture` | Exit 0; 2 focused tests passed. |
| `cargo test -p scriptcraft` | Exit 0; 77 app tests passed. |
| `cargo check --workspace` | Exit 0. |
| `cargo test --workspace` | Exit 0; app, core, integration, and doc tests passed, including 45 core unit tests. |
| `cargo build --workspace` | Exit 0. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0. |
| Installed API inspection | PASS; [`ViewportInfo`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/data/input.rs:194) is observed optional state/geometry, [`ViewportCommand`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/viewport.rs:948) includes request-style [`Maximized(bool)`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/viewport.rs:1006), and egui-winit maps it to [`set_maximized()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:1435). No supported requested-resize flag was found. |
| Native macOS window/display smoke | Unavailable; warning only because [`project-manifest.md`](../../../project-manifest.md:88) configures no required native/manual gate. No native result is claimed. |

## Exact criteria and results

- **Bounded diagnostics: PASS.** [`ViewportMetricSample`](../../../../apps/scriptcraft/src/main.rs:72) records frame/time, optional observed maximized state, native and egui scale, optional inner/outer rectangles, egui content/clip/available rectangles, page width/scale/font size, toolbar bounds, and page bounds. [`ViewportDiagnostics`](../../../../apps/scriptcraft/src/main.rs:99) retains at most 120 samples and 32 command observations through [`bounded_push()`](../../../../apps/scriptcraft/src/main.rs:188). Existing frame, toolbar, document, and title-strip observation points populate the record at [`update()`](../../../../apps/scriptcraft/src/main.rs:2014), [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:809), [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:1262), and [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:1977).
- **Normal/maximized/unknown command behavior: PASS.** [`title_strip_viewport_command()`](../../../../apps/scriptcraft/src/main.rs:1969) maps `Some(false)` to [`Maximized(true)`](../../../../apps/scriptcraft/src/main.rs:1971), `Some(true)` to [`Maximized(false)`](../../../../apps/scriptcraft/src/main.rs:1972), and `None` to no command at [`title_strip_viewport_command()`](../../../../apps/scriptcraft/src/main.rs:1973). The focused test covers all three cases at [`title_strip_viewport_toggle_is_safe_for_known_and_unknown_state()`](../../../../apps/scriptcraft/src/main.rs:2151).
- **Freeze policy: PASS.** [`presentation_metric_policy()`](../../../../apps/scriptcraft/src/main.rs:1730) selects [`PresentationMetricPolicy::LiveWidthDerived`](../../../../apps/scriptcraft/src/main.rs:1725); the implementation does not claim a page-metric freeze without a reliable interval. The policy test proves the selected path remains equivalent to [`presentation_metrics()`](../../../../apps/scriptcraft/src/main.rs:1739) at [`presentation_policy_retains_live_metrics_until_native_interval_is_proven()`](../../../../apps/scriptcraft/src/main.rs:2327). No unsupported freeze was added.
- **Forbidden additions: PASS.** Source inspection found no requested-resize flag, OS desktop/work-area polling, global [`set_pixels_per_point()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/context.rs:1885) use, unsupported animation control, full-screen command, or exact restore-frame claim. The only animation tokens are pre-existing caret behavior, not viewport transition control.
- **v1 preservation: PASS.** The diff preserves [`TOOLBAR_ELEMENTS_WIDTH`](../../../../apps/scriptcraft/src/main.rs:28), toolbar allocation and cursor helpers, the single document response in [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:1262), and the shared chrome/title-bar path. No toolbar/Elements/cursor/unified-chrome implementation is attributable to this work order.
- **Durable-document classification: PASS.** The work order declares requirements and blueprint `verify unchanged` at [`V2-WO-001-transition-metric-native-contract.md`](../work-orders/V2-WO-001-transition-metric-native-contract.md:81); the diff contains no requirements or blueprint update, and the existing bounded contract remains applicable.

## Diagnostic source review

The independent diagnostic considered seven plausible failure sources: stale/unknown macOS maximized state; inverted boolean direction; request-versus-observation API confusion; unsafe `None` toggling; width-derived page-scale changes during native resize; an app-owned frame/restore guess; and unrelated toolbar/Elements/cursor/chrome regression. Source inspection, installed API evidence, deterministic tests, diff comparison, and project-policy checks reduce these to no hard-stop finding. The only unresolved evidence is optional native display observation.

## Findings

### Blockers

- None. No plainly unmet authorized criterion, unrelated/rogue implementation, required-policy failure, or comparison blocker was found.

### Warnings

- **V2-WO-001-VAL-W-001 — Native macOS display/smoke unavailable.** **Severity:** warning. **Evidence:** the implementation log and work-order evidence record a bounded launch without a usable display/window, so no native outer/inner rectangles, transition frames, command response, restore result, pixels, or display-scale observation exists. **Recommendation:** preserve the bounded native limitation and do not claim observed maximize/restore behavior or exact prior-frame restoration until a window-capable macOS run exists. **Required next state:** `complete`.

### Notes

- **V2-WO-001-VAL-N-001 — First diagnostic command probe used unavailable `rg`.** **Severity:** note. **Evidence:** the initial symbol probe returned exit 127 because `rg` is not installed; equivalent `grep`/`find` inspection and the required Cargo checks completed successfully afterward. **Recommendation:** retain the successful replacement evidence; no implementation action. **Required next state:** `complete`.

## Changed-file and diff comparison

The recorded implementation changed [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1) and appended its evidence to [`execution-log.md`](../execution-log.md:17). The source diff adds bounded diagnostic storage/observation, a no-freeze live-width policy seam, command observations, and tests. It does not change requirements, blueprints, v1 artifacts, core crates, or V2-WO-002/003/004 implementation. The current diff is therefore aligned with the authorized file/system boundary and the work-order durable-document classification.

## Decision

- **Outcome:** `pass`
- **Severity:** no blocker; one optional native-smoke warning and one tooling note
- **Required next state:** `complete`
- **Fix work order required:** `no`
- **Recommendation:** Set V2-WO-001 to `complete`. Do not create a letter-suffixed fix work order. Preserve the warning that native macOS display/smoke evidence is unavailable; this is non-blocking under project policy and does not support a native success or exact restore claim. Continue only with the separately scoped V2-WO-002 work order after orchestration records completion.
- **Implementation/design claim boundary:** This artifact validates V2-WO-001 alignment only. It does not validate or claim completion of V2-WO-002, V2-WO-003, or V2-WO-004.

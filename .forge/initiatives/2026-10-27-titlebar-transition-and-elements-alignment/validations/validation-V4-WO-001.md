# Validation: V4-WO-001 — Vanilla Native Titlebar Comparison

## Validation metadata

- **Review type:** `validation`
- **Status:** `in-review`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** Not exposed by the execution interface; no identifier inferred.
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Initiative:** [`titlebar-transition-and-elements-alignment-v4.md`](../titlebar-transition-and-elements-alignment-v4.md:1)
- **Validation file:** [`validation-V4-WO-001.md`](validation-V4-WO-001.md:1)
- **Validated work order:** [`V4-WO-001-vanilla-native-titlebar-comparison.md`](../work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:1)
- **Compared commit/diff:** Current working tree on `dev/2026-10-27-titlebar-transition-and-elements-alignment-v3`; no commit comparison. See the dirty-tree and attribution limits below.
- **Fix chain:** `none`

## Authorized scope

This validation assesses V4-WO-001 only: the audited ordinary native macOS startup defaults, selection of the reversible native presentation path, suppression of the app-owned title strip and maximize response in that selected path, preservation of the custom hidden-titlebar rollback path and specified toolbar/content contracts, required focused/workspace checks, and the ordered native-chrome plus maximize/restore user checkpoint. V1–V3 implementation is not assessed, except for the work order's explicit preservation boundary and its cited prior evidence.

Comparison basis: [`05-validation.md`](../../../process/05-validation.md:1), [`.forge/_index.md`](../../../_index.md:1), [`project-manifest.md`](../../../project-manifest.md:73), the approved [`v4 plan`](../titlebar-transition-and-elements-alignment-v4.md:1) and [`v4 design review`](../reviews/design-review-v4.md:1), [`V4-WO-001`](../work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:1), the V4 implementation and user-gate entries in [`execution-log.md`](../execution-log.md:432), current [`main.rs`](../../../../apps/scriptcraft/src/main.rs:47), [`Cargo.toml`](../../../../Cargo.toml:1), [`apps/scriptcraft/Cargo.toml`](../../../../apps/scriptcraft/Cargo.toml:1), [`Cargo.lock`](../../../../Cargo.lock:875), the affected requirement [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90), the [`scriptcraft desktop blueprint`](../../../blueprints/containers/scriptcraft-desktop.md:105), and the cited preservation validation [`validation-V3-WO-001b.md`](validation-V3-WO-001b.md:48). The resolved API/default audit was read from the implementation evidence in the work order and execution log; no technical cause is inferred from the visual report.

The current Git status shows two modified tracked files—[`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1) and [`execution-log.md`](../execution-log.md:1)—and 14 untracked V3/V4 Forge initiative artifacts. `git diff --numstat` reports 657 insertions/56 deletions in `main.rs` and 195 insertions in the execution log (852 insertions/56 deletions total). The V4 preflight entry records that these tracked files and the V3/V4 artifacts were already dirty/untracked before V4 implementation, with no reset or checkout. The aggregate source diff therefore includes cumulative pre-existing work; it is not attributed wholesale to V4. The V4-relevant startup/options/surface/test hunks were compared to the authorized scope, and the current file state and implementation evidence agree on the selected native path and retained custom path. No other application/core source or dependency manifest/lockfile change appears in the current status. This validation does not assess the unrelated V3 changes.

## Validation checklist

- **Meaningful comparison: PASS.** The work order is `in-review`; its implementation evidence, current source, current tracked diff/status, focused tests, policy-check results, and direct user visual report are available. The aggregate diff is dirty/cumulative, so attribution is limited as described above, but the V4 startup delta itself can be compared meaningfully.
- **Authorized scope: PASS.** The V4-specific source changes select [`NativeMacOs`](../../../../apps/scriptcraft/src/main.rs:49), use framework defaults for that mode, retain [`CustomHidden`](../../../../apps/scriptcraft/src/main.rs:54), and keep the selected mode/options together through the sole runner handoff. Native mode omits the app title strip and app-owned maximize/policy response; the toolbar remains in app content. The recorded implementation files—source, work order, and execution log—are those authorized by the work order. The work-order and log updates are present as untracked/modified artifacts in the pre-existing dirty tree.
- **Rogue behavior: PASS.** No clearly unrelated, deceptive, backdoor, malicious, nefarious, or rogue V4 behavior was found in the compared V4 changes. No requirement, blueprint, dependency, core source, or unrelated application source change is shown in current Git status.
- **Required criteria and policy gates: PASS.** The focused tests, clean Cargo checks/build/lint, workspace suite, and whitespace gate were independently rerun successfully (details below). The direct user observation satisfies the native-chrome prerequisite and supplies separate maximize and restore animation observations. The title/color observations do not add acceptance criteria beyond the approved work order.
- **Durable-document classification: PASS.** Requirements and blueprint are correctly classified `verify unchanged`; current Git status shows no modifications to either durable document. No durable product decision is inferred.

## Findings

### Blockers

- None.

### Warnings

- **V4-WO-001-VAL-W-001 — Pre-existing cumulative dirty tree limits exact per-WO diff attribution.** **Severity:** warning. **Evidence:** The current tracked source diff includes substantially more than the V4 startup/options change, while the pre-implementation V4 preflight records the source and execution log as already modified and the initiative V3/V4 artifacts as already untracked. The current source diff and V4 evidence are consistent for the authorized V4 path, but the aggregate 657-addition/56-deletion source delta must not be represented as V4-only. **Recommendation:** Preserve this attribution boundary in subsequent orchestration; do not reopen or claim validation of unrelated V3 work. **Required next state:** `complete` (recommendation only; this validation does not change work-order status).

### Notes

- **V4-WO-001-VAL-N-001 — Animation comparison result.** **Severity:** note. **Evidence:** The user reported ordinary native chrome visible and the toolbar usable, followed by no swelling, shrinking, or final snap in either maximize or restore direction. This satisfies the ordered native-chrome prerequisite as recorded in the execution log and classifies the result as **absent in both directions** against the known custom-path swell/shrink/snap comparator. This is a positive answer to the native-animation question, not a claim that the animation defect is universally fixed. **Recommendation:** Retain the classification as experiment evidence only. **Required next state:** `complete`.
- **V4-WO-001-VAL-N-002 — Title and titlebar color remain follow-up observations, not V4 acceptance failures.** **Severity:** note. **Evidence:** The user additionally reported that the title reappeared by the traffic lights and visible titlebar color prevents the desired seamless visual continuation. V4-WO-001's acceptance test is the ordinary native baseline, visible-native-chrome prerequisite, and separate maximize/restore observation/classification; it does not require a titleless native bar, seamless app/native color continuation, or a permanent visual design. The work order explicitly frames the experiment as reversible evidence gathering, not a product fix or permanent design choice. **Recommendation:** Preserve these as unresolved visual-design observations for separately authorized follow-up; do not expand or fail V4-WO-001 on them, and do not infer a technical cause. **Required next state:** `complete`.
- **V4-WO-001-VAL-N-003 — Preservation and durable documents.** **Severity:** note. **Evidence:** The inspected current source retains `TOOLBAR_ELEMENTS_WIDTH = 128.0` at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:28), `TOOLBAR_BOTTOM_PADDING = 2.0` at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:37), and the existing chrome-derived clear-color path at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:2290). The V4 evidence reports the other named toolbar/content contracts unchanged, and the prior [`validation-V3-WO-001b.md`](validation-V3-WO-001b.md:48) is cited only as the preservation baseline, not revalidated here. The requirements and desktop blueprint remain unchanged. **Recommendation:** Keep these boundaries intact for this experiment. **Required next state:** `complete`.

## Exact checks and results

Independent command chain exited 0 after [`cargo clean`](../../../project-manifest.md:80):

| Check | Result |
|---|---|
| [`cargo clean`](../../../project-manifest.md:80) | **PASS** — removed 4,358 generated files (974.3 MiB); no source or Forge artifact was cleaned. |
| [`cargo fmt --all -- --check`](../../../project-manifest.md:81) | **PASS**. |
| Focused startup/default/rollback test `native_startup_selects_vanilla_defaults_and_custom_path_is_reversible` | **PASS** — 1 passed. |
| Focused surface/policy test `titlebar_mode_selects_custom_surface_and_policy_explicitly` | **PASS** — 1 passed. |
| Focused known/unknown maximize-command test `title_strip_viewport_toggle_is_safe_for_known_and_unknown_state` | **PASS** — 1 passed. |
| [`cargo check --workspace`](../../../project-manifest.md:84) | **PASS**. |
| [`cargo test --workspace`](../../../project-manifest.md:83) | **PASS** — 88 app unit tests, 45 core unit tests, integration tests, and doc-tests passed. |
| [`cargo build --workspace`](../../../project-manifest.md:85) | **PASS**. |
| [`cargo clippy --workspace --all-targets -- -D warnings`](../../../project-manifest.md:82) | **PASS**. |
| [`git diff --check`](../execution-log.md:439) | **PASS**. |
| Current status/diff inspection | **PASS WITH WARNING** — branch is the recorded V3 continuation branch; tracked diff is limited to `main.rs` and the execution log; pre-existing untracked V3/V4 Forge artifacts are accounted for above. |

The direct visual evidence is user-supplied, not inferred from tests. No separate screenshot is required by the work order. The resolved API/default evidence records eframe/egui-winit `0.29.1` and winit `0.30.13`; the unset native presentation fields map to ordinary decorated/opaque defaults with macOS title, titlebar, and buttons shown and full-size content disabled, while the runner supplies the product title. This describes the audited requested/default configuration, not the cause of the user's remaining title/color observations.

## Changed-file and diff comparison

- [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:47): the selected compile-time mode is [`NativeMacOs`](../../../../apps/scriptcraft/src/main.rs:49); native options resolve to [`eframe::NativeOptions::default()`](../../../../apps/scriptcraft/src/main.rs:2576), while the reversible [`CustomHidden`](../../../../apps/scriptcraft/src/main.rs:53) option branch remains. The mode-gated update omits custom title-strip rendering in native mode, and the selected `StartupPresentation` is passed directly to the single `eframe::run_native()` handoff at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:2580). Focused tests cover default/unset options, selected startup, reversibility, surface/policy ownership, and native-mode no-op for the app-owned maximize command. The rest of the current source diff is cumulative pre-existing work and is not attributed to V4.
- [`V4-WO-001-vanilla-native-titlebar-comparison.md`](../work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:87): contains the API/default audit, implementation and preservation evidence, check results, and the recorded visual gate; it remains `in-review`.
- [`execution-log.md`](../execution-log.md:432): records the implementation and direct user visual report. Its preflight entry identifies the already-dirty state; its user-gate entry records the animation classification and does not claim seamless titlebar/color presentation.
- No requirements, desktop blueprint, dependency manifest/lockfile, core source, unrelated application source, or validation artifact other than this report was changed by this validation. The V3 artifacts are outside assessment scope except for the explicit preservation boundary above.

## Decision

- **Outcome:** `PASS WITH WARNINGS`
- **Severity/evidence:** No implementation blocker or scoped acceptance failure was found. Required automated checks passed, the implementation evidence and relevant current source/diff agree on the authorized native-baseline change, and the user supplied both ordered visual observations. The warning records only the pre-existing cumulative dirty-tree attribution limit.
- **Recommendation:** Recommend V4-WO-001 `complete` under the Forge validation process. This is a recommendation only; the work order remains `in-review` until the orchestrator applies its own state transition. Keep the no-swell/no-snap result as native comparison evidence and carry the title-by-traffic-lights and visible-color observations only as follow-up, without claiming seamless design resolution or inferring cause.
- **Required next state:** `complete` (recommended; status not changed here).
- **Fix work order required:** `no`.

## Validation boundary

This artifact is the only file created or changed by this validation. No source, plan, work order, design review, execution log, V3/historical artifact, requirement, or blueprint was modified. No branch or work-order state change was made.

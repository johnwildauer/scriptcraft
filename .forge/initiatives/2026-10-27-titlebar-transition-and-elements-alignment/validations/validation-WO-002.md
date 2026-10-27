# Validation: WO-002 — Unified Title-Bar/Toolbar Surface and Transition Geometry

## Validation metadata

- **Review type:** `validation`
- **Status:** `in-review`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `validator-fresh-context-wo002-titlebar-2026-10-27T1737Z`
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Initiative:** [`titlebar-transition-and-elements-alignment-v1.md`](../titlebar-transition-and-elements-alignment-v1.md:1)
- **Validation file:** `validation-WO-002.md`
- **Validated work order:** [`WO-002-unified-titlebar-toolbar-surface-and-transition-geometry.md`](../work-orders/WO-002-unified-titlebar-toolbar-surface-and-transition-geometry.md:1)
- **Compared commit/diff:** Working-tree diff on `dev/2026-10-27-titlebar-transition-and-elements-alignment`, against the recorded `master` base `5b8eda725f767fbed530c021882b15bec82b6ceb`; implementation is uncommitted, so status/stat, source/blueprint diff, test output, and `git diff --check` are the comparable evidence.
- **Fix chain:** `none`

## Outcome summary

WO-002 is aligned and passes validation. The implementation replaces the adjacent title and toolbar panels with one top panel, one explicit app-chrome frame, no separator, and no frame stroke. The title remains a 30-point centered bold `ScriptCraft` region with full-width response allocation. WO-001 known/unknown viewport direction and no-op behavior remains present, and the 128-point Elements allocation is preserved. Deterministic geometry and metric tests use explicit `0.5`-point geometry and `0.01` metric tolerances. Stability claims remain limited to fixed-width/available-width behavior supported by the implementation; no animation-control API or universal native-frame promise was added.

Native macOS transition/seam smoke was unavailable. This is a warning only: [`project-manifest.md`](../../../project-manifest.md:88-95) configures no required native/manual gate, so the missing smoke does not block meaningful deterministic comparison.

## Authorized scope and comparison basis

This validation assesses [`WO-002-unified-titlebar-toolbar-surface-and-transition-geometry.md`](../work-orders/WO-002-unified-titlebar-toolbar-surface-and-transition-geometry.md:16-88) only: unified title/toolbar app-chrome ownership, seam/stroke/inset behavior, centered title and full-width hit coverage, deterministic rectangle/metric evidence, bounded transition-stability wording, preservation of WO-001 viewport behavior and the 128-point Elements baseline, applicable policy checks, and the declared desktop-blueprint impact. WO-003 and WO-004 are excluded.

The comparison uses the in-review work order, follow-up plan/design review, completed [`validation-WO-001.md`](validation-WO-001.md:1), follow-up [`execution-log.md`](../execution-log.md:51-64), current working-tree diff, relevant [`main.rs`](../../../../apps/scriptcraft/src/main.rs:25-40), installed eframe/egui API evidence recorded by the prior artifacts, and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:108-129). The implementation evidence is sufficient for meaningful comparison.

## Exact checks and results

| Check | Result |
|---|---|
| Work-order state | PASS — [`WO-002-unified-titlebar-toolbar-surface-and-transition-geometry.md`](../work-orders/WO-002-unified-titlebar-toolbar-surface-and-transition-geometry.md:5) is `in-review`. |
| Fresh-session evidence | PASS — implementation and validation have distinct session IDs; WO-003/004 were explicitly excluded. |
| `git status --short --branch` | PASS — exit 0; expected initiative branch, modified [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1), modified [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:1), and untracked initiative evidence. |
| `git diff --stat` / source and blueprint diff | PASS — exit 0; tracked diff is bounded to the app shell, its declared blueprint contract, and initiative evidence. |
| `git diff --check` | PASS — exit 0; no whitespace errors. |
| `cargo fmt --all -- --check` | PASS — exit 0. |
| Focused geometry test | PASS — `shared_chrome_geometry_has_no_gap_and_full_width_title_hit_region` passed. |
| Focused metric tests | PASS — both `presentation_metrics` tests passed. |
| Focused title/viewport tests | PASS — centered title and known/unknown viewport tests passed, 2/2. |
| `cargo check --workspace` | PASS — exit 0. |
| `cargo test --workspace` | PASS — exit 0; 72 app tests, 45 core tests, and integration suites passed. |
| `cargo build --workspace` | PASS — exit 0. |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS — exit 0. |
| Native macOS transition/seam smoke | WARNING — unavailable; optional under project policy, not a blocker. |

## Validation checklist

- **Evidence/preconditions: PASS.** The work order is `in-review`, implementation evidence and changed-file records are present, the follow-up log records attempts/results and native-smoke unavailability, and the current diff permits direct comparison.
- **Unified app-chrome surface: PASS.** [`update()`](../../../../apps/scriptcraft/src/main.rs:1789) creates one `TopBottomPanel`; [`chrome_frame()`](../../../../apps/scriptcraft/src/main.rs:1699) supplies the shared fill with `Stroke::NONE`; `.show_separator_line(false)` disables the panel separator. The title and toolbar therefore have one app-owned surface without the former adjacent-panel ownership seam.
- **Title geometry and hit coverage: PASS.** [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:1764) allocates `ui.available_width()` for exactly `TITLE_STRIP_HEIGHT`, uses strong `ScriptCraft` text, and centers the measured galley. [`chrome_geometry()`](../../../../apps/scriptcraft/src/main.rs:1717) and its test assert panel-edge title coverage and a contiguous title-to-toolbar boundary.
- **Insets/strokes/seam: PASS.** [`chrome_frame()`](../../../../apps/scriptcraft/src/main.rs:1699) uses a zero-inset `Frame::none()` with no stroke; the tested toolbar frame asserts zero inner/outer margins and `Stroke::NONE`; the panel separator is disabled. No unintended separator, doubled stroke, or inset is introduced by the WO-002 diff.
- **WO-001 preservation: PASS.** [`title_strip_viewport_command()`](../../../../apps/scriptcraft/src/main.rs:1756) still maps known normal to `Maximized(true)`, known maximized to `Maximized(false)`, and unknown to no command. The completed [`validation-WO-001.md`](validation-WO-001.md:29-38) establishes this as the prior validated behavior, and the current focused/full test runs preserve it.
- **Elements baseline: PASS.** [`TOOLBAR_ELEMENTS_WIDTH`](../../../../apps/scriptcraft/src/main.rs:26) remains `128.0`; WO-002 does not implement closed selected-text alignment, and its test retains the 128-point assertion. WO-003 remains outside scope.
- **Deterministic geometry and metric tolerances: PASS.** [`CHROME_GEOMETRY_TOLERANCE`](../../../../apps/scriptcraft/src/main.rs:37-40) is `0.5`; [`TRANSITION_METRIC_TOLERANCE`](../../../../apps/scriptcraft/src/main.rs:37-40) is `0.01`. Geometry tests use the geometry tolerance for seam/boundary comparisons, and normal/maximized metric tests use explicit scale/page/font comparisons. Narrow-width behavior is tested as a bounded limitation.
- **Transition stability/API boundary: PASS.** [`presentation_metrics()`](../../../../apps/scriptcraft/src/main.rs:1537) exposes the measured width-derived behavior used by tests. The work-order and log explicitly state that installed eframe/egui-winit APIs do not provide animation progress, duration, interpolation, or pixel-freeze control; no unsupported animation-control promise, pixel-identical transition claim, or universal exact frame-history claim is present.
- **Desktop blueprint impact: PASS.** The diff updates [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:108-129) exactly where the durable shell contract changes: unified surface, zero-inset/no-separator frame ownership, and platform-bounded restoration. Requirements are correctly verified unchanged; no unrelated durable document changed.
- **Scope and rogue behavior: PASS.** The diff is limited to the app shell geometry/metrics/tests, the matching desktop blueprint contract, and Forge evidence. No WO-003/004 behavior, core behavior, persistence/export behavior, viewport direction rewrite, or malicious/deceptive behavior was observed.
- **Required policy checks: PASS.** All configured Cargo and whitespace checks passed. Native smoke is not required by project policy and is recorded as a warning rather than a hard stop.

## Findings

### Blockers

- **None.** No plainly unmet authorized criterion, unrelated or rogue implementation, required-policy failure, or comparison blocker was found.

### Warnings

- **WO002-VAL-W-001 — Native macOS transition/seam smoke unavailable.** **Severity:** warning. **Evidence:** [`execution-log.md`](../execution-log.md:61-64) records no window-capable macOS run, OS/backend transition samples, seam-pixel observation, or native rectangle trace. **Recommendation:** Preserve the platform limitation and do not claim native animation-frame or pixel-level seam results until a window-capable run exists. **Required next state:** `complete`.

### Notes

- **WO002-VAL-N-001 — Working-tree implementation is uncommitted.** **Severity:** note. **Evidence:** current status shows the expected modified source/blueprint and untracked initiative evidence. **Recommendation:** Commit through normal Forge closeout. **Required next state:** `complete`.
- **WO002-VAL-N-002 — Diagnostic source review.** **Severity:** note. Candidate sources considered were separate panel ownership, frame fill/stroke mismatch, hidden frame margins, title allocation not covering the full panel, stale WO-001 behavior, changed Elements width, width-derived metric drift, unsupported animation-control assumptions, missing test tolerances, and unrelated WO-003/004 changes. Source/API inspection, deterministic tests, diff comparison, and policy checks reduce these to no hard-stop finding; only optional native comparison remains unavailable. **Recommendation:** No implementation change. **Required next state:** `complete`.

## Changed-file and diff comparison

The tracked implementation diff changes [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1699) for shared chrome frame/panel ownership, deterministic geometry/metric helpers/tests, and the combined title/toolbar update. It preserves the WO-001 command mapping, `TITLE_STRIP_TEXT`, title height, and `TOOLBAR_ELEMENTS_WIDTH = 128.0`. The desktop blueprint diff updates the durable shell description to match the unified surface and its bounded native behavior. The remaining initiative files contain implementation and validation evidence; no WO-003/004 source or behavior is present. This matches the authorized files/systems and the work order's `Blueprint impact: update` classification.

## Decision

- **Outcome:** `pass`
- **Required next state:** `complete`
- **Fix work order required:** `no`
- **Recommendation:** Set WO-002 to `complete`. Do not create a letter-suffixed fix work order; the unavailable native macOS transition/seam smoke is a non-blocking warning under project policy, and deterministic source/API comparison passes. Preserve the warning and the bounded transition-stability claim for later native-capable validation.
- **Implementation/design claim boundary:** This artifact validates WO-002 only. It does not validate or claim completion of WO-003 or WO-004.

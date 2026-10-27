# Validation: WO-002 — Custom Title Strip and Viewport Toggle

## Validation metadata

- **Review type:** `validation`
- **Status:** `in-review`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `validator-fresh-context-wo002-title-strip-2026-10-27T1559Z`
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Initiative:** [`toolbar-and-titlebar-polish-v1.md`](../toolbar-and-titlebar-polish-v1.md:1)
- **Validation file:** `validation-WO-002.md`
- **Validated work order:** [`WO-002-custom-title-strip-and-viewport-toggle.md`](../work-orders/WO-002-custom-title-strip-and-viewport-toggle.md:1)
- **Compared commit/diff:** Working-tree diff on `dev/2026-10-27-toolbar-and-titlebar-polish`, against recorded base `master` commit `f5b02d3eb58464f091c57667711f3be738e65e56`; implementation is uncommitted, so `git diff master...HEAD` is empty and the comparable evidence is the working-tree diff plus status/stat output.
- **Fix chain:** `none`

## Authorized scope

This validation assesses WO-002 only: the dedicated title strip above the toolbar,
separate title-strip height, exact centered bold `ScriptCraft` placeholder,
stable double-click response, known-state inverse maximize command, safe unknown
state, platform/backend-bounded restoration, preservation of existing toolbar
behavior, and the authorized requirements/desktop-blueprint impact. WO-001 is
treated only as the already-validated baseline; WO-003 and WO-004 are excluded.
Native smoke is optional under project policy and is recorded as a warning when
absent. No application source or work-order implementation was modified.

The comparison basis was the `in-review` work order, its implementation evidence,
the execution log, initiative plan, design review, requirements and blueprint,
the recorded WO-001 validation, installed eframe/egui API source, current branch
status, and the working-tree diff for [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1643), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:131), and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:107).

## Validation checklist

- **Evidence/preconditions: PASS.** [`WO-002`](../work-orders/WO-002-custom-title-strip-and-viewport-toggle.md:5) is `in-review`; the implementer, changed files, command attempts/results, native-smoke limitation, and comparable diff evidence are recorded. The current branch and working-tree diff permit meaningful comparison.
- **Boundary/comparability: PASS.** The source diff contains the title-strip renderer/panel, viewport helper, focused tests, and the already-recorded WO-001 baseline. The requirements and desktop blueprint changes are the two authorized durable-document updates. No core, persistence, export, toolbar-command, WO-003, or WO-004 implementation was observed.
- **Title-strip geometry: PASS.** [`update()`](../../../../apps/scriptcraft/src/main.rs:1677) submits a distinct `title-strip` top panel before the `toolbar` top panel and applies [`TITLE_STRIP_HEIGHT`](../../../../apps/scriptcraft/src/main.rs:36) through `exact_height`; toolbar geometry remains separately controlled by [`TOOLBAR_TOP_PADDING`](../../../../apps/scriptcraft/src/main.rs:32). The focused test verifies the independent 30-point title height and the separate combined geometry value.
- **Text and filename boundary: PASS.** [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:1655) paints only the constant [`TITLE_STRIP_TEXT`](../../../../apps/scriptcraft/src/main.rs:35), uses [`RichText::strong()`](../../../../apps/scriptcraft/src/main.rs:1657), and centers the measured galley with [`title_strip_text_position()`](../../../../apps/scriptcraft/src/main.rs:1647). It does not read or mutate [`native_path`](../../../../apps/scriptcraft/src/main.rs:457), which remains status/path state outside the title-strip renderer.
- **Double-click and viewport command: PASS.** The full-strip [`Response`](../../../../apps/scriptcraft/src/main.rs:1656) owns [`double_clicked()`](../../../../apps/scriptcraft/src/main.rs:1669). [`title_strip_viewport_command()`](../../../../apps/scriptcraft/src/main.rs:1651) maps `Some(false)` to the supported inverse maximize command, `Some(true)` to unmaximize, and `None` to no command; the focused test covers all three cases.
- **Restoration claim boundary: PASS.** The implementation does not synthesize frame history or claim universal exact restoration. It relies on native/backend unmaximize behavior, matching the installed API evidence that [`ViewportInfo::maximized`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/data/input.rs:227) and [`ViewportInfo::outer_rect`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/data/input.rs:218) are optional and that [`ViewportCommand::Maximized(bool)`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/viewport.rs:1006) is a request, not a universal frame-history contract.
- **Prior toolbar behavior: PASS.** The title panel is inserted before the existing toolbar panel; no toolbar command order/dispatch or editor response code was changed by the WO-002-specific hunk. The full workspace test suite and the prior focused toolbar behavior remain passing.
- **Durable-document impact: PASS.** WO-002 classifies requirements and blueprint impact as `update/update`. The diff adds [`AC-WRITING-005.6`](../../../requirements/features/screenplay-writing-loop.md:131) and the matching bounded desktop-shell contract at [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:107), with no unrelated durable-document change.
- **Project-policy checks: PASS.** All configured Cargo and diff checks passed. No required manual/native gate is configured in [`project-manifest.md`](../../../project-manifest.md:88).
- **Rogue behavior: PASS.** No deceptive, backdoor, malicious, nefarious, or otherwise unrelated behavior was observed.

## Findings

### Blockers

- None. No plainly unmet authorized criterion, unrelated implementation, required-policy failure, or comparison blocker was found.

### Warnings

- **WO002-VAL-W-001 — Native maximize/restore smoke unavailable.** **Severity:** warning. **Evidence:** The work order and execution log record that no window-capable native maximize/restore run was available; deterministic tests cover geometry, text centering, and known/unknown command selection only. The project manifest defines no native smoke as a required manual gate. **Recommendation:** Preserve the OS/backend limitation and do not claim observed native maximize, unmaximize, or visible frame restoration. If a supported native run becomes available, record OS, backend, observed rectangles, and ignored-command behavior. **Required next state:** `complete`.

### Notes

- **WO002-VAL-N-001 — Working-tree implementation is uncommitted.** **Severity:** note. **Evidence:** `git diff master...HEAD` is empty while the working-tree diff and status show the expected tracked implementation/durable files and untracked Forge evidence. **Recommendation:** Commit through normal Forge closeout. **Required next state:** `complete`.
- **WO002-VAL-N-002 — Diagnostic source review.** **Severity:** note. Plausible sources considered were (1) panel ordering/inset double-counting, (2) title height being folded into toolbar padding, (3) placeholder text accidentally reading filename state, (4) unstable or partial double-click ownership, (5) inverted/unsupported maximize command selection, (6) unsafe handling of unknown viewport state, and (7) overstated restoration or durable-document scope. The focused tests, source/diff inspection, API evidence, full policy checks, and filename-state search reduce these to no hard-stop finding; only native visual/backend comparison remains unavailable. **Recommendation:** No implementation change. **Required next state:** `complete`.

## Exact commands and results

| Command | Result |
|---|---|
| `git status --short --branch` | Exit 0; branch `dev/2026-10-27-toolbar-and-titlebar-polish`; three tracked files modified and the initiative evidence directory untracked. |
| `git diff --stat` | Exit 0; 3 tracked files, 159 insertions, 19 deletions; WO-002-specific behavior was isolated from the pre-existing WO-001 baseline during review. |
| `git diff --check` | Exit 0; no whitespace errors. |
| `cargo fmt --all -- --check` | Exit 0. |
| `cargo check --workspace` | Exit 0. |
| `cargo test -p scriptcraft title_strip` | Exit 0; 2 focused tests passed, 0 failed. |
| `cargo test --workspace` | Exit 0; 67 app tests and 44 core tests passed; existing integration suites passed. |
| `cargo build --workspace` | Exit 0. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0. |
| `git diff master...HEAD --stat` | Exit 0; no committed delta because implementation changes are uncommitted; working-tree diff used for comparison. |
| Installed API inspection with `grep`/`sed` | Exit 0; egui 0.29.1 exposes optional maximized/outer-rectangle state and `ViewportCommand::Maximized(bool)`; the inspected viewport update path supplies geometry conditionally. |

## Diff/file comparison

WO-002-specific source changes in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1643) are limited to the title-strip frame/text-position/viewport-command helpers, the full-strip response and double-click dispatch, the separate top panel in [`update()`](../../../../apps/scriptcraft/src/main.rs:1677), and two focused tests. The broader source diff also contains the previously validated WO-001 toolbar changes; those were not revalidated as WO-002 implementation and do not create a boundary conflict.

The requirements diff adds only the title-strip acceptance criterion at [`AC-WRITING-005.6`](../../../requirements/features/screenplay-writing-loop.md:131). The blueprint diff adds only the matching custom title-strip and bounded viewport contract at [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:107). This matches the work order's `update/update` classification. The execution log and work-order records are the expected Forge evidence updates. No WO-003 or WO-004 source/validation artifact was assessed or claimed.

## Decision

- **Outcome:** `PASS WITH WARNINGS`
- **Required next state:** `complete`
- **Fix work order required:** `no`
- **WO-002 disposition:** WO-002 should become `complete`; no letter-suffixed fix work order is warranted. The missing native smoke is a non-blocking warning under project policy and does not prevent meaningful deterministic comparison.
- **Implementation/design claim boundary:** This artifact validates WO-002 alignment only. It does not validate or claim completion of WO-003 or WO-004.

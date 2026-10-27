# Validation: WO-004 — Cross-Surface Native Validation and Durable-Document Reconciliation

## Validation metadata

- **Review type:** `validation`
- **Status:** `in-review`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `validator-fresh-context-wo004-cross-surface-2026-10-27T1746Z`
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Initiative:** [`titlebar-transition-and-elements-alignment-v1.md`](../titlebar-transition-and-elements-alignment-v1.md:1)
- **Validation file:** `validation-WO-004.md`
- **Validated work order:** [`WO-004-cross-surface-native-validation-and-document-reconciliation.md`](../work-orders/WO-004-cross-surface-native-validation-and-document-reconciliation.md:1)
- **Compared commit/diff:** Working-tree diff on `dev/2026-10-27-titlebar-transition-and-elements-alignment` against recorded `master` base `5b8eda725f767fbed530c021882b15bec82b6ceb`; the implementation is uncommitted, so status/stat, source and blueprint diff inspection, recorded changed-file evidence, fresh checks, and `git diff --check` are the comparable evidence.
- **Fix chain:** `none`

## Outcome summary

WO-004 passes validation with one native-display warning and one working-tree note. The reconciliation boundary is real and bounded: the integrated source and prior validation evidence cover viewport direction, unified zero-seam title/toolbar geometry, bounded transition metrics, closed/open Elements behavior at the preserved 128-point width, cursor ownership, title behavior, and project-policy gates. No WO-004 source correction or test change was made, no administrative-only completion claim was used, and no unsupported native behavior was claimed. Requirements and blueprints are correctly classified as `verify unchanged` for this work order.

Native display evidence remains unavailable in this session. Under project policy this is a warning, not a blocker, because deterministic comparison remains possible and no native/manual gate is required.

## Authorized scope and comparison basis

This validation assesses [`WO-004-cross-surface-native-validation-and-document-reconciliation.md`](../work-orders/WO-004-cross-surface-native-validation-and-document-reconciliation.md:18) only: final cross-surface comparison of WO-001–WO-003, deterministic and configured checks, bounded macOS/native-smoke recording, source-change authorization, reconciliation of REQ-WRITING-LOOP-005 / AC-WRITING-005.4–005.7 and the affected desktop/feature/technical blueprints, and explicit limitation handling. Future scope, closeout, design-review substitution, and unrelated product behavior are excluded.

The comparison uses the `in-review` work order, the initiative plan and design review, completed [`validation-WO-001.md`](validation-WO-001.md:1), [`validation-WO-002.md`](validation-WO-002.md:1), [`validation-WO-003.md`](validation-WO-003.md:1), the follow-up [`execution-log.md`](../execution-log.md:102), the recorded implementation evidence in [`WO-004-cross-surface-native-validation-and-document-reconciliation.md`](../work-orders/WO-004-cross-surface-native-validation-and-document-reconciliation.md:100), current [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1675), current [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105), requirements, and the working-tree diff. Meaningful comparison is possible.

## Validation checklist

- **Evidence/preconditions: PASS.** [`WO-004-cross-surface-native-validation-and-document-reconciliation.md`](../work-orders/WO-004-cross-surface-native-validation-and-document-reconciliation.md:5) is `in-review`; changed files, attempts/results, native limitation, session identity, and comparable diff evidence are recorded.
- **Cross-surface boundary: PASS.** The evidence addresses the approved viewport, chrome, transition, Elements, cursor, title, and policy boundary without introducing future scope.
- **Source correction scope: PASS.** No source, test, requirements, or blueprint correction was made in WO-004. The current tracked source/blueprint diff is attributable to WO-001–WO-003; WO-004 changed only its evidence record and [`execution-log.md`](../execution-log.md:102).
- **No administrative shortcut: PASS.** The implementer explicitly records no speculative source change and leaves the work order `in-review`; this validator independently compared source/diff and checks rather than treating documentation as implementation.
- **Rogue behavior: PASS.** No deceptive, backdoor, malicious, nefarious, or unrelated behavior was observed.
- **Durable-document classification: PASS.** Requirements and affected blueprints remain `verify unchanged`, matching the final WO-004 diff and the already accurate durable wording; WO-003’s prior authorized blueprint update remains attributable to WO-003.
- **Project policy: PASS.** All configured automated gates passed. Native display absence is optional under [`project-manifest.md`](../../../project-manifest.md:88), so it is recorded as a warning rather than a hard stop.

## Findings

### Blockers

- None. No impossible comparison, clearly unrelated or rogue behavior, plainly unmet required criterion, or failed/unavailable required project-policy gate was found.

### Warnings

- **WO004-VAL-W-001 — Native display/window smoke unavailable.** **Severity:** warning. **Evidence:** The implementer record reports unset `DISPLAY`/`WAYLAND_DISPLAY`, no visible window, and `screencapture` failure, with no native click/double-click, OS/backend state, outer/inner/content/clip rectangles, seam pixels, transition frames, restore, popup, cursor, or keyboard observation; the fresh bounded run also launched [`scriptcraft`](../../../../apps/scriptcraft/src/main.rs:1912) but had no display and was terminated after the bounded interval. **Recommendation:** Preserve the limitation and do not upgrade deterministic/API evidence into native observation or exact restoration claims. **Required next state:** `complete`.

### Notes

- **WO004-VAL-N-001 — Working-tree evidence is uncommitted.** **Severity:** note. **Evidence:** Current status shows modified [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1), modified [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:1), and the untracked initiative evidence directory; no WO-004 runtime delta appears in the tracked diff. **Recommendation:** Commit through normal Forge closeout. **Required next state:** `complete`.
- **WO004-VAL-N-002 — Diagnostic source review.** **Severity:** note. Candidate causes considered were stale/unknown viewport state, reversed maximize request semantics, split panel/frame ownership, hidden inset/separator seam, width-derived metric drift, closed/open ComboBox ownership or arrow collision, and competing cursor/title response ownership. Installed API evidence, source inspection, deterministic measurements, focused/full checks, and diff comparison validate the bounded implementation and reduce these to no hard-stop finding. **Recommendation:** No implementation change. **Required next state:** `complete`.

## Exact checks and results

| Check | Result |
|---|---|
| Work-order state | PASS — [`WO-004-cross-surface-native-validation-and-document-reconciliation.md`](../work-orders/WO-004-cross-surface-native-validation-and-document-reconciliation.md:5) is `in-review`. |
| Fresh-session evidence | PASS — implementer `implementer-fresh-wo004-cross-surface-2026-10-27T1743Z` and validator `validator-fresh-context-wo004-cross-surface-2026-10-27T1746Z` are distinct; no exception. |
| `git status --short --branch` | PASS — expected initiative branch; modified source/blueprint and untracked initiative evidence. |
| `git diff --stat` / changed-file comparison | PASS — two tracked implementation/document files remain modified; no WO-004 runtime delta; initiative artifacts are untracked evidence. |
| `git diff --check` | PASS — exit 0; no whitespace errors. |
| `cargo fmt --all -- --check` | PASS — exit 0. |
| Focused title/chrome/metric/Elements/cursor/shared-toolbar tests | PASS — 22 tests passed, 0 failed across the six focused filters. |
| `cargo check --workspace` | PASS — exit 0. |
| `cargo test --workspace` | PASS — 75 app tests, 45 core tests, integration suites, and doc-tests passed; 0 failures. |
| `cargo build --workspace` | PASS — exit 0. |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS — exit 0. |
| Viewport direction | PASS — `Some(false)` requests `Maximized(true)`, `Some(true)` requests `Maximized(false)`, and `None` is a no-op at [`title_strip_viewport_command()`](../../../../apps/scriptcraft/src/main.rs:1798); no full-screen or app-owned frame-history claim. |
| Unified title/toolbar geometry | PASS — recorded panel `(12,4)-(812,86)`, title `(12,4)-(812,34)`, toolbar `(12,34)-(812,86)`; seam gap and boundary delta `0.0`, full-width title coverage `800.0`, within `0.5` tolerance; shared frame has zero margins and `Stroke::NONE`. |
| Transition metrics | PASS — normal/maximized-equivalent widths `800.0`/`1400.0` both measured page width `576.0`, scale `0.941176`, font size `11.294117`; deltas `0.0` within `0.01`; narrow `400.0` behavior is explicitly bounded. |
| Elements alignment | PASS — `TOOLBAR_ELEMENTS_WIDTH` remains `128.0`; closed text region width `104.0`, sample centered galley starts at `x=46.0`, arrow separation `4.0`; truncation/containment and closed-only versus stock left-aligned popup tests passed. |
| Cursor/title behavior | PASS — title remains centered bold `ScriptCraft` with full-width response; controls request `CursorIcon::Default`, document surface requests `CursorIcon::Text`, and ownership tests passed. |
| Native/window-capable smoke | WARNING — macOS process launched but no usable display/window observation was available; no native result is claimed. Optional under project policy. |

## Changed-file and diff comparison

The final working tree contains the pre-existing tracked implementation/document changes in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1) and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:1), plus untracked initiative evidence. Direct inspection shows those runtime/document deltas are the combined WO-001–WO-003 implementation and blueprint changes already independently validated. WO-004 added no application source, tests, requirements, or blueprint file. Its only implementation-session changes are the evidence sections in [`WO-004-cross-surface-native-validation-and-document-reconciliation.md`](../work-orders/WO-004-cross-surface-native-validation-and-document-reconciliation.md:100) and [`execution-log.md`](../execution-log.md:102).

The requirements classification is correctly `verify unchanged` for REQ-WRITING-LOOP-005 and AC-WRITING-005.4–005.7: the current wording already covers role labels/routing, 128-point allocation, bounded known-state maximize behavior, title strip, and cursor ownership. The blueprint classification is correctly `verify unchanged` for the integrated behavior because [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:78) and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:113) already record closed-only Elements centering, popup alignment, unified chrome, bounded native restoration, and cursor ownership. No documentation update is justified by the final WO-004 diff.

## Decision

- **Outcome:** `pass`
- **Required next state:** `complete`
- **Fix work order required:** `no`
- **Recommendation:** Set WO-004 to `complete`. Do not create a letter-suffixed fix work order; the source correction branch was not needed, deterministic cross-surface evidence and required policy gates pass, and unavailable native display evidence is a non-blocking warning under policy. Preserve the native-smoke limitation and all bounded no-claim language.
- **Implementation/design claim boundary:** This artifact validates WO-004 only. It does not replace initiative closeout, human approval, or native evidence that was unavailable, and it does not validate future scope.

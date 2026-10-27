# Validation: V2-WO-004 — Integrated Native Validation and Durable-Document Reconciliation

## Validation metadata

- **Review type:** `validation`
- **Status:** `in-review`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `validator-fresh-context-v2-wo004-integrated-native-validation-2026-10-27T1924Z`
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Initiative:** [`../titlebar-transition-and-elements-alignment-v2.md`](../titlebar-transition-and-elements-alignment-v2.md:1)
- **Validation file:** `validation-V2-WO-004.md`
- **Validated work order:** [`../work-orders/V2-WO-004-integrated-native-validation-and-document-reconciliation.md`](../work-orders/V2-WO-004-integrated-native-validation-and-document-reconciliation.md:1)
- **Compared commit/diff:** Working-tree comparison on `dev/2026-10-27-titlebar-transition-and-elements-alignment-v2` against recorded `master` base `e77d6c4ee424403c5ab779d372745a51e4cafd4e`; aggregate tracked diff is [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1) plus [`execution-log.md`](../execution-log.md:1), with the v2 Forge artifacts untracked. V2-WO-004 itself changed only its work-order evidence and the appended log entry.
- **Fix chain:** `none`

## Authorized scope and comparison basis

This fresh validation assesses V2-WO-004 only: comparison of the completed V2-WO-001/002/003 ledgers, validations, diffs, tests, and API findings; integrated deterministic checks for viewport direction/metric policy, Elements response/cursor/popup/keyboard/128-point behavior, toolbar-bottom ownership/physical-row ledger, unified chrome, and project-policy gates; the native-display protocol and its limitation; any evidenced in-scope source correction; and requirements/blueprint reconciliation. Future scope, v1 completion, and unimplemented native behavior are excluded.

The comparison basis is the [`in-review` V2-WO-004 work order](../work-orders/V2-WO-004-integrated-native-validation-and-document-reconciliation.md:5), its implementation evidence, the v2 plan/design review, completed [`validation-V2-WO-001.md`](validation-V2-WO-001.md:1), [`validation-V2-WO-002.md`](validation-V2-WO-002.md:1), and [`validation-V2-WO-003.md`](validation-V2-WO-003.md:1), the v2 execution log, current branch status/diff, relevant [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1), installed eframe/egui/egui-winit/winit API evidence, requirements, blueprints, and [`project-manifest.md`](../../../project-manifest.md:73). Meaningful comparison is possible. No source, work-order, requirements, blueprint, v1 artifact, or prior validation was modified by this validation.

## Validation checklist

- **Evidence/preconditions: PASS.** [`V2-WO-004`](../work-orders/V2-WO-004-integrated-native-validation-and-document-reconciliation.md:5) is `in-review`; preceding V2 ledgers/validations are present and `pass`; changed-file records, deterministic attempts/results, native limitation, session identity, and comparable branch diff are recorded.
- **Integrated authorized behavior: PASS.** The aggregate source evidence preserves known-state viewport direction and unknown-state no-op, live-width metric policy, stock ComboBox response/ID/popup/keyboard path, non-sensing Elements caption, centered closed text, `128.0`-point width, document-only I-beam intent, unified chrome, and the bounded toolbar-bottom ledger. No V2-WO-004 runtime correction was added.
- **Correction boundary: PASS.** No evidenced mismatch justified a source or regression-test change. No blind native paint, speculative geometry/padding change, desktop polling, global egui scale lock, unsupported animation control, allocator replacement, full-screen behavior, filename plumbing, or exact restore-frame promise was introduced.
- **Rogue behavior: PASS.** No deceptive, backdoor, malicious, nefarious, or unrelated behavior was observed; no administrative-only completion claim substitutes for implementation evidence.
- **Required project policy: PASS.** Configured format, test, check, build, clippy, and whitespace gates passed. Native display/manual smoke is not a configured required gate under [`project-manifest.md`](../../../project-manifest.md:88-95), so its absence is recorded as a warning and not upgraded to success.
- **Durable documents: PASS.** Requirements [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90) AC-WRITING-005.4–005.7 and blueprints [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:59), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:78), and [`technical-context.md`](../../../blueprints/technical-context.md:31) were checked. `verify unchanged` matches the final V2-WO-004 diff because no supported integrated contract changed.

## Findings

### Blockers

- None. No comparison blocker, unrelated/rogue behavior, plainly unmet authorized criterion, or failed/unavailable required project-policy gate was found.

### Warnings

- **V2-WO-004-VAL-W-001 — Native display/window evidence unavailable.** **Severity:** warning. **Evidence:** independent bounded launch of `target/debug/scriptcraft` remained alive for 8 seconds with empty stdout/stderr; `DISPLAY` and `WAYLAND_DISPLAY` were unset; `screencapture` reported `could not create image from display`; no native window interaction or capture was available. Therefore maximize/restore clicks, native state/outer/inner/content/clip rectangles, transition frames, display scale, physical toolbar pixels, Elements cursor/popup/keyboard behavior, and native unified-chrome ownership are unavailable and unclaimed. **Recommendation:** retain the bounded native limitation and do not claim native success, pixel ownership, cursor success, or exact restore behavior. **Required next state:** `complete`.

### Notes

- **V2-WO-004-VAL-N-001 — Working-tree comparison is uncommitted.** **Severity:** note. **Evidence:** the required v2 branch is present, [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1) and [`execution-log.md`](../execution-log.md:1) are modified, and v2 Forge artifacts are untracked. **Recommendation:** preserve the normal Forge closeout commit/merge step; no implementation action. **Required next state:** `complete`.

## Exact checks and results

| Check | Exact result | Outcome |
|---|---|---|
| Work-order state | [`V2-WO-004`](../work-orders/V2-WO-004-integrated-native-validation-and-document-reconciliation.md:5) is `in-review`. | PASS |
| `git branch --show-current` | `dev/2026-10-27-titlebar-transition-and-elements-alignment-v2`. | PASS |
| `git status --short --branch` | Expected modified [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1)/log and untracked v2 Forge artifacts; no unexpected source family. | PASS |
| `git diff --stat` | Aggregate tracked diff: 486 added and 11 removed source lines plus 84 execution-log lines; V2-WO-004 adds no source line. | PASS |
| `git diff --check` | Exit 0; no whitespace errors. | PASS |
| `cargo fmt --all -- --check` | Exit 0. | PASS |
| Focused title/viewport tests | 2 title-strip tests passed; known normal → `Maximized(true)`, known maximized → `Maximized(false)`, unknown → no command. | PASS |
| Focused presentation metrics | 2 tests passed; normal/maximized-equivalent metrics remain page width `576.0`, scale `0.941176`, font `11.294117`; narrow width remains bounded. | PASS |
| Focused Elements/cursor tests | Elements cursor ledger 1 passed; cursor intents 1 passed; closed/control regions are standard arrow and document is text/I-beam. | PASS |
| Focused chrome/toolbar tests | Bottom ownership ledger 1, fractional physical-row 1, shared-chrome 1, and toolbar-allocation 6 passed. | PASS |
| `cargo test --workspace` | Exit 0; 81 app tests, 45 core tests, integration tests, and doc-tests passed. | PASS |
| `cargo check --workspace` | Exit 0. | PASS |
| `cargo build --workspace` | Exit 0. | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0. | PASS |
| Installed API/source review | PASS; egui/eframe/egui-winit expose observed optional viewport state and request-style `Maximized(bool)`, but no supported transition timeline, requested-resize flag, native pixel-owner API, or exact restore-frame contract. | PASS |
| Native macOS protocol | Darwin arm64 confirmed; process timeout/termination was clean, but no usable display/capture or native interaction was available. | WARNING; no native result claimed |

## Exact integrated criteria and results

- **Preceding ledger/validation/diff comparison: PASS.** V2-WO-001 preserves live-width metrics and safe known/unknown viewport requests; V2-WO-002 preserves stock ComboBox behavior while removing the decorative caption response; V2-WO-003 proves contiguous logical chrome and fractional physical-row derivation while leaving native pixel ownership unresolved. Their `pass` validations and optional native warnings agree with the aggregate source/diff.
- **Viewport direction and metric policy: PASS.** [`title_strip_viewport_command()`](../../../../apps/scriptcraft/src/main.rs:2118) uses the observed optional state with an unknown no-op; [`presentation_metric_policy()`](../../../../apps/scriptcraft/src/main.rs:1879) remains `LiveWidthDerived`, without an unsupported freeze or global egui scale mutation.
- **Elements cursor/popup/keyboard/128-point behavior: PASS deterministically.** [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:977) retains stock ComboBox response/ID/arrow/popup/role routing and paints closed text only; the decorative caption uses non-sensing allocation; [`TOOLBAR_ELEMENTS_WIDTH`](../../../../apps/scriptcraft/src/main.rs:28) remains `128.0`; focused ledger and full-suite tests pass. Native pointer/popup/keyboard observation remains unavailable.
- **Toolbar bottom ownership/ledger: PASS within the authorized evidence boundary.** [`ChromeSurfaceLedger`](../../../../apps/scriptcraft/src/main.rs:96) records panel/frame/title/toolbar row/bottom padding/central/clip and logical-to-physical rows; deterministic contiguous and fractional-row checks pass. Native/compositor ownership is unresolved rather than inferred, and no paint-over was added.
- **Unified chrome: PASS deterministically.** [`update()`](../../../../apps/scriptcraft/src/main.rs:2163) uses one exact-height top panel; [`chrome_frame()`](../../../../apps/scriptcraft/src/main.rs:2061) supplies the app fill with zero stroke/inset policy and the panel separator is disabled. The shared geometry test reports zero seam/boundary delta under the recorded tolerance.
- **Project-policy gates: PASS.** All configured automated gates passed; no manual/native gate is configured.
- **Native absence handling: PASS as limitation handling, not native success.** The exact unavailable observations are preserved above and in the work-order/log evidence. No native rectangle, cursor, popup, keyboard, pixel, transition, or restore claim is made.
- **Durable reconciliation: PASS.** The cited requirements/blueprints remain accurate for the supported contract; `verify unchanged` is not an administrative exemption and is supported by the absence of requirements/blueprint changes and the unchanged contract boundary.

## Diagnostic source review

The independent diagnosis considered seven plausible sources: stale or inverted viewport state/request direction; width-derived page/font metric changes during OS resize; missing transition/freeze API; decorative Elements response or stock ComboBox cursor precedence; document-response overlap; app-owned toolbar geometry/fill/padding; and native full-size-content/compositor or device-pixel ownership. Source/API review, the three preceding ledgers, deterministic tests, aggregate diff, and policy checks reduce the likely unresolved causes to native-display-dependent transition/cursor/pixel observations only. Those unresolved causes are evidence limitations, not grounds for a speculative correction.

## Changed-file and diff comparison

The aggregate implementation diff is attributable to V2-WO-001/002/003: [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1) contains bounded diagnostics, the Elements ownership correction, and deterministic ledgers/tests; [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:59) is part of the earlier v1/preceding evidence, not a V2-WO-004 change. V2-WO-004 changed only [`V2-WO-004-integrated-native-validation-and-document-reconciliation.md`](../work-orders/V2-WO-004-integrated-native-validation-and-document-reconciliation.md:1) and appended [`execution-log.md`](../execution-log.md:216). No application source, regression test, requirements, blueprint, prior validation, or v1 artifact changed for this work order. The final durable classification therefore correctly remains `verify unchanged`.

## Decision

- **Outcome:** `pass`
- **Severity:** no blocker; one optional native-display warning and one working-tree note.
- **Required next state:** `complete` for V2-WO-004.
- **Fix work order required:** `no`.
- **Recommendation:** Set V2-WO-004 to `complete`; do not create a letter-suffixed fix work order. Preserve the precise native-display absence warning and do not upgrade it to native success. The ordinary initiative closeout remains a separate process step.
- **Implementation/design claim boundary:** This artifact validates V2-WO-004 only. It does not validate future scope, add a source correction, or claim native display/window behavior that was not observed.

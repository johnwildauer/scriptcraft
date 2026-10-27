# Validation: WO-003 — Elements Closed Selected-Text Centering

## Validation metadata

- **Review type:** `validation`
- **Status:** `in-review`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `validator-fresh-context-wo003-elements-2026-10-27T1741Z`
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Initiative:** [`titlebar-transition-and-elements-alignment-v1.md`](../titlebar-transition-and-elements-alignment-v1.md:1)
- **Validation file:** `validation-WO-003.md`
- **Validated work order:** [`WO-003-elements-closed-selected-text-centering.md`](../work-orders/WO-003-elements-closed-selected-text-centering.md:1)
- **Compared commit/diff:** Working-tree diff on `dev/2026-10-27-titlebar-transition-and-elements-alignment` against recorded `master` base `5b8eda725f767fbed530c021882b15bec82b6ceb`; implementation is uncommitted, so status/stat, source/blueprint diff, installed egui source, test output, and `git diff --check` are the comparable evidence.
- **Fix chain:** `none`

## Outcome summary

WO-003 passes validation. The implementation keeps the stock egui ComboBox as the owner of the button response/ID, standard arrow, popup, keyboard/open behavior, widget role, and selectable-value routing. While closed, it suppresses only the stock selected galley and paints a truncated selected-role galley centered in the arrow-excluded content rectangle. The 128-point allocation, width/overflow/edge contracts, role routing, rejection preservation, and prior titlebar behavior remain intact. No WO-004 behavior was introduced.

Native visual smoke was unavailable. This is a warning only: [`project-manifest.md`](../../../project-manifest.md:88-95) configures no required native/manual gate.

## Authorized scope and comparison basis

This validation assesses [`WO-003-elements-closed-selected-text-centering.md`](../work-orders/WO-003-elements-closed-selected-text-centering.md:18) only: closed selected-text centering at the preserved 128-point width; arrow separation; truncation and containment; left-aligned opened choices; standard ComboBox response/ID, keyboard/open behavior, role routing, cursor, width/overflow/edge preservation; installed egui API alignment; deterministic checks; and the declared durable-document impact. WO-004 is excluded.

The comparison uses the `in-review` work order, follow-up plan/design review, completed [`validation-WO-001.md`](validation-WO-001.md:1), completed [`validation-WO-002.md`](validation-WO-002.md:1), [`execution-log.md`](../execution-log.md:76), current working-tree diff, [`main.rs`](../../../../apps/scriptcraft/src/main.rs:714), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:72), and installed egui [`ComboBox::show_ui()`](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/containers/combo_box.rs:187) / [`combo_box_dyn()`](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/containers/combo_box.rs:296) source. Meaningful comparison is possible.

## Exact checks and results

| Check | Result |
|---|---|
| Work-order state | PASS — [`WO-003-elements-closed-selected-text-centering.md`](../work-orders/WO-003-elements-closed-selected-text-centering.md:5) is `in-review`. |
| Fresh-session evidence | PASS — implementer and validator have distinct session IDs; WO-004 was explicitly excluded. |
| `git status --short --branch` | PASS — exit 0; expected initiative branch, modified [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1), modified [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:1), and untracked initiative evidence. |
| `git diff --stat` / source and blueprint diff | PASS — exit 0; tracked diff is limited to the app shell, its declared desktop blueprint contract, and initiative evidence. The source diff includes prior WO-001/WO-002 baseline changes plus the bounded WO-003 presentation/tests. |
| `git diff --check` | PASS — exit 0; no whitespace errors. |
| `cargo fmt --all -- --check` | PASS — exit 0. |
| `cargo test -p scriptcraft elements -- --nocapture` | PASS — 11 focused Elements tests passed, 0 failed. |
| `cargo check --workspace` | PASS — exit 0. |
| `cargo test --workspace` | PASS — 75 app tests, 45 core tests, and integration suites passed; 0 failures. |
| `cargo build --workspace` | PASS — exit 0. |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS — exit 0. |
| Installed egui API/source inspection | PASS — egui 0.29.1 exposes selected text, width, wrap mode, icon, and stock popup APIs, but no independent closed selected-text alignment API; stock `show_ui` returns `inner: None` when closed and owns the button path. |
| Native macOS visual smoke | WARNING — unavailable; optional under project policy and not a blocker. |

## Validation checklist

- **Evidence/preconditions: PASS.** The work order is `in-review`; changed files, implementation attempts/results, native-smoke limitation, session identity, and comparable diff evidence are recorded.
- **Closed centering and preserved width: PASS.** [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:714) retains `width(width)` and the stock button, while [`elements_closed_text_rect()`](../../../../apps/scriptcraft/src/main.rs:307) and [`elements_closed_text_position()`](../../../../apps/scriptcraft/src/main.rs:321) center the selected galley in the content rectangle after button padding and before the arrow/icon reservation. [`TOOLBAR_ELEMENTS_WIDTH`](../../../../apps/scriptcraft/src/main.rs:27) remains `128.0`. The focused geometry test proves the intended 104-point content region and centered placement.
- **Arrow placement and collision: PASS.** The standard stock icon remains enabled because no custom icon or button replacement was introduced. The geometry test confirms the text rectangle ends before the arrow rectangle with the icon-spacing separation.
- **Long-label truncation/containment: PASS.** The custom galley uses [`TextWrapMode::Truncate`](../../../../apps/scriptcraft/src/main.rs:749) and the bounded text rectangle; the focused test proves the galley and centered position remain inside that rectangle and therefore inside the control.
- **Popup alignment and ownership: PASS.** Painting is gated by `!popup_open` at [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:737), so opened popup entries continue through stock `selectable_value` calls at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:732-735). Installed egui popup layout remains stock and left-aligned; no popup centering or custom popup ownership was added.
- **Response/ID, keyboard/open behavior, role routing, and rejection preservation: PASS.** The stock [`ComboBox::from_id_salt()`](../../../../apps/scriptcraft/src/main.rs:725) and `show_ui` remain in place, with `next_role` updated by the existing seven `selectable_value` entries and changes routed through [`change_role_from_toolbar()`](../../../../apps/scriptcraft/src/main.rs:709). The focused suite covers role changes, rejected changes, overflow role routing, all seven labels, and the existing keyboard/open path remains stock by source/API comparison.
- **Standard-arrow cursor: PASS.** The ComboBox response and outer group retain [`toolbar_cursor_icon()`](../../../../apps/scriptcraft/src/main.rs:738), whose existing contract is the standard egui arrow/default cursor; no text cursor or competing interactive label response was introduced.
- **Width, overflow, edge contracts, and prior titlebar behavior: PASS.** The implementation preserves the explicit 128-point typed override, child width constraint, existing ComboBox/group boundary ledger, toolbar allocator/overflow path, and no changes to the WO-001/WO-002 titlebar/viewport behavior. Full workspace checks and prior validations confirm the preserved baseline; no WO-004 source behavior is present.
- **Scope and rogue behavior: PASS.** The diff is subordinate to the Elements closed-display change, its geometry/tests, the matching desktop blueprint wording, and Forge evidence. No deceptive, backdoor, malicious, nefarious, or unrelated behavior was observed.
- **Durable-document impact: PASS.** Requirements are correctly `verify unchanged` against role display/routing and typed width criteria. Blueprint impact is correctly `update`: [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:78-82) records closed-only centering, truncation/arrow/128-point preservation, stock ownership, and left-aligned popup routing. The blueprint diff also contains already-recorded WO-001/WO-002 reconciliations; those are not attributed to WO-003.
- **Required policy checks: PASS.** All configured automated checks passed. Native visual smoke is not required by project policy and is recorded as a warning rather than a hard stop.

## Findings

### Blockers

- None. No plainly unmet authorized criterion, unrelated or rogue implementation, required-policy failure, or comparison blocker was found.

### Warnings

- **WO003-VAL-W-001 — Native macOS visual smoke unavailable.** **Severity:** warning. **Evidence:** [`execution-log.md`](../execution-log.md:87) records no window-capable run, native closed/open alignment inspection, cursor observation, or mouse/keyboard smoke. **Recommendation:** Preserve the limitation and do not claim native visual or input observation until a window-capable run exists. **Required next state:** `complete`.

### Notes

- **WO003-VAL-N-001 — Working-tree implementation is uncommitted.** **Severity:** note. **Evidence:** current status shows the expected modified source/blueprint and untracked initiative evidence. **Recommendation:** Commit through normal Forge closeout. **Required next state:** `complete`.
- **WO003-VAL-N-002 — Diagnostic source review.** **Severity:** note. Candidate sources considered were unavailable stock alignment API, accidental popup centering, arrow collision, galley overflow, changed ComboBox identity/response, keyboard/open ownership loss, altered role routing/rejection semantics, cursor ownership drift, width/overflow edge regression, prior titlebar regression, inaccurate blueprint impact, and unrelated WO-004 scope. Installed-source inspection, focused/full checks, deterministic geometry assertions, and diff comparison reduce these to no hard-stop finding; only optional native smoke remains unavailable. **Recommendation:** No implementation change. **Required next state:** `complete`.

## Changed-file and diff comparison

The current working-tree diff reports two tracked implementation/document files plus the untracked initiative evidence. [`main.rs`](../../../../apps/scriptcraft/src/main.rs:714) contains the WO-003 closed-display helper, stock ComboBox integration, and focused tests; [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:78) contains the matching durable contract. The source diff also includes the already validated WO-001/WO-002 shell changes and their tests; those remain preserved and are not revalidated as new WO-003 scope. No requirements file, core crate, [`WO-004-cross-surface-native-validation-and-document-reconciliation.md`](../work-orders/WO-004-cross-surface-native-validation-and-document-reconciliation.md:1), or other runtime surface was changed.

## Decision

- **Outcome:** `pass`
- **Severity:** no blocker; one optional native-smoke warning and two notes
- **Recommendation:** Set WO-003 to `complete`. Do not create a letter-suffixed fix work order; the implementation aligns with the authorized closed-only centering contract and all required automated checks pass. Preserve the native-smoke warning for later native-capable validation.
- **Required next state:** `complete`
- **Fix work order required:** `no`
- **Implementation/design claim boundary:** This artifact validates WO-003 only. It does not validate or claim completion of WO-004.

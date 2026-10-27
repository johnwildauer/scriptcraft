# Validation: V2-WO-002 — Elements Cursor Ownership

## Validation metadata

- **Review type:** `validation`
- **Status:** `in-review`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `validator-fresh-context-v2-wo002-elements-cursor-2026-10-27T1915Z`
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Initiative:** [`titlebar-transition-and-elements-alignment-v2.md`](../titlebar-transition-and-elements-alignment-v2.md:1)
- **Validation file:** `validation-V2-WO-002.md`
- **Validated work order:** [`V2-WO-002-elements-cursor-ownership.md`](../work-orders/V2-WO-002-elements-cursor-ownership.md:1)
- **Compared commit/diff:** Working-tree comparison on `dev/2026-10-27-titlebar-transition-and-elements-alignment-v2` against the recorded `master` base `e77d6c4ee424403c5ab779d372745a51e4cafd4e`; tracked changes are [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1) and [`execution-log.md`](../execution-log.md:1), with v2 Forge artifacts untracked as recorded by `git status`.
- **Fix chain:** `none`

## Outcome summary

V2-WO-002 passes validation. The decorative Elements caption is now allocated and painted without a widget response, removing the competing label path that could request an I-beam. The stock egui ComboBox remains the owner of the closed response/ID, standard arrow, popup, keyboard/open behavior, and role rows; the closed selected galley remains painted-only and centered in the arrow-excluded region. Deterministic evidence covers standard-arrow ownership for all closed Elements regions and the popup, document-only text ownership, the preserved 128-point width, overflow/edge contracts, role routing, and rejection preservation.

Native pointer/display observation was unavailable. Per project policy this is a warning only: no native cursor, popup, keyboard, or overlap result is claimed.

## Authorized scope and comparison basis

This validation assesses [`V2-WO-002-elements-cursor-ownership.md`](../work-orders/V2-WO-002-elements-cursor-ownership.md:16) only: removal of the decorative-label competing cursor path; standard-arrow ownership for the closed Elements group, ComboBox, selected galley, label, and gap; stock ComboBox response/ID/arrow/popup/keyboard/open behavior; centered closed selected text; left-aligned popup choices; role routing and rejection preservation; document-only I-beam ownership; the fixed `128.0`-point width; overflow/edge preservation; installed egui API alignment; required checks; and durable-document classification. V2-WO-001 behavior is checked only as a preserved dependency. V2-WO-003 and V2-WO-004 are excluded.

The comparison uses the `in-review` work order, implementation evidence, the v2 plan/design review, the v2 execution log, completed [`validation-V2-WO-001.md`](validation-V2-WO-001.md:1), current branch status/diff, requirements/blueprints, [`main.rs`](../../../../apps/scriptcraft/src/main.rs:891), and installed egui 0.29.1 ComboBox source. Meaningful comparison is possible.

## Validation checklist

- **Evidence/preconditions: PASS.** The work order is `in-review`; the implementer session, changed-file boundary, validation attempts/results, native limitation, and comparable branch diff are recorded. The status is confirmed at [`V2-WO-002-elements-cursor-ownership.md`](../work-orders/V2-WO-002-elements-cursor-ownership.md:5).
- **Authorized behavior: PASS.** The source correction is limited to the decorative caption response path and supporting response/cursor ledger tests; stock ComboBox and document ownership remain in place.
- **Rogue behavior: PASS.** No deceptive, backdoor, malicious, nefarious, or unrelated behavior was observed.
- **Required checks: PASS.** All configured Cargo, formatting, lint, build, and whitespace checks passed independently. Native pointer/display smoke is not a configured required gate.
- **Durable documents: PASS.** Requirements and blueprint are correctly classified `verify unchanged`; no requirements or blueprint file changed and the existing user-facing/desktop contract remains accurate.

## Exact checks and results

| Check | Result |
|---|---|
| Work-order state | PASS; `V2-WO-002-elements-cursor-ownership.md` is `in-review`. |
| `git status --short --branch` | Exit 0; v2 initiative branch confirmed; [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1) and [`execution-log.md`](../execution-log.md:1) modified; v2 planning/validation artifacts untracked. |
| `git diff --stat` | Exit 0; tracked working-tree diff is 346 added and 11 removed source lines, plus 35 execution-log lines; the source diff contains prior V2-WO-001 changes and the bounded V2-WO-002 additions. |
| `git diff --check` | Exit 0; no whitespace errors. |
| `cargo fmt --all -- --check` | Exit 0. |
| `cargo test -p scriptcraft elements -- --nocapture` | Exit 0; 13 focused Elements tests passed. |
| `cargo test -p scriptcraft` | Exit 0; 79 app tests passed. |
| `cargo check --workspace` | Exit 0. |
| `cargo test --workspace` | Exit 0; 79 app tests, 45 core tests, integration suites, and doc-tests passed. |
| `cargo build --workspace` | Exit 0. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0. |
| Installed egui API inspection | PASS; egui 0.29.1 `ComboBox::show_ui` returns `inner: None` while closed, derives a persistent button ID, uses a stock click response and popup, paints the default arrow, and exposes no independent closed selected-text alignment API. |
| Native macOS pointer/display smoke | WARNING; unavailable. No native cursor, popup, keyboard, or overlap result is claimed; not a required project-policy gate. |

## Exact criteria and results

- **Decorative competing path: PASS.** [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:891) uses `allocate_space` and painter text for the `Elements` caption rather than `ui.label`; the caption therefore has no widget response. [`toolbar_combo_boundary_ledger()`](../../../../apps/scriptcraft/src/main.rs:396) and `decorative_elements_label_has_no_interaction_response()` verify the allocation is inside the group without asserting a label response.
- **Closed standard-arrow ownership: PASS.** The stock ComboBox response receives [`toolbar_cursor_icon()`](../../../../apps/scriptcraft/src/main.rs:1940), and the outer group response requests the same cursor at [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:953). The ledger covers the outer group, stock ComboBox, closed selected galley, decorative label, and group gap; every closed/control region resolves to `CursorIcon::Default`.
- **Popup and document ownership: PASS, with native observation unavailable.** The ledger includes a popup row and resolves it to the standard arrow; the only document response is [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:1407), which applies the text cursor to the editor response. Painted page rectangles and the closed galley are not responses. Deterministic geometry asserts Elements regions are not document-owned; native arbitration remains unobserved.
- **Stock ComboBox response/ID/arrow/popup/keyboard/open behavior: PASS.** [`ComboBox::from_id_salt()`](../../../../apps/scriptcraft/src/main.rs:902), `.show_ui()`, `combo_result.response`, stock arrow, and `combo_result.inner` open-state route remain intact. Installed egui source confirms `show_ui` retains the persistent button ID, stock `Sense::click()` response, default icon path, popup toggle, and popup contents path at [`combo_box.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/containers/combo_box.rs:187).
- **Centered closed selected text and popup alignment: PASS.** When closed, only the painted galley is added; [`elements_closed_text_rect()`](../../../../apps/scriptcraft/src/main.rs:482) excludes padding, icon spacing, and arrow width, and [`elements_closed_text_position()`](../../../../apps/scriptcraft/src/main.rs:496) centers it. When open, painting is skipped and stock `selectable_value` rows remain the popup path. Focused tests pass for centering, truncation/containment, and left-aligned popup choices.
- **Role routing and rejection preservation: PASS.** All seven supported roles remain in the stock `selectable_value` loop at [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:909); changed roles route through [`change_role_from_toolbar()`](../../../../apps/scriptcraft/src/main.rs:886). Focused tests pass for all labels, role mutation, overflow role routing, and rejected changes preserving session state.
- **Width, overflow, edge, and V2-WO-001 preservation: PASS.** `TOOLBAR_ELEMENTS_WIDTH` remains `128.0`; typed allocation/rendering, narrow overflow, clipped right-edge containment, and prior titlebar/viewport behavior remain unchanged by this work order. The completed V2-WO-001 validation remains the dependency evidence and is not revalidated as new scope.
- **Response/cursor ledger completeness: PASS.** [`elements_cursor_ledger()`](../../../../apps/scriptcraft/src/main.rs:450) explicitly records group, ComboBox, closed galley, label, popup, gap, and document regions, with arrow/text cursor intents and non-overlap assertions.

## Diagnostic source review

The independent diagnosis considered seven plausible sources: the decorative `ui.label` child response; stock ComboBox cursor precedence; parent-group ordering; a response accidentally added to the painted closed galley; popup-row ownership leaking into the closed control; document-response overlap; and a width/clip geometry error causing region misclassification. The non-sensing allocation, stock API/source inspection, explicit ledger, focused tests, full checks, and diff comparison reduce the likely implementation cause to the decorative label path, with stock/geometry overlap as the remaining native observation concern. No hard-stop defect was found.

## Findings

### Blockers

- None. No plainly unmet authorized criterion, unrelated/rogue implementation, required-policy failure, or comparison blocker was found.

### Warnings

- **V2-WO-002-VAL-W-001 — Native pointer/display observation unavailable.** **Severity:** warning. **Evidence:** the implementation log records no usable display or pointer automation; the validation run consequently has no native cursor shape, popup, keyboard, or overlap observation. **Recommendation:** preserve the deterministic ledger and do not claim native cursor success until a window-capable macOS run is available. **Required next state:** `complete`.

### Notes

- **V2-WO-002-VAL-N-001 — Working-tree implementation is uncommitted.** **Severity:** note. **Evidence:** `git status` shows the expected modified source/log and untracked v2 Forge artifacts. **Recommendation:** commit through normal Forge closeout. **Required next state:** `complete`.

## Changed-file and diff comparison

The recorded V2-WO-002 implementation changed [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:891), appended evidence to [`execution-log.md`](../execution-log.md:167), and did not change requirements, blueprints, core code, V2-WO-003/004 implementation, or the validation artifact before this validation. The source changes are bounded to the decorative caption ownership correction, response/cursor ledger helpers, and deterministic tests. Existing V2-WO-001 source changes are present in the working tree but are separately validated and not attributed to V2-WO-002. The work order's requirements/blueprint `verify unchanged` classification therefore matches the actual durable-document diff.

## Decision

- **Outcome:** `pass`
- **Severity:** no blocker; one optional native-display warning and one working-tree note
- **Required next state:** `complete`
- **Fix work order required:** `no`
- **Recommendation:** Set V2-WO-002 to `complete`. Do not create a letter-suffixed fix work order. Preserve the native pointer/display limitation as a non-blocking warning and continue to the separately scoped V2-WO-003 work order after orchestration records completion.
- **Implementation/design claim boundary:** This artifact validates V2-WO-002 only. It does not validate or claim completion of V2-WO-003 or V2-WO-004.

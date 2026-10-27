# Validation — WO-001 Toolbar Fit and Vertical Padding

## Validation metadata

- **Validation type:** `implementation alignment and anti-rogue validation`
- **Status:** `PASS WITH WARNINGS`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `debug-fresh-context-toolbar-final-fit-WO-001-validation-2026-10-27T0434Z`
- **Date:** 2026-10-27
- **Work order:** [`WO-001-toolbar-fit-and-vertical-padding.md`](../work-orders/WO-001-toolbar-fit-and-vertical-padding.md:1)
- **Initiative:** [`toolbar-final-fit-v1.md`](../toolbar-final-fit-v1.md:1)
- **Fresh-context statement:** This validation was performed as a separate fresh-context validator session. I did not implement WO-001, did not rely on implementer conversational memory, did not validate or implement WO-002, and did not mark WO-001 complete.

## Scope and comparison basis

This validation covers only the authorized Export PDF full-fit correction, the shared allocator/render width budget, named top/bottom padding with top `2.0` and the recorded reduced bottom value, preservation of Flexible Space/overflow/action semantics, the authorized tests and Cargo/diff gates, and the requirements/blueprint durable-document impact. It does not assess toolbar color or any other WO-002 obligation.

Inspected:

- Validation rules in [`05-validation.md`](../../../process/05-validation.md:1), lifecycle and scope rules in [`00-overview.md`](../../../process/00-overview.md:1), and project gates in [`project-manifest.md`](../../../project-manifest.md:73).
- Initiative scope in [`toolbar-final-fit-v1.md`](../toolbar-final-fit-v1.md:24), design-review warnings in [`design-review.md`](../reviews/design-review.md:19), and the authorized work order in [`WO-001-toolbar-fit-and-vertical-padding.md`](../work-orders/WO-001-toolbar-fit-and-vertical-padding.md:1).
- Implementer evidence and state transitions in [`execution-log.md`](../execution-log.md:35) and the implementation-evidence section of [`WO-001-toolbar-fit-and-vertical-padding.md`](../work-orders/WO-001-toolbar-fit-and-vertical-padding.md:59).
- Comparable working-tree diff, including [`main.rs`](../../../../apps/scriptcraft/src/main.rs:20), the authorized requirement update in [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:114), and the authorized blueprint update in [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:71).

## Diagnostic hypotheses and evidence used

The initial possible sources of the reported clipping were: (1) allocator/render width mismatch, (2) an omitted overflow reservation, (3) gap or horizontal-padding double counting, (4) Flexible Space consuming width differently from the renderer, (5) child minimum/layout allocation exceeding the logical Item width, (6) vertical panel padding reducing the usable row, and (7) a semantic or action-routing regression hidden by the geometry change. The two most likely sources from the initiative investigation were allocator/render mismatch and unmodeled framework/child extent.

The repository evidence used to validate those assumptions was the shared `toolbar_content_width()`/`toolbar_row_width()` calculation and `ToolbarAllocation.row_width` in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:157), the render path consuming that allocation in [`ScriptCraftApp::toolbar()`](../../../../apps/scriptcraft/src/main.rs:529), and the focused geometry, Flexible Space, overflow, action, and padding tests in [`mod tests`](../../../../apps/scriptcraft/src/main.rs:1442). No production logging was added because this validation is expressly non-invasive and the existing deterministic tests provide the required diagnostic evidence.

## Findings by severity

### Blockers

- **None.** The changed files and diff are comparable, the implementation is within WO-001's authorized boundary, and no rogue, deceptive, backdoor, malicious, or unrelated behavior was observed. No required project-policy gate failed.

### Warnings

#### W-001 — Native visual Export PDF inspection was unavailable

- **Severity:** warning
- **Evidence:** The implementer recorded that no fresh native/window smoke was performed in [`WO-001-toolbar-fit-and-vertical-padding.md`](../work-orders/WO-001-toolbar-fit-and-vertical-padding.md:66). This validation session had no native-window interaction channel. The automated fit tests prove the allocation contract, but do not provide pixel-level observation of the rendered Export PDF button in a real maximized window.
- **Recommendation:** Preserve this as an honest manual-evidence gap. Do not convert the Cargo/test result into a claim of native visual success.
- **Required next state:** Note/follow-up only. The manifest configures no manual or external hard-stop gate in [`project-manifest.md`](../../../project-manifest.md:88), so no fix work order is required from this warning.

### Notes

#### N-001 — Export PDF fit and shared width budget align

- **Severity:** note
- **Evidence:** [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:184) uses the same horizontal padding, gap, normal-item width, and overflow reservation represented by [`toolbar_content_width()`](../../../../apps/scriptcraft/src/main.rs:169). The resulting [`ToolbarAllocation::row_width`](../../../../apps/scriptcraft/src/main.rs:157) is consumed by [`ScriptCraftApp::toolbar()`](../../../../apps/scriptcraft/src/main.rs:529) for the actual row allocation. The wide/maximized-equivalent test keeps `Export PDF` as the final visible 72-point Item at 600 points, and the exact-fit/total-bounds tests pass.
- **Recommendation:** None for this validation.
- **Required next state:** Accept the implementation as aligned for the authorized full-fit geometry boundary.

#### N-002 — Flexible Space, overflow, and action semantics are preserved

- **Severity:** note
- **Evidence:** The typed default order and Item kinds remain covered in [`default_toolbar_items()`](../../../../apps/scriptcraft/src/main.rs:144) and the default-order test. The two Flexible Space Items continue to split the remaining width equally in [`toolbar_allocation_two_flexible_spaces_split_remaining_width_equally()`](../../../../apps/scriptcraft/src/main.rs:1603). Narrow allocation retains leftmost fitting Items and routes non-spacing Items to the far-right `>>` menu in [`toolbar_allocation_narrow_keeps_leftmost_items_and_far_right_menu()`](../../../../apps/scriptcraft/src/main.rs:1544). Existing action and Elements role routing remains exercised by [`overflow_entries_route_actions_and_elements_submenu_choices()`](../../../../apps/scriptcraft/src/main.rs:1645); no action adapter, persistence, core, export, or page-layout implementation file was changed.
- **Recommendation:** None for this validation.
- **Required next state:** Accept the implementation scope as aligned; leave WO-002 unvalidated and pending.

#### N-003 — Named vertical padding contract is explicit and durable

- **Severity:** note
- **Evidence:** [`TOOLBAR_TOP_PADDING`](../../../../apps/scriptcraft/src/main.rs:25) is `2.0`, and [`TOOLBAR_BOTTOM_PADDING`](../../../../apps/scriptcraft/src/main.rs:26) is explicitly recorded as the reduced `2.0` value. [`toolbar_vertical_padding_is_explicit_and_reduced()`](../../../../apps/scriptcraft/src/main.rs:1595) asserts both values. The requirement and desktop blueprint record the same final values in [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:114) and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:71).
- **Recommendation:** None. This resolves design-review warning DR-W-001 with an auditable final value.
- **Required next state:** Retain the requirements/blueprint `update` classification and its evidence.

#### N-004 — Durable-document impact classification is accurate

- **Severity:** note
- **Evidence:** The implementation changes user-visible transient toolbar geometry and the authorized durable requirement/blueprint wording, matching the `Requirements impact: update` and `Blueprint impact: update` classification in [`WO-001-toolbar-fit-and-vertical-padding.md`](../work-orders/WO-001-toolbar-fit-and-vertical-padding.md:52). No native project persistence schema, screenplay core semantics, or saved toolbar layout state was introduced.
- **Recommendation:** None.
- **Required next state:** Treat the durable-document classification as verified for WO-001.

#### N-005 — Diff is bounded and non-rogue

- **Severity:** note
- **Evidence:** The comparable diff contains [`main.rs`](../../../../apps/scriptcraft/src/main.rs:20), the specifically authorized requirement and blueprint updates, and the initiative evidence artifacts listed in [`WO-001-toolbar-fit-and-vertical-padding.md`](../work-orders/WO-001-toolbar-fit-and-vertical-padding.md:59). No WO-002 color change or unrelated production subsystem change was observed. No deceptive, backdoor, malicious, nefarious, or otherwise rogue behavior was observed.
- **Recommendation:** None.
- **Required next state:** Accept the changed-file boundary as aligned with WO-001.

## Validator gate results

The validator independently reran the required checks listed in [`WO-001-toolbar-fit-and-vertical-padding.md`](../work-orders/WO-001-toolbar-fit-and-vertical-padding.md:47). The chained command exited 0; the terminal output was captured by the validation session and is summarized below.

| Check | Exact result | Outcome |
|---|---|---|
| `cargo fmt --all -- --check` | Exit 0; no formatter diagnostics | PASS |
| `cargo check --workspace` | Exit 0; dev profile finished | PASS |
| `cargo test --workspace` | Exit 0; app 50 passed, core 44 passed, workspace integration suites 2/5/4/6/2/10/5/3 passed, doc tests 0 passed; 0 failed | PASS |
| `cargo build --workspace` | Exit 0; dev profile finished | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0; no diagnostics | PASS |
| `git diff --check` | Exit 0; no whitespace errors | PASS |

Native visual smoke was not available; no native visual success is claimed.

## Verdict and required next state

- **Result:** `PASS WITH WARNINGS`.
- **Why:** The implementation is aligned with the authorized WO-001 geometry and padding contract; Export PDF is covered by wide/exact-fit bounds tests; allocator and renderer use a shared row-width budget; Flexible Space, overflow, and action semantics remain covered; the final top/bottom values are durable and tested; the diff is bounded; and all required Cargo/diff gates passed independently. The only warning is unavailable native visual inspection, which is not a configured hard-stop gate.
- **Required next state:** Keep WO-001 `in-review` because the user explicitly requested that this validation not mark it complete. Record this validation in the execution log and let the orchestrator apply the pass-with-warnings transition separately. Do not create a fix work order from the current findings.
- **Scope guard:** This result validates only WO-001. It does not validate, implement, or complete WO-002.

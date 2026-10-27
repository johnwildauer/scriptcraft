# Validation — WO-002 Toolbar Width and Gap Correction

## Validation metadata

- **Validation type:** `implementation alignment and anti-rogue validation`
- **Status:** `PASS WITH WARNINGS`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `debug-fresh-context-toolbar-polish-WO-002-validation-2026-10-27T03:19:58Z`
- **Date:** 2026-10-27
- **Work order:** [`WO-002-toolbar-width-and-gap-correction.md`](../work-orders/WO-002-toolbar-width-and-gap-correction.md:1)
- **Initiative:** [`toolbar-polish-v1.md`](../toolbar-polish-v1.md:1)
- **Execution evidence:** [`execution-log.md`](../execution-log.md:1)
- **Prior dependency evidence:** [`validation-WO-001.md`](validation-WO-001.md:1)
- **Fresh-context statement:** This validation was performed as a separate fresh-context validator session. I did not implement WO-002, did not rely on implementer conversational memory, did not modify production code, and did not mark WO-002 complete. WO-001 was treated only as completed dependency evidence; its unrelated presentation boundary was not revalidated here.

## Scope and comparison basis

This validation covers only WO-002's authorized width/gap boundary: the shared allocation/render geometry budget, recorded four-point reduced gap, row-width bounds, wide/maximized-equivalent rightmost fitting, equal Flexible Space behavior, exact-fit and narrow overflow regressions, authorized durable-document impact, and required Cargo gates. It does not validate WO-001's icon/label hierarchy, general toolbar quality, native visual success, unrelated editor behavior, or future work.

Inspected:

- Forge rules in [`05-validation.md`](../../../process/05-validation.md:1) and lifecycle/scope rules in [`00-overview.md`](../../../process/00-overview.md:1).
- Required commands and policy in [`project-manifest.md`](../../../project-manifest.md:73).
- Initiative boundary and validation requirements in [`toolbar-polish-v1.md`](../toolbar-polish-v1.md:47).
- Design authorization in [`design-review.md`](../reviews/design-review.md:15).
- Authorized WO-002 and implementer evidence in [`WO-002-toolbar-width-and-gap-correction.md`](../work-orders/WO-002-toolbar-width-and-gap-correction.md:16).
- Separate dependency result in [`validation-WO-001.md`](validation-WO-001.md:1).
- Comparable working-tree diff for [`main.rs`](../../../../apps/scriptcraft/src/main.rs:19), the authorized requirement wording in [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:113), and the authorized blueprint wording in [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:70).
- Dispatch, completion, changed-file, and prior validation evidence in [`execution-log.md`](../execution-log.md:76).

The working-tree diff was meaningful and bounded. Production changes were limited to the desktop toolbar implementation/tests in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:19), with the two explicitly authorized durable-document updates and initiative evidence artifacts. No [`crates/scriptcraft-core/`](../../../../crates/scriptcraft-core/:1), persistence, export, page-layout, or unrelated UI files were changed by the reported implementation.

## Diagnostic hypothesis coverage

The seven plausible failure sources considered were: (1) allocator and renderer using different padding/gap budgets; (2) egui's default eight-point spacing remaining active in the rendered row; (3) Flexible Space receiving raw remainder instead of the post-budget remainder; (4) the rightmost Export PDF allocation exceeding the available row at wide or maximized-equivalent widths; (5) exact-fit or narrow widths changing leftmost-fit and `>>` overflow behavior; (6) the durable-document edits exceeding the authorized geometry contract; and (7) required Cargo gates or native visual evidence being misreported.

Evidence reduced hypotheses 1–6 to aligned/not observed: [`toolbar_content_width()`](../../../../apps/scriptcraft/src/main.rs:169) is shared by allocation and tests; [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:515) sets egui item spacing to zero and inserts the same four-point gap and row padding; focused tests cover flexible equality, exact fit, narrow overflow, bounds, and Export PDF; and the diff contains only the authorized requirement/blueprint geometry wording. Hypothesis 7 is split: all automated gates passed, while native visual observation was unavailable and is not claimed.

## Findings by severity

### W-002 — Native wide/maximized and narrow visual smoke unavailable

- **Severity:** warning
- **Evidence:** The implementer recorded no visible-window interaction channel, and this validation session likewise had no native-window inspection capability. No live toolbar pixels were inspected. The deterministic tests provide allocation evidence, but they do not prove native visual rendering at wide/maximized or narrow widths.
- **Recommendation:** Preserve the limitation as honest manual-evidence follow-up. Do not describe native visual success or claim that the rightmost label was visually inspected.
- **Required next state:** Note/follow-up only. [`project-manifest.md`](../../../project-manifest.md:88) configures no manual/external hard-stop gate, so this warning does not create a fix work order or block alignment validation.

### N-002 — Shared allocation/render geometry budget is aligned

- **Severity:** note
- **Evidence:** [`toolbar_content_width()`](../../../../apps/scriptcraft/src/main.rs:169) accounts for item widths, two four-point row-padding values, one four-point gap per rendered child separation, and the optional 32-point overflow width. [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:180) uses the same four-point padding/gap constants before distributing Flexible Space, and [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:515) disables egui's implicit horizontal spacing, inserts the explicit four-point gaps, and applies the same row padding and overflow reservation. The recorded constants are 88-point normal width, 32-point overflow width, 4-point row padding, and 4-point gap.
- **Recommendation:** None for this validation.
- **Required next state:** Accept the shared geometry contract as aligned with WO-002 and AC-WRITING-005.5.

### N-003 — Four-point gap is the recorded half-gap implementation

- **Severity:** note
- **Evidence:** [`TOOLBAR_ITEM_GAP`](../../../../apps/scriptcraft/src/main.rs:23) is `4.0`; the focused [`toolbar_geometry_uses_half_default_gap_and_stays_within_bounds()`](../../../../apps/scriptcraft/src/main.rs:1551) assertion verifies `4.0 == 8.0 / 2.0`. [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:523) inserts that value between rendered children after setting egui's implicit spacing to zero.
- **Recommendation:** None for this validation.
- **Required next state:** Accept the recorded four-point gap as the auditable reduced-gap implementation.

### N-004 — Row bounds and wide/maximized-equivalent right-edge fit are covered

- **Severity:** note
- **Evidence:** [`toolbar_geometry_uses_half_default_gap_and_stays_within_bounds()`](../../../../apps/scriptcraft/src/main.rs:1551) asserts the computed 800-point row content is within 800 points and that the final visible item is `Export PDF`. [`toolbar_allocation_wide_maximized_equivalent_keeps_export_pdf_fully_allocated()`](../../../../apps/scriptcraft/src/main.rs:1562) asserts no overflow at 600 points, keeps `Export PDF` last, and asserts computed content is within 600 points. The wide/exact-fit test also asserts a complete allocation at the generated exact width in [`toolbar_allocation_wide_and_exact_fit_are_deterministic()`](../../../../apps/scriptcraft/src/main.rs:1490).
- **Recommendation:** None for the deterministic geometry evidence. Native visual inspection remains covered by W-002 only.
- **Required next state:** Accept deterministic right-edge and row-bound evidence; retain the native-smoke warning.

### N-005 — Flexible Space remains equal and normal widths remain deterministic

- **Severity:** note
- **Evidence:** [`toolbar_allocation_two_flexible_spaces_split_remaining_width_equally()`](../../../../apps/scriptcraft/src/main.rs:1535) asserts both Flexible Space slots receive `106.0` at a 500-point available width after the shared budget. [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:194) gives every normal item the 88-point [`TOOLBAR_NORMAL_ITEM_WIDTH`](../../../../apps/scriptcraft/src/main.rs:20) and divides only the remaining width among the flexible count. The 800-point regression records 118-point flexible slots without changing normal hit widths.
- **Recommendation:** None for this validation.
- **Required next state:** Accept equal Flexible Space behavior and preserved normal hit-width behavior.

### N-006 — Exact-fit and narrow overflow behavior remains bounded

- **Severity:** note
- **Evidence:** [`toolbar_allocation_wide_and_exact_fit_are_deterministic()`](../../../../apps/scriptcraft/src/main.rs:1490) constructs the exact width from the shared content-width helper and asserts no overflow and exact total width. [`toolbar_allocation_narrow_keeps_leftmost_items_and_far_right_menu()`](../../../../apps/scriptcraft/src/main.rs:1521) asserts a 300-point row retains `New` and `Open`, routes `Save`, `Save As`, `Elements`, and `Export PDF` to overflow, and reserves 32 points for the far-right menu. Spacing items are skipped from overflow in [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:220), preserving the authorized narrow behavior.
- **Recommendation:** None for this validation.
- **Required next state:** Accept exact-fit and narrow deterministic regression coverage.

### N-007 — Durable-document impact is authorized and reconciled

- **Severity:** note
- **Evidence:** The requirement change updates only AC-WRITING-005.5's shared padding/reduced-gap and rightmost-fit wording in [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:113). The blueprint change records the four-point padding/gap, 32-point overflow reservation, equal Flexible Space remainder, and right-edge guarantee in [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:70). These match WO-002's authorized `Requirements impact: update` and `Blueprint impact: update` classification in [`WO-002-toolbar-width-and-gap-correction.md`](../work-orders/WO-002-toolbar-width-and-gap-correction.md:60); no persistence or core contract was changed.
- **Recommendation:** Retain the documented durable-document classifications.
- **Required next state:** Treat the durable-document impact as reconciled; no documentation correction is required.

### N-008 — No rogue or unrelated behavior observed

- **Severity:** note
- **Evidence:** The comparable diff is limited to the desktop toolbar geometry/rendering/test region, the authorized requirement and blueprint wording, and initiative evidence. No deceptive, backdoor, malicious, nefarious, or clearly unrelated behavior was observed. WO-001's presentation behavior was treated as dependency context and not assessed as part of this result.
- **Recommendation:** None.
- **Required next state:** Accept the implementation scope as bounded to WO-002.

## Required Cargo gate results

The validator independently reran the exact required automated chain from [`WO-002-toolbar-width-and-gap-correction.md`](../work-orders/WO-002-toolbar-width-and-gap-correction.md:54):

`cargo fmt --all -- --check && cargo check --workspace && cargo test --workspace && cargo build --workspace && cargo clippy --workspace --all-targets -- -D warnings && git diff --check`

| Check | Exact result | Outcome |
|---|---|---|
| `cargo fmt --all -- --check` | Exit 0 | PASS |
| `cargo check --workspace` | Exit 0 | PASS |
| `cargo test --workspace` | Exit 0; app 46 passed, core 44 passed, integration/doc-test groups passed | PASS |
| `cargo build --workspace` | Exit 0 | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0; no diagnostics | PASS |
| `git diff --check` | Exit 0; no whitespace errors | PASS |
| Wide/maximized and narrow native smoke | No native-window interaction channel; no visual inspection performed | UNAVAILABLE / NOT CLAIMED |

## Result and required next state

- **Result:** `PASS WITH WARNINGS`.
- **Why:** The implementation aligns with the authorized width/gap correction. Allocation and rendering consume one explicit geometry budget; the recorded 4-point gap is half the prior 8-point egui default; deterministic tests cover row bounds, wide/maximized-equivalent rightmost fitting, exact fit, narrow leftmost overflow, and equal Flexible Space behavior; the authorized durable-document updates reconcile with the diff; and every required automated Cargo gate passed independently. The only warning is unavailable native visual observation, which is not a configured hard-stop policy gate.
- **Required next state:** Keep [`WO-002-toolbar-width-and-gap-correction.md`](../work-orders/WO-002-toolbar-width-and-gap-correction.md:1) `in-review` until the orchestrator records this report and applies the Forge outcome. The orchestrator may advance it using the pass-with-warnings result; no fix work order is indicated. Do not mark WO-002 complete in this validation session.
- **Scope guard:** This report validates only WO-002's width/gap correction and does not revalidate WO-001 or unrelated behavior.

# Validation — WO-004 Screenplay Spacing Contract

## Metadata

- **Validation result:** `PASS WITH WARNINGS`
- **Validator session/agent ID:** `debug-fresh-context-ui-quality-of-life-WO-004-validation-2026-10-25T21:23:12Z`
- **Initiative:** [`ui-quality-of-life-v1.md`](../ui-quality-of-life-v1.md:1)
- **Work order:** [`WO-004-screenplay-spacing-contract.md`](../work-orders/WO-004-screenplay-spacing-contract.md:1)
- **Work-order state at validation:** `in-review`; this validator did not mark WO-004 complete.
- **Fresh-context statement:** This was a separate fresh-context validation session. I did not implement WO-004, did not validate WO-001, WO-002, or WO-003, and did not treat the implementer's report as proof without comparing the approved artifacts, source, diff, durable-document evidence, and independently rerun checks.

## Scope and comparison basis

This validation covers only the WO-004 spacing contract: the approved `612 × 792` profile; top/bottom/left/right margins `72/72/90/72`; nominal `55`-line capacity; twelve-unit line metric; adjacent baseline, line-box, and caret geometry; role measures and indents; empty, wrapped, keep-with-next, and page-boundary behavior; PDF parity; viewport-zoom separation; recorded tests/checks; and durable-document classification. It does not reassess the other work orders or general editor behavior.

Inspected:

- Forge rules in [`05-validation.md`](../../../process/05-validation.md:1) and [`00-overview.md`](../../../process/00-overview.md:1).
- The approved design review in [`design-review.md`](../reviews/design-review.md:1), initiative plan in [`ui-quality-of-life-v1.md`](../ui-quality-of-life-v1.md:1), approved/in-review work order and implementer evidence in [`WO-004-screenplay-spacing-contract.md`](../work-orders/WO-004-screenplay-spacing-contract.md:1), and WO-004 execution entries in [`execution-log.md`](../execution-log.md:31).
- Layout implementation and regressions in [`layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs:9), PDF export and parity regression in [`exports.rs`](../../../../crates/scriptcraft-core/src/exports.rs:66), and screen transform/caret regressions in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:278).
- Durable component evidence in [`screenplay-core.md`](../../../../.forge/blueprints/components/screenplay-core.md:126).
- The comparable working-tree diff for the scoped implementation files. The checkout also contains prior initiative changes in the same files; those were treated as baseline context rather than silently attributed to WO-004.

## Diagnostic hypotheses and validation log

Before deciding the result, I considered these possible sources of a spacing mismatch:

1. An incorrect or changed page/profile constant.
2. Incorrect content-bound arithmetic or role indentation/measure calculations.
3. Baseline and line-box accumulation or page-reset drift.
4. Caret geometry using a different height/origin than layout lines.
5. Wrapped, empty, or keep-with-next placement changing the fixed metric.
6. The screen painter applying a second spacing/metric transform or letting viewport scale alter pagination.
7. PDF export applying a different x/y transform or consuming independent spacing values.

The most likely risks were (3) line/page geometry drift and (6)/(7) consumer divergence. The following checks were the diagnostic log used to test those assumptions:

- [`margins_bounds_and_role_boxes_are_fixed()`](../../../../crates/scriptcraft-core/src/layout.rs:475) observes page bounds `612 × 792`, content origin `(90,72)`, content size `450 × 648`, and role boxes within the right content edge.
- [`fixed_metric_line_boxes_and_carets_have_twelve_unit_spacing()`](../../../../crates/scriptcraft-core/src/layout.rs:493) observes line y positions `72` and `84`, twelve-unit adjacency, twelve-unit line-box heights, and caret height/origin equal to each line.
- [`empty_wrapped_and_keep_lines_preserve_geometry_at_page_boundaries()`](../../../../crates/scriptcraft-core/src/layout.rs:516) exercises an empty line, wrapped content, keep-with-next role, reduced test capacity, page resets, and adjacent-line spacing.
- [`nominal_fifty_five_line_page_resets_to_top_margin()`](../../../../crates/scriptcraft-core/src/layout.rs:555) observes 55 lines on page zero, the last baseline at `72 + 54 × 12`, and page-one reset to y `72`.
- [`viewport_scale_only_transforms_fixed_print_geometry()`](../../../../apps/scriptcraft/src/main.rs:1412) observes that different viewport scales transform fixed coordinates while page count, profile bounds, and source layout remain unchanged.
- [`pdf_line_transforms_preserve_layout_positions_and_spacing()`](../../../../crates/scriptcraft-core/src/exports.rs:206) observes PDF x coordinates copied from layout and PDF y coordinates computed as `paper_height - line.y - line.height`, with twelve-unit adjacent spacing.

## Exact automated results

Reran from the repository root `/Users/sehnsucht-mac/code/scriptcraft`:

| Check | Exact result |
|---|---|
| `cargo fmt --all -- --check` | Exit `0`; passed with no output. |
| `cargo check --workspace` | Exit `0`; finished successfully. |
| `cargo test --workspace` | Exit `0`; `19` app tests passed, `28` core unit tests passed, `5` WO-003 integration tests passed, `3` E2E tests passed, `0` failed; doc-tests `0` run. Total: `55` passed. WO-004-specific geometry/PDF/viewport tests passed. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit `0`; passed with no warnings. |
| `cargo build --workspace` | Exit `0`; finished successfully. |

The native smoke attempt used `cargo run -p scriptcraft`. It successfully built and launched `target/debug/scriptcraft`, but this validation environment did not provide feasible native interaction or a reliable rendered-page/PDF visual inspection path. The process was stopped after launch; no native typing, zoom, caret, page-edge, or visual PDF result is claimed.

## Findings

### Finding W-004-001 — Native rendered/PDF visual smoke unavailable

- **Severity:** warning
- **Evidence:** The implementer explicitly recorded no native interaction in [`WO-004-screenplay-spacing-contract.md`](../work-orders/WO-004-screenplay-spacing-contract.md:86). The validator attempted [`cargo run -p scriptcraft`](../../Cargo.toml:1), observed the binary launch, but could not interact with the native window or inspect a rendered PDF at a known scale.
- **Recommendation:** Retain this as an honest manual-evidence limitation. Do not reinterpret the headless geometry and parity tests as observed native visual success.
- **Required next state:** `in-review` until the orchestrator records the validation outcome; this warning does not require a fix work order under [`05-validation.md`](../../../process/05-validation.md:77), because native interaction is not configured as a hard required project-policy gate.

### Finding N-004-001 — No spacing defect or scope violation found

- **Severity:** note
- **Evidence:** The layout retains the approved constants and exposes a shared twelve-unit metric in [`layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs:9). The structural regressions cover the requested bounds and edge cases in [`layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs:475). The screen painter derives coordinates from each layout rectangle and applies only the presentation scale in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:294), while caret bars use the same layout rectangle in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:798). PDF generation consumes the same layout and applies only the PDF coordinate-system conversion in [`exports.rs`](../../../../crates/scriptcraft-core/src/exports.rs:128). All rerun checks passed, and no deceptive, backdoor, malicious, nefarious, or clearly unrelated WO-004 behavior was found.
- **Recommendation:** None for implementation. Preserve the fixed profile unless future measured evidence demonstrates it is incorrect.
- **Required next state:** No fix work order; the orchestrator may proceed with the normal completion transition after linking this validation.

### Finding N-004-002 — Durable-document classification aligns with the final scoped result

- **Severity:** note
- **Evidence:** The work order classifies requirements as `verify unchanged` and blueprint impact as `update` in [`WO-004-screenplay-spacing-contract.md`](../work-orders/WO-004-screenplay-spacing-contract.md:63). The component blueprint records the measured profile and explicitly states that no metric correction was required in [`screenplay-core.md`](../../../../.forge/blueprints/components/screenplay-core.md:126). The feature-level screen/PDF contract was not changed, consistent with the work-order completion record in [`WO-004-screenplay-spacing-contract.md`](../work-orders/WO-004-screenplay-spacing-contract.md:76).
- **Recommendation:** Keep the recorded blueprint evidence and do not add a requirements change for this verification-only result.
- **Required next state:** Classification accepted; no documentation fix work order.

## Verdict and orchestrator action

- **Verdict:** `PASS WITH WARNINGS`.
- **Implementation alignment:** The approved spacing contract is implemented and regression-covered: `612 × 792`, margins `72/72/90/72`, content bounds `(90,72,450,648)`, nominal 55 lines, twelve-unit adjacent line/caret geometry, role measures/indents, empty/wrapped/keep/page-boundary behavior, screen scale separation, and PDF parity all match the recorded contract.
- **Orchestrator action:** The orchestrator should complete WO-004 under the pass-with-warnings rule in [`05-validation.md`](../../../process/05-validation.md:79), link this validation from [`WO-004-screenplay-spacing-contract.md`](../work-orders/WO-004-screenplay-spacing-contract.md:11) and [`execution-log.md`](../execution-log.md:33), and retain the native-smoke warning. It should **not** create a fix work order.
- **Status guard:** This validator did not mark WO-004 complete and did not validate any other work order.

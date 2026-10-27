# Validation — WO-001 Toolbar Text-Aware Fit

## Validation metadata

- **Validation type:** `implementation alignment and anti-rogue validation`
- **Status:** `PASS WITH WARNINGS`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `debug-fresh-context-toolbar-overflow-hover-WO-001-validation-2026-10-27T0509Z`
- **Date:** 2026-10-27
- **Work order:** [`WO-001-toolbar-text-aware-fit.md`](../work-orders/WO-001-toolbar-text-aware-fit.md:1)
- **Initiative:** [`toolbar-overflow-hover-v1.md`](../toolbar-overflow-hover-v1.md:1)
- **Fresh-context statement:** This validation was performed as a separate fresh-context validator session. I did not implement WO-001, did not rely on implementer conversational memory, did not validate or implement WO-002, and did not mark WO-001 complete.

## Scope and comparison basis

This validation covers only WO-001's full rendered/text-aware Export PDF group sizing, shared allocation/rendering width, wide/maximized fit, total bounds, exact-fit and narrow overflow behavior, preserved toolbar semantics, authorized tests, durable-document impact, and required Cargo/diff gates. It does not assess WO-002's hover illumination or toolbar color source.

Inspected:

- Validation and lifecycle rules in [`05-validation.md`](../../../process/05-validation.md:1) and [`00-overview.md`](../../../process/00-overview.md:1).
- Project commands, gate policy, manual-gate policy, and durable-document rules in [`project-manifest.md`](../../../project-manifest.md:73).
- Initiative scope/order in [`toolbar-overflow-hover-v1.md`](../toolbar-overflow-hover-v1.md:24), fresh design review in [`design-review.md`](../reviews/design-review.md:1), and the authorized work order/evidence in [`WO-001-toolbar-text-aware-fit.md`](../work-orders/WO-001-toolbar-text-aware-fit.md:13).
- Implementer events and recorded native limitation in [`execution-log.md`](../execution-log.md:35).
- Comparable working-tree diff for [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1), the authorized requirements wording update in [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:111), and the authorized blueprint wording update in [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:68).

## Diagnostic hypotheses and validation logs

### Candidate sources considered

1. The width helper could still use the old fixed 72-point normal-item width.
2. Allocation could use one width while rendering uses another width.
3. The Export PDF label could be omitted from the final group extent or clipped by the row budget.
4. Flexible-space distribution or the shared row padding/gap budget could make total bounds exceed the available width.
5. Exact-fit comparison could reject a boundary-width toolbar, or narrow allocation could change leftmost-fit/overflow semantics.
6. The implementation could have changed action/role routing or unrelated hover/color behavior while correcting geometry.

### Narrowed likely sources and logs

The two most likely implementation causes were (1) fixed text-unaware group width and (2) mismatched allocation/rendering width. Repository inspection and the fresh gate run validate those assumptions:

- **Width log:** [`toolbar_item_width()`](../../../../apps/scriptcraft/src/main.rs:171) returns `88.0` for `Export PDF` (`10 × 8.0 + 8.0`), greater than the 72-point baseline; `Space` remains 72.0 and flexible space remains allocated separately.
- **Shared-width log:** [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:200) uses `toolbar_item_width()` for full-fit and narrow candidates, and [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:541) passes each allocation width to the rendering functions. The Export PDF group therefore receives the same 88-point width used by allocation.
- **Bounds log:** [`toolbar_allocation_total_width_stays_within_available_bounds()`](../../../../apps/scriptcraft/src/main.rs:1617) asserts total content width is within 300, 600, and 800 points and equals `row_width`; the 600-point wide/maximized-equivalent test asserts Export PDF is visible at 88 points.
- **Boundary log:** [`toolbar_allocation_wide_and_exact_fit_are_deterministic()`](../../../../apps/scriptcraft/src/main.rs:1557) computes the exact boundary from the shared helper and requires all items to fit at that boundary; [`toolbar_allocation_narrow_keeps_leftmost_items_and_far_right_menu()`](../../../../apps/scriptcraft/src/main.rs:1580) requires `New`, `Open`, and `Save` to remain visible and `Save As`, `Elements`, and `Export PDF` to remain in overflow.
- **Semantics log:** The diff leaves [`toolbar_action_button()`](../../../../apps/scriptcraft/src/main.rs:446), [`toolbar_overflow_menu()`](../../../../apps/scriptcraft/src/main.rs:503), action dispatch, Elements role selection, item ordering, and Flexible Space item kinds within their existing behavior. The added tests include metadata/order and overflow routing coverage.

## Findings

### N-001 — Full text-aware Export PDF sizing is aligned

- **Severity:** note
- **Evidence:** [`toolbar_item_width()`](../../../../apps/scriptcraft/src/main.rs:171) deterministically accounts for label text plus group padding and takes the larger of that extent and the 72-point normal baseline. The `Export PDF` regression asserts 88.0 points and asserts it exceeds the baseline. Allocation uses this helper for wide and narrow paths.
- **Recommendation:** None for WO-001.
- **Required next state:** Record the implementation as aligned for full group sizing.

### N-002 — Allocation and rendering share the group width and preserve bounds

- **Severity:** note
- **Evidence:** [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:200) and [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:541) use the same per-item width values. [`toolbar_content_width()`](../../../../apps/scriptcraft/src/main.rs:184) includes row padding, inter-item gaps, and overflow reservation; tests assert `row_width` equals that calculation and never exceeds available width. The 600-point wide/maximized-equivalent regression keeps Export PDF visible and fully allocated at 88.0 points.
- **Recommendation:** None for WO-001.
- **Required next state:** Accept wide/maximized fit and total-bound evidence as aligned.

### N-003 — Exact-fit, narrow overflow, ordering, and semantics are preserved

- **Severity:** note
- **Evidence:** Exact-fit allocation uses the same helper-derived widths and succeeds at the calculated boundary. Narrow allocation retains the expected leftmost fitting actions and reserves the far-right `>>` menu for the remaining non-spacing items. Existing typed item kinds, approved order/labels, action metadata, action dispatch, and Elements role behavior remain covered by source inspection and passing tests.
- **Recommendation:** None; do not expand this validation into WO-002.
- **Required next state:** Accept the WO-001 semantic boundary as aligned.

### N-004 — Durable-document impact classification is accurate

- **Severity:** note
- **Evidence:** The work order classifies both Requirements and Blueprint impact as `update`. The comparable diff updates only the toolbar allocation contract wording in [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:114) and transient deterministic allocation wording in [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:71), matching the user-facing behavior change. No core, persistence, export, page-layout, or schema behavior changed.
- **Recommendation:** Retain the two authorized durable-document updates and their rationale.
- **Required next state:** Treat the durable-document classification as verified; no further document update is required by this validation.

### W-001 — Native visual confirmation is unavailable

- **Severity:** warning
- **Evidence:** The execution log records that no visible desktop test surface is available. This fresh validation session also had no native interaction channel. Automated geometry and source evidence support the allocation claims, but no pixel-level inspection of the Export PDF label at a maximized or narrow native window was performed.
- **Recommendation:** Preserve this as an honest manual-evidence limitation. Do not claim native visual or pixel-level success. The project manifest configures no manual/external hard-stop gate, so this warning does not require a fix work order.
- **Required next state:** Note/follow-up only; keep WO-001 in `in-review` until the orchestrator records the validation outcome. Do not dispatch or assess WO-002 from this artifact.

### N-005 — No rogue or unrelated WO-001 behavior observed

- **Severity:** note
- **Evidence:** The comparable diff is confined to the application toolbar geometry/tests and the two explicitly authorized requirements/blueprint wording updates, with initiative evidence documenting the same file set. No hover/color implementation, new command, ordering change, core/persistence/export/page-layout change, deceptive behavior, backdoor, malicious behavior, or other rogue behavior was observed.
- **Recommendation:** None.
- **Required next state:** Accept the changed-file boundary as aligned with WO-001.

## Required check results

The validator reran the complete command sequence required by WO-001:

```text
cargo fmt --all -- --check && cargo check --workspace && cargo test --workspace && cargo build --workspace && cargo clippy --workspace --all-targets -- -D warnings && git diff --check
```

| Check | Fresh result | Outcome |
|---|---|---|
| `cargo fmt --all -- --check` | Exit 0; no formatting diagnostics | PASS |
| `cargo check --workspace` | Exit 0 | PASS |
| `cargo test --workspace` | Exit 0; app 53 passed, core 44 passed, integration groups 2/5/4/6/2/10/5/3 passed, doc tests 0; no failures | PASS |
| `cargo build --workspace` | Exit 0 | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0; no warnings/errors | PASS |
| `git diff --check` | Exit 0; no whitespace errors | PASS |

## Verdict and required next state

- **Verdict:** `PASS WITH WARNINGS`.
- **Why:** The implementation is aligned with WO-001's authorized text-aware group sizing, shared allocation/rendering width, wide/maximized fit, total bounds, exact-fit/narrow overflow behavior, preserved toolbar semantics, tests, durable-document impact, and required Cargo/diff gates. The only warning is unavailable native visual confirmation, which is not a configured hard-stop policy gate.
- **Required next state:** Record this validation result in the initiative workflow and retain WO-001 as `in-review` until the orchestrator applies the result. No fix work order is required by this validation. WO-002 remains outside this validation and must not be treated as validated or implemented here.
- **Scope guard:** This artifact does not mark WO-001 complete and does not validate or implement WO-002.

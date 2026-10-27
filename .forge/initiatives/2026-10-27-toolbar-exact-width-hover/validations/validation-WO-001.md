# Validation: WO-001 — Exact Runtime Label Width

## Validation metadata

- **Validation session/agent ID:** `forge-validator-toolbar-exact-width-hover-2026-10-27-0739Z`
- **Validation type:** Fresh-context implementation validation
- **Date:** 2026-10-27
- **Work order:** [`WO-001-exact-runtime-label-width.md`](../work-orders/WO-001-exact-runtime-label-width.md:1)
- **Initiative:** [`toolbar-exact-width-hover-v1.md`](../toolbar-exact-width-hover-v1.md:1)
- **Implementer session:** `code-implementer-toolbar-exact-width-hover-2026-10-27-0734Z`
- **Session exception:** None

## Comparison basis

Reviewed the validation process in [`05-validation.md`](../../../process/05-validation.md:1), project policy in [`project-manifest.md`](../../../project-manifest.md:1), initiative plan, design review, work order, execution log, changed-file record, source diff, and implementation test output. The comparable implementation surface is [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), with authorized durable-document changes in [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90) and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58).

The implementation report and repository evidence are sufficiently consistent for meaningful comparison. No [`WO-002-uniform-persistent-icon-hover.md`](../work-orders/WO-002-uniform-persistent-icon-hover.md:1) behavior was validated or authorized by this review.

## Diagnostic assessment

### Candidate sources considered

1. The four failures might be stale predecessor assertions for geometry intentionally changed by WO-002.
2. WO-001 might have changed hover or vertical geometry constants accidentally.
3. The active-style measurement seam might differ between production and deterministic tests.
4. The measurement could be correct locally but not be propagated through allocation.
5. Flexible Space could still use a guessed or predecessor fixed-width budget.
6. Overflow or right-edge accounting could omit the measured final label.
7. The requirements/blueprint update could misstate the implemented contract.

The evidence distills these to two likely sources: (a) the exact-width implementation and its allocation propagation, which are supported by passing focused assertions and source inspection; and (b) an unrelated geometry edit in the same production diff, which is confirmed by the four failures and the changed constants. The test output is the diagnostic log used to validate this distinction; no production logging was added because the requested validation does not modify implementation code.

## Findings

### PASS — Exact active-egui-style label measurement

**Severity:** Pass

**Evidence:** [`toolbar_label_width()`](../../../../apps/scriptcraft/src/main.rs:188) obtains the active `Body` `TextStyle` from [`Ui::style()`](../../../../apps/scriptcraft/src/main.rs:189) and measures a no-wrap galley through [`Ui::fonts()`](../../../../apps/scriptcraft/src/main.rs:195). [`toolbar_item_width()`](../../../../apps/scriptcraft/src/main.rs:173) adds the group padding and preserves the minimum control width without using the removed character-count constant. The deterministic helper [`test_toolbar_label_width()`](../../../../apps/scriptcraft/src/main.rs:1561) runs the same egui measurement path in a context and the text-aware test rejects the old `character_count × 8` result at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1940).

**Recommendation:** None.

**Required next state:** Retain this portion of the implementation.

### PASS — Allocation, Flexible Space, overflow, rendering, and edge containment

**Severity:** Pass

**Evidence:** [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:233) uses [`toolbar_item_width()`](../../../../apps/scriptcraft/src/main.rs:173) for fixed widths, computes Flexible Space from the remaining shared budget, and reuses measured widths while fitting narrow overflow. [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:592) passes the live UI measurement closure into allocation and renders each visible Item with the allocated width. The focused workspace run reported 56 passing tests, including exact-fit/wide allocation, narrow overflow, equal Flexible Space, rendered-width agreement, and clipped right-edge containment tests at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1717), [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1747), [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1882), and [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1805). The approved requirements and blueprint now describe this same active-style runtime contract at [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:114) and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:72).

**Recommendation:** None for WO-001 scope.

**Required next state:** Preserve the measured-width/allocation changes while resolving the hard-stop finding below.

### FAIL — Unauthorized geometry changes are an implementation defect, not stale tests

**Severity:** Blocker / hard stop

**Evidence:** The WO-001 scope explicitly excludes hover-region behavior and vertical geometry. Nevertheless, the production diff changes `TOOLBAR_TOP_PADDING` from `2.0` to `32.0`, `TOOLBAR_BOTTOM_PADDING` from `2.0` to `1.0`, and `TOOLBAR_ICON_HOVER_PADDING` from `2.0` to `0.0` at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:21). These are precisely the behaviors reserved for [`WO-002-uniform-persistent-icon-hover.md`](../work-orders/WO-002-uniform-persistent-icon-hover.md:17), not exact label measurement.

The fresh [`cargo test --workspace`](../../../../Cargo.toml:1) run failed with four tests: `toolbar_icon_hover_fill_uses_the_existing_hover_color_source_only` at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1629), `toolbar_icon_hover_region_is_centered_and_excludes_the_label_area` at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1615), `toolbar_manual_geometry_controls_are_narrow_and_explicit` at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1763), and `toolbar_vertical_padding_is_explicit_and_reduced` at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1876). The failures are direct value mismatches (`0.0` vs `2.0`, `32.0` vs `2.0`, and `1.0` vs `2.0`) caused by the changed constants. They are therefore not stale tests to waive: they correctly detect unauthorized WO-002/predecessor-geometry changes in the WO-001 implementation diff.

**Recommendation:** Do not change or delete these tests as part of WO-001 validation. Revert the unrelated hover and vertical geometry edits in a separately dispatched fix work order, or otherwise provide an explicitly approved scope decision before implementation. The fix must not implement WO-002.

**Required next state:** `WO-001` remains `in-review`; create a letter-suffixed fix work order for the unauthorized geometry changes, obtain diagnosis confirmation before fixing, then run a fresh implementation and fresh validation. Do not dispatch or validate WO-002 until this fix chain passes.

### PASS — Durable-document impact

**Severity:** Pass

**Evidence:** The work order classified both Requirements and Blueprint impact as `update`. The changed requirement adds active egui style/font runtime measurement to AC-WRITING-005.5 at [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:114), and the changed desktop blueprint records the galley-based measurement contract at [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:72). Those updates are directly subordinate to the approved user-facing allocation contract and are not contradicted by the implementation.

**Recommendation:** Keep the durable-document updates; do not broaden them to WO-002 interaction behavior.

**Required next state:** No documentation correction required for WO-001.

### PASS — Required Cargo and diff gates, with test-gate failure recorded

**Severity:** Conditional / warning except for the hard stop above

**Evidence:** Fresh validation commands produced: `cargo fmt --all -- --check` PASS; `cargo check --workspace` PASS; `cargo build --workspace` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; and `git diff --check` PASS. `cargo test --workspace` FAILED with 56 passed and 4 failed, with all four failures attributable to the unauthorized geometry constants described above. No native visual smoke was claimed; the project manifest records no required manual/external gate, and the execution evidence states that visible-window measurement is unavailable.

**Recommendation:** Treat the test failure as resolved only after the unauthorized edits are removed and the full workspace test gate is rerun successfully. Do not mark the work order complete based on the 56 passing tests.

**Required next state:** Fix chain and fresh validation required; no completion transition.

## Rogue-scope assessment

No deceptive, backdoor, malicious, or nefarious behavior was observed. The issue is a plainly unrelated production-scope change, not rogue behavior.

## Result

- **Outcome:** `fail`
- **Hard-stop finding:** Unauthorized hover/vertical geometry changes in the WO-001 production diff cause four legitimate existing toolbar geometry tests to fail.
- **WO-001 status:** Must remain `in-review`; it must not be marked `complete`.
- **Initiative next state:** `execute` for a scoped WO-001 fix, followed by `validate` from a fresh context.
- **WO-002:** Not validated, not implemented by this validation, and must remain pending.
- **User/owner diagnosis confirmation:** Required before any fix implementation; this validation makes no production-code or test changes.


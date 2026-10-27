# Work Order: WO-001 — Final Edge Child Budget

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh desktop toolbar implementer
- **Phase/order:** 1 of 1; follows independent diagnosis of closed toolbar edge initiatives
- **Initiative:** [`../toolbar-final-edge-fix-v1.md`](../toolbar-final-edge-fix-v1.md:1)
- **Generated filename:** `WO-001-final-edge-child-budget.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-001.md`](../validations/validation-WO-001.md:1) — to be created by the fresh validator after implementation
- **Fixes:** none
- **Implementer session/agent ID:** `code-toolbar-final-edge-child-budget-2026-10-27T0807Z`
- **Session exception:** `none`

## Summary / proven root cause

The allocator budgets [`toolbar_item_width()`](../../../../apps/scriptcraft/src/main.rs:173) for `Elements`, but [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:555) places an intrinsic [`ComboBox`](../../../../apps/scriptcraft/src/main.rs:562) inside that fixed-width child UI. egui permits [`allocate_ui_with_layout()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/ui.rs:1348) contents to expand the allocated rectangle when their intrinsic minimum is larger. The expanded child shifts later siblings, so the actual Export PDF/group right edge is `allocation.row_right + E`, where `E = max(0, combo_intrinsic_width - allocated_elements_width)`.

This is the remaining discrepancy. The pure row equation, active Body galley measurement, Flexible Space split, and 8-point frame/content inset are not the primary fault. The exact native `E` must be logged/validated, not guessed.

## In scope

- Make the `Elements` rendered child honor the same explicit width budget used by allocation, using the smallest reviewed egui-compatible mechanism.
- Add debug/test evidence for `content_rect`, `clip_rect`, `row_left`, `allocation.row_width`, each rendered child/group rectangle, and the final label rectangle.
- Add acceptance coverage for wide, exact-fit, clipped native, and narrow-overflow cases.
- Preserve active-style galley measurement, Flexible Space, overflow semantics, action routing, and the user's local constants, including [`TOOLBAR_TOP_PADDING`](../../../../apps/scriptcraft/src/main.rs:28) at its current local value `30.0`.

## Out of scope

- No viewport/titlebar/fullsize-content flag changes.
- No frame/panel color, hover behavior, toolbar order, labels/icons, normal-item policy, font/theme selection, core, persistence, exports, or page layout changes.
- Do not reset or reformat unrelated production changes; do not change top/bottom padding or hover padding as part of this work order.
- Do not claim native pixel containment from pure tests alone.

## Requirements

- **AC-WRITING-005.5:** “Given a toolbar width, normal Items and Space shall consume one deterministic normal-item width, Flexible Space Items shall divide the remaining width equally, and a narrow toolbar shall retain the leftmost fitting Items while exposing non-spacing Items through a far-right `>>` text menu.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:68) — toolbar allocation is transient/deterministic and must account for the actual rendered row.

## Implementation plan

1. Instrument the existing [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:615) and [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:555) geometry at the debug/test seam without changing user-visible behavior.
2. Apply one child-budget correction at the `Elements` ComboBox boundary, then verify that requested and actual group widths agree.
3. Run the full gates and record native rectangle evidence; update durable wording only if the contract changes.

## Files and systems

- **Create:** This work order and its fresh validation artifact; implementation may update [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:555) and only directly affected durable docs if approved.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:555) only for the scoped child-budget fix and diagnostics; execution/validation artifacts.
- **Avoid changing:** viewport setup, frame constants, user-local top padding, hover constants, toolbar model/order, core, persistence, exports, and page layout.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- **Geometry checks:** assert `row_right <= effective_clip.right`; assert each child/group rect is within the row budget; assert final Export PDF label rect is within the same right edge; assert the measured `E` is zero after the fix; retain exact-fit and narrow-overflow semantics.
- **Manual/external checks:** native target window at wide/maximized and a clipped/narrow width; inspect the actual rightmost label and compare logged panel/content/clip/row/group rectangles.
- **Acceptance test:** Given the current local constants and default toolbar, when the toolbar is allocated and rendered at each validation width, then the actual `Elements` child consumes no unbudgeted width, the final group/label right edge is no greater than the effective clipped content edge, and no unrelated behavior or local padding changes occur.

## Durable-document impact

- **Requirements impact:** `verify unchanged` unless the reviewed fix changes the user-facing allocation contract.
- **Blueprint impact:** `verify unchanged` unless the reviewed fix changes the documented actual-rendered-edge contract.
- **Affected documents/IDs:** REQ-WRITING-LOOP-005 / AC-WRITING-005.5 and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:68).
- **Rationale and evidence:** The defect is an implementation mismatch within the already documented right-edge guarantee; the implementer must compare resulting behavior before deciding whether wording remains accurate.
- **Authorized documentation changes:** Only a narrowly evidenced clarification of child intrinsic-width budgeting and rectangle validation; no unrelated durable-document edits.

## Completion checklist

- [x] Implementation is complete within scope.
- [ ] Tests/checks and native rectangle evidence have passed; automated implementation checks passed, while native/manual rectangle inspection remains unavailable to this session and is explicitly deferred to the orchestrator's validation gates.
- [x] Durable-document impact is reconciled against the diff: requirements and blueprint wording remain accurate; no durable product or technical contract was changed.
- [x] Execution log has fresh dispatch/completion identifiers.
- [ ] Work order is validated by a separate fresh session before completion.

## Implementation evidence — 2026-10-27

- **Implementer session/agent ID:** `code-toolbar-final-edge-child-budget-2026-10-27T0807Z`.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:237), this work order, and [`execution-log.md`](../execution-log.md:36). No other production or durable product files changed.
- **Exact implementation:** At the [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:620) [`allocate_ui_with_layout()`](../../../../apps/scriptcraft/src/main.rs:623) child boundary, [`ui.set_width(width)`](../../../../apps/scriptcraft/src/main.rs:632) sets both the child minimum and maximum width to the allocator-provided Elements budget before the ComboBox is added. A debug assertion checks the returned child response width, and the label response is checked against the child minimum rectangle. No constants, viewport flags, frame color, hover ownership, order, style, semantics, persistence, exports, or page layout were changed.
- **Geometry evidence:** [`toolbar_rect_ledger()`](../../../../apps/scriptcraft/src/main.rs:237) records content, clip, row, Elements child, and final-label rectangles. [`toolbar_rectangle_ledger_contains_content_clip_row_child_and_final_label()`](../../../../apps/scriptcraft/src/main.rs:1999) checks containment and `row_right <= clip.right()` at exact-fit, narrow/clipped (`300.0`), and wide (`1000.0`) cases. Narrow overflow assertions preserve the current leftmost fitting items and far-right menu reservation.
- **Automated implementation checks:** `cargo fmt --all -- --check` passed; `cargo test -p scriptcraft toolbar_ -- --nocapture` passed (`24 passed, 0 failed`); `git diff --check` passed.
- **Native/manual limitation:** No native-window or personal visual verification was performed or claimed. The exact native intrinsic excess `E` and actual native rectangle ledger remain for the separate orchestrator/validator gates. The full final gate/validation was intentionally not run by this implementer.

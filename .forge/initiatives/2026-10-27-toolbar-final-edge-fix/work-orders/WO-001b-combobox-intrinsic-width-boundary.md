# Work Order: WO-001b — Bind ComboBox Intrinsic Width at the Widget Boundary

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh final-edge implementation/validation session
- **Phase/order:** Follow-up to WO-001 and WO-001a; native user verification reported the defect remains
- **Initiative:** [`toolbar-final-edge-fix-v1.md`](../toolbar-final-edge-fix-v1.md:1)
- **Predecessors:** [`WO-001-final-edge-child-budget.md`](WO-001-final-edge-child-budget.md:1), [`WO-001a-remove-panicking-geometry-assertion.md`](WO-001a-remove-panicking-geometry-assertion.md:1)
- **Session:** `code-toolbar-final-edge-combobox-boundary-2026-10-27T0819Z`
- **Production changes authorized by this artifact:** `implemented within this work order`

## User checkpoint and diagnosis

The app now launches after WO-001a, but the user personally verified that the native Export PDF right edge is still clipped/incorrect. This artifact records the fresh read-only diagnosis and the next implementation boundary; it does not modify production code.

The remaining cause is at the nested [`ComboBox`](../../../../apps/scriptcraft/src/main.rs:633), not at the removed assertion or parent row sum. [`ui.set_width(width)`](../../../../apps/scriptcraft/src/main.rs:632) constrains the `Elements` child Ui's own min/max region, but it does not configure the ComboBox. In egui 0.29.1, the ComboBox default minimum is [`ui.spacing().combo_width`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/containers/combo_box.rs:333), and its widget width is `max(galley_width + icon_spacing + icon_width, combo_width - 2 * button_padding.x)` at [`combo_box_dyn()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/containers/combo_box.rs:349). The child width contract therefore does not equal the ComboBox response width when the default intrinsic minimum exceeds `width`.

## Exact geometry trace

For native screen width `V`, [`TopBottomPanel::top()`](../../../../apps/scriptcraft/src/main.rs:1576) receives approximately `[0, V]`. [`Frame::side_top_panel()`](../../../../apps/scriptcraft/src/main.rs:1568) has horizontal inner margin `8`, so content is approximately `C = [8, V-8]`; the top-level panel clip is approximately `K = [0, V]` ([`panel.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/containers/panel.rs:377), [`panel.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/containers/panel.rs:384)). [`toolbar_actual_available_width()`](../../../../apps/scriptcraft/src/main.rs:215) computes `A = max(min(C.right, K.right) - row_left, 0)`, hence `A = V - 16` when `row_left = 8`. [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:300) returns [`ToolbarAllocation::row_width`](../../../../apps/scriptcraft/src/main.rs:166) `R = toolbar_content_width(visible, overflow_width)`; the requested row is `[row_left, row_left + R]`.

The row is allocated by [`ui.allocate_ui_with_layout()`](../../../../apps/scriptcraft/src/main.rs:701). egui computes `frame_rect` and `child_rect` from the requested size ([`ui.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/ui.rs:1364)), but after the closure [`allocate_new_ui_dyn()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/ui.rs:1405) advances the parent using `child_ui.min_rect()` ([`ui.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/ui.rs:1407)). The requested row width is therefore not a hard parent allocation boundary.

Within [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:622), [`set_width()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/ui.rs:943) calls `set_min_width(width)` followed by `set_max_width(width)`. The maximum operation cannot shrink below the current minimum ([`set_max_width()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/ui.rs:873)); neither operation caps a child widget's requested allocation. The ComboBox's [`button_frame()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/containers/combo_box.rs:422) creates a nested child from `outer_rect.shrink2(button_padding)` ([`combo_box.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/containers/combo_box.rs:434), [`combo_box.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/containers/combo_box.rs:437)). The selected-text closure calls [`allocate_space()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/containers/combo_box.rs:352), then the button response is rebuilt from the nested content `min_rect` ([`combo_box.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/containers/combo_box.rs:441) and the ComboBox advances its cursor with [`advance_cursor_after_rect()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/containers/combo_box.rs:464).

If `B` is the actual ComboBox/button response width and `W` is the allocated Elements budget, then the default-width path gives `B = max(W, galley_width + icon_spacing + icon_width + 2 * button_padding.x, combo_width)` up to the source's explicit margin cancellation. The unbudgeted excess is `E = max(0, B - W)`. The final Export PDF group edge is consequently `row_left + R + E` (including subsequent spacing), even while pure allocation tests prove only `row_left + R <= K.right`. This explains why the existing ledger and the removed runtime assertion can pass while native Export PDF remains clipped.

## Alternative sources considered

1. Native titlebar/full-size content insets or a second frame subtraction.
2. Active font/galley mismatch between allocation and rendering.
3. Flexible-space split or gap arithmetic.
4. [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:300) returning an incorrect [`row_width`](../../../../apps/scriptcraft/src/main.rs:372).
5. [`ui.set_width(width)`](../../../../apps/scriptcraft/src/main.rs:632) failing to set the child min/max region.
6. ComboBox's own default minimum and response rectangle expanding the child.

Source review makes (1)–(5) less likely: the panel/frame equation and active galley measurement are internally consistent, flexible spaces are explicit, and egui's [`set_width()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/ui.rs:943) does set both bounds. (6) is the remaining likely source because widget allocation expands the child `min_rect`.

## Distinct next fix boundary

The next fix must be at the [`ComboBox`](../../../../apps/scriptcraft/src/main.rs:633) widget boundary, not at the removed assertion or parent rectangle ledger. The implementer must make the ComboBox selected-value/button response obey the allocated `Elements` budget, using an egui-supported width/wrap or equivalent fixed-control mechanism, and capture the actual ComboBox response rect, nested child `min_rect`, Elements group response, parent row cursor/min rect, and final Export PDF label rect. Repeating child [`set_width()`](../../../../apps/scriptcraft/src/main.rs:632) is not an acceptable fix.

The implementation must preserve `TOOLBAR_NORMAL_ITEM_WIDTH = 46.0`, `TOOLBAR_TOP_PADDING = 30.0`, `TOOLBAR_BOTTOM_PADDING = 2.0`, `TOOLBAR_ICON_HOVER_PADDING = 2.0`, and all toolbar semantics.

## Acceptance criteria

- No production code is changed by this diagnosis artifact.
- The implementation proves `B <= W` (or an explicitly documented equivalent fixed-control contract), `Elements_group.right <= row_left + allocation.row_width`, and `Export_PDF_label.right <= effective_clip.right` using actual response rectangles, not only allocator arithmetic.
- Evidence covers wide, exact-fit, clipped/native, and narrow-overflow cases, including parent cursor/min-rect advancement after Elements.
- Full project gates pass and user-local constants are unchanged.
- **Required user verification checkpoint:** after implementation and automated gates, the user personally launches the native app and verifies the complete Export PDF icon and label at wide/maximized and clipped/narrow widths. The work order remains `in-review` until that report is recorded; no native containment claim may be made from source tests alone.

## Implementation evidence — 2026-10-27

- **Implementer session/agent ID:** `code-toolbar-final-edge-combobox-boundary-2026-10-27T0819Z`.
- **State:** `in-review`; implementation is complete and the required user-native verification remains outstanding.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:4), this work order, and [`execution-log.md`](../execution-log.md:117). No requirements, blueprint, core, persistence, export, or page-layout files changed.
- **Boundary fix:** [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:665) now supplies the allocated width to [`ComboBox::width()`](../../../../apps/scriptcraft/src/main.rs:676) and uses [`TextWrapMode::Truncate`](../../../../apps/scriptcraft/src/main.rs:677), while retaining the child [`set_width()`](../../../../apps/scriptcraft/src/main.rs:672). This constrains the selected-value/button response instead of relying on the child Ui bounds alone. Existing user-local constants and toolbar behavior remain unchanged.
- **Rectangle evidence:** [`toolbar_combo_boundary_ledger()`](../../../../apps/scriptcraft/src/main.rs:247) captures the actual ComboBox response, child `min_rect`, and Elements group response through egui. [`toolbar_combo_boundary_ledger_contains_response_child_group_and_final_edge_cases()`](../../../../apps/scriptcraft/src/main.rs:2088) covers wide (`1000.0`), exact-fit, and narrow (`46.0`) budgets and proves response, child, and group widths do not exceed the Elements budget. The existing [`toolbar_rect_ledger()`](../../../../apps/scriptcraft/src/main.rs:278) and its ledger test cover content, clip, row, child, and final Export PDF label rectangles, including clipped/narrow overflow semantics.
- **Safety:** No runtime geometry assertion was added; the prior panicking child-response assertion remains removed. The production boundary uses egui-supported width and truncation behavior.
- **Implementation gate results:** `cargo fmt --all -- --check` PASS; `cargo check --workspace` PASS; `cargo test --workspace` PASS (63 app tests, 44 core tests, integration/doc targets passed); `cargo build --workspace` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `git diff --check` PASS. A focused ComboBox ledger test also passed (`1 passed, 0 failed`).
- **Native/manual limitation:** This session did not launch or inspect the native window and does not claim native containment or user verification. The required user checkpoint is still wide/maximized and clipped/narrow inspection of the complete Export PDF icon and label. No separate final validation was performed.

## Durable-document impact

- **Requirements:** `verify unchanged`.
- **Blueprint:** `verify unchanged` unless the reviewed widget-boundary contract changes documented behavior.
- **Rationale:** This is a scoped correction to the actual rendered control response, not a toolbar allocation-policy change.

# Toolbar-to-Script View residual gap — source findings and proposed work orders

- **Status:** WO-012 source correction submitted; user runtime and visual review pending
- **Date:** 2026-11-06
- **Starting record:** [`toolbar-script-view-gap-continuation.md`](../2026-11-05-gui-spacing-and-scroll/toolbar-script-view-gap-continuation.md)
- **Investigation branch named by the prior record:** `dev/2026-11-06-script-view-boundary`
- **Research method:** Read-only application and installed-framework source inspection
- **Validation:** At the user's later explicit request, a coding agent ran only [`cargo check --workspace`](../../.forge/project-manifest.md:197); it succeeded with four warnings and no deprecation warning. No tests, Clippy, formatting, or manual runtime verification was run.

## 1. Executive finding

The residual gap has a specific source-level owner inside the Elements toolbar Item, not in Script View.

The closed Elements composition disables vertical Item spacing only **after** its ComboBox has already returned. The installed egui API inserts Item spacing after adding a widget, and changing the setting afterward cannot remove the cursor advance that already occurred. The next explicit caption-gap allocation therefore begins one normal vertical-spacing unit below the ComboBox. The final caption allocation expands the Elements group's minimum height by that same amount beyond its declared Item-group height.

That excess propagates through the toolbar row and top-panel response. The top panel's frame paint remains clipped to its configured rectangle, while the parent cursor advances to the larger returned response. Script View correctly begins at that returned boundary, leaving an unpainted workspace-colored band between the configured toolbar edge and the central surface.

This explains both prior observations:

1. Before WO-011, one normal vertical-spacing contribution remained in the outer top-padding / Item-group / bottom-padding composition.
2. A separate normal vertical-spacing contribution remained inside Elements after WO-011, so the visible gap became approximately half as large rather than disappearing.

The recommended fix is to suppress vertical spacing in the closed Elements child **before** showing the ComboBox and restore the captured normal spacing inside the popup child before creating role rows. No numeric spacing, Script View, panel-architecture, or document behavior change is needed for that correction.

## 2. Evidence boundaries

The earlier runtime evidence remains useful but incomplete:

- The stable captures summarized in [`WO-010-boundary-geometry-trace.md`](../2026-11-05-gui-spacing-and-scroll/work-orders/WO-010-boundary-geometry-trace.md) place the toolbar response bottom, central top, scroll-inner top, and first-page top at the same coordinate. Those measurements establish contiguous allocation after the toolbar response.
- They do not compare the configured toolbar height with the nested toolbar content minimum, and they do not inspect the Elements group's desired versus actual height.
- The WO-010 development trace was retired after the supplied capture; no boundary-trace source is currently present. The preserved capture summary is historical allocation/clip evidence, not a painted-pixel measurement.
- No simultaneous painted-pixel measurement was supplied. The conclusion below is a deterministic source/layout derivation consistent with the user's visual report, not a claim of agent-observed runtime confirmation.

A zero delta between toolbar response bottom and central origin is fully compatible with this owner: both can be at the same, excessive vertical coordinate.

## 3. Current-source discrepancies and record hygiene

### 3.1 Explicit toolbar bottom padding

The continuation and earlier durable records say toolbar trailing padding is zero, but current [`TOOLBAR_BOTTOM_PADDING`](../../apps/scriptcraft/src/toolbar.rs:18) is 4 points. Current [`toolbar_content_height()`](../../apps/scriptcraft/src/toolbar.rs:1021) includes that value, and current [`toolbar()`](../../apps/scriptcraft/src/toolbar.rs:840) explicitly allocates it after the Item group.

This is real source/document drift, but it is **not** the residual response-overflow owner derived below. Because the same explicit value participates in both configured height and content height, it cancels out of the mismatch equation.

The current value must not be silently reset during the residual-gap fix. Its disposition requires a user decision:

- Preserve it as intentional toolbar-owned padding and revise the durable contract to permit explicit trailing toolbar padding; or
- Restore zero separately to match the existing durable requirement and prior handoffs.

Either choice is independent of suppressing the unintended Elements cursor gap.

### 3.2 Stale source-line references

Several older notes refer to earlier line positions and values. This record uses the current checkout, including [`central_panel_frame()`](../../apps/scriptcraft/src/script_view.rs:1515) and the current toolbar constants. No conclusion relies on a stale line number from the earlier notes.

## 4. Framework facts that govern the boundary

### 4.1 Item spacing must be set before the preceding widget

Installed [`egui::Spacing::item_spacing`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:397) explicitly documents that spacing is inserted **after** adding a widget, so changing it must happen before adding that widget. Its installed default vertical value is 3 points in [`egui::Spacing::default()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:1438).

Installed [`egui::Ui::advance_cursor_after_rect()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/ui.rs:1415) captures the current Item spacing and advances the cursor after the supplied rectangle. The top-down branch adds the vertical component in [`egui::Layout::advance_after_rects()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/layout.rs:699).

Installed [`egui::Ui::scope_dyn()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/ui.rs:2430) advances the parent cursor after the child minimum rectangle. A child scope therefore returns its content extent to its parent's layout, which applies the parent's then-current Item spacing.

### 4.2 ComboBox participates in that cursor protocol

Installed [`egui::ComboBox::show_ui()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/containers/combo_box.rs:211) wraps its closed control in a horizontal child UI. Returning from that child allocates its minimum rectangle in the enclosing vertical Elements UI. The closed button itself also advances its cursor in [`button_frame()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/containers/combo_box.rs:428).

Consequently, when the ComboBox returns to [`elements_toolbar_item()`](../../apps/scriptcraft/src/toolbar.rs:626), the enclosing vertical cursor is already beyond the closed control by the normal vertical Item spacing.

### 4.3 Minimum rectangle and trailing cursor are different

Installed [`egui::Ui::cursor()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/ui.rs:1433) documents that the cursor can be one Item-spacing unit outside the minimum rectangle. A trailing cursor advance alone does not necessarily expand the returned frame.

This eliminates a competing hypothesis: merely returning from the outer toolbar scope does not automatically add another trailing spacing unit to the panel response. The residual requires a subsequent real allocation at the already-advanced cursor. Elements supplies exactly that allocation.

### 4.4 Fixed panel clipping and response-driven handoff

Installed [`egui::Panel::show_inside_dyn()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/containers/panel.rs:640):

1. Creates the configured panel rectangle and clips the panel UI to it.
2. Shows the frame and application contents.
3. Takes the frame's returned response rectangle.
4. Advances the parent cursor to that response's bottom for a top panel.

Installed [`egui::Prepared::end()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/containers/frame.rs:462) paints from the content minimum rectangle and allocates that outer rectangle as the response. With the application's margin-free frame in [`chrome_frame()`](../../apps/scriptcraft/src/main.rs:1044), an excessive content minimum therefore produces an excessive response while panel clipping prevents the excess from painting outside the configured panel rectangle.

The application then constructs the central panel after the top and bottom responses in [`show_application_panels()`](../../apps/scriptcraft/src/main.rs:1052). This is the paint/allocation split that makes the band visible.

## 5. Exact Elements overflow derivation

Define these symbols without changing any user-owned values:

| Symbol | Meaning |
|---|---|
| T | Explicit toolbar top padding |
| G | Shared Item-group height from [`toolbar_item_group_height()`](../../apps/scriptcraft/src/toolbar.rs:1017) |
| B | Explicit toolbar bottom padding |
| S | Normal vertical Item spacing captured in [`toolbar()`](../../apps/scriptcraft/src/toolbar.rs:842) |
| C | Actual closed ComboBox response height |
| K | Explicit control-to-caption gap |
| R | Remaining caption allocation height |

The configured toolbar height is:

**H = T + G + B**

The outer toolbar correction sets vertical spacing to zero before its explicit top/group/bottom allocations in [`toolbar()`](../../apps/scriptcraft/src/toolbar.rs:865), then restores normal spacing inside the horizontal Item-group child in [`toolbar()`](../../apps/scriptcraft/src/toolbar.rs:877). If every Item respects G, that outer content height is H.

Before WO-012, Elements was allocated a desired height G in [`elements_toolbar_item()`](../../apps/scriptcraft/src/toolbar.rs:626), but its sequence was:

1. Show the ComboBox while the inherited vertical spacing is still S.
2. The ComboBox returns with the enclosing cursor at C + S.
3. Only then set vertical spacing to zero.
4. Compute **R = max(G − C − K, 0)**.
5. Allocate the explicit gap K and then R.

For the ordinary case where G is at least C + K, the final allocated bottom is:

**C + S + K + R = C + S + K + (G − C − K) = G + S**

The Elements child therefore returns G + S rather than G. The row returns that excess to the outer toolbar composition, and the top-panel response becomes H + S. The visible panel remains clipped at H, while Script View starts at H + S.

The decorative caption paint itself uses the intended group-relative rectangle in [`elements_toolbar_item()`](../../apps/scriptcraft/src/toolbar.rs:626). The excess is allocation geometry, not a request to move the caption or add visible content below it.

### Numeric consistency with the report

The installed default S is 3 logical points. Before the outer correction, two S contributions could expand the toolbar composition by 2S; after it, the Elements path retains one S. At an effective scale of 2 pixels per point, that source-derived transition is approximately 12 pixels to 6 pixels. This is consistent with the user's approximate halving and the small residual band; the original screenshot's pixel estimate was not an exact measurement.

The current explicit B value does not appear in the final excess: it is present in both H and the explicit outer content composition. Changing B is neither necessary nor sufficient to remove the Elements-owned S overflow.

## 6. Why other investigated paths are not the primary owner

| Path | Finding |
|---|---|
| Script Workspace frame | [`central_panel_frame()`](../../apps/scriptcraft/src/script_view.rs:1515) remains margin-free and only supplies the workspace fill. |
| Script View top content margin | [`SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING`](../../apps/scriptcraft/src/script_view.rs:17) is zero and is passed to the scroll content margin in [`live_document_surface()`](../../apps/scriptcraft/src/script_view.rs:358). |
| First-page allocation | Pages are allocated from the scroll child in [`live_document_surface()`](../../apps/scriptcraft/src/script_view.rs:423); they follow the viewport origin rather than creating the upstream band. |
| Scrollbar origin | The scrollbar and page share the central/scroll viewport handoff, so their displaced common origin is a symptom of the panel response, not an independent toolbar fix site. |
| Status panel | Status participates in the lower remaining rectangle only. No evidence connects it to this top-only residual. |
| Outer toolbar scope trailing cursor | Framework minimum-rectangle semantics require a later allocation to materialize spacing into the response; the outer corrected composition has none beyond its explicit bottom allocation. |
| Global versus local style | [`toolbar_content_height()`](../../apps/scriptcraft/src/toolbar.rs:1021) and Item allocation could merit measurement if a mismatch remains, but no local interaction-size override was found. The Elements cursor sequence is a direct owner independent of that hypothesis. |
| Runtime/source mismatch | Retain only as a fallback if the running result cannot be reconciled after the targeted correction; do not assume it without evidence. |

No negative Script View margin, changed page-edge padding, or explicit three-region rewrite is justified by the current evidence.

## 7. Approved WO-012 — Close the Elements cursor-spacing overflow

The user approved this bounded correction and explicitly directed that the current 4-point toolbar bottom padding be preserved and allowed by the durable contract. A new pre-fix runtime trace was not required because the installed framework and application source establish the complete allocation chain.

### Implemented source behavior

1. [`elements_toolbar_item()`](../../apps/scriptcraft/src/toolbar.rs:626) captures normal vertical Item spacing and computes [`elements_role_popup_max_height()`](../../apps/scriptcraft/src/toolbar.rs:177) before changing the closed Elements child's spacing.
2. The closed Elements child sets vertical Item spacing to zero before [`egui::ComboBox::show_ui()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/containers/combo_box.rs:211), so the ComboBox cursor does not carry a hidden post-control spacing unit into the caption allocations.
3. The popup-content closure restores the captured normal spacing before creating role rows. Existing role-option font and horizontal button padding behavior remain intact.
4. Zero vertical spacing remains active in the closed Elements child for its explicit gap and remaining-caption allocations; the late spacing assignment was removed.
5. The ComboBox response, selected-role painting, decorative caption rectangle, nominal group height, action Items, overflow behavior, interaction, accessibility, and explicit toolbar padding constants are unchanged. In particular, the current 4-point [`TOOLBAR_BOTTOM_PADDING`](../../apps/scriptcraft/src/toolbar.rs:18) remains preserved and is now represented as explicitly owned trailing padding in the durable contract.
6. The WO-010 trace was retired after capture and is no longer present; no trace gate remains in the source.
7. The durable writing-loop requirement and technical blueprint now describe explicit trailing padding, bounded child responses, and normal popup row spacing.

### Expected geometry after the correction

The source layout now intends the ComboBox to return with the closed Elements cursor at C rather than C + S. With zero spacing retained for the explicit allocations, the final bottom becomes:

**C + K + R = G**

The row should no longer force the top-panel response beyond H, so the configured panel edge, returned toolbar boundary, central origin, scroll-inner origin, and first-page origin can coincide without changing Script View. This is a source-derived expectation, not a runtime result.

### Non-goals

- No change to Script View top or bottom page-edge spacing.
- No negative margin or hard-coded compensating offset.
- No toolbar/status explicit-padding change without a separate user decision.
- No ComboBox replacement or custom popup implementation.
- No explicit three-region architecture rewrite.
- No screenplay-core, persistence, pagination, export, or document-state change.
- No agent-run validation.

### User-owned acceptance checks

1. At the top scroll position, no workspace-colored band remains between the toolbar and first page.
2. The vertical scrollbar begins at the toolbar's actual lower edge.
3. Elements and action captions retain their existing alignment and clipping.
4. The Elements popup retains its existing role-row spacing, desired height, scrolling, keyboard behavior, selection, and accessibility.
5. Narrow-width overflow and its nested Elements submenu retain their existing behavior.
6. Ordinary scrolling, bottom page-end allowance, zoom compensation, visible-page status, pointer routing, caret reveal, resizing, and all Appearance themes remain coherent.
7. The user runs any desired build or workspace checks and records outcomes separately; no agent result is claimed.

## 8. Approved explicit trailing-padding disposition

The user approved preserving the current [`TOOLBAR_BOTTOM_PADDING`](../../apps/scriptcraft/src/toolbar.rs:18) value and updating [`AC-WRITING-005.8`](../../.forge/requirements/features/screenplay-writing-loop.md:615) plus the toolbar boundary paragraph in [`technical-context.md`](../../.forge/blueprints/technical-context.md:51). The exact panel-height contract now includes explicitly owned trailing padding while forbidding implicit outer gaps and nested child-response overflow. No explicit padding constant was changed by WO-012.

## 9. Conditional WO-013 — Nested boundary trace only if needed

Do not require this order if WO-012 resolves the user's visual result. Use it only if the residual persists, if the user wants direct nested geometry evidence before correction, or if another boundary discrepancy appears.

The WO-010 trace was retired after its capture and is no longer present. Do not assume its module or capture plumbing can be extended. Only if the residual persists and the user requests direct nested measurements, scope a new temporary, bounded instrumentation change for numeric, fixed-label fields such as:

- Configured toolbar height H and actual top-panel response height.
- Panel content minimum rectangle where capturable through the existing composition.
- Elements desired group height G and actual returned group response height.
- Closed ComboBox response bottom.
- Closed Elements cursor top immediately before the explicit caption-gap allocation.
- Final closed Elements content bottom.
- Existing central, scroll-inner, and first-page origins.

The decisive pre-fix relationship should be Elements actual height minus G equals S, and toolbar response height minus H equals that same excess. After correction both deltas should be zero.

Any newly authorized diagnostic capture must be bounded, numeric, free of document content, and retired after its evidence is collected. No development trace is currently present. Any further correction must target the first remaining discrepancy; the explicit three-region alternative in [`window-region-allocation.md`](../2026-11-05-gui-spacing-and-scroll/window-region-allocation.md) remains a fallback rather than an active recommendation.

## 10. Follow-up status

- [x] Obtain user approval to preserve explicit bottom padding and update the durable contract.
- [x] Implement closed-Elements spacing ordering and popup-local spacing restoration.
- [x] Reconcile explicit-padding contract in durable requirements and blueprint documentation.
- [x] Record the implementation submission and leave runtime/build validation to the user.
- [ ] Use conditional WO-013 only if user evidence shows another owner or requests nested measurements.

## 11. WO-012 implementation-submission handoff

- **Work-order record:** [`WO-012-elements-spacing-overflow.md`](work-orders/WO-012-elements-spacing-overflow.md)
- **Branch:** `dev/2026-11-06-script-view-boundary`; existing user changes were preserved.
- **Source change:** The closed Elements child now captures normal vertical Item spacing, computes popup height with that normal spacing, then suppresses vertical spacing before ComboBox allocation. Its popup child restores the captured spacing before role rows are created. The explicit caption gap and remainder allocate without the ComboBox's former inherited post-widget spacing.
- **Durable changes:** [`screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md) and [`technical-context.md`](../../.forge/blueprints/technical-context.md) now permit explicitly owned trailing toolbar padding while requiring no unintended implicit spacing or child-response overflow. The current 4-point source padding was preserved as directed.
- **Evidence limit:** The later one-off workspace check does not establish runtime layout or visual acceptance. The user owns those remaining gates. The WO-010 trace was retired after capture; WO-013 remains only a conditional proposal, and any new nested instrumentation would require separate scope and authorization if the residual persists or further evidence is requested.

## 12. One-off user-authorized workspace check

At the user's explicit request, a coding agent ran only [`cargo check --workspace`](../../.forge/project-manifest.md:197) on the current checkout. It exited successfully (code 0; reported completion in 0.22 seconds), emitted four warnings, and made no file edits. No tests, Clippy, formatter, additional build, or manual UI review was run. This is a one-off exception to the repository's otherwise user-owned validation policy, not a standing authorization for later checks.

Warnings reported:

1. **Future-incompatible float literal fallback** at [`script_view.rs`](../../apps/scriptcraft/src/script_view.rs:239): “falling back to `f32` as the trait bound `f32: From<f64>` is not satisfied”; the compiler says this behavior is being phased out and will become a hard error. Its suggestion was to use [`1.0_f32`](../../apps/scriptcraft/src/script_view.rs:239).
2. **Dead code** at [`material_font.rs`](../../apps/scriptcraft/src/material_font.rs:60): variants `Size20` and `Size48` are never constructed.
3. **Dead code** at [`material_font.rs`](../../apps/scriptcraft/src/material_font.rs:85): variants `Minus50` and `Plus200` are never constructed.
4. **Dead code** at [`toolbar_items.rs`](../../apps/scriptcraft/src/toolbar_items.rs:25): variant `Filled` is never constructed.

No deprecation warning was reported. The warnings are recorded for user disposition; no remediation was attempted.

# WO-012 — Remove closed Elements spacing overflow

- **Status:** Source and durable-contract changes submitted; workspace check succeeded with warnings; user runtime validation pending
- **Initiative findings:** [`toolbar-script-view-gap-findings.md`](../toolbar-script-view-gap-findings.md)
- **Branch:** `dev/2026-11-06-script-view-boundary`
- **Approval:** User approved the bounded Elements correction and directed preservation of the current explicit toolbar bottom padding, with corresponding durable-contract updates.
- **Validation:** A coding agent ran only [`cargo check --workspace`](../../../.forge/project-manifest.md:197) at the user's explicit request; it succeeded with four warnings and no deprecation warning. No tests, Clippy, formatter, or manual runtime verification was run.

## Outcome

Prevent the closed Elements ComboBox from adding inherited post-widget vertical spacing to the following explicit caption allocations. Keep the full Elements composition within its allocated Item-group height so the toolbar panel response does not extend below its configured and painted boundary.

## Source-derived cause

In the closed Elements child, the ComboBox was shown while normal vertical Item spacing was active. egui advances the child cursor by that spacing after the ComboBox. The code then set vertical spacing to zero and allocated an explicit control-to-caption gap and remaining caption height. Those allocations began after the already advanced cursor and expanded the child's returned minimum rectangle by one vertical spacing unit.

The top panel uses the content-derived response rectangle to advance the parent cursor, while its frame is clipped to the configured panel rectangle. The central Script View therefore follows an overextended response, exposing a workspace-colored band. Existing stable WO-010 captures did not measure the nested Elements group extent or painted toolbar edge, so this is a source/framework derivation consistent with the user report, not runtime confirmation.

## Scope and implementation requirements

1. Capture the current normal vertical Item spacing before changing the closed Elements child.
2. Calculate the popup desired height while normal spacing is active, preserving the existing popup row-gap budget.
3. Suppress vertical Item spacing before allocating the closed ComboBox.
4. Restore the captured spacing inside the popup-content child before creating role rows.
5. Keep the closed Elements spacing at zero for its explicit caption gap and remaining-caption allocations.
6. Preserve the ComboBox, selected role, caption position, common group height, keyboard and accessibility behavior, popup sizing/scrolling, overflow submenu, action Items, and toolbar interaction.
7. Preserve all current explicit padding constants, including the current 4-point [`TOOLBAR_BOTTOM_PADDING`](../../../apps/scriptcraft/src/toolbar.rs:18).
8. Do not reintroduce development tracing as part of this correction. The temporary WO-010 trace was retired after capture; any future instrumentation requires separate scope and authorization. Do not add an offset, change Script View margins, or replace panel composition.
9. Update durable requirements/blueprints for explicit trailing padding and for preventing implicit toolbar spacing/child-response overflow while preserving normal popup rows.

## Implementation submission

In [`toolbar.rs`](../../../apps/scriptcraft/src/toolbar.rs), [`elements_toolbar_item()`](../../../apps/scriptcraft/src/toolbar.rs:626) captures normal vertical spacing and computes the popup height before setting the closed child spacing to zero. The popup callback restores the captured spacing before role-row widgets; the closed control's explicit caption allocations remain under zero spacing.

The current [`TOOLBAR_BOTTOM_PADDING`](../../../apps/scriptcraft/src/toolbar.rs:18) value was not modified. [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md) and [`technical-context.md`](../../../.forge/blueprints/technical-context.md) now state that toolbar height includes any explicitly owned trailing padding, while implicit outer gaps and child-response overflow are disallowed and popup role rows retain normal spacing.

## User-owned acceptance

- At top scroll, the toolbar, scroll viewport, scrollbar, and first page appear contiguous with no workspace-colored band.
- The toolbar divider aligns with the visible lower edge.
- Elements and action captions remain aligned and unclipped.
- The Elements popup retains normal role-row rhythm, complete desired-height measurement, bounded scrolling, keyboard navigation, selection, and accessibility behavior.
- Overflow, scrolling, bottom page allowance, zoom compensation, visible-page status, pointer routing, caret reveal, resizing, and Appearance themes remain coherent.
- The user performs and records any desired builds, checks, tests, manual runtime review, and acceptance. No agent validation result is claimed.

## Implementation evidence limitations

The workspace check does not establish runtime layout or visual acceptance. If the visual residual remains and the user requests more measurements, separately scope and authorize fresh temporary nested geometry instrumentation as described in the findings document before changing offsets or panel architecture; no trace instrumentation is currently present.

## One-off user-authorized check record

The user's explicit request authorized a single coding-agent [`cargo check --workspace`](../../../.forge/project-manifest.md:197) on the current checkout. It exited with code 0 and reported completion in 0.22 seconds. It emitted four warnings:

1. Future-incompatible `float_literal_f32_fallback` at [`script_view.rs`](../../../apps/scriptcraft/src/script_view.rs:239): conversion fallback to `f32` because `f32: From<f64>` is not satisfied; the compiler says the behavior is being phased out and will become a hard error, and suggests [`1.0_f32`](../../../apps/scriptcraft/src/script_view.rs:239).
2. Dead code: `Size20` and `Size48` at [`material_font.rs`](../../../apps/scriptcraft/src/material_font.rs:60) are never constructed.
3. Dead code: `Minus50` and `Plus200` at [`material_font.rs`](../../../apps/scriptcraft/src/material_font.rs:85) are never constructed.
4. Dead code: `Filled` at [`toolbar_items.rs`](../../../apps/scriptcraft/src/toolbar_items.rs:25) is never constructed.

No deprecation warning was reported. The agent made no source edits and ran no tests, Clippy, formatter, other build/check, or manual UI review. This is a single-request exception, not standing authorization for further checks. No warning remediation was attempted.

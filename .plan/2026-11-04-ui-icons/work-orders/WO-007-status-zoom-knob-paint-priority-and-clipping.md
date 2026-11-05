# WO-007 — Status zoom knob paint priority and endpoint clipping

- **Status:** Implemented on 2026-11-05; user review and validation pending, not claimed
- **Initiative:** [`ui-icons-v1.md`](../ui-icons-v1.md)
- **Depends on:** [`WO-006`](WO-006-status-slider-circular-knob-and-rail-setting.md)
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or validators

## Outcome

Keep the custom circular zoom knob visually above every other right-hand status control, including adjacent icon hover fills, at minimum, intermediate, and maximum zoom. Allow the complete circle, stroke, and interaction expansion to render at both endpoints without being clipped by the slider's own response rectangle.

Preserve the stock slider response, zoom semantics, shared foreground colors, popup behavior, accessibility, and all current user-adjusted presentation values.

## Investigation findings

1. [`zoom_slider()`](../../../apps/scriptcraft/src/status_bar.rs:351) calls [`paint_zoom_slider_knob()`](../../../apps/scriptcraft/src/status_bar.rs:390) immediately at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:365). Appearance, Focus Mode, and Timer are subsequently painted by the right-hand sibling regions at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:1316), so later sibling painting can cover the custom knob.
2. [`paint_zoom_slider_knob()`](../../../apps/scriptcraft/src/status_bar.rs:390) intersects its painter clip with the slider response rectangle at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:414). The circular fill is radius-inset at the endpoints, but its stroke and interaction expansion can extend beyond that rectangle and be clipped.
3. Moving only the custom paint call without changing its response-local clip would retain endpoint clipping. Changing only the clip would leave later sibling controls able to overpaint the knob. Both parts belong to one correction.
4. A global foreground layer is unnecessary and risks painting over popup menus or unrelated overlays. The desired ordering is local to the right-hand status row.

## Scope

1. Carry the actual slider [`egui::Response`](../../../apps/scriptcraft/src/status_bar.rs:351) through the zoom-control rendering path so the final custom paint can occur after all right-hand sibling controls. A small explicit return/projection type is acceptable if it keeps zoom-change reporting and the response unambiguous.
2. Remove the immediate custom knob paint from [`zoom_slider()`](../../../apps/scriptcraft/src/status_bar.rs:351), but retain its stock slider creation, scoped rail height, circular handle selection, response, keyboard handling, and AccessKit metadata.
3. After Timer, Focus Mode, Appearance, slider, and percentage painting have completed, invoke [`paint_zoom_slider_knob()`](../../../apps/scriptcraft/src/status_bar.rs:390) using the retained slider response and the current zoom value. Keep this final paint within the right-hand status rendering pass, before returning from the status row.
4. Clip the final knob to the overall right-group rectangle intersected with the status row and parent clip rectangles, not to the slider response rectangle. Permit normal stroke and interaction expansion into existing inter-item gaps while preventing escape from the status row or panel.
5. Preserve the existing stock-compatible center mapping, handle radius, default/hovered foreground selection, fill, and stroke. Do not introduce a new color, numeric presentation setting, global paint layer, or replacement input model.
6. Ensure popup menus and unrelated overlays retain their normal layer ordering. The knob must be last among ordinary right-hand status controls, not globally above every application layer.
7. Record the final paint location and clip boundary in the implementation handoff. Preserve all current values in the [`presentation`](../../../apps/scriptcraft/src/status_bar.rs:19) module, including user-adjusted values.

## Non-goals

- No tooltip text edits; those remain manual user adjustments.
- No right-edge measurement or percentage alignment work; that belongs to [`WO-008`](WO-008-status-right-edge-measurement-and-alignment.md).
- No rail thickness, slider width, interaction height, icon padding, group gap, panel inset, font size, or color changes.
- No zoom range, presets, popup identity, keyboard behavior, accessibility, page retention, pinch, anchoring, persistence, pagination, or export changes.
- No new automated desktop-test target and no agent execution of validation.

## Structural acceptance for user review

1. At 75% and 300%, the full circle and stroke remain visible, including hover and drag expansion, without response-rectangle clipping.
2. Hovering or painting adjacent icon controls never covers the custom knob; the knob remains the final ordinary right-hand status paint.
3. The knob remains clipped to the status row/panel and does not paint over popup menus or unrelated overlays.
4. Slider value mapping, trailing fill, rail, colors, pointer/keyboard actions, AccessKit metadata, and shared zoom behavior remain intact.
5. All user-selected presentation values remain unchanged.

## Durable records

- Update [`AC-WRITING-006.7`](../../../.forge/requirements/features/screenplay-writing-loop.md) with the local knob paint-order and endpoint clipping contract while preserving existing zoom semantics.
- Update the status-slider paragraph in [`technical-context.md`](../../../.forge/blueprints/technical-context.md) with the deferred right-group paint and parent-row clip boundary.
- Append implementation details and evidence limitations to [`ui-icons-v1.md`](../ui-icons-v1.md) and this work order after implementation.

## User-owned review requests

- Inspect minimum, intermediate, and maximum zoom while idle, hovered, dragged, and keyboard-focused.
- Hover the adjacent Appearance icon and percentage while the knob is at each endpoint; confirm no sibling fill or padding covers it and no circle extends outside the status row.
- Open Appearance and percentage popups near the knob and confirm normal popup layering.
- Recheck slider dragging, arrows, Home/End, accessibility actions, native pinch, deep-scroll anchoring, page retention, and Save/reopen zoom restoration.
- Run any desired build/check/test/lint/formatting commands and record outcomes separately; no agent result is claimed.

## WO-007 implementation handoff — 2026-11-05

Implemented on the existing shared branch `dev/2026-11-04-ui-icons` in
[`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs). The zoom controls
now return the stock slider `Response` alongside the existing zoom-changed
result. The immediate custom knob paint was removed from `zoom_slider()`; its
stock slider response, circular handle configuration, keyboard behavior,
AccessKit metadata, value mapping, and interaction remain the input/accessibility
authority.

Final paint location: within `show_impl_with_appearance()`'s right-group
rendering callback, after zoom slider and percentage rendering and after
Appearance, Focus Mode, and Timer ordinary controls. The paint uses the current
zoom value and occurs last among ordinary right-hand status controls. The custom
circle painter is clipped to the intersection of the overall `right_rect`,
`row_rect`, and the region Ui's inherited parent clip. It no longer intersects
the stock response rectangle, so endpoint outlines and interaction expansion
can occupy available right-group gaps; painting remains bounded by the status
row and parent clip. The painter is not moved to a global foreground layer, so
normal popup and unrelated overlay layer ordering remains intact.

The only durable requirement change is within AC-WRITING-006.7. The only
technical-context change is the WO-006/WO-007 zoom-slider contract paragraph;
this WO-007 handoff and the initiative handoff record completion. The shared
foreground functions, circle geometry/stroke, stock rail and trailing fill, all
current `presentation` numeric/color settings, slider width, zoom semantics,
page retention, and popup behavior were not intentionally changed. WO-008
right-edge measurement/alignment and tooltip copy were not touched.

No implementation blocker was identified from source/document reading. The
actual rendered endpoint stroke/expansion, clipping in desktop contexts, popup
layering, compilation, and interaction/accessibility behavior remain unresolved
until user-owned review/validation. No build, check, test, lint, formatting
command, manual verification, implementation review, or validator was run; no
check is claimed as passed. User validation and acceptance remain pending.

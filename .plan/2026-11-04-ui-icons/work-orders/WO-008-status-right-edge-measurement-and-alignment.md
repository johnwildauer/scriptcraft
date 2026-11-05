# WO-008 — Status right-edge measurement and percentage alignment

- **Status:** Implemented on 2026-11-05 after WO-007 handoff; user review and validation pending, not claimed
- **Initiative:** [`ui-icons-v1.md`](../ui-icons-v1.md)
- **Depends on:** [`WO-007`](WO-007-status-zoom-knob-paint-priority-and-clipping.md)
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or validators

## Outcome

Remove unnecessary and digit-count-dependent space at the right edge by measuring the percentage currently displayed and aligning its galley to an explicit trailing inset. Keep the right-hand group anchored to the actual status content edge.

Preserve all current user-selected numeric presentation values. The implementation corrects measurement and alignment; it does not reset the panel inset or percentage reservation to agent-selected numbers.

## Investigation findings

1. [`plan_status_row()`](../../../apps/scriptcraft/src/status_bar.rs:223) already anchors the percentage rectangle directly to the row's right edge at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:282). The primary excess is therefore not a missing right-alignment of the overall group.
2. [`show_impl_with_appearance()`](../../../apps/scriptcraft/src/status_bar.rs:1237) measures the fixed string “300%” at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:1274), regardless of the current percentage. This reserves unused width for shorter values.
3. [`zoom_presets_button()`](../../../apps/scriptcraft/src/status_bar.rs:443) centers the galley at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:469). With a fixed maximum-width allocation, different digit counts produce different visual trailing gaps.
4. [`PANEL_HORIZONTAL_INSET`](../../../apps/scriptcraft/src/status_bar.rs:23) and [`PERCENTAGE_SIDE_RESERVATION`](../../../apps/scriptcraft/src/status_bar.rs:31) are deliberate user-adjustable settings. Their current values must be preserved; right alignment should make their effect predictable rather than silently changing them.
5. Font advance and side bearings can contribute a small natural visual inset. Pixel-perfect ink-edge alignment would require glyph-bound measurement and is not needed for this correction; consistent galley advance-edge placement is the intended contract.

## Scope

1. In [`show_impl_with_appearance()`](../../../apps/scriptcraft/src/status_bar.rs:1237), construct the actual current percentage label and pass that same value to [`measured_button_width()`](../../../apps/scriptcraft/src/status_bar.rs:1194) instead of measuring “300%”. Keep measurement and displayed text based on the same whole-percent value for the frame.
2. In [`zoom_presets_button()`](../../../apps/scriptcraft/src/status_bar.rs:443), right-align the percentage galley inside its allocated rectangle using [`PERCENTAGE_SIDE_RESERVATION`](../../../apps/scriptcraft/src/status_bar.rs:31) as the explicit trailing inset. Keep vertical centering unchanged.
3. Continue measuring percentage width as current text width plus the existing leading and trailing reservations. Do not add another hidden margin or hard-coded maximum-label width.
4. Keep finite, nonnegative geometry and existing clipping at compressed or physically unusable widths. Right alignment must not produce invalid positions, overlap the slider, panic on narrow rectangles, or expand the row beyond its allocation.
5. Preserve [`plan_status_row()`](../../../apps/scriptcraft/src/status_bar.rs:223)'s fixed right-edge anchoring, control priority, slider growth/compression, page/center allocations, and nonoverlap. The actual current label measurement may change the group's required width, but not its priority or right-edge anchor.
6. Preserve the current [`PANEL_HORIZONTAL_INSET`](../../../apps/scriptcraft/src/status_bar.rs:23), [`PERCENTAGE_SIDE_RESERVATION`](../../../apps/scriptcraft/src/status_bar.rs:31), item gaps, icon sizes, typography, rail height, colors, and every other user-adjusted value.
7. Preserve percentage popup identity, exact presets, pointer/Enter/Space activation, outside-click/Escape dismissal, accessible current-value naming, hover behavior, and zoom-change routing. Preserve WO-007's deferred knob paint and clip behavior.
8. Record the actual measurement and right-alignment locations in the implementation handoff, and explain that the remaining intentional trailing distance is the panel horizontal inset plus the percentage trailing reservation in logical points.

## Non-goals

- No agent-selected reduction of the panel inset, percentage reservation, gaps, padding, fonts, slider rail, or slider width.
- No glyph-ink-bound measurement, device-pixel rounding policy, global panel/frame change, toolbar change, or unrelated layout redesign.
- No tooltip text edits.
- No knob paint-order work; that belongs to [`WO-007`](WO-007-status-zoom-knob-paint-priority-and-clipping.md).
- No zoom range, preset, accessibility, page retention, native pinch, anchoring, persistence, pagination, or export changes.
- No new automated desktop-test target and no agent execution of validation.

## Structural acceptance for user review

1. Allocation measures the actual displayed label at 75%, 90%, 100%, and 300%, with no fixed “300%” measurement remaining.
2. The percentage galley's trailing advance edge is consistently placed by the explicit trailing reservation, independent of digit count.
3. The right group remains flush with the status content edge; remaining trailing distance is explained solely by the intentional panel inset, percentage reservation, and normal font bearing.
4. Narrow-width clipping, right-control priority, slider growth/compression, page/center behavior, and stable row height remain intact.
5. Popup behavior, accessibility, hover, zoom semantics, and WO-007 knob layering remain unchanged.
6. No current user-adjusted numeric or color setting is reset or replaced.

## Durable records

- Update [`AC-WRITING-006.4`](../../../.forge/requirements/features/screenplay-writing-loop.md) and [`AC-WRITING-006.7`](../../../.forge/requirements/features/screenplay-writing-loop.md) with current-label measurement and explicit trailing-inset alignment while retaining priority and zoom behavior.
- Update the status allocation/zoom paragraph in [`technical-context.md`](../../../.forge/blueprints/technical-context.md) with the right-edge formula and logical-point units.
- Append implementation details and evidence limitations to [`ui-icons-v1.md`](../ui-icons-v1.md) and this work order after implementation.

## User-owned review requests

- Compare 75%, 90%, 100%, and 300%; confirm the visible trailing gap remains consistent and no unused maximum-label width remains in allocation.
- Review wide, former-breakpoint, 480, 320, and extreme widths for right-edge fit, slider/percentage separation, page/center clipping, and nonoverlap.
- Change the panel inset and percentage reservation manually and confirm each has a direct, predictable effect in logical points.
- Recheck percentage hover, popup activation/dismissal, exact presets, keyboard/accessibility behavior, and every zoom route.
- Confirm the circular knob remains complete and above siblings at both endpoints after WO-007.
- Run any desired build/check/test/lint/formatting commands and record outcomes separately; no agent result is claimed.

## WO-008 implementation handoff — 2026-11-05

Implemented on the existing shared branch `dev/2026-11-04-ui-icons` in
[`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs). In
[`show_impl_with_appearance()`](../../../apps/scriptcraft/src/status_bar.rs:1248),
the displayed whole-percent label is prepared once for the frame and measured by
[`measured_button_width()`](../../../apps/scriptcraft/src/status_bar.rs:1205).
That current text advance plus the existing leading/trailing
[`PERCENTAGE_SIDE_RESERVATION`](../../../apps/scriptcraft/src/status_bar.rs:31)
is passed to the existing bounded row planner; the fixed “300%” measurement has
been removed. The same label is passed into
[`zoom_presets_button()`](../../../apps/scriptcraft/src/status_bar.rs:451) for
the displayed galley and accessible current-value name.

At the galley positioning code
([`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:479)), the text's
advance edge is right-aligned inside its allocation with the current percentage
side reservation as an explicit trailing inset, retaining vertical centering. If
the allocated width is too narrow to fit the inset and text, the text origin is
bounded at the allocation's left edge and painting remains clipped to the
allocation. The existing row planner still preserves finite, nonnegative
compressed rectangles, right-control priority, slider growth/compression, and
right-edge anchoring. No numeric/color presentation value was changed.

The remaining intentional trailing distance from the status panel's outer edge
to the galley's trailing advance edge is the sum of
[`PANEL_HORIZONTAL_INSET`](../../../apps/scriptcraft/src/status_bar.rs:23) and
[`PERCENTAGE_SIDE_RESERVATION`](../../../apps/scriptcraft/src/status_bar.rs:31)
logical points, currently 12 + 8 = 20 points. Font advance/bearings may add a
natural ink-edge gap. WO-007's deferred final knob paint and its
right-group/status-row/parent clipping remain untouched. Popup identity/presets,
keyboard activation, dismissal and accessibility, hover, page retention, zoom
semantics, and tooltip text were not intentionally changed.

No blocker was identified from authorized source and documentation reading. No
build, check, test, lint, formatting command, manual verification, implementation
review, or validator was run. Compilation and visual, interaction, accessibility,
clipping, and regression behavior remain unverified; all review/validation and
acceptance are explicitly user-owned and pending.

# WO-003 — Status priority and zoom readout

- **Status:** Implementation submitted 2026-11-04; user review/validation pending
- **Initiative:** [`ui-icons-v1.md`](../ui-icons-v1.md)
- **Depends on:** [`WO-002`](WO-002-status-material-icons-and-hover.md)
- **Validation owner:** User; no agent-run checks or independent reviewers/validators

## Outcome

Make every supported-width status row render Sprint Timer, Focus Mode, Appearance, the zoom slider, and the zoom percentage inline. Remove the status More button and right-item overflow planner completely. Allocate space by rendering priority rather than three equal columns: right controls first, visible-page indicator second, center operation/error text last.

Within the right-hand control group, remove every visible caption and arrow. The only ordinary text is the far-right integer percentage and percent sign. Popup menu entries and hover/accessibility text remain available.

## Scope

### 1. Replace equal columns with priority-based rectangles

Refactor [`show_impl_with_appearance()`](../../../apps/scriptcraft/src/status_bar.rs:1197) so it no longer uses three equal columns for the primary row. Use the actual clipped content width and explicit, nonoverlapping child regions with the existing bounded one-row height.

A small deterministic layout helper is preferred. Its inputs should include available width, item spacing, measured page-label width, measured percentage width, actual icon-control widths, and the existing slider bounds of 40 through 165 points. Its output should define page, center, slider, percentage, and icon rectangles without depending on status-message length.

Apply this priority:

1. Reserve the three icon controls, percentage control, inter-item gaps, and a usable slider at the far right.
2. Grow the slider from its 40-point minimum toward the existing 165-point maximum only from width that remains after fixed controls and the page allocation are considered.
3. Keep the page indicator at the far left, clipped to its own bounded region if an extreme narrow viewport requires it.
4. Give the center message only the residual rectangle between page and right controls. When no residual width exists, paint no center message; never take width back from the right controls to show it.

At the established 1200, 480, and 320 logical-point review widths, all five right-hand elements must be wholly inside the visible row and independently usable. At widths immediately around the former 720/721 breakpoints, behavior must be continuous: there is no compact overflow mode and no More threshold. Below the minimum physically usable viewport, retain bounded clipping and nonoverlapping hit regions without promising usable controls in zero width.

### 2. Remove status overflow rather than bypassing it

Delete or fully retire the following status-only overflow machinery after its callers are gone:

- [`RightStatusItemWidth`](../../../apps/scriptcraft/src/status_bar.rs:145), [`RightStatusLayout`](../../../apps/scriptcraft/src/status_bar.rs:151), and [`plan_right_status_items()`](../../../apps/scriptcraft/src/status_bar.rs:157);
- [`overflow_menu()`](../../../apps/scriptcraft/src/status_bar.rs:1082) and both More/! More trigger labels;
- any ordering, width-measurement, or appearance-choice helper that becomes dead solely because overflow is gone;
- the now-unneeded visible Focus Mode button-text projection in [`focus_mode.rs`](../../../apps/scriptcraft/src/focus_mode.rs:39), only after no overflow caller remains.

Do not remove or change the toolbar's separate overflow menu. Do not hide More with styling while retaining unreachable status actions.

### 3. Simplify the zoom slider without losing accessibility

Refactor [`zoom_controls()`](../../../apps/scriptcraft/src/status_bar.rs:244) and [`zoom_slider()`](../../../apps/scriptcraft/src/status_bar.rs:280):

- Keep one whole-percent value, inclusive 75–300 range, integer stepping, proportional trailing fill, and hidden slider value text.
- Remove the visible Page zoom caption and all width reservations associated with that caption or the old combo-box arrow.
- Preserve an accessible slider name such as Page zoom and its current/min/max values even though no caption is painted.
- Preserve focused arrow-key behavior and the existing Home/End endpoint handling.
- Continue reporting every slider/preset mutation through the single zoom-changed result consumed by [`show_application_panels()`](../../../apps/scriptcraft/src/main.rs:2558), so pinch baseline synchronization and viewport-center zoom compensation remain unchanged.

### 4. Replace the stock percentage combo box with an arrowless popup control

Replace the closed stock combo box in [`zoom_controls()`](../../../apps/scriptcraft/src/status_bar.rs:260) with a plain, frame-consistent button/response plus an egui 0.34 popup anchored to that response. The closed control paints only the current integer and percent sign, for example 100%; it has no chevron, arrow, icon, or surrounding words.

The popup must:

- contain exactly 75%, 90%, 100%, 125%, 150%, 175%, 200%, 250%, and 300%, in that order;
- update the same zoom value as the slider and native pinch path;
- display every intermediate integer accurately when opened or closed;
- close after selection and retain normal outside-click/Escape popup behavior;
- expose an accessible name that identifies page-zoom presets and the current percentage;
- support pointer and keyboard opening/selection without stealing focus during ordinary frames;
- keep stable popup IDs and avoid conflicting with Appearance or timer popups.

Do not simulate a hidden arrow by painting over a stock combo box. The closed widget itself should have no arrow primitive.

### 5. Preserve surrounding status behavior

- Keep operation/error projection, privacy-safe wording, red error color, left alignment, noninteractive text semantics, and clipping in [`paint_clipped_single_line()`](../../../apps/scriptcraft/src/status_bar.rs:804).
- Keep visible-page selection/freshness and the unavailable-on-zoom-change behavior.
- Keep Diagnostics collapsed by default, its optional fixed second row, and its single-line clipped summary.
- Keep the return values and caller-owned timer/focus/appearance/zoom actions equivalent; simplify only genuinely dead rectangle/ID plumbing after checking all callers.
- Maintain the established stable panel-height contract across repeated frames and changing status strings.

## Non-goals

- No zoom range, preset, zoom-anchor, native pinch, TextState, persistence, page geometry, pagination, source-map, PDF, or export changes.
- No new error-details popup or substitution for the center error text.
- No toolbar overflow change, toolbar glyph change, Diagnostics removal, or new status action.
- No minimum native window size imposed as a substitute for the responsive layout.
- No new automated desktop-test target unless separately authorized by the user.

## Structural acceptance

1. No More or ! More status trigger can be emitted at any width, and no status right item has an inline/overflow dual representation.
2. At 1200, former-breakpoint-adjacent, 480, and 320 widths, the visible-page region and all five right-hand elements remain within their intended clips; center text cannot overlap or displace them.
3. Long and changing operation/error messages affect only center clipping. They cause no horizontal displacement, new row, monotonic height growth, or More button.
4. The right side shows three Material icons, an uncaptioned slider, and only the current percentage text. No closed-control arrow is present.
5. The zoom popup retains exactly nine ordered presets; slider, popup, displayed percentage, native pinch baseline, and saved TextState percentage remain synchronized.
6. Pointer and keyboard operation, accessible names/values, timer anchoring, Appearance choices, Focus Mode gating, page freshness, and Diagnostics remain intact.
7. Extreme/zero-width handling remains bounded and nonoverlapping without reintroducing overflow or claiming physically impossible hit-target usability.

## Durable records

- Revise [`AC-WRITING-006.1`](../../../.forge/requirements/features/screenplay-writing-loop.md:569) to require permanently inline right controls at supported widths and remove status overflow language.
- Revise [`AC-WRITING-006.4`](../../../.forge/requirements/features/screenplay-writing-loop.md:598) to state right-group priority, residual center clipping, no More fallback, and the retained physically unusable/zero-width exception.
- Revise [`AC-WRITING-006.7`](../../../.forge/requirements/features/screenplay-writing-loop.md:619) only to describe the uncaptioned accessible slider and arrowless current-percentage popup while preserving its complete zoom semantics and exact presets.
- Verify AC-WRITING-006.8–006.10 and 006.12–006.13 remain behaviorally intact; amend wording only if the new presentation requires it.
- Update the status-row description and icon-foundation paragraph in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:43) with the explicit allocation priority, no-overflow rule, and arrowless percentage popup. Preserve the separate deep-scroll zoom coordinate contract.
- Append implementation details, unresolved evidence, and eventual user-supplied validation to [`ui-icons-v1.md`](../ui-icons-v1.md) and this work order. Do not mark the initiative accepted without the user's explicit disposition.

## User-owned review matrix

| Area | Requested user observation |
|---|---|
| Width priority | Resize gradually through wide, 721, 720, 480, and 320 widths; confirm all five right-hand elements stay visible and no More button ever appears |
| Long status/error | Produce short, long, and error messages at each width; confirm only the center text clips and right controls never move |
| Hover/focus | Compare the three icon hovers with toolbar hover; tab and activate all controls by keyboard |
| Zoom slider | Drag both directions, use arrows and Home/End, verify 75/300 endpoints and intermediate percentages |
| Zoom presets | Open the arrowless percentage, verify exact entries/order, select every preset, dismiss by outside click and Escape |
| Zoom preservation | Repeat status zoom at deep scroll and with native pinch; confirm established center anchoring and smoothness remain |
| Timer/focus/appearance | Recheck popup anchoring, focus safety states, all four Appearance choices, and icon-only closed controls |
| Diagnostics | Open/close Diagnostics with changing counters; confirm only its intentional second row affects height |
| Persistence/core | Save, close, and reopen to confirm zoom restoration; run desired workspace/core checks and confirm no document/export behavior changed |

All results and acceptance remain user-owned. Implementation details and evidence limits are recorded below.

## Implementation handoff — 2026-11-04

Implemented on the shared branch `dev/2026-11-04-ui-icons`. In
[`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs), the status row
now uses a deterministic bounded rectangle allocator based on clipped available
width, measured page and percentage labels, three actual icon widths, item
spacing, and the 40–165 point slider bounds. The right-hand group is reserved
first and anchored at the far right; after it, the page indicator receives its
measured width or a clipped remainder; center operation/error text receives
only the residual space. Slider growth consumes only remaining width after
fixed controls and page allocation. Slots and gaps compress proportionally
below the minimum physically usable width to keep layout and hit regions within
bounds; that does not promise usable controls at zero width. The row remains
one line and Diagnostics retains its existing optional second row.

Removed the status overflow planner, More/! More disclosure and content path,
right-item ordering/measurement plumbing, overflow-only appearance choice
helper, and the obsolete Focus Mode visible button-text projection. The
toolbar's separate overflow remains untouched. Timer, Focus Mode, and Appearance
still use their WO-002 Material icon response and hover paths. Timer activation
still returns the actual status-icon response rectangle used as the popup
anchor; Focus Mode still uses the existing state gate and accessible status;
Appearance retains its four menu choices and change result.

Zoom remains one shared 75–300 inclusive whole-percent integer. The slider is
uncaptioned, retains trailing fill and hidden value text, and publishes its
Page zoom accessible name plus current/minimum/maximum and step values; its
focused arrow and Home/End handling remain. The far-right percent is an
arrowless text-only clickable control with a stable popup ID, current-value
accessible name, and the nine exact ordered presets. Its button activation
supports the existing pointer and Enter/Space path, and the egui 0.34 menu
retains outside-click/Escape dismissal. Slider and preset mutations still
produce the existing `zoom_changed` result consumed by the caller for pinch
baseline synchronization, viewport-centered prepaint compensation, and root
FDX TextState persistence. No core document semantics, pagination, or export
behavior was intentionally modified.

The scoped writing-loop requirements and technical context now describe the
priority allocation and zoom presentation. WO-001/WO-002 behavior and this
implementation remain unbuilt and unvalidated. No build, check, test, lint,
formatting check, manual verification, implementation review, or independent
validation was run. Eframe/egui migration compatibility and status layout,
popup, accessibility, and persistence behavior remain unestablished. All
review and validation are pending with the user; this handoff is not acceptance
or initiative closeout.

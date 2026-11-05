# V2-WO-002 — Stable centered percentage slot

- **Status:** Implementation submitted on 2026-11-05 after V2-WO-001 handoff; user review and validation pending
- **Initiative:** [`ui-icons-v2.md`](../ui-icons-v2.md)
- **Depends on:** V2-WO-001 implementation handoff; preserve its trace
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or validators

## Outcome

Use one stable percentage slot measured from the active-font maximum reference label, with the actual current percentage centered inside it. Prevent digit-count changes from moving Timer, Focus Mode, Appearance, the slider allocation, or the residual center region.

Use the planned percentage rectangle as the single geometry authority for allocation, interaction, clipping, accessibility, popup anchoring, and painting rather than reconstructing its position through nested sequential widths.

## Scope

1. Replace current-label slot measurement in [`show_impl_with_appearance()`](../../../apps/scriptcraft/src/status_bar.rs:1248) with a helper that measures the maximum reference derived from [`MAX_ZOOM_PERCENT`](../../../apps/scriptcraft/src/status_bar.rs:14), using [`status_font_id()`](../../../apps/scriptcraft/src/status_bar.rs:69), then adds both existing [`PERCENTAGE_SIDE_RESERVATION`](../../../apps/scriptcraft/src/status_bar.rs:31) values.
2. Continue constructing the actual current label for visible painting and the accessible current-value name. Never expose the reference label as the accessible value.
3. Preserve [`plan_status_row()`](../../../apps/scriptcraft/src/status_bar.rs:223)'s priority, right-edge anchor, compression, and slider growth, but pass the stable slot width so digit count no longer changes those calculations.
4. Make the explicit planned percentage rectangle authoritative end-to-end in [`zoom_presets_button()`](../../../apps/scriptcraft/src/status_bar.rs:451). Use a scoped child at that rectangle or equivalent exact allocation; do not introduce a second effective right edge from [`zoom_controls()`](../../../apps/scriptcraft/src/status_bar.rs:333)'s nested horizontal layout.
5. Center the actual galley horizontally and vertically when it fits. At compressed widths, bound the origin and clip to the explicit rectangle without expanding, overlapping the slider, or panicking.
6. Preserve the existing response ID, full-slot hit region, hover treatment, popup anchor/menu/presets, Enter/Space and pointer activation, outside-click/Escape dismissal, and actual-value accessibility.
7. Preserve WO-007's deferred knob paint and parent-row clipping, every user-adjusted numeric/color value, and V2-WO-001's geometry trace.
8. Update [`AC-WRITING-006.4`](../../../.forge/requirements/features/screenplay-writing-loop.md), [`AC-WRITING-006.7`](../../../.forge/requirements/features/screenplay-writing-loop.md), the scoped [`technical-context.md`](../../../.forge/blueprints/technical-context.md) paragraphs, the initiative, and this handoff. Supersede WO-008's current-label/right-alignment contract explicitly.

## Centering contract

The slot width and text center remain fixed. Shorter labels intentionally have more unused side space. Centering is not expected to preserve WO-008's constant galley trailing advance edge. Right-edge investigation must therefore distinguish slot/right-group geometry from centered-galley whitespace.

## Non-goals

- No hand-picked fixed numeric width and no change to font size, reservations, panel inset, gaps, or colors.
- No final margin correction; V2-WO-004 owns any measured residual allocation defect.
- No Diagnostics gate, tooltip, zoom-range, persistence, page, pagination, or export changes.
- No agent-run validation.

## Structural acceptance for user review

1. Values 75, 90, 99, 100, 101, 125, 200, and 300 share one nominal slot and stable surrounding group geometry.
2. Actual text is centered, accessible names remain current, and compressed painting stays bounded.
3. The actual percentage rectangle equals the planned rectangle at ordinary widths, or the trace identifies the exact remaining delta for V2-WO-004.
4. Popup, keyboard, slider, knob layering, and all manual settings remain intact.

## Implementation handoff — 2026-11-05

Implemented on the existing shared `dev/2026-11-04-ui-icons` branch. This record is not a review or validation report. No build, check, test, lint, formatting command, application run, manual verification, review, or other validation was performed or claimed. User owns all review and validation. The user-supplied baseline file remains unchanged. V2-WO-003 Diagnostics gating and V2-WO-004 final margin correction/trace removal are out of scope and remain unimplemented.

### Baseline evidence from the user trace

[`W0-002-grep-report.txt`](W0-002-grep-report.txt) is user-supplied evidence, not agent validation. At egui content width 1512, planned row/content right is 1500, planned slider width is 165, actual slider width is 100, actual zoom right is 1439, and actual percentage right is 1435, leaving a 65-point actual-vs-planned percentage-right delta. Galley advance right is 1427: the content-right-to-galley delta is 73 points, while egui window right 1512 to galley advance is 85 points = 170 physical pixels at effective ppp 2. These measured rectangle deltas are geometric evidence, not a font-bearing explanation. This evidence motivated making the planned slot authoritative; it does not claim a post-slot result or prove a final margin correction.

### Exact implementation decisions and locations

- [`measured_percentage_slot_width()`](../../../apps/scriptcraft/src/status_bar.rs:1413) measures the `MAX_ZOOM_PERCENT` reference plus `%` with [`status_font_id()`](../../../apps/scriptcraft/src/status_bar.rs:254) in the active status Button font and adds the two existing [`PERCENTAGE_SIDE_RESERVATION`](../../../apps/scriptcraft/src/status_bar.rs:31) reservations. No numeric/color/presentation values changed.
- [`show_impl_with_appearance()`](../../../apps/scriptcraft/src/status_bar.rs:1480) retains the actual current label for visible paint and accessibility, passing only the stable width to [`plan_status_row()`](../../../apps/scriptcraft/src/status_bar.rs:408).
- [`zoom_controls()`](../../../apps/scriptcraft/src/status_bar.rs:518) creates bounded scoped children at the planner's exact slider and percentage rectangles; it no longer uses nested horizontal sequential layout to reconstruct the percentage's position. The slider remains the stock egui response, and WO-007's deferred final knob paint and right-group/row/parent clip remain intact.
- [`zoom_presets_button()`](../../../apps/scriptcraft/src/status_bar.rs:647) uses its planned scoped rectangle for slot allocation, clipped interaction/accessibility response, hover, popup anchor, painter clip, and painting. It keeps the actual current label and centers its galley horizontally/vertically when it fits. Narrow rendering bounds its origin and clips to the slot without enlarging or overlapping neighboring controls.
- V2-WO-001 trace instrumentation remains enabled. The removed nested horizontal UI is represented as unavailable in that trace field; planned/actual slider and percentage geometry and galley/clip data remain captured for user evidence.
- Updated [`AC-WRITING-006.4`](../../../.forge/requirements/features/screenplay-writing-loop.md:615), [`AC-WRITING-006.7`](../../../.forge/requirements/features/screenplay-writing-loop.md:652), scoped [`technical-context.md`](../../../.forge/blueprints/technical-context.md:91), and the V2 initiative. This explicitly supersedes WO-008's current-label-width and right-aligned-galley contract; its historical handoff is not silently rewritten.

### User-owned post-slot trace

Follow the capture dimensions, scale/screenshot notes, and tagged stderr extraction in [`V2-WO-001`](WO-001-status-right-edge-geometry-trace.md:39) against the current post-WO-002 checkout. Keep the original baseline and stderr evidence untouched; use a separate capture such as `/tmp/scriptcraft-v2-wo-002.stderr`, launch the user's current checkout with the same command and redirect stderr there, and extract only lines matching `^\[V2-WO-001 status geometry\]`. Capture normal size plus approximately 1200, 480, and 320 logical-point content widths where practical and one different zoom value, with screenshots and explicit native/effective scale, output scaling, and edge reference. Compare the planned and actual percentage/slider rectangles, zoom right edge, galley advance, and clip. This is an instruction for the user, not an agent-run evidence capture. V2-WO-004 remains blocked on that post-slot user trace and must not guess a margin.

### Agent command execution authorized by user — 2026-11-05

This is a command-execution record only, not agent review, acceptance, interpretation, or validation. At the user's explicit direction, ran `cargo run -p scriptcraft` from the repository root with stderr redirected to `/tmp/scriptcraft-v2-wo-002.stderr`. After observing a tagged geometry frame and collecting for five additional seconds, sent SIGINT to only the process group launched for this run; Cargo exited with return code `-2` (signal termination). Extracted only lines beginning with `[V2-WO-001 status geometry]` into [`WO-002-post-slot-trace.txt`](WO-002-post-slot-trace.txt). The capture contains two naturally produced frames, both at 800×600 egui content size, 100% zoom, and native/effective scale 2.000 pixels per point. No interactive resize or alternate zoom was captured. The original baseline [`W0-002-grep-report.txt`](W0-002-grep-report.txt) was not changed. Interpretation and all review/validation remain user-owned.

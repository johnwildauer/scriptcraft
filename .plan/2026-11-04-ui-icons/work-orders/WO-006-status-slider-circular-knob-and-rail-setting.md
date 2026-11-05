# WO-006 — Status slider circular knob and rail setting

- **Status:** Implemented on 2026-11-04 after WO-005 handoff; user review and validation pending, not claimed
- **Initiative:** [`ui-icons-v1.md`](../ui-icons-v1.md)
- **Depends on:** [`WO-005`](WO-005-status-local-presentation-and-percentage-hover.md)
- **Validation owner:** User; no agent-run checks or independent reviewers/validators

## Outcome

Render the status zoom knob as a full circle filled from the same shared status-local default/hovered foreground settings used by status text and icons. Expose a status-local rail-thickness setting initialized to the inherited 8-point value without changing that number during implementation. Preserve slider width, range, trailing fill, input, accessibility, and shared zoom behavior.

The user may subsequently reduce rail thickness manually by 30–40%, corresponding to 4.8–5.6 logical points from the current 8-point baseline. That numeric reduction is not applied by this proposal.

## Investigation findings

1. [`zoom_slider()`](../../../apps/scriptcraft/src/status_bar.rs:275) uses the stock slider with integer stepping, trailing fill, and hidden value text but no per-widget shape or rail-height override.
2. egui 0.34 exposes per-slider [`Slider::handle_shape()`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/widgets/slider.rs:388). The inherited default is a rounded rectangle with 0.75 aspect ratio at [`Visuals::dark()`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:1480), specifically [`style.rs`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:1536).
3. Rail thickness comes from [`Spacing::slider_rail_height`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:425), defaulting to 8 logical points at [`style.rs`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:1448). The rail rectangle and fill are painted at [`slider.rs`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/widgets/slider.rs:776). There is no per-slider rail-height builder method; a scoped child Ui spacing override is required.
4. The stock knob radius is response height divided by 2.5 at [`Slider::handle_radius()`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/widgets/slider.rs:886). Circle painting uses interaction-state background fill and foreground stroke at [`slider.rs`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/widgets/slider.rs:817). Trailing fill instead uses selection background at [`slider.rs`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/widgets/slider.rs:787).
5. Changing shared widget background fill is not a clean knob-only color override: the rail always uses inactive widget background fill at [`slider.rs`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/widgets/slider.rs:772), so the same scoped modification can recolor the rail as well as the knob. Changing global handle shape, rail spacing, or widget visuals would affect other descendants or every slider in that context.
6. [`Ui::scope()`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/ui.rs:2399) supports temporary child-style changes, and [`Ui::spacing_mut()`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/ui.rs:455) applies only to that Ui and subsequent children. This is appropriate for rail height but must not be broadened to global style mutation.
7. The slider's allocated width is explicitly planned at 40–165 logical points in [`show_impl_with_appearance()`](../../../apps/scriptcraft/src/status_bar.rs:1119). Rail height does not require changing those bounds, the 18-point default interaction height, hit target, keyboard behavior, or AccessKit values.
8. Status icons currently use the interaction-state foreground at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:477). [`WO-005`](WO-005-status-local-presentation-and-percentage-hover.md) establishes shared, easily adjustable status-local default and hovered foreground settings for status text and icons. The knob must consume those settings rather than introducing another color source or literal.

## Scope

1. Add a status-local, commented rail-height setting initialized to 8 logical points and apply it only within the child Ui that owns [`zoom_slider()`](../../../apps/scriptcraft/src/status_bar.rs:275). Do not modify global [`Spacing::slider_rail_height`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:425), other sliders, toolbar, or context styles.
2. Select [`HandleShape::Circle`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:1214) per status slider through [`Slider::handle_shape()`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/widgets/slider.rs:388). Do not set the global visuals handle shape.
3. Reuse the stock slider response, value mapping, input, and accessibility. Add a status-local final knob fill pass using public egui painting APIs:
   - compute the same horizontal normalized value position and circular position range used by the stock slider, including the radius from [`Slider::handle_radius()`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/widgets/slider.rs:886) and interaction expansion;
   - fill the circle with the shared status-local default or hovered foreground setting established by [`WO-005`](WO-005-status-local-presentation-and-percentage-hover.md), matching the stock circle's foreground stroke so no differently colored rim remains;
   - clip the fill to the slider's allocated region and preserve the existing rail and value-proportional trailing fill beneath it.
   Do not mutate widget background fills to obtain the knob color, because that also changes the rail. Do not replace the stock response with a new input model merely to restyle the knob.
4. Keep [`Slider::integer()`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/widgets/slider.rs:597), [`Slider::trailing_fill()`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/widgets/slider.rs:376), hidden value text, 75–300 inclusive range, Home/End handling, explicit AccessKit name/range/value/step, and the single zoom-changed result in [`zoom_slider()`](../../../apps/scriptcraft/src/status_bar.rs:275).
5. Preserve the 40-point minimum and 165-point maximum slider widths at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:1125), all layout compression/priority rules, popup behavior, viewport-centered compensation, native pinch baseline, and TextState persistence.
6. Record the new rail-height manual-adjustment location and the public-API paint approach in the implementation handoff. Do not change the initial 8-point value or claim a 30–40% visual reduction until the user selects it.

## Optional user numeric adjustment after implementation

Reduce only the new status-local rail-height setting from 8 to a selected value in the 4.8–5.6 logical-point range for a 30–40% reduction. Keep slider width and interaction height unchanged. Review circle/rail centering, trailing-fill junctions, and clipping at endpoints and narrow widths after the adjustment.

Logical points convert to physical pixels through [`Context::pixels_per_point()`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/context.rs:2238); the range above is not a promise of identical physical-pixel thickness across display scales.

## Shared-style and color boundaries

- Per-slider shape selection affects only this slider. Global handle-shape modification is out of scope.
- Scoped rail spacing affects the slider child Ui and its subsequent children only; the implementation must keep that scope narrow.
- Widget background-fill modification is not an acceptable knob-only shortcut because the stock rail consumes the same inactive fill.
- The knob follows the shared status-local default and hovered foreground settings established by WO-005, so editing either setting also updates the corresponding knob state alongside status text and icons. Do not add a knob-specific foreground literal.

## Non-goals

- No rail-thickness numeric change, slider-width change, interaction-height reduction, or new zoom behavior.
- No replacement of trailing selection fill, percentage popup, presets, range, stepping, zoom anchoring, persistence, pagination, or exports.
- No global style, toolbar, other-slider, document typography, or Appearance palette changes.
- No page-retention work; that belongs to [`WO-004`](WO-004-document-scoped-visible-page-retention.md).
- No new automated desktop-test target and no agent execution of any check or validation.

## Structural acceptance for later user review

1. The knob is circular at minimum, intermediate, and maximum values and uses the same interaction-state foreground source as status icons, with no rounded-rectangle remnant or mismatched rim.
2. The rail uses one status-local thickness setting initialized to 8 points; width remains within the existing 40–165-point allocation and trailing fill remains proportional.
3. Dragging, arrows, Home/End, AccessKit increment/decrement/set-value behavior, accessible naming/range/value/step, and the single zoom result remain intact.
4. Other sliders and global styles are unaffected, and no rail recolor is introduced as a side effect of knob fill.
5. The circle, rail, trailing fill, and hit region remain centered and clipped at wide, compact, extreme, and high-display-scale presentations.

## Durable records if authorized

- Revise [`AC-WRITING-006.7`](../../../.forge/requirements/features/screenplay-writing-loop.md:634) to require the circular status knob and status-local rail setting while preserving width, range, trailing fill, accessibility, and zoom semantics. Record a thinner numeric value only after user adjustment.
- Update the zoom presentation paragraph in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:41) with the per-slider shape, scoped rail height, stock-response preservation, and knob/rail color-source boundary.
- Append exact implementation details and evidence limitations to [`ui-icons-v1.md`](../ui-icons-v1.md) and this work order only after implementation.

## User-owned review requests if authorized

- Inspect the knob at 75%, several intermediate values, and 300%; compare idle, hover, drag, and keyboard-focus colors with status icons in every Appearance mode.
- Manually select a 4.8–5.6-point rail height if desired and review the result at 1200, 480, 320, and extreme widths and at available display scales. Confirm width and trailing-fill proportions do not change.
- Drag both directions and use arrows, Home/End, and accessibility increment/decrement/set-value actions. Confirm current/minimum/maximum values and step remain available.
- Recheck percentage presets, native pinch, deep-scroll center anchoring, Save/reopen zoom restoration, and page retention from [`WO-004`](WO-004-document-scoped-visible-page-retention.md).
- Inspect other sliders or slider-like controls for accidental global shape, thickness, or color changes.
- Run any desired workspace build/check/test/lint/formatting commands and record outcomes separately. No agent result is claimed by this proposal.

## WO-006 implementation handoff — 2026-11-04

Implemented on the existing shared branch `dev/2026-11-04-ui-icons` in
[`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs). The named,
commented status-local adjustment is
`presentation::ZOOM_SLIDER_RAIL_HEIGHT` (8.0 logical points), adjacent to the
WO-005 presentation settings. `zoom_slider()` applies it only to a narrow child
Ui through `spacing_mut().slider_rail_height`; the inherited value was not
reduced to the optional 4.8–5.6 points. The slider retains its existing
40–165-point layout allocation and interaction height.

Only this slider selects `HandleShape::Circle`. The stock `Slider` response,
integer range/value mapping and stepping, trailing fill, focus and keyboard
handling, AccessKit naming/range/value/step and actions, and zoom-change result
remain in the existing path. After the stock slider paints, a clipped circle
pass uses the same linear horizontal value position, radius, and interaction
expansion geometry. Its fill and stroke resolve through
`presentation::default_foreground()` or `presentation::hovered_foreground()`,
the same functions driven by `DEFAULT_FOREGROUND_OVERRIDE` and
`HOVERED_FOREGROUND_OVERRIDE`. No knob-specific color literal, widget
background mutation, or separate rail/trailing-fill paint was introduced; the
rail and proportional trailing fill remain stock-painted.

The authorized durable updates are AC-WRITING-006.7, the zoom presentation
paragraph in `technical-context.md`, this work-order handoff, and the WO-006
initiative handoff. WO-004 page retention and all WO-005 presentation/color/
hover behavior remain in place; no unrelated files or predecessor handoffs
were intentionally changed. Source/dependency documentation was read, but no
build, check, test, lint, formatting command, manual verification, review, or
validator was run, per project policy. Compilation, rendered centering/clipping
at endpoints and narrow widths, theme/hover appearance, keyboard and
accessibility actions, and all preserved lifecycle/zoom behavior remain
unverified and pending user-owned validation. No implementation validation or
initiative acceptance is claimed.

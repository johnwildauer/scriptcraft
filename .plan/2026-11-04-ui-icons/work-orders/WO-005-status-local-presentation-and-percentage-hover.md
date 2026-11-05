# WO-005 — Status-local presentation controls and percentage hover

- **Status:** Implementation submitted on 2026-11-04 after WO-004 handoff; user review and validation pending; acceptance not recorded
- **Initiative:** [`ui-icons-v1.md`](../ui-icons-v1.md)
- **Depends on:** [`WO-004`](WO-004-document-scoped-visible-page-retention.md)
- **Validation owner:** User; no agent-run checks or independent reviewers/validators

## Outcome

Introduce status-local, manually adjustable presentation settings for panel horizontal inset, icon hover padding and slot width, inter-item gaps, percentage horizontal reservation, status text/icon sizes, and shared foreground colors. Preserve the current effective numeric values in the implementation handoff so numeric tuning remains a separate user adjustment, not a hidden production change.

The user's clarified color contract is authoritative: default status icon and text foreground must be the same, and both the default foreground and hovered foreground must be easy to change from named, commented status-local settings. Give the current-percentage control the same immediate hover background, shared hovered foreground, corner radius, cursor, and clipped interaction treatment as the three status icons while preserving its click/keyboard popup and accessible behavior.

## Investigation findings and adjustment boundaries

1. The bottom panel uses the framework default frame because [`TopBottomPanel::bottom`](../../../apps/scriptcraft/src/main.rs:2538) supplies no custom frame. [`PanelSizer::new()`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/containers/panel.rs:200) selects [`Frame::side_top_panel()`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/containers/frame.rs:190), whose horizontal inner margin is 8 and vertical inner margin is 2 logical points. There is no app-owned status panel margin constant; reducing it needs a custom status frame.
2. Status icons import [`TOOLBAR_ICON_HOVER_PADDING`](../../../apps/scriptcraft/src/toolbar.rs:18), currently 2 logical points, and use it in [`status_icon_control()`](../../../apps/scriptcraft/src/status_bar.rs:448). Editing that toolbar constant would also change toolbar hover geometry. A status-local value is required before the user halves only status padding.
3. [`status_icon_width()`](../../../apps/scriptcraft/src/status_bar.rs:435) resolves the greater of the interaction width and 36 points. The framework interaction width is 40 at [`Spacing::default()`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:1438), so changing only the 36-point literal cannot narrow the current slot. Reducing the slot needs new status-local sizing code.
4. [`show_impl_with_appearance()`](../../../apps/scriptcraft/src/status_bar.rs:1118) reads horizontal item spacing for allocation and applies the compressed result to rendering at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:1138). The framework default is 8 logical points at [`style.rs`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:1441). One status-local setting must feed both allocation and rendering to prevent drift.
5. [`measured_button_width()`](../../../apps/scriptcraft/src/status_bar.rs:1036) reserves text width plus two horizontal button-padding values even though [`zoom_presets_button()`](../../../apps/scriptcraft/src/status_bar.rs:307) custom-paints the percentage. The inherited default is 4 points per side at [`style.rs`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:1444). A status-local reservation setting is needed; there is no existing percentage-padding constant.
6. The icon glyph size is directly adjustable at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:470), currently 18 logical points. Body and Button text inherit 13 logical points from [`default_text_styles()`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:1397). Their status uses include percentage painting at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:323), percentage measurement at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:1036), page measurement at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:1107), default Body selection at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:526) and [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:927), and Diagnostics sizing at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:1261). There are no app-owned status text-size constants. Global edits would also affect toolbar Body text in [`toolbar_label_width()`](../../../apps/scriptcraft/src/toolbar.rs:168) and [`live_toolbar_metrics()`](../../../apps/scriptcraft/src/toolbar.rs:588).
7. [`zoom_presets_button()`](../../../apps/scriptcraft/src/status_bar.rs:307) allocates a click response and paints only a galley. Unlike [`status_icon_control()`](../../../apps/scriptcraft/src/status_bar.rs:439), it has no hovered rectangle fill, no default-cursor request, and no interaction-state foreground. Its fixed noninteractive color at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:330) explains the muted-gray observation in dark themes: the framework uses gray 140 for noninteractive text and gray 180 for inactive button text at [`style.rs`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:1623), while icons use the interaction state at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:477). The icons' hover is an immediate state repaint, not a separate timed animation.
8. All numeric geometry and font values above are logical points, not guaranteed physical pixels. [`FontId::size`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/epaint-0.34.3/src/text/fonts.rs:27) is a point height, and [`Context::pixels_per_point()`](../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/context.rs:2238) defines physical pixels per logical point. A 1-point inset equals one physical pixel only at 1.0 pixels per point; a literal one-device-pixel policy would require separate conversion code.

## Scope

1. Add one small status-presentation settings surface with named, commented values. Initialize logical-point geometry and font values to the current effective behavior:
   - horizontal panel inset: 8;
   - icon hover padding: 2;
   - icon slot width: 40 under the current framework default;
   - horizontal item gap: 8;
   - percentage horizontal reservation on each side: 4;
   - icon glyph size: 18;
   - Body and Button status text sizes: 13 each.
   Also expose one default/idle foreground setting and one hovered-foreground setting in this same surface. Both must have obvious single-place manual-adjustment locations. Do not change the numeric values during implementation. Record every new setting location in the implementation handoff.
2. Supply a custom frame only to the bottom status panel in [`show_application_panels()`](../../../apps/scriptcraft/src/main.rs:2538), retaining the framework panel fill, vertical inset, stroke/separator behavior, Diagnostics height contract, and all other panels. Do not alter the toolbar's explicit frame in [`chrome_frame()`](../../../apps/scriptcraft/src/main.rs:2468).
3. Decouple status hover padding and slot width from [`TOOLBAR_ICON_HOVER_PADDING`](../../../apps/scriptcraft/src/toolbar.rs:18) and inherited interaction width while leaving toolbar geometry unchanged. Preserve [`toolbar_icon_hover_rect()`](../../../apps/scriptcraft/src/toolbar.rs:618), [`toolbar_group_fill()`](../../../apps/scriptcraft/src/toolbar.rs:629), the 2-point corner radius, and [`toolbar_cursor_icon()`](../../../apps/scriptcraft/src/toolbar.rs:625) as shared semantics.
4. Route the single status item-gap setting through both [`plan_status_row()`](../../../apps/scriptcraft/src/status_bar.rs:147) and the right-side renderer at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:1137). Preserve compression at physically unusable widths, slider growth bounds, fixed priority, clipping, and nonoverlap.
5. Apply status-local Body/Button sizes consistently to measurement and painting. Keep these overrides within status-owned child styles or explicit font helpers; do not mutate global context styles, Appearance visuals, toolbar text, document fonts, or unrelated menus. Popup entries retain their menu presentation unless separately requested.
6. Route every status-owned default/idle text and icon foreground through the shared default setting, including Timer, Focus Mode, Appearance, percentage, page indicator, center operation/error text, and Diagnostics text. Route the hovered foreground of Timer, Focus Mode, Appearance, and percentage through the separate shared hovered setting. Preserve distinct semantic error projection and accessibility content; color unification must not remove error information or make text unreadable in any Appearance mode.
   Theme-derived defaults are acceptable and preferred when needed for System, Day, Night, and Midnight legibility, but the settings surface must provide explicit, documented override points for both default and hovered colors. Do not hard-code an additional color at individual paint call sites. Do not alter toolbar or global Appearance colors.
7. Factor the existing status-icon hover painting into a reusable status-local helper and use it for [`zoom_presets_button()`](../../../apps/scriptcraft/src/status_bar.rs:307):
   - clip the hover rectangle to the allocated and parent clip rectangles;
   - paint [`toolbar_group_fill()`](../../../apps/scriptcraft/src/toolbar.rs:629) only while hovered, with the existing 2-point radius;
   - request [`toolbar_cursor_icon()`](../../../apps/scriptcraft/src/toolbar.rs:625);
   - use the shared hovered foreground setting for both icons and percentage.
   Do not add a timed animation, permanent frame, arrow, or hover-triggered popup opening.
8. Preserve [`interactive_response()`](../../../apps/scriptcraft/src/status_bar.rs:412), the stable popup ID at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:314), exact presets and ordering at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:342), accessible current-value naming, pointer/Enter/Space activation, outside-click/Escape behavior, and shared zoom result. Hover must not steal focus or activate the popup.

## Optional user numeric adjustments after implementation

These are manual follow-up choices, not values applied by this work order:

- Set horizontal panel inset to at most 1 logical point. If the user's “1px” means one physical device pixel rather than one logical point, implement a separate pixels-per-point conversion instead of treating the units as equivalent.
- Halve status icon hover padding from 2 to 1 logical point. Reduce icon slot width and item gaps independently if excess space remains; padding alone does not determine the current 40-point slot.
- Reduce percentage side reservation and status Body/Button/icon sizes by the desired one or two logical points. Re-review measured percentage width and all clips after any change.
- Keep the toolbar's 2-point padding and all global framework defaults unchanged.

## Resolved color contract

The user clarified that default status icon and text color must be the same and that both default and hover color values must be easy to change. Implement shared status-local default and hovered foreground settings rather than independent icon, percentage, page, center, or Diagnostics color literals. Preserve Appearance-aware legibility and provide clear override points if defaults remain theme-derived. Popup-menu entry colors remain menu-owned.

## Non-goals

- No numeric reduction applied by the implementation agent.
- No toolbar, global egui style, document typography, Appearance palette, or popup-menu redesign.
- No page-retention, zoom-state, preset, range, slider-shape, or rail-thickness work; slider appearance belongs to [`WO-006`](WO-006-status-slider-circular-knob-and-rail-setting.md).
- No new automated desktop-test target and no agent execution of any check or validation.

## Structural acceptance for later user review

1. Every numeric and color presentation setting has one status-local manual-adjustment location, and allocation/rendering consume the same resolved values.
2. Initial implementation preserves the recorded current effective numbers; visible behavior changes are limited to percentage hover parity and unified status text/icon foreground sources.
3. Default status text and icons use the shared default foreground, and percentage hover matches status-icon hover fill, shared hovered foreground, radius, cursor, and clipping without opening the popup.
4. Both default and hovered colors remain legible across System, Day, Night, and Midnight modes, with documented single-place overrides; toolbar and popup-menu colors remain unchanged.
5. Toolbar hover and typography, document typography, other sliders, global Appearance, and unrelated UI remain unchanged.
6. Popup identity, exact presets, pointer/keyboard activation, accessibility, zoom anchoring/persistence, panel height, and narrow-width nonoverlap remain intact.

## Durable records if authorized

- Revise [`AC-WRITING-006.1`](../../../.forge/requirements/features/screenplay-writing-loop.md:569) so status and toolbar share hover semantics without requiring one shared numeric padding constant; include percentage hover, shared default/hovered status text-and-icon foreground settings, and preserved accessible behavior. Do not claim a reduced numeric value until the user selects it.
- Update [`AC-WRITING-006.4`](../../../.forge/requirements/features/screenplay-writing-loop.md:608) only to identify status-local spacing/inset ownership while retaining priority, slider width, clipping, and nonoverlap.
- Update the icon/status paragraph in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:34) with the local geometry, typography, default/hovered foreground settings boundary and percentage hover behavior.
- Append actual setting locations and implementation evidence limitations to [`ui-icons-v1.md`](../ui-icons-v1.md) and this work order only after implementation.

## User-owned review requests if authorized

- Inspect source to choose and adjust the new numeric and color settings manually; record each selected value. Verify that no agent-applied numeric reduction is claimed and that default/hovered foreground each has one obvious override location.
- Compare Timer, Focus Mode, Appearance, percentage, page, center, and Diagnostics default text/icon colors and all four interactive hover states side by side in System, Day, Night, and Midnight modes. Confirm no hover animation timing is invented and that text/icons remain legible.
- Open the percentage by pointer, Enter, and Space; select every preset, dismiss with outside click and Escape, and verify no hover-only opening, focus theft, clipping, or ID conflict.
- Review wide, former-breakpoint, 480, 320, and extreme widths after any manual numeric tuning; check right-edge inset, slot/group gaps, center/page clipping, hit regions, and Diagnostics height.
- Check that toolbar icons/text and popup-menu typography did not change. Review logical-point versus physical-pixel appearance at available display scales.
- Run any desired workspace build/check/test/lint/formatting commands and record outcomes separately. No agent result is claimed by this proposal.

## WO-005 implementation handoff — 2026-11-04

Implemented WO-005 on the existing shared branch `dev/2026-11-04-ui-icons`.
The status-local settings surface is `presentation` in
[`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs). Its exact
manual-adjustment locations and preserved current values are:

| Setting | Exact source location | Initial value |
|---|---|---:|
| Bottom status panel horizontal inset | `presentation::PANEL_HORIZONTAL_INSET` | 8 points |
| Status icon hover padding | `presentation::ICON_HOVER_PADDING` | 2 points |
| Status icon slot width | `presentation::ICON_SLOT_WIDTH` | 40 points |
| Right-group item gap | `presentation::ITEM_GAP` | 8 points |
| Percentage side reservation | `presentation::PERCENTAGE_SIDE_RESERVATION` | 4 points per side |
| Status Body text | `presentation::BODY_TEXT_SIZE` | 13 points |
| Status Button text | `presentation::BUTTON_TEXT_SIZE` | 13 points |
| Material icon glyph | `presentation::ICON_GLYPH_SIZE` | 14 points |
| Default/idle text and icon foreground override | `presentation::DEFAULT_FOREGROUND_OVERRIDE` | `None` (theme-derived) |
| Hovered interactive-status foreground override | `presentation::HOVERED_FOREGROUND_OVERRIDE` | `None` (theme-derived) |

The color override options are the single adjustment points: setting either to
`Some(Color32::from_rgb(r, g, b))` selects an explicit color, while `None` follows
the active Appearance theme. Default text and icon paint uses the active theme's
text foreground; hovered Timer, Focus Mode, Appearance, and percentage use its
hovered-widget text foreground. All status-owned page, center operation/error,
and Diagnostics text uses the same default source. The Diagnostics scope also
sets the local text override so its heading follows an explicitly selected
default color. No toolbar or global Appearance color was changed, and popup
entries remain menu-owned.

The bottom panel now receives `status_panel_frame()` from `status_bar.rs`, which
starts with the framework side/top-panel frame and changes only its horizontal
inset. `show_application_panels()` applies it only to the bottom status panel;
the initial 8-point inset, 2-point vertical inset, panel fill, separator/stroke
behavior, and other panels remain unchanged. The status icon hover padding and
fixed 40-point slot no longer depend on toolbar padding or inherited interaction
width. The item-gap setting feeds both row allocation and rendering. Body and
Button text settings are applied only in status-owned text scopes and explicit
measurements; popup-menu entries keep their existing menu presentation.

Icon glyph size is initialized to 14 points, matching the existing source's
`icon.rich_text().size(14.0)` and preserving its current effective behavior. The
earlier investigation table's 18-point figure does not match that source; no
change to 18 was made. No numeric reduction was applied. Percentage now shares
the status icon hover painter: clipped interaction background, existing
two-point radius, toolbar default cursor, shared hovered foreground, and
immediate state repaint without timed animation or hover activation. Its stable
popup ID, accessible current-value label, pointer/Enter/Space activation,
outside-click/Escape dismissal, exact preset list/order, and zoom result route
were not intentionally altered. Status actions, Focus Mode gating, page/center
layout, slider bounds/shape/rail, and WO-004 page behavior were not intentionally
changed. No WO-006 knob painting or rail setting was implemented.

The scoped durable updates are recorded in AC-WRITING-006.1 and AC-WRITING-006.4,
the status paragraph of `technical-context.md`, and the initiative handoff. The
WO-005 inspection finding was accurate at that point in the shared source: the
tracker lacked the retained field and reset method even though the New/Open calls
were present. The narrowly scoped WO-004 reconciliation recorded in its handoff
and the initiative log has since added the missing tracker state/reset and fallback
behavior. The New/Open call sites already followed successful replacement and
required no edits; the same-frame zoom Unavailable override was already absent.
This source repair does not constitute validation; page-retention compilation and
behavior remain user-owned and unverified.

No build, check, test, lint, formatting command, manual verification,
implementation review, or independent validation was run. All review and
validation remain user-owned. Compilation, color legibility, hover behavior,
popup/accessibility behavior, responsive layout, and page-retention behavior are
unverified; no acceptance or initiative closeout is claimed.

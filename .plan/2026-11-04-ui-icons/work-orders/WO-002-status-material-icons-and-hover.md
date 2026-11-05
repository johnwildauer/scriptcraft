# WO-002 — Status Material icons and hover parity

- **Status:** Implementation submitted 2026-11-04; user review/validation pending
- **Initiative:** [`ui-icons-v1.md`](../ui-icons-v1.md)
- **Depends on:** [`WO-001`](WO-001-egui-034-material-icons-foundation.md)
- **Validation owner:** User; no agent-run checks or independent reviewers/validators

## Outcome

Replace the inline Sprint Timer, Focus Mode, and Appearance presentations with their assigned Material Symbols and no visible button text. Match the toolbar's icon hover treatment exactly while preserving every existing action, popup, enabled state, accessible name, and keyboard activation path.

The legacy More path may remain during this intermediate order; WO-003 removes it and its now-obsolete measurement/planning machinery.

## Icon assignments

| Status action | Material Symbol | Variant | Visible closed-control content |
|---|---|---|---|
| Sprint Timer | Timer | Outlined | Icon only |
| Focus Mode | Location Searching | Outlined | Icon only |
| Appearance | Mode Night | Filled | Icon only |

Use the 0.6.0 crate's typed icon values and rich-text/font-family conversion. Do not use the former Unicode stopwatch/fullscreen characters, ordinary proportional-font lookalikes, image assets, or hand-drawn glyphs.

## Scope

1. Add one reusable status-icon control helper in [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:304) rather than separately approximating hover geometry in three branches. It should accept the typed Material icon, stable ID, enabled state, and accessible name; return the response/activation and rectangle needed by existing routing.
2. Match toolbar hover semantics by reusing or directly sharing:
   - [`TOOLBAR_ICON_HOVER_PADDING`](../../../apps/scriptcraft/src/toolbar.rs:18), currently two points;
   - [`toolbar_icon_hover_rect()`](../../../apps/scriptcraft/src/toolbar.rs:621);
   - [`toolbar_group_fill()`](../../../apps/scriptcraft/src/toolbar.rs:632), which uses the hovered widget background and is transparent otherwise;
   - the toolbar's two-point corner radius and default cursor from [`toolbar_action_button()`](../../../apps/scriptcraft/src/toolbar.rs:343).
   Keep icon glyphs centered in the existing one-row interaction height. Do not add a permanent button frame, border, arrow, or visible caption.
3. Replace the inline timer glyph in [`render_right_item()`](../../../apps/scriptcraft/src/status_bar.rs:970) with outlined Timer. Preserve the current toggle behavior, just-opened flag, hover text, accessible name, returned timer rectangle, and popup anchoring through [`show_application_panels()`](../../../apps/scriptcraft/src/main.rs:2532).
4. Replace inline Focus Mode text with outlined Location Searching. Continue using [`FocusModeView::enabled()`](../../../apps/scriptcraft/src/focus_mode.rs:32) and [`FocusModeView::accessible_name()`](../../../apps/scriptcraft/src/focus_mode.rs:43) verbatim for interaction gating, tooltip, and accessibility. Activation must still flow through the existing pending/fullscreen request logic; the glyph does not encode or invent a different focus state.
5. Replace the closed Appearance text menu button with filled Mode Night and no arrow. Opening it must display exactly System, Day Mode, Night Mode, and Midnight Mode using the existing labels and selection behavior in [`appearance_menu()`](../../../apps/scriptcraft/src/status_bar.rs:933). Text inside the open menu remains; only the closed status button is icon-only. Preserve appearance-change reporting and repaint behavior.
6. Retain [`interactive_response()`](../../../apps/scriptcraft/src/status_bar.rs:367) semantics or an equivalent explicit implementation: stable button role/name, pointer click, Enter/Space activation, truthful enabled state, and no focus theft during ordinary painting.
7. Update intrinsic icon measurements to use the actual helper's interaction/hover bounds rather than the old text-button widths. Full right-group allocation is WO-003's responsibility; this order only prevents stale text measurements from being required by icon rendering.

## Non-goals

- No final right-group space-priority algorithm and no More removal in this order.
- No zoom slider, percentage, preset, arrow, or zoom-state changes.
- No dynamic Appearance glyph based on the selected theme and no dynamic Focus Mode glyph unless separately authorized.
- No toolbar visual redesign or glyph reassignment. Sharing hover helpers must not alter toolbar geometry or rendering.
- No timer/focus/appearance behavior or persistence changes.

## Structural acceptance

- All three closed inline controls contain only their specified Material glyph; no Sprint Timer, Focus Mode, or Appearance words and no legacy Unicode symbols remain in those closed controls.
- Timer and Focus Mode select outlined font families; Appearance selects the filled family. All render with the crate's accepted Regular weight.
- Idle controls have transparent backgrounds. Hovered controls use the same rectangle padding, fill source, corner radius, and cursor as toolbar icon hover.
- Timer popup opens above the actual new icon rectangle and retains all current timer state behavior.
- Appearance opens the same four-choice menu and reports changes exactly once through the existing path.
- Focus Mode remains disabled when required and exposes the same state-specific accessible descriptions; pointer and Enter/Space activation route to the existing controller without implicit inverse requests.
- No right-control geometry is derived from the removed visible captions.

## Durable records

- Update [`AC-WRITING-006.1`](../../../.forge/requirements/features/screenplay-writing-loop.md:569), and only closely related wording in 006.11–006.13 if necessary, to require the assigned icon-only closed controls, accessible names/tooltips, and toolbar-equivalent hover. Retain all action and state semantics.
- Extend the icon-foundation paragraph in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:20) with the status icon/variant assignments and shared hover contract. Do not describe overflow removal before WO-003 implements it.
- Record exact implementation changes and evidence limitations in [`ui-icons-v1.md`](../ui-icons-v1.md) and this work order. Keep user validation and acceptance unclaimed until supplied.

## User-owned review requests

- Compare status and toolbar icon hover side by side in System, Day, Night, and Midnight appearances.
- Activate every status icon by pointer and keyboard; tab through the controls and verify accessible names/tooltips remain meaningful without visible captions.
- Exercise all Appearance menu choices and verify the closed icon remains Mode Night.
- Open, dismiss, and reopen Sprint Timer from different window widths; confirm its popup remains anchored to the icon and timer state is unchanged by presentation edits.
- Review Focus Mode while windowed, fullscreen, pending, externally changing, unresolved, and unavailable; confirm the icon never bypasses the existing safety gates.
- Run desired workspace compilation/check commands and record outcomes under user-owned validation; no agent result is claimed.

## Implementation handoff — 2026-11-04

Implemented on `dev/2026-11-04-ui-icons` only. `status_bar.rs` now uses a
reusable status-icon control for inline Sprint Timer (outlined Timer), Focus
Mode (outlined Location Searching), and Appearance (filled Mode Night). It uses
the typed `MaterialIcon` values and rich-text font-family conversion. The
controls show no closed-state captions; Appearance has no closed-control arrow.
Their helper uses the toolbar's two-point icon hover padding and hover-rectangle
helper, hovered-widget background, two-point corner radius, and default cursor.
Status icon widths are derived from the fixed helper interaction bounds, not
the former label widths.

Timer's accessible name, tooltip, activation and just-opened routing are
preserved, and its returned interaction rectangle remains the popup anchor.
Focus Mode uses `FocusModeView::enabled()` and `accessible_name()` for the same
gating and detailed state-specific accessibility projection; activation still
sets the existing controller request flag. Appearance's icon button owns the
popup anchor and keeps all four existing choices, menu labels, selection
behavior, and change reporting. Button semantics preserve pointer/Enter/Space
activation without requesting focus during ordinary painting.

The existing More/overflow path remains for this intermediate order. WO-003's
priority algorithm, overflow removal, and zoom redesign were not implemented;
the toolbar and timer/focus/appearance behavior or persistence were not
redesigned. No build, check, test, lint, formatting command, manual verification,
agent review, or validation was run. The WO-001 migration and this UI behavior
remain unverified; user review/validation are pending, WO-003 is not started,
and no acceptance is claimed.

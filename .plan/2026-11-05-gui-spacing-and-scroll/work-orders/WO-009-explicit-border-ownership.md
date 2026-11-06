# WO-009 — Explicit toolbar, page, and status border ownership

- **Status:** Implementation submitted for user validation; no validation or acceptance recorded
- **Initiative:** [`boundary-and-border-investigation.md`](../boundary-and-border-investigation.md)
- **Depends on:** [`WO-008`](WO-008-toolbar-trailing-allocation.md) under sequential execution; border changes are otherwise independent
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or independent validators

## Outcome

Give toolbar, screenplay page, and status borders explicit local width ownership, and make the visible status top divider consume the status palette instead of egui's unrelated generic separator color.

Preserve the current user-adjusted toolbar and page widths. Replace only the status border mechanism, retaining its existing one-edge visual role rather than adding a complete box around the status panel.

## Pre-implementation findings

- The toolbar explicitly paints a 1.5-point bottom line from the toolbar palette at [`Stroke::new()`](../../../apps/scriptcraft/src/toolbar.rs:934).
- The page explicitly paints a nominal 3-point centered stroke from the page palette at [`paint_page()`](../../../apps/scriptcraft/src/script_view.rs:185), clipped by its page painter.
- [`status_panel_frame()`](../../../apps/scriptcraft/src/status_bar.rs:84) applies the status palette color to an inherited zero-width stroke, so that color has no visible effect.
- [`Panel::bottom()`](../../../apps/scriptcraft/src/main.rs:1115) retains egui's default separator, which independently paints a generic noninteractive 1-point line above the status panel.

## Scope

1. Add [`TOOLBAR_BOTTOM_BORDER_WIDTH`](../../../apps/scriptcraft/src/toolbar.rs:19), initialized to the current 1.5 logical points, and use it for the existing bottom divider. Preserve its current palette color, position, and full-width geometry.
2. Add [`PAGE_BORDER_WIDTH`](../../../apps/scriptcraft/src/script_view.rs:18), initialized to the current nominal 3 logical points, and use it for the existing page stroke. Preserve page palette color, corner radius, [`StrokeKind::Middle`](../../../apps/scriptcraft/src/script_view.rs:210), and painter clipping.
3. Add a status-local [`STATUS_PANEL_TOP_BORDER_WIDTH`](../../../apps/scriptcraft/src/status_bar.rs:24), initialized to 1 logical point to match the current generic separator's effective width.
4. Disable the bottom status panel's framework separator through [`show_separator_line(false)`](../../../apps/scriptcraft/src/main.rs:1115), mirroring the toolbar's explicit separator ownership.
5. Make the status frame stroke-free rather than inheriting a zero-width stroke. Do not enable a full frame stroke that would introduce new left, right, or bottom borders.
6. Paint one explicit full-width status top divider from [`StatusPalette::border`](../../../apps/scriptcraft/src/theme.rs:77) using the new status width setting. Anchor it to the actual outer status-panel rectangle returned by [`Panel::show_inside()`](../../../apps/scriptcraft/src/main.rs:1118), not the margin-inset content rectangle.
7. Keep the status divider on the ordinary panel paint path and inside the panel boundary. Preserve panel height, insets, controls, center/page/slider geometry, Diagnostics behavior, popups, and accessibility.
8. Do not change the global noninteractive widget stroke or shared control palette to accomplish the status fix.
9. Reconcile the durable border contracts and record the implementation handoff in this draft and the investigation note.

## Non-goals

- No intentional numeric change to the current toolbar or page border widths.
- No full status-panel box border and no additional separator above Script View.
- No page selection-boundary, underline, caret, or text stroke changes.
- No theme-palette restructuring or global egui visual changes.
- No screenplay-core, persistence, pagination, or export changes.
- No agent-run validation.

## Structural acceptance for user review

1. Toolbar and page widths have named local owners preserving their current values.
2. The status framework separator is disabled and the inherited zero-width frame stroke is not used as a substitute.
3. One explicit top-edge status divider uses the status palette and a named local width.
4. No additional status side/bottom borders or global noninteractive color changes are introduced.
5. Status allocation and all unrelated painting and interaction behavior remain unchanged.

## Durable records

- Update the toolbar, page, and status boundary requirements in [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:534) to identify independent semantic border ownership and the single explicit status top divider.
- Update [`technical-context.md`](../../../.forge/blueprints/technical-context.md:24) to distinguish explicit local stroke widths/colors from framework separators and to record the disabled status separator path.
- Keep durable records free of work-order identifiers, implementation chronology, and numeric border widths. Exact values and handoff evidence belong only in this planning directory.

## User-owned review requests

- Set the status border to black or another high-contrast test color and confirm the visible top divider changes accordingly.
- Confirm no second light generic separator remains between Script View and the status bar.
- Test distinct toolbar, page, and status border colors and verify that each changes only its owned surface.
- Inspect the boundaries at multiple native display scales and page zoom levels, including page-edge clipping.
- Confirm toolbar captions, status sizing/controls, Diagnostics, Script View scrolling, and popup layering remain intact.
- Run any desired workspace checks and record outcomes separately; no agent result is claimed.

## Implementation-submission handoff

- **Branch:** `dev/2026-11-05-gui-spacing-and-scroll`; the existing checkout and prior work, including WO-008 and user-owned source adjustments, were preserved. No branch or history operations were performed.
- **Exact source changes and values:** Added [`TOOLBAR_BOTTOM_BORDER_WIDTH`](../../../apps/scriptcraft/src/toolbar.rs:19) at `1.5` logical points and used it for the existing toolbar bottom divider without changing its palette color or position. Added [`PAGE_BORDER_WIDTH`](../../../apps/scriptcraft/src/script_view.rs:19) at `3.0` logical points and used it for the page's existing [`StrokeKind::Middle`](../../../apps/scriptcraft/src/script_view.rs:211) stroke through the page-clipped painter. Added [`STATUS_PANEL_TOP_BORDER_WIDTH`](../../../apps/scriptcraft/src/status_bar.rs:26) at `1.0` logical point, matching the former generic separator's effective width. The bottom status panel disables its framework separator; its frame uses [`Stroke::NONE`](../../../apps/scriptcraft/src/status_bar.rs:93). The explicit full-width status top divider uses the status palette border color, is clipped to the panel rectangle, and is painted at the outer panel's top edge from the rectangle returned by [`Panel::show_inside()`](../../../apps/scriptcraft/src/main.rs:1119), not the inset content rectangle.
- **Ownership/result:** Toolbar, screenplay-page, and status borders now have named local width owners and their existing semantic palette colors. Only one status top divider is painted; no status side/bottom frame stroke or global noninteractive color change was introduced. The existing toolbar/page widths, page clipping and stroke kind, and unrelated panel and content geometry were preserved.
- **Files changed for this submission:** [`toolbar.rs`](../../../apps/scriptcraft/src/toolbar.rs:19), [`script_view.rs`](../../../apps/scriptcraft/src/script_view.rs:18), [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:25), [`main.rs`](../../../apps/scriptcraft/src/main.rs:1115), [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:612), [`technical-context.md`](../../../.forge/blueprints/technical-context.md:26), this draft, and [`boundary-and-border-investigation.md`](../boundary-and-border-investigation.md:1). The theme palette implementation was not changed.
- **Evidence limitations:** No build, check, test, lint, formatting command, manual verification, reviewer, validator, or user validation was run or claimed. This is an implementation submission only; user validation and acceptance remain outstanding.

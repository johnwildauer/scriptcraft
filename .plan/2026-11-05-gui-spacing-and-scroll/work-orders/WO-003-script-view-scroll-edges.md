# WO-003 — Scrollable page-edge padding and no Script View fades

- **Status:** Implementation submitted for user review; not user-validated or accepted
- **Initiative:** [`gui-spacing-and-scroll.md`](../gui-spacing-and-scroll.md)
- **Depends on:** [`WO-002`](WO-002-script-view-extraction.md)
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or independent validators

## Outcome

Give the first and last screenplay pages deliberate breathing room at the scroll content's extremes without allowing fixed frame bands or fuzzy gradients to cover page content during ordinary scrolling.

The desired blank space must move with the scroll content. It is not a permanent viewport inset, a rubber-band scrolling effect, or a page-layout dimension.

## Current source findings

- [`central_panel_frame()`](../../../apps/scriptcraft/src/main.rs:2491) inherits egui's 8-point central-panel margin on every side. That fixed margin creates the initial gap and remains at the viewport edges while scrolling.
- The installed egui [`ScrollFadeStyle`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:792) defaults to 20-point gradients at 0.5 strength. [`paint_fade_areas()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/containers/scroll_area.rs:1531) paints these over content near overflow edges.
- [`ScrollArea::both()`](../../../apps/scriptcraft/src/main.rs:1414) does not override those fades, so they affect both vertical and horizontal overflow.
- [`PAGE_VERTICAL_GAP`](../../../apps/scriptcraft/src/main.rs:42) owns inter-page spacing, while [`PAGE_HORIZONTAL_PADDING`](../../../apps/scriptcraft/src/main.rs:41) owns the horizontal exterior page buffer. Neither is a document-end padding setting.
- [`prepaint_zoom_scroll_offset()`](../../../apps/scriptcraft/src/main.rs:2101) predicts content extent and page origins for same-frame zoom compensation. Any new content margin must be represented there as well as in actual allocation.

## Scope

1. In the Script View module established by the preceding extraction, replace the inherited central-panel frame with a margin-free frame retaining the active Script Workspace background fill.
2. Add [`SCRIPT_VIEW_PAGE_EDGE_PADDING`](../../../apps/scriptcraft/src/script_view.rs:1), initialized to 16 logical points as an agent-default pending user visual review.
3. Apply that setting as symmetric top/bottom content margin on the existing page scroll area. Keep left/right content margin at zero and preserve existing horizontal page centering and exterior side buffers.
4. Scope Script View's scroll fade strength to zero before scroll-area construction and paint. Disable the effect on every axis for this surface only; do not mutate global scroll styling or unrelated popup scroll areas.
5. Update vertical page-origin and content-extent calculations so the top margin precedes the first page and both vertical margins contribute exactly once to total scrollable height.
6. Preserve the existing page gap and framework row-spacing contribution coherently. Do not double-add content margins, inter-page gaps, or item spacing, and do not silently change the current inter-page stride.
7. Share or exactly reconcile the vertical geometry between actual allocation and prepaint zoom compensation. Page-local document coordinates scale with zoom; content margins, page gaps, outside-page displacement, and horizontal padding remain fixed logical-point quantities.
8. Preserve actual page-rectangle publication, true scroll clipping, visible-page selection, pointer routing, caret reveal, manual-scroll cancellation, one-shot zoom anchoring, and viewport culling/overscan.
9. Update the durable editor geometry contract and record the implementation handoff in the initiative records.

## Non-goals

- No page zoom limit, page size, horizontal buffer, inter-page gap, or visible-page rule changes.
- No elastic/rubber-band scrolling, scroll snapping, automatic centering, or new scroll persistence.
- No global disabling of egui fades and no changes to Elements or other popup scroll areas.
- No screenplay-core, fixed document layout, pagination, FDX, native save, PDF, or plain-text changes.
- No new automated desktop-test target and no agent execution of validation.

## Structural acceptance for user review

1. The Script Workspace frame has no fixed inherited margin; its semantic background remains intact.
2. One named vertical content-margin setting controls blank space before the first page and after the last page.
3. That blank space participates in scroll extent and moves with content rather than covering the viewport during ordinary scrolling.
4. Script View paints no top, bottom, left, or right scroll fade gradients; unrelated scroll areas retain their framework behavior.
5. Actual allocation and prepaint zoom geometry agree about first-page origin, page stride, total content extent, and clamping.
6. Existing page publication, status tracking, input, caret reveal, culling, and zoom-anchor contracts remain intact.

## Durable records

- Update the desktop editor geometry requirements in [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:137) to state that page-end breathing room belongs to scrollable content, viewport edges remain unobscured, and fixed presentation spacing does not alter document layout.
- Update the Script View and zoom contracts in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:177) to identify content-margin ownership, surface-local fade policy, and the requirement that actual allocation and prepaint compensation share equivalent geometry.
- Describe only the enduring repository behavior and ownership. Do not include work-order identifiers, implementation chronology, or numeric padding, margin, fade, or gap values in these durable records.
- Keep exact source values, implementation notes, and evidence limitations in this draft and the initiative overview rather than the requirements or blueprints.

## User-owned review requests

- At the document top, confirm the intended blank space precedes page one. Scroll down and confirm it scrolls away instead of remaining over the page.
- In the middle of a multi-page document, confirm pages can reach the top and bottom viewport edges without fuzzy overlays or fixed frame blocks.
- At the document bottom, confirm the intended allowance lies beyond the last page and no band covers that page while approaching or reaching the end.
- Repeat with horizontal overflow and confirm no left/right fades obscure page content.
- Repeat at the zoom limits and ordinary zoom, through continuous native pinch and status slider/presets, at beginning/middle/end positions. Confirm viewport anchoring and clamping remain stable.
- Check short documents, caret reveal, window resizing, page status, pointer selection, and every Appearance.
- Confirm Elements and unrelated popup scrolling retain their prior behavior.
- Run any desired workspace compilation and behavior checks and record outcomes separately; no agent result is claimed.

## Implementation-submission handoff

- **Branch:** `dev/2026-11-05-gui-spacing-and-scroll`. Existing checkout changes and the WO-001/WO-002 submissions were preserved; nothing was staged, committed, stashed, discarded, merged, or rebased.
- **Exact source changes:** [`script_view.rs`](../../../apps/scriptcraft/src/script_view.rs) adds `SCRIPT_VIEW_PAGE_EDGE_PADDING = 16.0` logical points and applies it as symmetric vertical scroll-content margin with zero horizontal margin. Immediately around the Script View scroll area, the current `Ui` style's fade strength is temporarily set to `0.0` and restored afterward; the shared context style and scroll-area identity are unchanged. The Script Workspace frame now uses a margin-free `Frame::NONE` while retaining the active background fill. Prepaint zoom geometry adds the top edge allowance to each page origin and both edge allowances once to predicted vertical content extent; the existing 12-point page gap and egui row-spacing contribution remain part of the page stride, and page-local document coordinates alone scale with zoom.
- **Durable records:** [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md) now specifies scroll-content-owned edge breathing room, unobscured viewport edges, isolated fade policy, presentation-only spacing, and equivalent actual/prepaint geometry. [`technical-context.md`](../../../.forge/blueprints/technical-context.md) now records the lasting Script View ownership and zoom geometry contract without tuning values or chronology.
- **Files changed for this submission:** [`apps/scriptcraft/src/script_view.rs`](../../../apps/scriptcraft/src/script_view.rs), [`.forge/requirements/features/screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md), [`.forge/blueprints/technical-context.md`](../../../.forge/blueprints/technical-context.md), and this draft. The WO-001 and WO-002 handoffs remain intact; WO-004 was not implemented.
- **Evidence limitations:** No build, check, test, lint, formatting command, manual verification, independent review, or validator was run. Compilation, framework/API compatibility, actual scroll extent and clipping, visual appearance, fades, zoom anchoring, input behavior, and cross-platform behavior remain unverified and user-owned. This is an implementation submission only; no user validation or acceptance is claimed.

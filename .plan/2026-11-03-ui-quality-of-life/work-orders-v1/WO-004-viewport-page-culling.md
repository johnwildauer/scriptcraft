# RS-WO-004 — Viewport-bounded page and caret painting

- **Status:** Implemented 2026-11-03; user review and validation pending; not user-validated
- **Initiative:** [`ui-quality-of-life-v1-render-speed.md`](../ui-quality-of-life-v1-render-speed.md)
- **Depends on:** [`RS-WO-003`](WO-003-same-frame-input.md)
- **Validation owner:** User; no agent-run checks or independent validators

## Objective

Allocate geometry for every page but paint only the viewport and a small fixed overscan region, and resolve the caret once per frame instead of once per page.

This order retains the existing per-character text adapter; RS-WO-005 replaces that separately.

## Implementation steps

1. Split page allocation from content painting in [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:887) and [`paint_page()`](../../../apps/scriptcraft/src/main.rs:737).
2. Continue allocating every page row and collecting every page rectangle for scroll extent, visible-page status, pointer routing, and caret reveal.
3. Paint only page rectangles intersecting the actual editor clip plus one page of vertical overscan above and below. Do not use an overscan proportional to document length.
4. Skip line iteration, selection painting, text layout, underlines, and page fills for hidden pages.
5. Resolve collapsed caret geometry once per frame through [`ScreenplayLayout::caret_geometry()`](../../../crates/scriptcraft-core/src/layout.rs:488), then paint the caret and IME preedit only on their owning visible page.
6. Keep title-page ownership, page selection, horizontal centering/overflow, visible-page status, and conditional reveal semantics unchanged.
7. Feed RS-WO-001 counters with considered, culled, and painted pages plus caret lookups.

## Structural acceptance

- Painted pages are limited to visible pages plus fixed overscan, independent of total document pages.
- Hidden pages issue no text-layout or line-paint work.
- Each frame performs at most one global caret geometry lookup before RS-WO-007 introduces indexes.
- Diagnostics distinguish pages considered, culled, and painted; line submissions cover painted pages only.
- All page rectangles and total page count remain available and identical to the unculled path.
- Scrolling preserves the same whole-layout object and causes no layout build.

## Regression coverage to author

Cover first/middle/last viewports, partial pages, one-page overscan boundaries, title pages, horizontal overflow, fractional zoom, offscreen caret/IME, caret reveal, pointer routing, and status page selection. Compare complete page-rectangle vectors with the pre-change path.

## Durable documentation

Add viewport-bounded desktop painting to [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:514) as a structural requirement without timing promises, and document allocation-versus-paint ownership in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:22).

## User validation

Scroll through a 120-page document while observing diagnostics and page status. Inspect first/last pages, title-page handling, zoom presets, horizontal overflow, caret reveal, IME placement, and pointer selection. Confirm offscreen paint counters remain flat.

## Upstream handoff from RS-WO-003 — 2026-11-03

`ScriptCraftApp::update()` is now the shared production/test scheduling boundary: it dispatches accepted surface input using the prior frame's focus state before `show_application_panels()` performs current-frame layout and paint. The test helper that drives full application frames uses that same method. RS-WO-004 should leave this input-before-layout/paint contract intact and keep migration-modal dispatch exclusive. Existing all-page layout/page rectangle preparation and current paint counters remain the baseline to change for this order; continue allocating complete page geometry for scrolling and pointer routing while culling only text/page paint. Same-frame, migration-modal, and prior WO-001/WO-002 authored tests are unexecuted, and user review/validation of all three orders remains pending. User baseline measurements are still absent; WO-003 had no measurement hard gate and no timing comparison is claimed.

## Handoff

## Implementation record — 2026-11-03

`live_document_surface()` now allocates every page row and records the complete rectangle vector before selecting the paint range. It paints only the pages intersecting the actual editor clip and, when there is a visible page, at most one allocated page immediately before and after that visible range. Full page geometry remains the basis for scroll extent, visible-page status, pointer routing, conditional reveal, and page dimensions. Painting (page fills, lines, selection, underlines, text, caret, and IME preedit) is skipped for culled pages. Caret geometry is resolved at most once when the caret is on a painted page or conditional reveal requires it, then shared by reveal and owner-page painting; an offscreen caret without reveal does not trigger a lookup. Title-page ownership remains separate from body caret rendering.

Added `pages-culled` to the existing value-free performance diagnostics alongside considered and painted page counts. The desktop oracle now expects viewport-bounded painting and counts lines only for painted pages; its full page-rectangle vector assertions preserve the previous vertical allocation flow. Authored regression cases cover first/middle/last visible ranges, partial-page intersection, exact one-page overscan boundaries, horizontal clip exclusion, title-page geometry/status totals, offscreen caret/IME behavior, 175% fractional presentation zoom, scroll positions, pointer selection, conditional reveal, and page rectangle completeness. The existing same-frame application update and migration-modal coverage remains unchanged. No galleys, wrap-cache reuse, or interaction indexes were introduced.

The implementation and authored regression coverage have not been run or independently checked. User review and validation remain pending under repository policy. User baseline and after measurements remain pending; no timing comparison, threshold, or measured speedup is claimed.

## Handoff

RS-WO-005 reduces the remaining visible-line cost by replacing per-character painter requests with fixed-cell line/span galleys. It must retain viewport culling and the allocation/paint split: all page rectangles continue to exist, while text requests remain confined to the visible range plus fixed one-page overscan. WO-004 deliberately keeps the existing per-character painter path, whole-layout geometry, and linear owner-page/caret search pending later orders. Same-frame input and conditional reveal behavior from RS-WO-003 remain unchanged.

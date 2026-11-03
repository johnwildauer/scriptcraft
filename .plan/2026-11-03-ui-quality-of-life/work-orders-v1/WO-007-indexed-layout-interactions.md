# RS-WO-007 — Indexed caret, hit-test, and vertical navigation

- **Status:** Implemented 2026-11-03; user review and validation pending; not user-validated
- **Initiative:** [`ui-quality-of-life-v1-render-speed.md`](../ui-quality-of-life-v1-render-speed.md)
- **Depends on:** [`RS-WO-006`](WO-006-per-element-wrap-cache.md)
- **Authorization:** Covered by the user's 2026-11-03 authorization for sequential execution of all twelve orders; authorization is not review, validation, or acceptance
- **Validation owner:** User; no agent-run checks or independent validators

## Objective

Build layout-owned indexes for element-to-line and ordered document-line lookup, eliminating repeated full-document scans in caret, pointer, and vertical-navigation paths.

This order does not change caret placement, nearest-line tie-breaking, selection semantics, or page geometry.

## Implementation steps

1. Add an element-to-placed-lines index to [`ScreenplayLayout`](../../../crates/scriptcraft-core/src/layout.rs:140), using current semantic element identity/index consistently with layout lines.
2. Add an ordered global body-line index with page, line, fixed vertical position, and source-range metadata.
3. Rewrite [`ScreenplayLayout::caret_geometry()`](../../../crates/scriptcraft-core/src/layout.rs:488) to inspect only lines belonging to the requested element and page placement.
4. Rewrite [`ScreenplayLayout::nearest_line_hit_test_fixed()`](../../../crates/scriptcraft-core/src/layout.rs:434) to locate the nearest vertical candidates through the ordered index rather than scanning every line.
5. Preserve exact document-order tie-breaking, title-page exclusion, page-local x mapping, margin clamping, and Unicode display/source mapping.
6. Update [`ScriptCraftApp::move_vertical()`](../../../apps/scriptcraft/src/main.rs:1273) to query current and adjacent lines directly without collecting a flattened vector.
7. Keep indexes synchronized with every fresh or cache-reused complete layout and exclude stale indexes on document replacement.

## Structural acceptance

- Caret lookup does not scan unrelated elements or every page.
- Nearest-line lookup examines only the necessary ordered candidates rather than the complete document.
- Vertical navigation allocates no flattened whole-document line vector.
- Indexed results exactly match the previous linear implementation for valid inputs, including ties and page boundaries.
- RS-WO-004 still performs at most one caret lookup per painted frame, now indexed.

## Regression coverage to author

Create differential tests against simple linear reference implementations for all roles, empty/oversized paragraphs, multiple lines per element, title pages, page starts, exact vertical ties, margins, Unicode expansions, first/last lines, and vertical preferred-x behavior. Include cache-reused layouts and split/join index rebuilding.

## Durable documentation

Document the derived interaction indexes and their non-authoritative status in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:34). Existing caret/hit-test requirements in [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:100) remain semantically unchanged.

## User validation

Exercise pointer selection, caret placement, nearest-line clicks, vertical arrows with preferred x, first/last lines, page boundaries, and title-page exclusion at several zoom levels. Confirm results match the baseline and diagnostics show indexed queries.

## Implementation record — 2026-11-03

Added complete-layout indexes to [`ScreenplayLayout`](../../../crates/scriptcraft-core/src/layout.rs). The element-to-line index stores page/line locations in original page/line order, including title-page lines so caret geometry and caret-owner selection retain the former first-match semantics. The body-line index stores page, line, element, source range, and fixed absolute vertical position in document order for vertical navigation; a separate sorted permutation supports lower-bound nearest-y lookup without changing document-order navigation. All indexes are rebuilt as part of every fresh complete layout, and cache hits reuse the same immutable layout (and indexes); project replacement continues to discard the cached layout through the existing identity gate. No presentation or egui cache data moved into core.

Caret geometry and owner-page resolution now inspect only the requested element's placed-line locations. Nearest-line hit testing probes the ordered y index and at most the neighboring vertical groups, retaining `(distance, page, line)` tie ordering and the prior saturating-distance edge behavior. Vertical movement locates the current body line and accesses its adjacent body line by index, without constructing a flattened document vector; title pages are outside that navigation domain. Display/source conversions remain delegated to the existing shared immutable [`LayoutLine`](../../../crates/scriptcraft-core/src/layout.rs) values, so UTF-8 boundaries, Unicode expansion mapping, page-local x mapping, preferred-x rounding, selection extension, and margin clamping are unchanged.

Desktop diagnostics now distinguish indexed interaction query calls from legacy linear query calls and report index probes/candidate entries examined. The desktop oracle expectations were changed only for pointer, caret-owner/geometry, and vertical-navigation query counters; page paint, layout, wrapping, galley, and refresh expectations were not assumed to reflect subsequent work orders. Authored differential coverage compares indexed nearest hit testing, caret geometry, and body-line order/current-line lookup with independent linear references over title/body pages, multiple roles, empty and wrapped text, forced page starts, narrow margins, Unicode uppercase expansion, combining marks, ties, and first/last positions. It also covers complete-layout cache hits and split/join-shaped rebuilds. These tests are authored but unrun. No tests, builds, lint, formatting, manual verification, implementation checks, reviews, or independent validation were performed; WO-001–WO-007 remain user-unvalidated.

## Handoff

RS-WO-008 next removes unchanged-revision derived refresh work; no refresh semantics are altered by this order. All review and validation remain user-owned. The indexed interaction APIs and diagnostics are implementation artifacts, not evidence of passing behavior.

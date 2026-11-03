# RS-WO-002 — Representative desktop render-path oracle

- **Status:** Implemented 2026-11-03; user review and validation pending; not user-validated
- **Initiative:** [`ui-quality-of-life-v1-render-speed.md`](../ui-quality-of-life-v1-render-speed.md)
- **Depends on:** [`RS-WO-001`](WO-001-performance-diagnostics.md)
- **Validation owner:** User; no agent-run checks or independent validators

## Objective

Create deterministic long-document fixtures and a desktop-level oracle that measures actual work performed during scroll, input, navigation, zoom, and refresh operations.

The oracle supplements rather than replaces the core-only benchmark in [`wo006_responsiveness.rs`](../../../crates/scriptcraft-core/tests/wo006_responsiveness.rs:1).

## Implementation steps

1. Add deterministic 1, 15, 120, and 180-page fixtures using paragraph-shaped screenplay content rather than one artificial element per page alone.
2. Add imported-FDX variants containing direct paragraph overrides, role font settings, scene metadata, Cast Lists, Unicode AllCaps expansions, empty paragraphs, and a title page.
3. Drive the real desktop surface through:
   - idle and caret-blink frames;
   - incremental and large scrolls;
   - pointer selection and caret reveal;
   - insertion at the beginning, middle, and end;
   - vertical navigation and horizontal zoom/overflow;
   - changed and unchanged derived-refresh ticks.
4. Record RS-WO-001 counters and layout object identity for each operation. Assert structural work bounds, not milliseconds.
5. Keep a fresh uncached layout control so later cache implementations can compare complete structural output.
6. Correct the scope comments and synthetic painted-page accounting in [`wo006_responsiveness.rs`](../../../crates/scriptcraft-core/tests/wo006_responsiveness.rs:63) so core-layout measurements cannot be mistaken for desktop paint measurements.
7. Add a baseline/result capture template to the initiative record with build profile, document dimensions, operation, before/after observation, counters, user, and date.

## Structural acceptance

- The harness records structural desktop work, detects viewport-bounded page painting, and retains fresh-versus-cached complete layout comparison without relying on timing thresholds.
- It distinguishes layout construction, element wrapping, text layout, actual page painting, and derived reconciliation.
- Every fixture has a known page/element/line dimension and no production screenplay content.
- No assertion depends on wall-clock duration, host font rasterization, or platform timing.

## Regression coverage to author

Cover fixture dimensions, first/middle/last viewports, same scroll geometry, beginning/middle/end edits, unchanged refresh ticks, and fresh-versus-cached structural equivalence. Record that all authored tests remain unexecuted until the user runs them.

## Durable documentation

Update the initiative baseline table and identify the desktop oracle separately from the existing core benchmark. No product behavior requirement changes are needed for instrumentation alone.

## Implementation record — 2026-11-03

Added deterministic imported-FDX screenplay fixtures and desktop render-path regression coverage in [`main.rs`](../../../apps/scriptcraft/src/main.rs). The fixtures expose exact 1-, 15-, 120-, and 180-layout-page dimensions; 15/120/180 include a separate title page and therefore contain 14/119/179 body pages of 54 lines each. The fixtures use four screenplay paragraphs per body page (with role variation), plus an empty managed paragraph, direct paragraph overrides, Character AllCaps font settings with the Unicode `straße` expansion, scene metadata/arc containers, Cast Lists, and title-page content. No production screenplay content is used.

The desktop oracle exercises actual egui surface frames and `PerformanceDiagnostics` across an idle/cache-hit frame, focused caret frames, pointer selection, incremental and large scrolls (first/interior/final viewport positions), beginning/middle/end insertions, vertical navigation, conditional caret reveal, horizontal zoom overflow, and changed/unchanged derived-refresh ticks. It compares fresh and cached complete layout structures. The WO-004 expectations assert that every page is considered and allocated while painting and line submission is limited to the actual editor clip plus a fixed one-page overscan; page-rectangle vectors remain complete. RS-WO-005 now expects visible nonempty line/selection spans to build or reuse fixed-cell galleys, with repeated frames reusing unchanged spans. Before RS-WO-006, edits rebuilt wraps for the full fixture; the current oracle expects a single affected wrap build with reuse of unchanged wraps. Indexed interaction counters remain pending WO-007.

The core-only benchmark in [`wo006_responsiveness.rs`](../../../crates/scriptcraft-core/tests/wo006_responsiveness.rs) now labels itself as layout-only and no longer fabricates desktop frame, event, painted-page, or repaint counters. Its existing timing observations remain diagnostic only; the desktop oracle uses structural counts and no timing threshold.

Authored cases include `render_oracle_fixtures_have_known_dimensions_and_cached_equivalence` and `desktop_render_oracle_measures_actual_scroll_input_zoom_and_refresh_work`, alongside existing WO-001 diagnostics tests. All authored tests remain unexecuted. The initiative record and technical-context blueprint distinguish the desktop oracle from the core layout benchmark and retain the user-owned before/after capture template. No application rendering, edit, save, FDX-preservation, or refresh behavior was changed by this order.

## User validation

Run the oracle and baseline application in the chosen build profiles. Capture current 120-page scroll and typing observations with counters, review fixture relevance, and confirm the diagnostic data is sufficient for later before/after comparisons.

## Handoff

RS-WO-003 extended the desktop oracle with an application-update same-frame regression and changed the full-app render helper to call the shared production update method. `desktop_render_oracle_paints_accepted_update_input_in_the_same_frame_once` checks accepted text/IME, paste, preedit/commit, and key events against that frame's output and exact frame/event/layout/paint work; a separate case checks migration-modal isolation. Before WO-004, its render expectations represented all-page painting. WO-004 changed only page paint/line submission expectations to viewport plus fixed overscan and added culled-page accounting; the uncached/cached layout baseline, same-frame input scheduling, and later-work-order zero counters remain unchanged. These authored cases are not passing evidence: WO-001/002/003 coverage remains unexecuted and user review/validation is pending. No later rendering optimization is represented by this oracle.

## Handoff after RS-WO-004 — 2026-11-03

The desktop oracle checks all-page geometry consideration alongside viewport-bounded painting, one-page overscan, painted-line accounting, full rectangle vectors, and a single global caret lookup. Authored cases include first/middle/last ranges, partial pages, overscan edges, horizontal clip exclusion, title-page geometry and status total, offscreen caret/IME suppression, 175% fractional zoom, pointer routing, and reveal. All coverage remains unexecuted; user review/validation and before/after measurements are pending. RS-WO-005 leaves page culling and complete geometry intact while changing only the visible-line text submission mechanism.

## Handoff after RS-WO-005 — 2026-11-03

The desktop oracle now expects initial visible-line/span galley builds and unchanged-frame cache reuse instead of zero galley counters. It asserts galley operations stay bounded by painted line count and the maximum three color spans for one contiguous selection. The viewport/culling, complete page rectangle, line-count, whole-layout cache, and same-frame input expectations are retained; per-element wrap reuse and indexed-query counters remain pending WO-006 and WO-007 and were not pre-asserted as implemented. New order-specific coverage compares fixed-cell glyph mesh placement with the prior one-scalar painter geometry for ASCII, non-ASCII, combining marks, emoji, Unicode expansions, all four faces, and multiple zoom scales; it also covers exact cache-key changes, selected display spans over an uppercase expansion, colors, style, and scale. Tests remain authored and unexecuted, and the prior WO-001–WO-004 coverage remains user-unvalidated.

### Handoff after RS-WO-006 — 2026-11-03

The desktop edit oracle now expects exactly one affected element wrap built and all other fixture wraps reused for beginning/middle/end ordinary insertion. It retains fresh-layout structural comparison and all prior page placement, viewport paint, line, galley, and same-frame event assertions. Scroll, zoom, selection-only paints, and unchanged complete-layout hits do not count per-element wrap reuse because no document layout build runs. WO-007 interaction counters remain zero pending that order. These authored expectations are not passing evidence; all WO-001–WO-006 review and validation remain user-owned and pending.

### Handoff after RS-WO-007 — 2026-11-03

Pointer nearest-line hits, caret-owner/geometry resolution, and vertical navigation now increment indexed-interaction query counts and report actual ordered-index probes/candidate entries examined. Those paths no longer increment the linear-interaction counter. Existing desktop expectations were revised for those counters and retain all layout, page-allocation/paint, wrap, galley, event, and refresh expectations; no assumptions about RS-WO-008 or later work were added. Core differential tests and desktop oracle assertions are authored but unrun. RS-WO-001–WO-007 remain user-unvalidated, and no baseline/after measurements or timing claims were added.

### Handoff after RS-WO-008 — 2026-11-03

The refresh oracle now expects an initial/current-revision pass, followed by due-tick skips with no additional refresh attempt until a relevant revision or document replacement. It asserts distinct attempted, completed, skipped, changed, unchanged, and deferred outcomes. Save-before-timer coverage expects save candidate correction to be published and the rebased revision marked current; deferred imported-scene coverage expects one safe deferral per changed revision rather than repeated idle reconciliation. The scheduler runs after queued editor input, and a same-update Character edit coverage case expects the current frame's refresh to consume the newly dispatched cue; the oracle retains migration-modal exclusion. Added structural cases cover repeated idle ticks, edit/undo/redo re-arming, same-revision document replacement, successful save before the pending timer, and deferred work rearming. All coverage remains authored and unrun; RS-WO-001–WO-008 are user-unvalidated, and no timing or performance result is claimed.

### Handoff after RS-WO-009 — 2026-11-03

The derived-refresh diagnostic projection and oracle now include detached project-candidate, managed-heading scene-candidate, and redundant complete preservation-clone counters. WO-009 core coverage asserts one project candidate, scene-scoped candidate counts, and zero additional whole-preservation clones per heading; desktop coverage asserts the value-free counter labels. Existing revision-gate, retry/deferral, save-before-timer, and input-order expectations are retained. The project-candidate count does not imply source-byte sharing: its single required deep clone still includes original source bytes and unrelated preservation subtrees, which is documented as an unresolved WO-009 structural limitation. Coverage remains authored and unrun; WO-001–WO-009 are user-unvalidated, and no timing or performance result is claimed.

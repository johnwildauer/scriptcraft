# RS-WO-006 — Per-element immutable wrap reuse

- **Status:** Implemented 2026-11-03; user review and validation pending; not user-validated
- **Initiative:** [`ui-quality-of-life-v1-render-speed.md`](../ui-quality-of-life-v1-render-speed.md)
- **Depends on:** [`RS-WO-005`](WO-005-fixed-cell-line-galleys.md)
- **Authorization:** Covered by the user's 2026-11-03 authorization for sequential execution of all twelve orders; authorization is not review, validation, or acceptance
- **Validation owner:** User; no agent-run checks or independent validators

## Objective

Retain immutable per-element wrap results across session revisions so ordinary edits rewrap only elements whose effective layout inputs changed.

Whole-document page placement remains in place for this order. Fully incremental suffix pagination is explicitly out of scope.

## Implementation steps

1. Extend [`LayoutCache`](../../../crates/scriptcraft-core/src/layout.rs:154) beyond its single complete-layout slot with a bounded map of immutable per-element wrap results.
2. Key each entry by stable element identity, canonical content, role, layout purpose, title/body context, effective paragraph geometry, effective text style, and wrapping limits.
3. Use a content fingerprint only as a lookup accelerator. Verify full canonical content on a hit, or use an equivalent collision-safe revision contract, so a collision cannot reuse incorrect text.
4. Store canonical text, display text, and display/source maps in immutable shared values so page placement does not deep-copy every string for each revision.
5. Continue running page placement and keep-with-next logic over all elements while reusing unchanged wraps. Preserve explicit page starts, oversized handling, empty paragraphs, and title-page behavior.
6. Avoid cloning [`SourceLayoutSettings`](../../../crates/scriptcraft-core/src/formatting.rs:509) and its direct-paragraph map on a complete-layout cache hit.
7. Track per-build used generations and evict stale entries deterministically when the cache exceeds a bounded retention policy.
8. Clear or correctly rekey entries on runtime layout-input changes, source-setting changes, purpose changes, and document replacement.
9. Report per-element builds and reuses through RS-WO-001 diagnostics.

## Structural acceptance

- A one-element ordinary edit builds wraps only for that element when all global effective inputs are unchanged.
- Unchanged canonical/display strings and source maps are shared rather than reallocated for each revision.
- Complete page results equal a fresh uncached build for the same project and effective inputs.
- Cache size remains bounded under repeated edits and document replacement.
- Scroll and zoom do not invalidate document-point wrap entries merely because presentation changed.

## Regression coverage to author

Compare cached and fresh structural output for mixed roles, direct overrides, imported role settings, hidden text, Unicode expansions, empty/oversized elements, title pages, explicit page starts, split/join, undo/redo, role changes, purpose switches, and document replacement. Add collision and eviction test seams plus allocation/reuse counters.

## Durable documentation

Define measured per-element reuse in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:51), and clarify in [`product-overview.md`](../../../.forge/requirements/product-overview.md:151) that this is not the existing non-goal of unmeasured incremental pagination. Add structural cache-equivalence requirements to [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:86).

## Implementation record — 2026-11-03

Added a bounded immutable per-element wrap cache to [`LayoutCache`](../../../crates/scriptcraft-core/src/layout.rs). Each collision-checked bucket lookup uses a fingerprint only to select candidates and verifies the complete canonical UTF-8 content plus element identity, role, Editor/Print purpose, title/body context, effective fixed-point paragraph geometry, effective text style, character-cell width, and wrapping limit before reuse. Whole-document page placement and keep-with-next handling still execute for every layout revision; explicit page starts, empty/oversized elements, title pages, hidden Print roles, and source/display mappings remain part of the existing placement/output behavior. [`LayoutLine`](../../../crates/scriptcraft-core/src/layout.rs) now retains immutable shared canonical/display strings and display/source maps so unchanged wrapped values are shared across layouts rather than deep-copied. A clone-stable opaque Project layout identity prevents complete-layout and wrap-cache aliasing across document replacement; semantic equality and serialization do not include it.

Retention is bounded by both 4,096 entries and an estimated 32 MiB of retained canonical/display text and source-map storage. Every successful layout build advances a generation; reuse stamps entries, and deterministic least-recent-generation/insertion-order eviction removes stale entries when a bound is exceeded. A single result larger than the byte limit is laid out normally but not retained. Effective wrap keys exclude egui font-atlas data, presentation zoom/scale, palette, and the RS-WO-005 context-local display-galley cache. Those remain owned by the egui presentation adapter; viewport changes therefore do not invalidate core document-point wraps.

`LayoutBuildDiagnostics` now distinguishes actual element wraps built from per-element cache reuses, while preserving value-free count/timing output. Complete-layout cache hits avoid cloning `SourceLayoutSettings` and its direct-paragraph map. The RS-WO-002 desktop oracle changes from whole-document wraps built on every edit to one affected wrap built and unchanged wraps reused; complete-layout, whole-page-placement, page-paint, line, and galley expectations remain separately measured.

Authored but unrun core regressions cover one-element editing with shared immutable geometry/maps and cached-vs-fresh layout equivalence across imported role/direct formatting, Unicode AllCaps expansion, empty and long elements, hidden text, title pages, explicit page starts, undo/redo, role changes, split/join, Editor/Print purpose changes, source-setting changes, runtime wrap-limit changes, and document replacement; additional cases force fingerprint collisions and bounded eviction. The RS-WO-002 desktop oracle now expects one wrap build and the remaining fixture wraps reused for beginning/middle/end insertion. No tests, builds, lint, formatting, manual verification, implementation checks, review, or independent validation were run. WO-001–WO-005 and WO-006 remain user-unvalidated.

## User validation

Run core layout/long-document/export tests and inspect cache counters for beginning/middle/end typing. Confirm page layout, source mapping, PDF/plain-text output, Save/Save As, and reopen remain unchanged while only affected elements rebuild wraps.

## Handoff to RS-WO-007

RS-WO-007 may add layout-owned indexes over the resulting complete line/page structure. It must preserve immutable wrap values and the existing whole-document placement path; it must not move egui atlas/presentation data into the core cache. No WO-007 implementation is included here.

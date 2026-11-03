# RS-WO-005 — Fixed-cell line and selection-span galleys

- **Status:** Implemented 2026-11-03; user review and validation pending; not user-validated
- **Initiative:** [`ui-quality-of-life-v1-render-speed.md`](../ui-quality-of-life-v1-render-speed.md)
- **Depends on:** [`RS-WO-004`](WO-004-viewport-page-culling.md)
- **Authorization:** Covered by the user's 2026-11-03 authorization for sequential execution of all twelve orders; authorization is not review, validation, or acceptance
- **Validation owner:** User; no agent-run checks or independent validators

## Objective

Replace per-character painter requests and per-character string allocation with line/selection-span galleys that preserve explicit fixed-cell glyph positions.

This is a text-emission optimization only. It must not change deterministic wrapping, canonical/display source maps, font roles, or selection geometry.

## Implementation steps

1. Replace the per-character text adapter in [`main.rs`](../../../apps/scriptcraft/src/main.rs) with a cached galley adapter that emits one galley per visible line or same-color selection span.
2. Assign glyph positions from the existing line origin, display column, and character-cell width. Do not use ordinary whole-line host-font advances as a shortcut around fixed-cell metrics.
3. Preserve regular/bold/italic/bold-italic face selection and effective role size from [`renderer_font_for_style()`](../../../apps/scriptcraft/src/main.rs:1534).
4. Split lines only at selected/unselected color boundaries and reuse unchanged galleys where their content, font, scale, fixed-cell geometry, and color span keys are unchanged.
5. Retain [`LayoutLine::display_char_for_source()`](../../../crates/scriptcraft-core/src/layout.rs:90) and [`LayoutLine::source_for_display_char()`](../../../crates/scriptcraft-core/src/layout.rs:112) as the authoritative display/source mapping, including Unicode uppercase expansions.
6. Keep underline geometry and hidden-text colors independent of the text batching implementation.
7. Record line/span galley builds and reuses through RS-WO-001 diagnostics. Eliminate the ordinary per-frame per-character string-allocation path.

## Structural acceptance

- Text-layout work is proportional to visible lines and their color spans, not total characters.
- Every glyph retains the same fixed-cell x/y position as the pre-change adapter within the existing presentation tolerance.
- Offscreen lines build no galleys.
- Selection changes do not alter unselected glyph geometry or canonical/source offsets.
- No ordinary host-font line measurement participates in screenplay pagination or fixed-cell painting.

## Regression coverage to author

Cover ASCII, non-ASCII, combining characters, emoji, Unicode AllCaps expansions, every role face, underline, hidden text, partial selections, expanded-source selection endpoints, zoom levels, and fixed-cell geometry against the previous adapter. Include cache-key invalidation for content, style, scale, palette, and selection changes.

## Durable documentation

Record the fixed-cell line/span rendering requirement and source-map preservation in [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:100) and the adapter/cache ownership in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:34).

## User validation

Inspect ordinary and selected text across roles, Unicode cases, AllCaps expansions, hidden roles, underline, zoom, and IME. Compare line wrapping, glyph alignment, caret positions, and selection geometry with the baseline. Review galley counters during scroll and typing.

## Handoff

RS-WO-006 reduces core layout work independently of this desktop text-emission path by reusing unchanged element wraps.

## Implementation record — 2026-11-03

Replaced the visible-line per-character painter loop in [`main.rs`](../../../apps/scriptcraft/src/main.rs) with a line/selection-span adapter. It creates spans only where the selected/unselected text color changes, submits each nonempty span as a galley, and positions every Unicode scalar from the existing line origin, display column, and authoritative fixed-cell width. The galley glyph records, advances, row/galley bounds, and glyph mesh x-coordinates are adjusted to explicit cell positions; font advances and kerning never determine screenplay pagination or fixed-cell placement. Bundled regular, bold, italic, and bold-italic face selection and egui font fallback remain in use. Selection background rectangles, hidden-text color, and underline geometry remain independent of the text batching path.

A bounded cache (16,384 entries, clear-on-capacity) lives in egui context-local temporary data, so cached atlas-dependent galleys cannot cross egui contexts. Its key includes complete span text, face family and size, fixed-cell width, line-height input, text color, and pixels-per-point. Content, style, scale, palette, selection spans, and context changes therefore produce distinct entries as applicable. Existing `LayoutLine` display/source maps remain authoritative for byte selection endpoints and Unicode casing expansions. RS-WO-001's value-free galley counters now count each actual span cache build or reuse; WO-002 checks visible-line-bounded counts. No page allocation, culling, document-point pagination, layout cache, caret/hit-test geometry, editing, FDX, save, or export path was changed.

Authored but unexecuted desktop regressions are `fixed_cell_galleys_match_scalar_mesh_positions_for_unicode_faces_and_zoom`, `fixed_cell_galley_cache_reuses_exact_keys_and_invalidates_changed_inputs`, and `unicode_expansion_selection_is_batched_at_authoritative_display_boundaries`. Existing rendered-font tests were updated for line/span text shapes, and the WO-002 oracle now expects initial galley builds and same-frame reuse rather than zero counters. The equivalence case covers ASCII, non-ASCII, combining marks, emoji, Unicode expansion scalars, four bundled faces, and three scale values by comparing the batched mesh placement with the prior one-scalar painter layout. Cache tests cover content, font face/size, cell width, line-height input, color, and scale-key changes; the AllCaps case checks selection of the source `ß` across its displayed `SS` expansion and verifies selected/unselected colors. Existing caret/hit-test tests continue to own document geometry coverage. No tests, builds, lint, formatting, manual verification, implementation checks, review, or independent validation were run; user review/validation remains pending.

The implementation relies on the pinned egui/epaint 0.29 galley representation: one glyph record per Unicode scalar and four mesh vertices per textured glyph. The code checks that structural contract before applying the fixed-cell mesh adjustment. If a future epaint backend changes that contract, a correctness fallback retains the previous scalar painting for that unsupported span rather than submit incorrectly placed text; such a fallback would reduce batching for that span and should be reconsidered only with geometry-equivalent support. User validation should inspect ordinary/selected text for every role face, Unicode fallback/combining/emoji, AllCaps expansions, hidden roles, underline, zoom, IME, glyph alignment, caret position, and selection geometry while observing privacy-safe galley counters.

### Handoff to RS-WO-006

WO-006 owns core document-point wrap reuse and whole-document pagination; it is independent of the egui context-local display-galley cache. Keep `LayoutLine` display text, source maps, fixed-cell widths, and complete page placement semantically equivalent so this adapter can continue to reuse unchanged span galleys by its own presentation key. Do not move presentation scale, palette, font atlas data, or galley-cache state into the core layout cache. User validation of WO-001 through WO-005 remains pending, and no timing baseline or after measurement is claimed.

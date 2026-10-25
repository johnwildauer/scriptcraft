# Stage 2 — Layout, pagination, hit testing, and rendering reuse

## Scope and method

This is a source-based assessment of the existing layout-to-output pipeline for a small
screenplay-writing app. It separates **observed facts** from **recommendations**. The traced path is:

`Document` → paragraph shaping and line breaking → paginated [`DocLayout`](../../crates/layout/src/lib.rs:432) / [`Page`](../../crates/layout/src/lib.rs:408) items → display-list [`Draw`](../../crates/layout/src/display.rs:24) values → screen raster, PNG, or PDF.

The target app is assumed to edit screenplay elements continuously, provide a page-preview mode,
and eventually export screenplay pages. It does not initially need Word-compatible tables, fields,
comments, tracked revisions, floating graphics, or complex secondary stories.

## Executive assessment

The pipeline is highly reusable for screenplay output. Its strongest boundary is the separation
between paragraph layout, pagination, display-list generation, and backends. A screenplay app can
keep the paragraph shaper, line geometry, pagination state, hit testing, and shared display list,
while supplying a smaller screenplay-oriented paragraph/style vocabulary and a fixed screenplay
page profile.

The current implementation is more general than the app needs. It pays for tables, notes, floating
objects, text boxes, sections, fields, revision markup, bidi, vertical text, and Word compatibility
rules. Those features should remain below an adapter only if reuse is cheaper than replacement; they
should not leak into the screenplay UI or become the screenplay's semantic model.

The main architectural caution is cache granularity. Paragraph layouts are memoized, but pagination
is rebuilt as a whole [`DocLayout`](../../crates/layout/src/lib.rs:432) whenever the session cache key
changes. That is a sound MVP design and already fast, but a long screenplay editor should eventually
add a screenplay-specific incremental pagination frontier rather than assuming that paragraph-level
memoization alone makes all downstream page geometry incremental.

## 1. Observed pipeline

### 1.1 Document to shaped paragraph

[`layout_para()`](../../crates/layout/src/para.rs:1135) resolves paragraph and character styles,
selects/falls back fonts, shapes text into glyphs, clusters, and style runs, then breaks the
paragraph into [`Line`](../../crates/layout/src/para.rs:136) records. A cluster carries source byte
range, advance, kind, style index, glyph range, break opportunity, and object metrics
([`Cluster`](../../crates/layout/src/para.rs:103)). This is the correct abstraction for screenplay
text: a paragraph role can determine width/indent/spacing while shaped clusters retain precise source
mapping for Unicode, ligatures, and inline emphasis.

The line record stores top, height, baseline, cluster range, per-cluster x positions, source byte
range, break kind, available left/right edges, and bidi visual geometry
([`Line`](../../crates/layout/src/para.rs:136)). [`ParaLayout`](../../crates/layout/src/para.rs:303)
also retains the resolved paragraph, styles, glyphs, clusters, lines, text length, proofing issues,
hyphenation information, and object/equation metadata. In other words, the shaped paragraph is both
the renderer's input and the hit-testing index; there is no need for a second screenplay-only text
measurement representation.

Line breaking is first-fit word-processor breaking with tabs, manual line/page/column breaks,
hyphenation, exclusions, labels, alignment, bidi, line spacing, and grid handling. The module
overview explicitly describes shaping, line breaking, tabs, list labels, and alignment
([`para` module documentation](../../crates/layout/src/para.rs:1)). For screenplay, the important
reuse is line measurement and source mapping, not list labels or Word's broad formatting matrix.

### 1.2 Paragraphs to pages

[`layout()`](../../crates/layout/src/lib.rs:1706) runs one or more pagination passes. It repeats when
body page-total fields change the result, and can repeat footnote numbering when numbering restarts
per page ([`layout()` pass loop](../../crates/layout/src/lib.rs:1706)). Each pass creates a
[`PageBuilder`](../../crates/layout/src/lib.rs:1224), walks body blocks in order, and places
paragraphs/tables into columns and pages ([`layout_pass()`](../../crates/layout/src/lib.rs:1744)).

For print layout, a page is created with section width/height and a body rectangle derived from
margins, gutter, headers, footers, and columns ([`PageBuilder::new_page()`](../../crates/layout/src/lib.rs:1609)).
The resulting [`Page`](../../crates/layout/src/lib.rs:408) contains page dimensions, section and
displayed page number, body rectangle, body items, header/footer items, first body block, and an
optional markup area. Page coordinates are points with y increasing downwards
([`Placed`](../../crates/layout/src/lib.rs:194)).

Each paragraph is laid out in the current column width, then split into one or more
[`Placed::Lines`](../../crates/layout/src/lib.rs:196) pieces. The placement loop accounts for space
before/after, floating exclusions, explicit page/column breaks, footnotes, and available bottom
space ([`place_para_in_flow()`](../../crates/layout/src/lib.rs:2683)). It applies:

* page-break-before;
* keep-lines-together and keep-with-next;
* widow/orphan control with a two-line minimum at either end;
* line-level splitting at page/column boundaries;
* unsplittable rows and text beside floating objects; and
* continuation of notes at line boundaries.

The relevant logic is explicit in [`place_para_in_flow()`](../../crates/layout/src/lib.rs:2753) and
the widow/orphan block ([`widow/orphan handling`](../../crates/layout/src/lib.rs:2846)). The tests
confirm long paragraphs wrap and paginate, page breaks start a new page, headers/footers can move
the body boundary, and long notes continue ([`long_paragraph_wraps_and_paginates()`](../../crates/layout/src/tests.rs:28),
[`page_break_char_starts_new_page()`](../../crates/layout/src/tests.rs:443),
[`a_tall_footer_pushes_the_body_up()`](../../crates/layout/src/tests_pages.rs:57),
[`long_endnotes_continue_on_new_pages()`](../../crates/layout/src/tests_pages.rs:101)).

### 1.3 Print, web, draft, and reading layout

The layout engine has view modes in [`ViewMode`](../../crates/layout/src/lib.rs:41): print, web,
draft, read, and outline. Web/draft/outline use one long page; reading mode uses a finite viewport
height and reflows into narrow pages. The print page uses document section metrics, while reflow
overrides width, height, margins, columns, borders, and vertical alignment
([`layout_pass()` reflow setup](../../crates/layout/src/lib.rs:1791)).

The tests establish an important product property: reading layout changes page/column boundaries
without changing the document or print/export layout ([`reading_layout_reflows_into_viewport_height_pages_without_changing_print_layout()`](../../crates/layout/src/tests.rs:48)). This is a good model for a screenplay editor's continuous-writing mode versus page-preview mode:

* continuous mode can use a web-like or reading-like stream for low visual interruption;
* page preview can use fixed screenplay paper and margins;
* both modes can point at the same document and paragraph positions; and
* export must always request print metrics, never the transient editor viewport.

### 1.4 Display list boundary

[`page_display()`](../../crates/layout/src/display.rs:361) converts a page's placed items to a vector
of [`Draw`](../../crates/layout/src/display.rs:24) values. The list includes glyph batches with
font face, size, glyph positions, source text, links, and byte ranges; fills; lines; images; shapes;
paths; figures; marks; transforms; clips; and WordArt. It handles ordering, header/footer dimming,
formatting marks, tracked changes, comments, and transformed content before a backend sees it
([`page_display()`](../../crates/layout/src/display.rs:361)).

This is a valuable renderer contract. A screenplay app can generate only glyphs, rules, fills, and
perhaps simple page decorations while retaining the same `Draw`-style backend boundary. It should
not make the screen renderer read document paragraphs directly: the current display layer is what
keeps screen, PNG, and PDF geometry aligned.

### 1.5 Screen raster and PNG

[`render_page()`](../../crates/render/src/lib.rs:401) chooses a pixels-per-point scale, clamps the
raster size, and delegates to [`render_region()`](../../crates/render/src/lib.rs:409). The renderer
fills page colour, obtains the display list, computes a visible page-space rectangle, and draws each
display item with `vello_cpu` ([`render_region()`](../../crates/render/src/lib.rs:409)). Glyphs are
drawn from font outlines; images use shared decoded-media caches; clipping and transforms are
handled in page space. [`Rendered`](../../crates/render/src/lib.rs:36) stores premultiplied RGBA8
pixels and provides PNG/JPEG conversion ([`Rendered::to_png()`](../../crates/render/src/lib.rs:57)).

The renderer bounds media caches by entries and approximate bytes and keys a decoded value to the
same source allocation ([`MediaCache`](../../crates/render/src/lib.rs:115)). This avoids stale decoded
media when a document replaces bytes under the same media key. For a screenplay MVP with no images,
the media cache can be omitted initially; the raster API and pixel-boundary safeguards are still
useful.

The render tests verify that text produces dark pixels and that the page raster has expected Letter
dimensions ([`renders_text_dark_pixels()`](../../crates/render/src/lib.rs:948)). Engine export commands
build an export layout, select a page, render it, and encode PNG ([`file::export_png()`](../../crates/engine/src/cmd/file.rs:230)).

### 1.6 PDF export

[`wordcraft_pdf::export()`](../../crates/pdf/src/lib.rs:97) creates a fresh print layout, then calls
[`export_layout()`](../../crates/pdf/src/lib.rs:104), which walks selected pages and sends the same
display-list vocabulary to `krilla`. Glyphs remain real selectable/searchable PDF text when the
font can be embedded; links, outlines, metadata, tagging, images, shapes, and page decorations are
also supported ([`PDF module overview`](../../crates/pdf/src/lib.rs:1)). If a font cannot be subset,
export retries with that face rendered as outlines ([`export_layout()` fallback](../../crates/pdf/src/lib.rs:117)).

Print Markup is an additional layout concern: the page gains a markup area, and PDF scales the
document page into the paper width ([`Fit::of()`](../../crates/pdf/src/lib.rs:213)). A screenplay app
should not include this complexity unless it needs review comments in exported pages.

## 2. Cache invalidation and incremental behavior

### 2.1 Paragraph cache

[`LayoutCache`](../../crates/layout/src/lib.rs:460) memoizes `Arc<ParaLayout>` values. Its key includes
paragraph revision, available width, list label, page-dependent fields, table formatting, notes,
float exclusions, equation number, reading line spacing, document grid, object-fit mode, and whether
deleted text is hidden ([`Key`](../../crates/layout/src/lib.rs:444),
[`Ctx::para_labelled()`](../../crates/layout/src/lib.rs:668)). This is appropriately conservative:
the same paragraph can shape differently when width, exclusions, style environment, page fields, or
visibility changes.

At the end of a pass, unused paragraph entries may be evicted if the cache is much larger than the
set used by the pass ([`layout_pass()` cache retention](../../crates/layout/src/lib.rs:2038)). A document/style
environment hash clears paragraph layouts when shared style, numbering, default tab, footnote,
proofing, hyphenation, numeral, field, or outline inputs change ([`env_hash()`](../../crates/layout/src/lib.rs:495)).
Picture wrapping outlines have a separate bounded cache keyed by media identity, byte length, and
crop ([`Ctx::wrap_outline()`](../../crates/layout/src/lib.rs:609)).

### 2.2 Session layout cache

[`Session::layout()`](../../crates/engine/src/session.rs:727) caches the complete `Arc<DocLayout>` by
document revision plus view width/height, view mode, reading spacing, proofing, outline state,
deletion visibility, and numeral mode ([`Session::layout()` cache check](../../crates/engine/src/session.rs:738)).
Document edits increment the session revision through [`Session::touch()`](../../crates/engine/src/session.rs:683),
so the complete layout is recomputed after an edit while unchanged paragraph layouts remain reusable.
Explicit invalidation, for example after font changes, clears both the complete layout and paragraph
cache ([`Session::relayout()`](../../crates/engine/src/session.rs:782)).

The engine deliberately creates an independent fresh cache for output
([`Session::export_layout_with()`](../../crates/engine/src/session.rs:775)). That makes export immune
to transient proofing, reading, viewport, and markup state, but means export does not reuse the live
session's paragraph cache. This is a good correctness boundary; a screenplay app can later add a
separately keyed print-layout cache if export latency becomes visible.

### 2.3 What invalidates screenplay page geometry

For a fixed screenplay profile, the following must invalidate pagination even if the document text is
unchanged:

* paper size, top/bottom/left/right margins, and page numbering settings;
* screenplay role/style metrics, especially dialogue and character indents;
* font availability, substitution, or metrics;
* line spacing, font size, and any user zoom-independent typography setting;
* page-break markers and keep/widow/orphan policy; and
* display-only modes that intentionally change layout, such as continuous versus page preview.

Zoom should not invalidate layout because it changes only the renderer transform. The existing
separation demonstrates this: layout is in points, while [`render_page()`](../../crates/render/src/lib.rs:401)
chooses pixel scale at draw time.

## 3. Caret and mouse mapping

### 3.1 Page index and position index

[`DocLayout`](../../crates/layout/src/lib.rs:432) stores an index from `(StoryRef, Path)` to page/item
pieces ([`DocLayout::index`](../../crates/layout/src/lib.rs:434)). Body paragraphs therefore avoid a
whole-document scan for caret geometry. Repeated header/footer stories and continued notes use a
page-hint-aware search across their placed pieces ([`DocLayout::pieces()`](../../crates/layout/src/hit.rs:334)).

### 3.2 Mouse point to text position

[`DocLayout::hit()`](../../crates/layout/src/hit.rs:236) first handles turned text, gathers lines for
the requested story, chooses the nearest line using vertical distance weighted more heavily than
horizontal distance, and calls [`ParaLayout::off_at_x()`](../../crates/layout/src/para.rs:2438). The
returned `Pos` preserves story, structural path, and UTF-8 byte offset. Object hit testing separately
handles z-order, rotated frames, text-box borders, and behind-text behavior
([`DocLayout::object_at_in()`](../../crates/layout/src/hit.rs:274)).

This is directly suitable for screenplay clicking. The screenplay UI needs only body-story hit
testing in the MVP, plus optional title-page story support later. It should retain the line scoring
and byte-offset conversion rather than map x positions by character count.

### 3.3 Caret geometry and keyboard navigation

[`DocLayout::caret_on()`](../../crates/layout/src/hit.rs:398) finds the line containing the position,
gets the x coordinate from [`ParaLayout::x_of()`](../../crates/layout/src/para.rs:2359), computes
ascent/descent-aware caret height, and transforms it for turned text. [`vertical()`](../../crates/layout/src/hit.rs:420)
keeps a page-space goal x for Up/Down navigation and crosses page boundaries when necessary.
[`line_bounds()`](../../crates/layout/src/hit.rs:464), [`visual_step()`](../../crates/layout/src/hit.rs:625),
selection rectangles, line spans, selection-bar mapping, and column segments all derive from the
same line geometry.

The agreement test is particularly important: caret positions are clicked back through `hit()` and
must return the same positions ([`hit_and_caret_agree()`](../../crates/layout/src/tests.rs:72)). For a
screenplay app, preserve this invariant for every role, especially dialogue lines with different
indents and wrapped parentheticals.

### 3.4 Caret invalidation after edits

The session owns `page_hint` and updates it during caret movement; the hint prioritizes the page where
the caret was last seen, which matters for repeated header/footer stories and continued notes
([`Session` page state](../../crates/engine/src/session.rs:344), [`find_object()`](../../crates/layout/src/hit.rs:315)).
After an edit, the session revision invalidates the layout. The document editing layer clamps or
returns the post-edit position, so the UI can ask the new layout for caret geometry rather than
trying to patch old x/y coordinates.

Recommendation: keep positions semantic and recompute geometry after every committed edit. Do not
persist a caret's page/x/y as identity; page numbers and line positions move as soon as a preceding
dialogue block wraps differently.

## 4. Screenplay-specific fit

### 4.1 Fixed page metrics

**Observed capability.** Section properties already provide page width/height, margins, gutter,
columns, headers/footers, page numbering, and page breaks. Print pages use those metrics directly in
[`PageBuilder::new_page()`](../../crates/layout/src/lib.rs:1609). Tests cover Letter/A4 setup, margins,
orientation, and explicit breaks ([`page_setup_and_breaks()`](../../crates/engine/src/tests.rs:1315)).

**Recommendation.** Reuse the fixed-point geometry and expose one screenplay `PageProfile` that
resolves to a fixed paper size and margins. Prefer a small profile API over exposing all section
properties. A screenplay page preview should be deterministic: no viewport-derived body width,
automatic columns, or markup area.

### 4.2 Margins and dialogue indentation

**Observed capability.** Paragraph indents are resolved into line left/right edges; the placement
code also handles logical RTL indents and mirrored pages ([`push_para()`](../../crates/layout/src/lib.rs:1185)).
Character and paragraph metrics are already part of the paragraph cache key and line geometry.

**Recommendation.** Model screenplay roles as paragraph styles or an adapter that supplies role
metrics: scene heading/action, character cue, dialogue, parenthetical, transition, and title-page
roles. Use style paragraph indents for the actual layout so wrapping, caret mapping, selection, and
PDF all agree. Avoid implementing screenplay indentation by inserting spaces or tabs; that would
make copy/paste, hit testing, and metric changes fragile. If standard screenplay rules require a
different left and right margin for dialogue, express them as resolved paragraph box edges, not
special renderer offsets.

### 4.3 Manual page breaks

**Observed capability.** Page-break clusters have a distinct `ClKind::PageBreak` and `LineEnd::PageBreak`
([`ClKind`](../../crates/layout/src/para.rs:89), [`LineEnd`](../../crates/layout/src/para.rs:125)). The paginator honors
page-break-before and explicit page breaks, including the edge case where a break at a full page must
not create an accidental blank page ([`place_para_in_flow()`](../../crates/layout/src/lib.rs:2822)).

**Recommendation.** Keep explicit page breaks as structural editing commands, not raw visible
characters in the screenplay UI. The app can render a subtle editor marker in continuous mode and a
page boundary in preview. Export should honor them exactly. Also consider a screenplay-specific
“force page break before element” property if users need a break that survives paragraph text edits.

### 4.4 Orphans, widows, and screenplay blocks

**Observed capability.** Existing widow/orphan control keeps two lines at each end when enabled, and
keep-lines/keep-with-next can move paragraphs or chains to a new page
([`widow/orphan handling`](../../crates/layout/src/lib.rs:2846), [`keep handling`](../../crates/layout/src/lib.rs:2753)).

**Recommendation.** Reuse the line-level mechanism but define screenplay semantics explicitly:

* a character cue should stay with at least the first dialogue line;
* a parenthetical should stay with the dialogue line it qualifies;
* a short dialogue block should prefer staying together, but a long speech may split;
* a transition should not be stranded at the bottom without the following scene heading if the
  product's pagination policy treats them as a unit; and
* scene headings may use keep-with-next for the first action line, subject to an escape hatch when
  the block exceeds a full page.

The current generic `keep_with_next` behavior is a useful primitive, but it cannot infer screenplay
relationships by itself. Add role-aware keep rules before pagination, or compile role relationships
into paragraph properties in a screenplay layout adapter. Do not globally enable “keep everything”:
that can create large blank areas for long dialogue.

### 4.5 Continuous editing versus page preview

**Observed capability.** Web/draft/read modes deliberately reflow while preserving the document;
print/export uses fixed print metrics. The session distinguishes cached live layout from a fresh
export layout ([`Session::layout()`](../../crates/engine/src/session.rs:727), [`Session::export_layout()`](../../crates/engine/src/session.rs:770)).

**Recommendation.** Offer two explicit presentation modes:

1. **Write mode:** a continuous column with screenplay role indents, minimal page chrome, and caret
   movement that does not jump as pages are visually tiled.
2. **Page preview:** fixed paper rectangles and page boundaries using the same print layout and
   display list used for export.

Both should share one `Pos` model and one shaped paragraph representation. The write mode may use a
continuous y stream for editing, but it must not silently alter role widths or line spacing relative
to export unless the product clearly labels it as a draft view. A “page-aware writing mode” can show
page breaks as separators while still using continuous scrolling.

## 5. Keep, simplify, and avoid

### Keep

* `ParaLayout`, `Line`, `Cluster`, and source-byte-to-x mapping.
* Font shaping and fallback, including grapheme/cluster-aware caret behavior.
* Point-based page geometry and a dedicated `Page`/`DocLayout` result.
* Paragraph-level cache keys that include width and all metric-affecting inputs.
* A page index from structural positions to placed line pieces.
* Shared display-list generation before any backend.
* One renderer transform for screen and PNG, and one PDF backend over the same display list.
* Caret/hit/selection helpers and the caret-hit agreement test.
* Generic keep-lines, keep-with-next, widow/orphan, and explicit-break primitives.
* A separate print/export layout so transient editor view state cannot change exported pagination.

### Simplify for a screenplay MVP

* Replace the broad Word section surface with one fixed screenplay page profile.
* Keep one body story and defer headers, footers, notes, tables, text boxes, columns, floats, and
  vertical sections.
* Keep one horizontal text direction initially; retain the underlying bidi-safe line/caret API if
  multilingual scripts are in scope.
* Generate only glyphs, simple rules, fills, and perhaps page numbers in the display list.
* Keep paragraph cache plus full-layout cache first; measure before designing incremental page
  invalidation.
* Use a built-in screenplay stylesheet/role adapter rather than exposing Word's hundreds of
  paragraph properties.
* Use a simple PDF exporter that preserves selectable text and fixed page size; defer tagging,
  comments, links, and outline generation unless product requirements demand them.

### Avoid

* Rendering directly from document paragraphs in the UI or export path.
* Mapping mouse x to Unicode scalar indexes or ASCII character counts.
* Treating page index, line index, or x/y caret geometry as persistent identity.
* Encoding screenplay roles with spaces, tabs, all-caps, font size, or indentation alone.
* Reusing Word's list-label, field, note, revision, comment-balloon, or float machinery as a
  screenplay semantic model.
* Making the continuous editor's viewport width the source of truth for PDF pagination.
* Implementing role-aware pagination by special cases in the raster renderer; role rules belong
  before or during pagination, where they can affect both hit testing and export.
* Assuming paragraph-level memoization makes pagination incremental. A change in one paragraph's
  height can move every subsequent page boundary.

## 6. Risks and follow-up tests

1. **Metric drift:** screenplay output depends on exact font metrics. Pin the selected open font,
   size, leading, page profile, and role indents; add golden tests for line wraps and page counts.
2. **Role coupling:** changing a visual style must not accidentally change semantic element role.
   Keep role policy separate from generic style inheritance.
3. **Pagination policy:** add tests for character cue plus one dialogue line, parenthetical plus
   dialogue, long dialogue split, scene heading at the bottom, explicit page breaks, and long blocks
   that cannot satisfy keep rules.
4. **Continuous/print divergence:** for the same document, assert that write mode can reflow while
   export remains fixed and that export is unaffected by zoom, caret position, proofing marks, or
   viewport height.
5. **Caret stability:** for every screenplay role and representative wrap, assert
   caret → mouse hit returns the same position, including paragraph start/end and wrapped lines.
6. **Font/cache invalidation:** change a role's font/size and verify page count, line geometry, and
   caret positions update; then restore it and verify cached geometry is reused.
7. **PDF/PNG parity:** compare a page screenshot with the PDF rasterized at the same scale. The
   display-list boundary should make glyph positions and page decorations identical.
8. **Large-script performance:** benchmark edits near the beginning, middle, and end of a 120–180
   page screenplay. If downstream pagination dominates, add a page-frontier cache keyed by profile,
   paragraph revisions, and accumulated pagination state.

## Conclusion

WordCraft's layout pipeline is a good foundation for a screenplay app if reused as a geometry and
backend service rather than as a complete Word feature surface. Keep shaped paragraph layouts,
fixed-point pages, generic pagination constraints, shared display lists, and hit/caret mapping. Add
screenplay role metrics and role-aware keep rules at the paragraph/pagination boundary. Use a
continuous writing view and a deterministic fixed-page preview/export view over the same document,
and defer the Word-specific machinery that does not contribute to screenplay editing.

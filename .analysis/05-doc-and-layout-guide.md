# A beginner's guide to WordCraft's document model and layout

This guide is a tour of two Rust crates that are easiest to understand together:
[`wordcraft-doc`](../crates/doc/Cargo.toml:2) stores a document, and
[`wordcraft-layout`](../crates/layout/Cargo.toml:2) works out how that document fits on pages.
The first crate is the main subject. The second is its consumer: it translates durable document
data into temporary geometry that a renderer or an editor can use.

The examples and statements marked **In this codebase** describe what the current source does.
Statements marked **A useful rule** are general advice for designing a smaller document app. This
distinction matters: WordCraft has many Word-compatibility features that a new app does not need.

## The one-picture mental model

Imagine a manuscript in a filing cabinet:

* The **document model** is the manuscript and its filing system. It knows the words, paragraphs,
  formatting, tables, images, sections and cursor addresses. It should still make sense if nobody is
  looking at it.
* **Layout** is the typesetter. Given a manuscript and a page size, it measures text, chooses line
  breaks, assigns paragraphs to pages and records coordinates.
* A **renderer** is the painter. It turns those coordinates and drawing instructions into pixels or
  PDF commands.
* The **GUI** is the editor's room: menus, toolbars, mouse handling, scrolling, caret animation and
  keyboard input. It asks the model to change and asks layout where things are.

So layout is **not the GUI**. It is also not quite the renderer. The clean pipeline is:

```text
Document --layout--> DocLayout/Pages --display conversion--> Draw items --renderer--> pixels/PDF
    ^                       ^                                  ^
    |                       |                                  |
 editing commands       hit testing                       paint commands
```

In this codebase, the page-to-drawing step is explicitly described in
[`display.rs`](../crates/layout/src/display.rs:1), while the egui canvas consumes both
[`DocLayout`](../crates/layout/src/lib.rs:432) and display data in
[`canvas.rs`](../crates/ui-egui/src/canvas.rs:1). The PDF crate likewise calls
[`page_display`](../crates/layout/src/display.rs:361). These are useful places to verify the
boundaries.

## 1. Start with the crates, not the details

### What the manifests tell you

[`crates/doc/Cargo.toml`](../crates/doc/Cargo.toml:1) calls itself the document model and lists
`wordcraft-geom`, Serde, error handling, and Unicode segmentation as dependencies. Notice what is
absent: no egui, window library, font database, or layout crate. It can represent and edit a
document without opening a window.

[`crates/layout/Cargo.toml`](../crates/layout/Cargo.toml:1) depends on `wordcraft-doc`, geometry,
fonts, proofing and Unicode line/bidi helpers. That is the important direction of the arrow:
layout understands documents, but the document model does not understand layout.

**A useful rule:** make the lowest layer useful in a command-line program, test, importer or server.
If the model imports a GUI type merely to store a colour or cursor, the layering is probably
backwards.

### Read each root file as a map

Begin with the module declarations and public re-exports in
[`crates/doc/src/lib.rs`](../crates/doc/src/lib.rs:13). The root introduces the vocabulary, defines
the main `Document`, and exposes focused modules. Its opening comment gives the short version:
stories contain blocks; blocks are paragraphs or tables; positions identify a story, path and byte
offset ([`lib.rs`](../crates/doc/src/lib.rs:1)).

Then read the overview and module declarations in
[`crates/layout/src/lib.rs`](../crates/layout/src/lib.rs:1). It gives a very useful reading order:
paragraphs become lines, sections become pages, `display` makes draw items, and `hit` maps points
back to positions.

Do not try to read every module immediately. The model has Word-specific modules for equations,
fields, charts, SmartArt, WordArt, controls and drawing objects. They are real features, but they
are not prerequisites for understanding “a paragraph of text on a page.” A first pass can safely
postpone `chart`, `connector`, `effects`, `freeform`, `graphic`, `math*`, `smart_art` and
`wordart`.

## 2. What `wordcraft-doc` owns

The document crate owns **meaning and editable structure**:

* the text and its paragraph boundaries;
* character and paragraph properties;
* tables and their nested cell content;
* stories such as the body, headers, footers, notes, comments and text boxes;
* sections, styles, numbering, fields, revisions and media references;
* stable positions and range editing;
* serializable data and safe model-level validation.

It deliberately does **not** own:

* which font file supplies a glyph;
* the width of a particular page on a particular screen;
* line breaking, pagination or page coordinates;
* painting pixels, PDF paths or egui widgets;
* keyboard event interpretation, undo-stack policy or application commands.

Some application layers use the model's edit methods to implement those behaviours, but the model
does not need to know that a change came from a key press or a ribbon button.

## 3. The document tree: the core concepts

### `Document`, stories and sections

The central [`Document`](../crates/doc/src/lib.rs:563) contains a body block list plus a map of
secondary `parts`, a stylesheet, numbering, comments, revisions, settings, metadata and media
([`lib.rs`](../crates/doc/src/lib.rs:566)). `Document::new` creates one empty Normal paragraph,
which is a helpful invariant for a beginner to copy ([`Document::new`](../crates/doc/src/lib.rs:601)).

A **story** is a stream of blocks with one purpose. The main body is `StoryRef::Body`; a part is
identified by `StoryRef::Part(id)` ([`StoryRef`](../crates/doc/src/lib.rs:157)). A header is not
stored as a special paragraph field inside the body: it is a part that can be selected and laid out
where a section asks for it. Footnotes, comments and text boxes use the same general idea, although
their relationships to anchors differ.

A **section** is the page-and-flow policy for a region: paper size, margins, columns, orientation,
headers and footers, numbering and related choices. Section properties are attached to the final
paragraph of a section in [`Paragraph::section`](../crates/doc/src/para.rs:745), matching the
document format's convention. Layout reads these properties; the model stores them.

### Blocks, paragraphs and runs

At the next level, a story is `Blocks`, a vector of shared `Block` values. A block is either
`Block::Para(Paragraph)` or `Block::Table(Table)` ([`Block`](../crates/doc/src/lib.rs:74)).
This is a deliberately small core: paragraphs and tables are the things that can appear in normal
flow.

A [`Paragraph`](../crates/doc/src/para.rs:745) owns one `String`, paragraph properties, the
formatting of its paragraph mark, inline objects, optional section properties and a revision number.
The text can contain ordinary Unicode plus special markers: `\n` for a manual line break, form-feed
for a page break, and U+FFFC for an inline object ([`para.rs`](../crates/doc/src/para.rs:1)).

A **run** is not another string. It is a byte length plus `CharProps`; adjacent text with the same
properties can share one run ([`Run`](../crates/doc/src/para.rs:38)). The run lengths add up to the
paragraph string length. This representation makes “make this range bold” a change to formatting
ranges rather than a forest of one-character objects.

Be careful with offsets: they are **byte offsets**, but every editing offset must be a UTF-8
character boundary. [`Paragraph::insert_text`](../crates/doc/src/para.rs:975) checks this and
returns `DocError::BadOffset` instead of slicing invalidly. Unicode grapheme and complex-script
caret rules are additional detail in [`cluster.rs`](../crates/doc/src/cluster.rs:1); learn the
invariant first, then learn why one visible character can contain several code points.

### Inline objects

Pictures, fields, equations, note references, bookmarks, shapes and other inline things occupy one
U+FFFC marker in the paragraph string. The corresponding rich value lives in `Paragraph::objects`;
`object_at` finds it by counting markers ([`para.rs`](../crates/doc/src/para.rs:951)). This keeps
text positions in one coordinate system while allowing an object to carry a large structure.

For a small app, start with an enum such as `InlineObject::Image` or skip objects entirely. The
marker-plus-side-array pattern becomes useful when an object must behave like one caret-stop-sized
thing but still needs metadata.

### Tables are nested blocks, not a separate document universe

`Table` has a column grid and rows; each `Cell` contains its own `Blocks` ([`table.rs`](../crates/doc/src/table.rs:1)).
That means a cell can contain several paragraphs and even another table. A path such as
`[table, row, cell, block]` addresses the paragraph inside it. `Path` records this repeating nesting
pattern and compares lexicographically in document order ([`Path`](../crates/doc/src/lib.rs:166)).

This is Word-specific complexity built on a general tree idea. A beginner's app can choose a simpler
model—perhaps a list of blocks and tables whose cells contain only text—while keeping the principle
that nested content needs an address.

### Styles and properties

`CharProps` describes character-level choices such as font, size, bold, italic, colour, language
and hidden text ([`CharProps`](../crates/doc/src/props.rs:251)). `ParaProps` describes alignment,
indents, spacing, line spacing, keep rules, numbering, tabs and direction
([`ParaProps`](../crates/doc/src/props.rs:685)). `None` generally means “inherit,” not “turn off.”

The [`StyleSheet`](../crates/doc/src/styles.rs:135) holds document defaults and named styles.
Styles may be based on other styles. The model stores these declarations; the resolver computes the
effective result in [`resolve_para`](../crates/doc/src/resolve.rs:297) and
[`resolve_char`](../crates/doc/src/resolve.rs:330). This separation is important: an edit can say
“this paragraph uses Heading1,” while layout later asks “what font, spacing and colour does that
mean here?”

For a first pass, remember the layering order: document defaults → named style chain → table style
when relevant → direct paragraph/run formatting. Leave the many compatibility fields alone until a
test or feature requires one.

### Positions and selections

The document's address system is [`Pos`](../crates/doc/src/lib.rs:212): a story, a paragraph
`Path`, and a byte offset. A selection is usually an engine-level pair of positions, but the model
operations accept endpoints such as `Document::delete_range`.

This is analogous to a database key, not a screen coordinate. It remains meaningful if the window is
resized or the paragraph moves to a different page. Layout later maps it to a caret rectangle.

### Editing primitives

The model exposes small, reusable operations in [`edit.rs`](../crates/doc/src/edit.rs:1): insert
text or an object, split a paragraph, insert/remove blocks, delete a range, format a range and copy
or paste a `Fragment`. `Document::insert_text` delegates to the paragraph and returns the next
position ([`edit.rs`](../crates/doc/src/edit.rs:242)). Newline inside inserted text is a manual
line break; a paragraph break is an explicit `split_paragraph` operation.

This is a useful boundary. A keyboard command can interpret Enter, Backspace and selection rules,
then call these primitives. Clipboard code can use `Fragment`. Undo can snapshot the document before
the primitive and restore it after—without making the document crate know what an undo button is.

### Ownership and cheap snapshots

`Blocks` is `Vec<Arc<Block>>` ([`Blocks`](../crates/doc/src/lib.rs:98)). Cloning a document initially
shares unchanged blocks. When an edit needs to change a shared block, `Arc::make_mut` creates a
private copy of only that block before mutation, as seen while descending into table cells in
[`edit.rs`](../crates/doc/src/edit.rs:70).

The intuitive analogy is photocopy-on-write: two draft snapshots can point at the same unchanged
pages; writing on one page photocopies that page, not the whole filing cabinet. This helps undo
snapshots and lets a renderer/layout pass inspect a stable clone. It does not make arbitrary mutation
thread-safe, and it does not mean every value is shared—`String`, runs and rows are ordinary owned
values inside a block.

The paragraph `rev` counter is a separate, lightweight change signal
([`next_rev`](../crates/doc/src/para.rs:31)). It is not document content and is skipped by equality
and serialization. Layout uses it to decide whether a cached paragraph measurement can be reused.

## 4. What layout translates

The public entry point is [`layout`](../crates/layout/src/lib.rs:1705): it accepts `&Document`, a
mutable [`LayoutCache`](../crates/layout/src/lib.rs:460), and [`LayoutOptions`](../crates/layout/src/lib.rs:139),
and returns a [`DocLayout`](../crates/layout/src/lib.rs:432). The input is borrowed, not consumed:
layout is a read-only calculation from the model's point of view.

### Paragraph layout: words become measurable lines

[`layout_para`](../crates/layout/src/para.rs:1134) resolves styles, chooses faces through the font
layer, shapes text into glyphs, groups it into grapheme/object clusters, applies bidi ordering,
places tabs and list labels, and breaks the result into lines. `Cluster` keeps the source byte range,
advance width, kind, glyph range and whether a break is allowed
([`Cluster`](../crates/layout/src/para.rs:103)). `Line` stores top, height, baseline, cluster range,
x positions and byte range ([`Line`](../crates/layout/src/para.rs:136)).

Think of this as turning “Hello world” into a ruler diagram, not drawing it yet. Shaping answers
which glyphs and advances a font needs; line breaking answers which clusters fit; alignment and
line spacing answer where the line sits in its paragraph box. Bidi, CJK rules, hyphenation, fields,
lists and inline objects explain why simply measuring `text.len()` is not enough.

`ParaLayout` is temporary derived data. It is not written back into the document. The cache key
includes the paragraph revision and layout environment such as width, styles, numbering, grid and
visibility settings ([`Key`](../crates/layout/src/lib.rs:444)).

### Sections, tables and pagination

The outer layout pass takes paragraph results and places them into columns and pages. It accounts for
margins, section starts, headers and footers, keep/widow rules, page and column breaks, floating
objects, tables, footnotes and page-dependent fields. `Page` contains dimensions, a body rectangle,
header/footer placement and a list of [`Placed`](../crates/layout/src/lib.rs:194) items
([`Page`](../crates/layout/src/lib.rs:409)).

`Placed::Lines` points back to the story/path and a shared `ParaLayout`; other variants represent
fills, rules, images, shapes, graphics, table cells and selectable objects. These are still not
pixels. They are page-coordinate facts that several consumers can use.

Some values require more than one pass. For example, `NUMPAGES` cannot know its final value until
pagination is complete, and per-page footnote numbering can affect page space. The bounded retry
logic is visible in [`layout`](../crates/layout/src/lib.rs:1705). This is an example of Word
compatibility complexity, not a requirement for a small note-taking app.

### Hit testing and caret mapping

The inverse of layout is in [`hit.rs`](../crates/layout/src/hit.rs:1). `DocLayout::hit` turns a
page point into the nearest document `Pos` ([`DocLayout::hit`](../crates/layout/src/hit.rs:236));
`caret_on` (in the same module) turns a document position into a [`Caret`](../crates/layout/src/hit.rs:11)
rectangle. Selection rectangles, visual left/right movement, object hits and turned table text use
the same line geometry.

This is why a click can put the caret between two Unicode clusters even though the GUI knows nothing
about font shaping. The GUI supplies a page point; layout performs the document-aware translation.

### Display data and the actual painter

[`page_display`](../crates/layout/src/display.rs:361) converts one laid-out page into `Draw`
items: glyph runs with font faces and positions, fills, lines, images, shapes, paths, marks and
transforms ([`Draw`](../crates/layout/src/display.rs:24)). A raster renderer or PDF exporter consumes
those items. The display module therefore is closer to a renderer adapter than to a GUI: it knows
what should be drawn, but not how egui schedules frames, handles buttons or scrolls.

## 5. Two end-to-end traces

### Example A: typing, then seeing the text

1. The user interface receives text input and an engine command decides the insertion position and
   active formatting. The GUI is not expected to mutate a screen string separately.
2. The command calls `Document::insert_text` with a [`Pos`](../crates/doc/src/lib.rs:212), text and
   [`CharProps`](../crates/doc/src/props.rs:251). The paragraph inserts UTF-8 text, inserts or
   merges a formatting run, updates `rev`, and returns the position after the inserted bytes
   ([`Paragraph::insert_text`](../crates/doc/src/para.rs:975)).
3. The next layout call sees the changed revision. `layout_para` resolves the run's effective
   formatting, asks fonts to shape it, makes clusters and lines, and the outer pass places those
   lines in a page.
4. The canvas asks `caret_on` for the new position and draws the caret itself. It asks display/
   rendering code to paint the page's glyph draw items. Layout supplied coordinates; egui supplied
   interaction and the actual on-screen widget.

### Example B: changing paragraph spacing

1. A formatting command changes `Paragraph::props.line_spacing` or `space_after` in the model. A
   style command might instead change a named style in `StyleSheet`.
2. The model remains text plus properties; it does not calculate a new y-coordinate.
3. Layout resolves the properties while building `ParaLayout`, changes line heights or the gap after
   the paragraph, then repaginates subsequent content if the page no longer fits.
4. The display list and caret coordinates now reflect the new result. The GUI can repaint without
   knowing whether the cause was direct formatting or a style-chain change.

This separation also explains why a layout cache must be invalidated when either paragraph content,
styles, numbering, page settings or relevant view options change.

## 6. A sensible file reading route

Read in this order, taking notes in your own words:

1. [`crates/doc/Cargo.toml`](../crates/doc/Cargo.toml:1): What is this crate allowed to depend on?
2. [`crates/doc/src/lib.rs`](../crates/doc/src/lib.rs:1): Find `Document`, `Block`, `Blocks`,
   `StoryRef`, `Path`, `Pos`, and `DocError`. Ask: what must always be valid?
3. [`crates/doc/src/para.rs`](../crates/doc/src/para.rs:745): Read `Paragraph`, `Run`,
   `insert_text`, `delete`, `split_off`, and boundary helpers. Ask: which fields change when a
   character is inserted?
4. [`crates/doc/src/props.rs`](../crates/doc/src/props.rs:251): Skim `CharProps` and `ParaProps`.
   Ask: which properties change meaning, and which can wait?
5. [`crates/doc/src/styles.rs`](../crates/doc/src/styles.rs:135) and
   [`crates/doc/src/resolve.rs`](../crates/doc/src/resolve.rs:272): Follow one style from its id
   to its resolved font and spacing. Ask: where does inheritance stop?
6. [`crates/doc/src/table.rs`](../crates/doc/src/table.rs:10): Follow a cell's blocks. Then inspect
   `container` and `container_mut` in [`lib.rs`](../crates/doc/src/lib.rs:679). Ask: how would a
   path reach the second paragraph of the first cell?
7. [`crates/doc/src/edit.rs`](../crates/doc/src/edit.rs:142): Trace insert, split, delete and
   format. Ask: what should an edit return so the caller can keep the caret correct?
8. [`crates/doc/src/tests.rs`](../crates/doc/src/tests.rs:3): Read `new_document_is_valid`,
   `clone_is_shallow`, `json_round_trip` and section/table tests. Ask: which invariants are promises?
9. [`crates/layout/Cargo.toml`](../crates/layout/Cargo.toml:1) and
   [`crates/layout/src/lib.rs`](../crates/layout/src/lib.rs:1): Identify `Page`, `Placed`,
   `DocLayout`, `LayoutCache` and `layout` before reading algorithms.
10. [`crates/layout/src/para.rs`](../crates/layout/src/para.rs:65): Follow `Cluster`, `Line`,
    `ParaLayout`, `ParaEnv` and `layout_para`. Ask: how does each output retain a link to source
    byte ranges?
11. [`crates/layout/src/table.rs`](../crates/layout/src/table.rs:13) and the page-building portion
    of [`lib.rs`](../crates/layout/src/lib.rs:1493): See how table measurements and page flow use
    the model. Ask: which geometry is temporary?
12. [`crates/layout/src/hit.rs`](../crates/layout/src/hit.rs:236) and
    [`crates/layout/src/display.rs`](../crates/layout/src/display.rs:361): Read the two outward
    translations—page point to document position, and page layout to draw instructions.
13. Finally compare the consumers: [`crates/ui-egui/src/canvas.rs`](../crates/ui-egui/src/canvas.rs:1)
    and [`crates/pdf/src/lib.rs`](../crates/pdf/src/lib.rs:1). Ask: what does each consumer add that
    layout intentionally omits?

On the first pass, skip the implementation details of bidi, hyphenation, Word fields, revision
balloons, rotated shapes and text-box recursion. Return to them when a test or feature points there.

## 7. What the tests teach

The document tests are executable statements about model behaviour. In
[`tests.rs`](../crates/doc/src/tests.rs:3), a new document has one valid paragraph, cloning shares
unchanged blocks, edits detach only touched blocks, JSON round-tripping preserves content and
formatting, and sections/parts/media have stable behaviour. These are better beginner lessons than
abstract ownership diagrams: make a clone, edit it, and observe which facts changed.

The layout tests reveal that layout is a translation contract, not merely a “draw some text” helper.
The basic tests cover wrapping, pagination, hit/caret agreement, alignment, tabs, lists and tables
([`crates/layout/src/tests.rs`](../crates/layout/src/tests.rs:15)). Other test modules pin bidi and
complex-script clusters ([`tests_bidi.rs`](../crates/layout/src/tests_bidi.rs:59) and
[`tests_complex.rs`](../crates/layout/src/tests_complex.rs:30)), page/header/footnote behaviour
([`tests_pages.rs`](../crates/layout/src/tests_pages.rs:36)), and vertical or text-box layout
([`tests_vertical.rs`](../crates/layout/src/tests_vertical.rs:31),
[`tests_textbox.rs`](../crates/layout/src/tests_textbox.rs:52)). Hostile-input tests are especially
instructive: a document app must not crash just because a file contains extreme dimensions,
malformed direction data or recursive-looking text boxes.

For a smaller app, begin with tests like `empty_doc_has_one_page`, `long_paragraph_wraps_and_paginates`
and `hit_and_caret_agree`. Add a regression test whenever an edit, line-break rule or hit-test fix
changes intended behaviour.

## 8. Borrowing the pattern for a smaller app

Do not copy WordCraft's entire feature surface. Borrow the boundary:

1. Define an owned `Document` with a list of blocks.
2. Make a paragraph contain text plus simple runs, or begin with one style for all text.
3. Define positions in document coordinates, not screen pixels.
4. Put edits in model functions that return `Result` and updated positions.
5. Write a layout function that borrows the document and returns lines/pages with source ranges.
6. Add hit testing as the inverse of those same line records.
7. Let a renderer consume layout output, and let a GUI call edits and hit testing.

You can initially omit stories, tables, styles, floats, bidi, fields, pagination retries and
copy-on-write. Add each only when its user-visible behaviour justifies the model complexity. The
architecture scales because the dependency direction stays stable: model first, layout second,
renderer/GUI last.

## Glossary

* **Block:** A top-level flow item; in this model, a paragraph or table.
* **Byte offset:** A position in a UTF-8 string measured in bytes, only valid at a character
  boundary.
* **Caret:** The insertion cursor, represented in the model by a `Pos` and in layout by a page
  rectangle.
* **Cluster:** A smallest layout unit, usually a grapheme, but also a tab, break or inline object.
* **Document model:** Persistent meaning and structure: what the document is.
* **Layout:** A derived calculation of line and page geometry: where the document fits.
* **Placed item:** A page-coordinate record such as lines, a cell, an image or a shape.
* **Run:** A byte range whose characters share `CharProps`.
* **Section:** A region's page, column, margin, header/footer and flow settings.
* **Story:** A block stream with a role, such as body, header, footnote or text box.
* **Style:** A named, inheritable bundle of paragraph or character properties.
* **Shaping:** Converting characters and formatting into font-specific glyphs and advances.
* **Hit testing:** Converting a page point back into a document position.
* **Copy-on-write:** Sharing unchanged data until one owner edits it, then copying only the changed
  part.

The core lesson is simple: store the document in a stable, screen-independent form; derive layout
from it; and keep the GUI as a consumer of both rather than making the GUI the owner of document
meaning.

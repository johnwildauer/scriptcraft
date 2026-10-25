# A ground-up project structure for a dependable screenplay MVP

## Purpose and assumptions

This document is a build proposal, not production code and not a general screenplay-format
tutorial. It answers a narrower question: **what is the smallest coherent Rust application that can
let one writer create, edit, paginate, save, reopen, and export a screenplay without losing work?**

The proposal is informed by the nearby research, especially the [document and layout guide](05-doc-and-layout-guide.md),
the [Rust codebase guide](04-rust-codebase-guide.md), the [screenplay architecture synthesis](03-synthesis-screenplay-architecture.md),
and the Stage 2 notes on editing, layout, persistence, and testing. It also uses the current
repository's [geometry crate](../crates/geom/src/lib.rs:1), [document crate](../crates/doc/src/lib.rs:1),
[layout crate](../crates/layout/src/lib.rs:1), [render crate](../crates/render/src/lib.rs:1), and
[architecture summary](../docs/architecture.md:9) as references.

The intended reader is a Rust beginner. The calm rule throughout is: start with a small number of
plain types, keep each type's job visible, and add a layer only when a user-visible requirement
needs it.

### Assumptions

* The first product is a single-user editor, initially on desktop.
* The native project is a UTF-8 JSON file owned by this application.
* A screenplay is an ordered sequence of semantic text elements, not a picture of pages.
* One fixed page profile is enough to validate editing and pagination.
* A simple system or bundled open font is acceptable for the first layout implementation.
* The first UI can be plain and focused: editor, role control, page preview, file actions, and
  an error/status area.
* All important behavior can be driven from a headless session, even if the first visible app is
  desktop-only.

These choices favor understandable code and recoverability over broad compatibility. They are not a
claim that every screenplay convention is simple or that every professional workflow uses one page
profile.

## 1. Product boundary

### The MVP promise

The first release should allow a writer to:

1. create a blank project with title-page information;
2. enter and edit screenplay elements with explicit roles;
3. move a caret and selection through Unicode text safely;
4. split, join, delete, undo, and redo elements;
5. see continuous writing mode and a deterministic page preview;
6. save a versioned native project atomically and reopen it;
7. recover a recent bounded snapshot after an interrupted session; and
8. export a paginated PDF and a deliberately lossy plain-text copy.

The promise is intentionally smaller than “replace a professional screenwriting suite.” A feature
belongs in the MVP only if it protects the writing loop, preserves semantic screenplay meaning, or
is required to inspect the result on pages.

### Explicit non-goals

Do not put these in the first product boundary:

* Word or DOCX compatibility as a native model;
* FDX import/export, even though it is a good later screenplay-specific adapter;
* tables, cells, arbitrary nested stories, headers, footers, footnotes, fields, equations, shapes,
  images, comments, tracked changes, collaboration, or revision comparison;
* a generic style system or a large formatting gallery;
* screenplay analysis, production breakdowns, scheduling, budgeting, or character databases;
* automatic scene-numbering workflows, dual dialogue, revisions, colored pages, or locked pages;
* spelling, grammar, cloud sync, plugins, MCP, remote control, or a command catalog;
* multiple page profiles, columns, facing pages, or fully configurable paper standards;
* incremental pagination before a long-script measurement demonstrates the need;
* pixel-perfect parity with any existing editor; or
* silently guessing a role from capitalization, spaces, tabs, or visual indentation.

Plain text and PDF are exports, not alternate sources of truth. If an export loses roles or IDs,
the app should say so in its API/result rather than pretending the round trip is lossless.

## 2. A simple mental model

Think of the project as an ordered manuscript with two small zones:

```text
Project
├── title-page metadata and title-page elements
├── screenplay elements, in writing order
└── page profile
```

Each screenplay element has one meaning and one editable text value. Its role points to rules in the
project's page profile; it does not carry a copy of every visual or pagination rule:

```text
Element = stable ID + role + canonical text
```

The arrows between the durable and temporary worlds are:

```text
saved Project --edit--> changed Project --layout--> temporary Pages --render--> pixels/PDF
                         ^             ^             ^
                         |             |             |
                    Selection       Session       caret geometry
```

The project does not contain page coordinates. Layout calculates them from the project and page
profile. The editor session does not become part of the saved screenplay. This is the same valuable
separation illustrated by the current [document-to-layout pipeline](../analysis/05-doc-and-layout-guide.md:13)
and the current [layout-to-display boundary](../crates/layout/src/lib.rs:194).

### Why not call the sequence a “Story”?

The current Word-oriented model uses “story” for body text, headers, footnotes, and text boxes; see
its [StoryRef type](../crates/doc/src/lib.rs:157). That is useful for a general word processor but
too vague here. A screenplay app should use **title-page elements**, **screenplay elements**, and
perhaps **element sequence**. Those names tell a beginner what the data is for and avoid inviting a
generic hierarchy before one is needed.

## 3. Roles: analyze the seven-ish kinds instead of assuming them

The word “role” describes semantic intent. It is not a font style. The proposed taxonomy starts with
seven screenplay writing roles, then treats title-page content separately because it follows a
different layout policy.

### MVP screenplay roles

| Role | Meaning | Typical MVP behavior |
|---|---|---|
| `scene_heading` | A new scene or location/time heading | Main measure, strong keep-with-next relationship |
| `action` | Descriptive prose and visible behavior | Main measure, ordinary wrapping |
| `character` | The speaker cue above dialogue | Narrower measure, keep with following dialogue |
| `dialogue` | Spoken words | Narrow measure, may split across pages |
| `parenthetical` | Short direction attached to dialogue | Narrow dialogue measure, keep with nearby dialogue |
| `transition` | A cut or transition cue | Right-aligned or profile-defined measure |
| `note` | Authored note that is intentionally part of the file | Visually distinct, included in native save |

These seven are enough to make role-aware typing and pagination meaningful. `note` is included only
because some writers need an authored, portable note inside a draft. It is not a comment or review
system; comments are out of scope.

### Title-page content

Do not force title-page lines into screenplay roles. Use a small explicit `TitlePageElement` with a
`kind` such as `title`, `credit`, `author`, `contact`, or `date`, plus text and a position hint such
as `center` or `bottom`. The MVP can expose only title, author, and contact fields in the UI while
allowing the serialized list to remain clear.

This avoids two mistakes: treating a title as an action paragraph, and adding a general-purpose
layout engine just to center a few title-page lines.

### Later roles, and why they wait

* **Section heading** can be useful for act or sequence labels, but it needs an outline policy and
  perhaps a numbering policy. Add it after basic navigation works.
* **Shot** can distinguish camera directions from ordinary action, but it expands screenplay
  semantics without being required to write a first draft. It can initially be `action` or `note`
  with an explicit warning on import/export.
* **Centered text** is a presentation instruction, not a strong screenplay meaning. It can be a
  later layout flag or a title-page hint, not a core role.
* **Dual dialogue**, **lyrics**, **montage**, **scene numbers**, and **revision marks** deserve
  explicit future design rather than being smuggled into `note`.

The principle is to add a role when it changes editing transitions, navigation, persistence, or
pagination. A different font alone is not enough.

### A role is interpreted through the project profile, not an element override

`Role` is a semantic label, not a font style and not a bag of settings attached to each element.
`scene_heading` means “this is a scene heading”; the current project profile says how scene headings
are normally laid out. The role definitions belong in `PageProfile`, or in a screenplay-format
configuration held by the project. They can carry rules such as:

* keep-with-next behavior and, where useful, keep-with-previous behavior;
* allowed or typical next roles, for example `character` followed by `dialogue`;
* capitalization policy;
* alignment;
* font family and size;
* margins and indents;
* line spacing;
* page-break behavior; and
* whether the role is available on the title page, script pages, or both.

This makes the distinction between a **role definition/profile** and an **element instance** clear:

* The role definition is reusable project configuration. It describes the normal behavior of every
  element with that role.
* The element instance stores its stable ID, semantic role, and content. It does not routinely store
  layout fields such as font, indent, capitalization, spacing, keep behavior, or page behavior.

For example, ten `action` elements should not each store the same font, indent, line spacing, and
keep policy. Duplicating those defaults makes files noisy, makes a format change difficult, and can
leave two elements that claim to have the same role rendering differently for no intentional reason.
The profile is the source of those defaults. This keeps the model small and makes a format change a
single project-setting change rather than a rewrite of every element.

`PageProfile` is a reasonable home because these rules affect pagination and are part of the named
screenplay format. It should not become a dumping ground for every document behavior, though. Keep
editing history, UI state, collaboration state, and unrelated semantic policies out of it. A clean
alternative is `PageProfile { role_rules: RoleRules }`, or a separate `ScreenplayFormat` referenced
by the project when the format grows beyond page geometry and role defaults.

For the MVP, the resolution order is deliberately short:

```text
format defaults → project/page-profile role rules → layout constraints
```

The layout engine resolves the element's role against the current profile each time it lays out the
project. A later CLI or UI settings command can edit that profile and change the defaults for the
whole project; ordinary elements do not need to be visited. A deliberate exception may be worth
adding later, but it should be a clearly named feature such as a manual page-break annotation, not an
`overrides` object repeated in every element ID block. Defer that feature until the normal profile
rules work and there is a demonstrated need.

Here is an intuitive, abbreviated JSON example:

```json
{
  "page_profile": {
    "name": "letter-screenplay-v1",
    "role_rules": {
      "scene_heading": {
        "keep_with_next": true,
        "typical_next_roles": ["action", "character"],
        "capitalization": "upper",
        "alignment": "left",
        "font": { "family": "Screenplay Sans", "size_pt": 12.0 },
        "margins_pt": { "left": 0.0, "right": 0.0 },
        "line_spacing_pt": 14.0,
        "page_break": "avoid_before_when_possible",
        "available_on": "script_pages"
      },
      "action": {
        "keep_with_next": false,
        "typical_next_roles": ["action", "character", "transition"],
        "capitalization": "preserve",
        "alignment": "left",
        "font": { "family": "Screenplay Sans", "size_pt": 12.0 },
        "margins_pt": { "left": 0.0, "right": 0.0 },
        "line_spacing_pt": 14.0,
        "page_break": "normal",
        "available_on": "script_pages"
      }
    }
  },
  "elements": [
    {
      "id": "element-01",
      "role": "scene_heading",
      "text": "INT. KITCHEN - NIGHT"
    },
    {
      "id": "element-02",
      "role": "action",
      "text": "The porch light flickers."
    }
  ]
}
```

Both elements receive their layout from the `page_profile` role rules. A future manual page break
would be an explicit, separately designed annotation; it is intentionally not part of this MVP
example.

Some rules are not properties of one element at all. `keep_with_next` describes a relationship
between an element and its neighbor, and `keep_with_previous` likewise looks backward. The profile
stores the role defaults; it should not put a resolved boolean on every element. During layout,
adjacent elements are inspected, role rules are resolved, and the result is turned into constraints
such as “place this heading with the first line of the next element when they fit.” This keeps the
model semantic and lets layout handle the cases where a rule must yield to an oversized element, a
future manual break annotation, or the end of the document.

## 4. Proposed serializable domain model

The native file should be a stable, human-readable JSON envelope. The exact Rust spellings can
change; the shape and invariants should be decided first. A v1 model could be described as:

```text
Project
  format: "screenplay-project"
  schema_version: 1
  id: ProjectId
  metadata: ProjectMetadata
  title_page: TitlePage
  page_profile: PageProfile
  elements: ordered list of ScreenplayElement

ScreenplayElement
  id: ElementId
  role: Role
  text: canonical UTF-8 string
  runs: deferred ordered byte ranges over text

RoleRules
  rules for each role, normally held by PageProfile or ScreenplayFormat

TextRun
  start/end byte range + optional character-style overrides; never a second text store
```

### Example JSON

This is illustrative data, not a required file extension or a promise that every field is editable
in the first UI. To keep the example readable, the full project below shows the page geometry and
base metrics but omits the repeated `role_rules` entries; the abbreviated example above shows their
shape.

```json
{
  "format": "screenplay-project",
  "schema_version": 1,
  "id": "project-01J9M4Y4H2K8D7A5Q6R3T1V0XZ",
  "metadata": {
    "title": "The Last Porch Light",
    "author": "A. Writer",
    "contact": "writer@example.test",
    "created_utc": "2026-10-24T07:00:00Z",
    "modified_utc": "2026-10-24T07:15:00Z"
  },
  "title_page": {
    "elements": [
      { "id": "title-01", "kind": "title", "text": "The Last Porch Light", "placement": "center" },
      { "id": "title-02", "kind": "author", "text": "Written by A. Writer", "placement": "center" },
      { "id": "title-03", "kind": "contact", "text": "writer@example.test", "placement": "bottom" }
    ]
  },
  "page_profile": {
    "name": "letter-screenplay-v1",
    "paper_width_pt": 612.0,
    "paper_height_pt": 792.0,
    "margin_left_pt": 108.0,
    "margin_right_pt": 72.0,
    "margin_top_pt": 72.0,
    "margin_bottom_pt": 72.0,
    "font_family": "Screenplay Sans",
    "font_size_pt": 12.0,
    "line_height_pt": 14.0
  },
  "elements": [
    {
      "id": "element-01J9M4YJ1W7N4P0B3C2D6E8FG",
      "role": "scene_heading",
      "text": "INT. KITCHEN - NIGHT",
      "runs": []
    },
    {
      "id": "element-01J9M4Z2R6P8S1T4U7V0W3X5Y",
      "role": "action",
      "text": "The porch light flickers beyond the rain-streaked window.",
      "runs": []
    },
    {
      "id": "element-01J9M50C9K4L2M7N8P1Q5R6ST",
      "role": "character",
      "text": "MARA",
      "runs": []
    },
    {
      "id": "element-01J9M51D8F3G6H0J2K4L7N9PQ",
      "role": "dialogue",
      "text": "Someone is still waiting out there.",
      "runs": []
    }
  ]
}
```

### Why each durable field exists

* `format` prevents an arbitrary JSON file from being mistaken for this product's project.
* `schema_version` makes migrations explicit. A reader should reject a newer unsupported version,
  not guess.
* `id` identifies the project across saves and recovery snapshots.
* `metadata` supports the title page, file information, and future migrations without searching the
  screenplay text.
* `title_page.elements` keeps title-page semantics and placement separate from writing flow.
* `page_profile` records the page assumptions that affect pagination. It belongs in the project so
  reopening it produces the same result on another machine. Its `role_rules` provide reusable
  defaults, but it should remain focused on screenplay-format and page behavior rather than becoming
  a container for every document setting.
* `elements` is ordered because screenplay reading order matters.
* each element `id` survives insertion, deletion of neighboring elements, and reordering. It is
  not a page number and must not encode an array index.
* `role` carries meaning directly. The renderer must not infer it from capitals or spaces.
* `text` is canonical UTF-8 content. A paragraph break is represented by a separate element, not
  by an ambiguous newline convention inside one giant string.
* `runs`, when introduced, contains only inline formatting ranges over canonical `text`; it does
  not contain replacement text segments. Routine layout behavior is absent from the element: the
  current profile supplies the role's font, indents, spacing, capitalization, keep rules, and page
  behavior.

### What a `Run` means in WordCraft

A WordCraft [`Paragraph`](../crates/doc/src/para.rs:745) stores its text exactly once, in the
`text: String` field. Its [`Run`](../crates/doc/src/para.rs:38) list is not a second copy of that
text: each run stores a byte `len` and [`CharProps`](../crates/doc/src/props.rs:251), and the
successive lengths cover the paragraph's string. [`Paragraph::run_ranges()`](../crates/doc/src/para.rs:941)
turns those lengths into ranges such as `0..5` and `5..11`. Serialization therefore preserves
canonical text plus formatting spans, not competing text representations.

Both concepts are needed because paragraph-level meaning and character-level appearance are
different. [`ParaProps`](../crates/doc/src/props.rs:685) holds properties for the paragraph as a
whole (alignment, indents, spacing, keep/page behavior and similar rules), while runs allow only a
range to be bold, italic, use another font, or carry another language. For example, one dialogue
paragraph could have canonical text `She said, "走."`; a run over `0..10` could be plain English,
another over the quoted words could be italic, and another could carry a CJK font/language. Mixed
bold and italic words, a character name in a different font, a hyperlink, or text with different
proofing languages all fit in one paragraph. The ranges refer back to byte offsets in the one
canonical UTF-8 string; they are not independent segment strings. Edits split, shift, merge, and
normalize those runs, as shown by [`Paragraph::insert_text()`](../crates/doc/src/para.rs:975),
[`Paragraph::format()`](../crates/doc/src/para.rs:1120), and
[`Paragraph::split_off()`](../crates/doc/src/para.rs:1152).

The screenplay MVP should initially use only `text` and the role's project-profile rules. That is
not a different text model: it is the same canonical-text model with inline formatting deferred.
When inline emphasis becomes a real requirement, add ordered range records alongside `text`:

```json
{
  "id": "element-03",
  "role": "dialogue",
  "text": "Do not touch that.",
  "runs": [
    { "start": 0, "end": 8, "style": {} },
    { "start": 8, "end": 18, "style": { "italic": true } },
    { "start": 18, "end": 19, "style": {} }
  ]
}
```

Here `start` and `end` are UTF-8 byte offsets, matching the repository's position convention; the
records must be ordered, non-overlapping, and cover the text if runs are present. The MVP can omit
`runs` entirely (or use one plain range) until needed, but it must never make `text` and run text
segments look like alternative sources of truth. The role profile still controls paragraph-level
behavior for the whole element; range styles are a later, small inline layer on top of it.

### IDs and positions

Use opaque, unique IDs generated by the application. A time-sortable random ID is convenient, but
the model should only require uniqueness and valid serialization. Never use an array index as an
element identity: inserting a scene at the top would otherwise make every saved reference point at
the wrong element.

For the active editor position, use a structural value such as:

```text
Position = { element_id: ElementId, byte_offset: usize }
Selection = { anchor: Position, focus: Position }
```

Offsets are bytes inside a UTF-8 string, but every edit must first verify a character boundary. The
current repository demonstrates this invariant in [paragraph insertion](../crates/doc/src/para.rs:975)
and describes the reason in its [Unicode position guide](05-doc-and-layout-guide.md:139). Visual
cursor movement can later use grapheme clusters; the MVP must at least avoid slicing through a code
point.

### What is deliberately not serialized

Do not save these as required project content:

* caret, selection, scroll position, zoom, focused widget, or open panels;
* calculated line boxes, page coordinates, glyph IDs, raster images, or layout caches;
* undo/redo stacks and typing coalescing state;
* window size, theme, recent files, clipboard, font database, or operating-system paths;
* transient error messages or “currently exporting” flags.

These belong to session, UI preference, or recovery state. A recovery snapshot may include a model
copy and enough provenance to identify the source file, but it should still not turn caret geometry
into canonical content.

## 5. Four kinds of state

Keeping these categories visible prevents many design mistakes.

| Category | Examples | Saved in native project? | Owner |
|---|---|---:|---|
| Semantic content | roles, text, title metadata, element IDs | Yes | model |
| Formatting/layout rules | page profile, role indents, line height, keep policy | Project settings or code defaults | model/layout |
| Editor/session state | caret, selection, undo, dirty flag, view mode, layout cache | No | session |
| Persistence/recovery state | path, last successful save identity, snapshot sequence, recovery candidates | No, except provenance in recovery files | persistence/host |

The phrase “formatting/layout rules” needs care. A role's default indent is a rule in the layout
implementation; a project-specific page profile is durable input to that rule. A calculated x/y
coordinate is neither content nor a setting—it is derived output.

## 6. One core crate first, then split deliberately

### Initial recommendation

Start with one library crate, perhaps `screenplay-core`, and one small application package,
perhaps `screenplay-app`:

```text
screenplay/
├── Cargo.toml
├── core/
│   └── src/
│       ├── lib.rs
│       ├── error.rs
│       ├── units.rs
│       ├── model.rs
│       ├── position.rs
│       ├── edit.rs
│       ├── session.rs
│       ├── layout.rs
│       ├── display.rs
│       ├── persistence.rs
│       └── tests/
└── app/
    └── src/
        ├── main.rs
        ├── ui.rs
        ├── input.rs
        └── platform_files.rs
```

This is not a rejection of the repository's crate structure. It is a beginner-friendly starting
point: module boundaries are still real, but there is only one dependency edge to understand. The
core crate must not depend on a window toolkit. The app crate may depend on one.

### Starting with one crate does not mean starting without boundaries

It is reasonable to ask whether a more “professional” Rust project should create
`screenplay-model`, `screenplay-layout`, `screenplay-render`, and several other library crates
immediately. Usually, no. Multiple crates are useful tools, not a maturity badge. For this MVP,
the better default is one `screenplay-core` crate whose modules already have the shape of future
boundaries, plus the small application crate shown above.

That recommendation is not “put everything in one file” and it is not permission for every module
to call every other module. It means that Cargo does not have to enforce every boundary before the
domain is understood. The Rust module system can enforce a great deal on day one, while keeping the
first edit/build/test loop short.

#### What splitting a crate later actually involves

Splitting is manageable when the boundary is clean, but it is not a rename and it is not free. A
typical extraction looks something like this:

1. Move a group of modules, their helper types, and their tests into a new `src/` directory. Some
   `mod` declarations become `pub mod` declarations, and the new crate gets a `lib.rs` that defines
   its intended entry points.
2. Add a `Cargo.toml` for the new package and add it to the workspace if the project uses one.
   The original crate then adds a path dependency such as `screenplay-model = { path = "../model" }`
   (or the equivalent workspace dependency).
3. Change internal `use crate::model::...` paths to external-crate paths such as
   `use screenplay_model::...`, and make only the necessary types, traits, constructors, and error
   values public. Code that was previously allowed to see `pub(crate)` or private fields may need a
   method or a deliberately public value instead.
4. Choose a dependency direction. If `model` needs a layout type while `layout` needs model types,
   the extraction has a cycle. Cargo cannot accept that cycle, so the shared concept must move to a
   lower-level crate, become a trait/interface owned by one side, or be represented by a simpler
   input/output type.
5. Move unit tests with the implementation and move integration tests to the crate whose public
   behavior they exercise. Tests that reached into private fields need to be rewritten against a
   public-ish API rather than made dependent on more internals.
6. Preserve the old public API where it matters. A temporary re-export such as
   `pub use screenplay_model::{Project, Role};` can let the app and callers migrate gradually.
   Without re-exports or compatibility wrappers, the split can become a broad, noisy API change
   even when behavior has not changed.

There are also ordinary cleanup costs: documentation links and examples change, feature flags and
optional dependencies must be assigned to the right package, and Cargo may compile more package
boundaries during a first build. A clean extraction is a controlled refactor with tests protecting
behavior; it is not a switch that Cargo performs automatically.

#### Easy splits, difficult splits

Some boundaries are usually straightforward:

* **Geometry and units** are easy when they contain self-contained values such as points,
  rectangles, dimensions, and checked arithmetic. They should not know about projects or widgets.
* **A pure model** is relatively easy when it owns serializable domain values and validation but
  does not know about layout caches, files, or UI types.
* **A renderer or export adapter** is often easy when it consumes a stable display list and does not
  decide line breaks or mutate the model.

These work because the data flow is mostly one-way and the inputs and outputs can be named plainly.

Other boundaries are harder:

* **Editing and session** can be intertwined with positions, undo snapshots, errors, layout caches,
  and command behavior. Extracting one without accidentally exposing all of the others takes design.
* **Layout** becomes difficult if it reaches into model-private fields, owns model policy, or returns
  UI-specific painter objects. It is easier to extract when it accepts a valid model and returns
  backend-neutral pages and source mappings.
* **Persistence** becomes difficult if model types serialize themselves by reaching into files, or
  if the file format is treated as the in-memory model. A deliberate serialized envelope and a
  persistence service keep this boundary movable.
* **UI/app shell** is difficult only when lower layers have imported UI concepts. Once a window
  toolkit leaks into the model or layout, extraction requires replacing those types rather than
  simply moving files.

The hardest warning sign is a pair of modules that exchange private structs, call each other's
helpers, and both own parts of one invariant. That is not a good first crate boundary. First make
the invariant explicit; then decide where it belongs.

#### The costs on both sides

Creating many crates immediately has real costs for a beginner and for a small project:

* each change may require understanding a manifest, a package name, a dependency feature, and a
  public API rather than just a module;
* iteration is slower and error messages contain more package/context noise while the design is
  still changing;
* every cross-crate type becomes a public API decision, including error types, constructors,
  lifetimes, traits, and ownership choices that may not yet be settled;
* dependency versions and feature flags can become awkward, especially when two crates need
  incompatible versions or when a graphics/font dependency should be optional; and
* a beginner has more directories and `Cargo.toml` files to navigate before the writing loop is
  working.

The opposite extreme is also bad. One giant crate with no internal rules creates accidental
coupling: UI code starts editing model fields directly, persistence starts deciding layout policy,
and rendering starts depending on session state. Later extraction then exposes a web of private
assumptions, creates dependency cycles, and turns a simple move into a redesign. The answer is not
“many crates now”; it is “one crate with deliberate module boundaries now.”

#### Boundaries worth establishing on day one

Even while they remain modules, give these areas separate ownership and dependency direction:

```text
geometry/units → model → editing/session → layout → display/rendering
                         ↘ persistence
app/UI shell ─────────────→ session and display/layout results
```

The arrows describe permitted knowledge, not necessarily a literal module tree. The useful first
boundaries are:

* **geometry/units:** points, rectangles, dimensions, finite-value checks;
* **model:** `Project`, elements, roles, title-page data, profile data, IDs, and validation;
* **editing/session:** positions, selections, edit primitives, undo/redo, dirty state, and live
  transactions;
* **layout:** wrapping, page placement, source mappings, hit testing, and derived page records;
* **persistence:** JSON envelopes, schema migration, atomic save, open, and recovery snapshots;
* **rendering/display:** backend-neutral drawing records and consumers such as screen or PDF; and
* **UI/app shell:** windows, IME/input, clipboard, dialogs, scrolling, zoom, and platform files.

The likely first future extractions are `screenplay-model` (often together with a small
`screenplay-geom`) and `screenplay-layout`, because headless tests, a CLI, and more than one UI can
reuse them. `screenplay-render` is a good candidate when PDF and screen consumers have different
dependencies. Persistence may become `screenplay-io` when another host or format needs it, but it
does not have to be split merely because it has a file named `persistence.rs`.

Do not split `editing` and `session` prematurely if they still share transaction and selection
invariants. Do not create separate crates for every role, title-page type, command, or tiny helper.
Do not split rendering before there is a stable display list, and do not split persistence before
the native schema and migration responsibilities are understood.

#### Practical rules that pave the road

The following rules give a later split something solid to build on:

* UI imports nothing into `model` or `layout`; lower layers return data and typed errors, not widget
  handles, colors from a UI theme, or window coordinates.
* Persistence code does not live inside model types. Model validation can be a model concern, but
  opening paths, writing bytes, atomic replacement, and recovery rotation belong to persistence.
* Do not leak module-private data through convenience accessors. Expose operations and small value
  types that preserve invariants instead of making every field public “temporarily.”
* Treat serialization types deliberately. It is fine for a model type to derive serialization in
  the MVP, but keep the versioned envelope, migration code, and file I/O visibly separate from the
  semantic model. Introduce dedicated wire types when the format begins to diverge.
* Test public-ish boundaries: edit operations through positions and selections, layout through
  projects and page results, persistence through bytes or an injected file store, and rendering
  through display-list records. Tests that only inspect private implementation details make a split
  harder.
* Use a Cargo workspace when there are multiple packages, shared dependency policy, or a real
  reason to build/test packages together—not simply because workspaces look more substantial.

#### A concrete later migration

Suppose the initial crate has `core::units`, `core::model`, `core::edit`, `core::session`,
`core::layout`, `core::display`, and `core::persistence`. After the headless path is stable, a
reasonable migration is:

```text
screenplay-model   owns Project, Role, PageProfile, IDs, validation, and model errors
screenplay-layout  depends on screenplay-model and geometry; returns Pages and source mappings
screenplay-app     depends on both, plus session/edit/persistence initially
```

In practice, move `core::model` and perhaps `core::units` first. Add `screenplay-model/Cargo.toml`,
move its tests, update imports, and re-export `Project` and `Role` from the old core facade so the
app continues to compile. Then move `core::layout` to `screenplay-layout`; make its public input a
validated `&Project` and its public output backend-neutral. Finally, move the UI-facing composition
to `screenplay-app`, leaving editing/session and persistence together until their shared invariants
or reuse justify another split.

The result may eventually look like this:

```text
screenplay-model  ← screenplay-layout
        ↑                    ↑
screenplay-app ───────→ screenplay-render
        └──────────────→ screenplay-io
```

The exact graph can differ. The important point is that the initial modules already named the
concepts and kept their arrows one-way, so the extraction moves ownership rather than inventing an
architecture under deadline pressure. It is manageable if those boundaries stay clean, but it is
not free—and starting with multiple crates immediately would not have made the MVP automatically
more professional.

### When to split

Split the core only when there is a concrete benefit: independent reuse, a meaningful compile-time
boundary, separate contributors, or a dependency that should not reach model users. A later shape
could be:

```text
screenplay-geom  →  screenplay-model  →  screenplay-layout  → screenplay-render
                         ↑                    ↑                     ↑
                    screenplay-io       screenplay-engine       screenplay-app
```

Possible future crates are `screenplay-geom`, `screenplay-model`, `screenplay-layout`,
`screenplay-render`, `screenplay-engine`, and `screenplay-io`. Do not create all six on day one just
to resemble [WordCraft's current layer table](../docs/architecture.md:15). The current repository
has about 15 crates because its compatibility target is broad; a screenplay MVP has fewer jobs.

### Module responsibilities in the first crate

#### `units`

Own page measurements, points, rectangles, and finite-value checks. A tiny version can use points
and a `Rect`; it need not support every conversion in [WordCraft's geometry module](../crates/geom/src/lib.rs:14).
Keep arithmetic checked or clamped when input came from JSON or UI settings.

#### `model`

Own project data, title-page elements, roles, page profile, IDs, and validation. It knows what a
scene heading means as data, but not where it lands on a page. It contains no UI types and no font
objects.

#### `position`

Own positions, selections, UTF-8 boundary checks, and safe movement helpers. It maps IDs to current
element indexes through the model; it does not store screen pixels.

#### `edit`

Own small pure mutations: insert text, delete range, split, join, change role, and create/remove
elements. Each operation returns a useful new position or selection and a typed error. It does not
know whether the caller is a key press or a menu item.

#### `layout`

Borrow a valid project and page profile, measure text, wrap lines, assign elements to pages, and
return source mappings plus rectangles. It should not mutate the model. The current layout crate's
separation between [paragraph layout](../crates/layout/src/para.rs:103) and [page placement](../crates/layout/src/lib.rs:194)
is a helpful pattern, but the MVP can use much simpler records.

#### `display`

Translate layout records into a small backend-neutral list such as text runs, lines, and rules. This
keeps a screen painter and a PDF writer from implementing pagination separately. It is analogous to
the current [page display conversion](../crates/layout/src/display.rs:361), without images and shapes.

#### `session`

Own the live project, selection, undo/redo, dirty state, revision, view mode, and a cached layout.
It is the transaction boundary. A failed mutation restores the project and session state.

#### `persistence`

Own JSON envelope validation, schema migration, deterministic serialization, atomic save, open, and
bounded recovery snapshots. It should accept abstract file operations in tests rather than reaching
into the desktop UI.

#### `render`

Initially this can be a small adapter in `display` or the app. It paints text and page rectangles;
it does not decide roles, line breaks, or page ownership. Split it when a PDF or image backend makes
the boundary valuable. The current [render crate](../crates/render/src/lib.rs:1) shows this consumer
role clearly.

#### `app`

Own the window, text/IME input, mouse coordinates, clipboard, dialogs, file pickers, scrolling,
zoom, and platform-specific paths. It translates input into session operations. A text widget must
not maintain a second authoritative screenplay string.

## 7. Smallest useful editing API

Deep command infrastructure is intentionally deferred. A tiny typed interface is still useful so
keyboard input, buttons, tests, and a future command palette call the same behavior.

```text
insert_text(selection, utf8_text) -> new_selection
delete_range(selection) -> new_selection
backspace(selection) -> new_selection
split_element(position, next_role) -> new_selection
join_with_previous(position) -> new_selection
set_role(element_id, role) -> changed
move_selection(selection, direction, unit) -> new_selection
undo() -> selection
redo() -> selection
new_project() -> project
open_project(bytes) -> project
save_project(project, destination) -> save_result
export_pdf(project, layout_options, destination) -> export_result
export_text(project) -> utf8_text
```

The API should return `Result`-like errors rather than panic. Parameters with untrusted lengths must
be bounded before allocation. Empty projects should have one valid editable element, probably an
empty `action`, so the caret always has a home.

### Enter behavior

Enter is a semantic operation, not merely insertion of a newline. The first implementation can use
an explicit transition table:

| Current role | Default new role |
|---|---|
| scene heading | action |
| character | dialogue |
| parenthetical | dialogue |
| dialogue | action or character, chosen by a simple explicit UI action |
| transition | scene heading |
| action | action |
| note | note |

Splitting first preserves the text before and after the caret. Choosing the next role should be one
undoable edit. The table belongs in editing/session logic, not in a button callback.

### Undo policy

Snapshot the model and selection before a successful mutating operation. Group nearby text insertions
into one typing undo step, but start a new step after navigation, role changes, split/join, save, or
an explicit pause. Ordinary owned clones are acceptable initially; copy-on-write like the current
[Arc-backed blocks](../analysis/04-rust-codebase-guide.md:231) can be added after measuring memory.

## 8. Layout and pagination for the MVP

### Fixed page profile

Choose one named profile, for example `letter-screenplay-v1`, with fixed paper dimensions, margins,
base font metrics, leading, and the role rules described above. Store its values in the project so a
reopen is reproducible. Role rules may refine the base metrics for a particular role, but they should
remain part of the screenplay-format configuration rather than being copied onto every element. Do
not start with a settings dialog for every value. A profile is easier to test than a collection of
unbounded user numbers.

The layout coordinate system should use points, with y increasing downward, like the repository's
[point and rectangle types](../crates/geom/src/lib.rs:130). Reject non-finite values and clamp
profiles to a sensible minimum printable area.

### Role-based boxes

For each element, resolve its role in the current project profile to a paragraph box before
wrapping. There are no routine per-element layout overrides in the MVP:

* scene heading and action: main text measure;
* character: narrower, indented measure and a layout constraint with the first dialogue line;
* dialogue: a narrower dialogue measure, with long content allowed to split;
* parenthetical: dialogue measure and a constraint with nearby dialogue;
* transition: role-profile alignment, usually toward the right;
* note: a visibly distinct but bounded box.

Never insert spaces or tabs to create these indents. The box carries the x coordinate; text remains
the same text when copied or exported.

### Wrapping

The first wrapper can break at whitespace and then fall back to character or grapheme-safe breaks
for a long unbroken token. It should return source ranges for each line, not only rendered strings.
That mapping is needed for caret rectangles, selection painting, and hit testing. A mature shaper
can later replace the measurement helper without changing the model.

### Page breaks and keep relationships

Interpret role relationships into layout constraints and implement these rules in order:

1. a future explicit manual page-break annotation, if that deferred feature is enabled;
2. natural line and element flow within the body rectangle;
3. scene heading keep-with-next;
4. character keep-with-first-dialogue-line;
5. parenthetical keep-with-nearby-dialogue where it fits;
6. a bounded widow/orphan rule for small blocks;
7. a safe escape hatch: if one element is taller than a page, split it rather than looping or
   making an unbounded blank page.

The rules should be compiled into a simple “does this group fit?” decision before placement. The
renderer should receive already placed lines. Rebuild all pages after each committed edit first;
cache only after a 120–180 page benchmark proves that full rebuild is a real problem.

### Title page

The title page is page zero with its own placement rules. It should not consume the first screenplay
body element. Render title-page elements using their placement hints, then start screenplay pagination
on the next page. If title-page fields are absent, render a clean blank page or make the behavior an
explicit project option; do not invent title text from the filename.

### Continuous writing mode and page preview

Both views use the same role metrics, wrapping, and source mappings:

* **continuous mode** lays elements one after another in a wide virtual flow and may show page-rule
  markers rather than page sheets;
* **page preview** uses fixed paper rectangles and actual pagination.

Continuous mode is an editing convenience, not a second layout truth. PDF always requests the fixed
print profile regardless of zoom or window width. Hit testing must ask layout for a document position,
following the current repository's [point-to-position boundary](../crates/layout/src/hit.rs:236).

### Implement now versus later

Implement now: fixed profile, role boxes, wrapping, source ranges, title page, page breaks, three
keep relationships, deterministic pagination, basic hit testing, and a display list.

Defer: font fallback databases, hyphenation dictionaries, bidirectional shaping, columns, floats,
widow/orphan sophistication, incremental pagination, print marks, scene numbering, and renderer
pixel caching. Add them only with a failing user requirement and tests.

## 9. Persistence, save, and recovery

### Native JSON

Serialize one canonical, pretty-printed UTF-8 JSON representation with stable field ordering where
the serializer permits it. On open, validate the envelope, cap input size and element count, reject
unknown future schema versions, and migrate known older versions explicitly.

### Atomic save

The save sequence is:

1. validate the in-memory project;
2. serialize it completely before touching the destination;
3. write a uniquely named sibling temporary file;
4. flush/sync where the host supports it;
5. replace the destination only after preparation succeeds; and
6. mark the session clean only after replacement succeeds.

This follows the useful contract of the repository's [safe-save implementation](../crates/engine/src/io/safe_save.rs:1),
but safe replacement is not the same as crash recovery.

### Recovery snapshots

Keep a separate bounded recovery directory. A snapshot needs the project ID, source path or source
identity when available, schema version, sequence number, timestamp, and model JSON. Write complete
snapshots under temporary names, rotate old valid snapshots, and never silently replace the user's
named project during recovery. On restart, offer the newest valid candidate and explain its age.

The MVP can take snapshots after a dirty idle period and before close. It does not need a journal
with individual keystrokes. Recovery is a candidate, not an invisible overwrite.

## 10. Build order: vertical slices with acceptance criteria

Each slice should leave the project in a runnable, testable state. Do not build a large model in
isolation and postpone the writing loop.

### Slice 1 — safe empty project

Build units, errors, project metadata, IDs, the seven-role enum, one empty action element, and JSON
validation. Add strict no-panic production lints.

**Accept when:** a new project validates, serializes deterministically, reloads with equal semantic
content, rejects unknown versions and malformed roles, and never allocates without bounds from an
untrusted count.

### Slice 2 — text and element editing

Build positions, selections, insertion, range deletion, Backspace, split, join, and role changes.
Use ordinary owned vectors and strings.

**Accept when:** Unicode text survives insert/delete at valid boundaries; invalid boundaries return
errors; splitting and joining preserve every code point and role metadata; an empty project remains
editable; and model tests run without a UI.

### Slice 3 — session and undo

Build a small session around the project and selection. Add dirty state, revision, bounded undo/redo,
typing grouping, and rollback on failed edits.

**Accept when:** keyboard-like sequences and direct API calls produce identical models; a failed edit
changes neither project nor selection/history; undo and redo restore exact semantic state; and role
transitions are tested as one undoable action.

### Slice 4 — first layout and hit testing

Build the fixed profile, role boxes, line wrapping, page records, source ranges, caret rectangles,
and basic point-to-position hit testing.

**Accept when:** fixtures produce stable page counts, all boxes stay within expected margins, lines
retain source ranges, long text wraps, and a caret round trip lands at a valid nearby position.

### Slice 5 — continuous editor shell

Build the smallest desktop UI: text input/IME path, role indicator, caret, selection, scrolling,
continuous mode, and visible errors. The UI calls session operations and never edits a parallel text
buffer.

**Accept when:** a user can type, press Enter, Backspace, change role, undo, redo, and resize without
losing semantic content; a recoverable error leaves the window usable.

### Slice 6 — page preview and title page

Add title-page rendering, fixed page sheets, page breaks, keep rules, page count, and a view toggle.

**Accept when:** the same project produces coherent continuous and page-preview views, scene headings
do not orphan from their next content when space permits, oversized dialogue splits safely, and the
title page does not consume a screenplay element.

### Slice 7 — native save, reopen, and recovery

Add open/save dialogs through a host boundary, atomic replacement, dirty semantics, and bounded
recovery snapshots.

**Accept when:** save failure preserves the last good file and dirty state; close/reopen preserves
roles, IDs, metadata, and visible pagination; an interrupted save cannot leave a partial canonical
file; and recovery offers the newest complete snapshot.

### Slice 8 — PDF and plain-text export

Add a display-list consumer for PDF and a clearly lossy text exporter. Keep export separate from the
native save path.

**Accept when:** PDF uses print layout rather than viewport layout, export does not mark the native
project clean, plain text reports its loss of roles, and a PDF failure leaves the session unchanged.

### Slice 9 — polish only after measurement

Measure long documents, IME behavior, font differences, and recovery timing. Only then decide whether
to split crates, add a better shaper, cache pagination, or introduce an interchange adapter.

## 11. Testing and safety habits to carry over

The goal is not to copy hundreds of repository tests. It is to copy the habits that protect writing.
The current examples worth studying are [JSON round trips](../crates/doc/src/tests.rs:104),
[failed-command rollback](../crates/engine/src/tests.rs:1845), [hostile parameter coverage](../crates/engine/src/tests.rs:1856),
and [deterministic page fixtures](../crates/layout/src/tests_pages.rs:36).

### Model tests

Test new-project invariants, unique IDs, ordered insertion/removal, split/join conservation,
selection clamping, role validity, and semantic JSON round trips. Add bounded property tests that
generate valid edit sequences, serialize, reload, and compare normalized projects.

### Unicode tests

Use accented characters, emoji, combining marks, and non-Latin scripts. Test that byte offsets at
valid boundaries work and offsets in the middle of a UTF-8 code point return an error. Later, add
grapheme-aware movement; do not claim byte movement is visually perfect.

### Malformed input and hostile sizes

Test missing fields, wrong JSON types, unknown enum strings, duplicate IDs, invalid page dimensions,
too many elements, very long text, and deeply nested or repeated recovery candidates. The expected
result is a bounded error, not a panic, hang, stack overflow, or enormous allocation.

### Session tests

For every mutation, test success, disabled/invalid input, rollback, dirty state, selection, undo,
redo, and typing coalescing. Keep filesystem and UI side effects out of pure session tests.

### Pagination tests

Prefer structural assertions: page count, element order, line source ranges, margins, no overlap,
forced breaks, keep relationships, oversized-block escape, title-page separation, and deterministic
repeated output. Add screenshot or PDF golden tests only after font and renderer choices are pinned.

### Persistence and no-panic production code

Use an injected temporary filesystem or test store to simulate partial writes, rename failures,
permissions, corrupt recovery files, and a full destination. Production code should use `Result` and
safe accessors rather than `unwrap`, `expect`, unchecked indexing, or unchecked arithmetic. A final
panic boundary may protect the UI, but it is not a substitute for validation. Every discovered crash
or data-loss defect gets a regression test.

## 12. Translation from WordCraft to the simplified app

| WordCraft concept/file/layer | Screenplay-app equivalent | Why it changes |
|---|---|---|
| [`geom`](../crates/geom/src/lib.rs:1) | `units` module, later `screenplay-geom` | Keep points, rectangles, and finite checks; omit OOXML conversion families until needed. |
| [`doc::Document`](../crates/doc/src/lib.rs:563) | `Project` | A project needs metadata, title page, one ordered element sequence, and a page profile. |
| `StoryRef` and secondary stories | Title-page zone plus screenplay element list | Avoid a generic hierarchy name and do not model headers, footnotes, or text boxes. |
| [`Block`](../crates/doc/src/lib.rs:74) | `ScreenplayElement` | Every MVP flow item is text with an explicit role; there are no tables. |
 | [`Paragraph` and `Run`](../crates/doc/src/para.rs:745) | Canonical element text plus deferred `TextRun` ranges | Preserve WordCraft's one-text-plus-spans arrangement without Word's broad properties. |
| [`Path` and `Pos`](../crates/doc/src/lib.rs:166) | `ElementId` plus UTF-8 byte offset | A flat sequence needs no table path; durable IDs survive reordering. |
| [`doc::edit`](../crates/doc/src/edit.rs:1) | `edit` module | Keep insert/split/join/delete primitives; remove objects, tables, and Word fragments. |
| [`engine::Session`](../crates/engine/src/session.rs:322) | Small `Session` | Keep selection, undo, dirty state, transaction, and layout cache; omit 422 commands. |
| [`CommandSpec`](../crates/engine/src/lib.rs:58) | Direct typed editing API | Defer registry/catalog analysis; preserve one path for UI and tests. |
| [`layout::DocLayout`](../crates/layout/src/lib.rs:432) | `Pages` and source-mapped lines | Keep page geometry and hit testing; remove tables, fields, shapes, notes, and sections. |
| [`layout::display`](../crates/layout/src/display.rs:361) | Small display list | Keep one page-to-drawing boundary for screen and PDF. |
| [`render`](../crates/render/src/lib.rs:1) | Tiny renderer adapter | Paint text and page geometry; no media cache or shape system. |
| [`engine::io`](../crates/engine/src/io.rs:1) | `persistence` plus host file service | One native JSON format first; explicit exports later. |
| [`safe_save`](../crates/engine/src/io/safe_save.rs:1) | Atomic native save | Retain sibling-temp replacement, but add separate recovery snapshots. |
| [`formats`](../crates/formats/src/lib.rs:1) | PDF/TXT exporters, then FDX adapter | Do not make a broad interchange crate define the semantic model. |
| [`ui-egui`](../crates/ui-egui/src/lib.rs:1) | Focused app UI | No ribbon, parity catalog, Backstage, MCP, or broad dialogs in the MVP. |

## 13. Recommended final MVP tree

The following is the target after the vertical slices, not a demand to create every file before
writing the first test:

```text
screenplay/
├── Cargo.toml
├── core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── error.rs
│       ├── units.rs
│       ├── model.rs
│       ├── role.rs
│       ├── title_page.rs
│       ├── position.rs
│       ├── edit.rs
│       ├── session.rs
│       ├── layout.rs
│       ├── display.rs
│       ├── persistence.rs
│       └── tests/
│           ├── model.rs
│           ├── editing.rs
│           ├── unicode.rs
│           ├── layout.rs
│           └── persistence.rs
└── app/
    ├── Cargo.toml
    └── src/
        ├── main.rs
        ├── ui.rs
        ├── input.rs
        ├── canvas.rs
        └── platform_files.rs
```

If a PDF library or font/shaping dependency makes the core too heavy, split only that boundary:
`screenplay-layout` and `screenplay-render` can become separate crates while `model`, `edit`,
`session`, and `persistence` remain together until their ownership is clear.

## Glossary

* **Canonical:** the authoritative representation; here, the native JSON project.
* **Caret:** the insertion cursor. Its document position is durable only during a session, while its
  rectangle on a page is derived.
* **Display list:** backend-neutral drawing instructions produced from layout.
* **Element:** one ordered screenplay text item with a semantic role.
* **Grapheme cluster:** a user-perceived character, which may contain multiple Unicode code points.
* **Hit testing:** converting a page point back to a document position.
* **Layout:** calculating line breaks, page membership, and coordinates from semantic content.
* **Metadata:** project information such as title, author, and timestamps, not screenplay prose.
* **Migration:** code that turns an older schema version into the current model.
* **Role:** semantic meaning such as dialogue or action. The current project profile supplies that
  role's layout and page rules; the element does not carry routine copies of them.
* **Run:** an ordered, contiguous range of the canonical text sharing optional inline formatting;
  in the proposed JSON it stores offsets and style, not another text segment. It is not a screenplay
  semantic role.
* **Session:** live editing state around a project, including selection, undo, and dirty state.
* **Source mapping:** a link from a rendered line or glyph range back to an element and text offset.
* **Transactional edit:** an edit that either completes fully or leaves model and session state
  unchanged.
* **UTF-8 byte offset:** an index into Rust's UTF-8 string bytes; valid edits must land on character
  boundaries.

## Final recommendation

Build one small core crate around a versioned `Project`, seven explicit screenplay roles with
project-owned profile rules, a separate title-page zone, and a flat ordered element sequence. Give
it safe edit primitives and a narrow session before adding a command registry. Derive all page
geometry and role defaults from one fixed profile, let future CLI/UI settings edit that profile for
the whole project, resolve adjacent-role relationships during layout, share the result between
continuous editing, page preview, and PDF, and keep native JSON separate from exports and recovery
snapshots. Keep screenplay elements to stable ID, semantic role, and canonical text at first; add
formatting ranges later only when inline emphasis is a real requirement.

The first meaningful milestone is not a polished window. It is a headless test that creates a
project, types a scene heading, transitions to action and dialogue, paginates deterministically,
undoes an edit, saves atomically, reopens the file, and finds the same roles, text, IDs, and pages.
Once that path is dependable, every additional feature has a clear question to answer: does it solve
a demonstrated screenplay-writing need without weakening the model, layout, or no-data-loss
guarantees?

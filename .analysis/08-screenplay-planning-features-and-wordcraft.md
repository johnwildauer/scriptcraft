# Screenplay planning features after the MVP: Beat Board, Outline Editor, and Structure Lines

## Purpose and scope

This is a post-MVP planning document. It does not enlarge the dependable screenplay MVP described in
[`analysis/06-screenplay-mvp-project-structure.md`](06-screenplay-mvp-project-structure.md). The MVP still comes first: a writer must be able to edit semantic screenplay elements, paginate them, save them safely, reopen them, and export a PDF.

The question here is narrower: once that writing loop is trustworthy, how could three planning features
often found in professional screenplay software be added without turning the screenplay editor into an
unbounded drawing application?

The three names are useful shorthand, not a complete specification. The requested capabilities are
described in neutral terms below and then compared with observed WordCraft mechanisms. **Observed fact**
means the repository supports the claim. **Recommendation** means a design for the future screenplay
application, not a claim that WordCraft already implements it.

## 1. The three capabilities in neutral terms

* **Beat Board:** a free-form, visual collection of planning cards. Each card has notes and optional
  colour/category information, can be placed on a board, and can be related to other cards. A card may
  point to one or more screenplay locations. The board is a planning surface, not another copy of the
  script.
* **Outline Editor:** an ordered, structured planning view. It groups or lanes cards into sequences or
  acts, allows reordering, and may assign goals such as a target page range. It should help a writer
  compare intended structure with the actual script.
* **Structure Lines:** visual guides connecting a planning item or structural marker to its location in
  the script or to a page-range projection. They are a view of relationships and layout, not content
  that should be positioned by hand in the manuscript.

These features overlap, but they solve different problems: the board supports spatial association, the
outline supports ordered structure, and structure lines support navigation and comparison.

## 2. WordCraft facts that matter

WordCraft already demonstrates several boundaries worth borrowing. Its document is structured rather
than a screenshot: [`Document`](../crates/doc/src/lib.rs:563) contains body blocks, styles, comments,
revisions, and media. A paragraph has canonical text and runs, while positions use a story, structural
path, and UTF-8 byte offset ([`Paragraph`](../crates/doc/src/para.rs:745), [`Pos`](../crates/doc/src/lib.rs:212)).
That is valuable for anchors, but paragraph paths are not durable identity: inserting or reordering
blocks can change their indexes.

The current document also has comments and secondary stories ([`Comment`](../crates/doc/src/lib.rs:314),
[`StoryRef`](../crates/doc/src/lib.rs:157)), range fields, bookmarks, and inline objects. They prove
that annotations and objects are possible, but they are Word-format mechanisms with assumptions that do
not automatically fit a screenplay planning model. In particular, an inline object is represented in
text by a U+FFFC placeholder; that is a poor identity for a board card that should remain meaningful
when text moves.

The layout boundary is more promising. [`DocLayout`](../crates/layout/src/lib.rs:432) returns pages and
an index from story/path to placed pieces. [`DocLayout::page_of()`](../crates/layout/src/hit.rs:703),
[`DocLayout::caret()`](../crates/layout/src/hit.rs:394), and [`DocLayout::hit()`](../crates/layout/src/hit.rs:238)
map between semantic positions and current geometry. Paragraph shaping retains source byte ranges, so
rendering and hit testing share one answer about where text is ([`ParaLayout`](../crates/layout/src/para.rs:303)).
The complete layout is cached by revision and view inputs in [`Session::layout()`](../crates/engine/src/session.rs:727),
while output gets a fresh print layout through [`Session::export_layout()`](../crates/engine/src/session.rs:770).

The command/session boundary is also directly reusable. [`Session::run()`](../crates/engine/src/session.rs:1116)
is the common path for UI, CLI, control, and automation; snapshots, rollback, dirty state, and undo are
centralized there. A drag can join repeated mutations into one undo step through
[`Session::join_next_undo()`](../crates/engine/src/session.rs:835). The UI already separates canvas
painting and hit testing ([`canvas::show()`](../crates/ui-egui/src/canvas.rs:680),
[`page_at()`](../crates/ui-egui/src/canvas.rs:1686)) from commands, and object dragging commits one
command on drop ([`objects`](../crates/ui-egui/src/objects.rs:134)).

Finally, WordCraft has an outline *view*, not a screenplay outline editor. Its outline state is
explicitly view state and is keyed by body block indexes ([`OutlineView`](../crates/layout/src/outline.rs:31)).
It collapses headings and lays them out on one long sheet. That is a useful interaction lesson, but not a
durable act/sequence/card model.

## 3. Cross-reference: Beat Board

| Requested behavior | Related WordCraft principle or mechanism | What transfers | What must be redesigned for a screenplay app | Likely new data/model |
|---|---|---|---|---|
| Create, edit, delete, and select planning cards | Structured document objects, command dispatch, and session selection | Cards should be edited through commands and included in transactional undo | A card is not a paragraph or U+FFFC inline object; it belongs to the project planning layer | `BeatId`, card title/body, category, colour, optional status, and script anchors |
| Place and resize cards freely | Canvas page coordinates, object frames, drag preview, and `join_next_undo` | Use screen coordinates only for the board viewport and commit a drag once | Page-relative floating objects are tied to document layout; board coordinates need a board coordinate system and bounds | `BoardPoint`/`BoardRect`, board ID, z-order or stable order, optional locked flag |
| Group cards visually or by category | Styles/tokens and Word outline grouping | Use semantic categories and theme tokens rather than scattered colours | Word styles describe text appearance, not an extensible planning taxonomy | `BeatCategory`, palette key, lane/group membership, optional tags |
| Connect cards with flow lines | Shapes/connectors and object selection | Reuse the idea of graph edges and endpoint hit testing | Word connectors are page objects and can be anchored to shapes; a board graph needs stable card IDs and cycle/duplicate policy | `BeatEdge { id, from, to, kind, label }` and a graph validator |
| Link a card to script content | Bookmarks, comments, fields, and structural positions | Use current positions to navigate and layout mapping to show links | Bookmarks and raw `Pos` do not provide a robust screenplay anchor after edits | `ScriptAnchor` using stable element/scene IDs plus a range policy |
| Attach optional reference images | `Document::media`, render media cache, injected file services | Reuse bounded loading, explicit services, and safe render fallbacks | A board reference is not automatically document content; untrusted image files must not become arbitrary embedded objects | Asset manifest with embedded or project-relative references, MIME/size checks, captions, alt text |
| Copy or convert a card into script text | Clipboard fragments and command-mediated paste | Conversion should produce a normal screenplay command and one undo step | There is no universal mapping from a note to role-aware screenplay elements | Explicit conversion command with a selected role, insertion target, and loss report |
| Export or print the board | Shared display-list idea and PDF renderer boundary | Board export can be deterministic and backend-neutral | Script PDF pagination is not the right coordinate system for an infinite/free-form board | `BoardDisplay`, bounded export region, SVG/PNG/PDF policy, deterministic ordering |

### Recommended Beat Board model

The board should be a project object, separate from screenplay text. A conceptual shape is:

```text
Project
├── screenplay elements: ordered semantic text, each with ElementId
├── planning
│   ├── boards: Board { id, name, cards, edges }
│   ├── cards: Beat { id, title, body, colour, category, rect, media, anchors }
│   └── edges: BeatEdge { id, from, to, kind, label }
└── page profile and project metadata
```

This is deliberately not a copy of the script. A beat may contain a short summary that differs from the
script, and a script scene may exist before it has a beat. The link is a relationship, not shared text.

A card needs at least:

* a globally unique project-local `BeatId`;
* a title and bounded body text;
* a stable category or palette key, not just a raw UI colour;
* a board-local rectangle (`x`, `y`, `width`, `height`) with finite, bounded values;
* zero or more `ScriptAnchor`s;
* zero or more safe media references; and
* optional tags, status, lane, and user-authored relationships.

An edge should use IDs, not array indexes. The graph validator should reject missing endpoints, duplicate
edge IDs, invalid self-links if the product disallows them, and allocations beyond configured limits.
Whether cycles are allowed is a product choice; the first version can allow them but should label them
clearly rather than silently trying to make every graph a tree.

Card moves and resizes are model mutations and therefore undoable. Intermediate pointer frames are UI
state; the first moved frame starts a command and later frames use the equivalent of
[`join_next_undo()`](../crates/engine/src/session.rs:835). A failed drop restores the previous rectangle.
Selecting a card, zooming the board, panning, and opening a filter are transient UI state, not document
undo steps.

“Convert to script” should be a deliberate command, for example `planning.beatToScript`, that validates
the target and creates one or more role-aware elements. It must not place card text into the script just
because a card was double-clicked. “Copy as screenplay fragment” can use the clipboard boundary, while
“insert as note” can make the loss of semantic role explicit.

Board colours and line styles should be semantic tokens with light/dark variants, following
[`Tokens`](../crates/ui-egui/src/theme.rs:197), rather than hard-coded colours. A board export should
include a legend only when requested and should have a stable sort order for equal z values. It should
not pretend that a board is a screenplay PDF page.

Images need special care. Only user-provided, public-domain, CC0, or otherwise redistributable assets may
be bundled. Do not copy product imagery or iconography from Final Draft, Microsoft, Adobe, Avid, or
Autodesk. A malformed, oversized, undecodable, or missing image reference should become a visible
placeholder and a structured warning, never a panic or an unbounded decode. Prefer a project asset
manifest and injected file service over silently reading arbitrary paths.

## 4. Cross-reference: Outline Editor

| Requested behavior | Related WordCraft principle or mechanism | What transfers | What must be redesigned for a screenplay app | Likely new data/model |
|---|---|---|---|---|
| Show ordered acts/sequences/lane rows | Outline levels, collapse state, navigation pane | Tree/ordered-view interaction and reveal/collapse behavior transfer conceptually | Word outline derives hierarchy from paragraph styles; screenplay structure should be explicit and may combine cards with scenes | `SequenceId`, `ActId`, ordered `BeatId` references, lane metadata |
| Drag to reorder beats | Paragraph move commands and object drag commit | One command, one undo step, stable selection after movement | Reordering a reference list must not silently reorder screenplay prose | `planning.reorderBeatRef` with source/target lane and index |
| Show target pages or scene goals | `Page`, page number, `DocLayout::page_of` | Current layout can answer actual page membership | A target is a plan; it cannot be confused with a measured printed page | Optional `PageGoal { start, end, mode }`, with derived actual ranges |
| Synchronize with board | Shared IDs and navigation commands | Both views can focus the same card | Board order and outline order are not necessarily identical | Cards stored once; outline contains references, not copies |
| Synchronize with script | Navigation pane and selection-to-layout mapping | Selecting a row can move the screenplay caret | Scene boundaries and stable element IDs must be screenplay-aware | Scene index derived from headings plus explicit scene/group metadata |
| Collapse and filter | [`OutlineView`](../crates/layout/src/outline.rs:31) and UI pane state | Keep collapse/filter choices out of canonical content unless user asks to save them | View state must use stable IDs, not mutable body block indexes | `OutlineUiState` in UI/session state; optional saved view preset later |
| Calculate approximate length | Paragraph measurement and page layout | Reuse point/line measurements as one estimate source | Estimated outline length may use summaries, while actual pages depend on full script and keep rules | `LengthEstimate`, assumptions/profile ID, and derived `ActualPageRange` |

### Lanes, order, and synchronization

An outline is best treated as a set of ordered references. A lane may represent an act, sequence, subplot,
character thread, or a user-defined planning category. A reference can appear in more than one view if the
product permits it; the card itself still exists once. If duplicate placement is confusing, require a card
to have one primary sequence and allow additional tags rather than duplicate references.

Suggested durable fields are:

```text
Outline
├── id, name
├── lanes: Lane { id, title, order, colour, beat_refs }
├── sections: Section { id, title, order, child_lane_ids }
└── goals: page/beat-count targets and optional notes
```

The outline's order is a planning order, not a second screenplay order. A drag can offer two explicit
operations: reorder the outline reference, or “reorder script to match,” the latter being a separate and
more consequential command. This distinction prevents an innocent planning gesture from moving authored
prose.

Actual pages should come from the same fixed print layout used by preview and PDF. A card linked to a
scene can display the first and last page of its anchor after layout. A card without an anchor can show an
estimate based on its summary, configured words-per-page, or a user-entered target. Label the result as
**estimated**. Page one in the estimate is not proof that the eventual screenplay will land on page one:
font metrics, dialogue wrapping, keep rules, title pages, and later edits all matter.

The current Word outline code is a useful warning. It computes levels and hidden blocks from paragraph
indexes ([`outline::hidden()`](../crates/layout/src/outline.rs:83)); that is fine for a view over one
document revision, but it is not enough for persisted planning data. Future screenplay outline state
should use `SequenceId`/`BeatId` and derive indexes only for the current render.

## 5. Cross-reference: Structure Lines

| Requested behavior | Related WordCraft principle or mechanism | What transfers | What must be redesigned for a screenplay app | Likely new data/model |
|---|---|---|---|---|
| Draw a line from a card/section to script content | Display overlays, object connectors, source-mapped glyph ranges | Paint an overlay after the base content and hit-test its endpoints | A planning line is a relationship projection, not a document shape | `StructureLine { id, source, target, style, visibility }` or a derived line from existing links |
| Follow a scene as pagination changes | Layout index, caret geometry, page mapping | Recompute projected endpoints after every layout revision | Stored x/y and page numbers go stale after edits | Durable anchor IDs plus derived `Projection { page, rect, revision }` |
| Navigate by clicking a line/card/marker | [`DocLayout::hit()`](../crates/layout/src/hit.rs:238) and canvas page mapping | Convert screen point to current page/document position | Board and script have different coordinate spaces and focus rules | Hit regions keyed by stable planning IDs and current layout revision |
| Show markers beside text or across page bands | Selection rectangles and display list | Share page-space geometry and theme tokens | Do not put planning markers into the screenplay's text stream | Layout adapter returns marker placements; UI paints them |
| Survive deleted or split content | Selection clamping and command rollback | Revalidate after every edit and show an unresolved state | WordCraft's current positions are clamped, not operation-aware rebased anchors | Anchor status: resolved, moved, ambiguous, orphaned; repair command |

### Anchor identity and projection

The key rule is: **a raw page number or x/y coordinate is not identity**. Page numbers change when text
wraps, a page profile changes, or a title page is added. Coordinates change with zoom, paper size, and
pagination. They are outputs of layout.

Use a durable anchor such as:

```text
ScriptAnchor
├── element_id: ElementId
├── range: WholeElement | TextRange { start, end } | Scene
├── optional quote/checksum for repair hints
└── anchor version/policy
```

The first implementation should prefer whole scene or whole element anchors. A range inside text is more
useful but harder: inserting text before it requires a mapping policy, and deleting the containing text
can make it ambiguous. A stable element ID plus a byte range is a good eventual model if edits can rebase
the range. Do not call a byte offset durable unless the edit engine explicitly maps it.

After layout, a projection service resolves an anchor against the current project and returns one or more
page-space rectangles: for a scene, perhaps its heading and final line; for a range, the first/last line
and intermediate line bands. Structure lines can then be drawn in the board or a margin overlay. The
projection is cached by project revision, layout revision, anchor ID, and view geometry, but never saved
as the source of truth.

When text changes, the session invalidates layout as it already does. The next frame recomputes projections.
If an anchor cannot be resolved, keep the card and show an “unlinked” state with a repair action; never
silently attach it to a nearby paragraph. Scrolling uses the current projection to scroll to the target.
Hit testing first checks the overlay's current screen regions, then delegates script clicks to the shared
canvas/layout hit tester.

## 6. Content versus planning/annotation state

This distinction should be made before implementing any of the three features:

```text
canonical project JSON
        │
        ├── screenplay content ──► screenplay layout ──► pages/display list ──► screen/PDF
        │          │                         │
        │          └── stable IDs            └── derived anchor projections
        │
        └── planning content ───► board/outline graph ──► board display/export
                         │
                         └── links by stable IDs/ranges

session/UI only: caret, selection, layout caches, pan/zoom, open pane, hover, drag preview
```

**Recommendation:** beats, edges, lanes, goals, and durable links are first-class project objects, but
they are not screenplay text elements. This makes them persistable and testable without polluting the
writing stream. Do not represent a beat as a hidden paragraph, comment balloon, field, or floating shape
merely because WordCraft has those features.

Persist in the native JSON project:

* schema version and project ID;
* screenplay elements and stable IDs;
* planning boards, beats, edges, lanes, order, categories, and durable anchors;
* project-relative asset manifest and any embedded asset bytes under explicit limits;
* page-goal assumptions and profile identifiers; and
* authored titles, descriptions, labels, and relationships.

Keep derived/transient state out of canonical JSON:

* caret and current text selection;
* open board, pan/zoom, viewport, hover, drag preview, and selected card;
* layout pages, page rectangles, source-to-page projections, and layout caches;
* undo/redo stacks and dirty bookkeeping;
* decoded image caches and temporary thumbnails; and
* a saved PDF/PNG preview.

A user may later choose “remember my last board and zoom,” but that is UI preferences or a view preset,
not evidence that page geometry belongs in the project model. If view presets become a real authored
planning artifact, version them explicitly and still store stable IDs rather than screen pixels.

## 7. Lessons to adopt, and traps to avoid

### Adopt

1. **One source of truth.** Keep screenplay text canonical and derive layout. Keep each beat canonical once
   and let board/outline/structure views reference it.
2. **Shared layout geometry.** Use the same source ranges, page mapping, and display boundary for editor,
   preview, PDF, and structure-line projection. WordCraft's [`page_display()`](../crates/layout/src/display.rs:361)
   is a good example of a backend-neutral boundary.
3. **Model/UI/persistence separation.** The three-state explanation in
   [`analysis/07-wordcraft-gui-system.md`](07-wordcraft-gui-system.md) is directly useful: document state,
   session state, and UI state have different lifetimes.
4. **Command and undo discipline.** Card edits, edge edits, lane reorder, anchor repair, and script
   conversion should have validated commands and transactional undo. Pointer motion is not a stream of
   permanent half-edits.
5. **Injected services.** File picking, image loading, clipboard, and export destinations belong at the
   host edge, following [`Services`](../crates/ui-egui/src/lib.rs:88), not inside the core model.
6. **Theme tokens.** Board colours, selection, unresolved-anchor warnings, and line styles should use
   theme semantics like [`Tokens`](../crates/ui-egui/src/theme.rs:197).
7. **Headless tests.** Exercise graph edits, save/reopen, projections, and command sequences without a
   window; add a small set of egui-kittest and screenshot tests only for interaction and visual layout.

### Do not copy

* Word's tables, stories, fields, ribbon breadth, and arbitrary formatting overrides as the screenplay
  planning model. They solve general word-processing compatibility, not this product's identity problem.
* A generic object system for every card, connector, picture, chart, and shape. A small typed graph is
  easier to validate than an open-ended object bag.
* Body block indexes as durable IDs. WordCraft's outline indexes are suitable for one layout revision,
  not for persisted links.
* A page number as a scene identity, or a line's x/y as a saved anchor.
* Product imagery, copied icons, or proprietary fonts. Reference material must follow the repository's
  asset and attribution rules.

## 8. A practical post-MVP roadmap

### Prerequisites: finish the dependable MVP

Before planning features, the project must have stable screenplay element IDs, role-aware editing, atomic
native save, recovery snapshots, deterministic print layout, PDF export, and headless command tests. This
is the acceptance boundary from [`analysis/06-screenplay-mvp-project-structure.md`](06-screenplay-mvp-project-structure.md:945),
not a suggestion to implement a board early.

### Phase 2A — project profile and durable anchors

Add a project profile and a small anchor index. Confirm that every screenplay element has a stable ID,
that scenes can be identified without relying only on current array position, and that insert/split/join/
delete define anchor behavior. Persist only IDs/ranges; derive page locations.

**Accept when:** save/reopen preserves IDs; deleting a target produces an explicit orphaned link; splitting
an element has deterministic anchor behavior; undo/redo restores anchors and text; and layout revision
changes do not change anchor identity.

### Phase 2B — Beat Board first

Implement one board with cards, colour/category, bounded rectangles, script anchors, and simple edges.
Add select, create, delete, edit, move, resize, connect, navigate-to-script, and undo/redo commands. Add
board-only rendering and a deterministic export before adding media.

**Accept when:** a board round trip preserves graph and card geometry; malformed graph input returns an
error; a drag is one undo step; a card can navigate to its linked scene; a failed command leaves the
project unchanged; and board export is identical for identical canonical data.

This comes first because it establishes the durable planning objects that the outline and structure lines
will consume. It also tests the hardest boundary—free-form planning data linked to semantic script data—
without yet requiring a full multi-pane planning product.

### Phase 2C — Outline lanes and page goals

Add ordered lane/sequence references to existing beats. Provide drag reorder, collapse/filter, target page
goals, and explicit “reorder planning” versus “reorder script” actions. Show actual page ranges only when
an anchor resolves against print layout.

**Accept when:** board and outline show the same card object; moving a reference does not move script text;
lane order survives persistence; target calculations identify their assumptions; actual page ranges update
after pagination; and estimated and actual values are visibly distinct.

### Phase 2D — Structure Lines

Add the projection service and a small overlay in the board/outline and script canvas. Begin with whole
scene/element anchors, one line style, and click-to-navigate. Add text ranges only after anchor mapping is
tested.

**Accept when:** line endpoints follow pagination and scrolling; zoom does not alter identity; text edits
recompute projections; deleted targets become unresolved rather than jumping; and line hit testing does
not interfere with text caret hit testing.

### Phase 2E — import/export and rich media

Only after the core graph is reliable, add a documented screenplay interchange adapter such as FDX, a
loss-reporting outline export, and bounded embedded/reference media. Keep native JSON canonical and PDF
output separate. Collaboration, concurrent editing, cloud sync, and external graph merge belong much
later because they require operation-aware anchor rebasing, conflict policy, and asset synchronization.

## 9. Suggested modules and eventual crate splits

Follow the one-core-crate-first approach in [`analysis/06-screenplay-mvp-project-structure.md`](06-screenplay-mvp-project-structure.md:1089).
Initially add modules beside the MVP core rather than creating a crate for every noun:

```text
core/src/
├── model.rs              # Project, screenplay elements, stable IDs
├── planning.rs           # Board, Beat, Edge, Lane, goals, validation
├── anchor.rs             # ScriptAnchor, resolution, orphan/ambiguous states
├── planning_edit.rs      # typed card/edge/lane/anchor mutations
├── projection.rs         # current layout -> page/line rectangles
├── board_display.rs      # backend-neutral board display items and export ordering
├── persistence.rs        # versioned JSON, assets, migrations, safe save boundary
└── session.rs            # selection, undo, revision, layout/cache coordination
```

The UI application can later add:

```text
app/src/
├── board_view.rs         # pan/zoom, card widgets, edge interaction
├── outline_view.rs       # lanes, filters, reorder gestures
├── structure_overlay.rs  # screen-space projection and hit regions
├── planning_dialogs.rs   # edit forms, asset picker, repair-anchor flow
└── theme.rs              # UI tokens, not model colours
```

Split only when dependency direction or compile time justifies it. A likely future arrangement is
`screenplay-core` for model/edit/session/persistence, `screenplay-layout` for page geometry and anchor
projection, `screenplay-render` for script and board display consumers, and `screenplay-ui-egui` for the
front end. A separate `screenplay-assets` crate is optional if media validation/cache code becomes large.
The core planning model must not depend on egui, eframe, winit, or a native file dialog.

Commands should remain small and semantic, for example:

```text
planning.createBoard
planning.createBeat
planning.updateBeat
planning.moveBeat
planning.connectBeats
planning.addAnchor
planning.reorderBeatRef
planning.repairAnchor
planning.beatToScript
planning.navigateToTarget
```

The exact IDs are future product decisions. The principle is observed in WordCraft's
[`CommandSpec`](../crates/engine/src/lib.rs:58): stable command identity, validation, enablement, and
one execution path for UI and tests.

## 10. Testing strategy

### Pure model and graph tests

* Generate card/edge sequences and assert every edge endpoint exists.
* Reject duplicate IDs, invalid rectangles, non-finite coordinates, oversized titles/bodies, and unknown
  categories without panicking.
* Test deletion policies: deleting a card either deletes its edges transactionally or leaves an explicit
  recoverable orphan, never a dangling edge.
* Verify board/outline references point to the same card and cannot silently duplicate card content.
* Test deterministic ordering and deterministic board export for equal inputs.

### Link and anchor tests

* Resolve scene, whole-element, and range anchors after insertion, split, join, reorder, and deletion.
* Test ambiguous and missing targets as first-class statuses.
* Ensure a stable `ElementId` survives array reordering and a new element never inherits an old ID.
* Verify navigation selects the intended current element and asks layout for current geometry.

### Page and goal tests

* Compare estimated outline length with actual rendered page ranges and assert that labels remain distinct.
* Edit text so a scene crosses a page boundary; structure-line projections must move while IDs remain.
* Change page profile/font metrics and verify derived ranges are recomputed, not loaded from stale JSON.
* Assert continuous layout and print layout use the intended profile, as the existing separation between
  live [`Session::layout()`](../crates/engine/src/session.rs:727) and export layout does.

### Undo, persistence, and hostile input

* Create/move/resize/connect/reorder/repair/convert, then undo and redo each operation.
* Drag many frames and verify they collapse into one logical undo step.
* Round-trip native JSON with boards, edges, IDs, anchors, goals, and asset manifest entries.
* Simulate failed atomic save and ensure the previous project and dirty state survive.
* Reject unknown future schema versions and boundedly migrate known versions.
* Pass missing, null, wrong-type, huge, negative, NaN-like, and duplicate values through every planning
  command; expected behavior is an error or safe clamp, never a crash.
* Give malformed, missing, oversized, and unsupported media references to the loader and renderer; draw a
  placeholder/warning and keep the project usable.

### UI and rendering tests

Use a headless session for most behavior. Add egui-kittest tests for creating a card, dragging it,
connecting two cards, opening a target, reordering an outline row, and clicking a structure line. Test
keyboard focus and pointer capture so a board drag does not insert text. Add a few stable screenshot or
display-structure tests for light/dark themes and deterministic board export; do not make every test
depend on GPU pixels. The existing canvas/control approach in [`crates/ui-egui/src/control.rs`](../crates/ui-egui/src/control.rs:1)
and kittest examples in [`crates/ui-egui/src/canvas.rs`](../crates/ui-egui/src/canvas.rs:2560) provide the
right style of test seam.

## 11. Open questions and explicit assumptions

These decisions should be answered before the corresponding feature is declared stable:

1. **What can a beat link to?** A whole scene is simplest. Should it also link to one element, a paragraph
   range, a page goal, or several alternatives? The recommendation is scene/element first, text ranges later.
2. **How are scenes identified?** Are scene boundaries derived from `scene_heading` elements, explicitly
   stored, or both with a validation rule?
3. **Are board coordinates durable?** The recommendation is yes within a board's coordinate system, with
   finite bounds and a migration policy. They must not be screen coordinates or page coordinates.
4. **What happens when script content is deleted?** Mark an anchor orphaned and offer repair, or
   automatically retarget only when a strong stable-ID/range rule proves the result. Never silently choose
   the nearest scene.
5. **What happens on split and join?** Does an anchor stay with the original element, expand to both
   results, or become ambiguous? The answer must be deterministic and undoable.
6. **Are page goals hard or soft?** A soft goal should not force pagination. A hard goal would require a
   special screenplay planning constraint and should not be added accidentally.
7. **Can a card appear in multiple lanes?** If yes, are those duplicate references or a graph of views?
   Prefer one card plus references/tags, with explicit primary placement.
8. **Are edges semantic or merely visual?** If an edge means “causes,” “follows,” or “parallel,” define
   the vocabulary and export behavior. Otherwise call it a visual relationship.
9. **Is media embedded or referenced?** Embedded bytes improve portability but grow files and require
   limits. Relative references preserve smaller projects but can break when moved. A manifest should make
   the choice explicit and show missing assets.
10. **Should planning state be collaborative?** Assume no in phase two. Collaboration would require stable
    operation IDs, anchor rebasing, merge rules, and asset conflict handling.
11. **Should view state be saved?** Assume no for canonical content. Add optional named view presets only
    after users demonstrate the need.
12. **What does board export mean?** A full board, a selected region, a lane report, or a PDF companion
    page? Define the output contract and avoid implying that it is a script page.

## Prioritized recommendation for the second phase

After the screenplay MVP is complete, implement in this order:

1. **Stable project IDs and anchor policy.** Without this, all three features will be built on moving
   indexes and will need to be rewritten.
2. **A small persisted Beat Board.** Cards, simple edges, scene/element links, commands, undo, JSON round
   trips, and deterministic board export establish the planning/content boundary.
3. **Outline lanes and page goals.** Reuse the same cards and stable links; distinguish planning order and
   estimated length from script order and actual pagination.
4. **Structure-line projections.** Build them from durable anchors plus current layout geometry, beginning
   with whole scenes and adding text ranges only after edit mapping is dependable.
5. **Interchange and rich media.** Add loss-reporting import/export and bounded assets only after the core
   graph and links are reliable.

This order keeps the first product small and protects the writing loop. The features are worthwhile only
when they remain views and relationships around a dependable screenplay—not when they turn the text model
into a generic Word-like object canvas.

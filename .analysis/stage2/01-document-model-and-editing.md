# Stage 2 — Document model and semantic editing foundation

## Scope and method

This report is a source-based assessment of the reusable editing foundation in
[`crates/doc`](../../crates/doc/src/lib.rs) and the command/session layer in
[`crates/engine`](../../crates/engine/src/session.rs). It distinguishes **observed facts**
(the current implementation) from **recommendations** for a small screenplay-writing app.
The proposed app is assumed to need screenplay elements such as scene headings,
action, character cues, dialogue, parentheticals, transitions, and notes, but not
Word-compatible tables, fields, tracked revisions, or complex page layout in its first
iteration.

## Executive assessment

The model is a strong editing substrate if the screenplay app treats a screenplay as
an ordered stream of paragraphs with semantic paragraph styles. The most valuable reuse
is the combination of:

* one canonical UTF-8 text buffer per paragraph;
* formatting runs measured in byte lengths;
* `Pos { story, path, off }` locations and anchor/focus selections;
* paragraph split/join, cross-paragraph deletion, copy/paste fragments, and undo snapshots;
* grapheme/complex-script-aware caret movement; and
* copy-on-write document trees using [`Arc`](../../crates/doc/src/lib.rs:3) and
  [`Arc::make_mut()`](../../crates/doc/src/lib.rs:643).

The main adaptation is semantic: screenplay roles should be a small, explicit vocabulary
of paragraph styles or a screenplay-specific semantic field, rather than inferred from
font attributes or encoded as inline objects. A screenplay app should keep the model's
stable document positions and command boundary, simplify stories/blocks/properties, and
avoid importing the large Word-specific surface area (tables, fields, notes, OOXML
revisions, floating shapes, and AutoFormat lists) unless a later feature needs it.

## 1. Observed document model

### 1.1 Document, stories, blocks, paths, and positions

**Facts.** A [`Document`](../../crates/doc/src/lib.rs:563) owns a body block list plus
styles and document metadata; secondary stories live in `parts`, keyed by numeric IDs
([`Document` fields](../../crates/doc/src/lib.rs:566)). A story is a list of
[`Block`](../../crates/doc/src/lib.rs:74) values, currently either a paragraph or table.
The module documentation explicitly says that body and secondary stories include headers,
footers, notes, comments, and text boxes, and that blocks are `Arc`-shared for cheap
clones and copy-on-write edits ([`doc module overview`](../../crates/doc/src/lib.rs:1)).

`StoryRef` identifies the body or a numbered part ([`StoryRef`](../../crates/doc/src/lib.rs:157)).
`Path` is a vector of block/table indexes; at top level it is `[block]`, and table-cell
paragraphs add `[table, row, cell, block]` ([`Path`](../../crates/doc/src/lib.rs:166)).
Its derived ordering is lexicographic, which the implementation uses as document order.
`Pos` combines story, path, and a byte offset ([`Pos`](../../crates/doc/src/lib.rs:212)).

The document guarantees nonempty stories/cells through [`Document::ensure_nonempty()`](../../crates/doc/src/lib.rs:631)
and provides paragraph traversal, start/end positions, clamping, and plain-text extraction
([`Document::para_paths()`](../../crates/doc/src/lib.rs:742), [`Document::start_of()`](../../crates/doc/src/lib.rs:751),
[`Document::clamp()`](../../crates/doc/src/lib.rs:791), [`Document::plain_text()`](../../crates/doc/src/lib.rs:803)).

**Implications for screenplay.** A screenplay's main story naturally maps to `body`.
Each screenplay element can be one paragraph, so path ordering already supplies the
canonical scene order. A separate story is useful only for genuinely separate editable
content (for example, a title page, character/locations database, or a revision note
pane); it should not be used as the primary representation of scenes. A plain linear
screenplay has no need for table paths, but retaining a generic `Path` or introducing a
small paragraph ID layer is preferable to replacing positions with raw paragraph indexes.

### 1.2 Paragraph text, runs, and inline objects

**Facts.** A [`Paragraph`](../../crates/doc/src/para.rs:745) stores one `String`, a list of
[`Run`](../../crates/doc/src/para.rs:38) records, paragraph properties, paragraph-mark
character properties, inline objects, an optional section, and a cache revision counter.
The module contract is especially important: run `len` values are byte lengths whose sum
must equal the UTF-8 text length, and inline objects occupy U+FFFC placeholders with the
k-th placeholder corresponding to `objects[k]` ([`paragraph representation`](../../crates/doc/src/para.rs:1)).

Text insertion rejects invalid offsets and strips U+FFFC from ordinary text
([`Paragraph::insert_text()`](../../crates/doc/src/para.rs:975)); object insertion maintains
the placeholder/object parallel structure ([`Paragraph::insert_object()`](../../crates/doc/src/para.rs:988)).
Run insertion splits and then normalizes adjacent equal runs
([`Paragraph::insert_run()`](../../crates/doc/src/para.rs:999), [`Paragraph::normalize()`](../../crates/doc/src/para.rs:1026)).
Deletion removes objects in the range, adjusts run lengths, and normalizes
([`Paragraph::delete()`](../../crates/doc/src/para.rs:1056)). Formatting similarly splits
runs, applies a callback, normalizes, and touches the paragraph
([`Paragraph::format()`](../../crates/doc/src/para.rs:1120)).

`split_off` preserves text, objects, run formatting, and the section break while moving
the tail to a new paragraph; the formatting at the split becomes the new paragraph mark
([`Paragraph::split_off()`](../../crates/doc/src/para.rs:1152)). `append` joins text/runs/objects
and keeps a section break if the appended paragraph has one
([`Paragraph::append()`](../../crates/doc/src/para.rs:1195)).

**Implications for screenplay.** This representation is a good fit for screenplay text:
dialogue emphasis, notes, omissions, and hyperlinks can remain run-level concerns, while
the element role belongs at paragraph level. Inline objects are useful for future inline
comments, production markers, or media references, but they should not carry screenplay
roles: U+FFFC is a content placeholder, not a semantic paragraph kind.

### 1.3 Properties and styles

**Facts.** Character and paragraph properties use `Option` fields, where `None` means
inheritance ([`props module overview`](../../crates/doc/src/props.rs:1)). [`CharProps`](../../crates/doc/src/props.rs:251)
contains typography, links, language, and revision markers. [`ParaProps`](../../crates/doc/src/props.rs:685)
contains style ID, alignment, indents, spacing, line rules, keep/page behavior, numbering,
tabs, borders, and other Word layout controls. Both expose overlay operations for patches
([`CharProps::overlay()`](../../crates/doc/src/props.rs:420), [`ParaProps::overlay()`](../../crates/doc/src/props.rs:799)).

[`Style`](../../crates/doc/src/styles.rs:18) has a stable ID, display name, kind, `based_on`
chain, `next` style for the following paragraph, optional linked character style, and
paragraph/character properties. [`StyleSheet::chain()`](../../crates/doc/src/styles.rs:433)
is cycle-safe and bounded; styles can be found by ID or display name and can be upserted
or removed ([`StyleSheet::find()`](../../crates/doc/src/styles.rs:423), [`StyleSheet::upsert()`](../../crates/doc/src/styles.rs:479),
[`StyleSheet::remove()`](../../crates/doc/src/styles.rs:486)).

**Implications for screenplay.** A screenplay can represent roles as stable paragraph
style IDs such as `SceneHeading`, `Action`, `Character`, `Dialogue`, `Parenthetical`,
`Transition`, and `Title`. The existing `next` style mechanism is useful for Enter
behavior (for example, Dialogue → Character or Action, depending on policy), but it is
not expressive enough by itself for context-sensitive screenplay continuation. The app
should put continuation policy in a screenplay command/role policy layer and use style
properties for rendering defaults.

Recommendations:

1. Keep style IDs stable and machine-readable; use display names only for UI.
2. Use paragraph style for the element role, with direct paragraph properties only for
   intentional local overrides.
3. Add a small screenplay-specific semantic value only if style IDs become insufficient
   for metadata such as scene IDs, dialogue parent linkage, or locked/generated content.
4. Do not encode roles in `CharProps` (bold/all caps/indent). Those are presentation and
   can be changed independently of meaning.

## 2. Text identity, Unicode positions, and selections

### 2.1 What currently identifies text

**Facts.** Text has no persistent per-character or per-paragraph identity. A location is
structural: story + path + byte offset. Paragraphs have a process-local `rev` counter for
cache invalidation, but equality intentionally excludes it
([`Paragraph::PartialEq`](../../crates/doc/src/para.rs:765), [`Paragraph::touch()`](../../crates/doc/src/para.rs:809)).
Document cloning therefore preserves content and structure, not stable semantic IDs.

Positions are ordered using derived ordering on `StoryRef`, `Path`, and `off`; range helpers
sort endpoints with [`edit::order()`](../../crates/doc/src/edit.rs:46). There is no anchor
rebasing system: session edit marks are explicitly “not mapped through later edits” and
are clamped when used ([`Session::edit_marks`](../../crates/engine/src/session.rs:442),
[`Session::go_back()`](../../crates/engine/src/session.rs:640)).

**Recommendation.** For a small single-user screenplay editor, structural positions are
adequate for the active caret and selection. For durable screenplay annotations, do not
persist raw `Pos` values as identity. Use stable paragraph/element IDs in an app-level
semantic index, or use bookmarks/markers only when the annotation must live in the text.
If collaboration, external annotations, or long-lived scene references are in scope,
introduce explicit stable IDs before building those features; deriving identity from
paragraph indexes will break on insertions and reordering.

### 2.2 Unicode and caret boundaries

**Facts.** Offsets are UTF-8 byte offsets. [`Paragraph::check()`](../../crates/doc/src/para.rs:890)
rejects offsets that are outside the string or not char boundaries, while
[`Paragraph::clamp()`](../../crates/doc/src/para.rs:897) moves an invalid offset backward
to a valid boundary. Caret movement uses cluster-aware [`Paragraph::prev_boundary()`](../../crates/doc/src/para.rs:1208)
and [`Paragraph::next_boundary()`](../../crates/doc/src/para.rs:1213); Backspace has a
script-specific [`Paragraph::backspace_from()`](../../crates/doc/src/para.rs:1217) rule.
Word navigation and sentence/word selection are paragraph-local
([`Paragraph::word_start()`](../../crates/doc/src/para.rs:1227), [`Paragraph::word_end()`](../../crates/doc/src/para.rs:1246),
[`Paragraph::word_at()`](../../crates/doc/src/para.rs:1267), [`Paragraph::sentence_at()`](../../crates/doc/src/para.rs:1298)).
The tests cover combining marks, Myanmar syllables, Devanagari, Khmer, and invalid UTF-8
boundaries ([`tests_typing.rs` Unicode tests](../../crates/engine/src/tests_typing.rs:537),
[`edit::tests::bad_positions_error`](../../crates/doc/src/edit.rs:843)).

**Implications for screenplay.** Keep byte offsets internally; they are efficient and
already correctly support multilingual screenplays. Expose character indexes only at an
API boundary if required, and convert through checked char-boundary logic. A screenplay
app should not assume ASCII despite common all-caps character cues. Character names,
dialogue, multilingual action, and combining marks need the same cluster behavior.

### 2.3 Selection model

**Facts.** The engine selection is an anchor/focus pair of positions
([`Selection`](../../crates/engine/src/session.rs:23)). A caret is a collapsed selection;
`ordered()` returns document-order endpoints ([`Selection::caret()`](../../crates/engine/src/session.rs:31),
[`Selection::ordered()`](../../crates/engine/src/session.rs:37)). Commands return the
selection as serialized positions ([`sel_result()`](../../crates/engine/src/cmd/mod.rs:130)).
The model supports ordinary stream selections plus specialized column/table selections,
but the latter are optional for screenplay.

Document range editing is story-local: [`Document::delete_range()`](../../crates/doc/src/edit.rs:300)
rejects ranges spanning stories, joins paragraphs when appropriate, and handles tables.
Formatting uses corresponding cross-paragraph helpers
([`Document::format_range()`](../../crates/doc/src/edit.rs:384), [`Document::format_paragraphs()`](../../crates/doc/src/edit.rs:419)).

**Recommendation.** Keep anchor/focus exactly. It supports forward/backward drag semantics,
keyboard extension, replacement typing, and future range annotations without adding a
second selection abstraction. Remove or defer column/table selection in a small app, but
do not simplify selection to `(start, end)` only: direction matters for caret collapse and
editing behavior.

## 3. Editing primitives and command semantics

### 3.1 Document-level operations

**Facts.** [`Document::insert_text()`](../../crates/doc/src/edit.rs:242),
[`Document::insert_object()`](../../crates/doc/src/edit.rs:250),
[`Document::split_paragraph()`](../../crates/doc/src/edit.rs:257), block insertion/removal,
range deletion, formatting, copying, and fragment insertion form a coherent primitive set.
`Fragment` contains copied blocks and associated text-box parts
([`Fragment`](../../crates/doc/src/edit.rs:14)); copy/paste clones text-box stories and
allocates new IDs rather than sharing them ([`Document::insert_fragment()`](../../crates/doc/src/edit.rs:553)).
The tests cover split/join, multi-paragraph deletion, formatting across paragraphs,
copy/paste, tables, bad positions, and copy-on-write behavior
([`edit::tests::split_and_join`](../../crates/doc/src/edit.rs:647),
[`edit::tests::copy_paste_round_trip`](../../crates/doc/src/edit.rs:679),
[`tests::clone_is_shallow`](../../crates/doc/src/tests.rs:93)).

**Recommendation.** Reuse the text/paragraph operations and fragment concept. For the
screenplay MVP, a fragment can be only paragraphs plus styles/metadata; the text-box
adoption machinery should be omitted until media or floating notes are real requirements.
Preserve atomic operations that return the post-edit position: this makes command code
and UI caret updates straightforward.

### 3.2 Engine commands and typing behavior

**Facts.** User-visible editing is command-driven. The text command registry includes
insert text, new paragraph, line/page/column breaks, tab, Backspace, Delete, word deletion,
and Unicode/symbol insertion ([`text::specs()`](../../crates/engine/src/cmd/text.rs:11)).
`type_text()` replaces the selection, applies pending typing properties, and optionally
adds insertion revisions ([`type_text()`](../../crates/engine/src/cmd/mod.rs:256)); paragraph
separators split paragraphs, while line breaks remain text characters
([`insert_text()`](../../crates/engine/src/cmd/mod.rs:268)). Enter delegates to split logic,
including style-next behavior and list handling ([`text::new_paragraph()`](../../crates/engine/src/cmd/text.rs:388),
[`split_para()`](../../crates/engine/src/cmd/mod.rs:340)).

Selection deletion chooses ordinary deletion or tracked deletion
([`delete_selection()`](../../crates/engine/src/cmd/mod.rs:148)). Backspace and Delete use
cluster boundaries, word boundaries, paragraph joining, and special list behavior
([`text::backspace_inner()`](../../crates/engine/src/cmd/text.rs:495), [`text::delete_inner()`](../../crates/engine/src/cmd/text.rs:581)).

Typing is coalesced into an undo step, while selection/caret changes delimit typing groups;
undo entries snapshot document, selection, pending formatting, and specialized selection
state ([`Session::checkpoint()`](../../crates/engine/src/session.rs:806), [`Session::run()`](../../crates/engine/src/session.rs:1116)).
The command boundary also catches panics and restores the pre-command document state
([`Session::run()` error/rollback path](../../crates/engine/src/session.rs:1193)).

The typing tests deliberately send one command per character/key and pin behavior for
Enter, Backspace, lists, Unicode, undo grouping, selection replacement, and overtype
([`tests_typing::typ()`](../../crates/engine/src/tests_typing.rs:19),
[`typing_then_selecting_then_typing_are_two_undo_steps`](../../crates/engine/src/tests_typing.rs:496),
[`typing_over_a_selection_follows_the_replace_selection_option`](../../crates/engine/src/tests_typing.rs:737)).

**Recommendation.** Reuse the command/session architecture, especially one command path
for UI, CLI, automation, and tests. A screenplay app should define a smaller command set:
`text.insert`, `text.newParagraph`, `text.backspace`, `text.delete`, `select.*`, `edit.undo`,
`edit.redo`, `screenplay.setElementRole`, and perhaps `screenplay.nextElement`. Implement
screenplay-specific Enter behavior as a command policy over [`Document::split_paragraph()`](../../crates/doc/src/edit.rs:257),
not as UI-only mutation. Keep typing undo coalescing and rollback semantics.

Do not inherit Word's AutoCorrect/AutoFormat wholesale. Screenplay-specific transforms
(automatic scene-heading capitalization, character cue continuation, dialogue/action
cycling) are domain rules and need explicit tests, opt-outs, and predictable undo grouping.

## 4. Mapping screenplay semantics

### Recommended MVP mapping

| Screenplay concept | Reuse mapping | Notes |
|---|---|---|
| Script | [`Document`](../../crates/doc/src/lib.rs:563), `body` | One primary story; serialized as the app's own format initially. |
| Element | [`Paragraph`](../../crates/doc/src/para.rs:745) | One element per paragraph keeps editing and navigation simple. |
| Element role | Paragraph style ID, or a small screenplay role field | Prefer stable style IDs first; add a field if metadata needs to survive style changes. |
| Element text | [`Paragraph::text`](../../crates/doc/src/para.rs:747) | Canonical UTF-8 text; no duplicated semantic text cache. |
| Inline emphasis | [`Run`](../../crates/doc/src/para.rs:38) / [`CharProps`](../../crates/doc/src/props.rs:251) | Keep independent from element role. |
| Caret/selection | [`Pos`](../../crates/doc/src/lib.rs:212) and [`Selection`](../../crates/engine/src/session.rs:23) | Preserve story/path/byte offset and anchor direction. |
| Scene metadata | App-level stable scene ID keyed to paragraph/element | Do not rely on `Path` remaining stable through edits. |
| Title page or notes | Separate story only if editing/rendering independently | Avoid making every screenplay component a `Part`. |
| Revision history | Session undo snapshots initially | Add persistent revision metadata only if collaboration/review is required. |

### Role transitions

The current style `next` property is a useful default transition hook
([`Style::next`](../../crates/doc/src/styles.rs:26)). For screenplay, recommended behavior
is a separate transition table with context, for example:

* Scene Heading → Action
* Action → Action, Character, or Transition
* Character → Dialogue
* Dialogue → Parenthetical or Character/Action
* Parenthetical → Dialogue
* Transition → Scene Heading or Action

This table should be a screenplay policy, not a general-purpose style inheritance rule.
The command can inspect the current role and surrounding paragraphs, call the existing
split primitive, assign the next role/style, and return the new caret position. That
preserves the document engine's atomic editing contract while allowing screenplay UX.

## 5. Keep, simplify, and avoid

### Keep

* UTF-8 byte offsets with checked boundaries and cluster-aware movement.
* Anchor/focus selection and structural ordering.
* Paragraph text + normalized formatting runs.
* Paragraph split, append, delete-range, copy-range, and fragment insertion.
* Stable style IDs with inheritance and explicit paragraph/character separation.
* Copy-on-write `Arc` blocks for undo snapshots and renderer reads.
* Session command dispatch, rollback, typing grouping, and tests that exercise commands.

### Simplify

* Limit blocks to paragraphs for the MVP; add tables only if production breakdowns need them.
* Use one body story; defer headers, footers, notes, comments, text boxes, and story reachability.
* Retain only screenplay-relevant `ParaProps` and `CharProps` in a façade or adapter rather
  than exposing the full Word property set to app code.
* Use a small built-in screenplay stylesheet instead of the full Word-style gallery.
* Use ordinary session undo first; defer persistent tracked changes and move revisions.
* Replace generic clipboard formats with a screenplay fragment format plus plain text until
  interoperability is a requirement.

### Avoid

* Treating paragraph indexes or raw positions as durable text identity.
* Encoding screenplay roles in bold, indentation, capitalization, or other visual cues.
* Using U+FFFC inline objects for paragraph/scene semantics.
* Copying the full `Document` schema into a new screenplay-specific model before validating
  the editing UX; it adds substantial Word-specific complexity.
* Reimplementing Unicode deletion/caret logic with `String` character counts or ASCII assumptions.
* Mutating the model directly from the UI and bypassing commands, undo, rollback, or tests.
* Importing AutoFormat list rules into screenplay typing without a screenplay-specific policy.

## 6. Risks and follow-up questions

1. **Identity risk:** if scene cards, notes, or external references are required, decide
   early whether to add stable element IDs to paragraphs or maintain a separate ID index.
2. **Role/style coupling:** changing styles can change rendering but should not accidentally
   change semantic role. A dedicated role field becomes preferable if users can freely edit
   styles.
3. **Position mapping:** the current Go Back/edit marks are clamped, not rebased. Persistent
   annotations and collaboration need an operation-aware mapping strategy.
4. **Layout boundary:** screenplay page formatting may eventually need page-aware layout,
   but the semantic model should remain paragraph-oriented so pagination does not become
   the identity model.
5. **Tests:** retain the existing pattern of command-level tests. Add screenplay parity
   tests for role transitions, Enter at every role, Backspace across element boundaries,
   paste of mixed roles, multilingual text, and undo after automatic role changes.

## Conclusion

The reusable foundation is not “Word's whole document schema”; it is the small, coherent
editing kernel inside it: paragraph text, normalized runs, structural positions, directional
selections, safe Unicode boundaries, atomic range operations, copy-on-write snapshots, and
command-mediated undoable edits. A screenplay app should build its semantic layer on top
of that kernel, with paragraph styles/roles and explicit role-transition commands. It should
defer or hide Word-specific stories, tables, objects, revision formats, and formatting
surface area until a concrete screenplay requirement justifies them.

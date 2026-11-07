# Product Overview — ScriptCraft

## Business problem

Individual screenwriters need a calm, dependable writing loop rather than a
general-purpose document suite. Losing screenplay meaning, predictable page flow,
or the last saved version interrupts creative work and makes a draft hard to
trust. ScriptCraft protects the draft first and adds breadth only when a
screenplay-specific need justifies it.

## Audience and stakeholder needs

| Audience | Need | Desired outcome |
|---|---|---|
| Individual screenwriter | Draft and revise scenes without losing screenplay meaning or control of the work | Write, revise, undo safely, inspect page layout, save and reopen the native project, and export review copies with their limitations clear |
| Product steward | Keep the product focused on a dependable screenplay-writing loop | A small, coherent product boundary that prioritizes safety over broad feature count |

## Product description

ScriptCraft is a focused desktop screenplay editor for one writer. A screenplay
project has a semantic source of truth containing an ordered sequence of
screenplay elements and retained FDX source content, including source-owned
metadata, paragraph identities, title-page structure, and document settings.
Internal identities used to edit and anchor content are transient; they are not
a promise of stable ScriptCraft IDs across reopen. Editing preserves explicit
roles and the writer's exact canonical UTF-8 casing across typing, paste, role
changes, and history operations. AllCaps affects display rather than canonical
text or native FDX source; runtime selection, undo/redo, dirty state, and derived
layout do not replace that source. Selection is serialized only as disposable
root TextState view state during transactional native saves.

Managed Scene Headings use generated SceneProperties/SceneArcBeats containers
for supported scenes. Length defaults to `1/8`, Page to `1`, and Title to empty;
supported imported non-arc container attributes and children are regenerated on
refresh, without claiming Page or Length recalculation. Supported
CharacterArcBeat membership is reconciled from normalized Character names:
matched nested Paragraph IDs and their ordered text/run content, formatting, and
source attributes remain associated by name; stored derived names are uppercase;
and new names receive fresh IDs. Retained Characters keep their arc content
through unrelated roster edits, while removed Characters lose the complete
derived arc and text. A rename to a different normalized name creates a blank
arc rather than transferring content by position or nested ID. Canonical cue
text is unchanged. Supported arc Paragraphs remain opaque to screenplay body
content. Duplicate, ambiguous, or unknown non-text arc structures remain
preserved and defer only that scene's derived refresh; they do not prevent an
otherwise valid canonical native Save or Save As, and independent supported
scenes may still update.
An explicit cross-paragraph selection deletion may remove a Scene Heading with
its recognized generated SceneProperties/SceneArcBeats subtree only after the
heading and all nested arc Paragraph identities pass unique-registration,
reference, anchor, and external-source-use checks. This narrow deletion rule does
not authorize removal of unknown or ambiguous attachments, Cast Lists, or
unrelated source nodes, and does not relax backspace/join, split, or role-change
safe-stops.
Cast List presentation consumes uppercase synchronized scene-arc names with the
existing alphabetical case-insensitive ordering. Lists in deferred scenes, and
formatted lists that cannot be corrected without rewriting their runs, remain
unchanged rather than blocking canonical saves. This bounded projection does
not promise editing of other Final Draft beat-board structures or recalculation
of scene page/length values.

Desktop derived refresh is scheduled at no more than the existing one-second
cadence for relevant pending work; unchanged successfully attempted revisions
and irrelevant edits do not cause repeated reconciliation. A fatal refresh keeps
its pending scope and error category but is suspended from retries until relevant
invalidation, separately from nonfatal safe-stop deferral. Explicit Save/Save As
prepares a complete derived candidate independently of timer suspension. Refresh
diagnostics are value-free counts/categories, not screenplay content or evidence
of user-perceived performance.

The writing experience supports continuous editing and repeatable fixed-page
preview from shared layout semantics for the same effective runtime layout
inputs. This does not promise exact layout fidelity for every FDX file. The
native project preserves screenplay meaning and retained FDX source data across
FDX save and reopen. PDF is a page-layout export; plain-text export is
deliberately lossy and communicates that roles, identities, and page layout are
not preserved. Plain-text export uses the shared print-layout display text, so
effective AllCaps presentation (including Unicode expansions) appears in the text
copy while roles without AllCaps retain canonical casing. This transformation does
not rewrite semantic text or native FDX. Role typography uses bounded imported FDX
settings with a deterministic fixed-cell Courier Prime fallback; presentation
transforms do not rewrite canonical text. HiddenText roles remain visible and
subdued while writing but are omitted from print/PDF and plain-text copies. Neither
export replaces the native project as the source of truth, and no exact FDX or
Final Draft rendering parity is promised.

New FDX paragraph identities use collision-checked UUID v4 values. Imported paragraph
identities remain verbatim even when they are not UUIDs. A never-saved project may
receive different random identities on separate first-time serializations; once a
saved/re-read source establishes those identities, later saves preserve them. This
identity contract does not add historical ID tombstones. Repeated raw IDs on opaque,
non-managed TitlePage Paragraph occurrences are preserved verbatim per occurrence but
are not treated as globally unique definitions; managed roles and other global identity
consumers continue to require unambiguous identities.

Native FDX lineage is durable source data distinct from semantic screenplay content:
DocumentRef retains its imported ID and records the current saving time after each successful
Save or Save As. A separate ephemeral session-open timestamp is captured when an editing session
starts and remains fixed through saves and rebases. Ordinary Save creates one XRef only when
DocumentRef has no direct-child XRefs; Save As always appends one. New XRefs use the session-open
timestamp, while existing XRefs remain unchanged. This timestamp is not undoable content, does not
come from DocumentRef DateTime, and may equal a saving timestamp within the same second. Direct
serialization does not mutate imported lineage. Failed saves publish neither lineage changes nor
destination/session changes.

The native root WindowState is a separate disposable view-state exception to semantic persistence:
the desktop projects only bounded direct-root geometry, attempts a valid imported restore once after
Open, and overwrites it on transactional Save/Save As from the latest native logical geometry. It is
not part of screenplay Project semantics, dirty state, undo history, fixed document layout, or PDF
geometry. Nested/title-page structures and unrelated source data remain preservation-owned.

Root TextState is the corresponding disposable body-editor view-state exception. Open projects
only a unique direct-root block, restores whole-percent page zoom and a valid ordered Selection
once, then actual Save/Save As overwrite it from current runtime values. Selection uses ScriptCraft's
bounded reversible UTF-16 flat offsets over canonical body paragraphs, with one separator per
paragraph boundary and UTF-8 session byte positions. Malformed or unmappable values safely become
caret 0,0; unusable Scaling defaults to 100 and valid values clamp to 75–300. Nested TitlePage
TextState remains source-preserved and is never applied to the body editor. Neither zoom nor caret
movement changes semantic content, dirty/history state, fixed layout, or export output.

Document layout and PDF use OS-independent document points at 72 points per inch; default Letter
remains 612 × 792 points. The desktop alone converts document points to logical presentation
coordinates using 1.0 on macOS and 96/72 on Windows, then lets egui apply the native display scale
once. Missing or invalid native scale uses neutral 1.0. TextState Scaling persists only the user's
whole-percent zoom, never either platform conversion or native display scale. These conversions
approximate physical size under the OS coordinate model and do not promise monitor calibration.

## Product scope

The product is the smallest useful, dependable writing loop for one desktop
screenwriter.

### In scope

- Create a screenplay project with title-page structure and the FDX content
  elements specified in the [writing-loop requirements](features/screenplay-writing-loop.md).
- Edit and navigate UTF-8 text safely, with split, join, delete, undo, and redo.
- Apply explicit role-aware transitions rather than infer meaning from
  capitalization, spaces, tabs, or indentation.
- Support continuous writing and repeatable page preview from shared layout
  semantics for the same effective runtime layout inputs; exact fidelity to
  every imported FDX layout is not promised.
- Apply supported role-level FDX font size/style and AllCaps as presentation,
  with fixed-cell metrics and source-safe display mapping. Keep HiddenText roles
  available in the editor with a contrast-safe subdued treatment and omit them
  from print/PDF and plain-text exports.
- Save and reopen native Version 6 FDX while preserving supported semantic data
  and safely retaining unsupported source content. Non-6 imports have the
  explicitly bounded conversion behavior specified in the writing-loop requirements.
  Save operations are atomic and failed operations preserve the prior valid work.
- Export PDF from fixed print layout and provide a deliberately lossy plain-text
  copy that communicates its loss.
- Keep core behavior headless-capable and testable without a desktop window.
- Route enabled ScriptCraft-owned actions through the UI-independent internal command contract:
  42 stable dotted identities, typed/serializable parameters, command origins, one desktop dispatcher,
  and a headless semantic editor executor. Screenplay mutations shall be executable through that
  contract without a GUI; desktop dialogs, clipboard, focus, viewport, and notifications remain host
  effects. Native predefined macOS Application-menu services remain OS-owned unless an existing safe
  host API can route them. The contract does not itself expose a command palette, CLI, MCP server,
  network service, macro UI, or plugin host; these remain out of product scope.
- The decorated native window title reflects the active FDX filename after successful lifecycle
  changes and otherwise shows `Untitled Screenplay`; failed/cancelled operations retain the prior
  title. Native decorations and title positioning remain OS-owned.
- The status center is reserved for Ready and live cursor/modifier guidance. Routine success text is
  not presented there. Retained operation errors are assigned to the Diagnostics disclosure, which
  is intentionally disabled by default; while disabled, those errors are hidden rather than moved to
  the center.
- Menu presentation follows the runtime OS default: native-only on macOS, in-window-only on Windows,
  and no backend by default on other targets. A process-transient development argument may select
  in-window menus without impersonating another host OS.

### Out of scope

- DOCX compatibility and blanket Final Draft compatibility.
- Collaboration, cloud sync, tracked revisions, comments, or revision comparison.
- MCP, remote control, network automation, or a user-facing command catalog/palette as a product
  feature. The internal command contract used to unify existing clients is not such a product feature.
- Planning, beat-board, outline-editor, structure-line, production breakdown,
  scheduling, budgeting, or character databases.
- Editing broad Word-like formatting, tables, fields, headers/footers, footnotes,
  shapes, images, equations, arbitrary nested stories, dual dialogue, scene numbering,
  colored pages, or locked pages. Retaining unsupported imported data is distinct
  from providing these editing features.
- Web support, multi-platform polish, pixel-perfect editor parity, font
  databases, and fully incremental suffix pagination. Bounded per-element wrap
  reuse within whole-document page placement is a separate structural
  optimization and does not require or claim unmeasured incremental pagination.

## Success outcomes

| Outcome | Desired result | Measurement approach |
|---|---|---|
| Complete writing loop | A writer can create and edit screenplay roles, inspect pages, undo/redo, save and reopen, and produce PDF or plain-text exports without confusing exports with the native project | Headless acceptance coverage and desktop scenario checks |
| Native semantic preservation | Roles, text, and retained FDX-owned metadata, source identities, title structure, document settings, and other source content remain preserved after native save and reopen; transient ScriptCraft identities and app-specific metadata do not become durable file fields | Deterministic serialization and source/semantic comparison checks |
| Failed-operation safety | A failed edit or save leaves the prior valid in-memory or on-disk state intact | Failure-injection and rollback checks |
| Predictable pagination | The same semantic content and effective runtime layout inputs produce stable structural page results regardless of viewport zoom; exact fidelity to every FDX layout is not promised | Structural pagination fixtures |
| Honest export boundaries | PDF uses fixed print layout; plain-text output communicates that roles, identities, and page layout are lost | Export acceptance checks |

## Product constraints

- Desktop-first and focused on a single writer's project.
- Native project data remains semantic and independent of screen presentation;
  page coordinates, caret geometry, TextState/editor state, and undo stacks are
  derived or transient. Root WindowState alone is persisted as disposable FDX
  desktop view state and is never promoted into screenplay semantics.
- FDX is the durable owner of document metadata, foreign paragraph identities,
  title-page structure, and source document settings. ScriptCraft does not add
  a private FDX extension or sidecar for project identity, schema, layout
  configuration, or editing policy. Such application-only state is not a
  persistence guarantee; nondefault legacy configuration and internal IDs may
  reset after reopen, and legacy JSON migration is not required.
- Newly created native FDX documents emit one deterministic ElementSettings
  block for each of the 17 directed roles from the shared reference-derived
  defaults. Imported source settings remain preservation-owned and are not
  regenerated from runtime defaults.
- Layout remains repeatable for the same effective runtime inputs. Explicit
  application defaults and only validated FDX mappings supply those inputs;
  preserving source layout settings does not assert that every FDX layout is
  interpreted or rendered identically.
- Body layout resolves native defaults and validated imported role
  `ParagraphSpec` geometry, alignment, first-line indent, spacing, and page-start
  fields at signed 1/100-point precision. Unsupported or unusable source values
  remain source-owned and fall back deterministically; display geometry does not
  rewrite canonical screenplay text.
- For managed body paragraphs, supported direct paragraph geometry, spacing, and
  page-start fields override the same role setting independently. Invalid or
  unsupported direct values fall through to the role setting and shared default;
  direct values remain attached to the managed semantic element while source XML
  remains authoritative for save/reopen. Title-page and run-level formatting are
  outside this runtime projection.
- Safety and preservation of the last valid native project take priority over
  breadth of features.

## Feature map

- [`features/screenplay-writing-loop.md`](features/screenplay-writing-loop.md) — semantic screenplay editing, fixed-profile layout, native persistence, and explicit exports.

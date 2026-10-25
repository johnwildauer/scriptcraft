# Synthesis: a deliberately small screenplay-writing app

## Purpose and reading guide

This is the Stage 3 conclusion from the broad architecture pass and all five Stage 2
reports: [Stage 1 broad architecture](01-broad-architecture.md), [document model and
editing](stage2/01-document-model-and-editing.md), [layout and pagination](stage2/02-layout-pagination-rendering.md),
[persistence and recovery](stage2/03-persistence-formats-recovery.md), [command/session and UI
boundary](stage2/04-command-session-ui-boundary.md), and [testing, safety, and portability](stage2/05-testing-safety-portability.md).

The goal is not to reproduce WordCraft. It is to extract the smallest set of ideas that
make a writing application dependable, understandable to a Rust beginner, and capable of
growing into a real screenplay tool.

Two labels are used throughout:

* **Observed WordCraft fact** means the reports traced this behavior to the repository.
* **Recommendation** means a design choice for the future screenplay app. It is not a
  claim that WordCraft already implements it.

## Executive conclusion

The best small architecture is a semantic screenplay document surrounded by a narrow,
transactional editing session. Keep the document independent from the UI, make all edits
commands, lay out paragraphs into pages through one geometry pipeline, and save a
versioned native project atomically. Start with one desktop shell and a headless test
runner; add interchange formats and web support only after the writing loop is reliable.

The decisive simplification is to separate **meaning** from **appearance**:

* “This paragraph is dialogue” is domain data.
* “This dialogue is 2.5 inches from the left edge” is a layout rule.
* “The caret is at byte offset 14” is transient editing state.
* “This project is saved at this path” is persistence/session state.

WordCraft supplies excellent patterns for the last three boundaries, but its full model is
too broad for the first product. Its reusable kernel is paragraph editing, Unicode-safe
positions, directional selections, command transactions, shared layout geometry, and safe
file handling—not its tables, stories, 400-plus commands, or Word compatibility surface.

## Ranked architectural lessons

### 1. Protect the writing before adding features

**Observed WordCraft fact.** Production crates prohibit common panic paths and route
commands through [`Session::run()`](../crates/engine/src/session.rs:1116), which validates,
snapshots, executes, rolls back on errors, and converts an escaped panic into an error. The
repository also tests hostile command parameters in
[`hostile_params_never_panic()`](../crates/engine/src/tests.rs:1856). Its safe-save helper
prepares a sibling temporary file before replacing the destination
[`safe_save::write()`](../crates/engine/src/io/safe_save.rs:8).

**Recommendation.** Make no-panic production code, transactional edits, atomic native saves,
and recovery tests the first architectural commitments. A screenplay editor's most serious
bug is not an imperfect toolbar; it is losing a scene. Keep `catch_unwind` only as a last
resort, never as permission to write unchecked code.

**Why it ranks first.** These guarantees apply to every future feature and are cheaper to
build before the model and UI become large.

### 2. Put screenplay semantics in the model, not in formatting tricks

**Observed WordCraft fact.** The document is paragraph-oriented: a
[`Paragraph`](../crates/doc/src/para.rs:745) owns canonical UTF-8 text, runs, and paragraph
properties; positions are structural plus a UTF-8 byte offset
[`Pos`](../crates/doc/src/lib.rs:212). Styles have stable IDs and inheritance
[`Style`](../crates/doc/src/styles.rs:18), while runs are presentation-level.

**Recommendation.** Represent each screenplay element as one ordered paragraph with an
explicit role (`scene_heading`, `action`, `character`, `dialogue`, `parenthetical`,
`transition`, or `note`). Use role defaults to derive typography and indents. Do not infer
meaning from all-caps, bold, spaces, tabs, or indentation. If editable styles later make
style IDs ambiguous, add a dedicated role field; do not make the renderer guess.

**Why it ranks second.** A stable semantic model makes navigation, role-aware Enter,
pagination rules, native persistence, and FDX export agree. If meaning is encoded only in
appearance, every later feature becomes a format-conversion problem.

### 3. Preserve one command path for keyboard, UI, tests, and automation

**Observed WordCraft fact.** [`CommandSpec`](../crates/engine/src/lib.rs:58) and
[`Registry`](../crates/engine/src/lib.rs:102) give commands stable IDs, parameter
documentation, mutability, enablement, and handlers. The UI interprets platform events;
the engine interprets editing intent. The same session path is used by the UI, CLI, and
headless automation (see the traced flow in
[Stage 2 command/session](stage2/04-command-session-ui-boundary.md:210)).

**Recommendation.** Retain the seam, reduce the registry to roughly 20–30 commands, and
keep dialogs out of commands. A user-facing adapter may ask “Save first?”; an automation
caller receives a structured error or explicit pending result. Never let a text widget
mutate the model directly.

**Why it ranks third.** This prevents keyboard, menu, script, and test behavior from
drifting apart without requiring WordCraft's ribbon or agent infrastructure.

### 4. Treat layout as a shared geometry service, not a painting detail

**Observed WordCraft fact.** WordCraft flows from document to shaped paragraphs and lines,
to pages, to a display list, then to screen/PNG/PDF backends. [`DocLayout`](../crates/layout/src/lib.rs:432)
and [`ParaLayout`](../crates/layout/src/para.rs:303) retain source-byte mappings used by
both rendering and hit testing. The reports also found that live layout is cached by
revision, while export requests a fresh print layout
[`Session::export_layout()`](../crates/engine/src/session.rs:770).

**Recommendation.** Use one screenplay layout adapter and one display-list boundary for
write mode, page preview, and PDF. Implement a fixed screenplay page profile, role indents,
explicit breaks, and a small set of keep rules. Start with a full pagination rebuild after
an edit; add incremental page-frontier caching only after a measured long-script benchmark
shows it is necessary.

**Why it ranks fourth.** Correct page breaks and caret geometry are product behavior, but
premature incremental layout is a complexity trap.

### 5. Make the native project the source of truth

**Observed WordCraft fact.** Its I/O dispatch supports many formats and distinguishes
document saves from lossy exports in [`file.save`](../crates/engine/src/cmd/file.rs:184).
Its JSON path serializes the current document, but repository documentation treats JSON as
an agent/document-model format rather than a stable screenplay project format
[`file-format-parity.md`](../docs/file-format-parity.md:40). Safe save is explicitly not
crash recovery, locking, or conflict detection (see
[Stage 2 persistence](stage2/03-persistence-formats-recovery.md:93)).

**Recommendation.** Define one versioned native project format first: deterministic UTF-8
JSON in v1, with a project identifier, metadata, ordered elements, stable element IDs,
roles, text/runs, page profile, and schema version. Use atomic replacement and a separate,
bounded recovery snapshot. Treat TXT, PDF, DOCX, and later FDX as explicit import/export
boundaries with warnings; none should silently become the canonical source.

**Why it ranks fifth.** Persistence is a product boundary, not merely a serializer. A
small, owned schema is easier to migrate and safer than making an interchange format define
the domain.

### 6. Keep portability in dependency direction, not in day-one feature count

**Observed WordCraft fact.** The repository keeps lower layers independent of egui,
eframe, winit, and rfd, and uses separate desktop/web shells
[`docs/architecture.md`](../docs/architecture.md:9). The web host has different file
semantics from the desktop host.

**Recommendation.** Keep the core compilable without a windowing toolkit. Support one
desktop target first; preserve a host boundary so a future web shell is possible. Do not
implement desktop event integrations, WASM, MCP, printing, and every platform at once.

## Adopt, modify, defer, reject

| WordCraft idea | Decision | Small-app form | Reason |
|---|---|---|---|
| Paragraph text, runs, split/join, range edits | **Adopt** | One body story; paragraphs are elements | Strong editing kernel; directly fits scripts |
| UTF-8 byte offsets and cluster-aware movement | **Adopt** | Keep internal byte offsets and validate boundaries | Avoids Unicode corruption; already proven in [`tests_typing.rs`](../crates/engine/src/tests_typing.rs:537) |
| Anchor/focus selection | **Adopt** | One ordinary stream selection | Direction matters for typing and drag behavior |
| Copy-on-write snapshots | **Adopt if reused** | `Arc` snapshots or an equivalent cheap immutable snapshot | Makes undo practical; do not invent it twice |
| Stable paragraph style IDs | **Modify** | Built-in screenplay roles plus optional explicit role field | Styles render; roles carry meaning |
| Generic `Document` stories/blocks | **Modify** | One body sequence; no tables or secondary stories in MVP | Preserve a future path without exposing Word breadth |
| Command registry and session transaction | **Adopt, reduce** | 20–30 commands; small typed error/result API | Highest leverage shared boundary |
| Typing-aware undo coalescing | **Adopt** | Group continuous typing; split at navigation/role transitions | Matches writer expectations |
| Full Word keymap/ribbon/catalog | **Reject for MVP** | Static shortcuts and a compact command palette | No product value before the writing loop |
| Paragraph shaping, hit testing, display list | **Adopt or wrap** | Reuse if dependency cost is lower than replacement | Geometry is hard and affects editing plus export |
| General Word pagination | **Modify** | Fixed page profile and screenplay keep policy | Retain primitives, hide sections/columns/notes |
| Incremental pagination | **Defer** | Full rebuild first, benchmark 120–180 pages later | Avoid optimizing an unmeasured bottleneck |
| Atomic sibling-temp save | **Adopt** | Native project only; report failures without clearing dirty state | Prevents partial document replacement |
| Recovery journal/snapshot | **Adopt early** | Rotating bounded snapshots, including unsaved projects | Safe save alone is not recovery |
| DOCX | **Defer** | Optional adapter after native format and TXT | Broad but semantically ambiguous and lossy |
| FDX | **Defer, then prioritize as first rich adapter** | Explicit XML mapping and loss report | Screenplay-specific, but not needed to validate editing |
| TXT | **Adopt later** | Plain export/import with an explicit loss contract | Cheap rescue format; not role preserving |
| PDF | **Adopt after pagination** | Output only, from print layout | Validates page delivery, not editable persistence |
| MCP/control server | **Reject initially** | Direct headless session tests instead | Network/security surface is not needed for MVP |
| Proofing/collaboration/tracked revisions | **Defer** | Add only after stable IDs and core editing | Requires larger semantics and persistence commitments |

## Recommended reduced architecture

### Dependency and data flow

```text
                 desktop shell (egui/eframe)
                   dialogs, clipboard, files
                              │ user intent
                              ▼
                    screenplay-ui adapter
                 keyboard/mouse → command IDs
                              │
                              ▼
       screenplay-engine: Session + commands + undo + dirty state
                    │                  │
                    │                  ├── native persistence service
                    │                  │      atomic save/recovery
                    ▼                  ▼
        screenplay-model ───────► screenplay-layout
         elements, roles,         shaped lines, pages,
         text, selection           hit testing, display list
                    │                  │
                    └──────────► screenplay-render
                                  screen / PNG / PDF

       headless CLI/test runner ───────┘
             calls the same Session commands
```

The arrows are intentionally one-way. The UI may request a layout and display it, but it
does not repair the document. Persistence serializes the model, not cached pages, caret
coordinates, or undo stacks. Rendering consumes layout/display data, not raw paragraphs.

### Reduced crate/module map

The future app does not need a workspace with WordCraft's current breadth. One crate can
contain several modules at first; split only when compile times or ownership become real
problems.

```text
screenplay/
├── app/                 # desktop binary; window, dialogs, clipboard, file picker
├── core/                # library crate; no egui or platform dependencies
│   ├── model.rs         # Project, Element, Role, TextRun, PageProfile
│   ├── position.rs      # Pos, Selection, checked UTF-8 boundary helpers
│   ├── edit.rs          # insert, split, join, delete, copy/paste fragments
│   ├── commands.rs      # command IDs, params, results, enablement, handlers
│   ├── session.rs       # transaction, undo/redo, revision, dirty state
│   ├── layout.rs        # role metrics, line wrapping, pagination, hit testing
│   ├── render.rs        # small display list and screen/PDF/PNG adapters
│   └── persistence.rs   # versioned native format, atomic save, recovery policy
├── ui/                  # focused editor, outline, page preview, status bar
└── cli/                 # optional headless command runner and inspection tool
```

**Recommendation.** If existing WordCraft crates are reused, treat them as implementation
dependencies behind these facades. Do not make screenplay code depend directly on every
Word property or every `Block` variant. The boundary is more important than whether the
code physically lives in one crate or five.

## Minimal domain model

```text
Project
  id: ProjectId
  schema_version: u32
  metadata: ProjectMetadata
  page_profile: PageProfile
  elements: Vec<Element>

Element
  id: ElementId                 # durable identity, not paragraph index
  role: Role
  text: String
  runs: Vec<TextRun>             # optional in the first editing slice
  force_page_break_before: bool
  scene_id: Option<SceneId>      # or derive scene grouping from headings initially

Role = Title | SceneHeading | Action | Character | Dialogue
     | Parenthetical | Transition | Note

Selection = { anchor: Pos, focus: Pos }
Pos = { element_id or structural location, utf8_byte_offset }

Session
  project: Project
  selection: Selection
  undo/redo: bounded history
  revision: u64
  dirty: bool
  layout_cache: derived/transient
```

This is a recommendation, not a claim that WordCraft uses durable element IDs. The
document report explicitly found that current positions are structural and are not rebased
through edits ([Stage 2 model report](stage2/01-document-model-and-editing.md:135)). For a
single-user MVP, structural locations are enough for the active caret; durable IDs are for
scene navigation, persistence, and future annotations. They must not be used to pretend
that page numbers or x/y coordinates are stable identity.

Runs can be omitted from the first vertical slice if the UI has no inline emphasis. Keep
the conceptual slot so adding italic emphasis does not force a semantic redesign. Notes
should be a role only if they are part of the authored script; a separate comment/review
system is intentionally out of scope.

## Command, session, and persistence boundaries

### Commands

Start with a deliberately boring command set:

* `text.insert`, `text.new_paragraph`, `text.backspace`, `text.delete`, `text.paste`;
* `selection.move`, `selection.extend`, `selection.select_all`, `selection.next_element`;
* `screenplay.set_role`, `screenplay.next_role`, `screenplay.insert_scene`;
* `edit.undo`, `edit.redo`;
* `project.new`, `project.open`, `project.save`, `project.save_as`;
* `export.pdf`, `export.txt`, `view.set_mode`, and `document.inspect`.

Each command should declare an ID, parameters, whether it mutates the project, an
enablement predicate, and a typed result/error. `Enter` should dispatch a domain-aware
new-paragraph command: split the current element, then choose the next role through an
explicit transition table. A recommended initial table is SceneHeading → Action,
Character → Dialogue, Parenthetical → Dialogue, Dialogue → Character or Action, and
Transition → SceneHeading or Action. Make the choice undoable and test it; do not hide it
in a toolbar callback.

### Session

The session is the in-memory transaction boundary, not the whole application. It owns the
project, selection, revision, dirty flag, bounded undo/redo, command registry, and derived
layout cache. A mutating command should:

1. validate all parameters and hostile sizes before mutation;
2. snapshot the project, selection, history state, and typing-group state;
3. perform model edits;
4. validate invariants and clamp the selection;
5. increment revision, mark dirty, invalidate derived layout, and return a result; or
6. restore the snapshot on error.

File writes, dialogs, clipboard access, and printing are external side effects. Keep them
in a host service or an explicit persistence operation whose failure cannot be mistaken for
a successfully rolled-back in-memory edit. The user-facing adapter can implement “ask to
save”; the engine must remain usable without a window.

### Persistence

The native file should contain only durable project data. A minimal v1 JSON envelope is:

```text
{
  "format": "screenplay-project",
  "version": 1,
  "project": { "id": ..., "metadata": ..., "page_profile": ..., "elements": [...] }
}
```

Use deterministic serialization, bounded parsing, explicit migration functions, and an
unknown-new-version error. Save by serializing first, writing a sibling temporary file,
syncing where supported, and replacing the destination only after preparation succeeds.
Keep a separate recovery directory with a bounded rotating snapshot containing project ID,
base-file identity, schema version, sequence, timestamp, and model snapshot. Recovery must
offer a candidate; it must never silently overwrite the original project.

## Pagination and rendering strategy

### First implementation

Use one fixed `PageProfile` with explicit paper dimensions, margins, font, leading, and role
indents. Resolve every role to a paragraph box before line breaking:

* scene headings and action use the main text measure;
* character cues use a narrower/indented measure and keep with the first dialogue line;
* dialogue uses its screenplay measure and may split when too long;
* parentheticals stay with the related dialogue where possible;
* transitions use their own alignment/indent rule;
* notes are visually distinct and may be hidden from export only if that is an explicit
  project setting.

Use point-based geometry and source-byte mappings. Do not insert spaces or tabs to simulate
indents. Build both continuous write mode and fixed page preview from the same role metrics;
only the arrangement of pages differs. PDF export must always request print metrics, not the
current viewport width or zoom.

### Pagination policy

Begin with generic line splitting, explicit page breaks, `keep-with-next`, and bounded
widow/orphan behavior. Compile screenplay relationships into those constraints before
pagination rather than special-casing the raster renderer. Include an escape hatch: if a
dialogue block is taller than a page, it must split rather than create an infinite loop or
large blank page.

Rebuild the document pagination after each committed edit in the MVP. Cache shaped
paragraphs and the complete layout by revision and profile. Only introduce a pagination
frontier cache after testing a 120–180 page script and measuring downstream page movement;
the layout report warns that paragraph caching alone does not make pagination incremental
([Stage 2 layout report](stage2/02-layout-pagination-rendering.md:27)).

## UI scope

The first UI should be calm and screenplay-specific:

1. a single editor canvas with reliable text/IME input;
2. a compact role indicator and role-change command palette;
3. a scene outline/sidebar showing scene headings and jumping by durable ID;
4. a page-preview toggle with page count and current scene/page status;
5. New/Open/Save/Save As, Undo/Redo, Export PDF, and a visible error/status area.

Defer ribbon customization, backstage screens, complex dialogs, comments panes, review
markup, tables, floating objects, media, and broad formatting galleries. The UI owns focus,
mouse coordinates, platform shortcuts, clipboard adaptation, and dialogs. It asks the
session for caret rectangles and hit-tests; it never derives caret geometry from character
counts.

## First vertical slice

The first slice should prove the entire writing loop, not merely serialize a struct:

1. Create a blank project containing one valid `Action` element and a fixed page profile.
2. Launch the desktop editor with a headless-capable core session.
3. Type a scene heading, press Enter, and verify the next element becomes `Action`.
4. Insert `Character`, press Enter, type a cue, press Enter, type dialogue, and wrap it.
5. Move the caret, select text, Backspace, Undo, and Redo through commands.
6. Show the same project in continuous mode and fixed page preview.
7. Save the native project atomically, close/reopen it, and verify roles, text, IDs, and
   visible pagination.
8. Export one PDF page from the same layout/display pipeline.

The slice is complete only when the direct command tests and the UI path produce the same
semantic result. It does not need FDX, DOCX, collaboration, MCP, spelling, or a polished
outline editor to be valuable.

## Safety and testing gates

### Per-change fast gate

* formatting, compiler checks, and strict Clippy/no-panic production linting;
* model invariants for ordering, valid IDs, split/join, Unicode boundaries, and nonempty
  projects;
* command tests for valid, missing, wrong-type, oversized, and disabled parameters;
* failed-command equality: project, selection, undo/redo, and dirty state remain unchanged;
* typing coalescing, role transitions, Backspace across element boundaries, and exact
  undo/redo;
* native serialize/parse semantic round trips and bounded malformed-input rejection;
* deterministic pagination fixtures asserting page count, order, margins, bounds, and no
  overlaps.

### Release gate

* interrupted-save and rename-failure tests preserve the last good project;
* recovery restart offers the newest valid snapshot and ignores stale/corrupt candidates;
* external modification does not get silently overwritten if conflict detection is enabled;
* PDF export leaves native dirty state unchanged;
* a small desktop smoke test covers typing, Enter, Backspace, save, reopen, resize, and a
  recoverable error;
* one portability compile check is run for the next promised target, rather than claiming
  all platforms prematurely.

Property tests should generate bounded valid edit sequences and assert that
serialize/reload preserves normalized semantic state. Geometry tests should prefer structural
assertions over pixel goldens until font and renderer choices are pinned. This follows the
testing report's warning that large test counts and brittle screenshots can obscure the
actual data-loss risks ([Stage 2 testing report](stage2/05-testing-safety-portability.md:282)).

## Explicit anti-goals

These are guardrails, not a list of future promises:

* Do not build a Word clone or a 400-command parity catalog.
* Do not model screenplay meaning with visual formatting, spaces, tabs, or capitalization.
* Do not let UI widgets mutate the document outside the session command path.
* Do not add tables, fields, headers/footers, footnotes, floating shapes, tracked changes,
  mail merge, equation editing, or arbitrary stories to make the architecture feel complete.
* Do not make DOCX or FDX the native source of truth before the native schema exists.
* Do not call sibling-temp replacement “crash recovery” without a recovery snapshot and
  restart policy.
* Do not add incremental pagination, collaboration, or stable annotation rebasing before
  measuring or requiring them.
* Do not add MCP, a remote control protocol, or agent-specific transport merely because the
  command API is automation-friendly.
* Do not promise every platform, web support, system-font parity, or printing in the first
  editing milestone.
* Do not use proprietary editor assets or fonts; follow the repository's clean-room and
  attribution rules in [`AGENTS.md`](../AGENTS.md:18) and [`ATTRIBUTION.md`](../ATTRIBUTION.md).

## Final decision

Adopt WordCraft's safety and boundary ideas, not its product scope. Build the smallest
semantic paragraph editor that can type a scene, transition roles predictably, paginate it
deterministically, save it without destroying the previous version, recover it after a
crash, and test all of those behaviors headlessly. Once that vertical slice is trustworthy,
the next highest-value additions are scene navigation, FDX interchange, richer PDF output,
and measured long-script performance—in that order. Everything else must earn its place by
solving a demonstrated screenplay-writing problem.

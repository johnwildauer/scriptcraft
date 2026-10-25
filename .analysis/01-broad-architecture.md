# WordCraft broad architecture research

> Stage 1 source-grounded pass for a Rust beginner, with a deliberately small screenplay-writing app as the future target.
>
> **Scope and method:** I inspected the workspace manifest, the desktop/CLI/web entry points, the engine session and I/O code, the document/layout/render roots, and the shared UI. Statements labelled **Observed** describe the current source. Statements labelled **Recommendation** are deliberately smaller-app advice, not claims about the existing product. Line references point to the inspected source; the committed architecture summary is useful orientation but this report prioritises implementation symbols.

## 1. Executive map

**Observed.** WordCraft is one Rust Cargo workspace containing reusable libraries plus three application shells. The desktop program is [`apps/wordcraft/src/main.rs`](../apps/wordcraft/src/main.rs:235), not the UI crate's `lib.rs`: it parses arguments, configures `eframe`, creates a `Session`, injects operating-system services, and starts the native event loop at [`eframe::run_native()`](../apps/wordcraft/src/main.rs:309). The UI crate supplies the reusable `WordApp`; the engine owns document editing behaviour; lower crates provide the model, formats, layout and rendering.

The shortest mental model is:

```text
desktop/web/CLI host
        │
        ▼
  WordApp (egui UI) ── commands ──► Session (engine)
                                      │
                         Document ◄──┘
                           │
                 layout ───┴─── I/O / PDF / render
                           │
                    pages and pixels/files
```

**Observed.** The repository's own layer table says the intended direction is L0 geometry; L1 document and fonts; L2 layout and format readers/writers; L3 raster/PDF/proofing; L4 engine; L5 MCP; L6 egui UI; then application binaries. It also states that lower layers must not depend on egui/eframe/winit/rfd ([`docs/architecture.md`](../docs/architecture.md:9)). The Cargo workspace enforces membership and centralises versions/path dependencies in [`Cargo.toml`](../Cargo.toml:1).

## 2. Cargo workspace and custom crates

### What “workspace” means here

**Observed.** [`[workspace]`](../Cargo.toml:1) makes the repository a coordinated Cargo build: `crates/*`, `apps/*`, and [`xtask`](../Cargo.toml:3) are members, and `default-members` selects the normal build set. Cargo resolves shared package metadata (Rust 2024 edition, version, license, minimum Rust version) from [`[workspace.package]`](../Cargo.toml:6), while [`[workspace.dependencies]`](../Cargo.toml:13) gives internal crates stable local names such as `wordcraft-doc`, `wordcraft-engine`, and `wordcraft-ui-egui`.

For a beginner, a crate is a separately compiled Rust library or binary with an API boundary and its own manifest. A workspace does **not** mean every crate is one giant module: each crate can be tested and depended on independently, while Cargo builds the dependency graph together. The custom `wordcraft-*` crates are the project's own libraries, split by responsibility rather than third-party packages.

### The important custom crates

**Observed.** The source-level responsibilities are:

* [`wordcraft-geom`](../crates/geom/Cargo.toml) is the units/rectangle foundation.
* [`wordcraft-doc`](../crates/doc/src/lib.rs:1) is the editable document model. A `Document` has a main body plus secondary stories (headers, footers, notes, comments and text boxes); blocks are paragraphs or tables and are `Arc`-shared for cheap snapshots.
* [`wordcraft-fonts`](../crates/fonts/Cargo.toml) supplies font discovery, metrics and shaping.
* [`wordcraft-layout`](../crates/layout/src/lib.rs:1) converts a document into pages, lines and placed objects, and provides hit testing.
* [`wordcraft-docx`](../crates/docx/src/lib.rs) and [`wordcraft-docbin`](../crates/docbin/src/lib.rs) handle Microsoft document formats; [`wordcraft-formats`](../crates/formats/src/lib.rs) handles text/Markdown/HTML/RTF/ODT/LaTeX.
* [`wordcraft-render`](../crates/render/src/lib.rs:1) rasterises layout display items; [`wordcraft-pdf`](../crates/pdf/src/lib.rs) exports PDF; [`wordcraft-proof`](../crates/proof/src/lib.rs) handles spelling/grammar/hyphenation.
* [`wordcraft-engine`](../crates/engine/src/lib.rs:1) is the central application-independent editing engine: `Session`, command registry, undo, view state and I/O dispatch.
* [`wordcraft-ui-egui`](../crates/ui-egui/src/lib.rs:1) is the Word-like front end, kept above the engine.
* [`wordcraft-mcp`](../crates/mcp/src/lib.rs) exposes commands to agents, while [`wordcraft-control-key`](../crates/control-key/src/lib.rs) supports the desktop control bridge.

**Recommendation for a screenplay app.** Keep the crate boundaries conceptually, but do not inherit the full feature count. A small app likely needs `geom`, a reduced `doc`, `layout`, `render`, `engine`, one UI crate, and a simple project/text exporter. Defer DOC binary compatibility, most drawing/objects, proofing languages, MCP, Zotero, mail merge and the 400+ command catalog until a real user need justifies their complexity.

## 3. Actual desktop entry point and startup flow

### Process startup

**Observed.** [`main()`](../apps/wordcraft/src/main.rs:235) performs this sequence:

1. Installs logging and the panic hook, then parses `--control`, `--sample`, `--version`, and positional file paths ([`main()`](../apps/wordcraft/src/main.rs:235)).
2. Builds [`eframe::NativeOptions`](../apps/wordcraft/src/main.rs:268): title/app id, initial and minimum window size, drag-and-drop, title-bar choices and icon. Windows graphics and non-macOS event-loop choices are conditional.
3. Loads user fonts and saved window geometry before creating the native window ([`load_user_fonts()`](../apps/wordcraft/src/main.rs:165), [`saved_window()`](../apps/wordcraft/src/main.rs:178)).
4. On macOS, installs Apple-event handling before the event loop so Finder/Dock document-open events are not missed ([`apple_events::AppleEvents::install()`](../apps/wordcraft/src/main.rs:303)).
5. Enters `eframe`'s native loop. Its app-creator closure chooses a region, creates either a sample or blank document, wraps it in [`Session::new()`](../crates/engine/src/session.rs:563), injects [`Services`](../crates/ui-egui/src/lib.rs:88), and constructs [`WordApp::new()`](../crates/ui-egui/src/lib.rs:459).
6. Applies UI preferences and the custom dictionary, optionally replaces the blank document after a saved region preference, watches the desktop theme, and optionally starts the JSON-lines control server ([`main()`](../apps/wordcraft/src/main.rs:312)).
7. With no file/sample/control request, opens the Start screen; for each positional file it dispatches [`file.open`](../crates/engine/src/cmd/file.rs:31) through [`WordApp::run()`](../crates/ui-egui/src/lib.rs:597) ([`main()`](../apps/wordcraft/src/main.rs:339)).

Each frame then calls the desktop wrapper's [`App::logic()`](../apps/wordcraft/src/main.rs:42), which polls platform events, calls [`WordApp::logic()`](../crates/ui-egui/src/lib.rs:1310), services dialogs, tracks geometry and handles close/quit. The separate [`App::ui()`](../apps/wordcraft/src/main.rs:76) calls [`WordApp::ui()`](../crates/ui-egui/src/lib.rs:1497) to paint the window. On exit, it finishes print jobs, saves preferences and removes the control key file ([`App::on_exit()`](../apps/wordcraft/src/main.rs:80)).

### What `WordApp` adds

**Observed.** [`WordApp`](../crates/ui-egui/src/lib.rs:343) contains the `Session`, persistent `UiState`, injected platform [`Services`](../crates/ui-egui/src/lib.rs:88), dialogs, canvas, printing and UI-only state. Its [`WordApp::run()`](../crates/ui-egui/src/lib.rs:597) is the interactive boundary: it may open an input dialog, ask whether unsaved work should be saved, fill clipboard parameters, and then call the engine. Agent/control calls use [`execute_user()`](../crates/ui-egui/src/lib.rs:641) instead and deliberately do not open dialogs. This is an important distinction for automation and for a future small app.

## 4. End-to-end data flow: open, edit, layout, render, save

### Open

**Observed.** The `file.open` command accepts a path or base64 bytes ([`specs()`](../crates/engine/src/cmd/file.rs:29)). It calls [`io::open_path_with()`](../crates/engine/src/io.rs:254) or [`io::open_bytes_with()`](../crates/engine/src/io.rs:106), then installs the resulting `Document` in the session and records the path/password state ([`open()`](../crates/engine/src/cmd/file.rs:137)). The dispatcher first uses extension/signature detection, then routes DOCX/DOC/format imports through [`io_ext::open()`](../crates/engine/src/io_ext.rs:5). `ensure_nonempty()` prevents an empty editable story.

Desktop file access is bounded and checks automation roots; HTML-relative images are separately resolved with size/budget limits in [`LocalImages`](../crates/engine/src/io.rs:274). That is robust but more policy than a tiny screenplay editor initially needs.

### Model and editing

**Observed.** [`Document`](../crates/doc/src/lib.rs:306) is structured, not merely a string: body/parts contain `Arc<Block>` values, and positions are `(StoryRef, Path, byte offset)` ([`Pos`](../crates/doc/src/lib.rs:212)). Paragraphs contain runs, properties and inline objects; tables and stories are nested. `Session` adds selection, dirty/revision state, view/preferences, undo/redo, registry, layout cache and file path ([`Session::new()`](../crates/engine/src/session.rs:563)).

Every user-visible operation is a JSON-parameterised [`CommandSpec`](../crates/engine/src/lib.rs:58). The same registry is used by UI, CLI, MCP and the control channel. [`Session::run()`](../crates/engine/src/session.rs:1116) looks up the id, checks enabled state/protection, snapshots mutating state, runs the command, catches an escaped panic, touches the revision, validates/clamps selection and restores the prior state on error. This is the central safety/undo contract.

### Layout and hit testing

**Observed.** [`Session::layout()`](../crates/engine/src/session.rs:727) computes a cache key from document revision, view mode, dimensions, proofing, numerals, outline and deletion/balloon settings. On a miss it calls [`wordcraft_layout::layout()`](../crates/engine/src/session.rs:751), producing `DocLayout` pages containing `Placed` lines, images, shapes, tables and hit-test objects ([`Placed`](../crates/layout/src/lib.rs:194)). The layout crate shapes paragraphs, breaks lines, paginates sections/columns/tables, handles headers/footers/objects and exposes caret/hit-testing APIs. `Session::export_layout()` deliberately creates a print-oriented layout without editor proofing marks ([`export_layout()`](../crates/engine/src/session.rs:770)).

An edit therefore changes the model and revision; the next UI read of layout recomputes or reuses cached work. Selection and mouse coordinates are translated through layout, not guessed from raw character counts.

### Draw and export

**Observed.** Layout's display module turns placed pages into draw items. [`render_page()`](../crates/render/src/lib.rs:402) uses `vello_cpu` and caches decoded media; [`Rendered::to_png()`](../crates/render/src/lib.rs:57) encodes pixels. The engine's PNG path first creates a print layout, selects a page and calls the renderer ([`io_ext::render_png()`](../crates/engine/src/io_ext.rs:39)). PDF walks the same conceptual layout through `wordcraft-pdf`, so screen/page geometry and export are related rather than separate document models.

The renderer declaration is [`render_page()`](../crates/render/src/lib.rs:402); its call site is confirmed in [`io_ext::render_png()`](../crates/engine/src/io_ext.rs:39).

### Save

**Observed.** `file.save`/`file.saveAs` eventually call [`io::save_path_with()`](../crates/engine/src/io.rs:423) and [`io::save_bytes_opts()`](../crates/engine/src/io.rs:222). The extension selects DOCX, JSON, PDF, PNG or a `formats` exporter via [`io_ext::save()`](../crates/engine/src/io_ext.rs:22). The session's `dirty` flag and path are then updated by the file command/UI. Desktop autosave is a UI policy: [`WordApp::autosave_tick()`](../crates/ui-egui/src/lib.rs:1468) writes only to a file explicitly saved in-session and disables itself after failure. The web host cannot write local files; it downloads bytes and maintains a browser `beforeunload` guard ([`web::services()`](../apps/wordcraft-web/src/web.rs:200), [`guard_unload()`](../apps/wordcraft-web/src/web.rs:246)).

## 5. The three application shells

### Desktop

**Observed.** [`apps/wordcraft`](../apps/wordcraft/Cargo.toml:1) is the native `eframe`/wgpu host. It owns OS integration that should not leak downward: native file dialogs, clipboard, printing, window geometry, Apple events, control server and platform theme/accessibility hooks. The reusable UI and engine remain portable.

### CLI

**Observed.** [`apps/wordcraft-cli/src/main.rs`](../apps/wordcraft-cli/src/main.rs:1) is a headless consumer of the same engine. It opens a file into a `Session` ([`open()`](../apps/wordcraft-cli/src/main.rs:46)), invokes the same command ids for convert/info/text/inspect/render/run, lists the registry, reports parity and starts or bridges MCP ([`run()`](../apps/wordcraft-cli/src/main.rs:176)). It is consequently valuable as a small-app test harness: screenplay transformations can be deterministic commands without launching a window.

### Web

**Observed.** [`apps/wordcraft-web/src/main.rs`](../apps/wordcraft-web/src/main.rs:1) is only a WASM/browser shell; native execution prints a build hint. [`web::start()`](../apps/wordcraft-web/src/web.rs:18) starts `eframe::WebRunner`, creates the same `Session` and `WordApp`, and supplies browser-specific services. Open/drop/paste arrive asynchronously through an `Inbox`; save/export downloads bytes; there is no TCP control channel ([`apps/wordcraft-web/src/main.rs`](../apps/wordcraft-web/src/main.rs:7)).

## 6. Focused Stage 2 shortlist for a small screenplay app

### Rank 1 — document model and screenplay semantics

**Investigate:** [`Document`](../crates/doc/src/lib.rs:306), [`Paragraph`](../crates/doc/src/para.rs), styles/properties, positions, and the text commands under [`crates/engine/src/cmd`](../crates/engine/src/cmd/mod.rs:1).

**Why first:** screenplay writing needs a stable representation for scenes, action, character cues, dialogue, parentheticals, transitions, notes and metadata. WordCraft's model is powerful but paragraph/run/table/story oriented; the key Stage 2 decision is whether screenplay elements are paragraph styles, a screenplay-specific block type, or a separate semantic layer mapped to ordinary paragraphs. This choice affects persistence, navigation, formatting, import/export and future outlining.

**Recommendation:** start with ordinary paragraphs plus a small typed screenplay role/style enum and explicit scene metadata only where required. Avoid tables, inline objects and arbitrary nested stories in the first vertical slice.

### Rank 2 — command/session editing contract

**Investigate:** [`CommandSpec`](../crates/engine/src/lib.rs:58), [`Registry`](../crates/engine/src/lib.rs:102), [`Session::run()`](../crates/engine/src/session.rs:1116), typing tests and undo grouping.

**Why:** typing, Enter, backspace, role changes, scene insertion, find, undo and save need predictable behaviour. The command architecture is a strong reuse point because UI, CLI tests and automation converge on one API, but the current registry/catalog is much larger than the product needs.

**Recommendation:** retain command dispatch and transactional undo, but define perhaps 20–40 screenplay commands first. Treat selection positions and Unicode boundaries as a dedicated risk study; offsets are byte offsets even though they must remain character-boundary safe.

### Rank 3 — layout/pagination and screenplay page rules

**Investigate:** [`LayoutOptions`](../crates/layout/src/lib.rs:139), [`wordcraft_layout::layout()`](../crates/layout/src/lib.rs:1706) and paragraph layout, pagination, hit testing and line numbering.

**Why:** screenplay users care about page count, Courier-like metrics, margins, dialogue indentation, scene headings, orphan/widow rules and page breaks. WordCraft already has a general pagination engine, but general Word layout may obscure screenplay-specific invariants.

**Recommendation:** reuse line shaping, caret geometry and pagination, then add a narrow screenplay layout policy. Prove page-break examples early; do not begin with tables, floating objects, columns or bidirectional typography unless requirements demand them.

### Rank 4 — save format and recovery

**Investigate:** [`io::open_bytes_with()`](../crates/engine/src/io.rs:106), [`io::save_bytes_opts()`](../crates/engine/src/io.rs:222), safe save, autosave and the CLI round trip.

**Why:** writing software must not lose drafts. DOCX compatibility is not automatically the best primary format for a small app; it brings a large fidelity surface. The current source also shows a subtle risk: saving a format that cannot preserve everything can discard unsupported content, while autosave intentionally only covers explicitly saved, lossless-enough files.

**Recommendation:** choose an owned, versioned screenplay project format first (JSON or a compact structured text format), add atomic save and recovery journals, and make DOCX/FDX/plain text export explicit. Stage 2 should test crash/interruption, malformed input, repeated save/reopen and version migration.

### Rank 5 — focused editor UI and host services

**Investigate:** [`WordApp::logic()`](../crates/ui-egui/src/lib.rs:1310), [`WordApp::ui()`](../crates/ui-egui/src/lib.rs:1497), canvas, keyboard routing and [`Services`](../crates/ui-egui/src/lib.rs:88).

**Why:** the existing UI is a full Word shell (ribbon, backstage, panes and many dialogs). A screenplay app needs a calmer editor, scene navigation, format palette, page/outline split and reliable keyboard flow. Reusing the whole shell would import complexity and interaction assumptions that are not screenplay-centric.

**Recommendation:** keep the service-injection idea and a thin `WordApp`-like host, but build a focused UI surface: document list/outline, editor, page preview, status bar and a handful of commands. Add desktop first; make web/CLI follow once the model and command contract are stable.

### Rank 6 — outline/navigation/search

**Investigate:** engine outline/find state and the UI navigation pane.

**Why:** scene headings are the screenplay equivalent of headings and are more valuable than Word's broad formatting catalog. This can become the first feature that differentiates the app: jump between scenes, reorder safely, show scene metadata and search dialogue/action separately.

**Recommendation:** make scene identity and navigation semantic, not inferred only from visual formatting. Define what happens to links/bookmarks when a scene is reordered.

### Rank 7 — proofing, collaboration and automation

**Investigate later:** [`wordcraft-proof`](../crates/proof/src/lib.rs:1), review/track-changes, MCP/control and web collaboration.

**Why lower:** useful, but not necessary to validate a small screenplay editor. Proofing can be added behind the same layout annotations; collaboration and MCP depend on stable document identity, conflict semantics and persistence.

## 7. Important simplifications and risks

* **Observed complexity risk:** the current architecture is Word-parity-shaped: many stories, objects, revisions, fields, formats and 422-ish commands (the exact catalog count is documented in [`docs/architecture.md`](../docs/architecture.md:27)). Copying it wholesale would make the small app harder to understand and test.
* **Recommendation — distinguish semantic text from presentation.** Screenplay roles should survive export and style changes. A model based only on font/indent properties risks losing meaning when users reformat.
* **Recommendation — preserve the command boundary.** It is the best architectural idea to keep: one command path makes UI, CLI tests and future agents agree. Reduce the command set, not the transaction/error discipline.
* **Observed portability risk:** desktop file paths/dialogs and web asynchronous bytes are intentionally different [`Services`](../crates/ui-egui/src/lib.rs:88). A future app should keep file access at the shell boundary and test both synchronous/headless and asynchronous/browser flows.
* **Observed safety strength:** the code denies common panic patterns at crate roots and [`Session::run()`](../crates/engine/src/session.rs:1193) catches escaped panics. This is good inheritance, but it is only a last resort; screenplay commands still need hostile-parameter and recovery tests.
* **Recommendation — make pagination a product decision early.** A screenplay app can initially offer continuous editing plus a reliable page preview. Full Word-compatible sections, headers, footnotes and floating objects should not be prerequisites for the first usable release.
* **Observed asset/licensing constraint:** the repository's agent rules require original/openly licensed assets. A small app should prefer code-drawn UI icons and existing permitted fonts, and should not borrow proprietary editor imagery.

## 8. Suggested Stage 2 order

1. Trace and prototype screenplay roles in the document model.
2. Trace typing/selection/undo through a minimal command set and add focused tests.
3. Measure the existing paragraph layout and pagination against screenplay examples.
4. Design the native project format, atomic save/recovery and explicit interchange exports.
5. Sketch the focused desktop UI around scenes, editor, outline and page preview.
6. Only then decide which web, CLI, proofing, automation and compatibility features are worth carrying forward.

This order follows dependency reality: semantics precede presentation, commands connect every host, layout depends on the model, and persistence must be trustworthy before polishing the shell.

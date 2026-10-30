# Container Blueprint: ScriptCraft Desktop Application

## Purpose and boundary

ScriptCraft Desktop is the independently runnable local application for an individual screenwriter.
It owns the window, input and IME integration, focused editor, page presentation, file selection,
clipboard, status and error presentation, and platform adapters. It delegates canonical screenplay
semantics, transactional editing, fixed-profile layout, native serialization, and export contracts
to the reusable, headless [Screenplay Core](../components/screenplay-core.md) .

Transient per-window presentation and interaction state is not canonical project content. The
desktop does not maintain a second authoritative screenplay buffer, and desktop UI or platform APIs
do not cross into the core. The application composes the
[Screenplay Writing Loop](../features/screenplay-writing-loop.md) and supports the
[screenplay-writing-loop requirements](../../requirements/features/screenplay-writing-loop.md) .

## Runtime and delivery

- **Technology/runtime:** A local Rust desktop process built with Cargo. The desktop uses a UI
                          framework and native file dialogs; framework and dependency versions are
                          implementation choices, not permanent architecture constraints.
- **Build/package:** The application is a runnable package in the Cargo workspace and depends on the
                     reusable screenplay-core library and desktop presentation/platform adapters.
- **Deployment/distribution:** Local desktop application. No packaging method, distribution channel,
                               or deployment pipeline is prescribed.
- **Configuration:** Window and presentation state remains separate from native screenplay projects.
                     No preference store or configuration location is prescribed; credentials and a
                     service configuration boundary are not required by this local product.
- **Dependencies:** The reusable Screenplay Core, desktop UI framework, native file selection, local
                    filesystem, and applicable windowing, input/IME, and clipboard facilities. The
                    application has no required network service, account system, or remote identity
                    boundary.

## Interfaces and composition

- **Inputs:** Writer keyboard, text, IME, paste, pointer and navigation input; semantic role and
              toolbar actions; file selection; open, save, and export requests; window and
              presentation interactions; and window/platform state reports.
- **Outputs:** Rendered continuous and fixed-page views; accessible toolbar and status interactions;
               operation state and errors; native project files; and noncanonical PDF or
               deliberately lossy plain-text export results.
- **Internal areas:** The desktop shell and editor adapt writer and window input to the core
                      session. The core owns the canonical project and transactional editing, shared
                      fixed-profile layout and source mappings, schema-versioned native
                      serialization, and noncanonical export contracts. Desktop presentation
                      consumes core editing and layout results; desktop file and platform adapters
                      own file selection, native file operations, clipboard and window interaction.
                      The [technical context](../technical-context.md) maps these boundaries.

## Contracts

### Canonical session, native files, and exports

- Startup and New obtain a new project from the core seed: exactly one empty Scene Heading with a
  valid initial caret at element 0, byte offset 0. Semantic edits, role changes, undo and redo flow
  through the transactional core session; the desktop does not duplicate semantic policy in a
  separate text model.
- New never replaces a dirty session without the writer's explicit informed consent. This contract
  does not prescribe a dialog, action sequence, or alternate recovery policy. Snapshot candidates
  are not an intended desktop contract; no replacement recovery capability is implied.
- Opening replaces the active session and its associated path only after reading, parsing, and
  validation succeed. Saving prepares the native project and atomically replaces the named
  destination; the session becomes clean and its associated path changes only after successful
  replacement. Failed open or save preserves the valid in-memory session and prior valid native
  destination. Failures are reported without discarding unsaved work.
- Native projects use schema version 1, default specified missing legacy fields, and reject
  malformed or unsupported versions safely. Cross-version migrations are not promised. The canonical
  project, not transient caret, layout, selection, undo, or presentation state, is persisted.
- PDF and deliberately lossy plain-text exports are noncanonical. They do not replace the native
  project or mark it clean. Plain text communicates that semantic roles, stable identifiers, and
  page layout are not preserved. PDF uses shared fixed print layout and preserves non-ASCII
  characters. Export destination and a particular desktop text-export interaction are not
  prescribed.
- Export or file-operation failure is surfaced while preserving the native project and valid
  in-memory state. Export behavior does not redefine native save state.

### Editor, layout, and typography

- One stable whole-document editor surface owns focus, keyboard, text, IME, and paste input.
  Rendered page rectangles are presentation and hit-test geometry, not separate editors. Semantic
  mutation and undo are delegated to the core session.
- Pointer placement uses shared document-global nearest body-line geometry and source-safe clamping;
  title-page clicks do not move body selection. Caret-affecting commands reveal the caret only when
  it is outside the actual scroll clip. Passive scrolling and pointer selection clear stale reveal
  intent rather than recentering the viewport.
- Shared cached layout represents the current project and complete fixed page profile. Replacing a
  project or session cannot reuse geometry derived from a prior project, even if revision and
  profile values match. Viewport scale and page zoom are presentation transforms, not fixed-layout
  inputs.
- New projects use Courier Prime. Supported stored font families remain supported without migration;
  bundled regular Courier Prime is available to the desktop presentation. Fixed character-cell
  geometry governs screenplay placement. PDF preserves non-ASCII characters without prescribing a
  font-encoding or embedding policy that would conflict with that behavior.
- A focused collapsed caret blinks at 500 ms, with a new visible phase after revision, caret, or
  focus change. Range selections and unfocused idle frames do not schedule blink repaint.

### Toolbar and native header

- The default transient typed toolbar order is New, Open, Save, Save As, Flexible Space, Elements,
  Flexible Space, Export PDF. Actions route through desktop adapters. Elements reflects the selected
  session role and routes all seven role choices through the transactional session; toolbar
  composition is not native project content.
- Action Items use icon-first borderless controls with small explicit padding as the hover/click
  response. Hover changes lightness only. Their centered labels remain outside that response, retain
  normal text color, and remain available to accessibility surfaces without a redundant
  visible-label tooltip. Action controls and the Elements control request the standard arrow cursor;
  the whole-document editing surface requests the text/I-beam cursor. Labels and painted pages do
  not create competing cursor owners.
- Elements uses a centered fixed-width outer group and selected role value, truncating the value as
  needed. Its standard arrow, keyboard, response, and popup behavior is retained; popup choices are
  left-aligned.
- Allocation uses measured label-aware widths. The Elements override is 128 logical points and is
  shared by allocation and rendering. A Space consumes 72 points; horizontal row padding is 4 points
  per side; item gaps are 4 points; overflow reserves 32 points; top and bottom padding are 0 and 2
  points. Flexible Space items divide remaining width equally. Narrow layouts retain the leftmost
  fitting items and expose actionable overflow entries, including nested Elements choices. Fitting
  items, labels, and overflow remain within the clipped content edge.
- On macOS the operating system owns the native window header; the toolbar begins at the top of
  application content below it. No app-owned title strip or maximize gesture is part of the target.
  App chrome uses the shared framework surface without inset or separator, but does not promise
  pixel equality with operating-system chrome or additional native header behavior.

### Status and visible page

- The status bar has three presentation regions. Noninteractive text owns no focus, activation,
  pointer response, or cursor behavior. Interactive controls have accessible names and keyboard
  activation. The right-hand controls appear in this order: Sprint Timer, Focus Mode, Appearance,
  and page zoom. Overflow preserves access and presents one actionable representation at a time.
- Status reports clean/unsaved and operation/error state, distinguishes a clean unnamed project from
  one previously saved, and keeps errors and bounded privacy-safe diagnostics reachable. It does not
  display the native project path, cumulative frame count, or a demo placeholder. Status and
  diagnostics text remain single-line and clipped; footer height stays bounded, with no
  clickable-control guarantee at a physically unusable viewport.
- The visible-page indicator derives from actual rendered sheet rectangles and the editor scroll
  clip, independently of caret/source position. It selects the sheet with greatest vertical overlap;
  without overlap it selects the nearest vertical interval. Exact ties choose the earlier rendered
  sheet. Counts include an optional title page. Missing, invalid, or stale geometry produces an
  explicit unavailable state until fresh geometry is available. Page status does not scroll, reveal
  the caret, change focus, or take editor input ownership.

### Page zoom and UI scale

- Each window has one transient whole-percent page zoom, initially 100% and bounded inclusively from
  75% to 300%. The accessible slider and adjacent preset menu share this value; presets are exactly
  75%, 90%, 100%, 125%, 150%, 175%, 200%, 250%, and 300%.
- At 100%, the page presentation width is 360 logical points, independent of viewport width. The
  page centers when it and its 16-point exterior side buffers fit; otherwise it remains reachable on
  both axes. Horizontal offsets are preserved when valid and otherwise clamped. Painting, hit
  testing, caret reveal, and source mapping use consistent forward and inverse presentation
  transforms.
- Zoom changes page presentation only. It does not change semantic content, session history or dirty
  state, fixed pagination or source ranges, PDF output, or global toolbar/status sizing.
- A supported native pinch is routed once to the same bounded page-zoom value; event delivery is
  platform/backend-dependent and no distinct squeeze event is promised. A finite positive native
  pixels-per-point value, including 1.0, is applied once with UI zoom 1.0. When absent or invalid,
  UI zoom uses a 2.3 fallback, replaced by a later valid native value. UI scale and page zoom remain
  independent.

### Appearance, timer, focus mode, and scheduling

- Appearance is transient per window and offers System, Day, Night, and Midnight. System follows the
  desktop framework's resolved theme or fallback. Day uses a light page; Night and Midnight use dark
  pages, with Midnight darker than Night. The page palette supplies paper, text, border, selection,
  caret, and IME colors. Page text and IME text retain at least 4.5:1 contrast against composited
  backgrounds, selected text at least 4.5:1 against selection, and caret and visible selection
  boundary at least 3:1 against adjacent page colors. Appearance does not modify project, layout,
  session, zoom, or export state; native operating-system theme behavior is not guaranteed.
- Sprint Timer is an accessible transient per-window Countdown/Stopwatch action. Countdown defaults
  to 25 minutes, accepts whole minutes from 1 through 1440, and preserves invalid input without
  starting. It supports Start, Pause, Resume, End, completion, and Reset; ending an active or paused
  countdown restores the captured input. Stopwatch supports Start, Pause, Resume, and End and
  displays elapsed hours without wrapping. Running or paused timers continue across popup dismissal;
  idle/completed dismissal and Reset restore the 25-minute default. Timing uses monotonic
  elapsed-time/deadline calculations and bounded repaint while running. Timer state is not persisted
  and does not change screenplay state. Completion shows the captured duration and makes at most one
  asynchronous best-effort OS notification request per completed sprint; request acceptance is not
  delivery, and in-app completion remains available.
- Focus Mode requests entry or exit only when the window reports a known fullscreen state. Accepted
  requests remain pending until reports permit reconciliation. Unknown, pending, timeout, and
  external-transition states do not imply success, trigger implicit retries, or queue an inverse
  request. The operating system/window manager owns fullscreen transition and restoration. A 600 ms
  stable-report interval is a duplicate-suppression heuristic, not a native completion
  acknowledgement.
- Diagnostics and repaint scheduling are bounded and privacy-safe. Diagnostics expose aggregate
  event/performance information only, not screenplay text, source text, or serialized project
  content. Active input, caret blinking, timer progression, and unresolved window transitions may
  schedule repaint; idle or paused state does not require an unconditional timer repaint.

## Decisions

- **Keep desktop APIs outside the core.** The core remains a reusable headless capability so
                                          semantic editing, layout, persistence, and exports can be
                                          exercised without a desktop window; the desktop owns
                                          presentation and platform adapters.
- **Keep the native semantic project canonical.** Transactional session state and safe atomic
                                                  replacement protect the writer's source of truth;
                                                  PDF and lossy text remain derived review outputs
                                                  and never change native cleanliness.
- **Require informed consent before replacing dirty work.** New does not silently discard an unsaved
                                                            session, while the specific consent
                                                            interaction remains a product/UI choice.
- **Keep print geometry independent of presentation.** Shared fixed-profile layout and complete
                                                       project/profile cache identity preserve
                                                       document geometry across window scaling and
                                                       project replacement.
- **Use the OS-owned macOS header.** The native header remains outside app-owned toolbar content; no
                                     application title strip or unproven native gesture is
                                     introduced.

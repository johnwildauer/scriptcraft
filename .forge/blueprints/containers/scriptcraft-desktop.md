# Container Blueprint: ScriptCraft Desktop Application

## Container Summary

ScriptCraft Desktop is the native runnable boundary for an individual
screenwriter. It owns the window, input/IME path, focused editor, page preview,
dialogs, clipboard, file selection, status/error presentation, and platform
services. It delegates semantic edits, layout decisions, and persistence
contracts to the core rather than maintaining a second authoritative text
buffer. The UI Polish WO-001–004 behavior below is implementation-backed by
separate work-order validations, with native OS observations explicitly limited
where noted.

## Document status

- **Status:** `draft` — implementation evidence exists for the desktop shell and selected UI contracts; independent blueprint review remains separate
- **Owner:** Forge initialization agent
- **Last updated:** 2026-10-30
- **Technical context:** [`../technical-context.md`](../technical-context.md)
- **Evidence basis:** desired architecture from `.analysis`, verified app/core behavior, and later work-order evidence linked in revision history; not every desired contract is independently reviewed

## Infrastructure

- **Technology/runtime:** Rust native desktop process; app package uses eframe/egui 0.29 and delegates native windowing through its locked backend stack. Platform-specific behavior remains evidence-dependent.
- **Build/package:** Cargo workspace; app package `scriptcraft` depends on eframe, rfd, and the reusable `scriptcraft-core` crate.
- **Deployment/distribution:** Local desktop executable/package; packaging method unknown and out of current initialization scope.
- **Configuration:** Keep UI preferences/session state separate from native screenplay projects; exact preference location unknown.
- **Dependencies:** Core capability and selected desktop UI framework; do not add MCP, web, or broad format dependencies for MVP.

## Entry points and boundaries

- **Inputs:** Keyboard/IME text, semantic role actions, selection/navigation, file dialogs, open/save/export requests, and page-preview interactions.
- **Outputs:** Updated session state, rendered continuous/page views, status/errors, native project writes, PDF output, and lossy text output.
- **External systems:** Local filesystem and desktop windowing/clipboard APIs; no network/authentication/MCP in MVP.
- **Internal areas:** `screenplay-core` capability, UI adapter, platform file service, and renderer/export adapters.

## System Contracts

### Key Contracts

- The desktop shell never becomes the source of truth for screenplay text.
- Startup and toolbar New delegate project creation to the core `Project::new()` seed; the desktop shell does not duplicate the Scene Heading seed or alter the existing Enter/Tab transition and opened-project delegation boundaries.
- Every semantic mutation goes through the core editing/session boundary and is undoable when it changes the project.
- Viewport size and zoom do not redefine print pagination.
- The desktop consumes the core's role-pair geometry and profile font identity unchanged: new projects use `Courier Prime`, existing supported stored families are retained without migration, and the supplied regular face is embedded into the application build and selected for default document text. Fixed character-cell placement continues to own screen geometry. PDF consumes the same fixed layout and the existing deterministic Type1 family mapping; it does not embed the supplied TTF or apply an independent spacing policy.
- A failed open/save/export reports an error and does not silently replace valid in-memory or on-disk project state.
- The painted page-layout editor has one stable whole-surface egui response/ID for
  focus and keyboard ownership. Page rectangles remain render/hit-test geometry;
  they are not independent focusable editors. While that response is focused, a
  narrow egui focus-lock filter protects editor navigation and the shell dispatches
  text, paste, key, and IME events through the existing editing session. Retained
  native diagnostics expose only bounded focus/event categories and counters, never
  document text or a second buffer.
- Pointer events are resolved against the shared layout's document-global body
  nearest-line query. Page margins, blank areas, inter-page gaps, and vertical
  document edges therefore select source-safe body positions; clicks inside a
  title page retain title-page ownership and do not move the body selection.
- Caret-affecting commands request conditional nearest reveal only when the caret
  is outside the actual scroll clip; passive scrolling and pointer selection clear
  stale reveal intent and do not recenter the user's viewport.
- The desktop owns a transient typed toolbar Item composition. Its initial
  default order is New, Open, Save, Save As, Flexible Space, Elements, Flexible
  Space, Export PDF. Action Items carry stable identity, text, icon/presentation
  metadata, and existing desktop command adapters. The Elements Item derives its
  selected/displayed label from the current session focus and routes all seven
  supported role choices through `EditingSession::change_role()` and the shared
  transactional mutation/error presentation; it does not own role policy or
  semantic project state. The toolbar composition is not persisted in native
  projects, and actionable Items use a centered icon-first borderless control
  plus small explicit padding as the hover/click response above a separate text
  label. The label is outside that response, its normal color is unchanged, and
  it remains available to accessibility surfaces without a redundant visible-label
  tooltip. The control
  response receives the lightness-only hover affordance. The Elements Item uses
  the same centered fixed-width outer group, placing a ComboBox above its
  centered `Elements` label. Toolbar metadata exposes a typed width policy;
  the shared resolver applies measured label-aware widths to ordinary Items and
  the evidence-backed 128-point Elements override to both allocation and
  rendering, while the label remains outside the action response.
  The closed Elements ComboBox centers only its selected role galley within
  the arrow-excluded control content region, using truncation when needed;
  its standard arrow, response/ID, keyboard behavior, and popup ownership
  remain stock. Open popup role choices remain left-aligned and continue to
  route through the existing transactional session operation.
- Toolbar allocation is transient and deterministic: `Space` consumes one
  72-point width, while rendered normal Items use their complete group width
  including label text measured through the active egui style/font galley path.
  The allocator and renderer use
  one shared row-width budget that includes the 4-point horizontal row-padding
  budget, 4-point inter-item gaps (half egui's default horizontal gap), and the
  32-point overflow reservation when present. These values are the named
  manual geometry controls `TOOLBAR_NORMAL_ITEM_WIDTH`,
  `TOOLBAR_ROW_PADDING`, `TOOLBAR_ITEM_GAP`, and
  `TOOLBAR_OVERFLOW_WIDTH`. The named vertical controls are
  `TOOLBAR_TOP_PADDING` = 0 points and `TOOLBAR_BOTTOM_PADDING` = 2 points.
  `Flexible Space`
  Items divide the remaining width equally after that budget, and narrow widths
  retain the leftmost fitting Items while placing a far-right `>>` text menu
  after them. Spacing Items are excluded from overflow; overflow action Items
  dispatch through the existing desktop adapters, and an overflow Elements Item
  exposes its role choices through a nested submenu-like menu that still routes
  role changes through `EditingSession`. The allocator and renderer derive one
  shared available width from the actual clipped `TopBottomPanel` content rect.
  A fully fitting row remains inside that rect, including the rightmost Item and
  its label, after outer panel/frame/viewport insets; the overflow reservation
  is included in the same edge budget.
- The current app configures visible native decorations, title text/titlebar,
  and titlebar buttons (`with_decorations(true)`, opaque presentation,
  `with_title_shown(true)`, `with_titlebar_shown(true)`,
  `with_fullsize_content_view(false)`, and
  `with_titlebar_buttons_shown(true)`). The top egui panel renders the toolbar
  directly; the central panel then renders the live document. The app does not
  currently render a separate app-owned title strip or a title-strip
  double-click maximize action. Native titlebar appearance and behavior remain
  platform-owned.
- Toolbar action controls and the Elements outer group request the standard
  arrow cursor; the Elements label remains decorative and does not own a
  competing response. The existing whole-surface painted editor response
  requests the text/I-beam cursor, while page rectangles remain geometry rather
  than independent cursor or focus owners.
- The desktop status bar owns three presentation regions and a typed status Item
  capability model: noninteractive text, clickable text, and clickable icons.
  Noninteractive labels are response-free and do not compete with the editor for
  focus or cursor ownership. The right-hand controls are ordered Sprint Timer,
  Focus Mode, Appearance, and page zoom; a deterministic width-aware layout
  moves the non-fitting suffix into one accessible overflow disclosure without
  duplicating actions. Interactive controls have accessible names and keyboard
  activation. The status adapter projects current `EditingSession::dirty()`
  state (including clean-but-never-named projects), operation state, and visible
  errors; it does not display the cumulative frame count, native path, or initial
  in-memory-demo placeholder. Status and Diagnostics text clip as single lines;
  overflow exposes page/clean/operation state and preserves error visibility.
  Bounded Diagnostics expose aggregate counters and event categories only,
  never screenplay/source text or serialized project contents. Diagnostics
  remains available for the current release scope, with removal deferred until
  pre-release.
- Appearance is transient per window and starts in System mode. System follows
  egui's resolved theme/fallback; Day uses light visuals, Night dark visuals, and
  Midnight a darker derived palette. The page palette follows the resolved light
  or dark theme and supplies paper, text, border, selection, caret, and IME
  colors. Rendered tests measured contrast for the authored opaque page palettes;
  native OS-theme propagation and native visual appearance have not been
  observed. Appearance does not persist or change core/project/export state.
- Sprint Timer is a transient per-window Countdown/Stopwatch model independent
  of its popup visibility. It uses monotonic elapsed-time/deadline calculations,
  bounded repaint scheduling while running, and no timer-owned idle/paused loop;
  active/paused state survives popup dismissal, while relaunch persistence is
  not provided. Completion commits a one-shot attempt per timer generation and
  submits a fixed-text notification request asynchronously through the local
  macOS `osascript` adapter. Process acceptance is not delivery evidence; OS
  authorization, identity, presentation, delivery, and sleep-inclusive timing
  are unverified. In-app completion remains available when the request is
  unavailable.
- Focus Mode observes this viewport's optional reported fullscreen state and
  issues one framework borderless-fullscreen viewport command per accepted
  activation when the state is known and no request is unresolved. Pending,
  unknown, timeout, and external-transition states are presented without an
  optimistic success claim; there is no implicit retry or inverse queue. The
  backend/window manager owns native transition and restoration. The app's
  600 ms stable-report interval is a duplicate-suppression heuristic, not a
  native completion acknowledgement. No macOS app/window smoke was performed;
  green-button/Space/menu-bar equivalence, Escape behavior, transition
  completion, and exact geometry/monitor restoration remain unverified.
- The left status Item is a viewport-visible rendered-sheet indicator, not a
  caret/page-source indicator. The editor publishes actual painted page
  rectangles and its actual `ScrollArea` clip with layout revision, presentation
  scale, viewport bounds, and publication frame. Because the bottom status panel
  is composed before the editor, it accepts only the immediately preceding
  geometry snapshot when revision, viewport, scale, and input/reveal invalidation
  signals remain current; otherwise it reports unavailable until the editor
  publishes fresh geometry and schedules a repaint. Selection is the maximum
  vertical overlap, with exact ties favoring the earlier rendered sheet; a
  no-overlap gap selects the nearest page interval, again favoring the earlier
  sheet on a tie. Numbering is one-based over all rendered pages, including an
  optional title page. Status projection never requests caret reveal, scrolls,
  edits, focuses, or replaces the editor's single whole-surface response.
- The desktop owns one transient integer zoom percentage per window, initialized
  to 100 and bounded to 75–300. Its accessible status-bar slider and adjacent
  nine-entry preset menu read and update that same value; the displayed percent
  stays accurate for intermediate values. The 100% page baseline is 360 egui
  logical points independent of viewport width. For the actual scroll area's
  inner clip width `V`, page width `P`, and 16-point side buffer `B`, content width
  is `max(V, P + 2B)`; the page is centered when it fits and scrollable
  horizontally when the buffered page exceeds the inner clip. Resize preserves
  a valid horizontal offset and otherwise clamps it to the current range. The
  percentage multiplies only page presentation scale and never changes global
  egui zoom, toolbar/status sizing, the core layout cache key, semantic
  coordinates, or persistent project data.
  The page editor uses two-axis scrolling for overflow while maintaining its
  vertical multi-page route; painted page rectangles, pointer hit tests, caret
  reveal, and selection/source mapping use the same forward/inverse scale. Zoom
  invalidates visible-page geometry until a fresh page/clip snapshot is
  published. Zoom alone does not mutate the editing session, saved/dirty or
  undo state, fixed pagination/source ranges, or fixed-layout PDF output.
- Supported native `egui::Event::Zoom` input is routed once at the app boundary
  to that same bounded transient page-zoom value; positive factors magnify and
  factors below one shrink. egui-winit 0.29.1 maps winit 0.30.13
  `WindowEvent::PinchGesture` to this event (positive native delta magnifies,
  negative delta shrinks). The pinned winit API documents native pinch delivery
  only on macOS and iOS; other platforms/backends may not deliver it, and no
  separate squeeze event is promised. The zoom event is not dispatched as text,
  selection, or editing input.
- Effective UI pixels-per-point uses a valid finite positive per-viewport native
  scale once at UI zoom factor 1.0; 1.0 is valid. When native scale is missing or
  invalid, the desktop uses UI zoom factor 2.3, replacing that fallback when a
  valid native value arrives. Resolution occurs in the pre-input hook against
  the incoming viewport scale. The resulting egui UI scale affects controls and
  framework-managed pointer/AccessKit/IME geometry; the independent document
  percentage remains a page presentation transform only. These are
  implementation/rendered-test results, not native multi-monitor observation.
- The unified title-bar/toolbar `TopBottomPanel` uses the egui style's open-widget
  `Widgets::open::weak_bg_fill` as its explicit app-chrome fill through a local
  shared frame helper. The shared frame has no inner/outer inset and disables
  the panel separator; this is the
  framework-exposed deterministic application-surface source and does not
  mutate global theme visuals or toolbar geometry. Native OS chrome remains
  platform-owned and exposes no portable sampleable title-bar color; therefore
  the contract is app-surface color alignment, not an unobservable pixel-
  equality claim against native OS chrome.
- A focused collapsed caret uses egui animation/repaint state to alternate visible
  and hidden phases every 500 ms. Its animation identity includes the rendered
  revision, caret position, and focus generation, so revision/caret changes and
  focus regain begin a distinct initial visible phase; focus transitions update
  the transient generation only at the transition boundary. Range selections,
  focus loss, and unfocused idle frames suppress the caret and do not schedule
  blink repaint. No timer thread or model blink state is used.

### Integration Contracts

- UI-to-core contract: typed semantic operations/results; exact API unknown until implementation.
- Core-to-render contract: backend-neutral pages/display data with source mappings; exact representation desired but unimplemented.
- Core-to-filesystem contract: versioned native project bytes, atomic replacement, bounded recovery candidates, and explicit export status.

## Architecture Decision Records

### ADR-001: Desktop-first, core-independent boundary

**Context:** The product is desktop-first, but dependable behavior must be testable without a window and future platform work must not force a semantic rewrite.

**Decision:** Keep the screenplay core independent of desktop UI APIs and place window/input/platform services at this container boundary.

**Consequences:** Headless tests and future hosts remain possible; exact framework selection is deferred to workspace discovery.

## Completion evidence placeholder

`TODO: link verified package manifest, build/run evidence, desktop smoke evidence, and independent blueprint review.`

## WO-006 verified responsiveness facts

- The desktop owns a bounded privacy-safe `PerformanceDiagnostics` record for cumulative frame, event, layout-build, paint, and repaint-request counts/timings. It stores no document text and exposes only aggregate counters in the status surface.
- The desktop session adapter owns the reusable core `LayoutCache` instance, keyed by editing revision plus the complete persisted page profile. Unchanged reads hit an immutable shared layout; edits and any profile geometry/font/policy change miss it. Viewport scale remains a paint transform and does not invalidate layout. The headless benchmark invokes this same adapter rather than an `Arc`-clone proxy.
- Repaint requests include active surface input, focused caret phase scheduling, explicit focus transitions, and bounded timer progression while a timer is running (including while its popup is closed). Focus Mode polls only during pending/recovery/settling transitions; idle/paused/completed timers and otherwise focused idle frames do not request a timer-owned unconditional repaint. Native notification, fullscreen, and general visual smoke was not performed; rendered-frame/source evidence must not be described as native OS behavior.

## Revision history

| Date | Change | Evidence/rationale |
|---|---|---|
| 2026-10-25 | Created MVP desktop container boundary | User-approved desktop-first scope and `.analysis`; implementation/framework not claimed |
| 2026-10-25 | Recorded foundation app seam | WO-001 verified a dependency-free native Rust binary and headless startup path; visible UI remains deferred |
| 2026-10-25 | Recorded unified editor and named persistence host behavior | WO-009/WO-010 implementation evidence; app owns the optional current named path and native `rfd` selection, while core owns JSON and atomic-save semantics; recovery remains outside the UI |
| 2026-10-25 | Recorded hardened painted-editor input boundary | WO-011d implementation evidence: one stable whole-surface response/ID, response-driven focus, egui focus-lock filtering, focused event ownership, and bounded privacy-safe diagnostics; native typing remains a separate unproven manual gate |
| 2026-10-26 | Recorded continuous focused-caret contract | WO-002 rendered evidence: egui-owned 500 ms phase scheduling is active only for a focused collapsed caret; range selection and unfocused idle frames suppress blink repaint, with revision/caret identity forming the transient reset boundary. |
| 2026-10-27 | Recorded native viewport presentation request | WO-001 implementation evidence: the eframe 0.29 native runner requests hidden title text, a transparent titlebar, and full-size content through the installed `ViewportBuilder` API; unsupported-platform behavior remains platform-controlled. |
| 2026-10-27 | Recorded icon-only toolbar hover ownership | WO-002 implementation evidence: action Items allocate an independent icon-plus-padding response for click/hover fill while the centered label remains outside the response with normal text color. |
| 2026-10-27 | Recorded unified title-bar/toolbar surface ownership | WO-002 implementation evidence: one zero-inset, no-separator app-chrome frame owns the title and toolbar surface; the title retains a full-width response and the toolbar retains explicit row allocation padding. |
| 2026-10-28 | Recorded item-based status and viewport-page ownership | Approved status-bar WO-002 implementation: desktop status items project live app/session state and consume fresh rendered-page/scroll-clip geometry without changing core layout or editor input ownership. |
| 2026-10-28 | Recorded transient page zoom and two-axis navigation | Approved status-bar WO-003 implementation: one window-owned page-only percentage state preserves the responsive baseline, source/input mapping, and fixed core/export contracts. |
| 2026-10-29 | Recorded status-content removal and horizontal clipping | WO-005 implementation: no displayed frame/path/demo placeholder; status and Diagnostics content are clipped to single-line regions, while clean/unsaved, operation/error, visible page, zoom, and Diagnostics-now behavior remain. Diagnostics removal is deferred until pre-release. |
| 2026-10-29 | Recorded supported native pinch and UI scale | WO-006 implementation: supported egui zoom events update transient page-only zoom; native pixels-per-point is applied once with exact 2.0 fallback when unavailable, distinct from page zoom. Native delivery remains platform/backend dependent. |
| 2026-10-30 | Reconciled fixed page geometry, UI scale, status actions, appearance, timer, and Focus Mode | UI Polish WO-001–004 implementations and separate passing validations; source/rendered evidence records but does not prove native notification delivery, native fullscreen completion, OS theme delivery, or native visual smoke. Corrected the app-owned title-strip/native-titlebar description to current source while preserving the unresolved AC-WRITING-005.6 discrepancy; status-bar WO-008 removed only the separate project-metadata heading. |

## WO-001 verified foundation facts

- `apps/scriptcraft` is the runnable Rust binary boundary and targets the native `aarch64-apple-darwin` host observed by `rustc -vV`.
- WO-001 intentionally selected no UI/window framework dependency; `cargo run -p scriptcraft` prints `scriptcraft foundation ready`.
- The visible-window smoke path is unavailable/deferred in this foundation WO, not claimed as passed.

## WO-006 verified persistence facts

- The app host owns [`StdFileSystem`](../../../crates/scriptcraft-core/src/persistence.rs) and path selection, while the core owns bounded bytes, atomic-save semantics, and recovery candidate contracts.
- The focused host exposes native file-dialog Open, Save, and Save As actions. The app owns the optional current named path and displays it in status; Open replaces the session and path only after successful parse/validation, while Save marks the session clean and updates the path only after atomic replacement. Recovery APIs remain core-only and are not exposed in this UI.

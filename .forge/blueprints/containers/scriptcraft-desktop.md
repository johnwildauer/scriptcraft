# Container Blueprint: ScriptCraft Desktop Application

## Container Summary

ScriptCraft Desktop is the desired native runnable boundary for an individual
screenwriter. It owns the window, input/IME path, focused editor, page preview,
dialogs, clipboard, file selection, status/error presentation, and platform
services. It delegates semantic edits, layout decisions, and persistence
contracts to the core rather than maintaining a second authoritative text
buffer.

## Document status

- **Status:** `draft` — desired boundary; not implemented or independently reviewed
- **Owner:** Forge initialization agent
- **Last updated:** 2026-10-25
- **Technical context:** [`../technical-context.md`](../technical-context.md)
- **Evidence basis:** desired architecture from `.analysis`, plus verified WO-009/WO-010 app-boundary behavior

## Infrastructure

- **Technology/runtime:** Rust native desktop process — language confirmed; framework and OS target unknown.
- **Build/package:** Cargo is an agent-default pending manifest verification; exact command unknown.
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
- Every semantic mutation goes through the core editing/session boundary and is undoable when it changes the project.
- Viewport size and zoom do not redefine print pagination.
- The desktop consumes the core's role-pair geometry and profile font identity unchanged: new projects use `Courier`, existing supported stored families are retained, and PDF consumes the same fixed layout rather than applying an independent spacing or font policy.
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
  response receives the lightness-only hover affordance. The Elements Item uses the same centered fixed-width outer group,
  placing a ComboBox above its centered `Elements` label.
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
  `TOOLBAR_TOP_PADDING` = 2 points and `TOOLBAR_BOTTOM_PADDING` = 2 points.
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
- The native eframe viewport requests a hidden title, a transparent titlebar,
  and a full-size content view through `ViewportBuilder`'s installed 0.29 API:
  `with_title_shown(false)`, `with_titlebar_shown(false)`, and
  `with_fullsize_content_view(true)`. These are native presentation hints: the
  full-size content and titlebar behavior are macOS-specific in eframe, while
  other platforms may ignore some or all of them and retain platform-managed
  decorations. The app must not make pixel-level cross-platform titlebar
  claims from these requests.
- The toolbar `TopBottomPanel` uses the egui style's open-widget
  `Widgets::open::weak_bg_fill` as its explicit app-chrome fill through a local
  frame helper. This is the
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
- Repaint requests are limited to active surface input, focused caret phase scheduling, and explicit focus transitions; an otherwise focused idle frame does not request an unconditional repaint. Native long-document smoke was not available; headless benchmark/rendered instrumentation is the required evidence boundary.

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

## WO-001 verified foundation facts

- `apps/scriptcraft` is the runnable Rust binary boundary and targets the native `aarch64-apple-darwin` host observed by `rustc -vV`.
- WO-001 intentionally selected no UI/window framework dependency; `cargo run -p scriptcraft` prints `scriptcraft foundation ready`.
- The visible-window smoke path is unavailable/deferred in this foundation WO, not claimed as passed.

## WO-006 verified persistence facts

- The app host owns [`StdFileSystem`](../../../crates/scriptcraft-core/src/persistence.rs) and path selection, while the core owns bounded bytes, atomic-save semantics, and recovery candidate contracts.
- The focused host exposes native file-dialog Open, Save, and Save As actions. The app owns the optional current named path and displays it in status; Open replaces the session and path only after successful parse/validation, while Save marks the session clean and updates the path only after atomic replacement. Recovery APIs remain core-only and are not exposed in this UI.

# Technical Context

This map describes ScriptCraft's desired technical structure and runtime boundaries. It is a navigation aid, not an implementation inventory.

## Project map

| Area | Path | Role in the target structure |
|---|---|---|
| Cargo workspace | [`Cargo.toml`](../../Cargo.toml) | Defines the workspace for the desktop application and reusable core crate. |
| Desktop application | [`apps/scriptcraft/`](../../apps/scriptcraft/), [`main.rs`](../../apps/scriptcraft/src/main.rs), [`script_view.rs`](../../apps/scriptcraft/src/script_view.rs), [`theme.rs`](../../apps/scriptcraft/src/theme.rs), and [`src/`](../../apps/scriptcraft/src/) | Runnable local desktop shell and its UI/platform modules; application orchestration, Script View presentation and interaction, semantic appearance palettes, and native file interaction remain desktop-owned. |
| Screenplay core | [`crates/scriptcraft-core/`](../../crates/scriptcraft-core/), [`lib.rs`](../../crates/scriptcraft-core/src/lib.rs), [`editing.rs`](../../crates/scriptcraft-core/src/editing.rs), [`text_state.rs`](../../crates/scriptcraft-core/src/text_state.rs), [`layout.rs`](../../crates/scriptcraft-core/src/layout.rs), [`persistence.rs`](../../crates/scriptcraft-core/src/persistence.rs), and [`exports.rs`](../../crates/scriptcraft-core/src/exports.rs) | Reusable headless library for semantic editing, bounded FDX view-state offset mapping, shared layout, native-project persistence, and noncanonical export adapters. |
| Core verification | [`crates/scriptcraft-core/tests/`](../../crates/scriptcraft-core/tests/) | Automated tests for core behavior independently of the desktop UI. |
| Bundled typeface | [`fonts/CourierPrime/`](../../fonts/CourierPrime/) | Bundled Courier Prime font assets used by the screenplay experience. |
| Product and technical intent | [`requirements/`](../requirements/), [`blueprints/`](./) | Durable product requirements and architecture descriptions; start at the [Forge index](../_index.md). |

## Runtime and delivery

ScriptCraft runs as a local Rust desktop application built and launched through the Cargo workspace. The desktop application is the independently runnable product boundary. It delegates screenplay semantics and shared document operations to the headless-capable core library, which is reusable by the desktop rather than a second runnable container. No distribution mechanism or deployment pipeline is prescribed here.

## Technology and boundaries

The workspace uses Rust and Cargo. The desktop boundary contains the eframe/egui presentation layer and native-file-dialog integration; UI and platform concerns stay outside the reusable core. The core owns semantic session/project operations, fixed-profile layout, native persistence, and noncanonical export adapters. Framework and dependency versions are implementation choices rather than permanent architecture constraints.

The desktop appearance model is owned by [`theme.rs`](../../apps/scriptcraft/src/theme.rs), above egui's binary Light/Dark theme selection. Transient System resolves through the framework-reported theme or configured fallback; Day, Night, and Midnight are concrete application palettes. One semantic palette maps to complete egui visuals and distinct toolbar, Script Workspace, status, shared-control, overlay, and feedback groups. The Script Workspace page palette remains desktop-only and supplies paper, text, hidden text, border, selection, selection boundary, caret, and IME colors. Toolbar and status panel surfaces are independent even when their initial compatibility values match. Item/icon overrides remain above toolbar palette defaults; status-local default, center, and hovered foreground overrides remain above status palette defaults. The palette is not persisted and is not part of screenplay core, document layout, or exports.

The initial Day/Night/Midnight RGBs in `theme.rs` are image-derived estimates, not sampled values, and remain subject to user side-by-side calibration. The theme builder preserves the existing page selection, selected-text, caret, IME, generic egui selection/text-cursor, hyperlink, warning, and error colors. Screenplay selection fill remains independent from egui global selection; the zoom slider's trailing fill continues to use egui selection while its rail is scoped to the inactive widget fill. The status slider's custom knob continues to share status default/hover foreground behavior. Toolbar, screenplay-page, and status borders have independent local width and semantic palette-color owners. The toolbar divider is drawn along the toolbar's lower edge only and does not add frame borders or change toolbar allocation. The page border retains its middle-stroke kind and is clipped by the page painter. The status panel disables egui's generic framework separator and uses a stroke-free frame; its single explicit full-width top divider uses the status palette border color and is painted against the outer panel rectangle, without introducing side or bottom borders. The visible page-selection boundary uses an explicit palette stroke so contrast can be reviewed against the lighter Night paper.

The desktop toolbar owns a declarative item catalog in
[`toolbar_items.rs`](../../apps/scriptcraft/src/toolbar_items.rs), while
[`toolbar.rs`](../../apps/scriptcraft/src/toolbar.rs) owns transient allocation,
painting, popup behavior, and command/session adapters. Each item keeps identity,
label/accessibility name, caption visibility, graphic, behavior, enablement, and
layout policy separate. Graphics support no graphic, one icon, independently
specified icon groups, an independently specified primary/disclosure pair, and a
behavior-supplied text dropdown value. Per-icon point-size and enabled/disabled
color overrides take precedence over per-item color overrides, toolbar-local
color overrides, and the theme-derived enabled foreground; absent an explicit
disabled color, disabled graphics and captions use the resolved enabled color at
reduced alpha. The toolbar-wide icon variant defaults to outlined, with optional
per-icon filled/outlined override. Toolbar icon size, item text size, Elements
role-option text size, and graphic gaps are separately owned presentation
settings. The item and Elements role-option text-size settings are independently
configurable. The closed selected Elements role uses the role-option size; the
Elements caption uses the item text size. Role-option row padding remains
separate from the shared control-to-caption gap. The toolbar owns one explicit
control-to-caption gap shared by action graphics and the Elements control. Action
Items and Elements captions use one shared group-relative rectangle derived from
the nominal control-slot height and that gap, and are centered within it. Elements
caption placement is independent of the stock ComboBox response height; its
separately owned vertical optical adjustment affects only that decorative caption.
Action Items and Elements share one derived Item-group height. The exact toolbar panel
height derives from its top padding, that group, and any explicitly owned trailing
padding. The outer top-down allocation suppresses implicit vertical spacing between
those explicit extents. The closed Elements control/caption composition suppresses
post-control vertical spacing before allocating its ComboBox and remaining caption
space, keeping the child response within the shared Item-group height; its popup child
restores normal vertical spacing for role rows. Other nested controls retain their
normal spacing. These scopes keep the returned toolbar boundary aligned with its
visible panel boundary. Its divider uses a toolbar-local width and palette color and
sits at the panel's lower edge; top-level composition places Script View immediately
after that shared boundary; the margin-free Script Workspace frame and zero top
scroll-content margin add no separate boundary inset. Toolbar icon
hover expansion uses independent per-edge padding for the icon hover/click
rectangle, not glyph baseline placement. ComboBox button padding and Elements
role-option row padding remain independent from the caption gap. The overflow
menu button's egui padding remains symmetric and projects each opposite edge pair
to its average. Status icon padding remains status-local and symmetric.
Item order, enabled state, presentation, allocation, and popup geometry are
desktop transient state and are not persisted in screenplay or FDX data. The
temporary catalog order is leading Flexible Space, New, Open, Save, Save As, pre-Elements
Flexible Space, Elements, post-Elements Flexible Space, Show/Hide, Export Text,
Export PDF, trailing Flexible Space, followed by a catalog-only overflow
indicator. The four uniquely identified Flexible Spaces divide the post-budget
remainder equally when ordinary Items fit and are excluded from narrow-width
overflow. Show/Hide uses independently configurable outlined Visibility and
Keyboard Arrow Down icons; Export Text uses its outlined File Export assignment.
Both items use the disabled foreground for every graphic and caption and have no
hover presentation change, activation, focus, popup, or command behavior. In
overflow they remain disabled-colored text labels with no interaction. Enabled
New/Open/Save/Save As/Export PDF and Elements role routing remain on their
existing adapters. The overflow control uses the catalog's outlined Double
Arrows Material icon, is icon-only with an accessible name, and is rendered only
    when allocation produces overflow entries; it adds no permanent wide-layout
    width. Both the central Elements ComboBox and its nested overflow submenu derive
    their scroll area's desired height from all role-row text/button metrics using
    the independently configurable role-option font size, and inter-row spacing.
    The central closed ComboBox uses the separately adjustable
    `DEFAULT_ELEMENTS_CLOSED_COMBO_HEIGHT = 18.0` logical points as its local minimum
    interaction height, matching egui's current default interaction-height floor; content
    and button padding can make the full response taller. The setting is restored before
    caption allocation and inside the popup before role-row layout.
    Its `.height(role_popup_height)` remains the independently derived popup content
    cap and is not the closed-control height. The closed outline uses
    `DEFAULT_ELEMENTS_CLOSED_COMBO_OUTLINE_WIDTH = 1.0` logical point and is painted
    only around the closed response using the app-owned toolbar palette's hover
    foreground, distinct from the toolbar hover fill. The stock hovered solid fill
    remains intact; open/active visuals remain framework-owned. The outline is a narrow
    painter overlay rather than a `Visuals` override, so popup frames and role rows
    do not inherit a changed border style.
    The central ComboBox paints the outlined Material Unfold More glyph in place of
    egui's default triangle through its icon-painter hook; ComboBox selection,
keyboard, popup, and accessibility behavior remain framework-owned. The overflow
submenu retains its menu-owned indicator. The usable egui content viewport minus
the popup frame margins
is the only height cap; a genuinely short viewport retains bounded scrolling.

The desktop uses eframe/egui and retains `egui_material_icons` for typed icon
constants, codepoints, and filled/outlined identity. The app bundles the upstream
Material Symbols Rounded variable TTF under Apache-2.0 and registers app-owned
faces under the crate's existing family names. The app-creation callback installs
Courier Prime first and adds the Material faces to that same egui context before
app construction; the initializer from the icon crate is not called. Named
application defaults govern weight, optical size, grade, and fill. Toolbar
`IconSpec` axes overrides take per-axis precedence over those defaults; status
icons retain the shared defaults. Supported axis combinations use separate
named faces where needed to avoid reusing glyph cache entries across variation
locations. The shared rich-text adapter keeps codepoint and fill/outline identity
while choosing the named face.
The native font is a desktop presentation asset and does not enter the headless
screenplay core or change document typography.

The desktop status row is owned by [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs).
Its presentation settings define the ordinary row height and the panel's symmetric
vertical insets; the default panel height is derived from those components rather
than maintained as an independent total. The application supplies this derived
value as the bottom panel's default size, not an exact constraint. This keeps the
ordinary row at its configured height while allowing the optional Diagnostics
disclosure to expand through the panel's normal layout behavior.

The status frame is stroke-free, and the bottom panel disables the generic egui
separator. The status module owns a named local divider width and paints one full-width
top divider from the status palette against the outer panel rectangle returned by the
panel, rather than the inset content rectangle. This divider is the status surface's
only frame-like edge; no side or bottom frame borders are added. These explicit local
border owners do not change egui's global noninteractive widget colors or unrelated
panel and content geometry.

The row gives priority to the inline Timer, Focus Mode, Appearance, zoom slider,
and current-percentage controls at the right. The visible-page counter has its own
bounded left allocation, and operation/error text uses a mirrored center region.
These regions and their control hit/accessibility bounds remain clipped to the row;
content compression does not create an overflow action or compact-mode switch.
The status module owns its row-local control geometry without changing global
framework spacing or toolbar sizing. The current page reflects accepted editor
viewport geometry rather than caret position.

Timer, Focus Mode, and Appearance use their assigned Material glyphs without
visible captions; Appearance choices remain textual in the menu. Focus Mode
retains its state-based enablement and accessible description, and timer popup
placement follows its interaction rectangle. Text, icons, hover treatment, and
the zoom knob use status-local foreground ownership with optional theme-aware
overrides; popup entries keep menu-owned styling. Center operation/error text is
single-line, centered in its explicit region, and independently styled. The zoom
slider and arrowless percentage control share the window's integer zoom value;
their bounded rectangles own painting, interaction, popup anchoring, and
accessibility. The custom circular knob follows the stock slider response and is
clipped to the status row, right group, and inherited parent clip.

Detailed status diagnostics and their disclosure are controlled by
`presentation::ENABLE_DIAGNOSTICS`, a manual production-off source gate.
Disabled mode constructs no detailed projection or Diagnostics row, placeholder,
focus target, or accessibility node. When explicitly enabled for development,
the disclosure retains its collapsed default and may expand the panel beyond the
ordinary row allocation. Operational instrumentation and privacy-safe operation
errors remain independent of this presentation gate.

Script View owns independent top and bottom scroll-content page-edge spacing and
its local policy for unobscured scrolling. Actual allocation and prepaint zoom
compensation use the same asymmetric page-origin and content-extent model.

The desktop [`script_view.rs`](../../apps/scriptcraft/src/script_view.rs) module owns
the screenplay page surface: page allocation and painting, viewport culling, scroll
and presentation-scale geometry, zoom compensation, pointer selection, caret and
IME presentation, surface input routing, vertical navigation, and fixed-cell text
galley support. It operates on the application-owned session, layout, theme, and
transient view state; it does not create a second document or view-state store.
[`main.rs`](../../apps/scriptcraft/src/main.rs) retains application and document
orchestration, file operations, top-level panel composition, and integration with
toolbar and status modules. The surface publishes its allocated page geometry and
viewport state for status projection and zoom coordination, while semantic edits
continue through the core session and shared layout remains the source for page
geometry.

The editor publishes its actual allocated page rectangles, scroll clip, scale,
viewport, revision, and scroll offset after rendering; the status row consumes
only a fresh immediately preceding publication. The viewport-page tracker keeps
that geometry snapshot separate from the last valid one-based page/total selected
from it. Same-document invalidation clears the published snapshot and requests a
fresh publication while retaining the last valid display value through scrolling,
zoom, resize, reflow, invalid geometry, and same-document failures; only a fresh
valid selection replaces it. Startup begins without a retained value. Successful
New/Open activation resets both lifetimes, while failed Open and same-document
Save do not reset the retained page. This reset is explicit because session
revision alone does not identify a document. Zoom presentation uses the tracker's
retained-or-fresh value rather than an independent same-frame unavailable
override. Script View's independent top and bottom page-edge spacing belongs to
scrollable content, not a fixed viewport inset, and contributes to scroll extent
without changing core document layout. The first page can begin at the viewport
edge while the document end retains its own allowance. The Script Workspace frame
has no fixed margin; Script View disables scroll-edge fades locally without
changing unrelated scroll areas. Actual page allocation and prepaint zoom
compensation share equivalent asymmetric origins, strides, and extents: page-local
coordinates scale with zoom, while page-edge spacing, inter-page separation,
horizontal buffers, and outside-page displacement remain fixed logical geometry.
Zoom compensation is
applied before allocation and paint, clamped against matching content extents, and
keeps published page rectangles and scroll state aligned. Manual scrolling takes
precedence over an unapplied zoom request, and caret reveal is deferred while a
compensation frame is pending.

- Desktop shell owns presentation and platform/file interactions. Screenplay Core supplies reusable headless document capability, not an independently runnable container. The [writing-loop requirements](../requirements/features/screenplay-writing-loop.md) describe the composed user-facing flow.

Desktop input delegates semantic operations to the core session and project. The core's shared fixed-profile page layout supplies desktop presentation and PDF generation; viewport zoom affects presentation, not print geometry. FDX Version 6 is the canonical native save boundary and uses atomic replacement. The headless core separates an [FDX codec](../../crates/scriptcraft-core/src/fdx.rs) from filesystem persistence and couples the semantic model to ordered source-preservation state; only managed fields are regenerated. PDF and deliberately lossy plain-text exports are noncanonical outputs. Fountain conversion is a separate secondary adapter, not native persistence. Transient view state remains separate from canonical project content.

The root WindowState projection is a narrow exception to fully transient view state: the core reader
projects one bounded direct-root block into an optional typed geometry value, while the desktop owns
latest native logical geometry and a one-shot pending restore. The save boundary carries that snapshot
through the save-scoped context into a detached preservation candidate; it never becomes Project,
EditingSession history, or layout/PDF input. Save/Save As replace only direct root WindowState nodes.
The core's typed WindowState default is the shared source for fresh-process native startup and generated
or fallback FDX state: Left/Top map to outer position, while Width/Height map to inner logical size.
Imported one-shot restore remains independent of that startup default, and a complete live native
snapshot takes precedence during transactional saves.

Document points are stable 1/72-inch units owned by core layout and PDF; a default Letter sheet is
612 × 792 document points on every OS. The desktop converts those points to egui logical page
coordinates only: macOS uses 1.0 logical point per document point, Windows uses 96/72, and
unsupported platforms use neutral 1.0. Egui's native display scale applies once after that
conversion, with neutral 1.0 if the native value is absent or invalid. Page zoom multiplies the
desktop page presentation and base typography only. Persisted root TextState Scaling is only the
user's whole-percent zoom; neither OS conversion nor native display scale enters core layout, PDF,
or the saved percentage. Desktop hit testing uses the matching layout-to-presentation transform.

## Desktop application-menu implementation context

The menu system is desktop-only and leaves screenplay core, persistence, fixed layout, and exports
unchanged. [`app_commands.rs`](../../apps/scriptcraft/src/app_commands.rs) owns the shared
`AppCommand` identities, current `AppCommandAvailability` projection, dispatcher, editor-focus
restoration helper, and the one explicit `arboard` clipboard adapter. Existing File/PDF operations
and Edit session operations flow through this dispatcher; clipboard extraction reads canonical
body-element UTF-8, copies exact selected text with one newline per crossed element boundary, and
does not inherit print/export transformations. Cut writes before invoking the transactional delete;
Paste reads only on activation and invokes the existing paste operation.

[`menu_items.rs`](../../apps/scriptcraft/src/menu_items.rs) is the one ordered,
presentation-independent catalog for File, Edit, View, Format, Insert, Document, Tools, Production,
Window, and Help, matching [`final-draft-13-menus.md`](../../.reference/final-draft-13-menus.md).
Each entry distinguishes `Command(AppCommand)` from `DisabledPlaceholder(PlaceholderKind)`; the
placeholder kinds are ordinary disabled row, disclosure-only, checked, and Help Search. The 13
existing File/Edit command rows retain their IDs, labels, accessibility names, icons, shortcuts,
aliases, fresh availability projection, and dispatcher behavior. The other 132 rows are disabled
placeholders (99 ordinary, 30 disclosure, two checked, one Search); none adds an application
command or a no-op handler. The two static checked rows are Script - Page View and `blank
document.fdx`. Disclosure rows have no child content and no functional nested navigation. Help
Search is a disabled accessible label, not an input or focus target. Placeholder icons remain unset.
`LogicalShortcut` carries display/native accelerator metadata including primary, literal Control and
Option, Shift, alphabetic/punctuation/bracket/backslash/pipe/Return/arrow keys; placeholder metadata
is not input-recognition authority. [`menu.rs`](../../apps/scriptcraft/src/menu.rs) owns the
source-only preview policy, menu-local egui renderer/metrics, recognition of command-only shortcuts,
in-window accelerator-consumption policy, and the FIFO of shared command identities. Its in-window
strip retains its existing 24-logical-point height and popup height bounds. On Windows the
in-window strip owns command accelerators; on macOS the preview remains enabled while native owns
command accelerators.

[`macos_menu.rs`](../../apps/scriptcraft/src/macos_menu.rs) is compiled only for macOS and owns
main-thread native Application-menu construction separately from all ten shared catalog menus,
muda-retained menu/item handles, native accelerators, the native-ID-to-command map, event inbox, and
label/enablement synchronization. Only the 13 command rows enter the native command map. Disabled
ordinary/Search placeholders are native disabled items and may carry their display accelerators;
checked placeholders are disabled checked items, and `>` placeholders are disabled empty submenus.
No placeholder ID maps to a command, and unknown/event IDs are ignored at the FIFO handoff. The
resolved `muda` 0.17.2 handler captures the egui context, appends activated IDs to a thread-safe
inbox, and requests repaint. In that version, installing the handler replaces delivery to
`MenuEvent::receiver()`, so `main.rs` drains the handler-fed inbox rather than polling that receiver.
The next app update maps only known command IDs into the shared FIFO; native command labels and
enabled state remain synchronized through the catalog and availability projection.

[`macos_sf_symbols.rs`](../../apps/scriptcraft/src/macos_sf_symbols.rs) is the macOS-only,
main-thread-confined image adapter owned for the full `NativeMenuController` lifetime. After muda
installs the root, it retains the borrowed root `NSMenu`, resolves unique catalog-title paths to
requested native items once, and retains the resolved `NSMenuItem` handles alongside that root.
The native-only mapping is File New `plus`, Open `arrow.up.right`, Close `xmark`, Save
`square.and.arrow.down`, Save As `square.and.arrow.down.on.square`; Edit Undo `arrow.uturn.backward`,
Redo `arrow.uturn.forward`, Cut `scissors`, Copy `doc.on.doc`, Paste `doc.on.clipboard`. Save As PDF,
Delete, and Select All remain image-free. AppKit system-symbol images use an 18-point initial size
and template rendering; missing handles or symbols produce bounded static warnings and leave the
item image-free without failing startup. The adapter performs no per-frame hierarchy lookup or
image assignment; muda continues to own command IDs, labels, accelerators, events, and enablement.

The AppKit boundary is deliberately narrow: `objc2` 0.6.5, `objc2-app-kit` 0.3.2, and
`objc2-foundation` 0.3.2 are macOS-target dependencies matching muda's resolved objc2 generation.
The sole unsafe operation is the function-local retain of muda's borrowed root `NSMenu` handle in
`retain_native_menu()`, guarded by one local `#[allow(unsafe_code)]` and a safety comment covering
the borrowed-handle lifetime and main-thread check. There are no raw Objective-C message sends or
module/crate-wide unsafe allowances; package unsafe-code denial remains in force elsewhere. The
in-window Material icon renderer and shared semantic icon assignments are unchanged.

[`main.rs`](../../apps/scriptcraft/src/main.rs) orchestrates both presentations and the update
handoff. The in-window panel order is menu strip, existing toolbar, Script Workspace, then the
existing bottom status panel; the menu strip is inserted immediately before the toolbar only when
the in-window policy enables it. Each update refreshes native-window state, drains native event IDs
into the FIFO, dispatches the detached FIFO batch in activation order with fresh availability
rechecked, then routes ordinary editor input and renders the application panels. Egui menu clicks
enqueue during panel rendering and are dispatched on a later update. Recognized pressed in-window
shortcuts are extracted from raw input before editor routing only when the in-window backend owns
accelerators; unrelated keys and IME input remain. Native macOS is the sole accelerator owner while
installed, including dual preview; Windows uses the in-window backend. Edit dispatch targets the
application session, not the menu's transient focus, and requests focus/repaint for the recorded
editor surface after dispatch when available. Native item enabled states are synchronized from the
shared availability projection at the end of each update. The in-window catalog buttons retain
egui click-to-open and ordinary keyboard/dismissal behavior; a menu-local bridge switches to a
different top-level popup on hover only while a tracked catalog popup is open, and does not enqueue
a command. Rows use disabled state for placeholders, with a display-only right caret for disclosure
rows, a checkmark in the existing icon slot for the two checked placeholders, and a disabled Search
label. Command matching explicitly ignores all placeholder rows, so their shortcut labels cannot
become raw-input commands. The locked egui 0.34.3 `MenuBar`/`MenuConfig` APIs expose styling and popup close behavior,
not top-level hover switching; `MenuButton` uses click-driven `Popup::menu`, while `SubMenuButton`
hover behavior is for nested menus and does not switch an already-open sibling. The adapter uses
egui popup memory (`Popup::open_id`) rather than changing global pointer or focus state.

The in-window switch `IN_WINDOW_MENU_PREVIEW_ENABLED` is currently `true`; it is a source-only
development switch, not a preference or persisted field. Windows always presents the complete egui
catalog; macOS currently presents the native Application menu, all ten shared native catalog menus,
and the complete egui catalog preview.
Automatic production OS-only selection and disabling the preview are deferred to a separately
approved change. Current egui menu metrics in source are: bar height 24, bar/item/shortcut text
12/12/12, icon size 14, row height 22, icon slot 28, bar gap 8×0, bar padding 8×0, row padding
4×0, row atom spacing 6×2, popup padding 10×8, separator spacing/stroke 9/1, and
bar/row/popup corner radii 7/4/8 logical units. Popup width remains content-driven without custom
narrow-window reflow; appearance and geometry are not claimed to match the reference pixel-for-pixel.

Known boundaries remain explicit: disclosure-only placeholders have empty content and do not enable
functional submenu navigation; no behavior is added for other placeholder rows. The 10-menu catalog
and disclosure rows supersede the prior File/Edit-only/additional-menu and no-caret exclusions, but
do not authorize content beyond the reference. Close has no new unsaved-change confirmation; New
still has the pre-existing dirty-session consent discrepancy in its replacement path; and neither OS
runtime behavior nor Windows acceptance is established by source implementation. No automated
desktop tests were added. User review and validation remain pending.

## Cross-cutting design

- **Configuration and secrets handling:** This local product architecture does not depend on service credentials or a secrets-management system; no additional configuration or deployment mechanism is specified here.
- **Identity and access:** An account or identity subsystem is not part of the described single-writer desktop product.
- **Errors, reliability, and observability:** Editing operations preserve transactional state. Save failures preserve the native project and leave the session's saved/dirty state accurate. Transactional save details project privacy-safe session-preparation, codec serialization/re-read, storage, or missing-baseline categories without screenplay text or unnecessary paths. The desktop status row keeps document-unchanged wording, horizontally and vertically centers its operation/error message in its explicit mirrored center region, and does not duplicate the session-owned dirty state with a Clean/Dirty indicator. Errors and diagnostics avoid unnecessary exposure of screenplay content. Core preservation COW counters report only wrapper-share and detachment counts, never document-derived values; these counters are not currently included in the desktop projection. The explicit `ENABLE_DIAGNOSTICS` source constant gates only status Diagnostics presentation and projection; its production-off value does not disable operational counters, native-input/performance instrumentation, scheduler state, repaint/layout scheduling, or error reporting.
- Desktop performance diagnostics are operationally retained and value-free: the desktop keeps cumulative saturating counters and nanosecond phase totals, while an explicit benchmark reset captures a baseline for deltas without changing the session or discarding prior totals. The status-specific detailed summary string, pending-derived-work presentation query, and `DiagnosticProjection` are constructed only when the manual Diagnostics gate is enabled. The single-line projection may include only fixed phase/counter names, numeric values, the debug/release build-profile label, and a closed set of operation labels. It must never include document text, FDX values or bytes, filesystem paths, IDs, or source data. Core layout wrapping reports build/reuse counts and elapsed actual-wrap time separately from the immutable layout value so instrumentation cannot affect layout equality or pagination. Whole-layout cache hits are not per-element wrap reuse. RS-WO-005 adds separate span-galley build/reuse counters; those counts never imply per-element wrap reuse or indexed interaction queries. Derived diagnostics distinguish due-tick attempts/completions/skips, irrelevant-revision skips, changed/unchanged outcomes, safe deferrals, and cumulative requested/attempted/deferred scene counts; live pending/complete/retryable and deferred-scene counts are numeric/value-free only.
- **Persistence and data evolution:** Native files are FDX Version 6. Explicit non-6 inputs receive a warning and a Version 6 importer attempt; previous versions use the lossy title/content-only path, while newer handling is best effort without a compatibility promise. Malformed input fails safely. Save validates a complete candidate before atomic replacement and publishes session corrections only after success. Legacy JSON migration is not a product commitment.
- **Verification approach:** Cargo workspace formatting, check, build, and Clippy checks cover build hygiene. Four consolidated headless core acceptance tests cover bounded editing/history, repeatable layout/export, native FDX preservation/reopen, and failed-Save-As safety scenarios. Desktop interaction and platform integration require applicable desktop/manual checks; automated core checks do not establish native OS behavior.

### Acceptance-suite consolidation — 2026-11-04

Under explicit user direction, existing tests were classified obsolete by default rather than treated as permanent work-order gates. The 37 historical integration files and 18 inline app/core unit-test modules were retired. The sole active test target is [`core_acceptance.rs`](../../crates/scriptcraft-core/tests/core_acceptance.rs:1), containing four contract-focused scenarios; no desktop automated tests remain. The final workspace formatting, check, build, Clippy, and test commands passed, with four tests passed and zero failed. Evidence and limitations are recorded in the [`build-hygiene report`](../../.plan/2026-11-04-build-cleanup/build-hygiene-report.md).

Later references in this document to deleted test files, desktop oracles, inline test modules, and authored-but-unrun historical coverage are historical design/implementation evidence only. They do not describe active test targets or current verification coverage, and the retired suite's assertion failures are not represented as repaired product behavior.

## Architectural constraints

- Maintain one semantic source of truth in the core; desktop presentation and exports derive from it.
- Keep semantic data bounded and edits transactional. Shared page geometry is repeatable for the same effective runtime layout inputs and independent of viewport scaling; this is not a promise of exact fidelity to every FDX layout. The core remains verifiable without a desktop runtime.
- Keep canonical native-project saving separate from noncanonical PDF and lossy text exports, and preserve failure safety at the native save boundary.
- On macOS, the header is owned by the native OS rather than an app-owned title strip; the application does not prescribe additional behavior for that OS-owned surface.

## Desktop performance diagnostics

The desktop owns the performance recorder, status-bar projection, benchmark-operation labels, and reset-baseline lifecycle. `status_bar.rs::presentation::ENABLE_DIAGNOSTICS` defaults to `false`; when disabled, status summary formatting, its pending-derived-work presentation query, projection construction, and the entire second-row disclosure are skipped. When explicitly enabled, the prior collapsed disclosure and summary remain available. Existing totals remain cumulative across document replacement in either mode; `reset_benchmark` records the current totals as the next benchmark baseline and `benchmark_delta` uses saturating subtraction. The reset affects diagnostics only and does not change editing revision, selection, dirty state, history, layout-cache identity, or rendered output. Layout build instrumentation lives beside core layout construction but returns `LayoutBuildDiagnostics` separately from `ScreenplayLayout`; wrap timings cover only `wrap_element` calls, not validation or pagination.

Core preservation diagnostics separately count shared vector/text/source-byte/image wrapper clones and detached vector/text/source-byte/image payloads. These process-wide, saturating counters are value-free structural counts, not timings, and are not projected into the desktop status-bar diagnostics.

The current renderer has whole-layout cache hit/build counts, page consideration/culling/painting, line and selection-span submissions, line/span galley build/reuse counts, per-element wrap build/reuse counts, caret/linear-query counts, refresh outcomes, and phase totals. Desktop page rows and complete page rectangles are allocated for the whole document to preserve scrolling, status tracking, pointer routing, and reveal; only pages intersecting the editor clip plus one fixed page of vertical overscan on either side are painted. Hidden pages issue no line, selection, text, underline, caret, or IME paint. Collapsed caret geometry is resolved at most once per frame and reused for reveal and owner-page paint. RS-WO-005 stores a bounded fixed-cell galley cache in egui context-local temporary data, keyed by span text, role-face family and size, fixed cell width, line height, text color, and pixels-per-point. Each galley is re-positioned to explicit cell coordinates; host-font advances do not participate in screenplay pagination or fixed-cell placement. Selection color boundaries form separate spans; underline and selected-background geometry remain independently painted, and the shared layout display/source map remains the authority for casing expansions and selection endpoints. RS-WO-006 stores a bounded immutable core wrap cache keyed by complete canonical content (verified after fingerprint lookup), stable element identity, role, purpose, title/body context, effective paragraph geometry and text style, and wrapping limits. It shares canonical/display line strings and source maps while rerunning page placement over the whole document. Project replacement changes an opaque clone-stable layout identity and clears both cached layout and wraps; runtime/source inputs and purpose participate in complete-layout and effective-wrap invalidation. The core cache excludes egui atlas data, presentation scale/palette, and the WO-005 context-local galley cache. Indexed interaction queries remain a later mechanism and their counters are independent. Timing values are observational only: automated regressions assert exact work counts and state isolation, never elapsed-time thresholds. User review and validation are required; no agent-run checks are permitted by repository policy.

The render-speed initiative maintains two deliberately distinct measurement surfaces. The core [`wo006_responsiveness.rs`](../../crates/scriptcraft-core/tests/wo006_responsiveness.rs) benchmark measures core layout build/cache lookup only and has no desktop frame, paint, or repaint counters. The desktop oracle is authored in the `main.rs` test module and drives the egui document surface against deterministic imported-FDX fixtures. Its current baseline distinguishes a whole-layout cache hit from all-page consideration/geometry allocation and viewport-bounded painting/line submissions; it asserts visible fixed-cell galley builds/reuses and one-wrap-build/unchanged-wrap-reuse work on ordinary edits. Indexed interaction query counts and index-entry probes are now included for pointer hit testing, caret-owner/geometry resolution, and vertical movement. Later orders must revise only the expectations for mechanisms they actually introduce. The oracle uses structural counts and layout equivalence, not timing thresholds; all review and validation remain user-owned.

RS-WO-007 stores derived page/line locations over complete immutable layout output. Its element-to-placed-line index retains page/line order, including title-page line locations needed to preserve the existing first-match caret geometry and owner-page behavior. A body-only document-order index records source ranges and fixed absolute vertical coordinates, while a separate sorted permutation supports nearest-y lookup without changing vertical-navigation order. Caret and owner-page queries inspect only the requested element's lines; nearest-line hit testing binary-searches the vertical permutation and considers only adjacent coordinate groups, retaining exact distance and page/line tie-breaking; vertical movement accesses indexed current and adjacent body lines without a flattened whole-document vector. Title pages are excluded from body hit testing/navigation. The core indexes reference shared immutable layout lines and carry no egui/presentation state; they are rebuilt with fresh complete layouts and reused with complete-layout cache hits. The desktop recorder reports indexed query calls and index-entry probes separately from legacy linear-query counts. These are diagnostics, not user measurements or proof of behavior; RS-WO-001–WO-007 coverage remains unvalidated by the user, and no timing results are recorded.

Desktop frame ordering for the render-speed initiative is explicit: eframe delegates to `ScriptCraftApp::update(&egui::Context)`, which captures native window state and configures notifications, dispatches accepted editor events using the preceding surface response's focus state, performs due scheduled cast-list refresh afterward, then builds/shows application panels for current-frame layout and paint. V2-WO-002.3 gates a due tick on the session-owned pending-derived summary before candidate cloning: the initial complete invalidation still receives one complete pass, relevant edits schedule their affected-scene scope, and irrelevant revisions skip without a candidate. The summary separately reports pending work, complete scope, retryable work, affected-scene count, and deferred-scene count without exposing identities. Safe-stop-deferred scenes remain pending but are not retryable until a later relevant edit invalidates their owning scope; unrelated edits and timer ticks do not attempt them. Successful Save/Save As still prepares and publishes complete derived data independently of the timer; the successful saved-session rebase records that prepared candidate as current so the timer does not duplicate it. Changed refreshes retain their repaint, revision, dirty, and input ordering behavior. A pending FDX migration modal excludes both scheduled refresh and editor dispatch and exclusively presents its own input surface. The panel pass continues to own pointer activation and refresh the focus lock for the next event-routing pass. Zoom normalization remains in the raw-input hook; timer/repaint setup and `RevealIntent` semantics remain unchanged. The pending-derived scheduler and desktop oracle are authored under V2-WO-002.3; they remain user-unvalidated until the user runs them.

V2-WO-002.3 adds public fields to `DerivedWorkSummary` for retryability and value-free deferred-scene count. This is source-compatible for callers using `Default` or reading existing fields, but external exhaustive struct literals must initialize the new fields; no compatibility guarantee is made for those literals. Successful saved-session rebase now records the completely prepared/published save candidate as current, so scheduled refresh is independent from Save/Save As without a revision watermark. Desktop diagnostics add numeric requested/attempted/deferred scene totals and live pending-scope counts, never scene IDs or document values. Authored desktop coverage covers irrelevant Action/Dialogue edits, owning-scene Character edits, deferred work waiting for a relevant edit, activation's complete pass, and Save timer independence; it has not been run. No test, build, lint, formatting, desktop automation, review, or independent validation was performed, and no latency claim or user acceptance is recorded.

## V2 source-identity and derived-heading indexes

Each [`PreservationState`](../../crates/scriptcraft-core/src/preservation.rs) owns two transient indexes built together by one ordered preservation-tree walk: the managed-paragraph location map and the source-identity-use summary. The first maps each managed semantic identity to every section/path occurrence, so duplicate owners remain explicit and indexed access rejects ambiguity. Heading element reads, depth/effective-`xml:space` context lookup, and detached mutable access resolve through those paths; context work is bounded by the target's ancestor depth rather than unrelated source branches. Mutable access traverses only the indexed COW path. It invalidates source-use data immediately, and any changed derived publication rebuilds both indexes together before candidate publication. Missing indexes are rebuilt at managed refresh boundaries; if an externally constructed unindexed state is queried through the preservation-owned accessor, the complete fallback is counted. Out-of-band mutations of public preservation collections must call [`rebuild_indexes()`](../../crates/scriptcraft-core/src/preservation.rs) before indexed queries. Indexes remain excluded from semantic equality and serialization.

The source-identity-use index separately summarizes Paragraph identity multiplicity, reference/anchor usage, external substring uses in attributes/text/CDATA/comments/PIs, and all plain source `id` values. It serves removal-safety and ID-reservation queries without repeating complete-tree scans per identity. Import/reconstruction, structural changes, managed source edits, undo/redo, replacement, derived publication, and saved-session rebase rebuild or invalidate indexes transactionally; direct external mutation uses the explicit rebuild API. Existing value-free diagnostics report index rebuilds, source nodes indexed, managed lookup/entry probes, fallback-tree nodes, and source-use query classes only. They expose no identity, source value, text, path, or timing claims. Structural counts are diagnostics, not proof of user-perceived speed.

V2-WO-003.2 authors coverage for nested heading depth and inherited XML-space, duplicate managed ownership, scoped-refresh lookup probes, zero indexed-query fallback visits, and one rebuild after changed derived publication. Existing source-index coverage authors mutation-invalidation and rebuild-boundary cases; existing scene metadata coverage includes unsupported-scene safe deferral. All coverage remains unrun under repository policy. The authorized V2-WO-002.4–V2-WO-003.2 batch is submitted for user testing except V2-WO-002.6, whose reported Scene Heading synchronization failure remains blocked and unestablished. No user validation, performance measurement, or acceptance is recorded; V2-WO-004 through V2-WO-006 remain deferred.

## Script View scrolling and viewport-centered zoom

Script View owns scroll-content page-edge spacing, page allocation, scroll extent,
and surface-local scroll presentation. Boundary spacing moves with the scrollable
content and contributes to its extent; it is not a viewport inset or a document
layout dimension. The Script Workspace frame has no fixed margin, and Script View
disables scroll-edge fades only for its own scroll area. Other scroll areas retain
their independent styling.

The surface publishes the actual scroll offset, clip, scale, and allocated page
rectangles. Zoom anchoring captures a bounded page-local document point and keeps
outside-page displacement as separate fixed logical presentation geometry. The
prepaint compensation derives its target from the newly scaled page origin and
page-local point, applies the pending offset before allocation and paint, and
clamps against the same content extent used by actual page allocation. Actual and
prepaint geometry account for page-edge spacing, row stride, and content extent
equivalently, with each contribution included once. Page-local coordinates scale;
page-edge spacing, inter-page separation, horizontal buffers, and outside-page
displacement do not. Published page rectangles, clip, scale, and committed offset
therefore describe the same frame. Manual scrolling cancels an unapplied request;
caret reveal defers while compensation is pending. Zoom remains presentation-only
and does not change canonical content, fixed layout, persistence, or exports.

V3-WO-001 adds numeric changed/unchanged scene totals to desktop performance diagnostics, projected from existing `CastListCorrection` outcomes alongside requested/attempted/deferred/failed counts. User evidence established the imported newly inserted Cast List repeated-refresh branch and WO-002's narrow production correction synchronizes semantic runs with retained source runs. The authorized focused regression command subsequently passed. Its later Save/reopen failure was test identity drift: the test had carried transient semantic `StableId` across rebase/reopen, and now resolves the list by durable foreign FDX paragraph identity. It continues to assert Cast List role, semantic/source runs, native save/reopen, Character values, and unrelated source preservation. Fatal refresh failure suspension and safe-stop deferral remain separate: failed pending work is nonretryable until relevant invalidation, while safe-deferred work is reconsidered after a relevant revision. Neither outcome weakens complete explicit Save/Save As preparation or fatal validation/storage gates. WO-001's broader diagnostic/scheduler cases remain unrun.

V3-WO-005's internal, deletion-only path permits cross-paragraph `delete()` to remove a Scene Heading with its recognized generated scene subtree only after all heading/nested Paragraph identities pass registration, source-definition, reference, anchor, and external-use checks. Unknown/ambiguous attachments and unsafe identities reject transactionally; backspace/join, split, and role transitions retain their safe-stops. Its focused deletion, identity/history, serialization/reopen, and unsafe-control coverage is authored but unrun. No public API changed. No V3 user evidence or acceptance is recorded by this closeout.

## RS-WO-011 managed editing validation

Each [`PreservationState`](../../crates/scriptcraft-core/src/preservation.rs) carries a transient
managed-paragraph index keyed by semantic [`StableId`](../../crates/scriptcraft-core/src/lib.rs).
The index maps an identity to every source-tree location, so duplicate ownership is represented as
ambiguous rather than silently selecting one. Index locations are rebuilt from the ordered
preservation tree on FDX import/session construction, structural reconciliation, derived-scene
publication, saved rebase/document replacement, and undo/redo restoration. Ordinary managed text
and role transforms preserve the indexed path; if a local topology/identity check does not match,
the detached candidate returns to complete validation and index reconstruction. The derived map
and source-text budget summary are excluded from preservation semantic equality. Shared preservation
wrappers remain the storage authority: cloning a candidate shares untouched bytes/subtrees, and
mutating a managed location detaches only its `Arc`-backed path.

The ordinary edit fast path is private to an already fully validated [`EditingSession`](../../crates/scriptcraft-core/src/editing.rs).
It is considered only for one selected body element by the ordinary text/delete/backspace/role
entry points, with a validated-baseline marker, same element count and identity, same-document
layout identity, exact equality of every non-target body element and the entire title page, unchanged
legacy metadata, a live managed index, and a same-element resulting selection. The candidate element
and selection are checked;
the indexed managed paragraph must be unique, remain at its baseline location, retain its identity,
anchor, all non-Type attributes, attachments and source topology, while all preserved tree nodes
outside it remain equal to the baseline; it must map its role Type exactly, project the candidate
text, and stay within the complete source-text budget. Source snapshot, layout
overrides, global anchors, identities, references, and board graph must still equal the valid
baseline. Thus a scoped edit cannot create semantic duplicates, alter a global reference graph, or
rewrite an opaque sibling. Any failed or uncertain precondition invokes complete project/source
validation on the detached candidate before publication; structural candidate reconciliation and
global save/import/replacement boundaries always retain complete validation. Rejection leaves the
live project, selection, history, dirty state, and revision unchanged.

Public [`Project::validate()`](../../crates/scriptcraft-core/src/lib.rs) and standalone layout builders
remain complete-validation boundaries. The desktop may use
[`LayoutCache::get_or_build_for_session_with_diagnostics()`](../../crates/scriptcraft-core/src/layout.rs)
because it accepts the live validated session rather than an arbitrary Project; ordinary render
cache misses therefore do not duplicate session validation. Save/Save As still perform complete
candidate validation, writer re-read, normalized source-tree/identity/reference checks, conflict
detection, and atomic publication.

Value-free structural counters are available from editing, project-validation, and preservation
index diagnostics: scoped attempts/accepts, edit-candidate full validations/fallbacks, total project
validation calls, index rebuilds/source nodes scanned/index lookups/index entries examined, and
fallback-tree nodes visited. They report work counts only, are not timings, are not a user speed
claim, and must be interpreted alongside their reset lifetime (process-wide core counters). No
agent-side tests, build, lint, formatting, implementation checks, or user verification were run for
RS-WO-011; the authored coverage remains unrun under repository policy.

## RS-WO-012 integration and closeout status

The integrated render-speed scenario and companion core regression are authored for user review, not executed evidence. [`WO-012`](../../.plan/2026-11-03-ui-quality-of-life/work-orders-v1/WO-012-integration-and-closeout.md) records the user-run 120-page desktop procedure, the existing desktop/core coverage map, and the measurement template. The new [`wo012_integrated_render_preservation.rs`](../../crates/scriptcraft-core/tests/wo012_integrated_render_preservation.rs) checks long-document cached/fresh layout equivalence across edit/undo/redo, codec-level FDX opaque-source round-trip, and export isolation when the user runs it; it does not replace filesystem Save/Save As or desktop FDX checks.

RS-WO-001 through RS-WO-011 are implemented/submitted, but all user review and validation remain pending. WO-009's no-source-byte/no-unrelated-subtree-copy criterion is addressed by WO-010's immutable sharing design; this is not execution-confirmed. WO-012 authored no timing values, test outcomes, acceptance, or measurement record. The original `master` base is the comparison reference, but no baseline was captured before implementation; a separate user-managed checkout/worktree can protect the current uncommitted initiative checkout.

User review must include the compatibility boundaries: WO-010 changed public preservation field types to `SharedVec`, `SharedText`, and `SharedBytes`, without guaranteeing source compatibility for every external caller; WO-011 removes public `PreservationState` struct-literal construction and adds public session-bound layout/diagnostic APIs; and scoped validation only applies to a validated baseline with strict unchanged-topology preconditions, falling back to complete validation otherwise. WO-006 still performs whole-document page placement. WO-005's pinned-egui representation fallback uses scalar painting for unsupported spans, preserving the documented correctness fallback while reducing batching. These are caveats for review, not claims of a known failure.

Final user-owned gates remain compile/build and workspace checks, test execution/review, manual desktop and FDX/source-preservation checks, Save/Save As/reopen and export review, before/after observations and counters in selected build profiles, and recorded final acceptance/remediation. No WO-012 scenario or validation was run.

## Save-path UnsupportedConstruct diagnostics

Save and Save As retain the same strict writer, semantic re-read, identity, normalized source-tree, conflict, and transaction gates. The crate-private `WriterStage` remains private. Most save-path `UnsupportedConstruct` failures use `FdxError::SaveDiagnosticUnsupportedConstruct(u16)`; the normalized source-tree gate uses `FdxError::SaveDiagnosticSourceTreeMismatch(String)`, whose underlying diagnostic starts at character zero with `30: FDX contains a construct that cannot be projected safely;`. It appends one bounded, value-free structural detail: `path=section[N]/node[N]...; field=CATEGORY; expected=SUMMARY; actual=SUMMARY`. The structural detail is capped at 240 bytes and 12 child-index path components; path indexes are ordered source-section and child indexes. Transactional and desktop error details preserve the complete code and structural suffix. Reader/import errors outside the Save/Save As output re-read propagation sites remain unnumbered.

FDX declaration separation is provenance-specific. Preservation-free generation retains the canonical UTF-8 declaration followed by one LF. For source-backed serialization, the declaration has no writer-added trailing LF; retained top-level nodes are then emitted unchanged and own all declaration-to-root whitespace, comments, processing instructions, and root position. Do not trim, normalize, or synthesize a separator around those nodes: their exact serialized structure must remain fixed-point stable across repeated saves, while code 30 continues to compare the strict normalized source trees.

Codes are stable identifiers, not source-line numbers. This table is the centralized durable mapping; append unused integers and never reuse or renumber an assigned value.

| Integer | Exact function and return/propagation call site | Condition |
|---:|---|---|
| 1 | [`write_v6()`](../../crates/scriptcraft-core/src/fdx.rs:484) | Preserved source format is not FDX or encoding is not UTF-8. |
| 2 | [`write_v6()`](../../crates/scriptcraft-core/src/fdx.rs:484) | Propagation of `UnsupportedConstruct` while re-reading the original source snapshot. |
| 3 | [`write_v6()`](../../crates/scriptcraft-core/src/fdx.rs:484) | Re-read original project has no preservation state. |
| 4 | [`write_v6()`](../../crates/scriptcraft-core/src/fdx.rs:484) | Candidate non-Paragraph identities, references, or board graph differ from original source graph. |
| 5 | [`reconcile_project_elements()`](../../crates/scriptcraft-core/src/fdx.rs:1145) | Final managed root count or ordered body/title identity projection does not match the project. |
| 6 | [`reconcile_node()`](../../crates/scriptcraft-core/src/fdx.rs:1208) | A managed source identity is encountered more than once. |
| 7 | [`reconcile_node()`](../../crates/scriptcraft-core/src/fdx.rs:1208) | A managed source element is outside Content or TitlePage. |
| 8 | [`reconcile_node()`](../../crates/scriptcraft-core/src/fdx.rs:1208) | Managed source identity has no corresponding semantic element. |
| 9 | [`reconcile_project_elements()`](../../crates/scriptcraft-core/src/fdx.rs:1145) | Top-level propagation of an unnumbered `UnsupportedConstruct` from `reconcile_node()`. |
| 10 | [`reconcile_managed_paragraph()`](../../crates/scriptcraft-core/src/fdx.rs:1280) | Preserved paragraph source role differs from current semantic role. |
| 11 | [`reconcile_managed_paragraph()`](../../crates/scriptcraft-core/src/fdx.rs:1280) | Direct Type attribute is missing, duplicated, or differs from current role. |
| 12 | [`reconcile_managed_paragraph()`](../../crates/scriptcraft-core/src/fdx.rs:1280) | Empty semantic text has semantic runs. |
| 13 | [`reconcile_managed_paragraph()`](../../crates/scriptcraft-core/src/fdx.rs:1280) | Empty semantic Text element contains CDATA. |
| 14 | [`reconcile_managed_paragraph()`](../../crates/scriptcraft-core/src/fdx.rs:1280) | Empty semantic Text element contains a child other than text or CDATA. |
| 15 | [`reconcile_managed_paragraph()`](../../crates/scriptcraft-core/src/fdx.rs:1280) | Empty semantic paragraph has nonempty source Text content. |
| 16 | [`reconcile_managed_paragraph()`](../../crates/scriptcraft-core/src/fdx.rs:1280) | Preserved Text element count differs from semantic run count. |
| 17 | [`reconcile_managed_paragraph()`](../../crates/scriptcraft-core/src/fdx.rs:1280) | Preserved run attributes or child-node kinds cannot project to the semantic run. |
| 18 | [`reconcile_managed_paragraph()`](../../crates/scriptcraft-core/src/fdx.rs:1280) | First preserved run child is not text or CDATA. |
| 19 | [`reconcile_managed_paragraph()`](../../crates/scriptcraft-core/src/fdx.rs:1280) | Later preserved run child is not text or CDATA. |
| 20 | [`make_new_document_nodes()`](../../crates/scriptcraft-core/src/fdx.rs:1407) | Generated Scene Heading paragraph is not represented by an element node. |
| 21 | [`make_new_document_nodes()`](../../crates/scriptcraft-core/src/fdx.rs:1407) | SceneProperties generation returns an unsupported metadata error. |
| 22 | [`new_paragraph()`](../../crates/scriptcraft-core/src/fdx.rs:1542) | Empty semantic text has semantic runs during new-document generation. |
| 23 | [`new_paragraph()`](../../crates/scriptcraft-core/src/fdx.rs:1542) | New-document Text run attribute has a qualified namespace prefix unsupported by this generator. |
| 24 | [`write_element()`](../../crates/scriptcraft-core/src/fdx.rs:1630) | Element has a namespace URI but no qualified name for XML output. |
| 25 | [`write_v6()`](../../crates/scriptcraft-core/src/fdx.rs:484) | Propagation of `UnsupportedConstruct` when re-reading serialized output. |
| 26 | [`write_v6()`](../../crates/scriptcraft-core/src/fdx.rs:484) | Re-read output semantic title/body projection differs from input project. |
| 27 | [`write_v6()`](../../crates/scriptcraft-core/src/fdx.rs:484) | Re-read output has no preservation state for an imported candidate. |
| 28 | [`write_v6()`](../../crates/scriptcraft-core/src/fdx.rs:484) | Imported candidate unexpectedly has no preservation state for identity comparison. |
| 29 | [`write_v6()`](../../crates/scriptcraft-core/src/fdx.rs:484) | Registered identity sets differ between candidate and re-read output. |
| 30 | [`write_v6()`](../../crates/scriptcraft-core/src/fdx.rs:490) | Strict normalized expected source tree differs from re-read output; carries the first deterministic bounded path/category detail. |
| 31 | [`NativeSaveCandidateCodec::reread()`](../../crates/scriptcraft-core/src/fdx_storage.rs:370) | Propagation of reader `UnsupportedConstruct` while storage re-reads prepared output. |
| 32 | [`save_session_with_codec()`](../../crates/scriptcraft-core/src/fdx_storage.rs:375) | Strict storage semantic save projection differs after prepared-output re-read. |
| 33 | [`reconcile_node()`](../../crates/scriptcraft-core/src/fdx.rs:1208) | Recursive child call propagates an unnumbered `UnsupportedConstruct`. |
| 34 | [`reconcile_node()`](../../crates/scriptcraft-core/src/fdx.rs:1208) | Managed paragraph call propagates an unnumbered `UnsupportedConstruct`. |
| 35 | [`reconcile_document_lineage()`](../../crates/scriptcraft-core/src/fdx.rs:916) | Direct root DocumentRef occurrences are ambiguous, so lineage reconciliation refuses to select one. |
| 36 | [`reconcile_document_lineage()`](../../crates/scriptcraft-core/src/fdx.rs:916) | DocumentRef has multiple recognized DateTime attributes, so no durable timestamp is selected. |
| 37 | [`SaveContext::allocate_lineage_id()`](../../crates/scriptcraft-core/src/fdx.rs:306) | Secure randomness for a new DocumentRef/XRef identity is unavailable. |
| 38 | [`SaveContext::allocate_lineage_id()`](../../crates/scriptcraft-core/src/fdx.rs:306) | A collision-free DocumentRef/XRef UUID v4 could not be allocated within the retry limit. |

The code 30 first-difference walk mirrors every field used by normalized `PreservedSection`/`PreservedNode` equality. Its deterministic comparison order is section presence/name/kind/ordinal, then document-order nodes; within an element it checks expanded/local and qualified names, ordered attribute names/order/count/value, namespace binding prefix/order/count/URI, source Type metadata, ownership category, dependency-confidence category, anchor presence, and then children recursively. Node comparisons classify child presence/order and Element/Text/CDATA/Comment/processing-instruction kind/content. Ordinary text and other character data are described only by character length and empty/whitespace-only/non-whitespace classification. The comparison uses the already normalized trees: empty ordinary Text nodes are absent, adjacent ordinary Text nodes are merged, and Managed ownership/anchors are cleared exactly as before.

Public categories include section metadata; element/section local, namespace-presence, and qualified-name differences; attribute name/order/count/presence/value (with an attribute local name where available); namespace-binding prefix/order/count/presence/namespace; source-type; ownership; dependency-confidence; anchor; node kind/order/presence; text/CDATA/comment/PI content; and child presence. Public summaries may show safe local element/attribute names, enum categories, counts, and value-free text classifications. They never show screenplay/arc text, raw ordinary or XML node content, attribute or namespace URI values, Type values, IDs/UUIDs/StableIds/anchor IDs, serialized XML, image data, or filesystem paths. Detailed metadata is computed only after the unchanged strict equality gate has failed; equality still refuses the save. Reference-backed leading categories are attributes/ID placement, element names, child order, and ordinary text/whitespace. Namespace declarations and CDATA/comments/non-declaration PIs remain synthetic generic-node coverage, not claims about the checked-in Final Draft examples. Private identity mismatch metadata reports only identity domain and missing/unexpected direction.

## FDX model and ownership contracts

For supported managed Scene Headings, SceneProperties and SceneArcBeats are generated containers:
Length is `1/8`, Page is `1`, Title is empty, and the container namespace follows the heading.
Supported-scene non-arc fields are regenerated on derived refresh; Page/Length are not recalculated.
This supersedes earlier v3 wording that treated those supported-scene fields as source-owned.
Supported CharacterArcBeat Paragraph subtrees remain preserved by normalized Character name in
the existing Project preservation state and editing-history snapshots, without a second arc model.
Ambiguous or unsupported arc payloads still defer the complete scene candidate and retain the
original SceneProperties/SceneArcBeats subtree; this generated-container rule does not authorize
discarding unsupported authored arc content.
- Managed Scene Heading deletion is a separate, deletion-only exception: an explicit valid
  cross-paragraph selection may remove the heading together with this exact recognized generated
  container and its supported nested arc Paragraph subtrees. Before any source mutation, the
  heading and every nested Paragraph must have one current registered identity, one source
  Paragraph definition, and no known references, foreign anchors, or external source-tree uses.
  Unknown/duplicate/unsupported children and unsafe or missing identities reject transactionally.
  Successful removal cleans those exact Paragraph registrations, the removed heading's managed
  anchors/direct overrides, and the transient indexes. It does not remove Cast Lists or other source
  nodes, and it does not authorize attachment removal through backspace/join, split, or role changes.

- One bounded shared FDX paragraph-ID allocator generates lowercase hyphenated UUID v4 values for new body, title-page, source-backed inserted/split, and nested scene-arc Paragraphs. It checks current preservation identities and source `id` attributes, stops after a fixed retry bound, and reports entropy failure rather than publishing partial candidate state. Existing source IDs—including non-UUID IDs—are never migrated, and deleted IDs are not retained in a tombstone registry. Ordinary edits, role changes, scene-cast refresh, Save, Save As, and reopen retain established IDs; split retains the original/left ID and allocates one right ID; join/delete remove the removed paragraph's ID; and session undo/redo restores exact snapshot identities. Recreated content is newly allocated, but the absence of historical tombstones means absolute non-reuse against every deleted historical ID is not guaranteed. Matching scene-arc names retain nested IDs and newly introduced or renamed names receive new allocations. First-time IDs for a never-saved project can differ between independent serializations; after successful save/re-read establishes source IDs, subsequent serialization preserves those IDs. This is identity stability, not a general byte-determinism or XML-formatting promise.
- Plain-text export consumes each print-layout `LayoutLine.display_text`, so it shares the effective Unicode AllCaps presentation and wrapping already used by layout/PDF rather than applying a second casing transform. Export remains deliberately lossy and noncanonical: HiddenText is omitted, role/identity/page-layout loss is reported, and neither semantic text nor native FDX is changed.
- The semantic document retains all 17 directed content types and ordered text/run data. Internal identities and edit anchors are transient implementation state and may be regenerated on reopen; they are not durable ScriptCraft document IDs. The preservation layer owns source order, namespaces, unknown nodes and attributes, foreign/source identities and anchors, FDX-owned metadata, title-page structure, and source document settings, including nested scene data and board/item/image relationships. The codec and editor share a transactionally consistent ownership map; the editor never silently converts Version 6 unknown types to Action. Do not add a private FDX extension or sidecar for ScriptCraft project identity/schema, metadata, layout configuration, or editing policy.
- Identity collection distinguishes managed or globally referenced Paragraph definitions from opaque TitlePage occurrences. A unique opaque TitlePage Paragraph retains its existing global identity registration. When multiple opaque TitlePage Paragraph occurrences share a raw ID and no global consumer needs that identity, the ordered preservation tree remains authoritative and each raw ID/element occurrence is retained verbatim without registering the repetition as a unique definition. Such occurrences cannot shadow a globally registered Paragraph identity; duplicate checks remain strict for managed body/title roles and all other global identity/reference consumers. This boundary does not rewrite source IDs or weaken board/image reference checks.
- The core formatting module owns the ordered typed defaults for all 17 roles. New native documents serialize one ElementSettings block per role from that table; imported settings remain authoritative in the preservation tree and are not regenerated. Runtime layout resolves bounded supported role `ParagraphSpec` geometry, alignment, first-line indent, spacing, leading/spacing tokens, and page starts independently over those defaults using signed 1/100-point geometry. Role typography independently resolves family, integer size from 1–72 points, supported style tokens, and visibility; unknown values remain source-owned and use deterministic defaults. Desktop maps Courier Final Draft to the bundled Courier Prime regular/bold/italic/bold-italic faces; Arial and unknown families use the same fixed-cell fallback, whose 7-unit character width and 12-unit line advance at 12 points scale by role size. Font advances from the host do not drive pagination. AllCaps is a display-only Unicode transform with explicit display-to-source spans; canonical text and source offsets remain authoritative. Layout purpose distinguishes editor from print: HiddenText roles remain editable and subdued in the editor with contrast-safe theme colors, and print/PDF and plain-text export omit them without changing source indexes. A source-visible role receives ordinary styling. The same fixed geometry feeds editor interaction/painting, PDF coordinates, pagination, and cache identity, which includes effective source font settings and layout purpose. PDF applies a bounded size/weight/slant subset with non-embedded standard Courier resources and separate underline geometry; it does not claim bundled-font embedding, exact visual parity, Final Draft interoperability, or remediation of the existing non-ASCII substitution. Source colors are preserved but not interpreted as authoritative display/PDF colors. This bounded interpretation is not exact source-layout fidelity. Legacy integer-point role boxes convert at the transient JSON boundary and no private fixed-point fields are persisted. Nondefault legacy per-project layout configuration is not durable and may reset after reopen. Editing policies such as cue casing, automatic parentheticals, and strict transition boundaries are runtime policy with explicit defaults, not document or FDX fields.
- Every serialized FDX output starts with exactly one reference-compatible UTF-8 standalone XML declaration. Only newly generated documents receive canonical top-level order: DocumentRef, Content, WindowState, TextState, all 17 ElementSettings in native role order, then TitlePage when present. Generated element-only structure uses LF, two-space indentation, blank lines between FinalDraft children and ElementSettings, adjacent Content paragraphs without blank lines, and one final LF. Canonical Text (including whitespace-only values, tabs, LF and CR), CDATA, comments, PIs, and mixed content are not formatted. Imported documents retain top-level and opaque source order; whitespace sibling gaps adjacent to newly inserted/split managed Content Paragraphs receive best-effort LF/two-space normalization only for unambiguous Paragraph/structural-whitespace-only Content without effective `xml:space="preserve"`. Expected source-tree comparison applies that same explicit normalization and otherwise remains strict. When the Content shape is mixed/opaque or effective `xml:space` preservation applies, insertion formatting is skipped and the reconciled managed paragraph is serialized at its exact source-tree position; cosmetic formatting coverage is not a save precondition. No imported opaque structure is globally pretty-printed. The declaration is emitted outside the preserved tree and does not authorize lexical normalization or blanket Final Draft compatibility.
- Supported managed Scene Heading reconciliation formats only its successfully validated, generated `SceneProperties`/`SceneArcBeats` container chain. LF and two ASCII spaces are calculated from the managed heading's actual depth in the preserved document tree; supported container whitespace is included in the detached expected tree before strict code 30 comparison. Deferred/ambiguous scene subtrees remain untouched. Effective `xml:space` preservation, mixed/opaque children, CDATA, comments, PIs, and arc Paragraph content/runs prevent cosmetic formatting in the affected context; empty `SceneArcBeats` remains childless and self-closing. The source-backed formatting boundary is not a global pretty-printer.
- Managed screenplay body and direct managed title-role Paragraphs use one empty-content projection: empty canonical text has no semantic runs and no direct managed `Text` children, while the Paragraph identity/role/Type/direct attributes, attachments, and non-Text source children remain source-owned. Imported empty or formatted-empty `Text` runs project to no semantic runs and are removed only during managed writer reconciliation; whitespace-only canonical content remains a Text run. Edit and derived plain Cast List paths synchronize semantic text/runs with source runs in a transactional candidate, and typing into a childless managed Paragraph creates a namespaced source `Text` element. Strict expected/actual tree comparison drops only serializer-invisible empty XML text nodes before joining adjacent nonempty text nodes; it retains empty CDATA, whitespace-only nodes, comments, PIs, empty elements, and opaque empty `Text` structures. Identity, managed mapping, mixed-content, and source-tree checks otherwise remain strict.
- Managed body Paragraphs may also project the eight supported direct geometry/spacing/page-start fields into an ephemeral map keyed by current semantic element identity. Resolution is per field in direct > role > application-default order, and the map participates in layout cache equality. Typography/visibility remain role-level; title-page and run-level settings remain preservation-only. The ordered source tree remains authoritative, so invalid and unknown values are not normalized. Splitting retains attributes only on the surviving original identity and does not copy them; joining refuses transactionally when the removed paragraph carries any supported direct field. Role changes preserve these attributes under the existing managed-attribute reconciliation.
- Runtime keyboard policy reads each role's application-default ReturnKey and TabKey from the shared formatting table and resolves target labels against that table; imported per-document Behavior is retained by source preservation but does not configure editing. Enter follows General → General, Scene Heading → Action, Action → Action, Character → Dialogue, Parenthetical → Dialogue, Dialogue → Action at collapsed end / Dialogue split at collapsed interior, Transition → Scene Heading, Shot/Cast List → Action, New Act/Sequence → Scene Heading, End of Act → New Act, and Summary/Outline 1/2/3/Note → Summary. Same-role Enter still performs a normal split. Tab follows General → General, Scene Heading → Action, Action → Character, blank Character → Transition, Parenthetical → Dialogue, Dialogue → Parenthetical, Transition → Scene Heading, Shot/Cast List → Action, New Act/Sequence → Scene Heading, End of Act → New Act, Summary → Outline 1, Outline 1 → Outline 2, Outline 2 → Outline 3, Outline 3 → Note, and Note → Summary. Same-role Tab is a consumed no-op. Existing UTF-8, transactional history, automatic-parenthetical, strict-boundary and source-preservation rules remain the editing boundary; this does not persist an application editing-policy field or execute imported Behavior.
- Every actual managed role transition uses one editing candidate to keep the semantic role, preserved FDX paragraph `Type` and `source_type`, and semantic foreign-role provenance synchronized. The preservation layer changes only the recognized Type mapping; unknown attributes and nested attachments remain source-owned. Mapping conflicts and Scene Heading attachment conflicts reject the candidate without publishing changes to project, selection, history, or dirty state. Auto-generated/removed Parenthetical text is reconciled through the managed text mapping in that same candidate. Typed, pasted, split, joined, role-assigned, undone, and redone text retains exact canonical casing; there is no active edit-time cue-uppercasing policy. AllCaps remains a display-only Unicode transform with display-to-source spans, while native FDX retains canonical case. This contract does not authorize structural edits to attached scene data.
- Import records cast-list text unchanged. The ordered refresh/save-candidate path reconciles supported `SceneProperties`/`SceneArcBeats` independently per managed heading using the shared normalized Character projection: trim outer Unicode whitespace, remove at most one recognized trailing delivery extension, deduplicate by Unicode lowercase association key in first-occurrence order, and store derived names in Unicode uppercase. A supported arc beat has one nested same-namespace Paragraph whose ordered children are same-namespace `Text` runs containing only text/CDATA, optionally separated by whitespace-only formatting nodes. The nested Paragraph stays opaque to screenplay body content; its source attributes, ID, text, run order, and run formatting are retained unchanged for matched normalized Character names while only derived beat Name/order is updated. Adds allocate fresh UUID v4 IDs; removals do not require runtime creation proof and remove the entire derived beat, Paragraph, and arc text. New nested paragraphs are opaque source nodes whose identities are registered in the preservation graph, not semantic elements or anchors. Removal still requires a unique registered Paragraph ID with no known references or external source-tree uses. Duplicate beats, multiple nested Paragraphs, unknown non-text child structures, and unsafe references defer only the affected scene: its complete SceneProperties/SceneArcBeats subtree and Cast Lists remain unchanged while independent supported scenes may reconcile. This safe-stop scopes the derived update, not an otherwise valid canonical native Save or Save As. New-document FDX generation uses the same derived projection. Cast List presentation consumes uppercase names from reconciled scene arcs, sorted case-insensitively using the existing list behavior; preservation-free projects use the same uppercase derived projection from canonical Character cues. A formatted list that cannot be corrected without unsupported run rewriting keeps its text and attributes and does not block other derived updates or native saves. Scheduled refresh, detached Save/Save As preparation, and undo/redo correction apply the same per-scene deferral and Cast List policy in one detached candidate. Model and preservation validation, managed-source synchronization, required identity allocation, serialization/re-read comparison, conflict detection, and atomic publication remain strict; only the derived update is deferred.
- Supported scene-arc membership and its supported nested arc text are app-owned after deserialization. Normalized Character names—not nested UUIDs or runtime creation history—associate entries across reopen and roster edits. A retained name keeps its entire nested Paragraph subtree; a rename to a different normalized key is removal plus a new blank arc, and removal discards that derived arc text. Canonical Character text is never rewritten by this projection. The separate-reopen stale-arc save block is removed for the recognized supported shape; unsupported structures continue to safe-stop under this order's bounded behavior.
- **Future compatibility TODO — nested scene-arc UUID regeneration:** Final Draft has been observed to regenerate nested scene-arc Paragraph UUIDs on every save, even without edits. ScriptCraft intentionally retains matched nested IDs and allocates IDs only for newly derived entries; the observed difference is not currently evidence of a compatibility defect. Revisit only if a concrete Final Draft interoperability scenario demonstrates that regeneration is required for semantics, references, or reopen behavior.
- Derived-name projection: new and supported imported SceneArcBeats use Character elements only, trim outer Unicode whitespace, remove at most one recognized trailing delivery extension ((V.O.), (O.S.), (O.C.), (CONT'D), (SUBTITLE), (TEXT), or (V.O)), ignore empty results, deduplicate by Unicode lowercase association key in first-occurrence order, and store names in Unicode uppercase. Canonical cue text is unchanged. Stored Cast List text uses those same uppercase derived names, including preservation-free projects; unsupported scenes and uncorrectable formatted lists may defer only their derived text updates, not canonical native saving.
- Derived refresh candidate ownership (RS-WO-009 + RS-WO-010): an actual refresh or explicit Save/Save As preparation creates one detached complete Project candidate for atomic publication/preparation. Preservation sections/child vectors and text/source-byte/image payloads use immutable `Arc`-backed wrappers; candidate/history clones share untouched storage, and mutable access detaches the accessed vector/payload. Managed paragraph mutation collects an immutable section-to-element index path, then traverses mutably only through that path. Scene reconciliation still attempts only the active managed heading subtree; unsupported headings are discarded intact while independent headings may reconcile, and fatal errors discard the detached top-level candidate. Changed derived candidates retain full project validation; unchanged outcomes use explicit changed/deferred tracking and avoid whole-project equality/full validation. This structural sharing addresses WO-009's no-original-byte/no-unrelated-subtree-copy criterion by implementation design, but remains user review/validation pending. FDX source ownership, native serialization, save gates, scene semantics, detached rollback, and complete explicit save preparation are unchanged. Core COW counters are value-free and not desktop-projected. Public preservation field types changed from `Vec`/`String`/`Vec<u8>` to wrapper types; source compatibility for all external callers is not promised and requires review.
- Script and board objects are independent; their cross-domain links may be silently orphaned and do not block script edits. Preserve board objects and their internal relationships and unrelated authored content where possible. A concrete unsolvable conflict with authored-data preservation is escalated rather than generalized into a ban on script edits.
- Preserve imported title-page structure without position-based metadata guesses; title-page editing is deferred. XML handling is bounded, namespace-aware, denies external entities and external resource resolution, and never dereferences recorded image paths. Unknown mappings and unrepresentable edits stop safely or escalate; preservation is not a claim of lexical identity or interoperability.
- Native Save and Save As serialize fully before replacement, detect conflicts, retain destination and live session on failure, and use platform-appropriate same-directory atomic replacement. Recovery, if retained, carries the preservation state; old JSON recovery is not migration support. Fountain's fixed conversion protocol and implementation remain separate from the native FDX boundary.
- DocumentRef DateTime is the current-saving-time value and its existing ID remains stable. `EditingSession` captures a separate session-open wall-clock timestamp for imported and new documents; it is ephemeral, independent of DocumentRef, outside snapshots, and unchanged through successful save rebases. Save appends one XRef using session-open time only when there are no direct-child XRefs; otherwise it preserves history without an addition. Save As always appends exactly one session-open XRef. Each successful Save/Save As independently writes its captured current saving time to DocumentRef DateTime. One-second FDX resolution permits these timestamps to coincide; do not fabricate monotonic values. `SaveContext` carries operation kind, saving time, and session-open time into the detached writer; local wall-clock time is formatted without a zone suffix through the small `time` dependency, with UTC fallback if local offset discovery fails. Fresh lineage UUID v4 allocation checks every registered identity and source `id` attribute. Original code 4 comparison runs before authorized lineage edits; expected identities/tree include those edits before unchanged code 29/code 30 checks. Direct source-backed `write_v6()` remains lineage-neutral, and storage publishes/rebases only after conflict checks and atomic publication.
- Root WindowState reads only a direct child matching the FinalDraft root's expanded namespace; duplicate or unusable geometry gives no pending restore but does not reject Open. Left/Top are bounded signed 32-bit logical coordinates; Width/Height are bounded positive 32-bit logical dimensions; unknown Mode is treated as Normal. The desktop captures the latest reported outer position and inner dimensions in logical coordinates and dispatches imported position/size once after activation, without retrying or claiming native acceptance. Save uses the latest complete snapshot or the 870 × 854 at (0,95) agent-default. Actual transactional Save/Save As replace duplicate root blocks with one canonical Height/Left/Mode/Top/Width element or insert after PageLayout/before ElementSettings; preservation-free output emits the block after Content. Existing direct source-backed `write_v6()` remains WindowState-neutral. Nested WindowState-like content, TitlePage, and unrelated source remain untouched; no window value enters Project or session revision/dirty/history/layout/PDF state. Detached candidate tree comparison and atomic publication safeguards remain unchanged.
- Root TextState is disposable native body-editor view state, distinct from semantic Project content. The reader projects only one direct FinalDraft child in the root expanded namespace; missing/duplicate blocks use safe defaults, Scaling defaults to 100 when missing or invalid and clamps valid positive numeric input to 75–300, and Selection falls back atomically to caret 0,0 when malformed, reversed, out of range, or unmappable. Core [`text_state.rs`](../../crates/scriptcraft-core/src/text_state.rs) owns the reversible ScriptCraft offset conversion: canonical body paragraph text in semantic order, UTF-16 code units, one separator between paragraphs, and UTF-8 session byte-boundaries; title-page, SceneProperties, arc paragraphs and opaque content are excluded. Desktop activation applies restored Selection once through the session API and restores whole-percent zoom while syncing the pinch baseline; neither enters Project, dirty/revision/history, fixed layout, or exports. The save-scoped candidate receives current Selection and zoom; actual Save/Save As replace duplicate direct-root TextState with canonical Scaling/Selection/ShowInvisibles=No attributes, after WindowState when inserting a missing block. Direct source-backed `write_v6()` retains imported root state. Nested TitlePage TextState and all unrelated source data remain unchanged; transaction failure publishes none of the staged view state.

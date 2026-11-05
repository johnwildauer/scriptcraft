# UI Icons v1 — Material icon foundation and status bar

- **Status:** WO-001 through WO-008 implementation handoffs recorded; all user review/validation remains pending; initiative acceptance not recorded
- **Created:** 2026-11-04
- **Validation owner:** User; no agent-run builds, tests, lint, formatting checks, desktop automation, reviews, or validators are authorized by this plan
- **Execution:** Sequential work orders on shared branch `dev/2026-11-04-ui-icons`, with a fresh coding agent for each order; WO-001 through WO-008 have implementation handoffs, with all user review/validation pending

## Outcome

Adopt egui_material_icons as the desktop icon foundation, migrate the desktop to the compatible eframe/egui 0.34 line, and make the status-bar controls compact, icon-led, permanently inline at supported widths, and subordinate the center status text to the right-hand controls.

The final right-hand order remains Sprint Timer, Focus Mode, Appearance, zoom slider, and zoom percentage. Timer and Focus Mode use outlined Material Symbols; Appearance uses filled Mode Night. The three action buttons show no visible text. The slider shows no visible caption. The far-right percentage shows only its integer and percent sign, with no arrow, while retaining the existing preset menu and shared zoom state.

## User-confirmed dependency decision

The user explicitly authorized the eframe/egui 0.34 upgrade plus egui_material_icons 0.6.0 with filled and outline features. The user also accepted the crate's embedded Regular weight and the additional framework-migration work.

This supersedes the weight-200 preference for this implementation only. The assignment reference should retain that original design preference and record the Regular-weight implementation exception rather than claiming weight 200 is active.

Published-package research supporting the decision:

- [egui_material_icons 0.1.0 dependency metadata](https://crates.io/crates/egui_material_icons/0.1.0/dependencies) matches egui 0.29 but provides only one rounded font.
- [egui_material_icons 0.6.0 documentation](https://docs.rs/egui_material_icons/0.6.0/egui_material_icons/) uses egui 0.34 and introduces filled/outline features and per-icon style selection.
- The 0.6.0 icon catalog contains Timer, Location Searching, and Mode Night. Its source embeds Regular filled and outlined font files; it has no weight-selection API.

## Current implementation findings

1. [`apps/scriptcraft/Cargo.toml`](../../apps/scriptcraft/Cargo.toml:8) currently declares eframe 0.29 and has no Material icon dependency. [`document_font::install()`](../../apps/scriptcraft/src/document_font.rs:53) installs Courier Prime at startup; Material initialization belongs immediately afterward in [`main()`](../../apps/scriptcraft/src/main.rs:2792), so the existing document-font installation is not accidentally replaced.
2. [`show_impl_with_appearance()`](../../apps/scriptcraft/src/status_bar.rs:1197) divides the entire status row into three equal columns. The right controls therefore receive only one third of the row regardless of their measured needs.
3. [`plan_right_status_items()`](../../apps/scriptcraft/src/status_bar.rs:157) intentionally moves non-fitting right controls into an overflow list, and [`overflow_menu()`](../../apps/scriptcraft/src/status_bar.rs:1082) renders the More trigger. Narrower labels alone cannot satisfy the requested behavior; the allocation and overflow contracts must change together.
4. The current visible labels and measured widths are hard-coded in [`appearance_menu()`](../../apps/scriptcraft/src/status_bar.rs:933), [`FocusModeView::button_text()`](../../apps/scriptcraft/src/focus_mode.rs:39), and the right-item measurement block in [`show_impl_with_appearance()`](../../apps/scriptcraft/src/status_bar.rs:1219). Focus Mode's enabled state and detailed accessible state must remain independent of its visible glyph.
5. [`zoom_controls()`](../../apps/scriptcraft/src/status_bar.rs:244) combines a slider carrying the visible Page zoom caption with a stock combo box that paints an arrow. The replacement must preserve the nine presets, intermediate integer display, keyboard behavior, and the single zoom value established by [`AC-WRITING-006.7`](../../.forge/requirements/features/screenplay-writing-loop.md:619).
6. Toolbar hover is explicitly painted by [`toolbar_action_button()`](../../apps/scriptcraft/src/toolbar.rs:343) using [`toolbar_group_fill()`](../../apps/scriptcraft/src/toolbar.rs:632), [`toolbar_icon_hover_rect()`](../../apps/scriptcraft/src/toolbar.rs:621), two points of hover padding, a two-point corner radius, and the default cursor. Status icon controls should reuse these semantics rather than approximate them with unrelated stock-button styling.
7. Timer activation returns its button rectangle through [`show_with_focus_mode()`](../../apps/scriptcraft/src/status_bar.rs:892) to [`show_application_panels()`](../../apps/scriptcraft/src/main.rs:2478), which anchors the timer popup. Appearance and Focus Mode similarly have existing action routing that must remain unchanged.
8. Current durable requirements explicitly permit status overflow in [`AC-WRITING-006.1`](../../.forge/requirements/features/screenplay-writing-loop.md:569) and [`AC-WRITING-006.4`](../../.forge/requirements/features/screenplay-writing-loop.md:598). Those clauses require scoped revision. The physically unusable/zero-width exception in AC-WRITING-006.4 remains valid; no layout can guarantee full controls in zero width.
9. The historical desktop-container blueprint linked by older work orders is absent from this checkout. This initiative will update the existing status requirements, [`technical-context.md`](../../.forge/blueprints/technical-context.md), [`project-manifest.md`](../../.forge/project-manifest.md), and [`icon_assignement.md`](../../.reference/icon_assignement.md), without recreating unrelated historical blueprint files.

## Target layout contract

At every supported viewport width, including the established 1200, 480, and 320 logical-point review widths:

- Reserve and paint the full right-hand control group inline at the far right. No right-hand control is moved into a menu.
- Keep the visible-page indicator at the left within its own bounded allocation.
- Give the center operation/error message only the space remaining after the right group and page indicator are accommodated. It clips or disappears when space is exhausted; it never pushes the right group out.
- Preserve one stable row height, noninteractive center text, privacy-safe error projection, and the existing optional Diagnostics row.
- Do not render More or an error-decorated More variant. Errors remain identifiable in the center message when space permits and retain their existing accessible projection; this initiative does not invent a replacement error-details overlay.
- Below the minimum physically usable viewport, remain bounded and avoid overlap or unsafe interactions. Do not reintroduce an overflow trigger as a fallback.

## Work-order plan

| ID | Single outcome | Depends on |
|---|---|---|
| [`WO-001`](work-orders/WO-001-egui-034-material-icons-foundation.md) | Migrate the desktop to eframe/egui 0.34 and register egui_material_icons 0.6.0 with filled and outline fonts, preserving all current behavior | None |
| [`WO-002`](work-orders/WO-002-status-material-icons-and-hover.md) | Replace Sprint Timer, Focus Mode, and Appearance text/chrome with the assigned Material icons and toolbar-identical hover treatment while preserving actions and accessibility | WO-001 |
| [`WO-003`](work-orders/WO-003-status-priority-and-zoom-readout.md) | Remove status overflow, enforce right-control rendering priority, and simplify the zoom slider/percentage presentation without changing zoom semantics | WO-002 |

Each order owns its implementation record and scoped durable-document updates. Do not start a later order until the preceding implementation is handed off. If WO-001 exposes a framework behavior regression outside the status bar, repair that migration regression within WO-001 rather than mixing it into icon or spacing work.

## Non-goals

- No toolbar icon reassignment or filling of the WIP toolbar table.
- No custom font assets, font patching, variable-font instantiation, or claim of weight-200 rendering.
- No change to zoom range, presets, viewport-center anchoring, native pinch handling, TextState persistence, document layout, pagination, FDX, save/open, PDF, or plain-text export semantics.
- No new timer, focus, appearance, error, or diagnostics features.
- No redesign of toolbar overflow; only status-bar More behavior is removed.
- No new automated desktop-test target is required by this plan. Any coverage the user requests must be explicitly authorized and reconciled with the consolidated acceptance-suite policy.

## User review and validation handoff

Implementation agents may inspect and edit the authorized source and documentation but must not run validation commands under [`project-manifest.md`](../../.forge/project-manifest.md:73). After each implementation handoff, the user owns compilation and behavior review. Suggested user-run checks are listed in each work order as requests, not executed evidence.

Final desktop review should cover normal and narrow resizing; long operation and error messages; collapsed and expanded Diagnostics; every Appearance choice; enabled, pending, unavailable, entering, and exiting Focus Mode states; timer popup anchoring and continued timer operation; slider dragging and keyboard endpoints; every zoom preset and intermediate values; hover parity; pointer and keyboard activation; and save/reopen confirmation that zoom still round-trips without changing document semantics.

No build, test, lint, formatting, native-window observation, implementation review, validation result, or acceptance is claimed by this planning record.

## WO-003 implementation handoff — 2026-11-04

Implemented WO-003 on the shared branch `dev/2026-11-04-ui-icons`. The status
row now uses bounded explicit regions instead of three equal columns or a
right-item overflow plan. The right-hand Timer, Focus Mode, Appearance, slider,
and percentage slots are allocated first and remain inline; the page label is
kept at the far left and clipped to its measured allocation; operation/error
text receives only the remaining center width. The slider grows from 40 toward
165 points only after fixed controls and the page region fit. At physically
unusable widths, right-side slots and spacing compress proportionally into
bounded, nonoverlapping rectangles without promising operable hit targets.

Removed the status More/! More menu, right-item width/order planner, obsolete
overflow presentation, and the now-unreferenced Focus Mode button-text
projection. Toolbar overflow was not changed. The three Material icons and
toolbar-equivalent hover remain on their prior action/accessibility routes;
the actual Timer icon response rectangle remains the popup anchor. The
Appearance choices, Focus Mode gating, Diagnostics behavior, and page freshness
path were not intentionally changed.

The slider retains 75–300 inclusive whole-percent integer values, trailing fill,
hidden value text, Page zoom accessibility naming/range/value, and Home/End
behavior. The closed percentage is an arrowless current-value control with a
stable popup ID and the exact nine shared presets; its activation continues to
feed the existing zoom-changed result used for pinch-baseline synchronization,
prepaint viewport-center compensation, and root FDX TextState persistence.
Document semantics, pagination, and exports were not changed. The durable
status and zoom contracts are updated in the scoped writing-loop requirements
and technical context; the separate deep-scroll coordinate contract is
preserved.

This is implementation handoff only. WO-001 and WO-002 and all WO-003 behavior
remain unbuilt and unvalidated. No build, check, test, lint, formatting check,
manual verification, implementation review, or independent validation was run.
The eframe/egui migration may still have incompatibilities, and popup,
accessibility, layout, and persistence behavior have no fresh evidence. All
review and validation remain user-owned; no initiative acceptance or closeout
is claimed.

## Implementation authorization — 2026-11-04

The user approved the investigation and three-work-order plan and authorized implementation to begin with WO-001 while preserving sequential boundaries. The user directed one new Git branch for all work and a newly spawned coding agent for each work order. The execution owner should use the manifest-conforming branch name dev/2026-11-04-ui-icons, verify the current checkout and preserve pre-existing work before creating it, and record actual branch/dispatch evidence during execution. This authorization does not permit agent-run validation or independent review agents.

## WO-001 implementation handoff — 2026-11-04

The actual branch is `dev/2026-11-04-ui-icons`, created from the observed
`master` branch. Before branch creation, the checkout contained a user-edited
`.reference/icon_assignement.md` and the untracked initiative plan directory;
both were preserved without staging, committing, discarding, or overwriting
user work.

Implemented WO-001 only: the desktop manifest now requests eframe 0.34 and
egui_material_icons 0.6.0 with the `outline` feature in addition to default
filled/compressed features; Material fonts initialize after Courier Prime; and
desktop framework incompatibilities were adapted in the app entry point,
font-data registration, painter/text layout, AccessKit labels, widget response
accessors, mouse-wheel input detection, row geometry, and stroke painting. The
existing fixed-cell text placement, scroll/zoom ordering and coordinates,
status behavior, and core crate were not intentionally redesigned. Dependency
resolution generated the lockfile and selected eframe 0.34.3 among 437 resolved
packages. Resolution is setup evidence only.

No known migration issue remains from the observed IDE diagnostics after the
compatibility edits, but there is no compilation or behavior evidence; further
incompatibilities may remain. User review and all validation are pending. No
build, test, check, lint, formatting command, manual verification, or agent
review was run. WO-002 and WO-003 remain unstarted, and no initiative acceptance
or closeout is claimed.

## WO-002 implementation handoff — 2026-11-04

Implemented WO-002 on the existing shared branch `dev/2026-11-04-ui-icons`.
The inline Sprint Timer and Focus Mode controls now render the crate's typed
outlined Timer and Location Searching icons, and Appearance renders filled Mode
Night. The closed controls have no visible captions or appearance arrow. One
reusable status-icon control paints through the crate's rich-text/font-family
conversion and uses the toolbar's shared two-point hover-rectangle helper,
hovered-widget fill, two-point corner radius, and default cursor. Icon control
measurements now derive from that helper's fixed interaction bounds rather than
the removed status captions.

Timer activation, the just-opened flag, tooltip/accessibility name, and the
returned icon interaction rectangle remain on the existing routing path for
popup anchoring. Focus enablement and its detailed state-specific accessible
description still come directly from `FocusModeView`; activation continues to
set the existing request flag. Appearance now uses the icon response as its
popup anchor and retains System, Day Mode, Night Mode, and Midnight Mode choices
and selection reporting. The icon helper retains button semantics and pointer,
Enter, and Space activation without requesting focus during ordinary painting.

The intermediate legacy More/overflow path remains. No right-group priority,
overflow-removal, zoom presentation, toolbar, timer/focus/appearance state, or
persistence redesign was included; those layout changes remain WO-003 scope.
No build, check, test, lint, formatting command, manual verification, agent
review, or validation was run. Dependency/API compatibility and all behavior
remain unestablished; user review and validation are pending. WO-003 has not
started and no user acceptance or initiative closeout is claimed.

## Follow-on implementation authorization — 2026-11-04

- **Status:** WO-004 through WO-006 are approved for implementation in sequence by separate fresh coding agents; none is implemented, reviewed, or validated yet
- **Planning and authorization date:** 2026-11-04
- **Branch context:** Reuse the existing shared branch `dev/2026-11-04-ui-icons`; preserve all existing work and do not stage, commit, merge, or switch to another initiative branch
- **Prerequisite:** WO-001 through WO-003 remain implementation-submitted with user review/validation pending. Follow-on authorization does not accept them or erase that boundary
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or validators
- **Resolved color direction:** Default status icon and text foreground must share one easily adjustable status-local value/source, and hovered status foreground must have its own easily adjustable value/source. The circular knob follows the same state-dependent foreground. Numeric spacing, font, and rail values remain unchanged by implementation for later manual user tuning

### Root-cause findings

1. **Percentage hover and muted color.** [`zoom_presets_button()`](../../apps/scriptcraft/src/status_bar.rs:307) creates a click response and tooltip but custom-paints only text. It has no hovered fill, default-cursor request, or interaction-state foreground. Its color is fixed to noninteractive text at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:330), whereas icons use the interaction state at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:477). In the framework dark widgets, those idle sources are gray 140 and gray 180 respectively at [`style.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:1623); hover foreground is gray 240 at [`style.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:1639). The lack of hover is therefore in app-owned painting, not a missing framework animation. Existing icon hover is an immediate state repaint, not a timed transition.
2. **Visible-page unavailability.** [`ViewportPageTracker::visible_page()`](../../apps/scriptcraft/src/status_bar.rs:769) intentionally returns Unavailable for explicit input invalidation, absent/stale/mismatched geometry, or empty pages. Scroll input reaches it through [`page_geometry_input_invalidated()`](../../apps/scriptcraft/src/main.rs:2761) and [`show_application_panels()`](../../apps/scriptcraft/src/main.rs:2500). Status zoom and native zoom clear the tracker at [`main.rs`](../../apps/scriptcraft/src/main.rs:2564) and [`main.rs`](../../apps/scriptcraft/src/main.rs:2666). Independently, [`show_impl_with_appearance()`](../../apps/scriptcraft/src/status_bar.rs:1241) forces Unavailable on any same-frame zoom edit. [`select_visible_page()`](../../apps/scriptcraft/src/status_bar.rs:623) is not the source of those deliberate freshness fallbacks.
3. **Document-replacement hazard.** [`ViewportPageTracker`](../../apps/scriptcraft/src/status_bar.rs:699) has no separate last-valid display value, and [`ViewportPageTracker::invalidate()`](../../apps/scriptcraft/src/status_bar.rs:834) clears the published snapshot. Simply retaining a fallback across that method would also retain it across successful New and Open, whose sessions can share the same revision and viewport. Same-document invalidation and document-lifetime reset must be separate operations, with explicit reset after successful replacement in [`dispatch_toolbar_action()`](../../apps/scriptcraft/src/toolbar.rs:317) and [`activate_opened_project()`](../../apps/scriptcraft/src/main.rs:943). The same-frame zoom override must also be removed for tracker retention to become visible.

### Exact current manual-adjustment map

All values below are logical points unless explicitly described otherwise. No numeric value was changed during this investigation.

| Setting | Current source of truth and value | What can be adjusted now versus what needs new code |
|---|---|---|
| Status icon glyph size | [`status_icon_control()`](../../apps/scriptcraft/src/status_bar.rs:439) passes 18.0 at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:470) | Direct app literal; manually adjustable in one place. It is a logical-point font height, not a device-pixel count |
| Status Body and Button text | Framework defaults are both 13.0 at [`default_text_styles()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:1397). Status consumes Button at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:323) and [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1036), Body at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1107), and default Body at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:526) and [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:927) | No app-owned status text-size constant exists. Add status-local font settings and use them consistently for measurement and painting; do not edit dependency defaults or global styles, which would affect toolbar and other UI |
| Bottom-panel horizontal inner margin | [`TopBottomPanel::bottom`](../../apps/scriptcraft/src/main.rs:2538) uses the default frame. [`PanelSizer::new()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/containers/panel.rs:200) selects [`Frame::side_top_panel()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/containers/frame.rs:190), which supplies horizontal 8 and vertical 2 | No app constant exists. A custom status-panel frame is required to reduce only the horizontal inset; vertical inset and other panels can remain unchanged |
| Status icon hover padding | [`status_icon_control()`](../../apps/scriptcraft/src/status_bar.rs:448) imports the toolbar's 2.0 value from [`TOOLBAR_ICON_HOVER_PADDING`](../../apps/scriptcraft/src/toolbar.rs:18) | The literal is manually adjustable but shared with toolbar hover. A status-local constant is required to halve status padding to 1 without changing toolbar geometry |
| Status icon slot width | [`status_icon_width()`](../../apps/scriptcraft/src/status_bar.rs:435) takes the maximum of interaction width and 36.0. Framework interaction width is 40.0 at [`Spacing::default()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:1438) | Changing only 36.0 has no effect while inherited width is 40.0. Narrowing the slot requires new status-local sizing code; padding alone does not shrink it |
| Right-group gaps | [`show_impl_with_appearance()`](../../apps/scriptcraft/src/status_bar.rs:1118) reads item spacing for allocation and applies the compressed value at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1138). Framework horizontal default is 8.0 at [`style.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:1441). Four gaps are budgeted at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:172) and [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:195) | No status-specific constant exists. Add one setting consumed by both allocator and renderer; changing only one side would cause layout drift |
| Percentage horizontal reservation | [`measured_button_width()`](../../apps/scriptcraft/src/status_bar.rs:1036) adds two inherited button-padding values. The framework default is 4.0 per side at [`style.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:1444) | The custom percentage control has no actual stock-button frame and no app padding constant. Introduce status-local measurement padding rather than changing global button padding |
| Slider rail thickness | [`zoom_slider()`](../../apps/scriptcraft/src/status_bar.rs:275) inherits [`Spacing::slider_rail_height`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:425), default 8.0 at [`style.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:1448). Rail painting consumes it at [`slider.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/widgets/slider.rs:776) | No app value or per-slider rail builder exists. Add a narrow child-Ui spacing override initialized to 8.0. A later user reduction of 30–40% corresponds to 4.8–5.6 points; it is not applied by the proposed implementation |
| Slider knob shape and fill | Framework default is a 0.75-aspect rounded rectangle at [`style.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:1536), painted at [`slider.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/widgets/slider.rs:826) | New code is required. Use per-slider circle shape plus status-local fill painting; there is no app knob constant or direct per-slider fill builder |
| Slider width | [`show_impl_with_appearance()`](../../apps/scriptcraft/src/status_bar.rs:1119) supplies 40.0 and 165.0 at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1125) | Direct app literals, but the user asked to preserve width. Rail-thickness work must not change them |
| Status interaction height | Controls use interaction height at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:277), [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:312), and [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1100); framework default is 18.0 at [`style.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:1446) | Not a requested numeric adjustment. Preserve it for hit targets, row stability, knob radius, and clipping rather than treating it as disposable padding |

[`FontId::size`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/epaint-0.34.3/src/text/fonts.rs:27) is measured in points, and egui paint coordinates are screen-space points rather than physical pixels at [`shape.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/epaint-0.34.3/src/shapes/shape.rs:18). [`Context::pixels_per_point()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/context.rs:2238) gives the physical-pixel multiplier. Thus a 1-point margin is one physical pixel only at 1.0 pixels per point. If “1px” literally means one device pixel on every display, a separate pixels-per-point conversion is new code and must not be replaced with an assumed 1.0-point constant.

### egui 0.34 slider implementation and shared-style impact

- Select a circle only for this slider through [`Slider::handle_shape()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/widgets/slider.rs:388) and [`HandleShape::Circle`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/style.rs:1214). Changing global visuals would affect all sliders in that context.
- Override rail height only inside the status slider's child Ui using [`Ui::scope()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/ui.rs:2399) and [`Ui::spacing_mut()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/ui.rs:455). There is no per-slider rail-height builder.
- Preserve the stock slider response and accessibility. Compute the stock-compatible normalized center and radius from [`Slider::handle_radius()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/widgets/slider.rs:886), then apply a clipped final circle fill from the same interaction-state foreground source as status icons. Match the stock foreground stroke so no mismatched rim remains.
- Do not obtain the knob fill by changing widget background fill. The stock circle uses interaction background at [`slider.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/widgets/slider.rs:817), but the rail also uses inactive widget background at [`slider.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/widgets/slider.rs:772). A shared style edit can therefore recolor both. Trailing fill has a separate selection color at [`slider.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/widgets/slider.rs:787) and must remain intact.
- This is a source-derived implementation design, not runtime evidence. Exact final paint/clipping appearance remains user-review pending.

### Authorized sequential work orders

| ID | Single outcome | Depends on | Numeric changes by implementation |
|---|---|---|---|
| [`WO-004`](work-orders/WO-004-document-scoped-visible-page-retention.md) | Retain the last valid page only within the active document, with explicit startup/New/Open reset semantics | WO-003 | None |
| [`WO-005`](work-orders/WO-005-status-local-presentation-and-percentage-hover.md) | Add status-local presentation controls, centralize shared default/hovered text-and-icon foreground values, and give percentage icon-equivalent immediate hover while preserving popup/accessibility behavior | WO-004 | None; expose current effective values for optional user adjustment |
| [`WO-006`](work-orders/WO-006-status-slider-circular-knob-and-rail-setting.md) | Render a circular icon-colored knob and expose a status-local rail-thickness setting while preserving stock slider interaction and width | WO-005 | None; initialize rail setting at inherited 8 points |

The user authorized these orders on 2026-11-04 and explicitly requested individual coding agents. They remain sequential on the existing shared branch. A later order must not start until the preceding implementation handoff exists. No user acceptance or initiative closeout is inferred from this authorization.

### Scope and numeric-adjustment separation

The authorized implementation scope is limited to behavioral page retention, percentage hover parity, shared status foreground settings, circular knob rendering, and the status-local infrastructure needed to make the requested numbers safely adjustable. It does not authorize an agent to apply the user's numeric reductions.

WO-005 owns one shared default/idle foreground source for status-owned text and icons, including the percentage, page indicator, center readout, and Diagnostics text, plus a separate shared hovered-foreground source for interactive status controls. Both must be named, commented, and editable in one status-local presentation surface. Preserve theme-appropriate behavior across all Appearance choices rather than introducing an unreadable fixed light-theme or dark-theme literal; if theme-derived defaults are retained, expose explicit override points in the same surface. Popup-menu entries keep their existing menu styling. WO-006 consumes this shared foreground contract for the knob instead of creating another color source.

After WO-005/WO-006 implementation, the user can manually select:

- horizontal status inset at most 1 logical point, or separate conversion code if literal physical pixels are intended;
- status icon padding of 1 point, plus independently reduced slot width and group gaps if needed;
- status Body/Button/icon sizes reduced by one or two points, with measurement and clipping re-review;
- rail thickness of 4.8–5.6 points for a 30–40% reduction while preserving width and interaction height.

The toolbar's 2-point hover padding, global egui defaults, document fonts, slider 40–165-point width, and 18-point interaction height are not tuning targets in this proposal.

### Durable requirements and blueprint impacts for implementation

- WO-004: revise [`AC-WRITING-006.6`](../../.forge/requirements/features/screenplay-writing-loop.md:628) from unavailable-on-every-stale-frame to last-valid retention scoped to one document, with explicit startup/replacement unavailability and no revision-alias regression. Preserve [`AC-WRITING-006.5`](../../.forge/requirements/features/screenplay-writing-loop.md:622). Update the geometry/status-before-editor contract in [`technical-context.md`](../../.forge/blueprints/technical-context.md:41).
- WO-005: revise [`AC-WRITING-006.1`](../../.forge/requirements/features/screenplay-writing-loop.md:569) so hover semantics are shared without one toolbar/status padding constant, add percentage hover, and record shared default/hovered status text-and-icon foreground sources. Reconcile status-local inset/gap/color ownership in [`AC-WRITING-006.4`](../../.forge/requirements/features/screenplay-writing-loop.md:608) and [`technical-context.md`](../../.forge/blueprints/technical-context.md:34). Do not record reduced numbers as applied until the user selects them.
- WO-006: revise [`AC-WRITING-006.7`](../../.forge/requirements/features/screenplay-writing-loop.md:634) for the circular knob and local rail setting while preserving width, range, trailing fill, accessibility, and zoom semantics; update the corresponding technical-context zoom paragraph.
- No durable requirement, blueprint, manifest, or icon-assignment file was edited by the planning task. Each implementation agent must reconcile only its scoped durable records. The user's completed color direction supersedes the earlier unfinished sentence; it does not require unrelated toolbar or global Appearance palette changes.

### User-owned validation handoff for implementation

No review or validation is performed or claimed by this plan. Each order must record an implementation-only handoff and the user separately owns all compilation, checks, tests, lint, formatting, native observation, accessibility inspection, review, and acceptance.

The combined user review matrix should cover:

1. **Page lifecycle:** initial unavailable-to-valid transition; continuous scroll and zoom; resize/reflow/reveal; persistent same-document failure and recovery; successful New/Open with different totals and aliasing revisions; failed Open; same-document Save; and proof that no prior-document value survives replacement.
2. **Presentation and hover:** all four appearances; status icon versus percentage hover fill/foreground/radius/cursor; no hover activation or focus theft; stable popup ID, exact presets, outside-click/Escape, keyboard activation, and clipping.
3. **Manual numeric tuning:** user-selected inset, padding, slot/group gaps, text sizes, and rail thickness at 1200, former-breakpoint, 480, 320, and extreme widths; right-edge fit; page/center clipping; stable Diagnostics height; usable hit regions at supported widths; and logical-versus-physical scale observations.
4. **Slider behavior:** circular knob and matching rim at minimum/intermediate/maximum and idle/hover/drag/focus; unchanged trailing fill and width; arrows/Home/End; AccessKit range/value/step and increment/decrement/set-value; percentage/pinch/deep-scroll synchronization; and Save/reopen zoom restoration.
5. **Isolation:** unchanged toolbar hover/typography, document fonts, other sliders, global Appearance, popup-menu presentation, editor input/focus, fixed pagination, persistence, and exports. Any desired Cargo/desktop checks are user-run requests, never agent evidence.

### Pending decisions and uncertainties

- **Resolved color direction:** The user subsequently clarified that default status icon and text color must match and that both default and hover color values must be easy to change. WO-005 therefore owns shared status-local default and hovered foreground settings; WO-006 reuses them for the knob. This replaces the earlier unresolved-color boundary.
- **Units caveat:** The requested 1px margin is not automatically one logical point at every display scale. The plan exposes logical-point settings consistent with egui and leaves any literal device-pixel conversion as additional code rather than silently equating units.
- **Evidence boundary:** Source inspection identifies deterministic code paths and framework defaults, but no native visual result, framework compatibility, accessibility behavior, popup behavior, or final circle-paint appearance is established. WO-001 through WO-005 implementation validation remains user-pending; WO-005 also records an unresolved source/handoff mismatch in the WO-004 page-retention implementation. No implementation is represented as reviewed, validated, or accepted.

## WO-004 implementation handoff — 2026-11-04

Implemented WO-004 on the existing shared branch `dev/2026-11-04-ui-icons`.
The page tracker now separates its actual published editor geometry from a
document-scoped last-valid visible-page display. A valid selection from accepted
fresh geometry replaces the retained value; invalid, missing, stale, mismatched,
or explicitly invalidated same-document geometry falls back to the retained value
or to Page unavailable if none exists. Same-document invalidation still clears
the geometry snapshot and requests another publication, but does not erase the
retained page/total. The independent same-frame zoom Unavailable override was
removed.

Successful New resets both tracker lifetimes after session replacement is
accepted; successful Open resets them after replacement activation. Failed Open
does not reset the active document's value. Save, zoom, resize, scrolling,
reflow, and same-document failure routes use ordinary geometry invalidation and
retain the last valid display. Revision equality is not used as document
identity. Actual geometry publication and page selection rules, zoom anchoring,
native pinch normalization, TextState persistence, pagination, exports, editor
input/focus, accessibility projection, and bounded status layout were not
intentionally redesigned. AC-WRITING-006.6 and the scoped geometry/publication
description in the technical context were updated; AC-WRITING-006.5 was left
unchanged. No spacing, typography, rail, or color setting was changed.

The retained total may describe a prior valid layout of the same document during
a prolonged refresh or failure, by design; only fresh valid geometry makes it
current again. The desktop implementation and durable-record updates have not
been built, checked, reviewed, or behaviorally validated. No build, check, test,
lint, formatting command, manual verification, review, or validator was run.
User review and all validation remain pending; compilation and runtime,
accessibility, lifecycle, and zoom behavior are unverified. WO-005 and WO-006
remain unimplemented. No initiative acceptance or closeout is claimed.

## WO-005 implementation handoff — 2026-11-04

Implemented WO-005 on the existing shared branch `dev/2026-11-04-ui-icons`.
[`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs) now owns the
`presentation` settings surface for status-only sizing, spacing, and shared
foreground policy; [`main.rs`](../../apps/scriptcraft/src/main.rs) applies the
status-specific frame only to the bottom panel. Initial numeric settings retain
their effective source values: horizontal inset 8 points, status hover padding
2, icon slot 40, item gap 8, percentage reservation 4 points per side, Body and
Button sizes 13 points each, and Material icon glyph size 14 points. No requested
numeric reductions were applied. The icon source's existing `.size(14.0)` does
not match the planning inventory's earlier 18-point estimate; the implemented
setting preserves the source's current 14-point behavior.

`DEFAULT_FOREGROUND_OVERRIDE` and `HOVERED_FOREGROUND_OVERRIDE` are the two
named, commented color adjustment points in `status_bar.rs::presentation`. Both
are theme-derived while `None`: default status text/icons follow the active
theme's text foreground, and interactive Timer, Focus Mode, Appearance, and
percentage hover follow its hovered-widget text foreground. All status-owned
page, center operation/error, and Diagnostics text uses the shared default
foreground. Diagnostics' local heading and summary follow an explicit default
override as well. Popup-menu entries remain menu-owned. No toolbar or global
Appearance color changed.

The status panel retains framework fill, separator/stroke behavior and 2-point
vertical inset while exposing its 8-point horizontal inset. Local slot width,
padding and gaps no longer depend on toolbar numeric settings or global
interaction width; allocation and rendering use the same item-gap setting.
Percentage now shares the icon hover painter and has clipped hover fill,
two-point radius, default cursor, and shared hovered foreground, with no timed
animation or hover activation. Existing popup identity, presets, keyboard and
pointer activation, dismissal and accessibility routing were not intentionally
changed. The WO-004 page retention, WO-006 knob/rail, slider dimensions, toolbar,
and global styles were not intentionally modified.

Implementation-source adjustment locations are enumerated in the WO-005
work-order handoff. The scoped durable records changed are AC-WRITING-006.1,
AC-WRITING-006.4 and the status paragraph in `technical-context.md`. The WO-005
source inspection found a real predecessor discrepancy at that time: retained
page state and `reset_for_document_replacement()` were missing while New/Open
already referenced the method. The narrowly scoped WO-004 reconciliation below
has since repaired the tracker; page-retention behavior remains unvalidated and
user-owned.

No build, check, test, lint, formatting command, manual verification,
implementation review, or independent validation was run. User review and all
validation remain pending. Compilation, theme legibility, status hover, popup
and accessibility behavior, layout, and WO-004 page retention remain
unverified. WO-006 remains unimplemented; no initiative acceptance or closeout
is claimed.

### WO-004 source reconciliation — 2026-11-04

The WO-005 handoff's source discrepancy was confirmed: at its inspection point,
`ViewportPageTracker` lacked independent retained page state and the document
replacement reset method, while successful New/Open call sites already referenced
that method. The tracker has now been reconciled with WO-004: only a valid page
selected from accepted fresh geometry updates the retained value; same-document
missing, stale, mismatched, invalidated, or invalid geometry falls back to that
value (or Page unavailable before the first valid value). Ordinary invalidation
clears published geometry but retains display state; successful New/Open reset
both after their replacement sessions are accepted. Failed Open and same-document
Save do not use the document reset. The zoom renderer had no independent same-frame
Unavailable override, so that portion required no edit. No WO-005 settings,
presentation, hover, or documentation was reverted or weakened, and WO-006 was
not implemented.

This is a source repair/handoff update only. No builds, checks, tests, lint,
formatting, manual verification, reviews, or validators were run. Compilation,
runtime and lifecycle behavior, and all WO-005 user review remain pending user-owned
validation; initiative acceptance and closeout are not claimed.

## WO-006 implementation handoff — 2026-11-04

Implemented WO-006 on the existing shared branch `dev/2026-11-04-ui-icons`.
The status-local rail adjustment point is
`status_bar.rs::presentation::ZOOM_SLIDER_RAIL_HEIGHT`, initialized to the
inherited 8 logical points. `zoom_slider()` applies that value only in its
narrow scoped child Ui; no global spacing, other slider, interaction height, or
40–165-point width allocation was changed. The optional 30–40% rail reduction
was not applied.

The status zoom slider alone selects egui's circular handle shape. The stock
slider still owns its response, range/value mapping, integer stepping, trailing
fill, pointer/keyboard/accessibility input, and zoom-change behavior. A clipped
final circle paint follows the stock horizontal position range, handle radius,
and interaction expansion. Both its fill and stroke use the existing shared
status-local default/hovered foreground functions associated with
`DEFAULT_FOREGROUND_OVERRIDE` and `HOVERED_FOREGROUND_OVERRIDE`; no knob-only
color literal or widget-background mutation was added, so the rail and
value-proportional trailing fill remain stock-painted beneath the knob.

The scoped durable changes are AC-WRITING-006.7 and the WO-006 status-slider
paragraph in `technical-context.md`. WO-004 retained-page behavior and all
WO-005 status-local presentation, hover, color, and zoom behavior were preserved;
no redesign or numeric value change is intended. Source and dependency
documentation were read for implementation. No build, check, test, lint,
formatting command, manual verification, agent review, or other validation was
run. Compilation, geometry/clipping, theme appearance, interaction,
accessibility, lifecycle, and zoom behavior remain unverified and explicitly
pending user-owned validation. Initiative acceptance and closeout are not
claimed.

## Final polish authorization — 2026-11-05

The user authorized two additional sequential work orders and a fresh coding
agent for each. Both reuse the existing shared branch
`dev/2026-11-04-ui-icons`; all existing uncommitted, untracked, predecessor, and
manually adjusted work must be preserved. Nothing is staged, committed, merged,
or validated by this authorization.

| ID | Single outcome | Depends on |
|---|---|---|
| [`WO-007`](work-orders/WO-007-status-zoom-knob-paint-priority-and-clipping.md) | Paint the custom zoom knob after all ordinary right-hand siblings and clip it to the parent row/group rather than its own response rectangle | WO-006 |
| [`WO-008`](work-orders/WO-008-status-right-edge-measurement-and-alignment.md) | Measure the actual current percentage and right-align its galley to the explicit trailing inset while preserving user-adjusted values | WO-007 |

WO-007 must not start WO-008. WO-008 starts only after WO-007's implementation
handoff. Tooltip text remains a manual user adjustment and is outside both
orders. No numeric spacing, font, rail, or color value is reset or reduced by
these agents. Each order updates only its scoped requirements, technical context,
initiative record, and implementation handoff. User review and all compilation,
runtime, visual, accessibility, and regression validation remain pending.

## WO-007 implementation handoff — 2026-11-05

Implemented WO-007 on the existing shared branch `dev/2026-11-04-ui-icons` in
[`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs). The zoom-control
path now carries the actual stock slider response separately from its
zoom-changed result. `zoom_slider()` no longer immediately paints the custom
knob; stock slider construction, circular handle selection, input, AccessKit
metadata, value mapping, and zoom-change behavior remain in place.

The final custom circle paint is invoked inside the right-hand status region,
after ordinary slider and percentage, Appearance, Focus Mode, and Timer painting
has completed, using the then-current zoom value. It remains the final ordinary
right-group status paint. Its painter clip is the right-group rectangle
intersected with the status-row rectangle and the inherited parent clip, not the
slider response rectangle. This permits the circle's normal outline and
interaction expansion to extend into right-group gaps at slider endpoints while
preventing escape from the status row or parent clip. The painter remains on the
ordinary status layer; popup menus and unrelated overlays are not promoted over
or painted over by a global foreground layer. The shared default/hovered
foreground source, stock circle geometry and stroke, slider rail/fill, and all
user-adjusted numeric/color settings in `status_bar.rs::presentation` are
unchanged.

The scoped durable changes are AC-WRITING-006.7 and the zoom/status-slider
paragraph in `technical-context.md`; this initiative record and WO-007 handoff
record this implementation. WO-008 right-edge measurement/alignment was not
started, and no tooltip text or presentation value was edited. No code or
dependency blocker was identified from the authorized source/document reading;
however, the rendered endpoint appearance, row/parent clipping under runtime
conditions, popup layering, compilation, interaction, accessibility, and
preserved zoom behavior have no new evidence. No build, check, test, lint,
formatting command, manual verification, review, or validator was run. All
review and validation remain explicitly user-owned and pending; initiative
acceptance is not recorded.

## WO-008 implementation handoff — 2026-11-05

Implemented WO-008 on the existing shared branch `dev/2026-11-04-ui-icons` in
[`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs). At
[`show_impl_with_appearance()`](../../apps/scriptcraft/src/status_bar.rs:1248),
the current whole-percent label is captured once for the frame; its text advance
is measured by [`measured_button_width()`](../../apps/scriptcraft/src/status_bar.rs:1205)
and passed to the existing row allocator. Allocation width remains the current
label's text advance plus the existing leading/trailing percentage reservations;
the fixed “300%” measurement is gone. The same captured label is passed to
[`zoom_presets_button()`](../../apps/scriptcraft/src/status_bar.rs:451) for the
visible galley and accessible current-value name, keeping the measured and
rendered label identical for the frame.

At the percentage galley paint location
([`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:479)), the text is
right-aligned using the current `PERCENTAGE_SIDE_RESERVATION` as its explicit
trailing inset, while vertical centering remains unchanged. When compression
cannot fit the text and inset, the origin is bounded to the allocation and the
existing clip bounds paint. The right group remains anchored at the status content
edge. Its remaining intentional trailing distance from the status-panel outer
edge to the galley's trailing advance edge is the sum of
[`PANEL_HORIZONTAL_INSET`](../../apps/scriptcraft/src/status_bar.rs:23) and
[`PERCENTAGE_SIDE_RESERVATION`](../../apps/scriptcraft/src/status_bar.rs:31)
logical points (currently 12 + 8 = 20 points); font bearings may add visible-ink
distance.

Only WO-008's AC-WRITING-006.4/006.7 clauses and status allocation/zoom paragraph
in `technical-context.md` were updated alongside this initiative and work-order
handoff. All current user-adjusted numeric and color values were preserved.
WO-007's final custom knob paint after ordinary right-hand siblings and its
right-group/status-row/parent clipping remain untouched. Popup identity/presets,
keyboard/dismissal/accessibility behavior, hover, page retention, zoom semantics,
and tooltip text were not intentionally changed. No blocker was identified from
authorized source and documentation reading. No build, check, test, lint,
formatting command, manual verification, review, or validator was run. Layout,
interaction, accessibility, clipping, compilation, and regressions have no new
validation evidence; all review, validation, and acceptance remain user-owned and
pending.

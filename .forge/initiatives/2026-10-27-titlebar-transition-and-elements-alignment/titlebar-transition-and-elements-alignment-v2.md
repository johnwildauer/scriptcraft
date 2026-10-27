# Initiative: Title-Bar Transition and Elements Alignment — v2 Research Draft

## Metadata and iteration boundary

- **Phase:** `closeout`
- **Status:** `closed`
- **Authoritative version:** `v2` — new research iteration; this is not a revision, reopening, or replacement of closed v1.
- **Created/updated:** `2026-10-27`
- **Mode:** `standard`; the user explicitly requested conversion of this research into a discrete v2 planning iteration.
- **Parent:** [`titlebar-transition-and-elements-alignment-v1.md`](titlebar-transition-and-elements-alignment-v1.md:1), closed after four validated work orders and merged to `master`.
- **Allowed artifact:** this v2 plan and the four v2-specific work orders listed below. Implementation and validation artifacts are now recorded; v2 remains separate from the closed v1 history.

## Branch/base evidence recorded before investigation

The repository was checked through the Git reference and reflog files before source investigation:

- [`HEAD`](../../../../.git/HEAD:1) contains `ref: refs/heads/master`; the current checkout is therefore `master`.
- [`refs/heads/master`](../../../../.git/refs/heads/master:1) contains the exact current HEAD `e77d6c4ee424403c5ab779d372745a51e4cafd4e`.
- [`logs/refs/heads/master`](../../../../.git/logs/refs/heads/master:11-13) records the v1 merge from `5b8eda725f767fbed530c021882b15bec82b6ceb` to `11bf317a11dfdc1ebfa7f49bc229c424849f37a6`, followed by the closeout commit `e77d6c4ee424403c5ab779d372745a51e4cafd4e`.
- [`logs/HEAD`](../../../../.git/logs/HEAD:76-81) independently records checkout from `master` to the v1 branch, commit `28024cdff84c4a60aa6c2fb74123745ec270905c`, merge commit `11bf317a11dfdc1ebfa7f49bc229c424849f37a6`, and closeout commit `e77d6c4ee424403c5ab779d372745a51e4cafd4e`.
- **Base branch:** `master`. **Initiative branch for v2:** none; this draft does not create or switch branches. The v1 plan's recorded merge and the live ref/reflog evidence agree that the v1 implementation is in the current base.
- **Cleanliness limitation:** this session did not run `git status`; the exact branch/ref evidence above is recorded, but no stronger clean-working-tree claim is made.

## Context carried forward from v1

The closed v1 outcome, scope, requirements, API boundary, and validation record are retained as historical context rather than repeated as current proof. V1 established deterministic source tests for known/unknown maximize command selection, unified title/toolbar geometry, bounded width-derived presentation metrics, closed-only Elements centering, and cursor intent. Its native macOS smoke was unavailable, so it did not observe real OS/backend rectangles, transition frames, pixels, cursor changes, or mouse/keyboard behavior. See the v1 [`Outcome`](titlebar-transition-and-elements-alignment-v1.md:34-46), [`Exploration scratchpad`](titlebar-transition-and-elements-alignment-v1.md:72-111), [`closeout record`](titlebar-transition-and-elements-alignment-v1.md:170-176), and the four v1 validation artifacts.

The current source/API baseline is:

- [`TOOLBAR_ELEMENTS_WIDTH`](../../../../apps/scriptcraft/src/main.rs:26-28) remains the user's local `128.0`-point baseline.
- The app depends on eframe `0.29` through [`apps/scriptcraft/Cargo.toml`](../../../../apps/scriptcraft/Cargo.toml:8-11).
- [`native_options()`](../../../../apps/scriptcraft/src/main.rs:1899-1909) requests hidden title/titlebar presentation and full-size content.
- [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:1806-1829) owns a full-width egui response and sends a viewport command only after a double-click.
- [`presentation_geometry()`](../../../../apps/scriptcraft/src/main.rs:1561-1570) derives page width and scale from available width; [`presentation_metrics()`](../../../../apps/scriptcraft/src/main.rs:1579-1591) derives font size from that scale; [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:1126-1145) consumes those metrics.
- [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:714-769) retains stock ComboBox response/ID/popup behavior while painting a closed-only centered galley.
- [`chrome_frame()`](../../../../apps/scriptcraft/src/main.rs:1741-1748) provides the explicit open-widget fill, no stroke, and no frame inset; [`update()`](../../../../apps/scriptcraft/src/main.rs:1831-1843) places title and toolbar in one top panel.
- [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:1198-1204) creates the only document editing response and requests the text cursor; the toolbar cursor helper is [`toolbar_cursor_icon()`](../../../../apps/scriptcraft/src/main.rs:1706-1712).

The installed API evidence recorded by v1 remains the applicable eframe/egui 0.29.1 evidence: `ViewportInfo::maximized` is optional observed state, `ViewportCommand::Maximized(bool)` is a request, and egui-winit's macOS path can leave maximize state stale after initialization. The v1 API/source citations and interpretation are preserved in [`WO-001`](work-orders/WO-001-viewport-state-direction-and-native-restore-contract.md:47-54) and [`validation-WO-001`](validations/validation-WO-001.md:29-33). No portable animation-progress, duration, interpolation, or pixel-freeze hook was found in that evidence.

## One coherent v2 outcome

Produce one evidence-bounded diagnosis and implementation-ready direction for the three post-v1 desktop-shell defects: transition-time visual scaling, Elements cursor ownership, and the black toolbar-bottom region. The outcome is a v2 research decision record, not a source fix or approval. It must distinguish what is proven by source/API inspection from what requires a window-capable macOS run.

### In scope

- Trace maximize/unmaximize requests, observed viewport state, native resize implications, egui available-width layout, `presentation_metrics()`, page scale, and renderer font size.
- Determine the strongest feasible scale-stability guarantee without inventing a native animation hook.
- Trace every Elements response and cursor owner for the ComboBox, closed galley, centered label, group, and popup; identify the competing path that can win over the intended standard arrow.
- Trace title/toolbar panel, frame, fill, stroke, separator, padding, clip, content, and native full-size-content inset ownership for the black bottom bar.
- Compare current source with recorded v1/pre-v2 evidence where available, retaining the `128.0` baseline.
- Record hypotheses, options, risks, unresolved decisions, validation measurements, native-smoke limitations, durable-document impact hypotheses, and likely future work-order boundaries.

### Out of scope

- Any application source or test change.
- Any requirements, blueprint, or v1 plan/log/work-order/validation/review modification.
- Creating a design review, validation results, or implementation evidence in this authoring session; matching validation placeholders are allowed only as pending filename anchors.
- Promising native animation duration/interpolation, OS animation suppression, exact universal restore frames, or pixel-identical egui content during native resize.
- Changing toolbar width, role semantics, document input behavior, persistence, export, title text, full-screen behavior, or broad chrome redesign.

## Investigation A — Maximize ↔ user-state transition scaling

### Observed request and state path

1. A double-click is detected by the full-width response in [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:1806-1829).
2. The caller reads `input.viewport().maximized` and delegates to [`title_strip_viewport_command()`](../../../../apps/scriptcraft/src/main.rs:1798-1804).
3. `Some(false)` produces `ViewportCommand::Maximized(true)`; `Some(true)` produces `ViewportCommand::Maximized(false)`; `None` produces no command. The mapping is directionally correct if the optional state is truthful.
4. `ctx.send_viewport_cmd()` submits a request to eframe/winit; it is not an acknowledgement, native animation controller, or restore-frame snapshot. Native unmaximize may restore the OS/backend's prior user frame, but the app has no universal exact-frame contract.
5. [`native_options()`](../../../../apps/scriptcraft/src/main.rs:1899-1909) uses hidden title/titlebar and full-size content. These are native presentation hints and can affect the egui content rectangle/inset; they do not expose animation progress.

### Resize-to-metric path

During a native width change, the egui available width can change between frames. [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:1135-1145) passes that width to [`presentation_metrics()`](../../../../apps/scriptcraft/src/main.rs:1579-1591), which calls [`presentation_geometry()`](../../../../apps/scriptcraft/src/main.rs:1561-1570). The default page width is `paper_width * 1.6 * 360 / paper_width = 576` points; when usable width falls below that default, page width becomes width-constrained. The scale is `page_width / paper_width`; the renderer uses that scale for page geometry, line positions, caret geometry, and [`renderer_font()`](../../../../apps/scriptcraft/src/main.rs:1612-1620) font size. Toolbar icons and egui text are independently laid out/painted in logical egui points, but the native content resize changes their available layout and can make rasterized text/icon bounds appear to grow or snap as the window crosses layout/scale thresholds.

### Finding and hypothesis

- **Observed/proven:** v1's fixed-width tests show equal metrics at representative `800` and `1400` available widths because both have enough room for the 576-point page; the narrow case is intentionally width-constrained. This proves only the helper's deterministic behavior, not native transition-frame behavior.
- **Strong hypothesis:** the apparent grow/snap-back is a composition of OS-owned native window animation changing the egui content width, width-derived page/font scale changing below the 576-point threshold, and discrete frame sampling/rasterization. It is not evidence that `Maximized(true/false)` itself animates egui scale.
- **Additional hypothesis:** full-size-content/titlebar insets may change the width/clip/content rectangle during the native transition, making the width-derived metric change look larger or discontinuous. This requires native rectangle samples.
- **Not supported by evidence:** an app-owned animation-progress signal, a safe guarantee that polling desktop dimensions gives the current native animation state, or a portable way to force native interpolation.

### OS polling and manual animation feasibility

Polling OS desktop width/height and manually animating a Rust/eframe presentation transform is technically imaginable but not a safe v2 default. Desktop dimensions are monitor/work-area facts, not authoritative window outer/inner bounds or maximize-transition progress; they can be stale, multi-monitor-dependent, affected by scale factors and insets, and sampled at a different cadence from winit/egui frames. It would also introduce a second geometry authority and could race the backend's viewport request. Rust memory safety does not make the behavioral contract safe. In eframe 0.29, the supported portable boundary is sending viewport requests and observing optional viewport information; no installed evidence exposes a native animation timeline, duration, interpolation, cancellation, or frame-freeze control. A platform-specific macOS integration could be researched separately, but it would be outside the current eframe contract and would need explicit unsafe/native API and multi-monitor policy review.

### Options

| Option | Benefit | Risk/limitation | v2 disposition |
|---|---|---|---|
| Keep native request; measure and document bounded metric behavior | Smallest and supported; preserves native UX | Cannot guarantee constant scale during OS animation | Strong default |
| Freeze page/font scale at last stable user width during transition | Could reduce visible scale jump | No reliable transition boundary/progress; stale freeze can persist | Research only; needs native evidence |
| Use a fixed presentation scale independent of available width | Stable text/page metrics | Can clip or waste space and changes current responsive behavior | Reject unless product chooses fixed-scale UX |
| Poll desktop/work-area size and manually animate | Apparent control | Wrong authority, races native backend, multi-monitor/HiDPI hazards | Reject as unsupported/safety-poor default |
| Platform-specific native animation hook | Potential exact macOS behavior | Not eframe-portable; API/lifecycle and maintenance risk | Future research only |

### Strongest feasible guarantee

Guarantee only that maximize/unmaximize uses the known-state request contract; semantic layout/profile metrics remain unchanged; and at stable widths with enough usable space for the default page, `presentation_metrics()` remains at the default page/font scale. During OS-owned resize, report width-derived paint-scale variation as bounded and platform-dependent. Do not guarantee constant icon/text raster size, animation smoothness, transition duration, or exact native-frame metrics. A stronger guarantee requires a window-capable macOS experiment first and likely a deliberate presentation-policy change.

### Required future measurements

Record per frame: timestamp, `ViewportInfo::maximized`, optional outer/inner rects, egui content/clip/available rects, page width, scale, font size, representative toolbar galley/icon bounds, OS/backend, display scale, and command result. Compare pre-command, intermediate samples, post-maximize, pre-restore, intermediate restore, and post-restore. Separate native outer-size animation from egui layout scale and from renderer rasterization.

## Investigation B — Elements cursor ownership

### Current ownership trace

- The outer Elements group is allocated by [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:714-724).
- The stock [`ComboBox`](../../../../apps/scriptcraft/src/main.rs:725-736) owns the closed button response, widget ID, arrow, selected-text slot, popup opening, popup response tree, keyboard/open behavior, and `selectable_value` role routing.
- The stock selected galley is suppressed with `.selected_text("")`; when `combo_result.inner.is_none()`, the code paints a separate centered role galley in [`elements_closed_text_rect()`](../../../../apps/scriptcraft/src/main.rs:740-755). That painted galley has no response and cannot itself own a cursor.
- The popup is the stock `show_ui` path and its role rows are stock `selectable_value` responses, so popup cursor/interaction ownership is distinct from the closed galley.
- The decorative `ui.label(item.label)` is still a child label response at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:757); the outer group response then calls `.on_hover_cursor(toolbar_cursor_icon())` at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:761-765). The label is intended to be non-interactive, but the parent cursor request does not erase a child response's cursor decision when egui resolves the hovered response tree.
- The document I-beam belongs to the later, non-overlapping whole-surface response in [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:1198-1204). Painted page rectangles and painted closed text are not responses. If an I-beam is visible over Elements in a native run, either the document response rectangle/clip is overlapping the toolbar unexpectedly or a child/stock ComboBox/label cursor request is winning; source geometry must be logged rather than inferred from the helper test.

### Finding and competing path

The strongest source-level competing owner is the interactive/response tree inside the stock ComboBox and its child label response, not the centered galley. Applying `Default` to `combo_result.response` and then separately to the group response is not a complete ownership contract for the label region: the label response remains independently eligible, and a parent response cannot be assumed to override a more specific hovered child. The exact runtime winner between stock ComboBox internals and the label response is unresolved without the installed egui implementation source at the current tool-visible path and native pointer observation; v1's deterministic cursor tests checked intent/geometry, not the actual macOS cursor.

### Smallest fix direction

Make the Elements group a single explicit standard-arrow owner over the closed control and label region while keeping the stock ComboBox response/ID/popup route intact. The minimal candidate is to make the decorative label non-interactive (paint its text or use a non-sensing allocation) and apply the standard cursor to the authoritative group/ComboBox response; do not add a response to the painted centered galley. If the stock ComboBox internally requests a different cursor, use the smallest local wrapper/response ordering supported by egui 0.29 rather than replacing the ComboBox. Preserve `128.0`, the arrow-excluded centering rectangle, stock popup left alignment, keyboard/open behavior, role routing, and document response ownership. Validate pointer positions over arrow, selected text, label, group gap, popup rows, and document surface separately.

### Options and risks

1. **Non-sensing label plus one group cursor owner — preferred.** Smallest ownership cleanup; risk is that the stock ComboBox still wins over the parent for a subregion.
2. **Explicit closed overlay response with standard cursor — fallback.** Covers the whole closed group; risk of stealing ComboBox click/keyboard identity or blocking popup opening.
3. **Custom closed widget plus stock popup — avoid unless necessary.** Maximum cursor control; highest risk to ID, keyboard, arrow, truncation, accessibility, and popup behavior; v1 already selected the stock-owner approach.

The unresolved decision is whether the observed I-beam is caused by the decorative label response, a stock ComboBox cursor path, or an actual document-response overlap. A deterministic response ledger plus native cursor observation is required before source selection.

## Investigation C — Black toolbar bar

### Rendered surface/frame trace

- [`update()`](../../../../apps/scriptcraft/src/main.rs:1835-1843) creates one `TopBottomPanel` with exact height `TITLE_STRIP_HEIGHT + toolbar_content_height(style)` and disables the panel separator.
- [`chrome_frame()`](../../../../apps/scriptcraft/src/main.rs:1741-1748) fills that panel with `Widgets::open::weak_bg_fill`, has no stroke, and has no explicit inner/outer margin.
- The title response is exactly 30 points in [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:1806-1821).
- [`toolbar_content_height()`](../../../../apps/scriptcraft/src/main.rs:1775-1777) is two egui interaction heights plus `TOOLBAR_BOTTOM_PADDING = 2.0`; [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:809-860) uses top padding `0.0`, row padding `4.0`, explicit inter-item gaps, and bottom padding `2.0`.
- The toolbar row is allocated inside the panel; the shared frame paints the panel surface behind it. Action hover fills are restricted to icon hover rectangles by [`toolbar_action_button()`](../../../../apps/scriptcraft/src/main.rs:649-699); they are not the persistent surface fill.
- The central document surface starts after the top panel in [`update()`](../../../../apps/scriptcraft/src/main.rs:1886-1890). Its scroll area and editor response are clipped to the central/content geometry, not the toolbar row.
- Native full-size content is enabled by [`native_options()`](../../../../apps/scriptcraft/src/main.rs:1899-1909), so the native content inset can affect the panel's actual placement/clip boundary even though the app frame has zero egui inset.

### Finding and hypotheses

V1 eliminated the former adjacent title/toolbar ownership seam in deterministic egui geometry: title `(12,4)-(812,34)`, toolbar `(12,34)-(812,86)`, panel `(12,4)-(812,86)`, zero seam gap, zero frame stroke, and zero frame margins. Therefore a black bar at the toolbar bottom is not currently explained by an intentional egui stroke or a second panel separator in the source. The leading hypotheses are:

1. **Uncovered/incorrectly filled region:** the observed black pixels are below the toolbar row but inside a native/content or panel allocation region that is not covered by the expected `weak_bg_fill`, possibly due to actual TopBottomPanel height/content height divergence or clip rounding.
2. **Native/full-size-content inset boundary:** the native titlebar/full-size-content configuration exposes a bottom edge/inset or compositor region outside the app-owned frame; deterministic egui rectangles cannot prove its pixel ownership.
3. **Fill mismatch:** the app surface uses `open.weak_bg_fill`, while the adjacent central/native surface uses a different fill, making a thin strip appear black even without a stroke. This is a color ownership mismatch, not a frame line.
4. **Geometry rounding/padding:** the 2-point bottom padding or interaction-height calculation leaves a narrow residual at a device-pixel boundary; this must be compared in logical points and physical pixels.

The source does **not** currently support claiming that the bar is a real stroke: `Stroke::NONE`, disabled separator, and zero-inset frame are explicit. It also does not prove that it is uncovered; native pixel capture is absent.

### Measurable fix direction

Before changing source, log the panel rect, frame outer/inner rect, title rect, toolbar row rect, toolbar content rect, clip rect, central-panel top rect, actual bottom padding rect, logical-to-physical scale, and sampled pixel rows at the toolbar bottom. The pass condition should identify the owner of every row: app-chrome fill through the exact panel bottom, or a deliberately documented boundary with matching fill. If the gap is app-owned, make the smallest allocation/fill correction so the shared frame covers the full intended chrome rect and retain title hit coverage, toolbar row budget, and the `128.0` Elements width. If it is native/compositor-owned, do not paint over it blindly; compare full-size-content/titlebar variants in a native-capable experiment and choose a supported configuration only if it preserves seamless title/toolbar behavior.

### Options and risks

- Extend/verify the app-chrome fill to the measured panel bottom — preferred if the gap is uncovered app geometry; risk of hiding a content inset or changing central-panel placement.
- Reconcile toolbar bottom padding/height to the measured row — appropriate only if logical geometry leaves the bar; risk of changing existing vertical contract.
- Match adjacent app/central fill — appropriate for a color mismatch; risk of masking native ownership.
- Change full-size-content/titlebar configuration — only if native evidence identifies the compositor inset; risk of losing custom title hit coverage or macOS presentation behavior.

## Cross-cutting outcome, validation boundary, and future decomposition

## Candidate approach assessment — supplied zoom-artifact proposal

This subsection records the supplied proposal as a **candidate hypothesis/approach**, not as a selected design or implementation instruction. The installed dependency set is eframe/egui/egui-winit `0.29.1` with winit `0.30.13` (the application declares eframe `0.29` in [`apps/scriptcraft/Cargo.toml`](../../../../apps/scriptcraft/Cargo.toml:8-11), and the lockfile resolves the versions). The assessment below is based on the installed source and the current v2 Investigation A evidence.

### 1. Locking scale during an active resize or viewport transition

- **API correction — rejected assumption:** egui `0.29.1` does **not** expose `ctx.input(|i| i.viewport().has_requested_resize)`, nor did the installed egui, eframe, egui-winit, or winit sources expose an equivalent active-resize/transition-progress flag. [`ViewportInfo`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/data/input.rs:186-237) contains optional native scale, monitor size, inner/outer rectangles, minimized/maximized/fullscreen state, focus, and close events, but no requested-resize field. This corrects the proposal's implication that a transition flag can directly delimit a freeze interval.
- **Requested versus actual state:** [`ViewportCommand::Maximized(bool)`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/viewport.rs:937-1007) is an output request. The backend applies it with winit's `set_maximized`, while [`egui_winit::update_viewport_info()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:943-993) samples native rectangles and scale. That distinction supports Investigation A's conclusion: observed rectangles and `maximized` are snapshots, not an acknowledgement or native animation timeline. On macOS, the installed source also intentionally avoids refreshing maximized/minimized state after initialization to avoid a deadlock ([`update_viewport_info()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:987-993)); this makes the proposed flag-based transition detector even less portable.
- **Per-frame zoom feasibility:** egui does expose [`Context::set_zoom_factor()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/context.rs:1924-1947) and [`Context::set_pixels_per_point()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/context.rs:1885-1900), but these are global egui UI-scale controls, not a paint-only freeze. They become active at the next pass; `pixels_per_point` is recomputed as zoom factor × native pixels-per-point ([`Context::run()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/context.rs:465-493)). Repeatedly overriding it during a resize would therefore alter toolbar/layout coordinates, pointer conversion, widget hit testing, IME geometry, and platform output—not only the screenplay page.
- **Font/accessibility/native-scale risk:** egui maintains font instances keyed by `pixels_per_point` and begins each pass with that scale ([`update_fonts_mut()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/context.rs:566-605). Its AccessKit root transform also uses the current scale ([`Context::begin_pass()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/context.rs:550-563)), while egui-winit converts pointer/wheel/touch/IME coordinates using the current native scale ([`State::take_egui_input()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:230-253), [`State::on_window_event()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:256-285)). A per-frame override may be internally supported as ordinary GUI zoom, but it is not evidence of a safe transition lock and could desynchronize input, font-atlas selection, accessibility transforms, or a real `ScaleFactorChanged` event.
- **Assessment:** **Supported:** investigate a last-stable presentation metric or page/render transform, and measure native rectangles, native scale, egui available width, and renderer font size per frame. **Uncertain:** whether a narrowly scoped app-owned freeze of `presentation_metrics()` can reduce the observed artifact without impairing responsive layout; this needs a native experiment. **Rejected:** treating `has_requested_resize` as an available API, treating `set_pixels_per_point()` as a page-only freeze, or promising that zoom override controls native animation progress. The proposal should first test a presentation-scale freeze independent of egui global zoom; global zoom may be a comparison arm only.

### 2. Replacing spacer arithmetic with egui elasticity

- **Current contract:** [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:388-471) first measures all non-flexible items, reserves row padding/gaps, and then either distributes the exact remaining width across the two typed `FlexibleSpace` items or performs a left-to-right fit/overflow pass with a fixed [`TOOLBAR_OVERFLOW_WIDTH`](../../../../apps/scriptcraft/src/main.rs:27-32). [`TOOLBAR_ELEMENTS_WIDTH`](../../../../apps/scriptcraft/src/main.rs:26-28) is a fixed 128-point override. Rendering then consumes those widths in order in [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:809-860), and [`toolbar_actual_available_width()`](../../../../apps/scriptcraft/src/main.rs:238-245) clips the authority to the actual visible edge. The tests explicitly cover narrow leftmost-fit overflow and right-edge containment ([`toolbar_allocation_narrow_keeps_leftmost_items_and_far_right_menu()`](../../../../apps/scriptcraft/src/main.rs:2300-2313), [`toolbar_shared_row_edge_contains_final_group_at_actual_content_edge()`](../../../../apps/scriptcraft/src/main.rs:2359-2375)).
- **What egui actually provides:** [`Ui::available_width()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/ui.rs:974-984), `add_space`, `allocate_space`, and layout justification are allocation primitives, not a named spring/flexible-space widget. [`Layout::centered_and_justified()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/layout.rs:235-249) is explicitly for a single widget filling a layout; it does not express two independently weighted sequential springs around a fixed middle item. “Springs” is therefore a metaphor unless a future helper is implemented by the application.
- **Comparison:** A sequential elastic rewrite could measure fixed children, use remaining `available_width()`, and allocate two application-owned elastic regions. That would reproduce the current arithmetic, not remove the allocation contract. Letting ordinary sequential widgets consume `available_width()` or relying on justification would lose the explicit distinction between fixed 128-point Elements, flexible slots, overflow reservation, and the final clipped edge. It also risks allowing an intrinsic ComboBox minimum or later child to expand the row differently from the fit calculation.
- **Assessment:** **Supported as an experiment:** express only the render-time flexible gaps through a small application-owned elastic abstraction, while feeding it the already-decided `ToolbarAllocation`. **Uncertain:** whether an egui-layout formulation can preserve exact widths and hit rectangles across all boundaries; compare geometry ledgers rather than visual preference. **Rejected as a wholesale replacement:** sequential elasticity that becomes the overflow allocator or makes `available_width()` the sole policy authority. The 128-point Elements override and deterministic right-edge containment must remain explicit either way.

### 3. Preserve overflow logic while refactoring spacing

The safe separation is already visible in source: allocation-time policy is [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:388-471), [`toolbar_item_width()`](../../../../apps/scriptcraft/src/main.rs:195-209), [`toolbar_content_width()`](../../../../apps/scriptcraft/src/main.rs:226-232), and [`toolbar_actual_available_width()`](../../../../apps/scriptcraft/src/main.rs:238-245). These functions decide measured/fixed widths, flexible-count distribution, overflow-menu reservation, left-to-right stopping, and row width. Render-time spacing is the ordered `add_space`/`allocate_ui_with_layout` sequence in [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:820-858), including row padding, inter-item gaps, and the already allocated width for each item.

The proposal is therefore **supported only with a narrow experiment boundary**: retain `ToolbarAllocation` and all fit/overflow tests unchanged, then substitute an elastic renderer for flexible slots in the wide/no-overflow branch. The experiment must compare per-item rectangles, row width, Elements width, final right edge, and narrow overflow membership against the current ledger. It must not move overflow decisions into egui's sequential layout, recompute the overflow reservation after rendering, or permit a render-time widget minimum to retroactively change visibility. If the elastic renderer cannot reproduce the allocation ledger exactly, retain the typed allocator; that is a contract-preservation result, not a failed research process.

### 4. Proxy window commands

The supplied [`ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(...))`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/context.rs:3520-3533) spelling is materially equivalent to the current [`ctx.send_viewport_cmd(command)`](../../../../apps/scriptcraft/src/main.rs:1823-1828). The current code already computes the direction from optional `input.viewport().maximized` through [`title_strip_viewport_command()`](../../../../apps/scriptcraft/src/main.rs:1783-1804); writing the call through `ui.ctx()` would only use the `Ui`'s context accessor. It does not proxy, queue, acknowledge, or smooth the native operation differently. The installed egui-winit backend maps `Maximized` directly to [`window.set_maximized(v)`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:1428-1438).

**Assessment:** **Supported:** use either spelling if a future local refactor improves ownership/readability. **Rejected:** treating the proxy spelling as a transition fix or as a control over native animation smoothness, duration, interpolation, restore-frame selection, or scale-factor changes. Those remain OS/backend behavior and require the native measurement protocol already specified in Investigation A.

### Recommended experiment order and measurements

1. **First, establish the baseline without changing behavior:** instrument a window-capable macOS run with command timestamp, optional `ViewportInfo` rectangles/state, `native_pixels_per_point`, egui `pixels_per_point`, content/clip/available widths, `presentation_metrics()` page width/scale/font size, toolbar allocation, each rendered toolbar rect, and representative page/toolbar raster bounds. Include `ScaleFactorChanged` events and display identity where available. This validates the separation between native transition progress, egui input scale, width-derived page scale, and rasterization.
2. **Second, test scale policy in the smallest surface:** compare current width-derived metrics with a last-stable page/presentation-scale freeze during the measured interval. Do not change global egui zoom in this experiment. Validate pointer coordinates, keyboard/IME behavior, font appearance, AccessKit transform behavior if enabled, native scale-factor changes, and post-transition recovery. If no reliable interval can be identified from measurements, classify the freeze as unsupported rather than infer one from a nonexistent resize flag.
3. **Third, run the allocation-preserving elasticity arm:** keep [`ToolbarAllocation`](../../../../apps/scriptcraft/src/main.rs:183-189) and overflow decisions authoritative; vary only rendering of already allocated flexible widths. Compare exact-fit, wide, narrow, and clip-constrained widths, with special assertions for 128-point Elements, leftmost overflow order, overflow width, and final right-edge containment.
4. **Last, compare command spellings only if useful for traceability:** issue the same maximize request through the existing title-strip path and record that no native animation metric changes. Do not spend an experiment on proxy syntax unless source ownership itself is under review.

The candidate is thus **partially supported**: it suggests useful measurement and a possible render-only elasticity experiment, but its resize-flag premise, global zoom-lock interpretation, spring terminology, and proxy-command effect are invalid or overstated. This subsection changes no source, requirements, blueprints, or v1 artifact. The user's explicit 2026-10-27 request later authorizes conversion of this research into the four v2 work orders below; it does not authorize implementation, design review, human approval, or validation.

### Validation boundary

This research iteration is complete when the v2 document contains source/API evidence for all three paths, identifies proven versus hypothesized causes, records exact branch/HEAD evidence, preserves the 128-point baseline, and defines native measurements that can distinguish them. It is **not** complete by a deterministic unit-test pass alone and does not claim native smoke. A future implementation iteration must separately validate: command/state/rectangles, transition metrics, cursor owner at every region, and toolbar-bottom pixels/rectangles.

### Final v2 work-order decomposition

These are the final work orders created from the research decomposition. They are ordered sequentially so each later boundary consumes the preceding evidence without overwriting closed v1 artifacts:

1. **[`V2-WO-001-transition-metric-native-contract.md`](work-orders/V2-WO-001-transition-metric-native-contract.md:1)** — first, to establish native/egui metric instrumentation and preserve the known-state viewport request contract.
2. **[`V2-WO-002-elements-cursor-ownership.md`](work-orders/V2-WO-002-elements-cursor-ownership.md:1)** — second, to isolate the smallest response-tree ownership correction while preserving stock ComboBox behavior.
3. **[`V2-WO-003-toolbar-bottom-surface-ownership.md`](work-orders/V2-WO-003-toolbar-bottom-surface-ownership.md:1)** — third, to assign the black region using logical/physical rectangle evidence without conflating it with Elements behavior.
4. **[`V2-WO-004-integrated-native-validation-and-document-reconciliation.md`](work-orders/V2-WO-004-integrated-native-validation-and-document-reconciliation.md:1)** — last, to consume all three independently reviewable evidence packages and make any final evidenced correction/reconciliation.

### Durable-document impact hypotheses

- **Requirements:** likely `verify unchanged`; existing [`AC-WRITING-005.5`](../../requirements/features/screenplay-writing-loop.md:115-130), [`AC-WRITING-005.6`](../../requirements/features/screenplay-writing-loop.md:131-136), and [`AC-WRITING-005.7`](../../requirements/features/screenplay-writing-loop.md:137-140) describe toolbar geometry, bounded viewport behavior, and cursor ownership. Update only if evidence establishes a stronger or materially different user-facing contract.
- **Blueprint:** likely `verify unchanged` for the planned implementation iteration; A future implementation may update the desktop blueprint only for a confirmed cursor-owner correction, surface-owner correction, or newly supported native limitation; no documentation delta is authorized by this draft.
- **No source/document implementation claim:** v1's unified surface and closed-only centering wording remain historical/current baseline evidence, not a reason to claim the three v2 observations are solved.

### Native-smoke limitations

The prior run confirmed macOS but had no usable display: no visible window, click/double-click trace, native outer/inner rectangles, compositor pixels, transition frames, popup/cursor observation, or keyboard smoke. This v2 research therefore makes no native observation claim. The work orders retain this limitation and require a usable window-capable run before any native result is claimed. A usable window-capable macOS run is required for the final distinctions in all three investigations; absence of that run is a confidence limitation, not implementation approval or a reason to infer the result.

## Unresolved decisions

- Whether scale stability should mean stable page/font metrics only at stable widths, a frozen display scale during native transitions, or a fixed presentation scale policy.
- Whether any macOS-native integration beyond eframe 0.29 is acceptable if the product requires transition-progress control.
- Which exact Elements child response wins the cursor after native observation: decorative label, stock ComboBox path, parent group ordering, or document overlap.
- Whether the black bar is app-owned fill/geometry, a color mismatch, device-pixel rounding, or native compositor/full-size-content space.
- Whether any confirmed v2 fix changes the durable desktop blueprint or merely verifies its existing bounded contract.
- Whether v2 should proceed as three sequential implementation work orders or split the native transition research into a separate initiative if it becomes platform-specific.

## Research conclusion

V1 correctly bounded maximize direction, unified the egui chrome surface, centered closed Elements text, and recorded deterministic evidence, but its unavailable native smoke left three runtime questions open. V2's strongest current conclusion is: native maximize remains a request with OS-owned resizing; width-derived `presentation_metrics()` can legitimately change during resize and cannot be stabilized by safe portable desktop-dimension polling; the closed Elements galley is not the I-beam owner, so response-tree ownership must be corrected/measured; and the black toolbar bar is not explained by a deliberate stroke/separator and must be assigned through rectangle plus pixel evidence before a fill/height or native-inset fix is selected. No implementation, approval, or native success is claimed.

## Planning transition record

- **Request:** On 2026-10-27, the user explicitly requested conversion of the v2 research into executable, independently reviewable work orders.
- **Transition:** `draft → plan`; four ordered `V2-WO-*` files were created, with matching future validation filenames referenced by each work order.
- **Pending gates:** none. Design review passed with warnings, human design approval was recorded, all four work orders completed fresh validation, and native-display limitations remain documented non-blocking warnings.

## User decision

On 2026-10-27, the user explicitly requested that the v2 research be converted into executable, independently reviewable work orders. This moved v2 from `draft` to `plan`; design review passed with warnings, human approval was recorded, and all four v2 work orders completed their fresh validation loops.

## Closeout record

- **Closeout status:** `pending final commit and merge`
- **Validation outcome:** All four v2 work orders passed fresh validation. Native display/window evidence remains unavailable and explicitly unclaimed; project policy defines no required native/manual gate.
- **Follow-up:** Repeat usable-display macOS smoke for transition frames, native rectangles, Elements cursor/popup behavior, and toolbar-bottom pixel ownership.
- **Closeout date:** `2026-10-27`
- **Closeout evidence:** Implementation commit `44f452a` was created on `dev/2026-10-27-titlebar-transition-and-elements-alignment-v2`; merge commit `2ab1eac` merged it into `master`. The final checkout is clean on `master`.

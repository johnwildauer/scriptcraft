# UI Icons v2 — Percentage stability, right-edge investigation, and Diagnostics gate

## Metadata and evidence boundary

- **Status:** V2-WO-001 through V2-WO-004 implementation submitted on 2026-11-05; user review and validation remain pending. V2-WO-004 consumed both supplied geometry traces and removed temporary production tracing.
- **Created:** 2026-11-05.
- **Branch context:** `dev/2026-11-04-ui-icons`. This research did not create, switch, stage, commit, merge, or otherwise modify a branch.
- **Scope:** Exactly three follow-up reports: digit-dependent percentage allocation, the reported additional roughly 90px at the right edge, and a development/production gate for status Diagnostics.
- **Evidence:** Targeted reading of current application source, the installed egui 0.34.3 source/documentation, [`ui-icons-v1.md`](ui-icons-v1.md), [`WO-007`](work-orders/WO-007-status-zoom-knob-paint-priority-and-clipping.md), [`WO-008`](work-orders/WO-008-status-right-edge-measurement-and-alignment.md), and the authoritative [`project-manifest.md`](../../.forge/project-manifest.md:73). The baseline values below come from the user-supplied [`W0-002 grep report`](work-orders-v2/W0-002-grep-report.txt), not agent validation. V2-WO-002 source/document updates are implementation evidence only; no validation was run.
- **Review and validation owner:** User. No builds, checks, tests, lint, formatting checks, manual verification, runtime inspection, agent reviews, or other validation were run or claimed.
- **Change boundary:** V2-WO-002 updated the percentage-slot implementation and its scoped documents; V2-WO-004 updated only the measured-owner requirements/context/initiative/handoff and status-bar/main-panel source required for the slider sizing correction and removal of temporary tracing. Both user evidence files were preserved unchanged. No v1 work order, dependency, manifest, or unrelated work was changed by these work orders; no Git staging or commit was performed.
- **Runtime caveat:** The user-supplied baseline at egui content width 1512 records planned slider width 165 versus actual 100, planned percentage right 1500 versus actual 1435, and galley advance right 1427. The post-slot user trace at content width 800 records matching planned/actual percentage rectangles `[746.78..788.00]`, but the slider still ends at 677.78 rather than its planned 742.78. The measured 65-point slider shortfall explains the percentage's prior 65-point early edge; combined with the intended 12-point panel inset and 8-point centered-galley reservation, that produced the reported 85-point content-edge-to-galley gap at 2x. After the slot correction, only the intended 20-point distance to the centered galley remains in the supplied post-slot frame. These are user-observed traces, not agent validation or post-fix runtime evidence.

## Executive findings

1. **The digit-count allocation defect is deterministic in current source.** [`show_impl_with_appearance()`](../../apps/scriptcraft/src/status_bar.rs:1248) measures the currently displayed label at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1286), and [`measured_button_width()`](../../apps/scriptcraft/src/status_bar.rs:1205) returns that current galley advance plus two side reservations. Consequently the percentage slot and total fixed-right budget change when the label gains a digit. [`plan_status_row()`](../../apps/scriptcraft/src/status_bar.rs:223) then recomputes slider growth, the right group's left edge, and the residual center region at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:248). WO-008 made the trailing advance edge stable, but did not make allocation or group geometry stable.
2. **Recommended percentage design:** use a stable, style-measured reference slot based on the maximum `300%` label plus the two existing reservations, render the actual current galley centered in that slot, and make the planner's explicit percentage rectangle authoritative through painting. This is preferable to a hand-picked numeric constant because it follows the active status font and existing manual reservation. It stabilizes the whole right group across zoom values while retaining bounded narrow-width compression.
3. **The nominal post-WO-008 trailing formula remains 12 + 8 = 20 logical points, not 90px.** Current source contains no fixed trailing 90-point or 90-pixel allocation. At 2 physical pixels per point, the nominal 20-point advance-edge distance is 40 physical pixels before normal ink/bearing and rasterization differences. The reported 90px must therefore be investigated using actual rectangles, edge references, and scale rather than explained away by the nominal formula.
4. **The supplied traces identify the owner; the percentage right edge is already corrected.** After V2-WO-002, planned and actual percentage rectangles match horizontally. The first remaining mismatch is the stock slider response, whose actual width is 100 rather than the planned 165 points. The same 65-point deficit had previously pulled the percentage's actual right edge left by 65 points; it was not a separate panel or percentage margin. V2-WO-004 sets egui's scoped slider-width setting from the planner width, without changing any presentation value or adding a compensation constant. No post-fix trace is claimed.
5. **Diagnostics was previously always enabled and projected before the panel rendered.** V2-WO-003 now adds a named manual production-off boolean in the existing status presentation module and gates both the disclosure row and construction/formatting of its detailed presentation projection. Operational counters remain intact. The manual source choice is independent of build profile and requires rebuilding/relaunching when changed.

---

## 1. Stable percentage allocation with centered text

### 1.1 Current measurement and painting path after WO-008

The current same-frame path is:

1. [`show_impl_with_appearance()`](../../apps/scriptcraft/src/status_bar.rs:1248) derives the row from [`available_rect_before_wrap()`](../../apps/scriptcraft/src/status_bar.rs:1271) intersected with the inherited clip and constructs a fixed-height row at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1272).
2. The actual display label is created once at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1286), then measured immediately at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1287). Capturing one label prevents measurement/display text drift within the frame, but intentionally preserves value-dependent width.
3. [`measured_button_width()`](../../apps/scriptcraft/src/status_bar.rs:1205) measures that exact current string with the status Button font and adds [`PERCENTAGE_SIDE_RESERVATION`](../../apps/scriptcraft/src/status_bar.rs:31) twice at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1215).
4. [`plan_status_row()`](../../apps/scriptcraft/src/status_bar.rs:223) includes that width in the fixed-right sum at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:248), computes compression and slider growth at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:252), and anchors the percentage allocation to the row's right edge at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:282).
5. The right group begins at the Timer allocation's left edge and ends at the row edge at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1299). The zoom subgroup width is slider + percentage + one gap at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1313).
6. [`zoom_controls()`](../../apps/scriptcraft/src/status_bar.rs:333) places the slider and then the percentage in a nested horizontal UI at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:342). [`zoom_presets_button()`](../../apps/scriptcraft/src/status_bar.rs:451) allocates the supplied width at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:458).
7. The actual current galley is built at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:470). WO-008 right-aligns its advance edge using the trailing reservation at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:478), vertically centers it, and clips painting to the percentage allocation at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:480).

### 1.2 Why the group still moves

Let `G(z)` be the current label's advance width at zoom value `z`, and let `R` be the existing side reservation. The current nominal percentage width is:

`P(z) = G(z) + 2R`.

For a two-digit value and a three-digit value, normally `G(100) ≠ G(90)`, so `P(100) ≠ P(90)`. This changes all of the following even though the percentage's right edge stays anchored:

- the fixed-right sum at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:248);
- compression near narrow widths at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:252);
- leftover width available for slider growth at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:266);
- total right-group width and center-message residue at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:271);
- the left positions of Timer, Focus Mode, Appearance, and the slider at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:282).

WO-008 therefore solved **trailing advance-edge alignment**, not **stable slot/group geometry**. Right alignment also means shorter labels have a different left edge within their allocation. This is expected from the current contract, not a new runtime claim.

### 1.3 Explicit numeric constant versus measured reference slot

| Approach | Benefits | Failure modes | Assessment |
|---|---|---|---|
| Hand-written fixed width in logical points | Simple and stable across values | Silently becomes wrong when the Button font, font family, side reservation, zoom range, or platform font metrics change; invites another unexplained magic number | Rejected as the primary design |
| Measure maximum reference `300%` with the active status font, then add both existing reservations | Stable across current values; follows manual typography and reservation settings; uses the same measurement pipeline as painting; no new width number to tune | Needs bounded behavior when compressed; assumes the designated maximum reference is the widest product label; visible trailing distance varies when text is centered | **Selected** |

The selected helper should derive its reference from [`MAX_ZOOM_PERCENT`](../../apps/scriptcraft/src/status_bar.rs:14), rather than duplicate a separate numeric range, and measure through the same [`status_font_id()`](../../apps/scriptcraft/src/status_bar.rs:69) path used by the current percentage. The `300%` reference is the product's designated maximum three-character numeric label. User review should include `100%` and every preset so proportional-font assumptions are not silently treated as universal. If a future font makes another permitted label wider, the reference policy must be revisited rather than allowing overflow.

No additional fixed numeric width is needed. Existing manual values, including [`BUTTON_TEXT_SIZE`](../../apps/scriptcraft/src/status_bar.rs:35) and [`PERCENTAGE_SIDE_RESERVATION`](../../apps/scriptcraft/src/status_bar.rs:31), remain unchanged and feed the measured slot.

### 1.4 The egui equivalent of a fixed-size container

This design does **not** require HTML-like nested elements or CSS. In egui, the equivalent is an explicitly allocated screen-space rectangle, optionally backed by a child UI, followed by manual placement of the galley within that rectangle:

- [`StatusRowLayout`](../../apps/scriptcraft/src/status_bar.rs:209) already owns an explicit percentage rectangle.
- [`plan_status_row()`](../../apps/scriptcraft/src/status_bar.rs:223) already computes that rectangle at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:283).
- [`one_line_region_at()`](../../apps/scriptcraft/src/status_bar.rs:1069) demonstrates the existing application pattern for a child UI with an explicit maximum rectangle and intersected clip at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1074).
- The actual galley can then be centered arithmetically and painted with the rectangle as its clip, as the current control already does for vertical centering and clipping at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:480).

For the follow-up implementation, the planned percentage rectangle should be authoritative end-to-end. Avoid reconstructing a second effective percentage position solely from nested sequential widths in [`zoom_controls()`](../../apps/scriptcraft/src/status_bar.rs:333). A scoped child at the planned rectangle, or an equivalent exact allocation against that rectangle, makes allocation, interaction, clipping, accessibility bounds, and galley placement share one geometry authority.

### 1.5 Centering, trailing alignment, and stable-group tradeoff

For a stable nominal rectangle `P` and current galley advance `G(z)`:

`text_left = P.center_x - G(z) / 2`, when the galley fits.

This produces stable allocation and a stable text center. It deliberately replaces WO-008's right-aligned galley at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:479).

The two contracts cannot both hold for variable-width text:

- **Right alignment** keeps the galley's trailing advance edge fixed but moves its left edge and does not stabilize current-label allocation.
- **Centering in a fixed slot** keeps allocation and text center fixed but makes the unused left/right space vary with the current galley width.

The selected priority is stable group geometry and centered text, as requested. The existing side reservations remain part of the slot width; they are not an additional right-alignment offset after centering. Visible glyph-ink bearings remain separate from advance geometry.

### 1.6 Interaction, popup, accessibility, clipping, and narrow widths

- **Interaction:** The response should continue to cover the full explicit percentage rectangle through the existing interaction path at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:460). A stable nominal rectangle also gives the closed control a stable popup anchor across percentage changes.
- **Popup:** Preserve the existing stable ID, toggle routing, exact presets, minimum menu width, and dismissal behavior at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:485). Centering the closed galley must not restyle or reposition menu entries except through the preserved response anchor.
- **Accessibility:** Preserve the full accessible current-value name at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:467), even if visible text clips. Keyboard activation remains in [`interactive_response()`](../../apps/scriptcraft/src/status_bar.rs:563). Do not substitute the reference `300%` string for the actual accessible value.
- **Narrow widths:** Keep the existing proportional compression in [`plan_status_row()`](../../apps/scriptcraft/src/status_bar.rs:252). When the current galley fits, center it. When it does not fit, use a bounded origin and the explicit allocation clip; do not expand the rectangle, overlap the slider, or create a second overflow representation. The current full value remains available to accessibility and the presets remain available through the control where physically usable.
- **Clipping:** Clip to the actual explicit percentage rectangle intersected with the inherited clip. Keep hover fill and interaction bounds on the same geometry. Physically unusable widths retain the existing no-usability-promise boundary.
- **Right edge:** Centering changes the galley-level trailing space by design; it must not change the nominal slot's right edge or introduce a new panel-level inset.
- **WO-007:** Preserve the deferred knob paint and its right-group/row/parent clip in [`paint_zoom_slider_knob()`](../../apps/scriptcraft/src/status_bar.rs:391) and its final invocation at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1394).

### 1.7 Exact proposed setting and code locations

These are proposal locations only, not edits made by this research:

1. Keep all numeric presentation settings in [`presentation`](../../apps/scriptcraft/src/status_bar.rs:19), especially the existing reservation at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:31), unchanged.
2. Replace or generalize [`measured_button_width()`](../../apps/scriptcraft/src/status_bar.rs:1205) with a clearly named stable-slot measurement helper. It should measure the reference derived from [`MAX_ZOOM_PERCENT`](../../apps/scriptcraft/src/status_bar.rs:14), using [`status_font_id()`](../../apps/scriptcraft/src/status_bar.rs:69), then add the existing two reservations.
3. At [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1286), continue creating the actual current label for paint/accessibility, but pass the stable reference-slot width to [`plan_status_row()`](../../apps/scriptcraft/src/status_bar.rs:1290).
4. Make the explicit rectangle produced at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:283) authoritative in [`zoom_presets_button()`](../../apps/scriptcraft/src/status_bar.rs:451), replacing the WO-008 right-alignment calculation at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:478) with fit-aware centered galley placement.
5. Retain the planner's current slider bounds at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1296), all manually adjusted values, and the right-group anchoring at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1299).

---

## 2. Reported additional roughly 90px at the right edge

### 2.1 Complete layer trace

The following is a source-derived geometry trace, not a runtime observation.

| Layer | Current behavior and exact source | Right-edge consequence |
|---|---|---|
| Native viewport | The application requests native decorations, an opaque window, visible title bar/buttons, and no full-size content view in [`native_options()`](../../apps/scriptcraft/src/main.rs:2786). The bottom panel is created before the central panel in [`show_application_panels()`](../../apps/scriptcraft/src/main.rs:2484). | The relevant app formula ends at egui's content edge. A screenshot measurement to the native **outer** window edge needs a separate native content-to-outer accounting. |
| Bottom panel | [`TopBottomPanel::bottom`](../../apps/scriptcraft/src/main.rs:2538) uses [`status_panel_frame()`](../../apps/scriptcraft/src/status_bar.rs:59). The panel begins from available screen space and clips its panel UI to the panel rectangle in installed [`panel.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/containers/panel.rs:667). | No app-owned right sidebar is present in this path. Panel allocation and central-panel ordering do not add a right margin. |
| Frame | The app retains [`Frame::side_top_panel()`](../../apps/scriptcraft/src/status_bar.rs:63) but replaces inner margins with horizontal 12 and vertical 2. Installed frame code subtracts total margin from the content rectangle in [`frame.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/containers/frame.rs:392). The base frame has zero outer margin and no stroke in [`frame.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/containers/frame.rs:160). | One 12-point right inner inset. No second app-owned frame inset, outer margin, or stroke width is added. |
| Available rectangle and clip | [`show_impl_with_appearance()`](../../apps/scriptcraft/src/status_bar.rs:1248) intersects [`available_rect_before_wrap()`](../../apps/scriptcraft/src/status_bar.rs:1271) with the inherited clip. Installed UI code delegates this rectangle to the placer in [`ui.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/ui.rs:1000), and the placer rounds it in [`placer.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/placer.rs:90). | The row cannot intentionally extend right of either available content or clip. A narrower clip can increase **visible-ink** distance by clipping text; it does not explain an extra allocated right margin without runtime rectangles. |
| Main row | [`one_line_region()`](../../apps/scriptcraft/src/status_bar.rs:1053) allocates the full planned row size with left-to-right centered layout and intersects its clip with its maximum rectangle at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1058). Installed allocation computes a child rectangle, then allocates the child's actual minimum rectangle in the parent in [`ui.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/ui.rs:1483). | Planned row and actual child rectangle should be compared at runtime rather than assumed identical in the visual investigation. |
| Explicit right region | [`right_rect`](../../apps/scriptcraft/src/status_bar.rs:1300) ends exactly at the planned row's right edge. [`one_line_region_at()`](../../apps/scriptcraft/src/status_bar.rs:1069) creates a scoped UI at that explicit rectangle and intersects its clip at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1074). | No explicit trailing blank region is inserted after `right_rect`. |
| Right-to-left layout | The right UI sets the planned item gap and selects right-to-left layout at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1309). Installed layout places the next right-to-left frame from the available right edge in [`layout.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/layout.rs:582). | The zoom subgroup is the first child and is therefore intended to touch the right region's right edge. Later icon children consume space to its left. |
| Zoom subgroup | The subgroup's requested width is slider + percentage + gap at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1313), and [`one_line_region()`](../../apps/scriptcraft/src/status_bar.rs:1316) gives it that explicit size. | Its width affects only its left edge; there is no budgeted trailing term after the percentage. |
| Nested horizontal UI | [`zoom_controls()`](../../apps/scriptcraft/src/status_bar.rs:333) creates a horizontal child at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:342). Installed [`Ui::horizontal()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/ui.rs:2562) chooses initial available width and left-to-right layout when the parent is not right-to-left at [`ui.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/ui.rs:2618). | Actual horizontal-child and slider/percentage rectangles are important ledger entries because allocated child minima, not only requested planner rectangles, ultimately contain painted controls. |
| Slider allocation and growth | The slider uses its supplied width and interaction height in [`zoom_slider()`](../../apps/scriptcraft/src/status_bar.rs:353). Planner growth is computed before right-edge placement at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:266). | Slider width changes move the slider and group left edge, not the percentage's nominal right edge. The custom knob is painted separately and does not determine percentage text placement. |
| Item spacing | The planned gap is applied at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1309). Installed layout advances the cursor by widget edge plus item spacing in [`layout.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/layout.rs:751). | One gap belongs between slider and percentage. A cursor advanced beyond the percentage after allocation is not a paint offset and does not move the already allocated galley. |
| Percentage allocation | [`zoom_presets_button()`](../../apps/scriptcraft/src/status_bar.rs:451) allocates the supplied width at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:458), intersects hover and clip rectangles at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:459), and uses that rectangle for interaction and painting. | This **actual** rectangle, not only [`StatusRowLayout::percentage`](../../apps/scriptcraft/src/status_bar.rs:215), must be captured to close the investigation. |
| Galley placement | The trailing inset is clamped to rectangle width and the galley origin is computed at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:478). Painting uses the rectangle clip at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:481). | In an uncompressed fitting frame, galley advance right = actual percentage rectangle right − 8. Narrow clamping is a separate branch. |
| Physical scale | Installed [`Context::pixels_per_point()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/context.rs:2238) is UI zoom × native scale. The app computes a native-scale-normalizing factor in [`ui_zoom_factor()`](../../apps/scriptcraft/src/status_bar.rs:309) and applies it in [`raw_input_hook()`](../../apps/scriptcraft/src/main.rs:2627). | Coordinates are logical points. A physical-pixel measurement must use the actual effective scale and any screenshot/output scaling, not assume 1px per point. |

### 2.2 Complete current formula

Define the following in one egui coordinate space:

- `S` = native egui content right edge;
- `F` = [`PANEL_HORIZONTAL_INSET`](../../apps/scriptcraft/src/status_bar.rs:23), currently 12;
- `R` = [`PERCENTAGE_SIDE_RESERVATION`](../../apps/scriptcraft/src/status_bar.rs:31), currently 8;
- `C` = inherited clip right edge;
- `A` = actual percentage allocation right edge;
- `G` = current galley advance width;
- `L` = actual percentage allocation left edge;
- `B` = any additional visible-ink trailing bearing/raster difference relative to the galley advance edge;
- `p` = physical pixels per logical point.

For the ordinary uncompressed, fitting path:

`panel_content_right = S − F`

`row_right = min(panel_content_right, C)`

`planned_right_rect_right = row_right`

`planned_percentage_right = row_right`

`galley_advance_right = A − R`

If actual allocation matches the planner and the clip does not further cut the text:

`galley_advance_right = S − F − R`

Therefore:

`content_edge_to_galley_advance = F + R = 12 + 8 = 20 logical points`

and:

`content_edge_to_galley_advance_physical = 20 × p physical pixels`.

For visible ink rather than the advance edge:

`content_edge_to_visible_ink_physical ≈ (20 + B) × p`.

If the user's edge reference is the native **outer** window rather than egui content, add the independently measured native content-to-outer physical distance. If the screenshot was resized, apply that output scale separately.

The fully clamped source formula, before considering clipping of the mesh itself, is:

`galley_advance_right = max(L + G, A − min(R, A − L))`.

That narrow branch is not evidence of a fixed extra margin at ordinary widths.

### 2.3 What current source does and does not explain

Deterministic source findings:

- The current nominal advance-edge inset is 20 logical points.
- The existing four inter-item gaps are to the **left** of the percentage; none is a trailing gap after it.
- Slider minimum/growth, page allocation, center residue, and current digit count can change left-side geometry but do not add a trailing constant to the WO-008 galley formula.
- There is no literal 90, fixed trailing spacer, additional outer frame margin, or right-aligned extra column in the current status path.
- At `p = 2`, nominal advance-edge distance is 40 physical pixels. At `p = 1`, it is 20 physical pixels.
- Font bearings can distinguish ink from advance edge, but the 10-point status font at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:35) does not provide source evidence for a fixed additional 50–70 physical pixels by itself.

**Measured-owner resolution from the user traces:** The baseline trace at 1512 points showed planned slider `[1289.78..1454.78]` (165 points), actual slider ending at 1389.78 (100 points), and actual percentage ending at 1435 rather than its planned 1500. The post-slot trace at 800 points showed that the percentage rectangle is now exactly planned at `[746.78..788.00]`; its centered `100%` galley advances to 780, preserving the intended 8-point reservation inside the slot. The remaining actual slider rectangle ends at 677.78 while the planner ends at 742.78, a 65-point width deficit. This isolates the mismatch to the stock slider's response sizing, not a residual percentage-right-edge, clipping, or panel-inset defect. egui 0.34's public `Spacing::slider_width` defaults to 100 points and is read internally by `Slider` when allocating its response; `add_sized` alone had not overridden that internal default. V2-WO-004 now assigns the planner-provided width to that setting within the slider's scoped UI. The original 85-point gap in the baseline decomposes into that 65-point discrepancy plus the intentional 12+8 point insets; there is no additional margin correction.

For an observed 90 **physical** pixels measured to the galley advance edge, the unexplained residual in logical points is:

`E = 90 / p − 20`.

Examples, before any bearing or native-edge correction:

| Effective scale | Nominal distance | Logical residual that must be explained |
|---|---:|---:|
| 1.0 | 20px | 70 points |
| 1.5 | 30px | 40 points |
| 2.0 | 40px | 25 points |

An extra fixed allocation or alignment **could** produce a width-independent gap if the actual zoom child or percentage rectangle ended `E` points before the planned right edge. Current source does not reveal such an `E`; runtime rectangle equality/difference must establish whether one exists. A stale or different rendered implementation could also have different alignment behavior, but no such runtime mismatch is asserted here.

### 2.4 Ranked, testable source-derived hypotheses before user traces (superseded by the measured resolution above)

These are hypotheses, not findings of runtime causation.

#### Rank 1 — An actual child/painter rectangle differs from the planned rectangle

**Why first:** The path contains several distinct geometry authorities: the planner, allocated child UIs, a nested horizontal UI, the slider response, the percentage allocation, and the painter. Installed egui APIs explicitly track actual child minimum rectangles in [`ui.rs`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/ui.rs:2430). A discrepancy at one boundary would be fixed across window widths if the child consumes or aligns a constant unexpected width.

**Discriminating measurements:** Compare `row_rect.right`, `right_rect.right`, zoom subgroup requested/actual right, nested horizontal response right, actual percentage `rect.right`, and `galley_advance_right`. The first nonzero right-edge delta identifies the layer to inspect. If `A = row_right` and `galley_advance_right = A − 8`, this hypothesis is excluded for the advance-edge measurement.

#### Rank 2 — The measurement mixes native outer/content edges, logical points, physical pixels, screenshot scaling, or ink/advance edges

**Why second:** The app has native chrome and an explicit native-scale path, while the formula is logical and ends at egui content. A measurement to outer window pixels is not directly comparable with 20 logical points. Visible ink is also distinct from galley advance. These are deterministic unit/reference distinctions even though their contribution to the user's 90px is unknown.

**Discriminating measurements:** Record native inner and outer right edges, egui screen/content right, native and effective pixels per point, UI zoom, screenshot scale, galley advance edge, and visible mesh/ink right. Recompute using one unit and one edge reference. At 2x, 40 nominal physical pixels versus the observed roughly 90 leaves about 50 physical pixels to attribute elsewhere.

#### Rank 3 — Clipping makes the visible text edge differ from the uncut galley advance edge

**Why third:** The row and region clips are intersected repeatedly, and the galley painter is clipped to the actual percentage rectangle at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:481). A right clip can remove the rightmost glyphs and create a larger visible gap without changing the uncut galley origin. This would be a paint/clip issue rather than an allocation inset.

**Discriminating measurements:** Compare uncut galley advance right, mesh/ink bounds before clip, actual painter clip right, percentage hover rectangle right, and visible mesh after clip. If the advance formula holds but the visible mesh ends much earlier, inspect the clip owner rather than changing padding.

#### Rank 4 — The observed application output does not correspond to this exact current source path

**Why fourth:** WO-008's source specifically removed the old fixed-`300%` measurement and center alignment, but all runtime review remains user-pending in [`ui-icons-v1.md`](ui-icons-v1.md:458). An older running binary or different working state could exhibit a different fixed gap. This is a practical possibility, not evidence that the user ran a stale build.

**Discriminating measurements:** Have the user identify the build/source revision and compare behavior with the exact checkout. In particular, determine whether the rendered path measures the current label and right-aligns it as shown at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1286) and [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:479).

### 2.5 Exact user-inspectable rectangle and measurement ledger

A single frame at each review width should record, in logical points unless noted:

1. Native viewport inner and outer rectangles; native pixels per point; effective pixels per point; UI zoom; screenshot/output scale.
2. Bottom panel response rectangle; panel content maximum rectangle; inherited panel clip.
3. `available_rect_before_wrap`, the available/clip intersection, and `row_rect`.
4. `right_rect`, planned percentage rectangle, planned slider rectangle, and planned gap.
5. Zoom subgroup requested rectangle and actual child minimum/response rectangle.
6. Nested horizontal UI requested and actual rectangle.
7. Stock slider response rectangle and current value.
8. Actual percentage allocated rectangle, hover/interact rectangle, and painter clip.
9. Actual current label, galley size, galley origin, uncut advance right, visible mesh/ink right, and final visible clip right.
10. Derived deltas: `panel_outer_right − panel_content_right`, `row_right − right_rect.right`, `right_rect.right − zoom_actual_right`, `zoom_actual_right − percentage_actual_right`, `percentage_actual_right − galley_advance_right`, and `galley_advance_right − visible_ink_right`.

Capture 1200, 480, and 320 logical-point widths at the same zoom, plus another zoom value at one width. Width-independent right-edge deltas separate fixed trailing geometry from slider growth and digit-dependent left-side allocation. This is a request for future user-owned observation, not instrumentation added or run by this research.

### 2.6 Numeric-adjustment boundary

Do not change the user's constants to compensate for the observation. In particular, preserve:

- [`PANEL_HORIZONTAL_INSET`](../../apps/scriptcraft/src/status_bar.rs:23) = 12;
- [`ICON_HOVER_PADDING`](../../apps/scriptcraft/src/status_bar.rs:25) = 2;
- [`ICON_SLOT_WIDTH`](../../apps/scriptcraft/src/status_bar.rs:27) = 28;
- [`ITEM_GAP`](../../apps/scriptcraft/src/status_bar.rs:29) = 4;
- [`PERCENTAGE_SIDE_RESERVATION`](../../apps/scriptcraft/src/status_bar.rs:31) = 8;
- [`BODY_TEXT_SIZE`](../../apps/scriptcraft/src/status_bar.rs:33) = 10;
- [`BUTTON_TEXT_SIZE`](../../apps/scriptcraft/src/status_bar.rs:35) = 10;
- [`ICON_GLYPH_SIZE`](../../apps/scriptcraft/src/status_bar.rs:37) = 17;
- [`ZOOM_SLIDER_RAIL_HEIGHT`](../../apps/scriptcraft/src/status_bar.rs:40) = 4;
- slider minimum/maximum allocations 40 and 165 at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1296).

A source correction is warranted only after actual measurements identify an owner. No “subtract 90”, inset reduction, or replacement magic constant is proposed.

---

## 3. Diagnostics development/production gate

### 3.1 Implemented enablement, opening, rendering, and state origin

**Enablement:** [`presentation::ENABLE_DIAGNOSTICS`](../../apps/scriptcraft/src/status_bar.rs:47) is the single manual boolean and defaults to `false`. [`diagnostics_enabled()`](../../apps/scriptcraft/src/status_bar.rs:258) exposes only this value to the parent application module. There is no Cargo feature, profile condition, runtime preference, hidden command, or document persistence. Changing the source constant requires rebuild/relaunch.

**Opening:** When enabled, [`show_with_focus_mode()`](../../apps/scriptcraft/src/status_bar.rs:1389) leaves the explicit open override absent, while the header retains ID salt `status-bar-diagnostics`, `.default_open(false)`, and egui-memory disclosure state. It remains outside application and native-document state. When disabled, no collapsing header exists to receive focus, pointer, or accessibility interaction; existing egui-memory state is not cleared.

**Rendering:** [`show_impl_with_appearance()`](../../apps/scriptcraft/src/status_bar.rs:1463) guards the entire header block with the same `presentation::ENABLE_DIAGNOSTICS` constant. Disabled mode creates no Diagnostics header/body, placeholder allocation, focus target, or accessibility node. Enabled mode retains the previous collapsed default, safe summary, clipped single-line body, and stock header behavior.

**Projection origin:** Before showing the bottom panel, [`show_application_panels()`](../../apps/scriptcraft/src/main.rs:2484) uses the narrow accessor to conditionally format the detailed performance summary and query pending derived work for presentation. It constructs an optional [`DiagnosticProjection`](../../apps/scriptcraft/src/main.rs:2525) only when the gate is enabled; disabled mode constructs no empty substitute. The enabled projection still gathers:

- [`NativeInputDiagnostics`](../../apps/scriptcraft/src/main.rs:88), including focus, focus-lock, dispatch, event, and pointer state;
- [`PerformanceDiagnostics`](../../apps/scriptcraft/src/main.rs:295), including frame/event/layout/cache/paint/repaint counters and the detailed value-free summary from [`PerformanceDiagnostics::safe_summary()`](../../apps/scriptcraft/src/main.rs:339);
- the live session pending-derived summary read at [`main.rs`](../../apps/scriptcraft/src/main.rs:2511).

Thus the body summary remains lazy when expanded, while the detailed performance string, presentation-only pending-derived query, and projection are skipped entirely when the gate is disabled. Underlying operational counters and native-input/performance instrumentation remain active.

### 3.2 Implemented gate and exact location

The named manual constant is in the existing [`presentation`](../../apps/scriptcraft/src/status_bar.rs:19) module, adjacent to the other status-owned presentation policy:

**Implemented semantics:** `ENABLE_DIAGNOSTICS: bool = false`, with comments identifying `false` as the production default and `true` as an explicit local development choice requiring rebuild/relaunch.

Because the module remains private to the status-bar module, [`diagnostics_enabled()`](../../apps/scriptcraft/src/status_bar.rs:258) is the narrow status-bar accessor for the application caller; it returns the named constant and is not a second setting or runtime state source.

The implementation uses that single value at both boundaries:

1. **Projection boundary:** In [`show_application_panels()`](../../apps/scriptcraft/src/main.rs:2484), an optional projection means disabled mode skips the detailed performance string, pending-derived presentation read, and [`DiagnosticProjection`](../../apps/scriptcraft/src/main.rs:2525) construction without fabricating an empty diagnostic value.
2. **Presentation boundary:** [`show_impl_with_appearance()`](../../apps/scriptcraft/src/status_bar.rs:1463) wraps the entire header block, so disabled mode renders no header, body, placeholder allocation, or accessibility node. When enabled, it requires the supplied projection and retains the existing rendering path.

The renderer also honors the same constant rather than relying solely on a distant caller to pass no data, keeping row visibility and projection availability coherent.

### 3.3 Why a manual constant instead of a profile or Cargo feature

| Mechanism | Consequence | Decision |
|---|---|---|
| Named manual boolean in [`presentation`](../../apps/scriptcraft/src/status_bar.rs:19) | One obvious product/presentation policy point; identical explicit choice in every profile; no manifest or dependency change; compiler can remove disabled branches | **Selected** |
| `cfg!(debug_assertions)` | Couples visibility to build profile rather than product intent; makes ordinary debug and release behavior differ; complicates reproducible user review and prevents an independent production-like debug run | Rejected |
| Cargo feature | Introduces build-variant API and manifest/lock coordination for a single status disclosure; useful only if a future distribution system needs independently selectable feature builds | Not warranted now |

The manual constant is compile-time configuration, not a user preference, persisted setting, runtime toggle, or hidden command. Changing it is an intentional source edit followed by user-owned rebuild and review.

### 3.4 Gate scope: hide row, disable projection, or both

**Both are implemented.**

- **Hide only** would leave unnecessary detailed projection/string work and make the production boundary misleading.
- **Disable projection only** would leave a focusable disclosure with missing or fabricated content and create accessibility/layout ambiguity.
- **Both** gives one coherent contract: production has no Diagnostics affordance or presentation projection; development can explicitly restore the complete existing disclosure.

Do **not** disable the underlying [`PerformanceDiagnostics`](../../apps/scriptcraft/src/main.rs:295), [`NativeInputDiagnostics`](../../apps/scriptcraft/src/main.rs:88), layout instrumentation, scheduler state, or counters merely because the row is hidden. Some are intertwined with application operation, repaint scheduling, zoom anchoring, and performance measurement ownership. The gate concerns presentation projection, not a broad instrumentation removal initiative.

### 3.5 Behavior, accessibility, layout, privacy, and state impacts

- **Behavior:** No Diagnostics row or expansion body exists when disabled. Center operation/error projection remains unchanged at [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1409), and privacy-safe save/error behavior is not removed.
- **Accessibility:** The stock collapsing header leaves the tab order and accessibility tree when disabled; no expanded summary label is published. Other status names, controls, and the center error projection must remain unchanged. There should be no disabled empty header announced to users.
- **Layout:** In production-off mode the bottom panel has only the primary status row plus existing frame margins, with no reserved Diagnostics height. The implementation does not change primary row width or right-edge geometry; enabling Diagnostics restores the existing disclosure/summary height behavior.
- **Durable state:** Existing egui memory for `status-bar-diagnostics` need not be cleared. If the constant is re-enabled in the same process/session, prior open state may remain; this is harmless presentation state and should not become native-document persistence.
- **Privacy:** Production no longer formats or displays the detailed diagnostic presentation string. This reduces presentation exposure but is not a claim that all operational counters or internal diagnostic data have been removed.
- **Performance:** Avoiding per-frame detailed projection is consistent with the gate, but no performance improvement is measured or claimed.
- **Development behavior:** With the constant enabled, the implementation retains the existing disclosure, summary content, clipping, accessibility, and state behavior; user review remains pending.

---

## Proposed sequential implementation slices

These slices are recommendations only and do not authorize implementation.

### Slice A — Capture the right-edge baseline before changing geometry

1. Have the user capture the ledger in section 2.5 at the exact current source/build and identify the first unexpected right-edge delta.
2. Record whether the 90px measurement is to native outer edge, egui content edge, galley advance, or visible ink, plus effective scale and screenshot scale.
3. If this resolves to unit/reference interpretation rather than an app defect, do not change production geometry. If it identifies an actual child, clip, or native-edge owner, use that evidence to bound Slice C.

### Slice B — Stabilize percentage geometry

1. Introduce active-font maximum-reference slot measurement while preserving every existing numeric setting.
2. Make the explicit percentage rectangle authoritative for allocation, response, clipping, accessibility bounds, and painting.
3. Center the actual current galley with fit-aware narrow clipping; preserve popup identity/presets, keyboard behavior, current-value accessibility, slider semantics, and WO-007 knob ordering.
4. Reconcile the scoped status allocation/zoom requirements and technical context described below.

### Slice C — Correct only a measured right-edge owner, if one remains

1. Recompute the full ledger with the stable slot and centered galley. Centering intentionally changes galley trailing space, so compare **slot/right-region geometry** separately from galley advance and visible ink.
2. If a deterministic actual-versus-planned delta remains, correct that specific allocation or clip boundary without altering the user's numeric settings.
3. If no app-owned delta remains but the outer-edge observation persists, document the native/content/unit accounting rather than inserting a compensating spacer.

This separation avoids hiding an existing geometry defect underneath the new centering contract.

### Slice D — Diagnostics presentation gate (implemented in V2-WO-003)

1. Added the manual production-off boolean and narrow parent-visible accessor in the status presentation surface.
2. Gated both detailed projection construction and the complete disclosure row.
3. Preserved operational counters/instrumentation, primary status/error behavior, and development-on behavior.
4. Reconciled only the scoped Diagnostics/status durable records described below.

---

## Scoped durable-document impacts for future implementation

No durable document was changed by this research. Future authorized implementation should make only the applicable scoped updates:

### Stable percentage slot and right-edge work

- [`AC-WRITING-006.4`](../../.forge/requirements/features/screenplay-writing-loop.md:615) currently requires current-label measurement. Revise it to stable active-font maximum-reference slot measurement, stable right-group geometry across current values, and bounded compression. If Slice C identifies a separate geometry owner, record that contract narrowly.
- [`AC-WRITING-006.7`](../../.forge/requirements/features/screenplay-writing-loop.md:652) currently requires current-label measurement and right-aligned galley placement. Reconcile it with a fixed reference slot and centered actual galley while preserving zoom range, presets, accessibility, slider behavior, clipping, and knob ordering.
- [`AC-WRITING-006.8`](../../.forge/requirements/features/screenplay-writing-loop.md:689) concerns height stability; retain it and clarify only if implementation needs to state that digit changes also do not destabilize the primary row/group geometry. Do not weaken the existing height oracle.
- [`AC-WRITING-006.9`](../../.forge/requirements/features/screenplay-writing-loop.md:695) remains applicable to clipped single-line content; update only if centered/gated content changes its wording.
- [`technical-context.md`](../../.forge/blueprints/technical-context.md:91) currently records WO-008's current-label/right-alignment formula and stale earlier numeric inventory. Update the scoped status allocation/zoom paragraph to the selected contract and the measured final right-edge accounting; do not rewrite unrelated history.

### Diagnostics gate

- V2-WO-003 conditionally scopes the Diagnostics wording in [`AC-WRITING-006.1`](../../.forge/requirements/features/screenplay-writing-loop.md:569), [`AC-WRITING-006.3`](../../.forge/requirements/features/screenplay-writing-loop.md:599), [`AC-WRITING-006.4`](../../.forge/requirements/features/screenplay-writing-loop.md:615), and [`AC-WRITING-006.9`](../../.forge/requirements/features/screenplay-writing-loop.md:695), preserving privacy-safe errors and specifying no disabled row or accessibility affordance.
- The status and observability paragraphs in [`technical-context.md`](../../.forge/blueprints/technical-context.md:60), [`technical-context.md`](../../.forge/blueprints/technical-context.md:136), and [`technical-context.md`](../../.forge/blueprints/technical-context.md:154) now distinguish retained operational instrumentation from gated status presentation/projection.
- No change to the authoritative workflow policy in [`project-manifest.md`](../../.forge/project-manifest.md:73) is needed for these UI proposals. Historical initiatives and work orders remain unchanged.

---

## User-owned review and validation requests

Nothing below has been run. These are requested future user checks after explicit implementation authorization, not acceptance evidence or a request for agent validation.

### Percentage and group geometry

1. Compare 75%, 90%, 99%, 100%, 101%, 125%, 200%, and 300%. Confirm the nominal percentage slot, Timer/Focus/Appearance positions, slider allocation, and center region no longer change merely because the displayed digit count changes.
2. Confirm each actual galley is horizontally centered in its slot and remains vertically centered. Verify the intended tradeoff: centered text has variable unused side space rather than WO-008's fixed trailing advance edge.
3. Check 1200, 480, 320, exact-fit, threshold-adjacent, and physically unusable widths. Confirm bounded nonoverlap, predictable clipping, no new row growth, no hidden overflow action, and no unsupported hit-target promise at extreme widths.
4. Exercise pointer, Tab/Shift-Tab, Enter/Space, slider arrows/Home/End, all nine presets, intermediate values, outside-click/Escape dismissal, and popup anchoring while values change. Confirm the accessible name always reports the actual value, not `300%`.
5. Inspect all Appearance choices and any user-selected status font/reservation adjustments. Confirm reference measurement follows the active font and no numeric manual setting was reset.
6. Recheck WO-007 knob completeness and sibling paint priority at minimum/maximum zoom and near popups.

### Right-edge investigation

1. Capture the section 2.5 ledger at multiple widths and effective scales, using the exact source/build under review. Identify the first nonzero unexpected right-edge delta rather than comparing only final screenshots.
2. Measure separately to native outer edge, egui content edge, allocation edge, galley advance edge, and visible ink edge. Record physical pixels per point and any screenshot rescaling.
3. Compare calculated `20 × p` nominal distance with observed distance and compute `E = observed / p − 20`. Do not treat bearing alone as the explanation without mesh/ink evidence.
4. If clipping is suspected, compare uncut galley/mesh bounds with the actual painter clip and visible result. If allocation is suspected, compare each planned rectangle with its actual child/response counterpart.
5. After the fixed-slot change, distinguish stable slot/right-region distance from intentionally different centered-galley side space. Do not approve a numeric inset change as a substitute for identifying the owner.

### Diagnostics gate

1. With the constant production-off, confirm there is no visible or keyboard-focusable Diagnostics header, no expanded summary accessibility node, no reserved second row, and no detailed status projection construction. Confirm operation/errors and all other controls remain available.
2. With development-on, confirm the prior collapsed default, click/keyboard toggling, clipped value-free summary, accessibility naming, and footer height behavior remain intact.
3. Toggle only through separate user-selected builds, not a runtime preference. Confirm native documents and operational counters/instrumentation are unaffected and no prior Diagnostics state enters persisted document state.
4. Review normal and narrow widths in both modes and verify that disabling Diagnostics reduces height without changing primary row/right-edge geometry.

### Compilation and regression ownership

Any desired build, check, test, lint, formatting, desktop automation, accessibility inspection, native visual review, and regression validation remains exclusively user-owned under [`project-manifest.md`](../../.forge/project-manifest.md:73). No such result is claimed by this document, and no initiative acceptance or closeout is recorded.

---

## Screenshot and direct work-order review addendum — 2026-11-05

The supplied screenshot confirms that the percentage ends well before the native window's right edge and that the visible whitespace is much larger than the nominal two-inset explanation. It strengthens the report but cannot distinguish planned allocation, actual child layout, clipping, native outer/content edges, or display scaling. No exact pixel ruler or scale assumption is inferred from the image.

At planning time the remaining input was a same-frame geometry ledger. The user supplied the V2-WO-001 baseline and post-V2-WO-002 trace, satisfying V2-WO-004's evidence gate. That evidence showed the percentage slot right edge corrected and the stock slider still 65 points narrower than its planned allocation. V2-WO-004 now sizes the stock slider from the planner width and removes V2-WO-001's temporary production trace; no post-correction runtime evidence is claimed.

### Proposed work orders for user review

| ID | Direct outcome | Dependency and evidence boundary |
|---|---|---|
| [`V2-WO-001`](work-orders-v2/WO-001-status-right-edge-geometry-trace.md) | Add bounded temporary geometry-only tracing for every planned and actual right-edge boundary | No layout changes; user owns the evidence run |
| [`V2-WO-002`](work-orders-v2/WO-002-stable-centered-percentage-slot.md) | Measure a stable active-font maximum-reference slot and center the actual current percentage in the explicit planned rectangle | After V2-WO-001 handoff; preserve trace and all manual values |
| [`V2-WO-003`](work-orders-v2/WO-003-diagnostics-development-gate.md) | Add a manual production-off Diagnostics boolean gating both row and detailed projection | After V2-WO-002; independent of the final margin owner |
| [`V2-WO-004`](work-orders-v2/WO-004-measured-status-right-edge-correction.md) | Correct only the allocation, alignment, or clip owner identified by actual traces, or document a non-app edge/unit explanation | Evidence gate satisfied by both user traces; implementation handoff below; no guessed offset |

The implementation sequence was V2-WO-001, V2-WO-002, V2-WO-003, then V2-WO-004 after receipt of the evidence. The only elevated execution dependency—the user-owned geometry run—has been satisfied.

The user authorized individual coding-agent delegation on 2026-11-05. Execution followed the stated sequence and dependencies on the existing shared branch, with all existing work preserved. The V2-WO-004 evidence gate was satisfied; its source/documentation implementation is recorded below. No build, runtime inspection, review, validation, staging, commit, or acceptance is claimed.

## V2-WO-001 implementation handoff — 2026-11-05

V2-WO-001 temporary geometry instrumentation is implemented on the existing `dev/2026-11-04-ui-icons` branch and is initially enabled for the user's evidence build. This is instrumentation only: no status layout, painting, interaction, accessibility, popup, zoom, presentation value, margin, or color was intentionally changed. The user owns the build, application run, evidence capture, review, and all validation under the authoritative project policy.

### Trace locations and output

- [`ENABLE_STATUS_GEOMETRY_TRACE`](../../apps/scriptcraft/src/status_bar.rs) is the temporary switch in the status presentation module and currently reads `true`. It is separate from the proposed Diagnostics gate and is intended for removal in V2-WO-004 after the user-supplied evidence is consumed.
- [`StatusGeometryTrace`](../../apps/scriptcraft/src/status_bar.rs) and the status-row capture in [`show_impl_with_appearance()`](../../apps/scriptcraft/src/status_bar.rs) record available/planned/actual row, right-group, zoom subgroup, nested horizontal, slider, percentage allocation/interaction/clip, galley origin/size/advance, mesh bounds, visible mesh bounds, and zoom percentage.
- [`show_application_panels()`](../../apps/scriptcraft/src/main.rs) captures the bottom panel response, panel content maximum rectangle and clip, then joins them with native viewport inner/outer data and the egui content rectangle. The same ledger includes native/effective pixels-per-point, derived UI zoom, and the seven right-edge deltas from panel outer to visible mesh.
- Each line is written to stderr with the prefix `[V2-WO-001 status geometry]`. The latest changed geometry signature is emitted at most once per 250 ms; unchanged frames produce no line. The pending latest signature is emitted on a later UI frame after the interval.

### Exact user evidence instructions

1. From this exact checkout on `dev/2026-11-04-ui-icons`, build and launch the evidence executable using the user's normal local workflow. For the recorded stderr route, launch `cargo run -p scriptcraft 2> /tmp/scriptcraft-v2-wo-001.stderr` from the repository root. Keep that process and window open; do not use a previously running or separately built app.
2. At the user's normal window size, then at approximately 1200, 480, and 320 logical-point egui content widths where practical, let a frame render after each resize and capture the corresponding native screenshot. Use `egui_content`/`available` widths in the emitted ledger to identify the actual logical width. Also capture one different zoom percentage at one of those widths.
3. Record the monitor/native scale and whether screenshots were resized or otherwise output-scaled. Keep the screenshot edge reference explicit (native outer edge versus egui content edge); screenshots supplement but do not replace the ledger.
4. Close the app normally, then extract only the matching geometry lines from the captured stderr file using `grep '^\[V2-WO-001 status geometry\]' /tmp/scriptcraft-v2-wo-001.stderr`. Supply those tagged lines with the screenshots and scale notes. Do not share unrelated stderr output.
5. The trace intentionally omits document text, paths, timer content, error details, and other project content. It records only status zoom percentage and geometry/scale values; no content-bearing label strings are logged.

### Trace limitations and evidence state

Native inner/outer rectangles are reported only when egui exposes them; their native coordinate origin may not be the same as the local egui content coordinate origin. Effective scale is egui pixels-per-point; native scale and derived UI zoom say `unavailable` when the backend does not expose a valid native scale. Galley mesh bounds are pre-rasterization geometry; `visible_mesh_bounds` is their intersection with the captured painter clip, not a pixel scan of final rasterized ink. The trace cannot infer screenshot rescaling or native-decoration pixel thickness. The user supplied both the baseline ledger and a post-slot ledger; the latter covers only 800×600 at 100% zoom and effective ppp 2.0. The measured owner is documented in the V2-WO-004 handoff below. No post-fix trace is available or claimed. V2-WO-002 and V2-WO-003 are implemented below.

## V2-WO-002 implementation handoff — 2026-11-05

V2-WO-002 is implementation-submitted on the existing shared `dev/2026-11-04-ui-icons` branch. This handoff records code/document changes only. No build, check, test, lint, formatting command, application run, manual verification, review, or other validation was run or claimed. The user owns all review and validation under the authoritative project policy. V2-WO-003's Diagnostics gate and V2-WO-004's final measured margin correction/trace removal are not included.

### User-supplied baseline evidence (not agent validation)

The original user trace file [`W0-002-grep-report.txt`](work-orders-v2/W0-002-grep-report.txt) is preserved unchanged. At egui content width 1512, its ledger records planned row/content right 1500; planned slider width 165 and actual slider width 100; actual zoom right 1439; planned percentage right 1500 and actual percentage/interaction/clip right 1435 (65 points before planned); and galley advance right 1427. The corresponding distances are content right 1500 minus galley advance right 1427 = 73 logical points, and egui window right 1512 minus galley advance right 1427 = 85 logical points = 170 physical pixels at effective ppp 2. This is the user's observed baseline geometry, not an agent-run observation, an agent validation, or evidence of post-slot behavior.

### Implementation decisions and exact locations

- [`measured_percentage_slot_width()`](../../apps/scriptcraft/src/status_bar.rs:1413) measures `MAX_ZOOM_PERCENT` plus the percent sign with [`status_font_id()`](../../apps/scriptcraft/src/status_bar.rs:254) in the active status Button font and adds both existing [`PERCENTAGE_SIDE_RESERVATION`](../../apps/scriptcraft/src/status_bar.rs:31) values. All numeric, spacing, and color settings remain unchanged.
- [`show_impl_with_appearance()`](../../apps/scriptcraft/src/status_bar.rs:1480) still formats the actual current percentage for paint and the accessible name; only the stable reference slot width is passed to [`plan_status_row()`](../../apps/scriptcraft/src/status_bar.rs:408).
- [`zoom_controls()`](../../apps/scriptcraft/src/status_bar.rs:518) scopes the stock slider and percentage control to their planner rectangles, with no nested horizontal sequential-width reconstruction. The slider remains the stock slider response; the existing delayed WO-007 knob paint and its right-group/row/parent clip remain unchanged.
- [`zoom_presets_button()`](../../apps/scriptcraft/src/status_bar.rs:647) takes its allocation rectangle from the exact scoped planned rectangle. Its interaction and accessibility response, hover, popup anchor, painter clip, and paint all derive from that rectangle intersected with the inherited clip. The current galley is horizontally and vertically centered when it fits; its origin is clamped at narrow widths and the painter remains clipped to the slot. The actual label remains the displayed and accessible value.
- The [`StatusGeometryTrace`](../../apps/scriptcraft/src/status_bar.rs:62) switch and capture remain enabled/present. The obsolete nested-horizontal field now reports unavailable because the nested layout was removed; actual slider, percentage, interaction, clip, and galley geometry continue to be captured for the post-slot evidence run.
- [`AC-WRITING-006.4`](../../.forge/requirements/features/screenplay-writing-loop.md:615), [`AC-WRITING-006.7`](../../.forge/requirements/features/screenplay-writing-loop.md:652), and the status paragraph in [`technical-context.md`](../../.forge/blueprints/technical-context.md:91) now specify the stable maximum-reference slot and centered actual label. This explicitly supersedes WO-008's actual-current-label-width and right-aligned-galley language; WO-008 remains historical evidence of the prior implementation contract.

### Post-slot user evidence instructions

Use the exact workflow and tagged stderr extraction documented in [V2-WO-001's user evidence instructions](work-orders-v2/WO-001-status-right-edge-geometry-trace.md:39), but capture the current post-WO-002 checkout in a separate file such as `/tmp/scriptcraft-v2-wo-002.stderr` using the same run command with stderr redirected there. Do not overwrite or edit the supplied baseline file or its original stderr file. Capture normal size plus approximately 1200, 480, and 320 logical-point content widths where practical, and one different zoom value; include screenshots, native/effective scale, output rescaling, and the precise edge reference. Extract only lines beginning `[V2-WO-001 status geometry]`. Compare planned/actual slider and percentage rectangles, zoom right, galley advance, and clip. This post-slot trace is user-owned evidence; provide it before V2-WO-004, which must make no guessed margin correction and must preserve the trace until the measured owner is known. No post-slot behavior is asserted here.

## V2-WO-003 implementation handoff — 2026-11-05

V2-WO-003 is implementation-submitted on the existing shared `dev/2026-11-04-ui-icons` branch. This handoff records code and scoped document changes only. No build, check, test, lint, formatting command, application run, manual verification, review, or other validation was run or claimed. User review and all validation remain pending under the authoritative project policy.

### Gate and implementation locations

- [`presentation::ENABLE_DIAGNOSTICS`](../../apps/scriptcraft/src/status_bar.rs:47) is the single named boolean, defaults to `false`, and documents production-off, explicit local development-on, and rebuild/relaunch requirements. [`diagnostics_enabled()`](../../apps/scriptcraft/src/status_bar.rs:258) is the narrow parent-visible accessor; the presentation module remains private.
- [`show_application_panels()`](../../apps/scriptcraft/src/main.rs:2484) uses that accessor to skip detailed performance formatting and the pending-derived-work presentation query while disabled. It passes an optional [`DiagnosticProjection`](../../apps/scriptcraft/src/main.rs:2525) only when enabled; no empty diagnostics substitute is fabricated.
- [`show_impl_with_appearance()`](../../apps/scriptcraft/src/status_bar.rs:1463) checks the same module constant before constructing the complete disclosure. Disabled mode has no Diagnostics row/body, placeholder allocation, focus target, or accessibility node. Enabled mode preserves the existing collapsed default, egui-memory disclosure behavior, safe clipped summary, keyboard/pointer/accessibility behavior, and footer sizing.
- Operational counters, [`PerformanceDiagnostics`](../../apps/scriptcraft/src/main.rs:295), [`NativeInputDiagnostics`](../../apps/scriptcraft/src/main.rs:88), repaint/layout scheduling, other status/error behavior, V2-WO-001 trace instrumentation, V2-WO-002 stable centered percentage geometry, WO-007 knob behavior, and all manual numeric/color values remain. Neither the V2-WO-001 trace nor baseline/post-slot trace artifacts were changed. V2-WO-004's slider-width/margin correction and trace removal were not performed.

Only the scoped Diagnostics-conditional requirements AC-WRITING-006.1/.3/.4/.9 and the status/observability documentation were updated. The user owns all review and validation; no acceptance, build, or runtime result is implied.

## V2-WO-004 implementation handoff — 2026-11-05

V2-WO-004 is implementation-submitted on the existing shared `dev/2026-11-04-ui-icons` branch after its user-evidence gate was satisfied. Both the baseline and post-slot trace files remain unchanged. No build, application run, checks, tests, lint, formatting, manual verification, review, or validation was run or claimed; all validation remains user-owned.

The traces show the percentage right edge already matches its planned rectangle after V2-WO-002 (`[746.78..788.00]` in the supplied post-slot 800-point frame). The remaining mismatch was the stock slider response: planned end 742.78, actual end 677.78, width 100 rather than planned 165, a 65-point shortfall. The baseline's former 85-point content/window-right-to-galley gap decomposes into this same 65-point shortfall plus the intended 12-point panel inset and 8-point centered-galley reservation; no additional percentage/panel margin defect or compensation is warranted.

[`zoom_slider()`](../../apps/scriptcraft/src/status_bar.rs) now assigns the planner-provided width to egui 0.34's public `Spacing::slider_width` on the scoped slider UI before adding the stock `Slider`. egui's slider response allocation reads that spacing value, which defaults to 100 points; `add_sized` alone did not override it. This uses the existing 40–165-point planner allocation directly and adds no sizing constant. Stock slider response semantics and adjacent percentage geometry are preserved.

Removed V2-WO-001's temporary production tracing from [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs) and [`main.rs`](../../apps/scriptcraft/src/main.rs): its switch, data structure, helpers, capture parameters/assignments, and main-panel logging call are gone. The evidence files are untouched and no unsolicited trace logging remains. Updated only the measured-owner portions of AC-WRITING-006.4/.7, the scoped status sizing/evidence passage in technical context, and this initiative/work-order handoff. User review and validation remain pending. The post-slot evidence covers only 800×600 at 100% zoom and effective ppp 2.0; no post-fix runtime result is asserted, and other widths/scales, narrow interaction, popup/knob behavior, and native outer/content accounting remain for user-owned review.

## Final status-bar presentation implementation handoff — 2026-11-05

Implemented on the existing shared `dev/2026-11-04-ui-icons` branch. The change adds [`presentation::PANEL_VERTICAL_INSET`](../../apps/scriptcraft/src/status_bar.rs:25), [`presentation::ZOOM_SLIDER_MIN_WIDTH`](../../apps/scriptcraft/src/status_bar.rs:33), [`presentation::ZOOM_SLIDER_MAX_WIDTH`](../../apps/scriptcraft/src/status_bar.rs:35), and [`presentation::PAGE_COUNTER_HORIZONTAL_INSET`](../../apps/scriptcraft/src/status_bar.rs:40); their current values are 2, 40/165, and 0 points. [`presentation::CENTER_TEXT_SIZE`](../../apps/scriptcraft/src/status_bar.rs:46) and [`presentation::CENTER_FOREGROUND_OVERRIDE`](../../apps/scriptcraft/src/status_bar.rs:62) provide a separate 10-point center font and optional color. [`presentation::center_foreground()`](../../apps/scriptcraft/src/status_bar.rs:71) falls back to the shared status-default foreground helper, and [`with_center_text_style()`](../../apps/scriptcraft/src/status_bar.rs:120) scopes the center font without changing Body/Button/page/Diagnostics styling.

[`plan_status_row()`](../../apps/scriptcraft/src/status_bar.rs:259) now bounds the separately left-aligned page allocation so it cannot overlap the mirrored center or right controls. For row width `W` and full right-group width `R` (row right to Timer left), the center is `[row_left + R, row_right - R]`, with nominal width `max(W - 2R, 0)`; if `W <= 2R`, it is zero-width at Timer's left edge. [`page_counter_content_rect()`](../../apps/scriptcraft/src/status_bar.rs:1178) and [`paint_page_counter()`](../../apps/scriptcraft/src/status_bar.rs:1187) apply the page inset to the painted/clipped/accessibility rectangle; the measured/allocation width also includes both insets. Center text is one-line clipped and centered within its explicit rectangle while its full operation/error string remains accessible. The scoped requirements and technical-context status allocation/typography descriptions were updated. No temporary tracing was added or restored. No tests, checks, formatting, build, manual verification, review, or validation were run; all validation remains user-owned.

## Final compiler cleanup and authorized checks — 2026-11-05

The final closeout authorization explicitly allowed compiler checks and fixes. The initial [`cargo check -p scriptcraft`](../../Cargo.toml:1) and [`cargo check --workspace`](../../Cargo.toml:1) both compiled successfully but emitted 20 application-owned warnings: 18 deprecated egui/eframe API uses and two dead-code warnings for the reusable status-item text constructor and renderer. No dependency-owned warnings were reported.

The app-owned deprecations were migrated to egui 0.34 APIs: the top/bottom panels use [`Panel::top()`](../../apps/scriptcraft/src/main.rs:2498) and [`Panel::bottom()`](../../apps/scriptcraft/src/main.rs:2548), with top, bottom, central, and migration panels rendered using [`Panel::show_inside()`](../../apps/scriptcraft/src/main.rs:2502) within the existing root UI and in the existing order. Panel height uses [`Panel::exact_size()`](../../apps/scriptcraft/src/main.rs:2501); former [`Context::screen_rect()`](../../apps/scriptcraft/src/main.rs:659) calls use [`Context::content_rect()`](../../apps/scriptcraft/src/main.rs:659), preserving the previous content-rectangle semantics; Context style reads use [`Context::global_style()`](../../apps/scriptcraft/src/status_bar.rs:85); the empty frame uses [`Frame::NONE`](../../apps/scriptcraft/src/main.rs:2474); and overflow menus use [`Ui::close()`](../../apps/scriptcraft/src/toolbar.rs:492). The two intentionally reusable status-item APIs have narrowly scoped dead-code annotations documenting their intended capability use; no broad warning allowances were added.

After those fixes, both [`cargo check -p scriptcraft`](../../Cargo.toml:1) and [`cargo check --workspace`](../../Cargo.toml:1) were rerun successfully with exit code 0 and no warning or error output. No full test suite, lint/format campaign, runtime launch, manual desktop validation, or independent review was performed as part of this closeout. The earlier work-order sections above record their contemporaneous evidence boundaries; this final compiler handoff supersedes their prior “no checks” status for these two authorized compiler commands only. Remaining limitations are user-owned runtime/accessibility/layout review and the explicitly unrun tests and other validations.

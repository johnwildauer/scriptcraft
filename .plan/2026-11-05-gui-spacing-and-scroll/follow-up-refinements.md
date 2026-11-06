# GUI follow-up refinements

- **Status:** WO-007 implementation submitted for user validation; no validation or acceptance recorded
- **Date:** 2026-11-05
- **Parent initiative:** [`gui-spacing-and-scroll.md`](gui-spacing-and-scroll.md)
- **Branch for any approved follow-up:** `dev/2026-11-05-gui-spacing-and-scroll`
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or independent validators
- **Boundary investigation:** [`boundary-and-border-investigation.md`](boundary-and-border-investigation.md) records the remaining toolbar-bottom allocation and the separate status frame/separator stroke owners; no correction is authorized by that note.

## 1. Outcome

Address the user's post-implementation observations without disturbing the parts already reported as satisfactory:

1. Center the Elements caption horizontally beneath its dropdown.
2. Make Elements and ordinary action captions share the same nominal vertical position, with a separate Elements-only vertical adjustment for optical alignment.
3. Remove the remaining gap between the toolbar and the first page while retaining bottom page-end padding.
4. Use the requested default native window geometry and align generated/fallback FDX window state with it.

Current user-adjusted source values are inputs to preserve except where this plan explicitly changes them. In particular, ordinary toolbar caption spacing, toolbar outer padding, and bottom Script View padding should not be reset to earlier agent-submitted values.

## 2. Toolbar findings and box model

### 2.1 Current action Item structure

The following is a JSX-like component outline, not a second implementation or a proposal to introduce HTML:

- [`<ActionItem>`](../../../apps/scriptcraft/src/toolbar.rs:538)
  - [`<ItemGroup>`](../../../apps/scriptcraft/src/toolbar.rs:546) with height derived from two framework interaction rows.
  - [`<IconControlSlot>`](../../../apps/scriptcraft/src/toolbar.rs:549) anchored to the group's top and centered horizontally.
  - [`<SharedCaptionGap>`](../../../apps/scriptcraft/src/toolbar.rs:592) using the ordinary control-to-caption setting.
  - [`<CaptionSlot>`](../../../apps/scriptcraft/src/toolbar.rs:589) occupying the remainder of the group.
  - [`<CenteredCaption>`](../../../apps/scriptcraft/src/toolbar.rs:596) centered horizontally and vertically in that remainder.

With current source values, this is a 36-point group, an 18-point nominal icon-control slot, a 1-point gap, and a 17-point caption remainder. The caption's horizontal position is correct and its ordinary gap should remain unchanged.

### 2.2 Current Elements structure

- [`<ElementsItem>`](../../../apps/scriptcraft/src/toolbar.rs:626)
  - [`<ItemGroup>`](../../../apps/scriptcraft/src/toolbar.rs:635) with the same total group height.
  - [`<ComboBox>`](../../../apps/scriptcraft/src/toolbar.rs:645) whose actual closed response height is measured after rendering.
  - [`<CaptionGap>`](../../../apps/scriptcraft/src/toolbar.rs:716) using the same ordinary gap.
  - [`<CaptionSlot>`](../../../apps/scriptcraft/src/toolbar.rs:712) whose height is calculated from the group minus the actual ComboBox response height and gap.
  - [`<LeftCenteredCaption>`](../../../apps/scriptcraft/src/toolbar.rs:723) painted at the caption rectangle's left-center anchor.

Two defects follow directly:

1. [`Align2::LEFT_CENTER`](../../../apps/scriptcraft/src/toolbar.rs:725) and [`label_rect.left_center()`](../../../apps/scriptcraft/src/toolbar.rs:724) explicitly left-align the caption. This is why it is no longer centered.
2. [`combo_response.rect.height()`](../../../apps/scriptcraft/src/toolbar.rs:713) participates in caption allocation. Any difference between the actual ComboBox response height and the action icon-control height shifts the caption's available rectangle and center. The dropdown therefore currently influences caption placement.

### 2.3 Target common structure

- [`<ItemGroup>`](../../../apps/scriptcraft/src/toolbar.rs:635)
  - [`<ControlSlot>`](../../../apps/scriptcraft/src/toolbar.rs:635) containing either the ordinary icon control or the Elements ComboBox.
  - [`<SharedCaptionGap>`](../../../apps/scriptcraft/src/toolbar.rs:15) unchanged for both Item types.
  - [`<CommonCaptionSlot>`](../../../apps/scriptcraft/src/toolbar.rs:589) derived from the same nominal control height and group rectangle for both Item types.
  - [`<CenteredCaption>`](../../../apps/scriptcraft/src/toolbar.rs:596) centered horizontally and vertically for both Item types.
  - [`<ElementsVerticalOffset>`](../../../apps/scriptcraft/src/toolbar.rs:15) applied only to the Elements caption rectangle, initially zero.

The ComboBox response height should no longer determine the Elements caption's nominal center. A shared caption-rectangle helper should receive the group rectangle and nominal control height; action and Elements captions should consume equivalent rectangles. The Elements-only offset then provides a narrow optical adjustment without changing ordinary Items.

## 3. Script View top-gap baseline

Before the submitted implementation, the central Script Workspace frame was already margin-free through [`Frame::NONE`](../../../apps/scriptcraft/src/script_view.rs:1476), and Script View fades were disabled locally. The remaining initial top gap was the explicit scroll-content margin:

- The former shared page-edge setting was 4 logical points and was applied symmetrically by the ScrollArea content margin.
- The former prepaint zoom geometry used that shared value for the first-page origin and twice in total content extent.

The submitted implementation replaces the symmetric setting with independent controls:

| Setting | Initial value | Behavior |
|---|---:|---|
| [`SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING`](../../../apps/scriptcraft/src/script_view.rs:17) | 0 logical points | First page starts flush with the toolbar-side viewport edge at the initial/top scroll position. |
| [`SCRIPT_VIEW_BOTTOM_PAGE_EDGE_PADDING`](../../../apps/scriptcraft/src/script_view.rs:18) | Preserve the current 4 logical points | Retain bottom page-end allowance beyond the last page. |

Actual content margin and prepaint zoom geometry use independent top and bottom values. The existing inter-page gap, trailing page gap, horizontal padding, page scale, and fade policy remain unchanged.

## 4. Default window geometry baseline findings

### 4.1 Native startup before implementation

At baseline, [`native_options()`](../../../apps/scriptcraft/src/main.rs:1358) configured native chrome but did not call [`ViewportBuilder::with_inner_size()`](../../../apps/scriptcraft/src/main.rs:1360) or [`ViewportBuilder::with_position()`](../../../apps/scriptcraft/src/main.rs:1360). egui therefore left initial dimensions to platform-specific defaults.

The existing capture/restore mapping is clear:

- [`window_state_from_rects()`](../../../apps/scriptcraft/src/main.rs:1311) maps FDX Left/Top to the native outer window origin and Width/Height to native inner dimensions.
- [`window_restore_commands()`](../../../apps/scriptcraft/src/main.rs:1331) restores those values through outer-position and inner-size viewport commands.

Startup should use that same mapping rather than treating FDX Width/Height as outer-frame dimensions.

### 4.2 FDX fallback before implementation

At baseline, [`WindowState::DEFAULT`](../../../crates/scriptcraft-core/src/fdx.rs:277) contained Left 0, Top 95, Width 870, and Height 854. It was used by generated save context and as the desktop save fallback when no actual geometry snapshot was available.

The requested common default is:

| Field | Requested value |
|---|---:|
| Height | 874 |
| Left | 0 |
| Mode | Normal |
| Top | 75 |
| Width | 870 |

The submitted implementation updates the single core default to Top 75 and Height 874 while retaining Left 0 and Width 870. [`native_options()`](../../../apps/scriptcraft/src/main.rs:1358) consumes that default for its initial outer position and inner size so native startup and FDX fallback do not drift.

### 4.3 TextState already matches

No change is needed to the requested TextState defaults:

- [`TextState::DEFAULT`](../../../crates/scriptcraft-core/src/text_state.rs:18) already uses Scaling 100 and a caret at element 0, byte offset 0.
- [`selection_to_flat_offsets()`](../../../crates/scriptcraft-core/src/text_state.rs:73) projects that default caret to Selection 0,0 for a new document.
- [`text_state_element()`](../../../crates/scriptcraft-core/src/fdx.rs:2633) already emits ShowInvisibles No.
- [`window_state_element()`](../../../crates/scriptcraft-core/src/fdx.rs:2619) already emits Mode Normal.

Imported projects must retain their existing one-shot native restore. New-document activation must not unexpectedly move or resize an already open window. Actual Save/Save As should continue to prefer the latest reported native geometry over the fallback.

## 5. Proposed sequential work orders

### WO-005 — Center and vertically align toolbar captions

- **Standalone draft:** [`WO-005-toolbar-caption-alignment.md`](work-orders/WO-005-toolbar-caption-alignment.md)

Create one common nominal caption rectangle for action Items and Elements, center both captions within it, and add a zero-initialized Elements-only vertical offset. Remove actual ComboBox height from nominal caption placement while preserving dropdown behavior and ordinary caption spacing.

### WO-006 — Independent Script View top and bottom margins

- **Standalone draft:** [`WO-006-script-view-asymmetric-page-edges.md`](work-orders/WO-006-script-view-asymmetric-page-edges.md)

Replace symmetric page-edge padding with independent top and bottom settings. Initialize top to zero and retain the current bottom value. Update actual scroll content and prepaint zoom calculations together.

### WO-007 — Shared startup and FDX default window geometry

- **Standalone draft:** [`WO-007-default-window-geometry.md`](work-orders/WO-007-default-window-geometry.md)

The implementation submission updates the existing FDX default and consumes it from native viewport construction for initial outer position and inner size. Imported restore, live-window capture, transactional saves, and the already-correct TextState behavior remain unchanged.

## 6. Durable-document policy

Each approved order should update only the lasting requirements and architecture affected by its behavior. Durable requirements and blueprints must remain independently reviewable and must not include work-order identifiers, implementation chronology, or numeric tuning/default values. Exact values and implementation handoffs belong only in this planning directory.

## 7. Non-goals

- No change to the ordinary toolbar caption gap, item widths, horizontal allocation, icon graphics, role options, popup behavior, or status bar.
- No reintroduction of Script View fades or fixed central-panel margins.
- No change to bottom page-end padding beyond preserving its current value under a new independent setting.
- No forced window repositioning on New or ordinary Save, and no change to imported one-shot restore behavior.
- No persisted application preference or new FDX field.
- No agent-run validation and no claim of native OS acceptance of requested window geometry.

## 8. WO-005 implementation-submission handoff

- **Branch:** `dev/2026-11-05-gui-spacing-and-scroll`; the existing checkout and all prior work were preserved.
- **Files changed for this submission:** [`toolbar.rs`](../../apps/scriptcraft/src/toolbar.rs:15), [`screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md:542), [`technical-context.md`](../../.forge/blueprints/technical-context.md:28), this overview, and the WO-005 draft. No WO-006 or WO-007 implementation was performed.
- **Exact source behavior and values:** `toolbar.rs` adds `TOOLBAR_ELEMENTS_CAPTION_VERTICAL_OFFSET = 0.0` and shared `toolbar_caption_rect(group_rect, nominal_control_height, control_caption_gap)`. Both action and Elements captions derive their rectangle from the nominal 18-point control slot and existing `TOOLBAR_CONTROL_CAPTION_GAP = 1.0`; the gap and all other user-adjusted toolbar values remain unchanged. Elements is centered in the common rectangle and only that caption receives the new vertical offset. Closed ComboBox response height is not used for nominal caption placement.
- **Resulting current geometry:** For the 36-point group, the common caption rectangle begins 19 points below the group top and ends at the group bottom (17 points high); its center is 27.5 points below the group top and at the group horizontal center. The Elements offset is initially zero.
- **Evidence limitations:** No agent-run build, check, test, lint, formatting command, manual verification, independent review, validator, or user validation was performed. This handoff submits the implementation for user review only; no acceptance is claimed. WO-006 and WO-007 remain untouched.

## 9. WO-006 implementation-submission handoff

- **Branch:** `dev/2026-11-05-gui-spacing-and-scroll`; the existing checkout and all prior work, including WO-005, were preserved. No branch or history operations were performed.
- **Files changed for this submission:** [`script_view.rs`](../../apps/scriptcraft/src/script_view.rs:17), [`screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md:198), [`technical-context.md`](../../.forge/blueprints/technical-context.md:136), this overview, and the [WO-006 draft](work-orders/WO-006-script-view-asymmetric-page-edges.md:1). WO-007 was not implemented or edited.
- **Exact source changes and values:** `script_view.rs` replaces `SCRIPT_VIEW_PAGE_EDGE_PADDING = 4.0` with `SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING = 0.0` and `SCRIPT_VIEW_BOTTOM_PAGE_EDGE_PADDING = 4.0`. The actual ScrollArea content margin uses zero horizontal margins and the independent top/bottom settings. Prepaint zoom geometry uses the top setting for page origin and adds top plus bottom once each to content extent. Page stride, inter-page spacing, horizontal padding, fades, page publication, and input behavior remain unchanged.
- **Resulting geometry:** The first page starts at the usable viewport edge at top scroll (zero top margin); the bottom keeps 4 logical points of content allowance after the existing page-row/trailing-gap extent. Prepaint zoom origin and extent use the same asymmetric geometry.
- **Evidence limitations:** No agent-run build, check, test, lint, formatting command, manual verification, independent review, validator, or user validation was performed. This handoff submits the implementation for user review only; no validation or acceptance is claimed. WO-007 remains untouched.

## 10. WO-007 implementation-submission handoff

- **Branch:** `dev/2026-11-05-gui-spacing-and-scroll`; the existing checkout and all earlier work, including WO-005 and WO-006, were preserved. No branch or history operations were performed.
- **Files changed for this submission:** [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs:277), [`main.rs`](../../apps/scriptcraft/src/main.rs:1358), [`screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md:399), [`technical-context.md`](../../.forge/blueprints/technical-context.md:184), and the [WO-007 draft](work-orders/WO-007-default-window-geometry.md:1), in addition to this overview.
- **Exact source changes and values:** `WindowState::DEFAULT` now supplies Left 0, Top 75, Width 870, and Height 874. Native viewport construction uses that value for outer position from Left/Top and inner size from Width/Height; there is no separate startup geometry source.
- **Resulting contract:** Fresh-process startup requests the shared native geometry and generated/fallback FDX state uses the identical typed default. Imported geometry retains one-shot restore; New-document window stability, actual live geometry capture, and transactional save behavior remain unchanged. TextState, Mode Normal, and ShowInvisibles remain unchanged.
- **Evidence limitations:** No agent-run build, check, test, lint, formatting command, manual verification, independent review, validator, or user validation was performed. Native OS acceptance is not established. This is an implementation-submission handoff for user review only; no validation or acceptance is claimed.

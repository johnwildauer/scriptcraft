# WordCraft document zoom: findings and an anchoring recipe

## Scope and quick conclusion

This note follows the document canvas zoom path, not print-preview zoom or interface-size scaling.
**Observed fact:** WordCraft explicitly anchors **Ctrl/Command + wheel zoom** to the page point beneath
the pointer. It records that point in page/document coordinates, computes where the same point lands
after changing scale, and requests a corrected scroll offset. It does **not** apply that correction to
every zoom control: the ribbon, dialog, and status-bar slider change the zoom value without capturing
an anchor. Thus the pointer-wheel path is the direct example to borrow for the screenplay app's
“stay under the cursor” behavior; toolbar zoom needs an anchor policy added if it should behave the
same way.

The central implementation is [`prepare_view()`](../crates/ui-egui/src/canvas.rs:496), with page
coordinate conversion in [`page_at()`](../crates/ui-egui/src/canvas/navigation.rs:5) and scroll
application in [`configure_scroll_area()`](../crates/ui-egui/src/canvas.rs:675). A regression test
for the pointer anchor is in [`ctrl_wheel_zoom_keeps_the_place_under_the_pointer()`](../crates/ui-egui/src/tests.rs:1113).

## 1. Where zoom state lives and how it is saved

**Observed facts:**

- The engine's [`ViewState`](../crates/engine/src/view.rs:4) owns `zoom: f32` as a multiplicative
  factor: the default is `1.0` (100%). The adjacent `fit: String` stores a fit mode, while
  `read_zoom: f32` is a separate, transient Read Mode text-size setting. `web_width`, reading
  dimensions, and reading text-size settings are marked not to serialize as normal view/document
  state ([`ViewState` fields and defaults](../crates/engine/src/view.rs:49)).
- The normal manual zoom range is 10–500%. The command validates that interval in
  [`zoom()`](../crates/engine/src/cmd/view.rs:289); the geometry also clamps the manual factor to
  0.1–5.0 in [`geometry()`](../crates/ui-egui/src/canvas/geometry.rs:166). Zoom In/Out steps by
  10 percentage points, rounded to the nearest 10-point boundary before stepping, and stops at
  10% / 500% ([`view.zoomIn` and `view.zoomOut`](../crates/engine/src/cmd/view.rs:76)).
- Each open window's engine [`WindowView`](../crates/engine/src/session/views.rs:12) contains its
  own `ViewState`, so zoom differs between windows. The canvas `scale` is the UI's current effective
  scale; it is recalculated from view state and fit geometry each frame
  ([`CanvasState` zoom/scroll fields](../crates/ui-egui/src/canvas.rs:193),
  [`prepare_view()`](../crates/ui-egui/src/canvas.rs:521)).
- Zoom is **not a user-interface preference**. On save, [`Session::stamp_save()`](../crates/engine/src/session/history.rs:83)
  copies the current zoom/fit into optional [`Document::settings.zoom`](../crates/doc/src/settings.rs:152).
  Opening a document restores it in [`file.open`](../crates/engine/src/cmd/file.rs:284). DOCX
  reads/writes this as `w:zoom` and clamps stored percentages to 10–500
  ([DOCX zoom reader](../crates/docx/src/read/meta.rs:84),
  [DOCX zoom writer](../crates/docx/src/write/settings.rs:27)). `None` represents ordinary 100%
  with no fit mode. Changing zoom alone is not a document edit; save stamps the current view setting.
- The canvas scroll area has a separate egui scroll state keyed with the document ID
  ([`configure_scroll_area()`](../crates/ui-egui/src/canvas.rs:681)). Its saved offset is not the
  zoom state and is not written into the DOCX zoom setting.

Fit modes supported by the engine include One Page, Page Width, Multiple Pages, and Text Width.
The on-screen Zoom dialog exposes the first three fit choices, plus a logarithmic 10–500% slider
with 1% steps and 200%, 100%, and 75% radio presets
([`zoom_dialog()`](../crates/ui-egui/src/dialogs/dispatch.rs:837)). Text Width is preserved/restored
by document zoom state, but is not one of those dialog buttons
([`ZoomFit`](../crates/doc/src/settings.rs:167), [`restore_zoom()`](../crates/engine/src/cmd/view.rs:275)).

## 2. Controls and input paths

- **View ribbon:** Zoom opens the dialog; 100%, Zoom In, Zoom Out, One Page, Multiple Pages, and
  Page Width route to `view.zoom100`, `view.zoomIn`, `view.zoomOut`, `view.onePage`,
  `view.multiplePages`, and `view.pageWidth` respectively
  ([View ribbon Zoom group](../crates/ui-egui/src/ribbon/view.rs:48),
  [command registrations](../crates/engine/src/cmd/view.rs:73)).
- **Status bar:** clicking the displayed percentage opens the Zoom dialog; minus/plus call the
  10-point step commands; the slider is logarithmic from 10% to 500% and calls `view.zoom` with
  the rounded percentage ([status-bar controls](../crates/ui-egui/src/chrome.rs:708)). In Read Mode,
  these controls instead change the independent `view.readTextSize` setting, 75–150%.
- **Keytips / menu-like route:** the legacy `VZ` sequence opens the Zoom dialog
  ([legacy keytips](../crates/ui-egui/src/keytips.rs:98)). The zoom command registrations have no
  dedicated keyboard shortcut. WordCraft also disables egui's keyboard zoom because it would scale
  the entire interface and collide with document shortcuts
  ([keyboard zoom disabled](../crates/ui-egui/src/app/fonts.rs:191)).
- **Ctrl/Command + wheel:** over the canvas, the configured zoom modifier turns wheel movement into
  a zoom delta. With animation effects on, egui's `zoom_delta()` supplies the eased change; with
  effects off, [`CanvasScroll::zoom()`](../crates/ui-egui/src/scroll.rs:265) collects movement into
  logarithmic steps of about 10% (`exp(0.1)` per step). A partial step expires after 0.75 seconds.
  Starting a zoom cancels queued scrolling/momentum so the scroll correction is not combined with
  leftover wheel motion ([`CanvasScroll::frame()`](../crates/ui-egui/src/scroll.rs:169)).
- **Trackpad / pinch:** egui's zoom event and multi-touch zoom delta use the same canvas zoom path.
  Pinch input is not step-quantized, including when animation effects are off
  ([`CanvasScroll::zoom()`](../crates/ui-egui/src/scroll.rs:269)). The canvas only captures a pointer
  anchor when the pointer is within its area and a latest pointer position is available; Read Mode
  does not use this page-point anchor ([`prepare_view()`](../crates/ui-egui/src/canvas.rs:524)).

There is no separate Ctrl/Command-plus/minus document zoom shortcut registered here; egui's own
keyboard-based interface zoom is explicitly turned off. The keyboard-accessible `VZ` route is a
dialog shortcut, not a zoom increment.

## 3. Layout versus display scaling

In Print Layout, zoom does not repaginate the document. The engine's
[`Session::layout()`](../crates/engine/src/session/layout.rs:7) builds a `DocLayout` from document
layout inputs; normal Print Layout does not include `view.zoom` in its cache key
([`layout_with()`](../crates/engine/src/session/layout.rs:15)). The UI then places the already
laid-out pages at a different scale. In [`geometry()`](../crates/ui-egui/src/canvas/geometry.rs:166),
manual zoom makes `scale = zoom × PX_PER_PT`; [`place_pages()`](../crates/ui-egui/src/canvas/geometry.rs:136)
multiplies page dimensions by that scale to make page rectangles. Rendering uses a raster scale
derived from `geo.scale × pixels_per_point` and redraws/caches page textures at that resolution
([`paint_pages()`](../crates/ui-egui/src/canvas.rs:881)). Glyphs and page content keep their
document-point layout; their painted size changes.

There are two meaningful exceptions to “only a view transform”:

- **Web Layout** reflows text to the available window width in document points. The UI derives that
  width from viewport width divided by zoom and updates `web_width` when zoom changes or a resize
  settles ([`update_viewport()`](../crates/ui-egui/src/canvas.rs:457)); `Session::layout()` uses
  `web_width` for Web Layout ([`layout_with()`](../crates/engine/src/session/layout.rs:15)).
- **Read Mode** has its own text-size and responsive-column presentation; its layout inputs include
  reading width/height and line spacing. It is not the regular page-zoom path
  ([`ViewState`](../crates/engine/src/view.rs:59),
  [`update_viewport()`](../crates/ui-egui/src/canvas.rs:436)).

Even in Print Layout, the *page arrangement* may change as zoom changes: `geometry()` can place
multiple pages in a row when they fit, except that One Page/Page Width keep a single column
([`geometry()`](../crates/ui-egui/src/canvas/geometry.rs:193)). This is page positioning, not text
repagination.

## 4. The exact scroll-anchor math

**Observed mechanism:** page-point-under-pointer anchoring, not viewport-center anchoring, not
caret/page recentering, and not a raw `old_scroll × zoom_ratio` adjustment.

1. Before changing zoom, [`prepare_view()`](../crates/ui-egui/src/canvas.rs:524) checks for a wheel or
   pinch zoom. If the pointer is in the canvas, it calls [`page_at()`](../crates/ui-egui/src/canvas/navigation.rs:5)
   with the **old** page rectangles and scale. `page_at()` returns the nearest page plus the
   pointer's `(x, y)` in that page's document-point coordinates: `(pointer - page_rect.min) / old_scale`.
2. The zoom command updates the factor. WordCraft computes new page geometry, then converts the same
   page-point coordinate using the **new** page rectangle and its effective scale.
3. It requests the scroll offset that makes that new content point remain at the same location
   relative to the viewport:

   ```text
   anchor_screen = pointer_screen - viewport_screen_origin
   anchor_content_after = new_page_origin + (anchor_x, anchor_y) * new_page_scale
   requested_scroll = anchor_content_after - anchor_screen
   ```

   This is the canvas code's [`scroll_request` calculation](../crates/ui-egui/src/canvas.rs:546).
   It is the page-aware form of `new_scroll = anchor_doc * new_zoom - anchor_screen`; the page
   origin term matters because pages may be centered or rearranged in rows.
4. [`configure_scroll_area()`](../crates/ui-egui/src/canvas.rs:691) clamps that request to
   `[0, scroll_max]` and passes it to egui as the scroll offset. `scroll_max` is recomputed from new
   content size minus viewport size. When zoom changes the number of page columns, the explicit
   anchor suppresses the fallback “scroll caret into view” behavior
   ([`prepare_view()`](../crates/ui-egui/src/canvas.rs:546)).

The test zooms while scrolled several pages down and checks that the original page point remains
within about two screen points, even while pages reflow into rows
([anchor regression test](../crates/ui-egui/src/tests.rs:1113)). When a layout change makes the
same horizontal page position impossible (for example, the pages now fit side by side and the
horizontal scroll range disappears), the test allows that horizontal constraint; vertical page
position remains anchored.

**Important limit:** the correction above is created only from pointer-based wheel/pinch input. A
toolbar, ribbon, dialog, or status slider command has no captured anchor. In that path egui retains
its scroll-area state and clamps it if content bounds shrink; if page columns change,
WordCraft may scroll the caret back into view. This is not an implicit guarantee that the same
document point remains under the viewport center. For a screenplay app that wants stable toolbar
zoom too, capture an explicit center or cursor anchor before applying the command.

## 5. Smoothness and per-frame behavior

There is no WordCraft-level `target_zoom` that eases toward a separate `current_zoom`. The command
sets `ViewState.zoom`; `prepare_view()` reads the input delta, applies the command, and computes new
geometry in the same UI frame. With animation effects enabled, egui may supply several smaller
wheel zoom deltas over subsequent frames; WordCraft repeats the anchor capture and correction for
each delta, so each intermediate step preserves the point currently under the pointer. With effects
off, wheel zoom lands in quantized steps immediately; pinch stays continuous. The
`zoom_pending` field in [`CanvasScroll`](../crates/ui-egui/src/scroll.rs:151) is only a buffer for
partial wheel steps, not an animation target. Scroll-area caret/navigation animation is separate
([`configure_scroll_area()`](../crates/ui-egui/src/canvas.rs:681)).

## 6. Coordinate systems and DPI

Keep three units separate:

1. **Document coordinates:** page layout and text positions are typographic points.
2. **Canvas coordinates:** egui uses logical points. WordCraft's `PX_PER_PT` is named in “px” but
   is actually the base ratio `96 / 72`: at 100% document zoom, one document point becomes about
   1.333 egui points. Document zoom multiplies this ratio
   ([`PX_PER_PT`](../crates/ui-egui/src/canvas.rs:81),
   [`geometry()`](../crates/ui-egui/src/canvas/geometry.rs:171)).
3. **Physical display pixels:** multiply canvas logical points by egui's
   `pixels_per_point`. The renderer does this when choosing raster density
   ([`paint_pages()`](../crates/ui-egui/src/canvas.rs:891)). Native display DPI and the separate
   **interface-size** setting affect egui's point-to-pixel scale, not `ViewState.zoom`
   ([`InterfaceSize`](../crates/ui-egui/src/theme.rs:82),
   [`WordApp` applies egui's zoom factor](../crates/ui-egui/src/app.rs:280)).

So the useful conversion for a document point is:

```text
screen logical points = page_origin + document_point * (document_zoom * 96 / 72)
physical pixels       = screen logical points * egui_pixels_per_point
```

Do not apply monitor DPI or interface zoom a second time to the document zoom. Keep the anchor
equation in one coordinate space (preferably egui logical points), and use `pixels_per_point` only
when rasterizing or aligning to physical pixels.

## 7. Minimal recipe for the screenplay app

For a simple canvas with no per-page placement offset, let `anchor_screen` be the pointer inside the
viewport (or the viewport center). Before changing zoom, convert that viewport point to document
coordinates; after changing zoom, solve for the scroll offset that restores it:

```text
anchor_doc = (old_scroll + anchor_screen) / old_zoom
new_zoom = clamp(requested_zoom, min_zoom, max_zoom)
new_scroll = anchor_doc * new_zoom - anchor_screen
new_scroll = clamp(new_scroll, 0, new_content_size - viewport_size)
```

If pages have their own origins or the zoom scale includes a base conversion factor, use the
page-aware version:

```text
anchor_doc = (anchor_screen + old_scroll - old_page_origin) / old_page_scale
new_scroll = new_page_origin + anchor_doc * new_page_scale - anchor_screen
```

Tiny implementation outline (all positions and scroll values below are in the same logical-point
space):

```text
anchor_screen = pointer_inside_canvas ? pointer - viewport_origin : viewport_size / 2
anchor_doc = (scroll + anchor_screen - old_page_origin) / old_page_scale
zoom = clamp(requested_zoom, min_zoom, max_zoom)
layout_or_place_pages_for(zoom)
new_scroll = new_page_origin + anchor_doc * new_page_scale - anchor_screen
scroll = clamp(new_scroll, 0, content_size - viewport_size)
```

The essential sequence is **capture → update zoom/layout → recompute content geometry → solve scroll
→ clamp → apply scroll**. If the app has no page layout, the shorter formula in the first block is
enough. For a screenplay editor, preserve the logical document point (page + local point, or a
stable script location if pagination can change), not a raw screen pixel.

## 8. Edge cases worth copying

- **Clamping:** manual zoom is 10–500%; Read Mode text size is 75–150%. Clamp both requested zoom
  and the final scroll offset. The canvas calculates `scroll_max` from the resized content and
  viewport, then clamps explicit requests ([`configure_scroll_area()`](../crates/ui-egui/src/canvas.rs:691)).
- **Fit modes:** Page Width, One Page, and Multiple Pages calculate a scale from available canvas
  dimensions in [`geometry()`](../crates/ui-egui/src/canvas/geometry.rs:181). Because geometry is
  recomputed from the current available size, the fit scale responds to window-size changes. Page
  Width and One Page keep one page per row; Multiple Pages requests at least two columns.
- **Manual zoom after fit:** switching from One Page/Page Width to a percentage clears the fit
  mode but retains a one-page-per-row arrangement. Zoom In/Out starts from the actual displayed fit
  scale because [`sync_fit_zoom()`](../crates/ui-egui/src/canvas/geometry.rs:230) copies that scale
  back into the session view.
- **Web Layout:** a resize can change line wrapping. WordCraft waits for the resize to settle before
  changing its `web_width`, then scrolls the caret back into view rather than promising that an old
  pixel offset maps to the same text ([`update_viewport()`](../crates/ui-egui/src/canvas.rs:457)).
- **Page-row changes and bounds:** a pointer anchor is best-effort at content edges and when pages
  rearrange: the final offset must remain inside the new scroll range. Suppress any automatic
  caret-centering behavior when an explicit anchor request is active, as WordCraft does.
- **Read Mode:** it uses a different 75–150% range, reading columns, and its own resize/reflow
  behavior; do not assume the normal page-zoom anchor applies there.

## Apply this to your app

- [ ] Keep zoom separate from DPI/interface scaling; name the units for document, logical canvas,
      and physical display coordinates.
- [ ] Before zoom, capture the pointer or viewport-center position as a document/page coordinate.
- [ ] Apply the new zoom, recompute layout/placement, then solve `new_scroll = anchor_doc * new_zoom - anchor_screen` (including page origins when present).
- [ ] Clamp to the new scroll bounds and prevent caret auto-scroll from overriding the anchor.
- [ ] Use the same anchor policy for toolbar and slider zoom if they should feel as stable as wheel zoom.
- [ ] Test deep-scroll zoom-in and zoom-out, cursor versus center anchoring, page/column reflow, and
      zoom near all four content edges.

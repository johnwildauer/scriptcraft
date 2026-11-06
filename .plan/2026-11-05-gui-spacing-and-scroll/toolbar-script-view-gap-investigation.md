# Remaining toolbar-to-Script View gap investigation

- **Status:** Source findings complete; runtime rectangle owner unresolved pending user-captured trace
- **Date:** 2026-11-06
- **Parent investigation:** [`boundary-and-border-investigation.md`](boundary-and-border-investigation.md)
- **Evidence:** User-supplied screenshot after WO-008 and WO-009 implementation submissions
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting commands, manual verification, reviews, or independent validators

## 1. Screenshot interpretation

The screenshot shows a workspace-colored band between the toolbar's lower edge and the first page. The page is clipped at the bottom of that band, and the visible vertical scrollbar also begins near the page's top rather than at the toolbar's lower edge.

The band is approximately 16 screenshot pixels tall. If the screenshot uses the previously discussed effective 2.0 pixels-per-point scale, that corresponds to roughly 8 logical points. This is a visual estimate, not a runtime rectangle measurement.

The shared page/scrollbar top is important: it points to the Script View viewport origin or an outer allocation handoff, not merely page painting or the page border.

## 2. Source paths eliminated by inspection

| Path | Current source finding |
|---|---|
| Toolbar trailing padding | [`TOOLBAR_BOTTOM_PADDING`](../../../apps/scriptcraft/src/toolbar.rs:19) is now zero and remains included in exact panel height through [`toolbar_content_height()`](../../../apps/scriptcraft/src/toolbar.rs:1012). |
| Toolbar framework separator | Disabled through [`show_separator_line(false)`](../../../apps/scriptcraft/src/main.rs:1067). |
| Toolbar frame margin | [`chrome_frame()`](../../../apps/scriptcraft/src/main.rs:1043) uses [`Frame::NONE`](../../../apps/scriptcraft/src/main.rs:1046). |
| Script Workspace frame margin | [`central_panel_frame()`](../../../apps/scriptcraft/src/script_view.rs:1476) uses [`Frame::NONE`](../../../apps/scriptcraft/src/script_view.rs:1481). |
| Script View top content margin | [`SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING`](../../../apps/scriptcraft/src/script_view.rs:17) is zero and is used explicitly in [`content_margin()`](../../../apps/scriptcraft/src/script_view.rs:401). |
| Eframe root UI margin | The installed eframe [`App::ui()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/eframe-0.34.3/src/epi.rs:176) contract explicitly says the supplied UI has no margin or background. |
| CentralPanel frame bypass | [`CentralPanel::frame()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.34.3/src/containers/panel.rs:1005) stores the supplied frame, and its show path uses that frame rather than the default. |
| Scrollbar vertical style inset | The installed scroll painter uses the scroll inner rectangle's vertical range directly; its bar margins affect cross-axis placement, not an additional top content margin. |
| Alternate application editor path | The only ordinary live-editor composition is [`show_application_panels()`](../../../apps/scriptcraft/src/main.rs:1051). The migration-modal branch uses a default central panel but does not show the live document surface. |

No checked-in source setting currently explains an additional fixed band between these panels. In particular, the approximate 8-point visual size resembles egui's old default central-panel margin, but the active code explicitly replaces that frame with a margin-free one.

## 3. Remaining possibilities

1. The toolbar response rectangle extends below its visibly painted Item content, so the central panel's available origin is lower than expected.
2. The central panel receives the expected origin but its child/scroll viewport begins lower because of runtime UI state or layout behavior not apparent from constants alone.
3. The scroll inner origin is correct but the first allocated page row is offset inside the scroll child.
4. The running executable does not contain the current source values or frame construction. This must be established by runtime evidence rather than assumed.

Replacing the central panel with explicit rectangles or adding a negative offset before measuring would conceal which of these is true and could break scrolling, panel resizing, geometry publication, or zoom compensation.

## 4. Proposed WO-010 — One-frame boundary geometry trace

- **Standalone draft:** [`WO-010-boundary-geometry-trace.md`](work-orders/WO-010-boundary-geometry-trace.md)

Add a development-only, production-off boundary trace without changing layout or paint behavior.

### Required measurements at the top scroll position

- Root available rectangle before toolbar allocation.
- Actual top toolbar response rectangle.
- Parent available rectangle immediately before central-panel construction.
- Actual central-panel response rectangle.
- Central child available and clip rectangles immediately before Script View scroll construction.
- Actual scroll outer and inner rectangles and committed offset.
- First allocated page rectangle and its clip.
- Effective pixels-per-point and the current top/bottom content-margin settings.

### Derived deltas

- Central origin minus toolbar bottom.
- Scroll inner origin minus toolbar bottom.
- First page origin minus scroll inner origin.
- First page origin minus toolbar bottom.

These values contain geometry only: no screenplay text, paths, source identities, or document values.

### Instrumentation constraints

- Gate the trace behind a manual source constant defaulting to disabled.
- Emit only on startup/top-scroll geometry changes or another bounded development trigger, not an unbounded every-frame log stream.
- Do not alter allocation, clipping, paint order, persistent scroll identity, or user state.
- Keep the trace out of durable product requirements and default Diagnostics presentation unless separately approved.
- The user runs the application, captures the trace at the visible gap, and supplies the output for a targeted correction.

## 5. Correction decision rule

- **Evidence-gated draft:** [`WO-011-boundary-owner-correction.md`](work-orders/WO-011-boundary-owner-correction.md)

- If the central origin is already below the toolbar bottom, investigate the top-panel response/root cursor handoff in [`main.rs`](../../../apps/scriptcraft/src/main.rs).
- If the central origin is correct but the scroll inner origin is lower, investigate the central frame/child UI handoff.
- If the scroll inner origin is correct but the first page is lower, investigate the first page-row allocation inside [`live_document_surface()`](../../../apps/scriptcraft/src/script_view.rs:358).
- If traced constants or frame behavior do not match current source, establish the actual running binary/build path before making more source edits.

A subsequent correction order should modify only the owning layer identified by this trace. No further numeric spacing change is justified before that evidence exists.

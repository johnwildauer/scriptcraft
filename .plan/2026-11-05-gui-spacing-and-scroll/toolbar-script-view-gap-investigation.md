# Remaining toolbar-to-Script View gap investigation

- **Status:** Source correction implemented; user runtime review pending
- **Date:** 2026-11-06
- **Parent investigation:** [`boundary-and-border-investigation.md`](boundary-and-border-investigation.md)
- **Evidence:** User-supplied screenshot after WO-008 and WO-009 implementation submissions
- **Region model:** [`window-region-allocation.md`](window-region-allocation.md) maps toolbar/status/central anchoring, response-driven sizing, and the optional explicit three-region architecture.
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting commands, manual verification, reviews, or independent validators
- **Standalone continuation:** [`toolbar-script-view-gap-continuation.md`](toolbar-script-view-gap-continuation.md) consolidates the current residual-gap state, evidence, layout model, and recommended next work order.

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

The active central frame and top scroll margin remain margin-free. Source inspection instead identifies an implicit toolbar response extent: the explicit top-padding allocation, Item-group allocation, and bottom-padding allocation are children of an outer top-down scope using normal vertical item spacing. The layout adds implicit gaps between those children, extending the toolbar's returned content minimum beyond its explicit visible composition. The top panel uses that response rectangle to advance its parent cursor, while the toolbar divider is painted at the panel UI's available lower edge. The extra response tail can therefore position Script View after an unpainted band.

## 3. Remaining possibilities

1. The toolbar response rectangle extends below its visibly painted panel composition because outer implicit vertical item spacing inflates its content extent. Source inspection supports this owner.
2. The central panel receives the expected origin but its child/scroll viewport begins lower because of runtime UI state or layout behavior not apparent from constants alone.
3. The scroll inner origin is correct but the first allocated page row is offset inside the scroll child.
4. The running executable does not contain the current source values or frame construction. This must be established by runtime evidence rather than assumed.

Replacing the central panel with explicit rectangles or adding a negative offset before measuring would conceal which of these is true and could break scrolling, panel resizing, geometry publication, or zoom compensation.

## 4. WO-010 — One-frame boundary geometry trace (completed and retired)

- **Standalone draft:** [`WO-010-boundary-geometry-trace.md`](work-orders/WO-010-boundary-geometry-trace.md)

WO-010's development-only, production-off boundary trace was captured and is now retired. The original requirements below describe the completed historical instrumentation, not code that remains in the checkout.

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

## 5. WO-010 instrumentation handoff

WO-010 was implemented on branch `dev/2026-11-06-script-view-boundary`, based on clean baseline commit `728282270489822770fea5141ddb3595aad340e9`. The user supplied a four-snapshot capture summarized below. After that capture was complete, the development instrumentation was removed. The runtime visual boundary owner remains subject to user review; the capture records allocations and clips, not painted pixels.

### Changed files

- Temporary [`boundary_trace.rs`](work-orders/WO-010-boundary-geometry-trace.md#instrumentation-retirement) module — development-only trace gate, bounded change detection, and fixed-label numeric output; deleted after capture.
- [`apps/scriptcraft/src/main.rs`](../../apps/scriptcraft/src/main.rs) — root available, actual toolbar response, pre-central parent available, and central response captures.
- [`apps/scriptcraft/src/script_view.rs`](../../apps/scriptcraft/src/script_view.rs) — child available/clip, actual scroll outer/inner rectangles and offset, first allocated page and its clipped portion, pixels-per-point, and configured top/bottom margins.
- [`work-orders/WO-010-boundary-geometry-trace.md`](work-orders/WO-010-boundary-geometry-trace.md) — implementation record and capture request.
- This investigation note.

Historically, a source gate defaulted to false. When manually enabled, output consisted of numeric `x,y,width,height` rectangles in egui points; committed scroll offset; effective pixels-per-point; top/bottom margin settings; and vertical-origin deltas for central-minus-toolbar-bottom, scroll-inner-minus-toolbar-bottom, first-page-minus-scroll-inner, and first-page-minus-toolbar-bottom. The scroll inner rectangle was derived from returned content size capped to the allocated viewport, and its outer rectangle added the existing allocated scrollbar extent. Each fixed-label WO010 line went to stderr. Identical snapshots were suppressed, snapshots were emitted only at a near-zero vertical scroll offset (within 0.5 points), and output was capped at 16 distinct lines per application thread. This instrumentation is no longer present.

### User runtime capture

**Completed.** The user supplied four distinct top-offset snapshots, and the temporary gate was returned to its disabled state before the source instrumentation was retired. No agent build/check/test/lint/format/manual verification/review/validator was run for this capture or retirement. No runtime owner is claimed from source-only reasoning; the supplied capture remains allocation/clip evidence only.

### User-supplied capture and interpretation

The user supplied four distinct top-offset snapshots. At both stable viewport heights (root height 874 and 842 points), the toolbar bottom, central-panel top, scroll-inner top, and first-page top are y=50 points; all four reported vertical deltas are zero. At root height 842, one intermediate snapshot places the allocated page top at y=39 points, with its clip beginning at y=50 and the two page-origin deltas at -11 points; the subsequent snapshot returns the page to y=50. Effective scale is 2 pixels per point and configured margins are top 0/bottom 4 points. These capture details are retained in WO-010; the user-provided evidence has not been edited.

This capture does not show a persistent positive allocation offset at the sampled stable states. The one-frame negative page offset is transient in the supplied sequence. The trace measures allocations and clips, not the toolbar's painted lower edge or content UI minimum; the task supplied a summary rather than raw trace lines, and no simultaneous paint-boundary measurement was supplied. Source inspection explains how the response can exceed the painted composition despite contiguous traced response/central origins. This supports the source correction but does not establish runtime confirmation; user review remains pending.

No durable requirements or blueprints were changed for this development-only instrumentation.

## 6. WO-011 source correction and evidence limits

- **Implementation record:** [`WO-011-boundary-owner-correction.md`](work-orders/WO-011-boundary-owner-correction.md)

The actual source owner is the outer vertical allocation in [`toolbar()`](../../apps/scriptcraft/src/toolbar.rs:840): its explicit top padding, Item-group allocation, and bottom padding were separated by implicit vertical item spacing. The layout's expanded content response was returned through the top panel and used to advance the parent cursor, but the toolbar divider remained at the explicit panel edge. The correction disables vertical item spacing only in that outer scope and restores the captured normal spacing inside the Item-group child UI, preserving role and overflow popup row gaps and all explicit toolbar geometry.

The stable user-supplied WO-010 summary reports the toolbar response bottom, central top, scroll-inner top, and first-page top at the same vertical coordinate with zero deltas. Those are allocation boundaries, not a paint measurement, so they neither contradict nor runtime-confirm the source-level response-versus-paint explanation. The supplied summary omits raw trace lines, and no post-correction runtime evidence exists. The temporary instrumentation was retired after capture and is no longer present. No build, check, test, lint, formatting command, manual verification, reviewer, or validator was run for this cleanup; runtime review belongs to the user.

## 7. Later source-state reconciliation — 2026-11-06

The current source subsequently diverged from several earlier snapshot statements in this investigation: [`TOOLBAR_BOTTOM_PADDING`](../../apps/scriptcraft/src/toolbar.rs:18) is 4 points, not zero. The user directed that value be preserved and the durable toolbar-height contract revised to allow explicitly owned trailing padding. This padding is not the remaining response-overflow owner because it is included in both configured height and explicit toolbar content.

The later [`WO-012 findings`](../2026-11-06-toolbar-script-view-gap/toolbar-script-view-gap-findings.md) identify an additional closed-Elements spacing contribution: the ComboBox was allocated before vertical Item spacing was suppressed, and the explicit caption allocations then materialized the already-advanced cursor into the Elements group's returned content extent. WO-012 now sets the closed-control spacing before ComboBox allocation and restores normal spacing inside popup role rows. This source change and the durable requirement/blueprint updates are submitted for user validation; no runtime result is claimed. See the standalone [`WO-012 handoff`](../2026-11-06-toolbar-script-view-gap/work-orders/WO-012-elements-spacing-overflow.md).

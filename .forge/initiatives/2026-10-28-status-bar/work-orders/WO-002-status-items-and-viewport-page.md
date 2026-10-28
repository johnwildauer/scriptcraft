# Work Order: WO-002 — Status Items and Viewport Page

## Metadata

- **Status:** `draft`
- **Assignee:** Desktop application implementer (one fresh session; not dispatched)
- **Phase/order:** 2 of 3; depends on WO-001; precedes integrated zoom regression in WO-003
- **Initiative:** [`status-bar-v1.md`](../status-bar-v1.md)
- **Generated filename:** [`WO-002-status-items-and-viewport-page.md`](WO-002-status-items-and-viewport-page.md)
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`validation-WO-002.md`](../validations/validation-WO-002.md) (future artifact; not created)
- **Fixes:** none
- **Implementer session/agent ID:** Not assigned; no dispatch authorized
- **Session exception:** none

## Summary

Create the desktop-owned item-based status bar and viewport-page presentation. Support noninteractive text, clickable text and clickable icons as capabilities without inventing actions/icons; provide truthful center clean/unsaved, cumulative frames, path, operation/error status and safe diagnostic access with a deterministic narrow-width fallback. The left page item reports the currently visible rendered sheet from actual page geometry and the editor's actual scroll clip—not caret position—and never presents stale page information as current.

## In scope

- Consume page rectangles and clip/scroll geometry produced by the editor's current `ScreenplayLayout` rendering path in [`main.rs`](../../../../apps/scriptcraft/src/main.rs); do not estimate pages from caret position, page index under cursor, nominal viewport center, or fixed pixel thresholds.
- Add a discrete desktop-owned item-based status-bar model/renderer (for example, `status_bar.rs`) with noninteractive text, clickable text and clickable-icon item capabilities. Capabilities must not invent or instantiate an action/icon not requested by the user.
- Compose left, center and right regions. The left region contains the viewport-page item; the center contains current clean/unsaved state, cumulative frames, path and operation/error status with access to privacy-safe diagnostics. Leave right-region zoom behavior to WO-003, using a neutral/available slot without implementing zoom controls here.
- Read live `EditingSession::dirty()` and represent currently clean versus unsaved truthfully; a clean in-memory project does not imply it was previously saved. Preserve current error visibility, path and operation status, and access to bounded native-input/performance diagnostics, using compact disclosure where needed. Do not expose document/source text, serialized project data or sensitive diagnostic payloads.
- Provide deterministic bounded narrow-width fallback that retains access to page, clean/unsaved, frames, path, operation/error status and diagnostics. Interactive item capabilities have keyboard focus/activation and explicit accessible names; decorative/noninteractive text owns no competing response, focus or cursor.
- Define a testable desktop-owned page selector: for each rendered sheet, compute its vertical overlap with the actual editor scroll clip; choose the greatest overlap; exact overlap ties choose the smaller/earlier rendered page index.
- If the clip is in an inter-page gap and no page has positive overlap, select the nearest rendered page by vertical interval distance; exact distance ties choose the earlier page.
- Use one-based visible numbering and total count over **all rendered sheets**, including an optional title page plus body pages from `ScreenplayLayout.pages`. Keep title-page membership from `LayoutPage.title_page`; do not conflate title page with body caret/source-map indexing.
- Report an explicit unavailable state if the layout is unavailable/empty, rendered page geometry is invalid, clip geometry is invalid, or the geometry snapshot is stale. Never silently keep a previous `N of total` when it no longer describes current geometry.
- Keep page state current through passive vertical scroll, resize, presentation-scale/zoom changes, layout reflow, and conditional caret reveal. Account for the bottom status panel being laid out before the central editor: key/cache geometry by sufficient current layout revision, presentation scale/size, clip and scroll state; invalidate when inputs/events can make the last editor geometry stale, and request a repaint after fresh geometry is published. A bounded unavailable frame is preferable to a stale number.
- Ensure page status updates do not request caret reveal, move the viewport, steal editor focus, or change editor pointer/cursor/input ownership.
- Add item capability, state projection, error/path/diagnostic, accessibility/focus, narrow-fallback, page-selector and rendered-geometry regressions, including visible page differing from caret page and currentness across each named geometry change.

## Out of scope

- Toolbar extraction and preservation (WO-001).
- Zoom-control interaction and horizontal navigation implementation (WO-003); this WO must tolerate presentation-scale changes and keep its page result current.
- Changes to core layout/pagination, title-page content, source mappings, PDF/export geometry, page numbering inside the screenplay, caret reveal policy, editing/focus/input semantics, persisted data, or core crate APIs.
- Turning native smoke into a project hard-stop; none is configured in the current manifest.

## Requirements

Applicable existing requirements (verbatim):

- **REQ-WRITING-LOOP-002 / AC-WRITING-002.1:** “Given the same project and fixed page profile, the system shall derive stable page structure and source mappings independent of window zoom.”
- **REQ-WRITING-LOOP-002 / AC-WRITING-002.4:** “Given a document position and rendered geometry, the system shall support a safe caret/hit-test round trip at valid text boundaries.”
- **NFR-WRITING-001:** “Production paths should prefer typed errors and bounded inputs over panic-prone handling; a discovered crash or data-loss defect requires a regression test.”
- **NFR-WRITING-002:** “Core editing, layout, and persistence behavior shall be exercisable without a desktop window for deterministic tests.”

Initiative-confirmed behavior proposed for new/additive criteria in the future requirement update: text, clickable-text and clickable-icon item capabilities without invented actions/icons; truthful current clean/unsaved state, cumulative frames, path and operation status; visible errors and access to privacy-safe diagnostics; deterministic bounded narrow-width fallback; and viewport-visible page selection (not caret-based), where greatest vertical overlap against the actual editor scroll clip wins, ties go earlier, gaps select nearest/ties earlier, total includes a rendered title page, and stale/failed geometry is explicitly unavailable. These are not yet durable requirements.

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) — desktop owns item-based status presentation and viewport rendering; update for status capabilities/data ownership, narrow fallback, page selection/currentness and status-before-editor invalidation.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) — shared fixed-layout and source-map boundary; verify unchanged.
- [`technical-context.md`](../../../blueprints/technical-context.md) — desktop/core separation and presentation-only scale; verify unchanged.
- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) — layout pages/title page/source mapping remain core-owned and unchanged.

## Implementation plan

1. Re-read the approved WO-001 toolbar boundary, current status/footer and actual editor `ScrollArea`; define the status item model/renderer and three-region composition without changing the editor response/ID or core layout API.
2. Project truthful center state, preserving error visibility, path, operation status, cumulative frames and bounded privacy-safe diagnostics. Implement keyboard/accessibility behavior and deterministic bounded narrow fallback that preserves access to critical status and page information; do not implement zoom controls.
3. Add a pure/testable page selector over ordered page rectangles and clip interval. Use finite valid rect checks, overlap length, interval distance, stable earlier-page tie breaks, and explicit unavailable result; use checked/defensive indexing rather than panic-prone assumptions.
4. Publish editor geometry/currentness to the status adapter without making page rectangles independent widgets. Since status is rendered before editor, validate cached geometry against the current frame's viewport/layout/presentation/scroll invalidation signals; if uncertain, render unavailable until fresh geometry arrives and request repaint after publication.
5. Add deterministic tests for item capabilities, state projection, accessibility/focus, narrow fallback, failures, overlap/ties, gaps/ties, first/last bounds, title-plus-body numbering, off-screen caret, geometry failure and invalidation after scroll/resize/zoom/reflow/caret reveal. Verify page selection is observational only and does not mutate/recenter editor state.
6. Update only the authorized requirements and desktop-blueprint wording below; run focused and configured project checks and preserve comparable changed-file evidence.

## Files and systems

- **Create:** A discrete status-bar item/model/renderer module and pure desktop page-selector/currentness helpers or bounded submodule; tests for item behavior, status projection, selector/geometry and rendered status integration.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs) and the status-bar module for status composition and geometry publication/rendering; [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md); [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md).
- **Avoid changing:** `crates/scriptcraft-core/`, `ScreenplayLayout` page generation/source mapping, title-page serialization, editor input/focus/caret-reveal semantics, PDF, persistence/schema, and toolbar behavior.

## Validation and E2E acceptance tests

- **Automated checks:** [`cargo fmt --all -- --check`](../../../../Cargo.toml:1); `cargo test -p scriptcraft status_bar_`; `cargo test -p scriptcraft visible_page_`; `cargo test -p scriptcraft toolbar_`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`. These are future commands, not results from this planning session.
- **Focused status-item matrix:** Noninteractive text, clickable text and clickable icon remain capability types only, with no invented actions/icons; clean in-memory state is distinct from unsaved, save/failed save/open project truthful state/path/status, cumulative frames use the existing counter, errors remain visible, safe diagnostics remain reachable and contain no document text; keyboard activation/accessibility labels work for interactive items and decorative text remains noninteractive; narrow fallback preserves access to page and status/error/path/diagnostics.
- **Focused selector matrix:** Positive overlap selects maximum height; exact overlap tie chooses earliest page; pages with unequal overlap choose the greatest even if caret is elsewhere; clip in gap chooses nearest; equal gap distance chooses earlier page; clip above/below all sheets chooses nearest edge page; invalid/empty/no-layout geometry returns explicit unavailable; output is one-based with exact rendered-page count including optional title page. Test title-page sheet and first body sheet separately.
- **Rendered/currentness matrix:** Scroll to another page while caret stays elsewhere; put caret far offscreen and verify no caret-follow; then perform caret reveal and verify current page only after geometry is fresh; trigger resize, scale/zoom, reflow/revision and actual scroll input; on any mismatch show unavailable rather than previous page, then refresh to correct value; verify no page status event scrolls, focuses or edits the editor. Retain title/body page distinction and failure path.
- **Manual/external checks:** Recommended bounded desktop smoke at normal and narrow widths with a multi-page project and optional title sheet. Observe all three item capabilities as used without invented actions, center clean/unsaved/frame/path/operation/error and accessible safe diagnostics, narrow fallback, and page selection with caret on a different/off-screen page while scrolling across page boundaries/gaps, resizing and triggering caret reveal. Also observe zoom-change invalidation/currentness without implementing zoom controls. If window/input unavailable, record unavailable honestly; this is not a configured project hard-stop.
- **Acceptance test:** Given a clean/unsaved session or failed operation, when the item-based status renders, then it accurately represents current clean/unsaved state and cumulative frames, preserves path and operation status, visibly retains errors and safe diagnostic access, exposes only the three interaction capabilities without invented actions/icons, and remains keyboard-accessible/reachable at narrow widths. Given rendered pages and an actual editor clip, when one or more pages overlap, then status displays the one-based page with greatest vertical overlap and ties earlier; given a gap, then nearest-page/ties-earlier applies; given title page, it counts in total; given caret on a different page, the result follows viewport only; and given stale/failed geometry after any relevant viewport/layout change, status reports unavailable until current geometry is published rather than displaying stale `N of total`.
- **Independent validator boundary:** A separate fresh-context validator after implementation assesses only this WO, approved plan/review, source/test/diff evidence, durable updates and independently rerun commands. It must distinguish deterministic/rendered geometry evidence from actual native-window smoke; unavailable native observation is never a claimed pass.

## Durable-document impact

- **Requirements impact:** `update` — selected: add additive user-visible criteria for item capabilities, truthful center clean/unsaved and cumulative frame state, path/operation/error/diagnostic access, narrow fallback, and viewport-derived visible-page reporting with deterministic overlap/gap/title/unavailable behavior to [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md); preserve existing toolbar, fixed-page and source-mapping criteria.
- **Blueprint impact:** `update` — selected: record desktop-owned item-based status capabilities/data ownership, privacy-safe center presentation, narrow fallback, and page geometry publication/currentness, tie/gap behavior and status-before-editor stale-snapshot boundary in [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md).
- **Affected documents/IDs:** `REQ-WRITING-LOOP-002 / AC-WRITING-002.1, AC-WRITING-002.4` and `REQ-WRITING-LOOP-005 / AC-WRITING-005.1–005.5, AC-WRITING-005.7` (preserve verbatim); proposed additive status/page criteria under `screenplay-writing-loop.md` with stable IDs assigned during implementation/review; desktop status/page contract in `scriptcraft-desktop.md`.
- **Rationale and evidence:** Existing requirements guarantee toolbar behavior, zoom-independent fixed page structure and valid geometry mapping, but do not define item-based status or viewport-visible page reporting. Current `live_document_surface()` has actual page rectangles and actual scroll clipping, while bottom status renders first; desktop-specific currentness is needed without changing the core layout contract. Feature, technical and core blueprints remain unchanged if implementation stays app-owned.
- **Authorized documentation changes:** After review and execution authorization only, add the scoped status/page criteria and matching desktop-owned item/presentation/geometry-currentness contract. Preserve existing toolbar criteria; record feature/technical/core blueprints as verified unchanged. No durable document is modified now.

## Completion checklist

- [ ] Implementation is complete within scope.
- [ ] Tests/checks and required gates have evidence.
- [ ] Durable-document impact classification is accurate against the diff, and authorized documentation updates are applied.
- [ ] Execution log is appended with dispatch/completion session identifiers; fresh independent validation artifact exists.
- [ ] Work order is `in-review` with evidence; completion comes only from passed validation.

# WO-004 — Document-scoped visible-page retention

- **Status:** Implementation submitted on 2026-11-04; user review and validation pending; acceptance not recorded
- **Initiative:** [`ui-icons-v1.md`](../ui-icons-v1.md)
- **Depends on:** [`WO-003`](WO-003-status-priority-and-zoom-readout.md)
- **Validation owner:** User; no agent-run checks or independent reviewers/validators

## Outcome

Keep the last known valid visible-page value during transient same-document scroll, zoom, resize, reflow, and geometry-refresh frames. Replace it only when another valid value is available. Show Page unavailable only when the active document has no last known valid value, including startup and the period immediately following successful document replacement.

This changes the current explicit unavailable-on-stale contract in [`AC-WRITING-006.6`](../../../.forge/requirements/features/screenplay-writing-loop.md:628). It must not allow a page and total from a previous document to survive successful New or Open activation.

## Investigation findings

1. [`ViewportPageTracker`](../../../apps/scriptcraft/src/status_bar.rs:699) stores only a published geometry snapshot and a repaint flag. It has no independent last-valid display value, and [`ViewportPageTracker::invalidate()`](../../../apps/scriptcraft/src/status_bar.rs:834) clears the snapshot.
2. [`ViewportPageTracker::visible_page()`](../../../apps/scriptcraft/src/status_bar.rs:769) deliberately returns Unavailable for explicit invalidation, a missing snapshot, a non-adjacent frame, revision/viewport/scale mismatch, or empty geometry at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:777). Scroll and pointer input feed that invalidation through [`page_geometry_input_invalidated()`](../../../apps/scriptcraft/src/main.rs:2761) and the page query in [`show_application_panels()`](../../../apps/scriptcraft/src/main.rs:2500).
3. Status zoom invalidates the tracker at [`main.rs`](../../../apps/scriptcraft/src/main.rs:2564), native zoom or effective-scale changes invalidate it at [`main.rs`](../../../apps/scriptcraft/src/main.rs:2666), and the status renderer independently substitutes Unavailable whenever the current frame reports a zoom edit at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:1241). All three paths must be reconciled; changing only the tracker is insufficient.
4. Fresh selection itself is not the root cause: [`select_visible_page()`](../../../apps/scriptcraft/src/status_bar.rs:623) returns a valid one-based page and total for valid nonempty geometry. The unavailable presentation occurs before that selection or from the explicit zoom override.
5. Successful New invalidates through [`dispatch_toolbar_action()`](../../../apps/scriptcraft/src/toolbar.rs:307), and successful Open invalidates through [`activate_opened_project()`](../../../apps/scriptcraft/src/main.rs:921). Save, layout failure, incomplete geometry, and zoom use the same invalidation method at [`main.rs`](../../../apps/scriptcraft/src/main.rs:797), [`main.rs`](../../../apps/scriptcraft/src/main.rs:1368), [`main.rs`](../../../apps/scriptcraft/src/main.rs:1586), and [`main.rs`](../../../apps/scriptcraft/src/main.rs:2570). Reusing that method unchanged for both same-document refreshes and document replacement would be unsafe once a fallback is added.
6. Revision alone is not a document identity. New and Open can replace a session with another session at the same revision while viewport and scale remain unchanged. Document reset must therefore be explicit at successful replacement boundaries.

## Scope

1. Extend [`ViewportPageTracker`](../../../apps/scriptcraft/src/status_bar.rs:699) with a separate last-valid [`VisiblePage::Available`](../../../apps/scriptcraft/src/status_bar.rs:595) value. Update it only after [`select_visible_page()`](../../../apps/scriptcraft/src/status_bar.rs:623) returns a valid value from accepted fresh geometry. Never manufacture Page 1 or infer a value from the caret.
2. Make ordinary same-document invalidation clear stale published geometry while retaining the last-valid display value. When current geometry is invalid, missing, stale, or explicitly invalidated, return the retained value if one exists; otherwise return [`VisiblePage::Unavailable`](../../../apps/scriptcraft/src/status_bar.rs:595). Preserve repaint requests and fresh-publication behavior in [`ViewportPageTracker::publish()`](../../../apps/scriptcraft/src/status_bar.rs:801).
3. Add a distinct document-lifetime reset that clears both published geometry and the retained value. Route only successful document replacement through it:
   - successful New after the replacement session is accepted in [`dispatch_toolbar_action()`](../../../apps/scriptcraft/src/toolbar.rs:317);
   - successful Open after the replacement session is accepted in [`activate_opened_project()`](../../../apps/scriptcraft/src/main.rs:943).
   Failed Open must leave the active document and its retained value untouched. Same-document Save, zoom, resize, scrolling, layout errors, and incomplete geometry must not perform a document-lifetime reset.
4. Remove the independent zoom-change substitution in [`show_impl_with_appearance()`](../../../apps/scriptcraft/src/status_bar.rs:1241) so it uses the tracker's retained-or-fresh result rather than forcing Unavailable after tracker fallback has been introduced.
5. Preserve [`select_visible_page()`](../../../apps/scriptcraft/src/status_bar.rs:623), actual editor geometry publication in [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:1572), viewport-centered zoom compensation, native pinch normalization, TextState persistence, and all editor input/focus behavior. Page presentation remains observational and must not scroll, reveal, focus, or edit the document.
6. Preserve the existing noninteractive AccessKit label projection in [`paint_noninteractive_text()`](../../../apps/scriptcraft/src/status_bar.rs:520), bounded left-page clipping in [`show_impl_with_appearance()`](../../../apps/scriptcraft/src/status_bar.rs:1241), and the stable one-row layout. No new live region, popup, or focus owner is introduced.

## Initial and replacement semantics

- At application startup, no retained value exists; Page unavailable remains until the first valid editor publication.
- During ordinary same-document transient geometry changes, the most recent valid page and total remain visible until a new valid value replaces them, even if refresh takes multiple frames or a same-document layout failure persists.
- Immediately after successful New or Open, the previous document's retained value is cleared before the next status projection. Page unavailable remains until the replacement document publishes valid geometry.
- A failed Open does not replace the document and therefore does not clear its last-valid value.
- The retained total may describe the same document's prior valid reflow during a transient refresh, but it may never describe a prior document.

## Non-goals

- No change to zoom range, presets, slider behavior, zoom anchoring, native pinch, persistence, fixed pagination, source maps, exports, or editing semantics.
- No change to overlap, gap, tie, title-page, or one-based numbering rules.
- No stale-value timer, caret-derived fallback, document-content hashing, new document identifier persisted in FDX, or cross-document heuristic.
- No numeric spacing, typography, margin, padding, track, knob, or palette changes.
- No new automated desktop-test target and no agent execution of any check or validation.

## Structural acceptance for later user review

1. Startup shows Unavailable only until the first valid value; subsequent same-document scroll, zoom, resize, and reflow never flash Unavailable when a retained value exists.
2. A fresh valid value replaces the retained value; invalid or stale geometry never overwrites it.
3. Successful New and Open clear the old value before projection even when old and new session revisions alias, and only fresh replacement geometry supplies the next value.
4. Failed Open preserves the active document's value; same-document Save and zoom do not clear it.
5. Page updates create no editor scroll, caret reveal, focus, input, persistence, pagination, or export side effects.
6. The page label remains clipped, noninteractive, and accessibly named at wide and narrow widths.

## Durable records if authorized

- Revise [`AC-WRITING-006.6`](../../../.forge/requirements/features/screenplay-writing-loop.md:628) to distinguish same-document last-valid retention from startup/document-replacement unavailability. Keep fresh-value replacement, no-side-effect, and no-caret-fallback rules explicit.
- Verify [`AC-WRITING-006.5`](../../../.forge/requirements/features/screenplay-writing-loop.md:622) remains unchanged.
- Update the status geometry description in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:41) and the status-before-editor publication contract with the two invalidation lifetimes.
- Append actual implementation details and unresolved evidence to [`ui-icons-v1.md`](../ui-icons-v1.md) and this work order only after implementation. Do not mark this proposal implemented or validated now.

## User-owned review requests if authorized

- Start a new window and observe the initial transition from Unavailable to the first valid page.
- Scroll continuously across pages and gaps; drag the slider, use presets and native pinch, resize, edit across a page boundary, and trigger caret reveal. Confirm no unavailable flash after a value exists and that each fresh value is correct.
- Open documents with different page counts and visible pages, including cases where revision numbers match; create New repeatedly. Confirm no old-document page or total appears before fresh geometry.
- Cancel or fail Open and confirm the current document's value remains. Save the same document and confirm its value remains through republishing.
- Exercise a same-document layout failure and recovery, checking that the retained value is understood as last valid rather than freshly computed.
- Inspect pointer, keyboard, screen-reader label, focus, editor scroll position, clipping, and narrow-width behavior.
- Run any desired workspace build/check/test/lint/formatting commands and record outcomes separately. No agent result is claimed by this proposal.

## WO-004 implementation handoff — 2026-11-04

Implemented on the existing shared branch `dev/2026-11-04-ui-icons`. The
`ViewportPageTracker` now keeps the last valid page/total independently from its
published geometry snapshot. Only an accepted fresh snapshot whose actual page
rectangles and editor clip produce a valid selection replaces that display
value. Missing, stale, mismatched, explicitly invalidated, or invalid geometry
returns the retained value when present, otherwise Page unavailable; the tracker
continues to request repaint/fresh publication as needed.

Ordinary `invalidate()` clears only published geometry and retains the last
valid display value, covering same-document zoom, scale/resize, scroll/reflow,
Save republishing, and layout/geometry failure paths. A separate
document-replacement reset clears both geometry and retained display state. It
runs after successful New and after successful Open activation; Open failures
remain on paths that do not replace the session or reset the tracker. The status
renderer no longer substitutes an independent same-frame Page unavailable value
for zoom edits.

The existing actual editor-geometry publication and `select_visible_page()`
rules remain in place. Page status remains observational and does not introduce
caret fallback, editor movement/focus/input effects, persistence, or semantic
document changes. Zoom anchoring, native pinch, TextState persistence, fixed
pagination, exports, accessibility projection, bounded clipping, and row layout
were not intentionally changed. AC-WRITING-006.6 and the technical-context
geometry/publication description now state the two lifetimes; AC-WRITING-006.5
was left unchanged. No spacing, typography, margin, padding, track, knob, or
palette values were changed, and no test target was added.

The retained page and total are intentionally last-known-valid values, not a
fresh calculation during a prolonged same-document failure or reflow. Startup
and successful replacement remain unavailable until valid replacement geometry
arrives. Source implementation and durable records are submitted only; no build,
check, test, lint, formatting command, manual verification, implementation
review, or independent validation was run. Compilation, runtime behavior,
accessibility, zoom and document lifecycle behavior remain unverified and
user-owned. At the time of this handoff, WO-005 and WO-006 were not yet
implemented; the subsequent WO-005 handoff is recorded separately. No initiative
acceptance or closeout is claimed.

## WO-004 implementation reconciliation — 2026-11-04

The WO-005 source inspection correctly found that the earlier source did not
contain the retained display field or `reset_for_document_replacement()`, despite
the successful-New and successful-Open call sites already invoking that method.
This reconciliation adds the missing tracker state and reset behavior. Accepted,
fresh geometry updates the retained value only when `select_visible_page()` returns
a valid page/total; missing, invalidated, stale, mismatched, or invalid geometry
now returns the retained same-document value, or Page unavailable when none
exists. Ordinary invalidation clears only published geometry, while the distinct
replacement reset clears both lifetimes and requests fresh publication. The
existing New and Open calls are already positioned after their replacement
sessions are accepted. The independent same-frame zoom Unavailable override was
absent in the inspected renderer and required no change.

This closes the reported source discrepancy, not the user-owned validation gate.
No build, check, test, lint, formatting, manual verification, review, or validation
was run; compilation, runtime, accessibility, zoom, and document-lifecycle
behavior remain user-unverified. No WO-006 work or initiative acceptance is
claimed.

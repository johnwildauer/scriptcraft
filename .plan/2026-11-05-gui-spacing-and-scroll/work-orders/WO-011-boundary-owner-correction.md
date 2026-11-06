# WO-011 — Correct the identified toolbar-to-Script View boundary owner

- **Status:** Source correction implemented; awaiting user runtime review
- **Initiative:** [`toolbar-script-view-gap-investigation.md`](../toolbar-script-view-gap-investigation.md)
- **Depends on:** [`WO-010`](WO-010-boundary-geometry-trace.md) and user-supplied runtime output
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or independent validators

## Outcome

Remove the remaining workspace-colored gap between the toolbar and Script View at its measured owner, so the first page and vertical scroll viewport begin immediately below the toolbar at the top scroll position.

No numeric offset or panel rewrite is authorized before the trace identifies the first incorrect rectangle transition.

## Source trace and correction

The user reports that the visible gap persists while the stable WO-010 allocation origins coincide. The supplied trace summary reports matching toolbar-response, central-panel, scroll-inner, and first-page vertical origins, with zero deltas; it does not measure the painted toolbar edge or the toolbar content UI's minimum/response extent. Source inspection identifies the toolbar response/paint boundary as the remaining owner:

- The toolbar explicitly allocates top padding, one Item group, and bottom padding in an outer top-down scope. That scope inherited egui's normal vertical `item_spacing`, so its layout inserted implicit gaps between those explicit allocations and extended the toolbar content minimum/response beyond the painted panel composition.
- The top-panel implementation consumes the returned response rectangle to advance the parent cursor; the central panel therefore follows the expanded response. The toolbar divider is painted against the panel UI's available rectangle, leaving the response-only tail as an unpainted band.
- The correction sets vertical item spacing to zero only in the outer composition scope, then restores the captured normal vertical spacing in the Item-group child UI. Role-picker and overflow popup rows therefore continue to receive their normal spacing; the explicit toolbar padding, caption/control gap, horizontal Item gaps, and Item-group height are unchanged.

This is a source-level explanation consistent with the user-supplied stable allocation trace and persistent visual gap. The trace does not include a simultaneous paint-boundary measurement, raw output was not supplied in this task, and the user has not yet reviewed the corrected runtime. No claim of runtime confirmation or acceptance is made.

## Additional user evidence and authorized direction

The user confirms that the visible gap persists unchanged on the WO-010 branch. Stable traces show contiguous allocation origins, shifting the source investigation to the toolbar's visible panel edge versus its returned content/response extent. Inspection supports implicit vertical item spacing inside the toolbar as the response-expansion mechanism.

The source correction prevents implicit toolbar vertical item spacing from expanding the returned panel content beyond its explicit top/group/bottom composition. All intentional toolbar padding and caption gaps remain explicit; role-popup row spacing retains the normal metrics of its nested Item-group UI.

## Implementation record

In [`toolbar.rs`](../../../apps/scriptcraft/src/toolbar.rs), the toolbar captures its normal vertical item spacing before creating the outer scope, suppresses only vertical item spacing in that outer top-down composition, and restores the captured spacing inside the allocated Item-group child before creating Items and nested controls. This makes the outer scope's content extent equal its explicit top-padding, Item-group, and bottom-padding composition. As a result, the panel's returned response no longer includes the implicit unpainted tail that previously advanced the central panel below the visible toolbar edge.

No Script View geometry, scroll identity, zoom compensation, status border, toolbar Item behavior, popup behavior, accessibility, or user-owned value was changed by WO-011. The temporary WO-010 trace was retired after its capture and is no longer present. No builds, checks, tests, lint, formatting, manual verification, reviewers, or validators were run for WO-011; user runtime review remains required.

## Scope

1. Modify only the source owner identified by response/paint inspection consistent with the supplied trace.
2. Keep the toolbar, central Script Workspace, scroll viewport, and first page rectangle contiguous at the top scroll position, with no workspace-colored band or early clip.
3. Preserve the existing semantic toolbar/status borders, zero toolbar trailing allocation, zero top content margin, retained bottom page-end padding, and disabled Script View fades.
4. Update actual and predicted geometry together if the correction affects page origin or content extent. Preserve page-local zoom scaling, outside-page displacement, page stride, clamping, page publication, visible-page status, pointer routing, caret reveal, and manual-scroll cancellation.
5. Preserve panel resizing, native window behavior, popup layering, accessibility, input routing, and document/session behavior.
6. Remove or retain the WO-010 instrumentation only as its development-only design permits; do not leave default-enabled output.
7. Reconcile the durable boundary contract and record exact evidence, source changes, and limitations in this draft and the investigation note.

## Non-goals

- No speculative negative margin or unrelated padding change.
- No full replacement of egui panel composition unless the trace proves it necessary.
- No screenplay-core, FDX, pagination, persistence, export, toolbar Item, or status-control changes.
- No agent-run validation.

## Structural acceptance for user review

1. The corrective edit removes the source-level response/paint extent mismatch consistent with the captured allocation boundaries and visible gap, without claiming that the trace measured paint.
2. At the initial/top scroll position, the first page and vertical scroll viewport meet the toolbar's lower boundary without the prior band.
3. Ordinary scrolling remains unobscured, and bottom page-end padding remains available.
4. Zoom compensation, page publication, input, visible-page status, and native/panel resizing remain coherent.
5. Durable records describe lasting behavior without work-order identifiers, chronology, or numeric tuning values.

## User-owned review requests

- Confirm the top-of-document gap is gone and the scrollbar begins immediately below the toolbar.
- Scroll through representative documents and confirm no new clipping, overlays, or page-position regressions.
- Recheck horizontal overflow, zoom paths, caret reveal, resize, visible-page status, and all Appearance themes.
- Run any desired workspace checks and record outcomes separately; no agent result is claimed.

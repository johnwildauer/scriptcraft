# WO-011 — Correct the identified toolbar-to-Script View boundary owner

- **Status:** Drafted but blocked pending the user-captured WO-010 trace; do not dispatch before that evidence exists
- **Initiative:** [`toolbar-script-view-gap-investigation.md`](../toolbar-script-view-gap-investigation.md)
- **Depends on:** [`WO-010`](WO-010-boundary-geometry-trace.md) and user-supplied runtime output
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or independent validators

## Outcome

Remove the remaining workspace-colored gap between the toolbar and Script View at its measured owner, so the first page and vertical scroll viewport begin immediately below the toolbar at the top scroll position.

No numeric offset or panel rewrite is authorized before the trace identifies the first incorrect rectangle transition.

## Evidence gate

Before implementation, attach the user's WO-010 output to the investigation note and identify exactly one of these owners:

1. **Toolbar response/root cursor:** central origin is lower than the toolbar response bottom.
2. **Central frame/child UI:** central origin is correct but its child or scroll inner origin is lower.
3. **Script page-row allocation:** scroll inner origin is correct but the first page row is lower.
4. **Build/runtime mismatch:** traced constants or frame behavior do not match the checked-out source.

Do not proceed with a source layout correction if the evidence is incomplete or identifies a different running build; establish the correct executable first.

## Scope

1. Modify only the layer implicated by the captured trace.
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

1. The corrective edit maps directly to the first incorrect runtime rectangle transition in the captured trace.
2. At the initial/top scroll position, the first page and vertical scroll viewport meet the toolbar's lower boundary without the prior band.
3. Ordinary scrolling remains unobscured, and bottom page-end padding remains available.
4. Zoom compensation, page publication, input, visible-page status, and native/panel resizing remain coherent.
5. Durable records describe lasting behavior without work-order identifiers, chronology, or numeric tuning values.

## User-owned review requests

- Confirm the top-of-document gap is gone and the scrollbar begins immediately below the toolbar.
- Scroll through representative documents and confirm no new clipping, overlays, or page-position regressions.
- Recheck horizontal overflow, zoom paths, caret reveal, resize, visible-page status, and all Appearance themes.
- Run any desired workspace checks and record outcomes separately; no agent result is claimed.

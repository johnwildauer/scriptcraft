# WO-010 — Boundary geometry trace

- **Status:** Approved for implementation on the separate investigation branch
- **Initiative:** [`toolbar-script-view-gap-investigation.md`](../toolbar-script-view-gap-investigation.md)
- **Depends on:** Committed GUI baseline and new investigation branch
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or independent validators

## Outcome

Capture the actual runtime rectangles responsible for the remaining toolbar-to-Script View gap without changing application layout, painting, scrolling, or document behavior.

The trace must distinguish the toolbar/root handoff, central frame/child handoff, scroll viewport origin, and first page-row allocation.

## Scope

1. Add a manual development-only source gate for boundary geometry tracing, defaulting to disabled. Do not enable it in production or couple it to screenplay content.
2. At the relevant existing composition points, capture numeric geometry for:
   - Root available rectangle before toolbar allocation.
   - Actual top toolbar response rectangle.
   - Parent available rectangle immediately before central-panel construction.
   - Actual central-panel response rectangle.
   - Central child available and clip rectangles immediately before scroll construction.
   - Actual scroll outer and inner rectangles and committed offset.
   - First allocated page rectangle and its clip.
   - Effective pixels-per-point and current top/bottom Script View content-margin settings.
3. Derive numeric deltas for central origin minus toolbar bottom, scroll inner origin minus toolbar bottom, first page origin minus scroll inner origin, and first page origin minus toolbar bottom.
4. Emit a bounded trace only when relevant geometry changes or at another explicit limited development trigger. Do not log every frame indefinitely.
5. Keep the trace value-free except numeric geometry and fixed labels. Do not include screenplay text, source values, file paths, IDs, or persisted document data.
6. Do not alter allocation, clipping, paint order, persistent scroll identity, panel sizing, page publication, zoom compensation, input, or user state.
7. Update this draft and the investigation note with exact instrumentation and user capture instructions. No durable requirement or blueprint change is needed for development-only instrumentation.

## Non-goals

- No corrective layout change in this order.
- No default-enabled logging, new runtime preference, or document-derived diagnostic content.
- No replacement of the central panel or explicit negative offset.
- No agent-run validation.

## Structural acceptance for user review

1. Disabled tracing has no output or behavior change.
2. Enabled tracing reports enough actual rectangles and deltas to identify the first nonzero boundary offset.
3. Output is bounded, numeric, and free of document-derived content.
4. Existing layout, scrolling, painting, and interaction code paths remain unchanged.

## User capture request

- Enable the trace constant locally, rebuild and launch the app, and display the document at the initial/top scroll position where the gap is visible.
- Capture the bounded geometry output and restore the trace gate to disabled afterward.
- Provide the output for the evidence-gated corrective order. No agent runtime result is claimed.

## Handoff requirement

Record the branch, changed files, exact trace fields, and evidence limitations in this draft and [`toolbar-script-view-gap-investigation.md`](../toolbar-script-view-gap-investigation.md). Do not mark the runtime owner identified until user trace evidence exists.

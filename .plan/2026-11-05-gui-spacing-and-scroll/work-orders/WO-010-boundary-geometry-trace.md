# WO-010 — Boundary geometry trace

- **Status:** User capture recorded; development instrumentation retired; runtime owner unresolved
- **Implementation branch:** `dev/2026-11-06-script-view-boundary`
- **Baseline:** `728282270489822770fea5141ddb3595aad340e9`
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

## Original user capture procedure (completed; historical)

- The trace was enabled locally for the requested capture and then retired after the user supplied the output. Its source is no longer present.
- The historical procedure was to rebuild and launch the app using the normal development workflow, open the affected document, and leave Script View at the top scroll position where the gap is visible.
- The user captured stderr lines beginning `WO010` from the application terminal. The trace emitted only at a near-zero committed vertical offset, only when the captured geometry changed, and at most 16 lines per application thread.
- The gate was restored to disabled after capture. The user-supplied summary is recorded below and remains unchanged; it does not by itself identify a runtime owner.

## Handoff requirement

### Implementation record

- **Branch and baseline:** `dev/2026-11-06-script-view-boundary`, based on `728282270489822770fea5141ddb3595aad340e9`.
- **Changed files for the original instrumentation:** The temporary module at [`apps/scriptcraft/src/boundary_trace.rs`](#instrumentation-retirement), [`main.rs`](../../../apps/scriptcraft/src/main.rs), [`script_view.rs`](../../../apps/scriptcraft/src/script_view.rs), this WO-010 draft, and [`toolbar-script-view-gap-investigation.md`](../toolbar-script-view-gap-investigation.md). The temporary module has since been deleted, and only trace-specific changes were removed from the application files.
- **Instrumentation (historical):** Its manual source gate defaulted to disabled. At the existing panel composition points the trace recorded root available, top toolbar response, pre-central parent available, and central response rectangles. At the live Script View/scroll composition points it recorded child available and clip rectangles, scroll outer and inner rectangles, committed offset, first allocated page rectangle and its viewport-clipped portion, effective pixels-per-point, and configured top/bottom content margins. The final scroll inner rectangle was derived from the returned content size capped to the allocated viewport; the final outer rectangle added the already-allocated scrollbar extent.
- **Output fields:** One fixed-label `WO010` stderr line contains `root_avail`, `toolbar`, `parent_avail`, `central`, `child_avail`, `child_clip`, `scroll_outer`, `scroll_inner`, `scroll_offset`, `first_page`, `first_page_clip`, `ppp`, `margin_top`, `margin_bottom`, `d_central_toolbar_y`, `d_scroll_toolbar_y`, `d_page_scroll_y`, and `d_page_toolbar_y`. Rectangles are numeric `x,y,width,height` values in egui points; deltas are vertical-origin differences in points.
- **Bound:** No emission while committed vertical scroll offset is more than 0.5 points from the top; identical snapshots are suppressed; no more than 16 distinct lines are emitted per application thread. Output contains fixed labels and numeric geometry only.
- **User runtime capture:** Four distinct snapshots were supplied. With vertical offset 0.000 and scale 2.0000, the stable snapshots report toolbar bottom, central top, scroll-inner top, and first-page top all at y=50.000, with all four derived deltas 0.000. At root height 842, one intermediate snapshot reports first-page top y=39.000, first-page clip top y=50.000, and both page-origin deltas -11.000; the next snapshot returns the page top to y=50.000. Top/bottom margins are 0.000/4.000. The two root heights represented are 874 and 842 points.
- **Evidence limitation:** The capture shows no persistent positive toolbar-to-page allocation offset in its stable snapshots. The single transient -11-point page allocation is not a stable explanation for the reported visual gap. The trace reports allocation/clip geometry, not painted pixels, and no simultaneous screenshot or repeat capture is included. The runtime boundary owner therefore remains unresolved. No agent build, check, test, lint, format, manual verification, review, or validator was run.

### Instrumentation retirement

After the requested capture was complete, the temporary boundary trace was retired: its module was deleted, its module declaration and capture/emission plumbing were removed from [`main.rs`](../../../apps/scriptcraft/src/main.rs), and its capture parameters/assignments were removed from [`script_view.rs`](../../../apps/scriptcraft/src/script_view.rs). The supplied capture evidence above is preserved unchanged. No trace instrumentation is currently present; a future diagnostic capture would require a separately scoped implementation. This cleanup did not change the WO-011 outer toolbar spacing correction, WO-012 Elements spacing correction, the current explicit 4-point toolbar bottom padding, or other non-trace behavior. No build, check, test, lint, format, or manual validation was run for this cleanup.

Do not update durable requirements/blueprints for this development-only instrumentation, and do not implement WO-011 in this order.

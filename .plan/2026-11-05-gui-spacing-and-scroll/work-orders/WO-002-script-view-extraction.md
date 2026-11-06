# WO-002 — Extract Script View without behavior changes

- **Status:** Implementation submitted for user review; not user-validated or accepted
- **Initiative:** [`gui-spacing-and-scroll.md`](../gui-spacing-and-scroll.md)
- **Depends on:** [`WO-001`](WO-001-status-height-ownership.md) under sequential execution; no source-level dependency
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or independent validators

## Outcome

Move the desktop page editor into a dedicated Script View module before changing its geometry. Establish the same presentation-module boundary already used by the toolbar and status bar while preserving all editing, rendering, input, scrolling, zoom, and diagnostics behavior.

This is a structural extraction only. No spacing, fade, color, pagination, or interaction change belongs in this order.

## Current source boundary

[`main.rs`](../../../apps/scriptcraft/src/main.rs) currently combines application/session orchestration with the complete Script View implementation, including:

- Page allocation, painting, culling, and caret/IME presentation through [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:1376) and [`paint_page()`](../../../apps/scriptcraft/src/main.rs:1204).
- Pointer hit testing and selection through [`caret_from_pointer()`](../../../apps/scriptcraft/src/main.rs:1076), [`pointer_body_page()`](../../../apps/scriptcraft/src/main.rs:1117), and [`apply_pointer_selection()`](../../../apps/scriptcraft/src/main.rs:1148).
- Surface event dispatch and keyboard editing through [`dispatch_surface_events()`](../../../apps/scriptcraft/src/main.rs:1686), [`key_event()`](../../../apps/scriptcraft/src/main.rs:1760), and [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:1791).
- Presentation scale and scroll geometry through [`presentation_metrics_at_zoom()`](../../../apps/scriptcraft/src/main.rs:2038), [`page_scroll_geometry()`](../../../apps/scriptcraft/src/main.rs:2078), and [`prepaint_zoom_scroll_offset()`](../../../apps/scriptcraft/src/main.rs:2101).
- Fixed-cell galley caching and painting through [`FixedCellGalleyCache`](../../../apps/scriptcraft/src/main.rs:2182) and [`paint_fixed_metric_text_with_selection()`](../../../apps/scriptcraft/src/main.rs:2199).
- Script View-owned constants at the head of [`main.rs`](../../../apps/scriptcraft/src/main.rs:39) and page/caret/cursor helpers near the panel composition.

## Scope

1. Add [`script_view.rs`](../../../apps/scriptcraft/src/script_view.rs) and register it from [`main.rs`](../../../apps/scriptcraft/src/main.rs:27).
2. Move Script View-owned page allocation, painting, viewport culling, scrolling, presentation scaling, zoom compensation, pointer selection, caret/IME interaction, surface input routing, vertical navigation, and fixed-cell galley support into the new module.
3. Move only constants and helpers owned by that surface. Preserve their names and numeric values unless a visibility-only rename is necessary for module organization.
4. Keep application/session/file operations, native dialogs, project activation, save/export adapters, toolbar/status composition, Appearance, Sprint Timer, notifications, and top-level frame orchestration in their existing owners.
5. Use minimal module visibility. Shared application state may remain in the existing app type, but do not create a parallel session, selection, zoom, geometry, or diagnostics state.
6. Preserve actual page-rectangle publication and invalidation, visible-page tracking, deep-scroll viewport-centered zoom, prepaint compensation, manual-scroll cancellation, conditional caret reveal, focus locking, native input ordering, viewport culling/overscan, and performance counters.
7. Preserve the existing public/headless screenplay-core boundary. Script View remains desktop presentation and interaction code only.
8. Update the durable module map and record the extraction handoff in the initiative records.

## Non-goals

- No Script View spacing, margin, fade, clipping, color, typography, or page-gap changes.
- No changes to zoom limits, page scale conversion, page dimensions, scroll-anchor semantics, or visible-page selection.
- No semantic editing, FDX, native save/reopen, pagination, PDF, plain-text export, toolbar, status, timer, or Appearance behavior changes.
- No new state store, runtime service, dependency, or persisted setting.
- No new automated desktop-test target and no agent execution of validation.

## Structural acceptance for user review

1. [`script_view.rs`](../../../apps/scriptcraft/src/script_view.rs) is the clear owner of desktop page-surface rendering and interaction.
2. [`main.rs`](../../../apps/scriptcraft/src/main.rs) retains application and document orchestration rather than page-painting implementation details.
3. All moved constants and behavior remain unchanged; the following scroll-geometry correction is not performed early.
4. Existing app/session adapters and core boundaries remain intact, with no duplicate mutable view state.
5. Geometry publication, input routing, focus, zoom, caret reveal, and diagnostics retain their existing contracts.

## Durable records

- Update the project map and desktop boundaries in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:7) to identify Script View as the owner of page-surface presentation and interaction, distinct from application orchestration and the headless core.
- Describe the durable relationships among Script View, published page geometry, status projection, presentation zoom, and semantic session operations without documenting a file-moving exercise.
- No requirements change is needed for a behavior-preserving module extraction.
- Keep the blueprint free of work-order identifiers, implementation chronology, and numeric spacing or tuning values. Record those details only in this draft and the initiative overview.

## User-owned review requests

- Exercise text entry, paste, selection, keyboard navigation, undo/redo, IME, caret blinking, and conditional caret reveal.
- Exercise vertical and horizontal scrolling, title pages, multi-page documents, narrow windows, and all zoom paths, including native pinch and status controls.
- Confirm visible-page status remains correct through scrolling, resize, zoom, reflow, New/Open, and same-document Save.
- Confirm focus locking, pointer/cursor ownership, popup behavior, diagnostics projection, and document replacement remain unchanged.
- Review the module boundary to ensure no semantic document behavior moved into desktop presentation and no duplicate view state was introduced.
- Run any desired workspace compilation and behavior checks and record outcomes separately; no agent result is claimed.

## WO-002 implementation-submission handoff

- **Branch:** `dev/2026-11-05-gui-spacing-and-scroll`, the existing initiative branch. Existing checkout changes and the WO-001 implementation/handoff were preserved; nothing was staged, committed, stashed, discarded, merged, or rebased.
- **Exact source changes:** Added [`script_view.rs`](../../../apps/scriptcraft/src/script_view.rs) and registered the module in [`main.rs`](../../../apps/scriptcraft/src/main.rs). The new module owns the Script View surface implementation: page allocation/painting/culling, scroll and presentation-scale geometry, prepaint zoom compensation, pointer selection, caret/IME presentation, surface event and keyboard routing, vertical navigation, fixed-cell galley caching/painting, and the Script Workspace frame/clear-color helpers. [`main.rs`](../../../apps/scriptcraft/src/main.rs) retains application/session/file orchestration and panel composition, delegating the surface to the new module. Moved constants, algorithms, and behavior without changing their values or intended interactions; no WO-003/WO-004 presentation changes were made.
- **Durable record:** [`technical-context.md`](../../../.forge/blueprints/technical-context.md) now describes the lasting Script View ownership boundary, its use of application-owned state, published geometry/status/zoom coordination, and the separation from orchestration and screenplay-core semantics. No requirements change was made.
- **Files changed for this submission:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs), [`apps/scriptcraft/src/script_view.rs`](../../../apps/scriptcraft/src/script_view.rs), [`technical-context.md`](../../../.forge/blueprints/technical-context.md), this WO-002 draft, and the initiative overview. The prior WO-001 handoff is preserved.
- **Evidence limitations:** No build, check, test, lint, formatting command, manual verification, independent review, or validator was run. Compilation, framework compatibility, runtime rendering, editing/input behavior, scrolling/zoom, diagnostics, and visual appearance remain unverified and user-owned. This is an implementation submission only; no user validation or acceptance is claimed.

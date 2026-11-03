# RS-WO-003 — Same-frame editor input dispatch

- **Status:** Implemented 2026-11-03; user review and validation pending; not user-validated
- **Initiative:** [`ui-quality-of-life-v1-render-speed.md`](../ui-quality-of-life-v1-render-speed.md)
- **Depends on:** [`RS-WO-002`](WO-002-render-path-oracle.md)
- **Validation owner:** User; no agent-run checks or independent validators

## Objective

Dispatch accepted editor input before the document layout and paint stage so an edit need not inherently wait for a second frame to become visible.

This order does not change edit operations, derived-refresh cadence, page painting, or layout caching.

## Implementation steps

1. Reorder [`ScriptCraftApp::update()`](../../../apps/scriptcraft/src/main.rs:1824) so [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:1188) runs before [`show_application_panels()`](../../../apps/scriptcraft/src/main.rs:1641) when no FDX migration modal owns input.
2. Preserve native window-state capture, notification setup, appearance handling, timer behavior, and migration-modal exclusivity.
3. Use the previous frame's editor focus only as the established input-routing state; do not change focus lock filters, event consumption, IME handling, or pointer activation.
4. Preserve [`RevealIntent`](../../../apps/scriptcraft/src/main.rs:84) behavior so the same frame lays out and minimally reveals the new caret only when required.
5. Leave [`refresh_cast_lists_on_schedule()`](../../../apps/scriptcraft/src/main.rs:245) cadence and placement for RS-WO-008; record any remaining same-frame derived-work delay rather than hiding it.
6. Keep frame/event diagnostics accurate under the new order.

## Structural acceptance

- A text event accepted at the start of an update reaches that update's layout and paint stage.
- The same event is applied exactly once, with no duplicate IME commit, paste, key, or zoom processing.
- Modal migration, toolbar, status, and unfocused-surface behavior remain unchanged.
- Scroll handling does not create a new conditional caret reveal.

## Regression coverage to author

Add desktop tests proving same-frame text/paste/IME/key processing, no duplicate event dispatch, focus-lock preservation, migration-modal isolation, conditional reveal, and unchanged non-editor controls. Compare frame and event counters before and after dispatch.

## Durable documentation

Record desktop input/frame ordering in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:22) and the initiative execution log. No product requirement change is expected.

## Implementation record — 2026-11-03

The production eframe update now delegates to `ScriptCraftApp::update(&egui::Context)`, which dispatches editor input after native window-state capture, notification setup, and the existing scheduled derived refresh, but before `show_application_panels()` performs current-frame layout and paint. Dispatch remains excluded while the FDX migration dialog is pending. Focus routing continues to use `surface_focused` from the previous surface response; the current panel pass still owns pointer activation, focus-lock installation, and focus-generation updates. Zoom preprocessing, timer handling, conditional caret reveal, scroll cancellation of reveal intent, and scheduled refresh placement/cadence were not moved or redesigned.

The WO-002 desktop helper `render_app_frame_with_fullscreen()` now runs this same application-update method instead of manually rendering panels and then separately dispatching input. The added `desktop_render_oracle_paints_accepted_update_input_in_the_same_frame_once` case drives actual update frames and asserts frame/event deltas, one layout build, page painting, one edit revision, and one rendered occurrence for text paired with its duplicate IME commit, paste, IME preedit/commit, and Enter. It also checks that a scroll cancels pending conditional reveal. `application_update_keeps_migration_modal_exclusive_of_editor_input` exercises a staged non-6 import and asserts the editor event is not dispatched or applied while the modal owns input. Existing pointer focus-lock coverage remains, and the rendered focus-mode/status-control coverage now goes through the shared app-update helper.

Tests were authored but not run. No build, test, formatting, lint, manual verification, or independent review was performed. WO-001/WO-002 coverage is still unvalidated by the user. No baseline or before/after timing observation is recorded; this order has no measurement hard gate, and no comparison or performance result is claimed.

## User validation

Exercise typing, paste, IME composition/commit, Enter/Tab, arrows, scroll, toolbar/status controls, and the FDX migration dialog. Confirm accepted edits appear without the prior extra frame and that no event is duplicated or lost.

## Handoff

RS-WO-004 can consume the post-input layout in the same frame without changing the input contract established here. `ScriptCraftApp::update()` is the shared application scheduling boundary and now dispatches before `show_application_panels()`; retain that order while splitting all-page geometry allocation from viewport-bounded paint. Continue to preserve page rectangles for scroll extent and pointer routing, one conditional caret reveal, title-page ownership, and visible-page status. Update the desktop oracle's paint expectations only for the culling mechanism itself; do not reinterpret event/layout counters or add timing thresholds. Same-frame and prior-order coverage remains authored but unexecuted and user validation remains pending.

### RS-WO-008 scheduling-order handoff — 2026-11-03

RS-WO-008 moved the scheduled derived-refresh call to immediately after `handle_surface_input()` and before the current panel/layout/paint pass. This supersedes WO-003's earlier frame-order note that refresh preceded input; focus ownership, same-frame edit rendering, and migration-modal exclusion remain unchanged. Added same-update coverage expects a due refresh to consume the queued Character edit in that frame. The coverage is authored but unrun and remains user-owned for validation.

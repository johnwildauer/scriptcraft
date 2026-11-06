# WO-007 — View, status, appearance, and sprint-timer commands

- **Status:** Implemented; user-owned validation pending
- **Initiative:** [Hotkeys and consolidated command layer v1](../hotkey-command-layer-v1.md)
- **Depends on:** [WO-006](WO-006-document-and-platform-effects.md)
- **Validation owner:** User; no agent-run checks, reviews, or validators

## Single outcome

Route all existing enabled view, status, appearance, focus-mode, and sprint-timer actions through typed commands and explicit desktop effects without changing their visible feature set or behavior.

## Scope

1. Change status rendering from direct mutable zoom/appearance/timer/focus writes into command-intent production. [`show_with_focus_mode()`](../../../apps/scriptcraft/src/status_bar.rs:1220) may return or emit typed commands, but must not dispatch parallel application operations internally.
2. Route zoom slider and preset changes through `view.zoom.set` with a whole-percent parameter. Reuse existing normalization/clamping, viewport-center anchor capture, pinch baseline synchronization, page-geometry invalidation, repaint, and FDX TextState projection.
3. Translate native pinch into `view.zoom.pinch` with a validated factor, queue/dispatch it before affected layout, and preserve raw Zoom event removal and manual-scroll precedence. Invalid factors leave zoom unchanged.
4. Route System/Day/Night/Midnight selection through `view.appearance.set`; retain transient theme resolution and palette behavior without persisting command state.
5. Route the Focus Mode action through `view.focus-mode.toggle`. Preserve current enablement, observed/pending/uncertain/external-settling gates, generation commit before viewport effect, timeout/recovery, and fullscreen request in [`focus_mode.rs`](../../../apps/scriptcraft/src/focus_mode.rs:183).
6. Route Sprint Timer status toggle, popup Close, outside/Escape close, Countdown/Stopwatch selection, minutes editing, Start, Pause, Resume, Reset, and End through stable commands. UI labels may project the next command from current timer state, but execution must not infer a different operation from label text.
7. Preserve numeric-only minutes filtering, 1–1440 validation, idle/mode gates, current clock/deadline arithmetic, completion reconciliation, generation limits, and popup cleanup in [`sprint_timer.rs`](../../../apps/scriptcraft/src/sprint_timer.rs:105).
8. Route completion notification requests/results through the existing notification host adapter. Preserve bounded asynchronous result handling, repaint wakeup, static privacy-safe feedback, and no claim of delivery confirmation.
9. Keep production-disabled Diagnostics absent. If explicitly enabled for development, its disclosure remains framework-owned presentation bookkeeping unless the durable contract is separately changed.
10. Preserve all row geometry, palettes, icons, accessibility labels, popup placement, timers, and status text. This is command-routing work only.

## Out of scope

- No new zoom shortcuts, view mode, appearance theme, focus behavior, timer feature, notification feature, persisted preference, or diagnostics product surface.
- No screenplay layout/pagination changes and no command/view-state serialization into FDX.
- No low-level commands for hover, paint, caret blink, layout publication, or native clock observations.

## Structural acceptance for user review

- Every enabled status and timer control produces a typed command and is executed by the consolidated dispatcher.
- View/timer state changes occur in command handlers rather than directly inside widget response branches.
- Native pinch, focus-mode viewport commands, and timer notifications remain narrow effects outside the serializable command payload.
- Existing transient-only ownership, scheduling, and failure semantics are preserved.

## User-owned checks

- Exercise zoom slider/presets/pinch, manual scroll during pending zoom, appearance choices, Focus Mode pending/timeout/external changes, and TextState save/reopen zoom behavior.
- Exercise timer open/close/outside/Escape, mode gates, minutes filtering/validation, Enter-to-start, Start/Pause/Resume/Reset/End, completion, notification pending/accepted/unavailable results, and repaint behavior.
- Confirm no visible layout, palette, accessibility, status, or timer presentation regression and no new action appears.
- Run applicable focused and workspace checks.

## Durable-document impact

Update the desktop/status/timer technical context with command intents and effect ownership. Requirements impact is update for uniform action routing; existing appearance, zoom, focus, timer, and persistence behavior remains unchanged.

## WO-007 implementation handoff — 2026-11-06

- Status controls now produce typed command intents instead of directly mutating app-owned zoom, appearance, popup, or focus state. Slider/preset changes emit `view.zoom.set`; appearance choices emit `view.appearance.set`; Focus Mode and the timer status button emit their stable commands. The dispatcher applies whole-percent zoom with the existing anchor capture, pinch-baseline synchronization, page-geometry invalidation, repaint, and TextState projection behavior; appearance remains transient and theme resolution remains in the existing theme owner.
- Raw native Zoom events are converted to validated `view.zoom.pinch` commands and dispatched before layout; invalid factors do not change zoom, and raw Zoom events remain removed from subsequent egui input. Valid pinch retains continuous baseline math and viewport-center anchoring, while slider/preset commands synchronize the pinch baseline. Existing manual-scroll cancellation/reveal precedence remains in the existing surface path.
- Focus Mode now dispatches `view.focus-mode.toggle`; the dispatcher preserves controller enablement/request gates, commits the request generation before sending fullscreen through the viewport effect adapter, and retains observation, timeout, recovery, external-settling, and repaint scheduling behavior.
- Sprint Timer popup controls now produce typed toggle/close/mode/minutes/start/pause/resume/reset/end commands. Primary button semantics are selected from timer state into a stable command rather than parsed from the rendered label. Timer handlers retain idle/mode gates, ASCII-digit filtering, 1–1440 validation, clock/deadline arithmetic, completion reconciliation, generation limits, and popup-close cleanup. Popup bounds/placement and outside/Escape observation remain presentation bookkeeping.
- Completion notification requests and result application now pass through the desktop timer-notification effect adapter while retaining the existing sink, bounded asynchronous result channel, generation matching, repaint wakeup, and privacy-safe feedback. Existing feedback states explicitly distinguish pending/OS acceptance/unavailability from delivery confirmation.
- Exact files changed: `apps/scriptcraft/src/app_commands.rs`, `apps/scriptcraft/src/main.rs`, `apps/scriptcraft/src/sprint_timer.rs`, `apps/scriptcraft/src/status_bar.rs`, `.forge/blueprints/technical-context.md`, `.forge/requirements/features/screenplay-writing-loop.md`, this work order, and `.plan/2026-11-06-hotkey-command-layer/hotkey-command-layer-v1.md`. No command-crate, manifest, or lockfile change was needed; the typed command vocabulary already contained the necessary view and timer commands.
- Source-level implementation evidence: `status_bar::show_with_focus_mode` now returns typed `Command` intents; the desktop dispatcher owns view/focus/timer mutations and routes fullscreen/notification effects through narrow adapters; `raw_input_hook` validates and dispatches `ViewZoomPinch` before removing raw Zoom events; timer popup response branches construct typed commands, while `primary_command()` selects an operation from timer state independently of button-label text. This is implementation evidence only.
- Risks and user-owned validation: no compilation, runtime behavior, focus/fullscreen transition, pinch event delivery, viewport anchoring, rendering, timer transition, notification platform behavior, or command-dispatch result has been validated. User validation should exercise slider/presets/pinch and scroll precedence; appearance choices and TextState save/reopen; Focus Mode pending/timeout/recovery/external changes; timer open/close/outside/Escape, mode and input gates, minutes filtering/range, Enter/start/pause/resume/reset/end/completion, generation limits and popup cleanup; notification pending/accepted/unavailable results and repaint; and confirm existing status/error/editing/document/clipboard/modal/viewport behavior remains unchanged. No tests, builds, checks, Clippy, formatting, manual verification, review, or independent validation were run.
- WO-008 handoff: reconcile durable command/effect contracts and inventory any remaining enabled direct-action bypasses against the implemented view/status/focus/timer paths. Keep notifications as host effects, preserve the `EffectDispatched` versus effect-success distinction and existing status/error authority, and report only user-owned validation scenarios/results; do not infer that authored source paths compile or behave correctly.

# Keyboard input handoff

## Current problem

The native ScriptCraft window opens and displays the page-layout editor, including a visible caret. In
the latest manual run, clicking the document and attempting to type does not insert text. The application
launches, but the primary editor is not reliably receiving keyboard text input.

This remains a closeout blocker. The Forge initiative must stay open until a real native-window test can
confirm typing, Unicode input, editing keys, and rerendering in the user-visible application.

## Intended behavior

The page-layout view is the one and only editing surface. After clicking in the document, the user should
be able to:

- type ordinary and Unicode text directly into the document;
- press Enter to split an element and apply the role transition;
- use Backspace and Delete;
- move with arrow keys and select by dragging or keyboard extension;
- replace a selection and use Undo/Redo; and
- see the updated layout and caret immediately after each edit.

There must be no hidden or duplicate multiline text editor used as a fallback.

## Work already attempted

1. Inspected sibling WordCraft's keyboard loop in `../craft/wordcraft`:
   - the focused canvas requests egui focus;
   - `keys::canvas_events()` consumes text, key, paste, and IME events;
   - events are converted to command dispatch;
   - the engine session mutates transactionally;
   - the next frame recomputes layout and paints caret/selection.
2. Implemented WO-011 keyboard/IME dispatch in [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs):
   - consumes egui text/IME events;
   - maps Enter, Backspace, Delete, arrows, Undo, and Redo;
   - routes mutations through `EditingSession`;
   - invalidates/rebuilds layout and caret state.
3. Implemented WO-011a to retain page-editor focus across frames.
4. Implemented WO-011b to wire a stable page-editor egui ID, `ui.interact(...)`,
   `request_focus()`, and `response.has_focus()` into the rendered page path.
5. Implemented WO-011c to test genuine rendered egui pointer activation from an unfocused state,
   followed by later-frame text and Backspace dispatch.
6. Independent validators passed the WO-011c rendered-path regression and all Cargo quality gates.
7. The fixes were committed in `a6d3586` (`Fix: capture keyboard input in live editor`).

## WO-011d diagnostics lifecycle

WO-011d retains diagnostics as bounded status instrumentation rather than a
temporary log. The status line reports only boolean focus/lock/dispatch state,
saturating event/frame/activation counters, and the last fixed event category
(`text`, `paste`, `key-*`, `ime-*`, `other`, or `none`). It does not retain or
display document text, event payloads, or an editable/shadow buffer. This
instrumentation is available for the native acceptance run and is part of the
hardened desktop boundary; removal requires a later authorized work order and
blueprint reconciliation.

## Important discrepancy

The headless/egui regression tests and Cargo validation pass, but the latest human manual run still cannot
type into the native window. A previous testing attempt did accept typing, so behavior appears sensitive
to a runtime focus, event-loop, platform, or test-sequence condition rather than uniformly absent.

Do not treat the passing synthetic/rendered-context tests as proof of native macOS keyboard behavior.

## Next investigation priorities

1. Reproduce manually with `cargo run -p scriptcraft` on macOS.
2. Log or expose, temporarily and visibly, each frame's:
   - page response `has_focus()`;
   - pointer press/release and `request_focus()` result/state;
   - received `egui::Event::Text`, `Event::Key`, and IME commit/preedit events;
   - `surface_focused` state entering `handle_surface_input()`.
3. Confirm the native window is the active macOS application and that the click lands inside the
   page interaction rectangle rather than only on a non-interactive painter layer.
4. Check whether eframe/winit delivers macOS text input as `Event::Text`, IME commit, or another event
   variant in the failing run.
5. Compare the first successful historical run with the failing run: launch command, click location,
   focus timing, keyboard layout, window activation, and whether the test began after a Save/Open dialog.
6. Add a real native/manual smoke path or an automation harness that proves pointer activation and a
   subsequent physical key/text event, not only synthetic `egui::Context` input.

## Authorized next work-order direction

The owner authorized a production hardening work order rather than treating the
passing rendered regression as native proof. The bounded work is
[`WO-011d-native-input-capture-hardening.md`](work-orders/WO-011d-native-input-capture-hardening.md:1):
align the app more closely with WordCraft's single-canvas boundary by using one
stable page-editor response/ID for focus, an explicit egui focus-lock filter,
focused event consumption, and clearly labeled native-input diagnostics. The
work order preserves the existing [`EditingSession`](../../../crates/scriptcraft-core/src/editing.rs:59)
and single painted surface, then requires both fresh automated validation and an
observable native macOS smoke sequence. This authorization is planning
direction only; Forge design review and amended human approval remain required
before implementation dispatch.

## Authorized next work-order direction

The owner authorized a production hardening work order rather than treating the
passing rendered regression as native proof. The bounded work is
[`WO-011d-native-input-capture-hardening.md`](work-orders/WO-011d-native-input-capture-hardening.md:1):
align the app more closely with WordCraft's single-canvas boundary by using one
stable page-editor response/ID for focus, an explicit egui focus-lock filter,
focused event consumption, and clearly labeled native-input diagnostics. The
work order preserves the existing [`EditingSession`](../../../crates/scriptcraft-core/src/editing.rs:59)
and single painted surface, then requires both fresh automated validation and an
observable native macOS smoke sequence. This authorization is planning
direction only; Forge design review and amended human approval remain required
before implementation dispatch.

## Relevant records

- [`WO-011-keyboard-text-input-loop.md`](work-orders/WO-011-keyboard-text-input-loop.md)
- [`WO-011a-retain-page-editor-focus.md`](work-orders/WO-011a-retain-page-editor-focus.md)
- [`WO-011b-wire-rendered-page-editor-focus.md`](work-orders/WO-011b-wire-rendered-page-editor-focus.md)
- [`WO-011c-prove-pointer-rendered-focus.md`](work-orders/WO-011c-prove-pointer-rendered-focus.md)
- [`validation-WO-011.md`](validations/validation-WO-011.md)
- [`validation-WO-011a.md`](validations/validation-WO-011a.md)
- [`validation-WO-011b.md`](validations/validation-WO-011b.md)
- [`validation-WO-011c.md`](validations/validation-WO-011c.md)
- [`execution-log.md`](execution-log.md)

No further WO should be closed, and the initiative should not enter Forge closeout, until this native
keyboard discrepancy is resolved or explicitly accepted by the owner as a documented MVP limitation.

## Owner closeout acceptance — 2026-10-25

The owner observed native text entry after WO-011d and requested MVP closeout
and commit after the WO-011e caret/focus polish. This explicitly accepts the
remaining unavailable full-native validation sequence as a documented MVP
limitation. WO-011d and WO-011e are tabled rather than represented as complete
proof of every native acceptance step. Future native-capable testing should
still exercise Unicode, editing keys, IME, caret blink/alignment, blank-space
activation, and toolbar/dialog focus transfer.

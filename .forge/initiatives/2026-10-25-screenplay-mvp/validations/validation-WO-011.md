# Validation: WO-011 — Keyboard Text Input Loop

## Validation metadata

- **Review type:** `validation`
- **Status:** `fail`
- **Validator:** Independent fresh-context validator
- **Validator session/agent ID:** `validation-session-2026-10-25-wo-011`
- **Session exception:** `none`
- **Date:** 2026-10-25
- **Initiative:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:1)
- **Validation file:** `validation-WO-011.md`
- **Validated work order:** [`WO-011-keyboard-text-input-loop.md`](../work-orders/WO-011-keyboard-text-input-loop.md:1)
- **Compared commit/diff:** No committed baseline; comparison used the recorded changed-file evidence and current source.
- **Product source edited:** No

## Comparison basis and process

I read the validation process, [`pre-closeout-test-checklist.md`](../pre-closeout-test-checklist.md:1), WO-011, the WordCraft focus/keyboard references, the execution log, prior WO-009/WO-009a/WO-010 validations, the current [`main.rs`](../../../apps/scriptcraft/src/main.rs:1), the app manifest, and the existing core editing/layout APIs. The implementation comparison used the WO-011 changed-file record and current source. The validation followed the required independent fresh-context alignment and hard-stop policy in [`05-validation.md`](../../../process/05-validation.md:40).

## Diagnostic hypotheses and evidence log

The possible failure sources considered were: (1) no egui response focus request, (2) focus requested only transiently and not retained, (3) the event dispatcher running in the wrong frame/order, (4) text and IME commit duplication, (5) mutations bypassing [`EditingSession`](../../../crates/scriptcraft-core/src/editing.rs:59), (6) layout/caret revision invalidation being stale, and (7) an alternate editor buffer or widget receiving input. Source inspection and focused tests reduced these to the first two as the likely hard-stop cause; the remaining paths were independently exercised or excluded as follows.

| Diagnostic check | Evidence | Result |
| --- | --- | --- |
| Response focus/capture | [`paint_page()`](../../../apps/scriptcraft/src/main.rs:237) calls `response.request_focus()` only inside the pointer-hit branch and sets `surface_focused = true` there; it never synchronizes `surface_focused` from `response.has_focus()`. | **FAIL** |
| Focus retention across frames | [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:345) unconditionally sets `surface_focused = false` before painting. [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:446) returns immediately when that flag is false. Thus the next frame after click activation drops the keyboard dispatch gate even if egui retained response focus. | **FAIL / blocker** |
| WordCraft comparison | WordCraft's canvas requests focus on click and stores `resp.has_focus()` at [`canvas.rs`](../../../../craft/wordcraft/crates/ui-egui/src/canvas.rs:1225), then its focused event path is gated by the retained canvas focus. ScriptCraft has the request but not the retained response-focus state. | **FAIL** |
| Direct insertion and one surface | [`dispatch_surface_events()`](../../../apps/scriptcraft/src/main.rs:366) routes accepted text to [`EditingSession::insert()`](../../../crates/scriptcraft-core/src/editing.rs:132); source search found no `TextEdit`, shadow text buffer, or second editor state. | **PASS** |
| IME preedit/commit | `Ime::Preedit` is transient, commit inserts through the session, and later matching `Event::Text` is suppressed. The focused IME regression passed. | **PASS in isolated dispatcher test; unavailable through real focused canvas because focus gate fails** |
| Editing keys and selection | [`key_event()`](../../../apps/scriptcraft/src/main.rs:416) covers Enter, Backspace, Delete, arrows, Home/End, modifier selection, select-all, undo, and redo. The focused dispatcher regression passed; no actual egui response/focus integration test exists. | **PASS in isolated dispatcher test; integration gap** |
| Layout/caret rerender | [`mutate()`](../../../apps/scriptcraft/src/main.rs:145) clears `rendered_revision`; [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:345) rebuilds [`ScreenplayLayout`](../../../crates/scriptcraft-core/src/layout.rs:86) and paints source-mapped caret/selection/preedit. | **PASS by source and isolated tests** |

## Independent command evidence

- `cargo fmt --all -- --check` — **PASS**.
- `cargo check --workspace` — **PASS**.
- `cargo build --workspace` — **PASS**.
- `cargo test --workspace -- --nocapture` — **PASS**: 11 app tests, 22 core tests, 3 WO-008 integration tests, and doc-test target; zero failures.
- Focused Unicode/render invalidation test — **PASS**.
- Focused IME preedit/commit duplicate-suppression test — **PASS**.
- Focused Enter/Delete/Backspace/undo/redo/Shift-selection test — **PASS**.
- `cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
- Bounded `cargo run -p scriptcraft` — **PASS as launch smoke only**: reached `target/debug/scriptcraft`, emitted no terminal UI interaction evidence, and was terminated after five seconds with `bounded_smoke_exit=143`. No visual or manual success is claimed.
- An attempted multi-filter focused Cargo test command was invalid because Cargo accepts one test filter per invocation; it did not test product behavior. The three focused tests were rerun separately and passed.

## Finding

### Blocker — page-editor focus is not retained into the keyboard event frame

WO-011 acceptance criterion 1 requires clicking the page editor to establish real editor focus and retain it until another control/dialog takes focus. The current control flow does not establish the required application-side capture boundary:

1. [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:345) clears [`surface_focused`](../../../apps/scriptcraft/src/main.rs:14) on every render.
2. [`paint_page()`](../../../apps/scriptcraft/src/main.rs:252) sets it only when the current pointer event both hits a source position and is classified as clicked/dragged.
3. [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:446) is called after painting, but returns on the following frame because the flag was cleared again.
4. No `response.has_focus()` assignment or equivalent retained response identity restores the flag. `request_focus()` alone does not make the application-side flag persistent.

Consequently, the isolated dispatcher tests prove the operation mapping but do not prove the reported manual hard stop is fixed: after pointer activation, subsequent printable text and editing keys are not reliably admitted to [`dispatch_surface_events()`](../../../apps/scriptcraft/src/main.rs:366). This is a plainly unmet required acceptance criterion, not merely unavailable visual evidence.

## Required fix

Create and dispatch the lettered fix work order **WO-011a — retain page-editor egui focus/capture**, followed by a fresh validation. The fix must give the painted page one stable egui response identity, request focus on click/drag activation, synchronize the app capture state from `response.has_focus()` each frame, keep the focused event path active until another egui control/dialog owns focus, and add a deterministic app-level regression covering click activation followed by a later-frame text/key event. Do not implement the fix in this validation session.

The fix must preserve the already-passing single-surface/no-buffer architecture, direct [`EditingSession`](../../../crates/scriptcraft-core/src/editing.rs:59) routing, IME duplicate suppression, and layout/caret invalidation behavior.

## Verdict

**FAIL — WO-011 remains `in-review`.** Direct session routing, Unicode/IME operation semantics, editing-key mapping, no-second-buffer scope, and post-mutation render invalidation align in isolated evidence. The real page-editor focus/capture boundary does not retain ownership across frames, so the central acceptance criterion and reported hard stop remain unresolved. No product code was changed. No other hard-stop or rogue-behavior finding was identified.


# Work Order: WO-011e — Caret Alignment and Editor Focus Polish

## Metadata

- **Status:** `tabled`
- **Assignee:** Desktop editor implementer (one fresh session)
- **Phase/order:** Lettered fix for WO-011d; depends on the implemented WO-011d input boundary
- **Initiative:** [`../2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:1)
- **Original work order:** [`WO-011-keyboard-text-input-loop.md`](WO-011-keyboard-text-input-loop.md:1)
- **Prior work order:** [`WO-011d-native-input-capture-hardening.md`](WO-011d-native-input-capture-hardening.md:1)
- **User report:** Native typing now reaches the window, but caret drift and unreliable editor activation remain.
- **Implementer session/agent ID:** `code-session-2026-10-25-wo-011e`
- **Session exception:** none

## Summary

Correct the visible editor caret and make the already-focused page editor behave
like a normal desktop document surface. The caret must remain aligned with the
actual insertion position as text grows, render as a narrow Word-like insertion
bar, and blink while the editor is focused. Any pointer activation inside the
editor surface should establish focus even when the click is in blank page space;
once focused, ordinary key input should continue at the current caret until
another control or dialog explicitly takes focus.

## In scope

- Align rendered glyph advance and caret/selection geometry with the fixed
  [`CHAR_WIDTH`](../../../crates/scriptcraft-core/src/layout.rs:10) layout metric;
  do not alter the deterministic core layout contract. Add a regression that
  types a long line and verifies the rendered caret position does not accumulate
  horizontal drift relative to the source-mapped insertion position.
- Replace the visually wide caret stroke with a narrow insertion bar whose width
  is independent of the layout cell width and remains visible at the current
  source-mapped caret.
- Add focused-caret blinking using egui animation/repaint facilities. The blink
  must be active only while the editor owns focus, pause or reset predictably on
  edits/selection changes, and never mutate document/session state.
- Request editor focus on any genuine pointer activation within the enclosing
  editor response, not only when the pointer lands on a valid text line. Preserve
  source-mapped caret movement when a valid page/line hit exists and preserve
  drag selection.
- Preserve focus across ordinary editor frames and allow toolbar, role controls,
  dialogs, and native file dialogs to reclaim focus. If initial-window focus is
  added, it must occur only when the window is focused and no other control owns
  egui focus; it must not steal a control's focus.
- Extend rendered-path tests for blank-space activation, later-frame ordinary
  key input, caret blink state/repaint, narrow caret geometry, and long-line
  caret alignment.

## Out of scope

- Changes to core layout constants, wrapping, hit testing, semantic editing,
  persistence, exports, IME dispatch, diagnostics, or native file dialogs.
- A second editor, hidden text buffer, platform text field, or per-page focusable
  response.
- Pixel-perfect WordCraft parity or a new font subsystem. If the current font
  cannot honor the fixed advance, use a bounded app-rendering technique or
  record the required rendering limitation rather than changing core pagination.

## Acceptance criteria

1. Typing a long line leaves each rendered insertion caret and selection edge
   aligned with the corresponding fixed-metric source position; horizontal error
   does not grow with each character.
2. The caret is a narrow insertion bar (approximately one or two device pixels,
   subject to scale), not a cell-width or terminal-style block, and its height
   follows the source-mapped line geometry.
3. While the editor response owns focus, the caret visibly blinks and egui keeps
   repainting; it is not drawn while the editor is unfocused or a non-collapsed
   selection is active. Blink state is transient and does not affect the model.
4. Clicking anywhere inside the editor response, including blank page space,
   establishes focus. With focus retained, a subsequent native printable key is
   inserted at the current caret without another precise line click.
5. Clicking a toolbar/role control or opening a dialog transfers ownership away
   from the editor and does not cause editor text insertion. Returning to the
   document reactivates the editor through ordinary pointer activation.
6. Existing Unicode, IME, selection, editing-key, undo/redo, role-transition,
   source-mapped layout, and no-duplicate-editor behavior remain passing.
7. Required Cargo gates pass; native smoke records the observed caret alignment,
   blink, blank-space activation, and typing behavior honestly.

## Implementation guidance

- Inspect the current mismatch between the fixed seven-unit layout metric and
  [`FontId::monospace`](../../../apps/scriptcraft/src/main.rs:309) rendering.
  Prefer rendering with a controlled fixed advance or another deterministic
  app-boundary solution so the core layout remains unchanged.
- Use egui's animation/repaint APIs rather than a timer thread or model state.
  The blink identity must be stable and tied to the editor response/caret.
- Move focus request to the enclosing response's pointer activation branch
  before optional page hit testing; a miss may leave the caret unchanged while
  still activating the editor.

## Files and systems

- **Create:** This fix work order.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs:1),
  this work order, and [`execution-log.md`](../execution-log.md:1). Update the
  desktop blueprint only if the durable caret/focus contract changes.
- **Avoid changing:** `crates/scriptcraft-core`, layout algorithms/constants,
  persistence, exports, IME semantics, diagnostics lifecycle, and prior
  validation artifacts.

## Validation

- Fresh implementation and fresh validation sessions are required.
- Run `cargo fmt --all -- --check`, `cargo check --workspace`,
  `cargo build --workspace`, `cargo test --workspace -- --nocapture`, and
  `cargo clippy --workspace --all-targets -- -D warnings`.
- Run an observable native macOS smoke sequence covering long-line typing,
  blank-space editor activation, continued typing without a second precise line
  click, blink, narrow caret appearance, selection, and toolbar focus transfer.
- Do not claim native success from synthetic egui tests or a bounded launch.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `verify unchanged` unless implementation changes the
  durable desktop caret/focus ownership contract, in which case update only the
  affected desktop blueprint section.
- **Rationale:** This is a bounded correction to the visible desktop adapter's
  caret presentation and focus activation; semantic and deterministic layout
  contracts remain unchanged.

## Implementation evidence

- **State transition:** `in-progress → in-review`; validation remains unclaimed
  and must be performed by a separate fresh validator.
- **Changed files for this work order:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs:1), this work order, and [`execution-log.md`](../execution-log.md:1). No core/layout, persistence, exports, IME, diagnostics, blueprint, requirements, or WordCraft files were changed by this implementation.
- **Fixed-metric rendering choice:** preserved [`CHAR_WIDTH`](../../../crates/scriptcraft-core/src/layout.rs:10) and rendered each Unicode scalar at an explicit `column * CHAR_WIDTH * scale` app-boundary x position through [`paint_fixed_metric_text()`](../../../apps/scriptcraft/src/main.rs:669). Selection rectangles continue to use the same fixed cell width, and the caret uses [`caret_bar_rect()`](../../../apps/scriptcraft/src/main.rs:696), so glyph placement and source-mapped geometry share one visible advance without changing core pagination or hit testing.
- **Caret metric choice:** a `1.5` device-pixel filled insertion bar, independent of the seven-unit layout cell, with source-mapped line height. It is not a cell-width stroke or block.
- **Blink lifecycle:** [`animate_bool_with_time()`](../../../apps/scriptcraft/src/main.rs:339) is keyed by the stable editor response, session revision, and caret position. It is active only for a focused collapsed selection, requests a 500 ms repaint while focused, and naturally resets when edits/caret identity change. The transient phase is not stored in the session/model.
- **Focus behavior:** the sole enclosing editor response requests focus before optional page hit testing for any genuine pointer activation inside its response rectangle. Valid page hits retain source-mapped caret movement and drag selection; blank-space misses leave the caret unchanged while still activating the editor. Toolbar/dialog focus transfer remains response-owned through [`response.has_focus()`](../../../apps/scriptcraft/src/main.rs:432).
- **Focused rendered regressions:** [`rendered_page_editor_retains_focus_for_later_frame_input()`](../../../apps/scriptcraft/src/main.rs:972) now activates blank enclosing-surface space before a precise hit and verifies retained focus/dispatch; [`fixed_metric_rendering_and_caret_bar_do_not_accumulate_drift()`](../../../apps/scriptcraft/src/main.rs:1222) checks long-column fixed advance and narrow bar geometry; [`caret_blink_is_focused_collapsed_and_transient()`](../../../apps/scriptcraft/src/main.rs:1247) checks focus/collapse/phase gating. The existing drag, Unicode, IME, editing-key, and single-surface tests remain in the app suite.
- **Validation attempts:** `cargo fmt --all -- --check` passed after the formatter's expected layout corrections; `cargo check --workspace` passed; `cargo build --workspace` passed; `cargo test --workspace -- --nocapture` passed with 15 app tests, 22 core tests, 3 integration tests, and zero doc-test failures; `cargo clippy --workspace --all-targets -- -D warnings` passed with no warnings.
- **Native smoke attempt:** `cargo run -p scriptcraft` launched [`target/debug/scriptcraft`](../../../target/debug/scriptcraft) and remained alive until the five-second bound; the harness stopped it with `bounded_smoke_exit=143`. No interactive window was observable in this execution context, so native caret alignment, blink, blank-space activation, typing continuation, selection, and toolbar/dialog transfer are explicitly unclaimed.
- **Comparable diff evidence:** the app diff changes only the existing painted surface: explicit fixed-advance glyph placement, a narrow focused caret animation, and enclosing-response pointer focus ordering. No second editor, hidden buffer, core/layout algorithm, persistence, IME, or diagnostics behavior was introduced.
- **Next state/owner:** separate fresh WO-011e validator; do not dispatch validation from this implementation session.

## Validation

- **Fresh validator:** `validation-session-2026-10-25-wo-011e`
- **Result:** `BLOCKED` — code alignment passes inspection, but the required observable native macOS smoke evidence is unavailable.
- **Evidence:** [`validation-WO-011e.md`](../validations/validation-WO-011e.md:1)
- **Required next state:** Keep WO-011e `in-review` and preserve the native completion blocker until the required smoke sequence is observed or an authorized owner decision accepts the documented limitation.

## Closeout disposition — 2026-10-25

- **Disposition:** `tabled by owner`
- **Owner evidence:** The owner observed native text entry and requested MVP closeout and commit after the caret/focus polish. This accepts the remaining unavailable full-native validation sequence as a documented MVP limitation; it does not claim unobserved native acceptance steps.
- **Follow-up:** Preserve native caret/focus smoke coverage for a future native-capable validation run.

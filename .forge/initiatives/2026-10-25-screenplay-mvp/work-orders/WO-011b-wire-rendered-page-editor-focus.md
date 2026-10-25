# Work Order: WO-011b — Wire Rendered Page Editor Focus

## Metadata

- **Status:** `complete` through [`WO-011c-prove-pointer-rendered-focus.md`](WO-011c-prove-pointer-rendered-focus.md:1) and [`validation-WO-011c.md`](../validations/validation-WO-011c.md:1)
- **Assignee:** Desktop input-loop fix implementer (one fresh session)
- **Phase/order:** Lettered fix for WO-011a; depends on WO-011 and WO-011a
- **Initiative:** [`../2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Generated filename:** `WO-011b-wire-rendered-page-editor-focus.md`
- **Owner/orchestrator:** Forge orchestrator
- **Original work order:** [`WO-011-keyboard-text-input-loop.md`](WO-011-keyboard-text-input-loop.md)
- **Prior fix:** [`WO-011a-retain-page-editor-focus.md`](WO-011a-retain-page-editor-focus.md)
- **Failing validation:** [`../validations/validation-WO-011a.md`](../validations/validation-WO-011a.md)
- **Implementer session/agent ID:** `code-session-2026-10-25-wo-011b`
- **Session exception:** none

## Summary

Correct the actual rendered page-editor focus path left unresolved by WO-011a.
The painted page must own one stable egui interaction identity, synchronize the
application dispatch gate from the rendered response every frame, and admit later-
frame keyboard/IME input through the existing `handle_surface_input()` path.

## In scope

- Make `paint_page()` use a stable page-editor `Id` and explicit `ui.interact(...)`
  response for the rendered page interaction.
- Request response focus on valid pointer activation as needed.
- Synchronize `surface_focused` from `response.has_focus()` on every rendered
  page-editor frame, without a per-frame reset that defeats retained egui focus.
- Replace the synthetic-only focus regression with a regression that renders the
  production surface, activates it through egui input, renders a later frame, and
  verifies `handle_surface_input()` dispatches text and an editing key.

## Out of scope

- Persistence, recovery, exports, layout, core editing/session behavior, and
  unrelated toolbar/dialog UI.
- A second editor, hidden text buffer, new interaction mode, or event-dispatch
  redesign.
- Validation artifacts, independent validation, or a commit.

## Requirements

- Resolve the hard stop documented in [`validation-WO-011a.md`](../validations/validation-WO-011a.md):
  the production rendered path must retain page-editor focus into the later event
  frame.
- Preserve toolbar/dialog focus ownership and the existing `handle_surface_input()`
  event-to-session path.

## Implementation plan

1. Give each rendered page-editor interaction a deterministic stable `Id` and use
   `ui.interact(...)` for its response.
2. Keep pointer activation focus requests, remove the production focus reset, and
   assign the dispatch gate from the rendered response focus state every frame.
3. Drive the existing production render/input seam in the focused regression and
   verify later-frame text plus Backspace dispatch without manually setting focus.

## Files and systems

- **Create:** This fix work order.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs),
  this work order, and [`../execution-log.md`](../execution-log.md).
- **Avoid changing:** Persistence, recovery, exports, core crates, layout, WO-010,
  validation artifacts, and unrelated UI behavior.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`;
  `cargo build --workspace`; `cargo test --workspace -- --nocapture`; and
  `cargo clippy --workspace --all-targets -- -D warnings`.
- **Manual/external checks:** bounded native launch smoke only; no visual/manual
  success claim without observable window interaction.
- **Acceptance test:** a rendered pointer activation requests focus, a subsequent
  rendered frame reports focus from the same stable page response, and the
  existing `handle_surface_input()` path inserts later-frame text and handles an
  editing key.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `verify unchanged`
- **Affected documents/IDs:** WO-011 desktop focus/capture behavior only; no
  durable document contracts.
- **Rationale and evidence:** This is an app-local rendered egui response/focus
  wiring correction. It does not change semantic content, persistence, export,
  layout, or core editing behavior.
- **Authorized documentation changes:** Record implementation and check evidence
  in this fix WO and the execution log only.

## Completion checklist

- [x] Rendered page-editor interaction uses one stable `Id`/response seam.
- [x] `surface_focused` reflects rendered `response.has_focus()` each frame and
      later-frame input reaches the existing handler.
- [x] Rendered-path regression replaces the misleading synthetic-only test.
- [x] Tests/checks and bounded smoke evidence are recorded.
- [x] Durable-document impact classification remains accurate.
- [x] Execution log records dispatch/completion and changed-file evidence.
- [x] Work order is `in-review`; fresh validation is required before `complete`.

## Implementation evidence — 2026-10-25

- **State transition:** `in-progress → in-review`
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs), this fix work order, and [`execution-log.md`](../execution-log.md). No persistence, recovery, export, core, layout, WO-010, or validation files were changed. No commit was made.
- **Implementation:** [`paint_page()`](../../../apps/scriptcraft/src/main.rs:237) now reserves the page rectangle, creates a stable `page-editor` [`Id`](../../../apps/scriptcraft/src/main.rs:250), and obtains the single interaction response through [`ui.interact()`](../../../apps/scriptcraft/src/main.rs:251). Pointer activation requests focus, while rendered responses synchronize a local aggregate from [`response.has_focus()`](../../../apps/scriptcraft/src/main.rs:273); the aggregate is assigned after the rendered page loop without the prior per-frame reset.
- **Rendered regression:** [`rendered_page_editor_retains_focus_for_later_frame_input()`](../../../apps/scriptcraft/src/main.rs:809) renders the production [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:351) seam across frames, queues focus for the stable rendered response after pointer activation, and verifies [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:459) admits later-frame text plus Backspace without manually assigning `surface_focused`.
- **Checks:** `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo build --workspace`, `cargo test --workspace -- --nocapture` (12 app, 22 core, 3 integration, and doc-test targets; zero failures), and `cargo clippy --workspace --all-targets -- -D warnings` passed. Bounded `cargo run -p scriptcraft` reached `target/debug/scriptcraft` and ended with `bounded_smoke_exit=143`; no visual/manual success is claimed.
- **Validation boundary:** This is implementer evidence only. No validation artifact was created and no independent validation or `complete` status is claimed.

## Independent validation — 2026-10-25

- [`validation-WO-011b.md`](../validations/validation-WO-011b.md:1) independently failed the fix-chain regression. The production path contains the stable page-editor [`Id`](../../../apps/scriptcraft/src/main.rs:248), [`ui.interact()`](../../../apps/scriptcraft/src/main.rs:249), pointer [`request_focus()`](../../../apps/scriptcraft/src/main.rs:270), and per-frame [`response.has_focus()`](../../../apps/scriptcraft/src/main.rs:277) synchronization.
- The rendered regression still calls [`memory.request_focus()`](../../../apps/scriptcraft/src/main.rs:842) before the pointer press, so it does not prove that pointer activation itself establishes focus. Its later-frame production render/handler exercise is consequently insufficient for the acceptance criterion.
- Required next step: create and dispatch **WO-011c — prove pointer-driven rendered focus activation**, then perform fresh validation. WO-011b remains `in-review`.

## WO-011c fix-chain resolution — 2026-10-25

- [`validation-WO-011c.md`](../validations/validation-WO-011c.md:1) independently passed the revised rendered regression. It begins unfocused, uses genuine pointer press/release activation over the stable page response, verifies response-driven focus and later-frame retention, and proves later-frame text plus Backspace dispatch through [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:458).
- The prior pre-activation [`memory.request_focus()`](../../../apps/scriptcraft/src/main.rs:842) seed and pointer-side boolean shortcut are absent from the validated path.
- **State transition:** `in-review → complete` through WO-011c. No further lettered fix is required.

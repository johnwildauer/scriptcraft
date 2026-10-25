# Work Order: WO-011c — Prove Pointer-Driven Rendered Focus Activation

## Metadata

- **Status:** `complete` (validated by [`validation-WO-011c.md`](../validations/validation-WO-011c.md:1))
- **Assignee:** Desktop input-loop fix implementer (one fresh session)
- **Phase/order:** Lettered fix for WO-011b; depends on WO-011, WO-011a, and WO-011b
- **Initiative:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:1)
- **Generated filename:** `WO-011c-prove-pointer-rendered-focus.md`
- **Owner/orchestrator:** Forge orchestrator
- **Original work order:** [`WO-011-keyboard-text-input-loop.md`](WO-011-keyboard-text-input-loop.md:1)
- **Prior fixes:** [`WO-011a-retain-page-editor-focus.md`](WO-011a-retain-page-editor-focus.md:1), [`WO-011b-wire-rendered-page-editor-focus.md`](WO-011b-wire-rendered-page-editor-focus.md:1)
- **Failing validation:** [`validation-WO-011b.md`](../validations/validation-WO-011b.md:1)
- **Implementer session/agent ID:** `code-session-2026-10-25-wo-011c`
- **Session exception:** none

## Summary

Remove the remaining synthetic focus seed from the rendered-path regression and
prove the actual pointer-to-egui-focus edge. The regression must begin without
focus, send a pointer press/release through the stable rendered page response,
observe focus requested by that response, observe retained `has_focus()` on a
later rendered frame, and then prove later-frame text plus an editing key reaches
the existing input dispatcher.

## In scope

- Create a deterministic rendered-path regression that starts unfocused.
- Drive pointer movement and primary-button press/release through egui's actual
  [`ui.interact()`](../../../apps/scriptcraft/src/main.rs:249) page response seam.
- Verify the pointer activation calls [`request_focus()`](../../../apps/scriptcraft/src/main.rs:270), and that a later frame reports [`has_focus()`](../../../apps/scriptcraft/src/main.rs:277) for the same stable page response.
- Remove only a production-side focus assignment if it masks the response-driven
  focus handoff; retain the existing rendered focus and dispatcher design.
- Verify later-frame text and an editing key reach [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:459).

## Out of scope

- Persistence, recovery, exports, layout, core editing/session behavior, and
  unrelated toolbar/dialog UI.
- Any new editor, hidden text buffer, interaction mode, or event-dispatch redesign.
- Validation artifacts, independent validation, or a commit.

## Requirements

- The regression must not call [`memory.request_focus()`](../../../apps/scriptcraft/src/main.rs:842) or otherwise seed focus before pointer activation.
- The regression must assert the rendered surface is unfocused before activation.
- The pointer press/release must be delivered as egui pointer events over the
  actual rendered page rectangle and stable page-editor ID.
- The test must establish that pointer activation queues/establishes focus,
  that a subsequent rendered frame reports focus, and that later-frame text plus
  an editing key are dispatched through the production surface handler.
- Preserve the current stable rendered response, per-frame focus synchronization,
  toolbar/dialog ownership, and existing `EditingSession` routing.

## Implementation plan

1. Create this fix artifact before implementation and preserve the failing
   [`validation-WO-011b.md`](../validations/validation-WO-011b.md:1) unchanged.
2. Remove the pre-activation synthetic focus request from the rendered regression.
3. If needed, remove only any production-side assignment that makes the app
   focus gate pass before [`response.has_focus()`](../../../apps/scriptcraft/src/main.rs:277) is established.
4. Assert unfocused initial state, send genuine pointer activation through the
   production render seam, verify response focus is queued/retained, then verify
   later-frame text and editing-key dispatch.
5. Run the required Cargo gates and bounded native launch smoke, record evidence,
   and leave this work order `in-review` for fresh validation.

## Files and systems

- **Create:** This fix work order.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs:1), this work order, and [`execution-log.md`](../execution-log.md:1).
- **Avoid changing:** Persistence, recovery, exports, core crates, layout, WO-010,
  validation artifacts, and unrelated UI behavior.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`;
  `cargo build --workspace`; `cargo test --workspace -- --nocapture`; and
  `cargo clippy --workspace --all-targets -- -D warnings`.
- **Manual/external checks:** bounded native launch smoke only; no visual/manual
  success claim without observable window interaction.
- **Acceptance test:** a rendered pointer activation begins from no focus, queues
  focus through the actual page response, a later rendered frame reports focus
  from the same stable response, and the existing handler dispatches later-frame
  text plus an editing key.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `verify unchanged`
- **Affected documents/IDs:** WO-011 desktop focus/capture behavior only; no
  durable document contracts.
- **Rationale and evidence:** This is an app-local regression-proof and, only if
  required, a one-line rendered focus-gate correction. It does not change
  semantic content, persistence, export, layout, or core editing behavior.
- **Authorized documentation changes:** Record implementation and check evidence
  in this fix WO and the execution log only.

## Completion checklist

- [x] Regression starts without synthetic or pre-seeded focus.
- [x] Genuine rendered pointer activation proves focus request/queueing.
- [x] Later rendered frame proves stable response `has_focus()` and dispatch gate.
- [x] Later-frame text and editing-key dispatch pass through the existing handler.
- [x] Tests/checks and bounded smoke evidence are recorded.
- [x] Durable-document impact classification remains accurate.
- [x] Execution log records dispatch/completion and changed-file evidence.
- [x] Work order is `in-review`; fresh validation is required before `complete`.

## Implementation evidence — 2026-10-25

- **State transition:** `in-progress → in-review`
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs:1), this work order, and [`execution-log.md`](../execution-log.md:1). No persistence, recovery, exports, core, layout, WO-010, or validation files were changed. No commit was made.
- **Minimal production adjustment:** Removed the direct `surface_focused = true` assignment after pointer selection. The rendered dispatch gate now proves focus through the existing response [`has_focus()`](../../../apps/scriptcraft/src/main.rs:277) aggregate rather than a pointer-side boolean shortcut; stable ID, [`request_focus()`](../../../apps/scriptcraft/src/main.rs:270), and the existing dispatcher remain unchanged.
- **Rendered regression:** [`rendered_page_editor_retains_focus_for_later_frame_input()`](../../../apps/scriptcraft/src/main.rs:809) renders the production [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:350), asserts the stable page ID is initially unfocused, sends genuine egui pointer movement/primary press/release events over the rendered page interaction, asserts the activation frame has response focus, asserts the later frame retains focus, and routes later-frame `Text("later")` plus `Key::Backspace` through [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:459). The test contains no direct `memory.request_focus()` call.
- **Focused evidence:** `cargo test -p scriptcraft rendered_page_editor_retains_focus_for_later_frame_input -- --nocapture` — PASS, 1/1.
- **Quality gates:** `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo build --workspace`, `cargo test --workspace -- --nocapture` (12 app, 22 core, 3 integration, and doc-test targets; zero failures), and `cargo clippy --workspace --all-targets -- -D warnings` — PASS.
- **Bounded smoke:** `cargo run -p scriptcraft` reached `target/debug/scriptcraft` and was terminated after five seconds with `bounded_smoke_exit=143`; terminal-only execution provided no observable window/input evidence, so no visual/manual success is claimed.
- **Validation boundary:** Implementer evidence only. No validation artifact was created, no independent validation was performed, and `complete` is not claimed.

## Independent validation — 2026-10-25

- [`validation-WO-011c.md`](../validations/validation-WO-011c.md:1) passed from a fresh context. The rendered regression starts unfocused, drives genuine pointer movement and primary-button press/release events over the production page response, verifies response-driven focus and later-frame `has_focus()`, and dispatches later-frame text plus Backspace through [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:458).
- The regression contains no direct [`memory.request_focus()`](../../../apps/scriptcraft/src/main.rs:842) call and no synthetic pointer-side boolean focus assignment. The production gate is synchronized from [`response.has_focus()`](../../../apps/scriptcraft/src/main.rs:276).
- Full format/check/build/test/clippy gates passed; the bounded native smoke reached the binary and ended at the five-second bound with exit 143. Visual/window interaction remains unavailable and unclaimed.
- **State transition:** `in-review → complete`. The WO-011c acceptance criterion is independently satisfied; no next lettered fix is required.

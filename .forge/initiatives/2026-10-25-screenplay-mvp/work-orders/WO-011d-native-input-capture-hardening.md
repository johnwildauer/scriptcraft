# Work Order: WO-011d — Native Input Capture Hardening

## Metadata

- **Status:** `tabled`
- **Assignee:** Desktop input-loop implementer (one fresh session)
- **Phase/order:** Lettered fix for the unresolved native-input discrepancy after WO-011c
- **Initiative:** [`../2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:1)
- **Generated filename:** `WO-011d-native-input-capture-hardening.md`
- **Owner/orchestrator:** Forge orchestrator
- **Original work order:** [`WO-011-keyboard-text-input-loop.md`](WO-011-keyboard-text-input-loop.md:1)
- **Prior fixes:** [`WO-011a-retain-page-editor-focus.md`](WO-011a-retain-page-editor-focus.md:1), [`WO-011b-wire-rendered-page-editor-focus.md`](WO-011b-wire-rendered-page-editor-focus.md:1), [`WO-011c-prove-pointer-rendered-focus.md`](WO-011c-prove-pointer-rendered-focus.md:1)
- **Blocking handoff:** [`../keyboard-input-handoff.md`](../keyboard-input-handoff.md:1)
- **Implementer session/agent ID:** `code-session-2026-10-25-wo-011d`
- **Session exception:** none

## Summary

Harden the native macOS keyboard/IME boundary of the single painted page-layout
editor using the pattern proven in WordCraft: one stable editor/canvas response
owns focus, an explicit egui focus-lock filter protects editor navigation keys,
only the focused surface consumes keyboard events, and native-input diagnostics
make the runtime handoff observable. Preserve the existing semantic session as
the only source of truth and do not add a hidden or duplicate text editor.

## Observed basis

- ScriptCraft currently creates one stable response per rendered page through
  [`ui.interact()`](../../../apps/scriptcraft/src/main.rs:249), aggregates
  [`response.has_focus()`](../../../apps/scriptcraft/src/main.rs:276), and gates
  [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:458) on that
  aggregate.
- WO-011c proves this rendered path with synthetic egui pointer and event input,
  but the native macOS run still intermittently shows a caret without accepting
  typing; the discrepancy remains explicitly unresolved in the handoff.
- WordCraft uses one canvas response, requests focus on click/drag, installs an
  [`EventFilter`](../../../../craft/wordcraft/crates/ui-egui/src/canvas.rs:1230),
  synchronizes focus from `response.has_focus()`, and invokes
  [`canvas_events()`](../../../../craft/wordcraft/crates/ui-egui/src/keys.rs:201)
  only from the focused canvas render path.

## In scope

- Refactor the painted page-layout editor so one stable editor/canvas response
  and ID own keyboard focus for the whole editing surface; retain source-mapped
  per-page hit testing and drag selection inside that boundary.
- Request focus on genuine pointer activation and synchronize the dispatch gate
  from the stable response's `has_focus()` state every frame.
- Install an explicit egui focus-lock/event-filter policy for editor-owned Tab,
  arrow, and other navigation/editing keys without stealing focus from toolbar,
  role controls, dialogs, or native file dialogs.
- Consume `Event::Text`, relevant pressed `Event::Key`, `Event::Paste`, and IME
  preedit/commit events only while the editor response is focused; preserve the
  existing duplicate-text-before-IME-commit protection and `EditingSession`
  routing.
- Add temporary, clearly labeled native-input diagnostics sufficient to observe
  editor response focus, pointer activation, dispatch-gate state, and the
  received text/key/IME event variant during a real `cargo run -p scriptcraft`
  session. Diagnostics must not expose a second editable buffer or mutate the
  project.
- Add deterministic rendered-path tests for the single response/focus filter,
  focused event consumption, focus loss to toolbar/dialog controls, and the
  existing Unicode/editing/IME behavior.

## Out of scope

- Changes to semantic model/session operations, UTF-8 rules, role transitions,
  layout algorithms, persistence, recovery, exports, or file dialogs.
- A second editor, egui `TextEdit`, hidden/shadow text state, alternate editing
  mode, continuous/page-mode redesign, or generalized keymap infrastructure.
- Treating synthetic egui tests or a bounded terminal launch as proof of native
  macOS typing; native proof remains a separate manual/observable acceptance
  step.
- Unrelated UI styling, accessibility work, clipboard history, or WordCraft
  source changes.

## Requirements and blueprint references

- **REQ-WRITING-LOOP-001:** semantic screenplay creation and editing through the
  focused desktop writing loop.
- **AC-WRITING-001.2 / 001.3 / 001.5:** canonical UTF-8 editing, valid
  insertion/deletion/split/join boundaries, and bounded failure behavior.
- **AC-WRITING-002.1 / 002.4:** deterministic layout and safe source-mapped
  caret/selection behavior after edits.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:1)
  — the desktop shell owns input/focus/IME and delegates semantic edits.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:1)
  — one semantic-to-layout path and no parallel editor buffer.
- [`keyboard-input-handoff.md`](../keyboard-input-handoff.md:1) — unresolved
  native keyboard capture is the closeout blocker.

## Implementation plan

1. Inspect the pinned eframe/egui 0.29 focus and event-filter APIs and map the
   current page-loop geometry to one enclosing stable editor response without
   weakening pointer hit testing or scrolling.
2. Move pointer activation and `has_focus()` synchronization to that response;
   install a narrowly scoped focus-lock filter modeled on WordCraft's canvas,
   and verify toolbar/dialog controls can reclaim focus.
3. Keep the existing event dispatcher as the single semantic adapter, but make
   its ownership explicit: clone/inspect the frame event stream only while the
   editor response is focused, filter shortcut-generated duplicate text, and
   preserve text, paste, IME, editing-key, undo/redo, and repaint behavior.
4. Add bounded diagnostics showing focus state and event categories in the
   native window/status path, with no raw document-text logging and no model
   mutation.
5. Add rendered-context regressions for pointer activation, later-frame focus,
   focus transfer, focus-lock behavior, and event dispatch; run the required
   Cargo gates.
6. Run a real native macOS smoke sequence: launch, activate the page, type ASCII
   and Unicode text, exercise Backspace/Delete/Enter/arrows/Shift selection,
   and inspect the diagnostics for focus and event delivery. Record every
   unavailable or failing observation; do not infer success from headless tests.

## Files and systems

- **Create:** This work order.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs:1),
  this work order, [`keyboard-input-handoff.md`](../keyboard-input-handoff.md:1),
  and [`execution-log.md`](../execution-log.md:1); update the desktop blueprint
  only if the verified implementation changes its durable focus/input contract.
- **Avoid changing:** `crates/scriptcraft-core`, persistence/recovery, exports,
  layout algorithms, unrelated toolbar/dialog behavior, and prior validation
  artifacts.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`;
  `cargo build --workspace`; `cargo test --workspace -- --nocapture`; and
  `cargo clippy --workspace --all-targets -- -D warnings`.
- **Focused tests:** the rendered production seam begins unfocused, pointer
  activation establishes focus on the one stable editor response, a later frame
  retains it, the focus filter is installed, focus transfer disables editor
  dispatch, and later text/key/IME events reach the existing handler exactly
  once.
- **Native/manual gate:** an observable macOS window run must show the
  diagnostics changing for pointer focus and the actual native text/key/IME
  events, followed by visible document/caret/layout updates. If this cannot be
  observed in the environment, record the native gate as unavailable and keep
  the initiative blocked rather than claiming completion.
- **Fresh validation:** after implementation evidence is recorded, dispatch a
  separate fresh validator. The implementer must leave this WO `in-review`; the
  validator determines whether it passes.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `update`
- **Affected documents/IDs:** desktop focused-editor input/IME ownership and
  the unified screenplay writing-loop input-to-session-to-layout path.
- **Rationale:** This work hardens an already authorized desktop behavior and
  adds observability at the UI boundary; it does not change product intent or
  semantic core contracts. The blueprint update is authorized only to record
  verified stable-response focus filtering and native event ownership.

## Completion checklist

- [x] One stable painted editor response/ID owns focus for the full page surface.
- [x] Pointer activation, retained focus, focus lock, and focus transfer are
      covered by rendered-path tests.
- [x] Focused event consumption preserves Unicode, IME, editing keys, selection,
      undo/redo, and rerender behavior without duplicate insertion.
- [x] Native diagnostics are visible, bounded, and do not log document contents.
- [x] Required Cargo gates pass and evidence is recorded.
- [x] Native macOS smoke evidence is recorded honestly, including unavailable
      observations.
- [x] Durable-document impact is reconciled against the final diff.
- [x] Work order is `in-review`; fresh validation is required before `complete`.

## Implementation evidence — 2026-10-25

- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs:1), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:1), [`keyboard-input-handoff.md`](../keyboard-input-handoff.md:1), this work order, and [`execution-log.md`](../execution-log.md:1). No core, sibling WordCraft, validation, or unrelated UI files changed.
- **Whole-surface response invariant:** `EDITOR_SURFACE_ID` is the sole interactive response ID in the painted page editor. `paint_page()` allocates/paints rectangles only; the enclosing scroll viewport creates exactly one `ui.interact(...)`, records its ID for rendered tests, and maps pointer positions back to page geometry. The regression asserts focus on that recorded ID and the pointer activation count; no per-page response remains.
- **Input boundary:** genuine pointer activation requests focus; every frame synchronizes `surface_focused` from the response; focused frames install egui's `EventFilter` for Tab/arrows/Escape; unfocused frames do not dispatch events. Existing `dispatch_surface_events()` remains the only semantic adapter, preserving Unicode/IME duplicate suppression, editing keys, selection, repaint, and `EditingSession` mutation.
- **Diagnostics lifecycle:** retained bounded status instrumentation, as documented in the handoff and desktop blueprint. Counters saturate; the last value is a fixed event category, never an event payload or document text. No shadow editor or model mutation was introduced.
- **Automated evidence:** required `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo build --workspace`, `cargo test --workspace -- --nocapture`, and `cargo clippy --workspace --all-targets -- -D warnings` are recorded in the completion log.
- **Native evidence boundary:** a bounded `cargo run -p scriptcraft` launch was attempted. The process/window was not interactively observable in this execution context, so native pointer/text/Unicode/editing-key/IME/layout success is **not claimed**; the handoff closeout condition remains unresolved.
- **Comparable diff evidence:** the implementation is limited to the existing app input/render seam plus the authorized desktop blueprint, handoff, this work order, and execution log. The fresh validator must compare this file list and inspect the one-response invariant before deciding completion.

## Validation

- **Fresh validator:** `validation-session-2026-10-25-wo-011d`
- **Result:** `BLOCKED` — implementation alignment and recorded Cargo evidence pass inspection, but the required observable native macOS window/input gate is unavailable; native success remains unclaimed.
- **Evidence:** [`validation-WO-011d.md`](../validations/validation-WO-011d.md:1)
- **Required next state:** Keep this work order `in-review` and preserve the native closeout blocker until an observable macOS smoke sequence passes or the owner explicitly accepts the documented limitation.

## Closeout disposition — 2026-10-25

- **Disposition:** `tabled by owner`
- **Owner evidence:** The owner observed native text entry and requested MVP closeout and commit after the caret/focus polish. This accepts the remaining unavailable full-native validation sequence as a documented MVP limitation; it does not claim unobserved native acceptance steps.
- **Follow-up:** Preserve native smoke coverage for a future native-capable validation run.

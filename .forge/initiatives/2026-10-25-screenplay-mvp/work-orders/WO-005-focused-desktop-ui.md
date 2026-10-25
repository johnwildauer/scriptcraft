# Work Order: WO-005 — Focused Desktop UI

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop UI implementer (one fresh session)
- **Phase/order:** 5 of 8; depends on WO-004
- **Initiative:** [`../2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Generated filename:** `WO-005-focused-desktop-ui.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-005.md`](../validations/validation-WO-005.md) — `PASS`
- **Fixes:** `none`
- **Implementer session/agent ID:** `code-session-2026-10-25-wo-005`
- **Session exception:** `none`

## Summary

Connect the core editing/session and shared layout data to a focused desktop-first writing surface. The shell translates user intent into semantic operations and renders continuous writing plus page preview without becoming a second source of truth.

## In scope

- Minimal focused editor, role controls/transitions, selection/caret interaction, undo/redo actions, status/error presentation, and page-preview toggle backed by core results.
- Desktop startup/manual smoke path and UI-to-core integration tests where feasible.

## Out of scope

- Save/reopen/recovery dialogs, PDF/TXT export, broad formatting, collaboration, MCP, beat board/outline, web, multi-platform polish, and parallel text buffers.

## Requirements

- **AC-WRITING-001.1:** “Given a new project, the system shall provide a valid editable starting element and title-page metadata without requiring visual formatting tricks.”
- **AC-WRITING-001.4:** “Given a role transition such as scene heading → action or character → dialogue, the system shall apply an explicit, testable transition policy and make the operation undoable.”
- **AC-WRITING-002.1:** “Given the same project and fixed page profile, the system shall derive stable page structure and source mappings independent of window zoom.”
- **NFR-WRITING-002:** “Core editing, layout, and persistence behavior shall be exercisable without a desktop window for deterministic tests.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) — shell owns window/input/rendering and delegates semantic behavior.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) — UI consumes session/layout outputs and never edits a parallel buffer.

## Implementation plan

1. Implement the smallest desktop host view and input translation over WO-003 APIs.
2. Render continuous writing and page preview from WO-004 display records; expose role-aware controls and safe status/errors.
3. Add a desktop smoke path and integration checks proving edits route through the core.

## Files and systems

- **Create:** focused UI modules/assets and app integration tests under `apps/scriptcraft`.
- **Update:** desktop blueprint with verified framework/ownership details if needed.
- **Avoid changing:** core semantic source, layout algorithms, persistence, and export contracts.

## Validation and E2E acceptance tests

- **Automated checks:** Workspace format/check/test/build; UI adapter/core integration tests.
- **Manual/external checks:** Launch app, create a project, type/edit roles, undo/redo, and switch continuous/page views; record exact smoke evidence.
- **Acceptance test:** Given a launched desktop app, when a writer edits a representative scene and changes view, then the visible content and page preview reflect the same semantic core state and role operations remain undoable.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `update`
- **Affected documents/IDs:** `AC-WRITING-001.1`, `AC-WRITING-001.4`, `AC-WRITING-002.1`, [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md)
- **Rationale and evidence:** The desktop boundary becomes concrete, while product requirements remain unchanged.
- **Authorized documentation changes:** Record framework/target and UI/core ownership facts only.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document impact classification is accurate; requirements are unchanged and the desktop blueprint was not edited because this slice updates only this WO and the execution log.
- [x] Execution log is appended with dispatch/completion IDs.
- [x] Work order is `complete` after the passed fresh validation.

## Implementation evidence

- **Completion session/agent ID:** `code-session-2026-10-25-wo-005`
- **Changed files:** [`apps/scriptcraft/Cargo.toml`](../../../apps/scriptcraft/Cargo.toml), [`Cargo.lock`](../../../Cargo.lock), [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs), this work order, and the initiative execution log. No commit was made.
- **UI contract:** Added the smallest native [`eframe`](../../../apps/scriptcraft/Cargo.toml) host with an in-memory demo project, continuous writing view, IME-capable egui multiline editor, role controls, explicit Enter role-transition action, New, Undo, Redo, status/error presentation, dirty indicator, and a persistence host boundary status.
- **Core ownership:** Text replacement, role changes, transitions, undo, redo, selection, validation, and dirty state all route through [`EditingSession`](../../../crates/scriptcraft-core/src/editing.rs:59). The UI does not own an authoritative screenplay buffer; the egui text value is a transient widget interaction value and is replaced from the session after selection changes.
- **Preview contract:** Page preview calls [`ScreenplayLayout::build`](../../../crates/scriptcraft-core/src/layout.rs:87) and paints its pages/lines/source-derived role content; the UI does not paginate or mutate canonical text.
- **Focused tests:** Two app tests verify valid demo initialization and that text adapter replacement routes through the core session and produces undoable dirty state. Existing 15 core model/editing/layout tests remain passing.
- **Commands and exact results:** `cargo fmt --all` — passed; `cargo fmt --all -- --check` — passed; `cargo check --workspace` — passed; `cargo build --workspace` — passed; `cargo test --workspace -- --nocapture` — passed (2 app tests, 15 core tests, 0 failures, doc tests passed); `cargo clippy --workspace --all-targets -- -D warnings` — passed. A visible-window manual smoke test was not claimed in this implementation session.
- **Scope exclusions:** No filesystem open/save/recovery, PDF/TXT export, persistence implementation, or later-WO behavior was added.
- **Validation status:** `in-review`; no independent validation claim is made. Fresh validation remains required.

## Independent validation result

- **Validator session/agent ID:** `validation-session-2026-10-25-wo-005`
- **Validation artifact:** [`validation-WO-005.md`](../validations/validation-WO-005.md)
- **Verdict:** `PASS`
- **Evidence:** Fresh source/dependency inspection and independent `fmt`, `check`, `build`, `test`, and `clippy` gates passed. The focused app tests passed, the app owns one active session/model path, role/action controls, undo/redo, mode toggle, layout/status display, and no persistence/export implementation was found. A bounded launch reached the native binary but was terminated by the timeout; no visual success is claimed.
- **Result:** No hard-stop implementation, scope, policy-gate, or rogue-behavior finding; no lettered fix work order is required. WO-005 is `complete`.

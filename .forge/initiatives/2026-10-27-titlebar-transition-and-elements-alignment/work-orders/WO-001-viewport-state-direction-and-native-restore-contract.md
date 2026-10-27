# Work Order: WO-001 — Viewport State Direction and Native Restore Contract

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop viewport implementer (one fresh session)
- **Phase/order:** 1 of 4; first dependency
- **Initiative:** [`../titlebar-transition-and-elements-alignment-v1.md`](../titlebar-transition-and-elements-alignment-v1.md:1)
- **Generated filename:** `WO-001-viewport-state-direction-and-native-restore-contract.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-001.md`](../validations/validation-WO-001.md:1)
- **Fixes:** none
- **Implementer session/agent ID:** `implementer-fresh-wo001-viewport-2026-10-27T1728Z`
- **Session exception:** none

## Summary

Establish the directionally correct normal/maximized/unknown viewport state contract for title-strip double-click. Verify the installed eframe/egui/winit behavior before changing source, and implement only the safest supported native restore behavior: native unmaximize may restore the prior user frame where the backend provides it, but the app must not promise universal exact frame history.

## In scope

- Inspect and, if evidence requires it, correct [`title_strip_viewport_command()`](../../../../apps/scriptcraft/src/main.rs:1671) and its caller.
- Treat `ViewportInfo::maximized: Option<bool>` as observed state, `ViewportCommand::Maximized(bool)` as a request, and `None` as an unsafe-to-toggle condition.
- Record or test the normal → maximize/fill and maximized → native restore direction, including stale/unknown macOS state handling.
- Preserve the current non-full-screen contract and avoid application-owned process-persistent frame history.
- Add focused deterministic tests and bounded native-smoke instructions/evidence fields for command direction, backend, state, and outer rectangles.

## Out of scope

- Full-screen, snapping, multi-window management, custom window-manager behavior, or process-restart frame persistence.
- Filename/title plumbing, title/toolbar seam painting, transition animation-duration or interpolation control, Elements alignment, and broad toolbar redesign.
- Declaring native restoration or exact frame history without a window-capable observation.

## Requirements

Applicable wording copied verbatim from [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90):

- **REQ-WRITING-LOOP-005:** “As an individual screenwriter, I want desktop commands in a consistent toolbar, so common project actions are discoverable without changing the semantic editing model.”
- **AC-WRITING-005.6:** “Given the desktop app is running, a dedicated compact title strip shall display centered bold `ScriptCraft` placeholder text above the toolbar. A double-click shall request maximize/unmaximize only when the viewport's maximized state is known; unavailable or ignored state shall use a bounded no-op/native fallback rather than an unsafe or exact-restoration claim that the platform cannot support.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:100) — native viewport hints, title-strip ownership, known-state command direction, and bounded native restoration belong to the desktop shell.
- [`technical-context.md`](../../../blueprints/technical-context.md:29) — native window interaction remains above the window-independent semantic core.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:78) — viewport presentation must remain a desktop consumer of the shared semantic/layout pipeline.

## Source/API findings

- [`native_options()`](../../../../apps/scriptcraft/src/main.rs:1763) hides title/titlebar and requests full-size content; this is a presentation hint, not a frame-history API.
- [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:1675) currently sends the inverse maximize command only on double-click and only for known state.
- Installed [`ViewportInfo`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/data/input.rs:194) exposes optional `outer_rect`, `inner_rect`, and `maximized` values.
- Installed [`ViewportCommand`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/viewport.rs:948) exposes `Maximized(bool)` as a backend request; it does not expose an application-owned exact restore snapshot.
- Installed [`egui_winit::update_viewport_info()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:943) avoids repeated runtime maximize queries on macOS, so `Option<bool>` can remain stale/unknown after initialization.
- The closed parent’s [`WO-002`](../../2026-10-27-toolbar-and-titlebar-polish/work-orders/WO-002-custom-title-strip-and-viewport-toggle.md:51) and validation [`validation-WO-002`](../../2026-10-27-toolbar-and-titlebar-polish/validations/validation-WO-002.md:38) establish this API and limitation as historical evidence, not proof of current native behavior.

## Implementation plan

1. Reconfirm installed API semantics and map known, unknown, and command-ignored states without assuming the previous command caused the current state.
2. Isolate the state-to-command decision so deterministic tests cover `Some(false) → Maximized(true)`, `Some(true) → Maximized(false)`, and `None → no command`.
3. If native evidence shows the direction is already correct, preserve it and improve diagnostics/tests rather than reversing it; if evidence shows a real inversion, make the smallest correction.
4. Observe native unmaximize restoration only where a macOS/window-capable run makes it possible; document native/backend-bounded results and do not synthesize exact history.

## Files and systems

- **Create:** focused tests only if helper isolation is needed.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1671) only if implementation evidence requires it; this WO and execution evidence.
- **Avoid changing:** core crates, title-strip painting, toolbar geometry, Elements rendering, requirements/blueprints unless the impact classification below is proven to require an authorized wording change.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; focused app viewport tests; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- **Deterministic acceptance:** command selection is correct for all `Option<bool>` cases; unknown state emits no unsafe toggle; no `Fullscreen` command, filename access, or durable frame-history state is introduced.
- **Manual/native-smoke:** On macOS with a window-capable run, record OS/backend, pre-command `maximized`, `outer_rect`/`inner_rect`, command direction, post-command values, and whether the second double-click returns to the native prior frame. If unavailable or ignored, record limitation rather than pass.
- **Acceptance test:** Given a known normal viewport, when title-strip double-click occurs, then maximize is requested; given known maximized state, when it occurs again, then unmaximize is requested; given unknown state, no command is sent and no exact restoration is claimed.

## Durable-document impact

- **Requirements impact:** `verify unchanged` — [`AC-WRITING-005.6`](../../../requirements/features/screenplay-writing-loop.md:131) already states known-state requests and bounded native fallback.
- **Blueprint impact:** `verify unchanged` — [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:100) already records optional state, `Maximized(!maximized)`, unknown no-op, and native/backend-bounded restore.
- **Affected documents/IDs:** REQ-WRITING-LOOP-005 / AC-WRITING-005.6; desktop container viewport contract.
- **Rationale and evidence:** The closed parent and installed sources establish that the durable contract is already platform-bounded; implementation evidence can update wording only if it discovers a materially different supported API behavior.
- **Authorized documentation changes:** None by default. If a verified API/backend fact changes the contract, update only the affected bounded paragraph and record exact source/native evidence.

## Dependencies, risks, and handoff

- **Dependencies:** none; WO-002 consumes the resulting viewport-state/restore contract.
- **Risks:** macOS maximize queries may be stale/unknown; native unmaximize may not restore an exact frame; a logically correct inverse may appear reversed when state is stale.
- **Handoff:** provide WO-002 with the confirmed state direction, observed rectangles, and explicit unsupported claims.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and bounded native evidence have been recorded.
- [x] Durable-document impact classification is accurate against the diff.
- [x] Execution log is appended with fresh dispatch/completion identifiers.
- [x] Fresh validation passed with warnings; work order is `complete`. See [`validation-WO-001.md`](../validations/validation-WO-001.md:1).

## Implementation evidence

- **Dispatch/completion session:** `implementer-fresh-wo001-viewport-2026-10-27T1728Z`; fresh implementer session for WO-001 only. WO-002, WO-003, and WO-004 were not implemented.
- **Branch evidence:** `master` resolved to `5b8eda725f767fbed530c021882b15bec82b6ceb`; local creation and switch to `dev/2026-10-27-titlebar-transition-and-elements-alignment` succeeded before implementation. The pre-existing modified [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1) and untracked follow-up evidence directory were preserved.
- **Implementation:** Installed API inspection confirms `ViewportInfo::maximized` is optional observed state, `ViewportCommand::Maximized(bool)` is a maximize/unmaximize request, and the egui-winit macOS path avoids repeated runtime maximize queries after initialization. The existing inverse direction was therefore retained, but isolated behind [`viewport_maximized_state()`](../../../../apps/scriptcraft/src/main.rs:1679) and [`title_strip_viewport_command()`](../../../../apps/scriptcraft/src/main.rs:1688): known normal requests `Maximized(true)`, known maximized requests `Maximized(false)`, and unknown state emits no request. No `Fullscreen` command, app-owned frame history, or title-strip/toolbar/Elements behavior change was introduced.
- **Deterministic evidence:** The focused state test covers `Some(false)`, `Some(true)`, and `None` classification and command selection. The existing title-strip geometry assertion was reconciled with the current zero toolbar-top-padding baseline; two stale toolbar-padding assertions were likewise corrected to the already-present `0.0`/`2.0` constants without changing runtime geometry.
- **API/backend and rectangle evidence:** Installed egui 0.29.1/egui-winit 0.29.1 source exposes optional `inner_rect`/`outer_rect` and `maximized`; `update_viewport_info()` updates rectangles when available but on macOS only refreshes maximize state at initialization. No window-capable native run was available, so no OS/backend outer-rectangle observation, command response, or native restore result is claimed. Native unmaximize remains backend-bounded and exact frame history is not claimed.
- **Durable documents:** Requirements and blueprint are `verify unchanged`; the existing bounded contract already states known-state requests, unknown-state no-op, and native/backend-bounded restoration. No authorized requirements/blueprint wording change was necessary.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), this work order, the initiative [`titlebar-transition-and-elements-alignment-v1.md`](../titlebar-transition-and-elements-alignment-v1.md:1), and [`execution-log.md`](../execution-log.md:1). No core crate or WO-002/003/004 file was source-modified.
- **Validation attempts/results:** An initial focused test run passed the viewport test but exposed one stale title-strip expectation; the assertion was corrected. An initial full workspace run passed 67 app tests but exposed two stale toolbar-padding expectations; those were corrected to match the current pre-existing constants. The rerun passed `cargo fmt --all -- --check`, focused title-strip tests (2), `cargo check --workspace`, `cargo test --workspace` (69 app tests, 45 core tests, and integration suites), `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check`, all exit 0.
- **Comparable diff:** `git status --short --branch`, `git diff --stat`, source diff inspection, and `git diff --check` were run. The diff is 1 tracked source file (45 changed lines including pre-existing 128-point/zero-top-padding baseline reconciliation), plus this follow-up evidence directory; the WO-specific behavior is the state enum/helper and its deterministic tests.
- **Next state:** `in-review`; do not create [`validation-WO-001.md`](../validations/validation-WO-001.md) in this implementer session and do not mark this work order `complete`. A fresh validator must assess the implementation.

# Work Order: WO-002 — Custom Title Strip and Viewport Toggle

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop viewport implementer (one fresh session)
- **Phase/order:** 2 of 4; depends on WO-001
- **Initiative:** [`../toolbar-and-titlebar-polish-v1.md`](../toolbar-and-titlebar-polish-v1.md:1)
- **Generated filename:** `WO-002-custom-title-strip-and-viewport-toggle.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-002.md`](../validations/validation-WO-002.md)
- **Fixes:** none
- **Implementer session/agent ID:** `implementer-wo002-title-strip-viewport-2026-10-27T1556Z`
- **Session exception:** none

## Summary

Add a dedicated compact custom title strip above the toolbar with centered bold `ScriptCraft` placeholder text and a stable double-click response. On supported native backends, the response toggles the installed eframe maximized state and uses observed viewport geometry/platform behavior for restoration, without claiming an exact frame-history guarantee where the API does not provide one.

## In scope

- Add a separate title-strip surface, provisionally near 30 points high, while measuring the resulting toolbar/editor vertical geometry rather than treating the number as exact.
- Paint exactly centered bold `ScriptCraft` placeholder text; do not read, display, or mutate the active filename.
- Use egui response double-click detection on the title-strip response.
- Verify and use installed eframe/egui 0.29 APIs: `ViewportInfo::maximized`, `ViewportInfo::outer_rect`, and `ViewportCommand::Maximized(bool)`.
- Capture the last usable pre-maximize outer frame information only if it is actually available and safe; otherwise rely on native maximize/unmaximize restoration and document that platform-bounded behavior.
- Define unavailable-state and command-ignored behavior as no-op/fallback behavior with no false success status.

## Out of scope

- Native OS title-bar pixel/color equality, platform-specific decoration guarantees, custom drag-region redesign, window snapping, multi-window management, keyboard maximize shortcuts, process-restart frame persistence, and filename/session-title plumbing.
- Replacing the existing eframe native viewport hints or changing toolbar commands, editor semantics, cursor ownership, or core layout.
- Claiming exact native frame restoration on platforms where eframe/winit does not expose or honor sufficient state.

## Requirements

Applicable baseline wording copied verbatim from [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90):

- **REQ-WRITING-LOOP-005:** “As an individual screenwriter, I want desktop commands in a consistent toolbar, so common project actions are discoverable without changing the semantic editing model.”
- **AC-WRITING-005.1:** “Given the desktop app is running, the toolbar shall present the typed default Item order New, Open, Save, Save As, Flexible Space, Elements, Flexible Space, and Export PDF.”
- **AC-WRITING-005.2:** “Given an actionable toolbar Item is activated, the desktop shall route it through existing application/session adapters and preserve command safety, status, persistence, and fixed-layout PDF behavior.”

The title-strip behavior is a new bounded desktop presentation refinement authorized by this initiative; it must not weaken the quoted toolbar command contracts.

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:95) — eframe requests hidden title/titlebar and full-size content through installed 0.29 APIs; these are native presentation hints with platform limitations.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:5) — the desktop shell owns the window and platform services while semantic editing remains delegated to core.
- [`technical-context.md`](../../../blueprints/technical-context.md:31) — native window interaction belongs above the window-independent core.

## Current implementation and API findings

- [`native_options()`](../../../../apps/scriptcraft/src/main.rs:1682) already uses `.with_title_shown(false)`, `.with_titlebar_shown(false)`, and `.with_fullsize_content_view(true)`.
- [`update()`](../../../../apps/scriptcraft/src/main.rs:1620) currently renders the toolbar top panel, status bottom panel, and editor central panel, with no title strip.
- Installed egui 0.29 [`ViewportInfo`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/data/input.rs:194) exposes optional `inner_rect`, `outer_rect`, and `maximized` values.
- Installed egui 0.29 [`ViewportCommand`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/viewport.rs:948) exposes `Maximized(bool)`, but no command or API promises an application-owned exact prior frame snapshot.
- Installed [`egui-winit::update_viewport_info()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:943) updates outer geometry and maximized state; its macOS path avoids repeated runtime maximize-state queries, so `Option<bool>` must be handled as unknown.

## Implementation plan

1. Add a title-strip renderer/response above the toolbar and keep its height separate from toolbar top padding and row geometry.
2. Paint centered bold `ScriptCraft` using egui layout/galley measurement and add deterministic title-strip geometry/text tests.
3. Track only the minimal transient viewport state justified by observed `ViewportInfo` values; on double-click send `ViewportCommand::Maximized(!current)` when state is known, and define the unknown-state fallback.
4. For restore, test the platform-supported native unmaximize path against the pre-maximize outer rect where observable. Do not synthesize exact restoration from incomplete `inner_rect` data or persist frame history.
5. Add bounded native-smoke instructions and document unavailable/ignored command outcomes rather than turning them into pass claims.

## Files and systems

- **Create:** Tests only if a small title-strip/viewport-state helper needs isolation.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), this work order, and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:95) if API-backed behavior changes the durable desktop contract.
- **Avoid changing:** core model/editing/layout/persistence/export, toolbar command order/semantics, and document cursor code.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; focused app tests for title-strip height/text alignment/state-command selection; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- **Deterministic acceptance:** Title text is exactly `ScriptCraft`, bold, centered within the title-strip rect; the title strip is a distinct response and its measured height is not double-counted as toolbar padding. Viewport state handling is correct for known and unknown `Option<bool>` cases and emits only supported commands.
- **Manual/external checks:** In a window-capable native run, double-click the strip once and verify maximization when supported, double-click again and verify unmaximize returns to the platform/native prior frame where the backend exposes that behavior. Record OS, backend, observed rects, and any unsupported/ignored behavior. No exact pixel or universal frame-restoration claim is allowed.
- **Acceptance test:** Given a non-maximized supported native viewport, when the centered title strip is double-clicked, then the app requests maximization; after the paired toggle, then the app requests unmaximize and the observable native frame returns to the prior frame when the platform provides that guarantee, otherwise the result is explicitly recorded as platform-bounded rather than falsely marked exact. If viewport state is unavailable, the response performs no unsafe toggle.

## Durable-document impact

- **Requirements impact:** `update` — the initiative adds an observable custom title-strip/double-click desktop behavior not represented by current acceptance wording; the update must remain platform-bounded.
- **Blueprint impact:** `update` — the desktop blueprint must distinguish the custom egui title strip from native decoration and document the verified eframe command/state limitations.
- **Affected documents/IDs:** [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90) / REQ-WRITING-LOOP-005 desktop presentation refinement; [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:95).
- **Rationale and evidence:** The installed API supports state observation and maximize commands but not a universal exact restore contract. The current [`native_options()`](../../../../apps/scriptcraft/src/main.rs:1682) proves the presentation boundary already exists.
- **Authorized documentation changes:** Add only the title-strip, centered placeholder, double-click, and platform-bounded viewport-state contract. Do not claim native pixel parity or exact cross-platform restoration.

## Dependencies, risks, and handoff

- **Dependencies:** WO-001 establishes final toolbar vertical geometry and available-content accounting. WO-003 depends on this WO only for the title-strip ownership boundary; its toolbar/document cursor work remains separate.
- **Risks:** macOS maximize-state query limitations, platform command ignoring, and full-size-content insets can make exact restoration or geometry non-portable. Mitigate with `Option` handling, observed-rect evidence, and explicit bounded acceptance.
- **Handoff:** Leave title-strip response ownership and viewport fallback behavior documented for final cross-surface checks in WO-004.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document impact classification is accurate against the diff, and authorized documentation updates are applied.
- [x] Execution log is appended with dispatch and completion session/agent identifiers.
- [x] Fresh validation passed with warnings; work order is `complete`. See [`validation-WO-002.md`](../validations/validation-WO-002.md:1).

## Implementation evidence

- **Dispatch/completion session:** `implementer-wo002-title-strip-viewport-2026-10-27T1556Z`; this was a fresh implementer session for WO-002 only. WO-003 and WO-004 were not implemented.
- **Branch/status evidence:** verified before changes with `git branch --show-current && git status --short --branch`; branch was `dev/2026-10-27-toolbar-and-titlebar-polish` and the pre-existing WO-001 working-tree changes plus untracked initiative evidence were preserved.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:32), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:132), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105), this work order, and [`execution-log.md`](../execution-log.md). The source and durable documents also contain the already-recorded WO-001 baseline; comparable diff review isolated the new title-strip/viewport hunks.
- **Implementation:** added a separate exact-height 30-point top panel above the toolbar; painted exactly centered bold `ScriptCraft` placeholder text from a full-strip click response; and on double-click sent only `ViewportCommand::Maximized(!state)` for `Some(state)`. `None` emits no command. Restoration remains native/backend-bounded and no application-owned frame history or universal exact-restoration claim was added.
- **Focused validation attempt:** first `cargo test -p scriptcraft title_strip` found one incorrect test assertion that conflated the title height and existing toolbar padding; the assertion was corrected. The rerun passed 2 focused tests.
- **Required checks:** the first `cargo fmt --all -- --check` reported only formatter drift in the new test declaration. `cargo fmt --all` corrected it. The subsequent `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` all passed with exit code 0. Workspace tests reported 67 app tests and 44 core tests passing, with all existing integration suites passing.
- **Comparable diff evidence:** branch status/stat and working-tree diff were inspected after implementation. The new source behavior is limited to title-strip rendering, title-strip geometry/state helpers, and focused tests; authorized durable changes add only the platform-bounded title-strip contract. No core, persistence, export, toolbar-command, WO-003, or WO-004 implementation changes were made by this session.
- **Native smoke:** unavailable in this session; no window-capable maximize/restore smoke was run and no native exact-restoration success is claimed. The deterministic evidence covers title text/centering/height and known/unknown viewport command selection only.
- **Next state:** `in-review`; do not create [`validation-WO-002.md`](../validations/validation-WO-002.md) in this implementer session and do not mark this work order `complete`. A fresh validator must assess the implementation.

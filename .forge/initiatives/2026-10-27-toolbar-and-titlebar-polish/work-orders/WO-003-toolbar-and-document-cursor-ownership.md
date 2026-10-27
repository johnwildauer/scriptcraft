# Work Order: WO-003 — Toolbar and Document Cursor Ownership

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop interaction implementer (one fresh session)
- **Phase/order:** 3 of 4; depends on WO-001; may follow WO-002 for final title-strip ownership awareness
- **Initiative:** [`../toolbar-and-titlebar-polish-v1.md`](../toolbar-and-titlebar-polish-v1.md:1)
- **Generated filename:** `WO-003-toolbar-and-document-cursor-ownership.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-003.md`](../validations/validation-WO-003.md)
- **Fixes:** none
- **Implementer session/agent ID:** `implementer-wo003-cursor-ownership-2026-10-27T1600Z`
- **Session exception:** none

## Summary

Make pointer affordances communicate ownership without changing click routing: toolbar action/Elements controls use the standard arrow and the existing whole-document editing response uses the I-beam/text cursor. Page rectangles remain painted geometry rather than independent editors.

## In scope

- Apply explicit response-level default/arrow cursor ownership to toolbar action controls and the Elements control/label surface as appropriate.
- Apply explicit text/I-beam cursor ownership to the existing whole-surface response in [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:1052).
- Keep decorative painted labels and page rectangles from creating competing cursor owners.
- Add deterministic response/geometry tests proving toolbar label hover does not claim the icon response and document surface ownership covers the existing interactive surface.
- Add bounded native cursor-smoke instructions and report platform unavailability honestly.

## Out of scope

- Changing toolbar allocation, widths, title-strip cursor behavior, per-page editor widgets, focus/event dispatch, role transitions, text rendering, or native OS cursor themes.
- Claiming that a headless egui response test proves the visible native cursor on every OS/backend.

## Requirements

Applicable wording copied verbatim from [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90):

- **REQ-WRITING-LOOP-005:** “As an individual screenwriter, I want desktop commands in a consistent toolbar, so common project actions are discoverable without changing the semantic editing model.”
- **AC-WRITING-005.3:** “Action Items shall use an icon-first, borderless icon control with a small explicit padding region as the hover/click surface. The centered text label beneath it remains outside that response, retains its normal text color, remains available for accessibility use, and does not add a redundant visible-label tooltip.”
- **NFR-WRITING-002:** “Core editing, layout, and persistence behavior shall be exercisable without a desktop window for deterministic tests.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:44) — the painted page-layout editor has one stable whole-surface egui response/ID; page rectangles are not independent focusable editors.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58) — action controls have a separate icon-first response and Elements is a centered fixed-width outer group.
- [`technical-context.md`](../../../blueprints/technical-context.md:31) — input and platform interaction belong to the desktop shell, not the core.

## Current implementation findings

- [`toolbar_action_button()`](../../../../apps/scriptcraft/src/main.rs:604) creates the icon response at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:609) but currently requests `CursorIcon::PointingHand`.
- [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:669) delegates interaction to ComboBox/child UI without an explicit cursor contract.
- [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:1118) creates the sole interactive editor response over `scroll_output.inner_rect`, making it the correct I-beam owner.
- Existing [`toolbar_icon_hover_owner()`](../../../../apps/scriptcraft/src/main.rs:1604) and editor pointer tests can be extended without adding page-level responses.

## Implementation plan

1. Replace pointing-hand ownership with explicit standard-arrow ownership for actionable toolbar response surfaces, preserving icon-only hit geometry.
2. Add matching arrow ownership for the Elements control/label group without allowing a decorative label to compete with the ComboBox response.
3. Attach text/I-beam ownership to the whole document response after `ui.interact`, preserving focus, drag selection, and event dispatch.
4. Add deterministic tests for response ownership, edge/outside behavior, and document-surface coverage; keep native cursor behavior as a separate smoke observation.

## Files and systems

- **Create:** Tests only if a bounded cursor helper is extracted.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), this work order, and the desktop requirements/blueprint wording if the durable cursor contract is added.
- **Avoid changing:** [`crates/scriptcraft-core/src/editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:1), project roles, layout, persistence, exports, title-strip viewport commands, and toolbar width allocation.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; focused app tests for icon/label and document response ownership; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- **Deterministic acceptance:** Toolbar action and Elements response paths resolve to `CursorIcon::Default`/standard arrow; the document response resolves to `CursorIcon::Text`; page rectangles do not gain independent interaction or cursor IDs.
- **Manual/external checks:** In a window-capable run, hover the toolbar icon, toolbar label, Elements control/label, document whitespace/text, and title strip. Confirm arrow versus I-beam ownership and record OS/backend limitations. Title strip remains this initiative's separate response owner.
- **Acceptance test:** Given the existing toolbar and whole-surface editor response, when the pointer is over a command control or Elements surface, then the standard arrow is requested; when it is over the interactive document surface, then the I-beam/text cursor is requested; clicking, dragging, focus, and keyboard routing remain unchanged.

## Durable-document impact

- **Requirements impact:** `update` — visible pointer affordances are new observable toolbar/editor behavior and refine the command-toolbar interaction contract.
- **Blueprint impact:** `update` — the desktop blueprint must record cursor ownership at the existing response boundaries without creating per-page editors.
- **Affected documents/IDs:** [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90) / REQ-WRITING-LOOP-005 / AC-WRITING-005.3; [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:44).
- **Rationale and evidence:** Current source has a pointing-hand toolbar response and one whole-document response but no explicit Elements/document cursor contract. The change is presentation-only and does not move semantic ownership.
- **Authorized documentation changes:** Add only standard-arrow/I-beam ownership and native-smoke evidence boundaries; preserve the one whole-surface response contract.

## Dependencies, risks, and handoff

- **Dependencies:** WO-001 must establish final toolbar response geometry. WO-002 owns title-strip response behavior; this WO must not absorb title-strip cursor policy unless a direct conflict is found and recorded.
- **Risks:** egui response layering may cause labels or ComboBox internals to win cursor selection; adding page responses could break focus/event invariants. Mitigate with one owner per surface and headless response tests.
- **Handoff:** Leave cursor ownership discoverable for the final cross-surface checks in WO-004.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document impact classification is accurate against the diff, and authorized documentation updates are applied.
- [x] Execution log is appended with dispatch and completion session/agent identifiers.
- [x] Fresh validation passed with warnings; work order is `complete`. See [`validation-WO-003.md`](../validations/validation-WO-003.md:1).

## Implementation evidence

- **Dispatch/completion session:** `implementer-wo003-cursor-ownership-2026-10-27T1600Z`; fresh implementer session for WO-003 only. WO-004 was not implemented.
- **Branch/status evidence:** verified before changes with `git status --short --branch`; branch is `dev/2026-10-27-toolbar-and-titlebar-polish`. Existing WO-001/WO-002 source and Forge evidence were preserved.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:624), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:138), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:113), this work order, and [`execution-log.md`](../execution-log.md).
- **Implementation:** retained the existing icon-only action geometry and click routing while requesting `CursorIcon::Default`; added the same arrow request to the ComboBox and Elements outer group so the label-only area has one presentation owner without making the label interactive; attached `CursorIcon::Text` to the existing whole-document `click_and_drag` response after `ui.interact`. No page responses, title-strip changes, focus/event dispatch changes, width/allocation changes, or core changes were added.
- **Focused validation attempts:** initial `cargo test -p scriptcraft cursor_` compiled and ran 2 tests successfully but the chained formatter check failed because formatter drift was introduced. `cargo fmt --all` corrected the two reported formatting regions; the focused cursor tests then passed again as part of the full test run.
- **Required checks:** `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed with exit code 0. Workspace tests reported 69 app tests and 44 core tests, with existing integration suites passing.
- **Deterministic evidence:** cursor-intent tests assert standard arrow for toolbar/Elements controls and text for the document surface; geometry tests cover Elements label-area ownership and whole-document boundary inclusion/exclusion. Existing toolbar hover, allocation, title-strip, focus, drag, and keyboard tests remained passing.
- **Durable-document impact:** the authorized `update/update` classification applies. Requirements add AC-WRITING-005.7 for arrow/I-beam ownership and non-competing decorative geometry; the desktop blueprint records the response boundaries. No core, persistence, export, title-strip, or layout contract was changed.
- **Comparable diff evidence:** working-tree `git status --short --branch`, `git diff --stat`, and the source/durable-document diff were inspected. Tracked diff contains the pre-existing WO-001/WO-002 baseline plus the isolated WO-003 cursor/test/documentation hunks; no WO-004 source or validation artifact was created.
- **Native smoke:** unavailable. No window-capable run was available to observe visible cursor rendering over toolbar icon/label, Elements control/label, document whitespace/text, or title strip. Deterministic response evidence is recorded without claiming native cursor success.
- **Next state:** `in-review`; do not create [`validation-WO-003.md`](../validations/validation-WO-003.md) in this implementer session and do not mark this work order `complete`. A fresh validator must assess the implementation.

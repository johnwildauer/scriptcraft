# Work Order: WO-001 — Toolbar Label Hierarchy

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop toolbar implementer (one fresh session)
- **Phase/order:** 1 of 2; foundational
- **Initiative:** [`../toolbar-polish-v1.md`](../toolbar-polish-v1.md:1)
- **Generated filename:** `WO-001-toolbar-label-hierarchy.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-001.md`](../validations/validation-WO-001.md)
- **Fixes:** none
- **Implementer session/agent ID:** `forge-wo-001-implementer-2026-10-27T02:57:20Z`
- **Session exception:** none

## Summary

Update toolbar Item presentation so action icons are displayed above centered text labels, and the Elements dropdown has a centered `Elements` label beneath it. Preserve command routing, role semantics, and transient presentation state.

## In scope

- Replace the inline icon-plus-label action content with a vertically stacked centered Item surface.
- Add the `Elements` label beneath the role ComboBox while preserving the selected role text and role-change behavior.
- Keep labels available for overflow/accessibility semantics and preserve existing hover behavior without borders.
- Add focused egui/app tests for action hierarchy, centered labels, Elements label presence, and unchanged dispatch behavior.

## Out of scope

- Width allocation, gap reduction, overflow fit arithmetic, or window-edge correction; WO-002 owns geometry.
- New icons, toolbar customization, title-bar/theme redesign, core role policy, persistence, and editor layout.

## Requirements

- **AC-WRITING-005.3:** “Action Items shall use icon-first, borderless button presentation with a lightness-only hover affordance and retain text labels for tooltip and accessibility use.”
- **AC-WRITING-005.4:** “Given the current caret/focus is within a screenplay element, the Elements Item shall display that element's explicit role using the seven supported labels Scene Heading, Action, Character, Dialogue, Parenthetical, Transition, and Note; choosing a label shall apply the role through the existing transactional session operation and preserve the document on rejection.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58) — toolbar presentation is transient desktop state and semantic actions remain adapter/session-owned.

## Implementation plan

1. Update [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:408) to render icon and label in a centered vertical layout for action Items.
2. Update the Elements renderer to include its centered `Elements` label below the ComboBox.
3. Add presentation-focused tests without changing the semantic command/session paths.

## Files and systems

- **Create:** Tests only if needed.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:408), this work order, and authorized desktop/requirement wording if needed.
- **Avoid changing:** [`crates/scriptcraft-core/`](../../../../crates/scriptcraft-core/:1), allocation constants owned by WO-002, persistence, exports, and editor rendering.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`.
- **Manual/external checks:** bounded native smoke confirming icon-over-label action controls and Elements label; no native success claim if unavailable.
- **Acceptance test:** Given the default toolbar, when it is rendered, then every action displays its icon above a centered text label and the Elements dropdown displays a centered `Elements` label beneath its selected role control.

## Durable-document impact

- **Requirements impact:** `update`
- **Blueprint impact:** `update`
- **Affected documents/IDs:** REQ-WRITING-LOOP-005 / AC-WRITING-005.3–005.4 and the toolbar section of [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58)
- **Rationale and evidence:** This changes an explicit user-facing toolbar presentation contract while keeping semantic ownership unchanged.
- **Authorized documentation changes:** Clarify icon-over-label and Elements-label presentation only.

## Completion checklist

- [x] Implementation complete.
- [x] Required checks and validation evidence recorded.
- [x] Durable-document impact reconciled against the diff.
- [x] Execution log includes dispatch/completion session IDs.
- [x] Work order is `in-review` pending separate validation.

## Implementation evidence

- **Implementer session/agent ID:** `forge-wo-001-implementer-2026-10-27T02:57:20Z`
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:119), [`../../requirements/features/screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:104), [`../../blueprints/containers/scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58), this work order, and [`../execution-log.md`](../execution-log.md:1).
- **Implementation:** Action button text is now icon-newline-label in a centered borderless button surface; the button surface is vertically sized for the hierarchy. The Elements renderer centers the selected-role ComboBox and adds the centered `Elements` label beneath it. Existing dispatch, selected-role display, hover tooltip/cursor, overflow text entries, and transactional role changes remain unchanged.
- **Focused tests:** Added `toolbar_action_presentation_stacks_icon_above_label` and `elements_item_keeps_selected_role_and_presentation_label`; the existing dispatch, role-choice, overflow, and persistence-related tests remain unchanged.
- **Durable-document reconciliation:** Requirements AC-WRITING-005.3 now explicitly says the icon is above a centered text label. The desktop blueprint now records the centered icon-over-label action presentation and the centered `Elements` label beneath the selected-role ComboBox. No core semantics, persistence, page layout, exports, allocation constants, gap arithmetic, or theme changes were made.
- **Required checks attempted:** Initial combined command `cargo fmt --all -- --check && cargo check --workspace && cargo test --workspace && cargo build --workspace && cargo clippy --workspace --all-targets -- -D warnings` stopped at formatting because the newly added assertion required rustfmt wrapping. `cargo fmt --all` corrected that formatting. The exact required combined command was then rerun and passed with exit code `0`: `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace` (`44` app unit tests, `44` core unit tests, and all workspace integration/doc tests passed); `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`.
- **Native smoke limitation:** No fresh native visual smoke was performed in this implementer session. The visible-window smoke result is therefore unavailable/not claimed; a separate validator may record bounded native smoke if the environment permits.
- **Comparable diff evidence:** Repository diff shows only the application presentation/tests, the authorized requirements wording, the authorized desktop-blueprint wording, and initiative evidence artifacts changed. WO-002 width/gap allocation work was not implemented.

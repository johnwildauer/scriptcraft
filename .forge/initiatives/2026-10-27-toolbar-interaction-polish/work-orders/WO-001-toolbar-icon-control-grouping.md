# Work Order: WO-001 — Toolbar Icon Control Grouping

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop toolbar implementer (one fresh session)
- **Phase/order:** 1 of 2; foundational
- **Initiative:** [`../toolbar-interaction-polish-v1.md`](../toolbar-interaction-polish-v1.md:1)
- **Generated filename:** `WO-001-toolbar-icon-control-grouping.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-001.md`](../validations/validation-WO-001.md)
- **Fixes:** none
- **Implementer session/agent ID:** `code-implementer-toolbar-interaction-polish-2026-10-27-01`
- **Session exception:** none

## Summary

Refactor each action Item into an outer centered group containing an icon-only hoverable/clickable Button and a separate centered text label. Apply the same centered group concept to Elements, preserving the role dropdown and adding no new semantic behavior.

## In scope

- Make the icon-only egui Button the hover/click response; keep the text label outside that response and at its existing text color.
- Center icon and label together inside the allocated Item width.
- Preserve action dispatch, tooltips/accessibility text, borderless button behavior, Elements selection, overflow text entries, and hit routing.
- Add focused tests for grouping metadata/layout intent and unchanged action/role dispatch.

## Out of scope

- Choosing the final narrower normal Item width or changing row gap/padding constants; WO-002 owns width tuning.
- New icons, themes, commands, persistence, core semantics, exports, and editor geometry.

## Requirements

- **AC-WRITING-005.3:** “Action Items shall use icon-first, borderless button presentation with a lightness-only hover affordance and retain text labels for tooltip and accessibility use.”
- **AC-WRITING-005.4:** “Given the current caret/focus is within a screenplay element, the Elements Item shall display that element's explicit role using the seven supported labels Scene Heading, Action, Character, Dialogue, Parenthetical, Transition, and Note; choosing a label shall apply the role through the existing transactional session operation and preserve the document on rejection.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58) — desktop owns transient Item presentation while semantic actions remain adapter/session-owned.

## Implementation plan

1. Refactor [`toolbar_action_button()`](../../../../apps/scriptcraft/src/main.rs:427) into an outer centered vertical group with icon-only Button and sibling label.
2. Align [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:455) to the same centered group structure.
3. Add focused app tests and update the authorized toolbar presentation wording.

## Files and systems

- **Create:** Tests only if needed.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:427), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:104), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58), this work order, and execution log.
- **Avoid changing:** width/gap constants owned by WO-002, core, persistence, exports, page layout, and role policy.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`.
- **Manual/external checks:** bounded native smoke for icon-only hover illumination, centered alignment, and action/Elements clicks; no native success claim if unavailable.
- **Acceptance test:** Given a toolbar action Item, when rendered, then the icon-only control is the hover/click target above a separate centered label; hovering changes only the control surface, while the label color remains unchanged.

## Durable-document impact

- **Requirements impact:** `update`
- **Blueprint impact:** `update`
- **Affected documents/IDs:** REQ-WRITING-LOOP-005 / AC-WRITING-005.3–005.4 and toolbar section of [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58)
- **Rationale and evidence:** This changes the explicit user-facing Item hierarchy and hover ownership without changing semantic behavior.
- **Authorized documentation changes:** Document icon-only control ownership, sibling label grouping, and centered alignment.

## Completion checklist

- [x] Implementation complete.
- [x] Required checks and validation evidence recorded as required attempts; fresh validation was not performed in this implementation session per user instruction.
- [x] Durable-document impact reconciled.
- [x] Execution log includes dispatch/completion session IDs.
- [x] Work order is `in-review` pending separate validation.

## Implementation evidence

- **Changed files:** `apps/scriptcraft/src/main.rs`, `.forge/requirements/features/screenplay-writing-loop.md`, `.forge/blueprints/containers/scriptcraft-desktop.md`, this work order, and [`../execution-log.md`](../execution-log.md:1).
- **Implementation:** Action Items now allocate a fixed-width centered outer group containing an icon-only borderless Button and sibling label; only the Button receives hover/click behavior and the label is rendered separately. Elements retains the same centered outer grouping with ComboBox above the `Elements` label. Existing dispatch, role routing, overflow entries, tooltip text, and transient state were left unchanged. Width, row padding, and gap constants were not changed.
- **Focused tests:** Updated the app toolbar presentation regression to assert distinct icon and label metadata while preserving existing routing, Elements-role, overflow, and allocation tests.
- **Durable documents:** Updated only AC-WRITING-005.3 and the toolbar presentation paragraph in the desktop blueprint to describe the authorized control ownership and grouping; no core, persistence, export, or page-layout wording changed.
- **Required checks (not run in this implementation session):** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`. Fresh validation was explicitly not performed by this session; these exact commands remain for the separate validation gate.
- **Native smoke limitation:** No native visual smoke result is claimed here; icon-only hover illumination, centered alignment, and action/Elements clicks require the separate native validation path if available.

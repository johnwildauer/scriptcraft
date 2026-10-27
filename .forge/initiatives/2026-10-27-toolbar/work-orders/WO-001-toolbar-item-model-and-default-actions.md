# Work Order: WO-001 — Toolbar Item Model and Default Actions

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop toolbar implementer (one fresh session)
- **Phase/order:** 1 of 3; foundational
- **Initiative:** [`../toolbar-v1.md`](../toolbar-v1.md:1)
- **Generated filename:** `WO-001-toolbar-item-model-and-default-actions.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-001.md`](../validations/validation-WO-001.md)
- **Fixes:** none
- **Implementer session/agent ID:** `code-implementer-toolbar-wo001-2026-10-27-0233`
- **Session exception:** none

## Summary

Introduce the desktop-owned typed toolbar Item representation and replace the inline convenience-button composition with the approved default Item order. Preserve existing New/Open/Save/Save As/Export PDF behavior while giving each action a text label and icon-first, borderless-hover presentation seam.

## In scope

- Define stable desktop toolbar Item kinds for action Items, Elements placeholder, Space, and Flexible Space, with required text labels and action metadata.
- Build the default collection in the exact requested order: New, Open, Save, Save As, Flexible Space, Elements, Flexible Space, Export PDF.
- Route New/Open/Save/Save As/Export PDF activation through the toolbar adapter without changing their existing safety behavior.
- Render action Items as icon-first buttons with accessible text/tooltip labels and hover lightness feedback without a border.

## Out of scope

- Elements selection behavior beyond the placeholder/composition seam; WO-002 owns its role synchronization and mutation.
- Flexible-space measurement and overflow extraction; WO-003 owns toolbar layout.
- Toolbar customization, persistence, plugin registries, new commands, and title-bar/theme redesign.

## Requirements

- **REQ-WRITING-LOOP-001 / AC-WRITING-001.3:** “Given a valid selection, the system shall support insertion, deletion, split, join, role change, undo, and redo while preserving valid text boundaries and element order.”
- **REQ-WRITING-LOOP-003 / AC-WRITING-003.2:** “Given a save preparation or replacement failure, the system shall preserve the prior valid destination and retain an unsaved/dirty indication.”
- **REQ-WRITING-LOOP-004 / AC-WRITING-004.1:** “Given a project, PDF export shall use fixed print layout rather than transient viewport layout.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:1) — the desktop shell owns toolbar controls, status, dialogs, and platform services while delegating semantic operations to the core.
- [`technical-context.md`](../../../blueprints/technical-context.md:31) — preserve the desktop-shell-over-headless-core topology.

## Implementation plan

1. Add the Item data model and default toolbar factory in [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), keeping presentation metadata separate from project state.
2. Extract the existing direct action handlers into a toolbar dispatch path that preserves status/error and native path behavior.
3. Add app-level tests for default ordering, labels, action dispatch, and borderless hover styling decisions that are testable without a native window.

## Files and systems

- **Create:** No new production module required unless the implementer identifies a bounded app-local extraction; validation fixtures may be created.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), this work order, and [`execution-log.md`](../execution-log.md:1).
- **Avoid changing:** [`crates/scriptcraft-core/`](../../../../crates/scriptcraft-core/:1), native project schema, persistence semantics, layout constants, and exports.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`.
- **Manual/external checks:** bounded `cargo run -p scriptcraft`; native hover/click smoke if the environment exposes a window. Do not claim unavailable visual interaction.
- **Acceptance test:** Given the app starts with the default toolbar, when the toolbar is inspected and New/Open/Save/Save As/Export PDF are activated, then the Items appear in the approved order, expose labels, and preserve each existing command/status behavior.

## Durable-document impact

- **Requirements impact:** `update`
- **Blueprint impact:** `update`
- **Affected documents/IDs:** [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:38), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:35)
- **Rationale and evidence:** This work makes the user-facing desktop command surface and toolbar composition an explicit product behavior; the desktop blueprint must record the Item boundary and command routing.
- **Authorized documentation changes:** Add toolbar acceptance language and the desktop toolbar contract only; do not expand the core semantic or persistence requirements.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document impact classification is accurate against the diff, and authorized documentation updates are applied.
- [x] Execution log is appended, including dispatch and completion session/agent identifiers.
- [x] Work order is `in-review` with recorded evidence; completion comes only from a passed validation.

## Implementation evidence

- **Implementer session/agent ID:** `code-implementer-toolbar-wo001-2026-10-27-0233`
- **Changed files:**
  - [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1) — added the desktop-owned typed `ToolbarItem`/kind/action model, approved default order, toolbar dispatch adapter, icon-first borderless action presentation with tooltip labels, Elements composition placeholder, and focused model/dispatch tests; removed the former inline toolbar and separate role-controls presentation from WO-001's surface.
  - [`../../../requirements/features/screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:80) — added authorized toolbar acceptance criteria.
  - [`../../../blueprints/containers/scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:55) — added the authorized transient toolbar Item and presentation contract.
  - [`WO-001-toolbar-item-model-and-default-actions.md`](WO-001-toolbar-item-model-and-default-actions.md:1) — recorded implementation evidence and transitioned status.
  - [`../execution-log.md`](../execution-log.md:1) — appended dispatch and completion evidence.
- **Validation attempts/results:**
  - `cargo fmt --all -- --check` — PASS after applying formatter output.
  - `cargo check --workspace` — PASS.
  - `cargo test --workspace` — PASS; app 35 tests, core 44 unit tests, and all workspace integration/doc tests passed (0 failures).
  - `cargo build --workspace` — PASS.
  - `cargo clippy --workspace --all-targets -- -D warnings` — PASS.
  - `cargo run -p scriptcraft` — launched `target/debug/scriptcraft` successfully, but the native window remained running in this environment and was stopped after the bounded smoke attempt; native visual hover/click inspection was unavailable and is not claimed as passed.
- **Comparable diff evidence:** The implementation is confined to the desktop app plus the two authorized durable documents and Forge evidence artifacts; [`git diff -- apps/scriptcraft/src/main.rs .forge/requirements/features/screenplay-writing-loop.md .forge/blueprints/containers/scriptcraft-desktop.md`](../../../../apps/scriptcraft/src/main.rs:1) is the comparable file diff boundary.
- **Scope confirmation:** WO-002 and WO-003 were not implemented; no core crate, persistence schema, layout constants, or export implementation was changed.
- **Next state:** Await fresh-context WO-001 validation. Do not mark this work order `complete` from the implementer session.

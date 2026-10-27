# Work Order: WO-003 — Toolbar Spacing and Overflow

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop toolbar implementer (one fresh session)
- **Phase/order:** 3 of 3; depends on WO-001 and WO-002
- **Initiative:** [`../toolbar-v1.md`](../toolbar-v1.md:1)
- **Generated filename:** `WO-003-toolbar-spacing-and-overflow.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-003.md`](../validations/validation-WO-003.md)
- **Fixes:** none
- **Implementer session/agent ID:** `code-implementer-toolbar-wo003-2026-10-27-0241`
- **Session exception:** none

## Summary

Implement the responsive toolbar layout contract. Normal Space consumes one normal-button width, Flexible Space Items divide remaining width equally, and narrow widths retain the leftmost fitting Items while exposing non-spacing Items through a right-edge `>>` menu with executable nested dropdown behavior.

## In scope

- Define deterministic width measurement/minimum rules for normal Items, Space, Flexible Space, Elements, and the overflow affordance.
- Distribute remaining width equally among all Flexible Space Items until the toolbar fills the available window.
- Extract non-fitting non-spacing Items into the overflow menu while retaining left-to-right priority and the `>>` affordance at the far right.
- Support text-menu execution for action Items and submenu-like execution for dropdown Items such as Elements.
- Add headless/layout-level tests for wide, exact-fit, and narrow toolbar widths, including two Flexible Spaces and overflow extraction.

## Out of scope

- Toolbar customization, persistence, drag/reorder, arbitrary command registries, or animation.
- Adding new toolbar commands or changing command/session semantics established by WO-001/WO-002.
- General window responsive layout, title-bar changes, and editor/page geometry.

## Requirements

- **REQ-WRITING-LOOP-002 / AC-WRITING-002.1:** “Given the same project and fixed page profile, the system shall derive stable page structure and source mappings independent of window zoom.”
- **NFR-WRITING-001:** “Production paths should prefer typed errors and bounded inputs over panic-prone handling; a discovered crash or data-loss defect requires a regression test.”
- **NFR-WRITING-002:** “Core editing, layout, and persistence behavior shall be exercisable without a desktop window for deterministic tests.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:35) — toolbar geometry is transient desktop presentation and must not redefine semantic or print state.
- [`technical-context.md`](../../../blueprints/technical-context.md:31) — keep presentation state above the headless semantic core.

## Implementation plan

1. Add a pure or app-testable toolbar measurement/allocation helper in [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1) or a bounded desktop module.
2. Render flexible spaces and the overflow affordance based on available toolbar width without mutating the Item collection or project state.
3. Add overflow menu/submenu routing and focused width/allocation/interaction regressions.

## Files and systems

- **Create:** A bounded toolbar helper module is allowed if it improves testability; tests may be added.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:1), this work order, and [`execution-log.md`](../execution-log.md:1).
- **Avoid changing:** core semantic model, page layout, persistence, export behavior, and native project serialization.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`.
- **Manual/external checks:** bounded native smoke at wide and narrow window sizes. If no interactive window is available, record the limitation and rely on deterministic allocation tests.
- **Acceptance test:** Given the default toolbar at a wide width, when it is laid out, then both Flexible Spaces receive equal remaining width; given a narrow width, when items no longer fit, then only the leftmost fitting items remain visible, `>>` is at the far right, and every overflow action—including Elements role selection—remains executable by text/menu navigation.

## Durable-document impact

- **Requirements impact:** `update`
- **Blueprint impact:** `update`
- **Affected documents/IDs:** [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:58), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:35)
- **Rationale and evidence:** Responsive toolbar behavior is a new desktop presentation contract with observable acceptance behavior; it must be documented without coupling it to print layout or canonical project data.
- **Authorized documentation changes:** Add transient toolbar allocation/overflow contracts and acceptance evidence only.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document impact classification is accurate against the diff, and authorized documentation updates are applied.
- [x] Execution log is appended, including dispatch and completion session/agent identifiers.
- [x] Work order is `in-review` with recorded evidence; completion comes only from a passed validation.

## Implementation evidence

- **Implementer session/agent ID:** `code-implementer-toolbar-wo003-2026-10-27-0241`
- **Dispatch evidence:** WO-001 and WO-002 passed their recorded fresh validations before this fresh WO-003 implementation session; this session was dispatched for WO-003 only.
- **Changed files:**
  - [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1) — added the pure transient allocation contract, fixed normal/Space widths, equal Flexible Space distribution, leftmost-fit overflow extraction, far-right `>>` menu rendering, executable action entries, nested Elements role menu routing, and focused layout/interaction regressions.
  - [`../../../requirements/features/screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:114) — added the authorized responsive toolbar acceptance criterion.
  - [`../../../blueprints/containers/scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:68) — added the authorized transient allocation and overflow contract.
  - [`WO-003-toolbar-spacing-and-overflow.md`](WO-003-toolbar-spacing-and-overflow.md:1) — recorded implementation evidence and transitioned to `in-review`.
  - [`../execution-log.md`](../execution-log.md:1) — appended WO-003 dispatch and completion evidence.
- **Validation attempts/results:**
  - Initial combined implementation check — FAIL at `cargo clippy --workspace --all-targets -- -D warnings` because an unused helper was left after the menu extraction; all preceding fmt/check/test/build steps passed. The helper was removed.
  - Final implementation-session checks — PASS: `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace` (42 app tests, 44 core unit tests, all workspace integration/doc tests passed with zero failures); `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`.
  - `git diff --check` — PASS.
  - No fresh WO-003 validation was performed by this implementer session; the listed commands are implementation-session checks, not the required separate validation.
  - Native visual smoke was not attempted in this implementation session and is not claimed as passed.
- **Comparable diff evidence:** `git diff -- apps/scriptcraft/src/main.rs .forge/requirements/features/screenplay-writing-loop.md .forge/blueprints/containers/scriptcraft-desktop.md .forge/initiatives/2026-10-27-toolbar/work-orders/WO-003-toolbar-spacing-and-overflow.md .forge/initiatives/2026-10-27-toolbar/execution-log.md` is the bounded implementation/evidence diff boundary.
- **Scope confirmation:** No core semantic model, persistence schema, page layout, export behavior, project serialization, toolbar customization, or unrelated editor behavior was changed. Toolbar allocation remains transient.
- **Durable-document evidence:** Requirements and desktop blueprint updates are limited to the authorized responsive toolbar allocation, overflow, and nested Elements-menu contracts.
- **Next state:** Fresh WO-003 validation is required. Do not mark this work order `complete` from the implementer session.

# Work Order: WO-001 — Toolbar Fit and Vertical Padding

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop toolbar implementer (fresh session)
- **Phase/order:** 1 of 2; foundational
- **Initiative:** [`../toolbar-final-fit-v1.md`](../toolbar-final-fit-v1.md:1)
- **Validation:** [`../validations/validation-WO-001.md`](../validations/validation-WO-001.md)
- **Implementer session/agent ID:** `code-implementer-toolbar-final-fit-wo001-2026-10-27T0431Z`
- **Session exception:** none

## Summary

Correct the actual toolbar width calculation so Export PDF remains fully visible, and expose separate top/bottom padding controls. Set top padding to 2 points and reduce bottom padding from the current effective padding while preserving overflow and Flexible Space semantics.

## In scope

- Diagnose allocator/render mismatch and correct it with shared measured width assumptions.
- Add named `TOOLBAR_TOP_PADDING` and `TOOLBAR_BOTTOM_PADDING` constants; top is `2.0`, bottom is reduced from the current value.
- Test wide/maximized, exact-fit, narrow, Export PDF visibility, and vertical padding.
- Update authorized requirement/blueprint wording with final numeric values.

## Out of scope

- Toolbar color, new commands, Item grouping, core, persistence, exports, page layout, and global theme changes; WO-002 owns color.

## Requirements

- **AC-WRITING-005.5:** “Given a toolbar width, normal Items and Space shall consume one deterministic normal-item width, Flexible Space Items shall divide the remaining width equally, and a narrow toolbar shall retain the leftmost fitting Items while exposing non-spacing Items through a far-right `>>` text menu.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:68) — toolbar geometry is transient desktop presentation.

## Implementation plan

1. Update [`toolbar_content_width()`](../../../../apps/scriptcraft/src/main.rs:164), [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:177), and [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:519) to use actual shared horizontal/vertical geometry.
2. Add top/bottom constants near [`TOOLBAR_NORMAL_ITEM_WIDTH`](../../../../apps/scriptcraft/src/main.rs:21).
3. Add deterministic fit and padding tests.

## Files and systems

- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:21), requirements/desktop blueprint wording, this work order, and execution log.
- **Avoid changing:** color behavior, core, persistence, exports, page layout, and Item semantics.

## Validation

- **Automated:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- **Acceptance:** At wide/maximized-equivalent width, Export PDF is fully allocated with no overflow; top padding is 2 points; bottom padding is the recorded reduced value; narrow overflow remains valid.

## Durable-document impact

- **Requirements impact:** `update`
- **Blueprint impact:** `update`
- **Affected:** REQ-WRITING-LOOP-005 / AC-WRITING-005.5 and toolbar geometry in [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:68)
- **Rationale:** User-visible geometry and manual tuning controls change.

## Implementation evidence

- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:20), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:114), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:71), and this work order plus [`execution-log.md`](../execution-log.md:1).
- **Implementation:** Normal Item width is 72 points; allocation and rendering now share the calculated row-width budget including horizontal padding, inter-item gaps, and overflow reservation. `TOOLBAR_TOP_PADDING` is 2.0 and `TOOLBAR_BOTTOM_PADDING` is an explicit reduced 2.0 value. Flexible Space, narrow overflow, action/role routing, and color behavior were not changed.
- **Deterministic coverage:** Wide/maximized-equivalent fit, exact fit, narrow overflow, total bounds/shared row width, and top/bottom padding tests pass.
- **Cargo gate attempt 1:** `cargo fmt --all && cargo fmt --all -- --check && cargo check --workspace && cargo test --workspace && cargo build --workspace && cargo clippy --workspace --all-targets -- -D warnings && git diff --check` reached tests successfully, then clippy failed on the constant-expression padding assertion (`clippy::assertions_on_constants`). The assertion was corrected without changing production behavior.
- **Cargo gates final:** `cargo fmt --all -- --check` passed; `cargo check --workspace` passed; `cargo test --workspace` passed (50 app tests, 44 core unit tests, and all workspace integration/doc tests passed); `cargo build --workspace` passed; `cargo clippy --workspace --all-targets -- -D warnings` passed; `git diff --check` passed.
- **Native smoke limitation:** No fresh native/window validation was performed in this implementer session. Pixel-level Export PDF visibility and native title-bar comparison remain for the separate validation/native environment; no native smoke success is claimed.
- **Comparable evidence:** The implementation diff and the final Cargo gate command/output are available in the working tree; final command output artifact is `cmd-1793075573874.txt`.
- **Validation result:** [`validation-WO-001.md`](../validations/validation-WO-001.md) returned `PASS WITH WARNINGS` in a separate fresh-context session. The required automated gates passed independently; native visual inspection remained unavailable and no native visual success was claimed.
- **Next state:** Await fresh-context validation; do not mark this work order complete in the implementer session.

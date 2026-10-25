# Work Order: WO-001 — Vertical Edge Clamping

## Metadata

- **Status:** `complete`
- **Assignee:** Code-mode implementer
- **Phase/order:** `1 of 2`
- **Initiative:** [`../editor-geometry-corrections-v1.md`](../editor-geometry-corrections-v1.md)
- **Generated filename:** `WO-001-vertical-edge-clamping.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-001.md`](../validations/validation-WO-001.md)
- **Fixes:** `none`
- **Implementer session/agent ID:** `B89C490C-ABBB-4EC3-9CEE-FE4AC01752CC`
- **Session exception:** `none`

## Summary

Correct the edge behavior of visual-line vertical navigation. Up from the first visual line must move to the beginning of that line; Down from the last visual line must move to the end of that line. Ordinary adjacent-line movement, preferred horizontal goal, Shift selection, and UTF-8-safe source ranges must remain intact.

## In scope

- Update [`move_vertical()`](../../../apps/scriptcraft/src/main.rs:671) for first/last visual-line edge clamps.
- Add focused tests for first-line Up, last-line Down, wrapped first/last lines, Unicode boundaries, repeated movement, and Shift behavior.
- Preserve transient preferred-goal reset semantics at document edges as explicitly documented.

## Out of scope

- Page width, line spacing, role metrics, Enter/Tab policy, or persistence changes.

## Requirements

- **REQ-WRITING-LOOP-002:** “As an individual screenwriter, I want continuous writing and fixed-page preview to agree about semantic content and layout inputs, so that I can write comfortably and inspect a predictable page result.”
- **AC-WRITING-002.4:** “Given a document position and rendered geometry, the system shall support a safe caret/hit-test round trip at valid text boundaries.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) — the desktop host translates keyboard navigation into semantic caret positions.
- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) — positions and selections remain checked UTF-8 boundaries.

## Implementation plan

1. When no adjacent visual line exists, clamp Up to the current line's `source.start` and Down to its `source.end` rather than returning the unchanged caret.
2. Retain ordinary preferred horizontal goal behavior for successful adjacent-line movement; reset the goal according to the existing contract after a boundary clamp.
3. Add regression tests and run the standard workspace checks.

## Files and systems

- **Create:** focused app navigation tests if needed.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs).
- **Avoid changing:** [`crates/scriptcraft-core/src/layout.rs`](../../../crates/scriptcraft-core/src/layout.rs) and the native schema.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`.
- **Manual/external checks:** in the running editor, place the caret at a non-edge offset on the first line and press Up, then place it at a non-edge offset on the last line and press Down; verify exact line-start/end behavior.
- **Acceptance test:** Given a caret on the first visual line, when Up is pressed, then the caret is at that line's `source.start`; given a caret on the last visual line, when Down is pressed, then the caret is at that line's `source.end`; adjacent-line movement remains unchanged.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `verify unchanged`
- **Affected documents/IDs:** [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md)
- **Rationale and evidence:** This corrects transient caret navigation within the already-described semantic/layout boundary; no canonical or durable geometry changes.
- **Authorized documentation changes:** Append implementation and validation evidence to this initiative and execution log only unless the final navigation contract needs clarification.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Work order was `in-review` before fresh validation and became `complete` after the passed validation in [`validation-WO-001.md`](../validations/validation-WO-001.md).

## Implementation evidence

- **Implementation session:** `B89C490C-ABBB-4EC3-9CEE-FE4AC01752CC` (fresh Code-mode implementer session, 2026-10-25).
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs) only for implementation and focused regressions; this work order and the initiative execution log for evidence.
- **Behavior:** `move_vertical()` now clamps a first-line Up to the current visual line's `source.start` and a last-line Down to its `source.end`, preserving adjacent-line preferred horizontal goals and resetting the goal on edge clamps. Shift keeps the existing anchor/focus behavior.
- **Regression coverage:** single-line first/last edges; wrapped first/last visual lines; UTF-8-safe Unicode boundaries; repeated edge movement; and Shift anchor preservation.
- **Durable documents:** Requirements and blueprints remain `verify unchanged`; the change is transient caret navigation within the existing semantic/source-mapping contract.

### Validation attempts

- `cargo fmt --all -- --check` — initially failed because the newly added assertions required rustfmt formatting; remediated with `cargo fmt --all`, then passed.
- `cargo check --workspace` — passed.
- `cargo test --workspace` — passed: app 22, core 28, WO-003 integration 5, WO-008 integration 3, doc-tests 0.
- `cargo clippy --workspace --all-targets -- -D warnings` — initially failed on newly added test `unwrap()` calls; remediated with scoped test lint allowances, then passed.
- `cargo fmt --all -- --check` — passed after remediation.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed after remediation.

Implementation evidence is complete; fresh validation returned `PASS WITH WARNINGS`, and the orchestrator marked this work order `complete`. Native interaction remains an explicitly recorded warning.

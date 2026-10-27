# Work Order: WO-002 — Uniform Persistent Icon Hover

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop toolbar implementer (fresh session)
- **Phase/order:** 2 of 2; depends on WO-001
- **Initiative:** [`../toolbar-exact-width-hover-v1.md`](../toolbar-exact-width-hover-v1.md:1)
- **Validation:** [`../validations/validation-WO-002.md`](../validations/validation-WO-002.md)
- **Implementer session/agent ID:** `code-implementer-toolbar-exact-width-hover-2026-10-27-0743Z`
- **Session exception:** none

## Summary

Make the hover region uniformly surround the icon with equal padding on all sides, keep it active as the pointer moves across the painted icon, prevent decorative labels from competing for hover, and remove the redundant label tooltip.

## In scope

- Center the icon hover rectangle around the icon control and expand it uniformly by the named padding value.
- Keep one explicit interactive response as the hover/click owner; render icon and label as non-competing decorative content where appropriate.
- Remove [`on_hover_text()`](../../../../apps/scriptcraft/src/main.rs:468) from the toolbar action path.
- Add deterministic rectangle/hover ownership tests for outside, edge, and icon-center pointer positions.

## Out of scope

- Exact label measurement, viewport flags, right-edge allocation, commands, core, persistence, exports, page layout, and global theme.

## Requirements

- **AC-WRITING-005.3:** “Action Items shall use icon-first, borderless button presentation with a lightness-only hover affordance and retain text labels for tooltip and accessibility use.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58) — transient Item interaction ownership.

## Validation

- **Automated:** format, check, tests, build, Clippy with `-D warnings`, and diff check.
- **Acceptance:** Hover fill has equal top/right/bottom/left padding around the icon, remains active over the icon glyph, does not activate over the label, click dispatch remains intact, and no redundant tooltip appears.

## Durable-document impact

- **Requirements impact:** `update`
- **Blueprint impact:** `update`
- **Rationale:** Hover ownership and tooltip behavior are user-facing Item interaction contracts.

## Implementation evidence

- **Completion date:** 2026-10-27
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:490), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:104), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:66), this work order, and [`../execution-log.md`](../execution-log.md:113).
- **Implementation:** Centered the icon control rectangle, expanded its hover rectangle uniformly from the control center by the named padding, retained one explicit hover/click response and existing action dispatch, rendered icon/label as painter-only decorative content, removed [`on_hover_text()`](../../../../apps/scriptcraft/src/main.rs:497), and added deterministic outside/edge/icon-center ownership tests.
- **Scope confirmation:** Exact runtime width measurement, viewport configuration, actual available-width/right-edge logic, commands, core, persistence, exports, page layout, global theme, and unrelated code were not changed by WO-002. Existing WO-001/WO-001a changes remain in the working tree as prior initiative work.
- **Required gates:** `cargo fmt --all -- --check` PASS (exit `0`); `cargo check --workspace` PASS (exit `0`); `cargo test --workspace` PASS (exit `0`, including the new deterministic hover ownership test); `cargo build --workspace` PASS (exit `0`); `cargo clippy --workspace --all-targets -- -D warnings` PASS (exit `0`); `git diff --check` PASS (exit `0`).
- **Comparable diff evidence:** `git status --short` and `git diff --stat` recorded the existing initiative working-tree changes in [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), the authorized requirements/blueprint updates, and initiative artifacts; `git diff --check` passed. No unrelated files were modified by this work order.
- **Native smoke limitation:** No fresh native visual/window smoke was performed or claimed; visible-window measurement remains unavailable in this session and is a validator/manual follow-up limitation.
- **Validation boundary:** This implementer ran only the required implementation gates above. No fresh independent validation was performed; the work order is `in-review` and awaits a separate validator.

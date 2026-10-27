# Work Order: WO-001 — Conditional Caret Reveal

## Metadata

- **Status:** `in-review`
- **Assignee:** Code-mode implementer
- **Phase/order:** `1 of 2`
- **Initiative:** [`../cursor-reveal-and-paste-safety-v1.md`](../cursor-reveal-and-paste-safety-v1.md)
- **Generated filename:** `WO-001-conditional-caret-reveal.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-001.md`](../validations/validation-WO-001.md)
- **Fixes:** `none`
- **Implementer session/agent ID:** `2026-10-25-code-session`
- **Session exception:** `none`

## Summary

Stop automatically centering the caret after ordinary navigation and selection. The editor should preserve the user's scroll position, reveal the caret only when a caret-affecting command places it outside the visible viewport, and use minimal scroll rather than center alignment.

## In scope

- Separate reveal intent from generic successful mutation in [`mutate()`](../../../apps/scriptcraft/src/main.rs:182).
- Treat arrow/Home/End, text entry, Enter/Tab, delete/backspace, undo/redo, and large paste as caret-affecting actions that may request conditional reveal.
- Treat passive scrolling and pointer selection as non-reveal actions; a pointer/scroll action must not be overridden by a stale pending reveal.
- Compare caret geometry with the actual scroll clip/viewport and reveal only when it is outside or partially outside, without forcing center alignment.
- Add focused tests for visible caret/no scroll, above/below viewport reveal, boundary navigation, pointer drag selection, and paste reveal.

## Out of scope

- Paste parsing/model changes, new scrollbars, persisted viewport state, or changing page geometry.

## Requirements

- **REQ-WRITING-LOOP-002:** “As an individual screenwriter, I want continuous writing and fixed-page preview to agree about semantic content and layout inputs, so that I can write comfortably and inspect a predictable page result.”
- **AC-WRITING-002.4:** “Given a document position and rendered geometry, the system shall support a safe caret/hit-test round trip at valid text boundaries.”
- **NFR-WRITING-002:** “Core editing, layout, and persistence behavior shall be exercisable without a desktop window for deterministic tests.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) — the host owns scrolling, focus, and caret presentation.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) — UI consumes derived geometry without changing semantic layout.

## Implementation plan

1. Introduce explicit reveal intent/reason classification at the event/command boundary rather than setting one boolean for every mutation.
2. Calculate the post-action caret rectangle after the page layout is rendered and compare it with the visible clip rectangle.
3. Request minimal scroll only for an out-of-view caret; clear pending reveal on passive scroll/pointer selection and avoid `Align::Center` for ordinary reveal.
4. Add deterministic UI geometry tests and record native manual limitations honestly.

## Files and systems

- **Create:** focused app tests for conditional reveal and scroll intent.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs).
- **Avoid changing:** [`crates/scriptcraft-core/src/layout.rs`](../../../crates/scriptcraft-core/src/layout.rs), paste semantics, persistence, and page-profile geometry.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`.
- **Manual/external checks:** scroll away from the caret, click/drag to select, press arrows while the caret remains visible, type onto a new line, and paste enough text to move the caret beyond the viewport. Verify the view does not center on ordinary movement and only adjusts when required.
- **Acceptance test:** Given a visible caret and a user-scrolled viewport, when the user scrolls or selects text, then the caret and view remain where they are; when a caret-affecting command places the caret outside the clip rect, then the smallest scroll needed reveals it without centering it.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `verify unchanged`
- **Affected documents/IDs:** [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md)
- **Rationale and evidence:** This is transient viewport policy; no canonical document or print geometry changes.
- **Authorized documentation changes:** Append implementation/validation evidence and clarify the conditional reveal behavior if needed.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [ ] Work order is `in-review` before fresh validation and becomes `complete` only after a passed validation.

## Implementation evidence

- Changed [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs): added explicit `RevealIntent` classification; caret-affecting edits request conditional reveal, while role changes, pointer selection, and passive scroll clear reveal intent. Caret geometry is compared with the actual egui clip rectangle and uses `scroll_to_rect(..., None)` for nearest/minimal scrolling rather than centering.
- Added focused tests for visible, above, below, and partially visible caret geometry, plus pointer selection clearing stale intent.
- Validation evidence: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo build --workspace` all passed.
- Native manual smoke remains a validation follow-up because this session did not operate a visible desktop window.

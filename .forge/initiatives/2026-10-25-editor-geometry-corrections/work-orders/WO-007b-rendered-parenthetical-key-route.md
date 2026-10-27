# Work Order: WO-007b — Rendered Parenthetical Key Route

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer after WO-007a validation failure
- **Phase/order:** `fix for 7 of 7`
- **Initiative:** [`../editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md)
- **Original work order:** [`WO-007-parenthetical-transition-and-boundary-policy.md`](WO-007-parenthetical-transition-and-boundary-policy.md)
- **Predecessor fix:** [`WO-007a-parenthetical-advance-boundary-fix.md`](WO-007a-parenthetical-advance-boundary-fix.md)
- **Failing validation:** [`../validations/validation-WO-007a-v2.md`](../validations/validation-WO-007a-v2.md)
- **Validation:** [`../validations/validation-WO-007bc-v2.md`](../validations/validation-WO-007bc-v2.md)
- **Fixes:** [`WO-007c-delimiter-only-parenthetical-delete.md`](WO-007c-delimiter-only-parenthetical-delete.md)

## Summary

Close the WO-007a validation hard stop by proving that the rendered editor's caret mapping and actual Tab/Enter event path reach the approved Parenthetical inner-text-end boundary for both commands.

## In scope

- Drive the production rendered editor surface with a text-bearing closed Parenthetical and position the pointer at the visual location immediately after the final inner character and before `)`.
- Assert the resulting source selection offset is the inner-text-end byte boundary rather than manually assigning it.
- Dispatch both Tab and Enter through the production focused event path and assert each creates the Dialogue below with the preserved/repaired Parenthetical above.
- Preserve all WO-007/WO-007a settings, delimiter, strict-boundary, provenance, undo/redo, persistence, and rollback contracts.

## Out of scope

- Core boundary semantics changes, new delimiter policies, settings changes, layout/pointer policy changes, or native-only claims.

## Requirements and blueprints

- **Requirements (verbatim):** `AC-WRITING-002.4`: “Given a document position and rendered geometry, the system shall support a safe caret/hit-test round trip at valid text boundaries.” `NFR-WRITING-002`: “Core editing, layout, and persistence behavior shall be exercisable without a desktop window for deterministic tests.”
- **Blueprint:** [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md).

## Acceptance criteria

- **WO007B-AC-001:** A production rendered pointer hit at the final inner character maps to the expected source byte offset before `)`.
- **WO007B-AC-002:** With the rendered surface focused, Tab dispatches through [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:633) and [`EditingSession::tab()`](../../../crates/scriptcraft-core/src/editing.rs:253), producing the approved Dialogue result.
- **WO007B-AC-003:** Enter dispatches through the same production path and produces the same approved result.
- **WO007B-AC-004:** The regression does not directly assign the source offset or bypass the rendered surface; existing WO-007 core tests remain unchanged and pass.

## Validation

- Run focused rendered test and all Cargo gates.
- Native typing smoke is optional/unavailable unless actually performed.
- Fresh validation must compare the production pointer mapping and both event routes, not only core helper behavior.

## Durable-document impact

- **Requirements:** `verify unchanged`.
- **Blueprint:** `verify unchanged` unless the rendered evidence requires a clarification.
- **Rationale/evidence:** [`validation-WO-007a-v2.md`](../validations/validation-WO-007a-v2.md) found the existing app regression directly assigned a source offset and exercised only Enter.
- **Authorized documentation changes:** This fix WO, its validation, and execution log; do not change completed predecessor history or WO-007 semantics.

## Implementation evidence — 2026-10-27

- **Root cause addressed:** The prior app regression supplied the source offset directly and called the private key helper for Enter only. It did not exercise the rendered surface's pointer mapping or focused event dispatcher.
- **Implementation:** Replaced that helper-level regression in [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs) with a rendered egui surface test. The test renders a real Parenthetical, computes the visual point after the final inner character using the rendered line geometry, clicks that point through the production surface, and asserts the resulting hit-test selection is the UTF-8-safe byte offset immediately before `)`. It then sends Tab and Enter as focused [`egui::Event::Key`](../../../apps/scriptcraft/src/main.rs) input through [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:739), without assigning the source offset or calling [`key_event()`](../../../apps/scriptcraft/src/main.rs:708) directly.
- **Result:** Both production event routes preserve `(words)` and create/select the empty Dialogue below at offset `0`; the pointer assertion proves the rendered route supplied the semantic boundary.
- **Changed product/test files:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs). No core, layout, persistence, settings, predecessor, or WO-007 semantic behavior changed.
- **Focused evidence:** `cargo test -p scriptcraft rendered_parenthetical_inner_end_routes_pointer_and_both_keys -- --nocapture` passed; the test reported `1 passed`.
- **Required gates:** `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace` (`31` app tests, `44` core unit tests, and all integration suites), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace`, and `git diff --check` passed on 2026-10-27.
- **Native smoke:** Not performed and not inferred from rendered/headless evidence.
- **Handoff:** Fresh independent validation is required; WO-007a and WO-007 remain `in-review`.

## WO-007c fix-chain evidence — 2026-10-27

- The delimiter-only behavior identified during the rendered-route review is formalized in [`WO-007c-delimiter-only-parenthetical-delete.md`](WO-007c-delimiter-only-parenthetical-delete.md). Its bounded change corrects only the first-element fallback in the approved exact `(`, `()`, `)` Delete matrix and adds deterministic Delete/Tab history coverage.
- The rendered WO-007b pointer/key route is unchanged: non-empty `(words)` still reaches the inner-text-end boundary through the production rendered surface, and focused Tab/Enter still create the Dialogue below. No rendered-route semantics or WO-007a boundary predicate was changed.
- WO-007b remains `in-review`; fresh independent validation must assess the complete WO-007 → WO-007a → WO-007b → WO-007c chain.

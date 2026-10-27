# Work Order: WO-007c — Delimiter-Only Parenthetical Delete

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer after WO-007b validation review
- **Phase/order:** `fix for 7 of 7`
- **Initiative:** [`../editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md)
- **Original work order:** [`WO-007-parenthetical-transition-and-boundary-policy.md`](WO-007-parenthetical-transition-and-boundary-policy.md)
- **Predecessor fix:** [`WO-007b-rendered-parenthetical-key-route.md`](WO-007b-rendered-parenthetical-key-route.md)
- **Validation:** [`../validations/validation-WO-007bc-v2.md`](../validations/validation-WO-007bc-v2.md)

## Summary

Formalize and validate delimiter-only Parenthetical Delete/Tab behavior that was discovered while checking the rendered route: an empty delimiter-only Parenthetical must be removable or convertible from any valid caret position without violating the one-element model invariant.

## In scope

- For exact `(`, `()`, or `)` Parenthetical text, collapsed Delete removes the entire element and moves to the preceding element end, with first/only fallback to empty Dialogue at element 0/offset 0.
- For exact `(`, `()`, or `)` Parenthetical text, collapsed Tab converts the element in place to empty Dialogue at offset 0.
- Preserve the approved non-empty Parenthetical inner-end Tab/Enter route, rendered pointer/key proof, settings, strict boundaries, provenance, undo/redo, persistence, rollback, and completed predecessor contracts.
- Add formal fix-chain evidence for the existing product delta and focused deterministic tests.

## Out of scope

- New delimiter shapes, settings changes, layout/pointer changes, native-only behavior, or completed WO-006 changes.

## Requirements and validation

- **Requirements:** `AC-WRITING-001.3`, `AC-WRITING-001.5`, `AC-WRITING-001.4a` as copied in [`WO-007`](WO-007-parenthetical-transition-and-boundary-policy.md).
- **Validation:** Focused delimiter-only tests, existing WO-007/007a/007b suites, full Cargo gates, and explicit transactional/history/persistence assertions.

## Durable-document impact

- **Requirements:** `verify unchanged` — this fixes the already approved WO-007 Delete/Tab matrix.
- **Blueprint:** `verify unchanged` unless the delimiter-only wording requires clarification.
- **Rationale/evidence:** The behavior was identified while validating the rendered WO-007b route and directly matches the user-approved empty-parenthetical Delete/Tab matrix.
- **Authorized documentation changes:** This fix WO, original WO-007 fix-chain evidence, execution log, and validation artifact; do not alter completed WO-005/005a history.

## Implementation evidence — 2026-10-27

- **Root cause/delta:** The existing working-tree delta already recognized exact delimiter-only Parenthetical text in the transactional Delete/Tab paths, but its Delete fallback treated only an only-element document specially. A delimiter-only Parenthetical at index `0` with a following element could be removed and leave the selection on the following element instead of preserving the required empty Dialogue fallback at index `0`/offset `0`.
- **Implementation:** [`EditingSession::delete()`](../../../crates/scriptcraft-core/src/editing.rs:163) now applies the empty Dialogue fallback when the delimiter-only element is first **or** the only element; predecessor removal remains unchanged for later elements. The exact-shape predicate remains bounded to `(`, `()`, and `)` and [`EditingSession::tab()`](../../../crates/scriptcraft-core/src/editing.rs:330) remains gated by `auto_parenthetical`, preserving WO-007 setting-off precedence.
- **Focused tests:** [`wo007_parenthetical_policy.rs`](../../../crates/scriptcraft-core/tests/wo007_parenthetical_policy.rs:161) covers every valid caret offset for all three exact shapes across first-with-successor, only-element, and predecessor-removal cases, plus one-step Delete undo/redo. The Tab matrix at [`delimiter_only_parentheticals_tab_to_empty_dialogue_from_every_caret_position()`](../../../crates/scriptcraft-core/tests/wo007_parenthetical_policy.rs:250) asserts one-step undo/redo for every shape/offset. Existing non-collapsed delegation, non-empty WO-007/WO-007a advancement, strict-boundary, provenance, persistence, and rollback tests remain unchanged and pass.
- **Focused result:** `cargo test -p scriptcraft-core --test wo007_parenthetical_policy -- --nocapture` passed with `10` tests.
- **Scope:** Changed product/test files are [`editing.rs`](../../../crates/scriptcraft-core/src/editing.rs:174) and [`wo007_parenthetical_policy.rs`](../../../crates/scriptcraft-core/tests/wo007_parenthetical_policy.rs:161). No rendered-route, layout, persistence-schema, settings, or predecessor product semantics were broadened.
- **Validation handoff:** [`validation-WO-007c-v2.md`](../validations/validation-WO-007c-v2.md) records the implementation evidence and pending independent validation. Native typing smoke was not performed and is not inferred.

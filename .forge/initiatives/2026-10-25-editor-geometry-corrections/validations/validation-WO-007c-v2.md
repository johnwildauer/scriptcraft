# Validation Handoff: WO-007c — Delimiter-Only Parenthetical Delete

## Metadata

- **Status:** `pending-independent-validation`
- **Work order:** [`WO-007c-delimiter-only-parenthetical-delete.md`](../work-orders/WO-007c-delimiter-only-parenthetical-delete.md)
- **Fix chain:** [`WO-007`](../work-orders/WO-007-parenthetical-transition-and-boundary-policy.md) → [`WO-007a`](../work-orders/WO-007a-parenthetical-advance-boundary-fix.md) → [`WO-007b`](../work-orders/WO-007b-rendered-parenthetical-key-route.md) → [`WO-007c`](../work-orders/WO-007c-delimiter-only-parenthetical-delete.md)
- **Branch:** `dev/2026-10-26-editor-geometry-corrections-v2`
- **Date:** 2026-10-27

## Implementation handoff evidence

- [`EditingSession::delete()`](../../../../crates/scriptcraft-core/src/editing.rs:163) recognizes only exact delimiter-only Parenthetical text `(`, `()`, or `)`. A later element is removed and selection moves to the predecessor text end. A first or only element becomes empty Dialogue at index `0`, offset `0`, preserving the one-element invariant.
- [`EditingSession::tab()`](../../../../crates/scriptcraft-core/src/editing.rs:330) converts exact delimiter-only Parentheticals in place to empty Dialogue at offset `0` when `auto_parenthetical` is enabled; the existing setting-off no-cleanup behavior remains covered by the WO-007 suite.
- [`wo007_parenthetical_policy.rs`](../../../../crates/scriptcraft-core/tests/wo007_parenthetical_policy.rs:161) covers every valid offset for all three exact shapes, first-with-successor, only-element, and predecessor fallback, with one-step Delete undo/redo. The Tab matrix covers every shape/offset with one-step undo/redo. Existing non-empty/WO-007/WO-007a behavior remains in the same focused suite.

## Results

- Focused: `cargo test -p scriptcraft-core --test wo007_parenthetical_policy -- --nocapture` — passed, 10 tests.
- Required gates: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace`, and `git diff --check` — passed on 2026-10-27.
- Native typing smoke: not performed and not inferred from deterministic headless/rendered evidence.

## Scope/status guard

The only product correction is the first-element fallback in the delimiter-only Delete branch. No rendered-route semantics, layout, persistence schema, settings, WO-007a boundary predicate, or WO-007b app test behavior changed. This is an implementation handoff for fresh independent validation; WO-007c must remain `in-review`, not complete.

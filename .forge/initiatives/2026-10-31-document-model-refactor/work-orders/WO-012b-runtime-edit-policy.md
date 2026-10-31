# Work Order: WO-012b — Separate editing policy from document state

- **Status:** `draft / not started`; depends on WO-012a.

## Outcome and scope
Move cue casing, automatic parenthetical and strict transition-boundary decisions out of persistent project/profile state into explicit runtime/session policy. Preserve existing default editing behavior without applying policy during import.

## Components and implementation
- Update [editing](../../../../crates/scriptcraft-core/src/editing.rs), [model](../../../../crates/scriptcraft-core/src/lib.rs), desktop session construction and editing tests. Default all three policies to their current true behavior; allow nondefault values only as ephemeral session/application inputs, not new durable settings or FDX fields.
- Ensure selection, generated-parenthetical tracking, undo/redo, UTF-8 case expansion, role transitions, safe-stops and pending edits behave consistently. Policy changes alone must not dirty a document; semantic edits still do. Decide whether undo restores runtime policy separately and document the choice without recording it as FDX state.
- Keep transitional JSON adapter capable of safe lossless old-format behavior, or explicitly reject incompatible old JSON operations without changing session/destination. Do not activate FDX Open yet.

## Acceptance and checks
- Tests cover default and false policies, imported mixed case, parenthetical and transition edge cases, undo/redo, dirty state, clean reopen/reset, and safe transitional rejection. No JSON value is silently discarded in a successful transitional save.
- Run `cargo test -p scriptcraft-core --test wo004_role_casing`, `cargo test -p scriptcraft-core --test wo007_parenthetical_policy`, `cargo test -p scriptcraft-core --test wo008_dialogue_enter`, and `cargo test --workspace`; record outcomes. Run applicable format/lint/build checks per project policy.

## Risks and dependency
Depends on WO-012a and existing WO-004/005. One attributable commit, implementer checks only; no independent validator. Risk: casing alters imported text or policy is confused with FDX ElementSettings. Hand explicit policy API to WO-012c.

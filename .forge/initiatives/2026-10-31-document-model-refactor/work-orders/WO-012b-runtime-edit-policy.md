# Work Order: WO-012b — Separate editing policy from document state

- **Status:** `implemented`; implementation checks recorded below. No independent validation was dispatched under the initiative-specific user direction.

## Outcome and scope
Move cue casing, automatic parenthetical and strict transition-boundary decisions out of persistent project/profile state into explicit runtime/session policy. Preserve existing default editing behavior without applying policy during import.

## Components and implementation
- Update [editing](../../../../crates/scriptcraft-core/src/editing.rs), [model](../../../../crates/scriptcraft-core/src/lib.rs), desktop session construction and editing tests. Default all three policies to their current true behavior; allow nondefault values only as ephemeral session/application inputs, not new durable settings or FDX fields.
- Ensure selection, generated-parenthetical tracking, undo/redo, UTF-8 case expansion, role transitions, safe-stops and pending edits behave consistently. Policy changes alone must not dirty a document; semantic edits still do. Decide whether undo restores runtime policy separately and document the choice without recording it as FDX state.
- Keep transitional JSON adapter capable of safe lossless old-format behavior, or explicitly reject incompatible old JSON operations without changing session/destination. Do not activate FDX Open yet.

## Implementation decisions

- Added session-scoped `EditPolicy` with the established all-true defaults. Policy is not stored on `Project`/`PageProfile`, in snapshots, or in FDX. Changing it does not alter project data, revision, history, or dirty state. Undo/redo do **not** restore policy; it remains a live session/application input across those operations and document replacement. A fresh default session starts with all three defaults.
- The explicit API handed to WO-012c is `EditPolicy`, `EditingSession::with_policy`, `EditingSession::policy`, and `EditingSession::set_policy`.
- Routed cue normalization, parenthetical generation, and strict transition-boundary behavior through the runtime policy; imports leave mixed-case text untouched. Desktop startup creates a default policy and New/Open retain the current app-session policy.
- Transitional JSON continues to emit legacy policy keys only with their historical `true` defaults and accepts missing/default-true fields. A legacy explicit `false` is rejected as `UnsupportedJsonState`, rather than being discarded; JSON decoding fails before replacing the active session. Nondefault runtime overrides remain ephemeral and are not written as settings.
- FDX Open activation and native save behavior were not enabled or otherwise changed by this work order.

## Implementation checks — 2026-10-31

- `cargo test -p scriptcraft-core --test wo004_role_casing` — passed (5 tests).
- `cargo test -p scriptcraft-core --test wo007_parenthetical_policy` — passed (11 tests).
- `cargo test -p scriptcraft-core --test wo008_dialogue_enter` — passed (5 tests).
- `cargo test -p scriptcraft legacy_policy_json_rejection_preserves_live_session_path_and_source_file` — passed (1 targeted desktop test).
- `cargo test --workspace` — passed (303 tests, 0 failed; includes 154 desktop unit tests, 105 core unit tests, 44 integration tests, and 0 doc tests).
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo check --workspace`, `cargo build --workspace`, and `git diff --check` — all passed (exit 0).
- During test development, the first full-workspace run failed the new desktop rejection test because its dirty fixture correctly hit the existing unsaved-document Open guard before decoding. The fixture was changed to a clean active session; the focused test and final full-workspace run then passed.
- These are implementer checks only, not independent validation. No manual Final Draft test or external interoperability claim was made.

## Acceptance and checks
- Tests cover default and false policies, imported mixed case, parenthetical and transition edge cases, undo/redo, dirty state, clean reopen/reset, and safe transitional rejection. No JSON value is silently discarded in a successful transitional save.
- Run `cargo test -p scriptcraft-core --test wo004_role_casing`, `cargo test -p scriptcraft-core --test wo007_parenthetical_policy`, `cargo test -p scriptcraft-core --test wo008_dialogue_enter`, and `cargo test --workspace`; record outcomes. Run applicable format/lint/build checks per project policy.

## Risks and dependency
Depends on WO-012a and existing WO-004/005. One attributable commit, implementer checks only; no independent validator. Risk: casing alters imported text or policy is confused with FDX ElementSettings. Hand explicit policy API to WO-012c.

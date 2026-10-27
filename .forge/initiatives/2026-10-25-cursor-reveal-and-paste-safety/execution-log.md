# Execution Log: Cursor Reveal and Paste Safety

This log records execution-phase dispatches, completions, validations, failures,
and closeout follow-ups. Planning discussion remains in the initiative plan.

## Entries

- 2026-10-25 — Initiative created in `plan` on branch `dev/2026-10-25-cursor-reveal-and-paste-safety`; no implementation dispatched.
- 2026-10-25 — Code-mode implementation session `2026-10-25-code-session` completed WO-001 and WO-002 in scope. Changed [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs), [`crates/scriptcraft-core/src/editing.rs`](../../../crates/scriptcraft-core/src/editing.rs), and added [`crates/scriptcraft-core/tests/wo002_paste_safety.rs`](../../../crates/scriptcraft-core/tests/wo002_paste_safety.rs). All required automated commands passed; both work orders moved to `in-review` pending fresh validation. Native visible-window smoke is recorded as unavailable in this session.
- 2026-10-25 — Follow-up reproduction isolated the reported multi-paragraph `Model(Invalid)` behavior to [`split_at()`](../../../crates/scriptcraft-core/src/editing.rs:421) chaining the source ID into every new paragraph. After enough Enter operations, `StableId` rejected the overlong ID as `Model(InvalidId)`; paste avoided the path because it retained one element. Short collision-checked split IDs, live session validation, exact error/status diagnostics, and a 64-paragraph regression were added. Required automated checks passed again.

# Work Order: WO-002 — Paste Model Validity

## Metadata

- **Status:** `in-review`
- **Assignee:** Code-mode implementer
- **Phase/order:** `2 of 2`
- **Initiative:** [`../cursor-reveal-and-paste-safety-v1.md`](../cursor-reveal-and-paste-safety-v1.md)
- **Generated filename:** `WO-002-paste-model-validity.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-002.md`](../validations/validation-WO-002.md)
- **Fixes:** `none`
- **Implementer session/agent ID:** `2026-10-25-code-session`
- **Session exception:** `none`

## Summary

Reproduce and fix the reported `Model(Invalid)` state after pasting large, multi-line, whitespace-heavy content. The work begins with a fixture matrix and exact error classification; it must not assume that spaces/newlines are invalid or silently change paste into multiple screenplay elements without an explicit semantic decision.

## In scope

- Test paste dimensions independently: empty/space-only text, repeated spaces, tabs, LF/CRLF/CR, multiple blank lines, long lines, Unicode, large byte size, many pages, and paste at nonzero selections/offsets.
- Trace errors through [`EditingSession::insert()`](../../../crates/scriptcraft-core/src/editing.rs:132), project validation, layout construction, native serialization, and app error presentation.
- Verify failed paste/layout/serialization leaves the previous valid session/model and keeps subsequent edits possible.
- Decide and document whether embedded line breaks remain in one semantic element or become multiple elements; if splitting is required, define role inheritance, transitions, one-undo behavior, and bounded allocation.
- Add a regression fixture that reproduces the original failure or proves the bounded error path and final-page/caret behavior.

## Out of scope

- Rich clipboard formats, HTML/RTF interpretation, arbitrary formatting import, unbounded limits, or general paste/clipboard UX redesign.

## Requirements

- **REQ-WRITING-LOOP-001:** “As an individual screenwriter, I want to create and edit an ordered screenplay with explicit element roles, so that my draft retains screenplay meaning while I write.”
- **AC-WRITING-001.3:** “Given a valid selection, the system shall support insertion, deletion, split, join, role change, undo, and redo while preserving valid text boundaries and element order.”
- **AC-WRITING-001.5:** “Given malformed or out-of-bound input, the system shall return a bounded error without corrupting the project or session state.”
- **AC-WRITING-002.3:** “Given long text or an oversized element, the system shall wrap or split it safely without an infinite loop or unbounded blank-page behavior.”
- **NFR-WRITING-001:** “Production paths should prefer typed errors and bounded inputs over panic-prone handling; a discovered crash or data-loss defect requires a regression test.”

## Blueprints

- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) — transactional editing, bounded model validation, and canonical UTF-8 text.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) — typed error presentation and unchanged state after failed operations.

## Implementation plan

1. Create the paste fixture matrix and record the first failing operation/error variant for each dimension.
2. Test direct session paste independently from UI event dispatch to distinguish model failure from presentation/stale-error failure.
3. Fix the smallest root cause or add a bounded paste transaction/normalization policy; preserve one-paste undo semantics.
4. Add hostile-size, newline, whitespace, Unicode, save/reopen, layout, and recovery regressions.

## Files and systems

- **Create:** focused paste/model integration tests under [`crates/scriptcraft-core/tests/`](../../../crates/scriptcraft-core/tests/) and app event tests if needed.
- **Update:** only implicated files among [`crates/scriptcraft-core/src/editing.rs`](../../../crates/scriptcraft-core/src/editing.rs), [`crates/scriptcraft-core/src/lib.rs`](../../../crates/scriptcraft-core/src/lib.rs), [`crates/scriptcraft-core/src/layout.rs`](../../../crates/scriptcraft-core/src/layout.rs), [`crates/scriptcraft-core/src/persistence.rs`](../../../crates/scriptcraft-core/src/persistence.rs), and [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs).
- **Avoid changing:** cursor reveal policy, page geometry, role profile, or rich clipboard formats.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo build --workspace`.
- **Manual/external checks:** paste representative lorem ipsum with blank lines, CRLF, spaces, Unicode, and enough pages to scroll; capture exact status/error and confirm the window remains usable.
- **Acceptance test:** Given each bounded paste fixture, when it is inserted, laid out, and saved/reopened, then valid content remains valid and editable, failures are typed/bounded with the previous model preserved, no stale `Model(Invalid)` state masks the true cause, and the chosen newline semantic policy is deterministic and undoable.

## Durable-document impact

- **Requirements impact:** `update` if paste semantic policy changes; otherwise `verify unchanged`
- **Blueprint impact:** `update`
- **Affected documents/IDs:** [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md), and any paste policy reference created by implementation.
- **Rationale and evidence:** The current requirements cover bounded edits but do not specify paste newline semantics or error classification. The final impact follows the reproduced root cause and selected policy.
- **Authorized documentation changes:** Record the fixture matrix, root cause, paste semantic policy, bounds, rollback behavior, and validation evidence.

## Completion checklist

- [x] Root cause is reproduced or bounded failure is proven.
- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [ ] Work order is `in-review` before fresh validation and becomes `complete` only after a passed validation.

## Implementation evidence

- Changed [`crates/scriptcraft-core/src/editing.rs`](../../../crates/scriptcraft-core/src/editing.rs): added the explicit transactional `insert_paste()` boundary. Embedded LF/CRLF/CR, whitespace, and Unicode remain canonical text in one semantic element; layout remains responsible for visual line breaks and one paste remains one undo step.
- Changed [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs): paste events use `insert_paste()` and preserve conditional caret reveal behavior.
- Added [`crates/scriptcraft-core/tests/wo002_paste_safety.rs`](../../../crates/scriptcraft-core/tests/wo002_paste_safety.rs): fixture matrix covers spaces, repeated whitespace, tabs, newline conventions, blank lines, Unicode, bounded large paste, layout, serialization, undo, oversized rejection, rollback, and continued editing.
- The reproduction matrix found no `Model(Invalid)` defect for these dimensions. Oversized text returns typed `EditError::TextTooLong`; the prior session remains unchanged and editable. No model limits or newline semantics were changed.
- Follow-up reproduction identified the multi-paragraph failure as chained split IDs: each Enter previously appended `-split-N` to the prior element ID until the bounded 128-byte stable-ID rule returned `Model(InvalidId)`. Split IDs now use short, collision-checked `element-N` candidates, so paragraph growth no longer depends on nesting depth.
- [`EditingSession::validate()`](../../../crates/scriptcraft-core/src/editing.rs:109) exposes a non-mutating live session health probe. The desktop error status now reports the exact operation error plus session validation and live JSON validity/byte size.
- Validation evidence: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo build --workspace` all passed.
- Native manual smoke remains a validation follow-up because this session did not operate a visible desktop window.

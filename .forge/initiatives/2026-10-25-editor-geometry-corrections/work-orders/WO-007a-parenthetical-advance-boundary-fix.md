# Work Order: WO-007a — Parenthetical Advance Boundary Fix

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer after approval
- **Phase/order:** Follow-up fix for WO-007; before WO-007 validation/closeout
- **Initiative:** [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:1)
- **Predecessor:** [`WO-007-parenthetical-transition-and-boundary-policy.md`](WO-007-parenthetical-transition-and-boundary-policy.md:1)
- **Validation:** [`../validations/validation-WO-007bc-v2.md`](../validations/validation-WO-007bc-v2.md)
- **Fixes:** [`WO-007b-rendered-parenthetical-key-route.md`](WO-007b-rendered-parenthetical-key-route.md)

## Problem and diagnosis

The current parenthetical transition helper only accepts a repair/advance when
the caret offset equals the complete element byte length at
[`parenthetical_repair_boundary()`](../../../crates/scriptcraft-core/src/editing.rs:652).
That excludes the normal inner-text-end caret in a closed pair: for `(words)`,
the caret after `s` is immediately before `)` rather than at `text.len()`.
Consequently Tab/Enter take the ordinary non-empty-parenthetical no-op branch
instead of creating the next Dialogue element.

The incomplete `(words` shape is intended to transition at its byte end through
the existing repair branch, but the reported desktop behavior demonstrates that
the end-boundary path is not reliably observable through the actual caret/input
route. The fix must therefore verify both the semantic boundary predicate and
the desktop mapping/dispatch path; it must not assume that a passing isolated
helper test proves the user-visible command works.

## Objective

Make Tab and Enter advance from the inner-text end of supported Parenthetical
shapes while preserving the repaired/current Parenthetical above a new empty
Dialogue. Specifically:

- `(words)`: a collapsed caret immediately after `s` and before `)` advances;
  the pair remains unchanged and the new Dialogue is selected at offset 0.
- `(words`: a collapsed caret immediately after `s` at the UTF-8-safe end
  repairs the closing `)`, then advances to a new Dialogue at offset 0.
- `text)` and `text` retain the WO-007 accepted-shape behavior and are not
  regressed.
- Interior positions, positions before the opening delimiter, and unsupported
  malformed shapes remain no-ops for Tab/Enter according to WO-007.
- Both Tab and Enter use the same boundary policy, setting precedence,
  transactional history, and generated-provenance rules already established by
  WO-007.

## In scope

- Correct the centralized Parenthetical shape/boundary analysis in
  [`editing.rs`](../../../crates/scriptcraft-core/src/editing.rs:652).
- Add deterministic core regressions for closed-pair inner-end advancement,
  missing-closing-delimiter repair, both commands, UTF-8 text, no-op positions,
  resulting caret positions, one-step undo/redo, and rollback invariants.
- Add or adjust desktop/input-path regression coverage in
  [`main.rs`](../../../apps/scriptcraft/src/main.rs:708) if the reported
  discrepancy is caused by caret mapping or event dispatch rather than the core
  predicate alone.
- Preserve the existing [`WO-007`](WO-007-parenthetical-transition-and-boundary-policy.md:1)
  setting behavior, persistence, cache identity, delete matrix, strict
  Scene Heading/Character/Transition boundaries, and before-opening Enter rule.
- Update only the affected core/desktop blueprint or requirement wording if the
  accepted inner-text-end contract is not already explicit; record the evidence
  in this work order and the initiative execution log.

## Out of scope

- General bracket matching, arbitrary delimiter completion, new roles, rich
  text, settings UI, or changes to canonical text outside the accepted
  Parenthetical shapes.
- Reopening completed [`WO-005`](WO-005-parenthetical-entry-policy.md:1),
  [`WO-005a`](WO-005a-invalidate-parenthetical-provenance.md:1), or
  [`WO-006`](WO-006-long-document-responsiveness.md:1) contracts.
- Layout geometry, pointer nearest-line policy, caret blink, export geometry,
  or performance architecture changes.
- Broad changes to the app key event system unrelated to proving the
  Parenthetical caret/input route.

## Acceptance criteria

### WO007A-AC-001 — Closed pair advances at inner-text end

For a Parenthetical with exact text `(words)`, place a collapsed valid caret at
the byte offset immediately after `s` and before `)`. Tab and Enter each create
one empty Dialogue immediately below, leave `(words)` unchanged, and select the
new Dialogue at offset 0. Each successful command is one undoable mutation;
undo restores the single original element and caret, and redo restores the
Dialogue result.

### WO007A-AC-002 — Missing closing delimiter repairs and advances

For `(words`, place a collapsed valid caret at `text.len()` immediately after
`s`. Tab and Enter each change the current text to `(words)`, insert one empty
Dialogue below, and select that Dialogue at offset 0. The inserted closing
delimiter is the only text repair. The operation remains transactional and
undoable.

### WO007A-AC-003 — Boundary and malformed-shape matrix remains exact

Add explicit tests for UTF-8-safe inner ends and no-op positions. The accepted
closed-pair boundary is before the final `)`; the accepted incomplete-shape
boundary is the complete byte length. Interior positions, before `(`, after the
closing `)` when not an accepted inner end, and unsupported malformed delimiter
arrangements do not mutate project, selection, revision, dirty state, or
history. Existing `text)` and `text` repairs retain their exact WO-007 text and
caret results.

### WO007A-AC-004 — Actual desktop route reaches the semantic boundary

If the defect includes caret mapping or event dispatch, add a focused app
regression proving that the caret produced by the rendered editor at the visual
position after the final inner character maps to the intended source byte
offset, and that both Tab and Enter dispatch to [`EditingSession::tab()`](../../../crates/scriptcraft-core/src/editing.rs:354)
and [`EditingSession::enter()`](../../../crates/scriptcraft-core/src/editing.rs:281)
without an intermediate error/no-op. Do not loosen UTF-8 boundary validation or
add a second text buffer.

### WO007A-AC-005 — Predecessor and WO-007 contracts preserved

Run the existing WO-005/005a and WO-007 focused suites unchanged. Preserve
generated-provenance invalidation, setting-off precedence, empty-delete
branches, before-opening Enter, strict transition boundaries, save/reopen
compatibility, cache/replace behavior, and failed-edit rollback. No completed
predecessor artifact is rewritten.

## Required validation gates

- Focused parenthetical tests, including this regression, must pass.
- Run and record `cargo fmt --all -- --check`.
- Run and record `cargo check --workspace`.
- Run and record `cargo test --workspace`.
- Run and record `cargo clippy --workspace --all-targets -- -D warnings`.
- Run and record `cargo build --workspace`.
- Run and record `git diff --check`.
- Native typing smoke is optional/unavailable unless actually performed; do not
  infer it from headless or rendered tests.

## Durable-document impact

- **Requirements:** `verify unchanged` unless the inner-text-end boundary is
  absent from the approved writing-loop wording; if clarification is needed,
  update only the existing Parenthetical transition contract.
- **Blueprint:** `verify unchanged` or make the same narrowly scoped contract
  clarification in [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:67).
- **Rationale/evidence:** User-reported failure on 2026-10-27 and source
  inspection show that the closed-pair inner-text-end boundary is not accepted;
  the incomplete-shape route also requires an end-to-end caret/input regression.
- **Authorized documentation changes:** This work order, the initiative
  execution log, the new validation artifact, and only the directly affected
  requirement/blueprint wording if needed. Do not modify completed predecessor
  history.

## Required implementation handoff

Implementation must report the exact root cause found for the `(words` failure
at the desktop boundary, list changed product/test files, show focused and full
gate results, and leave this work order `in-review` pending a fresh validator.
The validator must compare the user-visible cases rather than accepting only a
helper-level unit test.

## Implementation evidence — 2026-10-27

- **Root cause:** [`parenthetical_repair_boundary()`](../../../crates/scriptcraft-core/src/editing.rs:652) required `offset == text.len()` for every accepted shape, so a closed `(words)` pair could not advance from the inner-text-end caret at the byte offset immediately before `)`. The incomplete `(words` branch was already semantically valid at `text.len()`; the new end-to-end desktop key-route regression confirms that this path now reaches the core operation.
- **Implementation:** Closed pairs longer than `()` now accept only the inner-text-end boundary (`text.len() - 1`) in addition to the existing incomplete/missing-opening end boundaries. Repair and insertion behavior remains centralized and unchanged for the other accepted shapes.
- **Changed product/test files:** [`editing.rs`](../../../crates/scriptcraft-core/src/editing.rs:652), [`wo007_parenthetical_policy.rs`](../../../crates/scriptcraft-core/tests/wo007_parenthetical_policy.rs:163), and [`main.rs`](../../../apps/scriptcraft/src/main.rs:1482). No layout, persistence, settings, predecessor, or WO-006 product contract was changed.
- **Focused evidence:** `cargo test -p scriptcraft-core --test wo007_parenthetical_policy` passed with 7 tests; `cargo test -p scriptcraft` passed with 31 tests. Coverage includes `(words)` and `(words` for both Enter and Tab, UTF-8 inner-end handling, interior no-op behavior, undo/redo, and the desktop key dispatch route.
- **Required gates:** `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace`, and `git diff --check` all passed on 2026-10-27. Native typing smoke was not performed and is not inferred.
- **Handoff:** WO-007a remains `in-review`; fresh independent validation is required before completion.

## WO-007b fix-chain evidence — 2026-10-27

- The WO-007a validation hard stop `EGC-WO007A-F-001` was addressed by [`WO-007b-rendered-parenthetical-key-route.md`](WO-007b-rendered-parenthetical-key-route.md), without changing the centralized boundary semantics implemented here.
- The replacement app regression renders `(words)`, obtains the inner-text-end byte offset from a pointer hit at the visual position after `s` and before `)`, then routes both Tab and Enter through the focused production surface path. It does not directly assign the source offset or call the private key helper.
- Focused and full Cargo evidence is recorded in WO-007b and the initiative execution log. WO-007a remains `in-review` pending fresh validation of the complete chain.

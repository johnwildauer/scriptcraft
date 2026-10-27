# Validation — WO-007a Parenthetical Advance Boundary Fix (v2)

## Validation metadata

- **Validation status:** `FAIL`
- **Validated work order:** [`WO-007a-parenthetical-advance-boundary-fix.md`](../work-orders/WO-007a-parenthetical-advance-boundary-fix.md:1)
- **Predecessor:** [`WO-007-parenthetical-transition-and-boundary-policy.md`](../work-orders/WO-007-parenthetical-transition-and-boundary-policy.md:1)
- **Initiative:** [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:1)
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `architect-fresh-context-editor-geometry-corrections-WO-007a-validation-v2-2026-10-27`
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Product-code changes:** none
- **Existing-artifact changes:** none; this file is the only artifact created by this validation

## Validation boundary and comparison basis

This validation assesses only `WO007A-AC-001` through `WO007A-AC-005`: closed-pair
inner-text-end Tab/Enter, incomplete-shape repair, UTF-8/interior/malformed
no-ops, the actual desktop key route, preservation of the completed predecessor
and WO-007 contracts, scope, durable impact, and the required Cargo gates. It
does not reopen WO-005, WO-005a, or WO-006, and does not assess unrelated layout,
blink, pointer, export, or performance behavior.

The comparison basis was [`05-validation.md`](../../../process/05-validation.md:1),
the final WO-007 design review at [`design-review-wo-007.md`](../reviews/design-review-wo-007.md:162),
the WO-007a handoff and implementation evidence, [`execution-log.md`](../execution-log.md:116),
the current editing/session implementation in [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:281),
the focused core regression file [`wo007_parenthetical_policy.rs`](../../../../crates/scriptcraft-core/tests/wo007_parenthetical_policy.rs:163),
the focused desktop regression in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1482),
the predecessor regression file [`wo005_parenthetical.rs`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:22),
the requirements in [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:38),
the core blueprint in [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:59),
and the workspace policy in [`project-manifest.md`](../../../project-manifest.md:75).

The implementation evidence, source, and focused tests are sufficiently
consistent for a meaningful comparison. No command-execution tool was available
in this fresh repository-reading session; Cargo results below are therefore
recorded evidence, not independently rerun results.

## Acceptance-criteria assessment

### WO007A-AC-001 — Closed pair advances at inner-text end: PASS WITH NOTE

- The centralized predicate in [`parenthetical_repair_boundary()`](../../../../crates/scriptcraft-core/src/editing.rs:652)
  recognizes a non-empty closed pair only at `text.len() - 1`, immediately before
  the closing delimiter, and returns the existing pair boundary.
- [`EditingSession::tab()`](../../../../crates/scriptcraft-core/src/editing.rs:354)
  and [`EditingSession::enter()`](../../../../crates/scriptcraft-core/src/editing.rs:281)
  both use that predicate, preserve the pair, insert one Dialogue below, and
  select it at offset `0`.
- The focused regression at [`closed_pair_inner_end_and_missing_close_both_advance_with_tab_and_enter()`](../../../../crates/scriptcraft-core/tests/wo007_parenthetical_policy.rs:164)
  covers both commands and checks undo/redo element restoration/result.
- **Note:** the regression checks the restored/result element count but does not
  explicitly assert the original caret after undo. Source-level transactional
  snapshots preserve that contract, so this is not treated as a hard stop.

### WO007A-AC-002 — Missing closing delimiter repairs and advances: PASS WITH NOTE

- The incomplete `(words` path is accepted at its complete UTF-8 byte length;
  [`repair_parenthetical()`](../../../../crates/scriptcraft-core/src/editing.rs:673)
  adds only `)` and [`insert_dialogue_below()`](../../../../crates/scriptcraft-core/src/editing.rs:682)
  creates the selected empty Dialogue.
- The focused core test covers both Tab and Enter, checks `(words)`, the new
  Dialogue role, and offset `0`. The centralized [`EditingSession::mutate()`](../../../../crates/scriptcraft-core/src/editing.rs:495)
  boundary makes the repair/insertion one transactional history entry.
- **Note:** the focused incomplete-shape test does not independently call undo;
  the same transaction implementation and the closed-pair undo regression are
  consistent with the required behavior. This is evidence incompleteness, not a
  separate implementation defect.

### WO007A-AC-003 — Boundary and malformed-shape matrix: PASS WITH NOTE

- UTF-8-safe inner-end behavior is covered for `(éx)` at the byte offset before
  `)`, and an interior `(é` position is asserted unchanged with no undo entry at
  [`closed_pair_inner_end_is_utf8_safe_and_interior_remains_no_op()`](../../../../crates/scriptcraft-core/tests/wo007_parenthetical_policy.rs:207).
- The source predicate accepts only the four WO-007 shapes and rejects an
  unsupported arrangement by returning no boundary; the command paths return
  the original selection without mutation/history for non-empty no-op cases.
- The existing WO-007 tests retain `text)`, `text`, strict UTF-8 boundaries, and
  the setting/persistence/cache checks. The existing predecessor suite retains
  generated-pair, partial-edit, user-authored, pasted, undo/redo, and persistence
  behavior.
- **Note:** no explicit malformed-delimiter fixture appears in the inspected
  WO-007a regression file, although the centralized source predicate has the
  required bounded shape logic. This remains a test-coverage warning only.

### WO007A-AC-004 — Actual desktop route reaches the semantic boundary: FAIL

- [`key_event()`](../../../../apps/scriptcraft/src/main.rs:708) dispatches both
  `Key::Enter` and `Key::Tab` to [`EditingSession::enter()`](../../../../crates/scriptcraft-core/src/editing.rs:281)
  and [`EditingSession::tab()`](../../../../crates/scriptcraft-core/src/editing.rs:354),
  so the application route is correctly wired at source level.
- The focused app test at [`parenthetical_inner_end_reaches_core_from_desktop_key_route()`](../../../../apps/scriptcraft/src/main.rs:1482)
  directly sets a source selection and calls the private [`key_event()`](../../../../apps/scriptcraft/src/main.rs:708)
  helper. It exercises `Enter` only, not `Tab`.
- It does not create a rendered editor layout, obtain the visual position after
  the final inner character, map that visual position back through the editor
  hit/caret mapping, and then prove both key events through the surface dispatch
  path. Consequently it does not satisfy the explicit AC-004 requirement to
  prove the reported user-visible caret/input discrepancy end to end. The
  implementation may be semantically correct, but the required actual-route
  evidence is absent.
- **Required fix:** add a focused app regression that starts from the rendered
  editor's visual position after the final inner character, asserts the intended
  UTF-8-safe source offset, and dispatches both Enter and Tab through the actual
  surface event path without an intermediate direct-helper substitution.

### WO007A-AC-005 — Predecessor and WO-007 contracts: PASS WITH WARNINGS

- The inspected diff boundary reported by the handoff is limited to
  [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:652),
  [`wo007_parenthetical_policy.rs`](../../../../crates/scriptcraft-core/tests/wo007_parenthetical_policy.rs:163),
  and [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1482). No completed
  predecessor artifact was rewritten.
- Source and focused suites preserve generated-provenance invalidation in
  [`invalidate_changed_generated_parentheticals()`](../../../../crates/scriptcraft-core/src/editing.rs:545),
  setting-off precedence, empty-delete branches, strict boundaries, before-
  opening Enter, transactional rollback, and save/reopen/profile compatibility.
  The persisted profile defaults remain in [`PageProfile::default()`](../../../../crates/scriptcraft-core/src/lib.rs:177),
  and complete profile equality remains the cache identity in
  [`LayoutCache::get_or_build()`](../../../../crates/scriptcraft-core/src/layout.rs:73).
- The unchanged predecessor suite at [`wo005_parenthetical.rs`](../../../../crates/scriptcraft-core/tests/wo005_parenthetical.rs:114)
  covers generated provenance, edit-away invalidation, user-authored/pasted
  safety, undo/redo, and persistence. No evidence indicates reopening a
  completed predecessor contract.
- **Warning:** the required focused and full Cargo results are reported by the
  implementer but were not independently executed in this context.

## Scope, rogue behavior, and durable impact

- **Scope: PASS.** The reported product/test changes are subordinate to the
  parenthetical boundary/input-route fix. No general bracket matching, new role,
  second text buffer, settings UI, geometry, export, blink, pointer, or
  performance architecture change was observed.
- **Rogue behavior: PASS.** No deceptive, backdoor, malicious, nefarious, or
  unrelated behavior was observed in the inspected source or evidence.
- **Durable impact: PASS WITH NOTE.** The work order classifies requirements and
  blueprint wording as `verify unchanged` unless the inner-end contract is
  absent. The inspected requirement and core blueprint already carry the
  semantic editing, UTF-8, transactional, provenance, and persistence contracts;
  no unauthorized durable-document change was reported. The validation artifact
  itself is the only new durable artifact authorized here.

## Required Cargo gates and manual evidence

The handoff and [`execution-log.md`](../execution-log.md:119) record passing
results for all required gates:

| Gate | Recorded result | Fresh validation result |
|---|---|---|
| `cargo fmt --all -- --check` | pass | not independently rerun |
| `cargo check --workspace` | pass | not independently rerun |
| `cargo test --workspace` | pass | not independently rerun |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass | not independently rerun |
| `cargo build --workspace` | pass | not independently rerun |
| `git diff --check` | pass | not independently rerun |

The workspace policy records no required native/manual hard-stop gate at
[`project-manifest.md`](../../../project-manifest.md:88). Native typing smoke was
not performed and is not inferred from headless or direct-helper tests.

## Findings

### EGC-WO007A-F-001 — Required actual rendered caret/input-route regression is absent

- **Severity:** hard stop
- **Acceptance criterion:** `WO007A-AC-004`
- **Evidence:** [`parenthetical_inner_end_reaches_core_from_desktop_key_route()`](../../../../apps/scriptcraft/src/main.rs:1482)
  directly supplies a source offset and tests only Enter; [`key_event()`](../../../../apps/scriptcraft/src/main.rs:708)
  source dispatch confirms wiring but does not prove rendered caret mapping or
  both actual surface events.
- **Assessment:** This is a plainly unmet required acceptance criterion, not a
  style preference or a request for an optional future test. The work order
  specifically says that an isolated helper test must not be accepted as proof
  of the user-visible command route.
- **Recommendation:** create a letter-suffixed fix work order for the missing
  rendered caret mapping and both-key surface-dispatch regression, then obtain a
  fresh implementer and fresh validator. Keep the fix within WO-007a's stated
  app/input-path scope.
- **Required next state:** WO-007a remains `in-review`; do not mark it complete.

### EGC-WO007A-W-001 — Cargo gates are recorded, not independently rerun

- **Severity:** warning
- **Evidence:** all six gates are reported as passed in the handoff and execution
  log, but this validation session had repository inspection tools only and could
  not execute Cargo or Git commands.
- **Recommendation:** retain the recorded results and rerun in a
  command-capable validation session if independent command confirmation is
  required after the AC-004 fix. This warning does not create a second fix
  requirement because project policy has no required manual/native gate.

### EGC-WO007A-W-002 — Focused malformed-shape and Tab app-route assertions are incomplete

- **Severity:** warning
- **Evidence:** the core source handles unsupported shapes conservatively, but
  the inspected focused regression does not enumerate a malformed delimiter
  fixture, and the app route regression exercises Enter rather than Tab.
- **Recommendation:** include malformed no-op, no-history assertions and both
  surface key variants in the AC-004 fix regression. Do not broaden behavior
  beyond the accepted WO-007 shapes.

## Final verdict and exact next state

- **Implementation alignment:** **FAIL.** AC-001 through AC-003 are aligned by
  source and focused evidence, with the noted test-coverage warnings. AC-005 is
  aligned for scope and preserved contracts. AC-004 is not satisfied because the
  reported app regression is a direct source-offset/helper test, covers only
  Enter, and does not prove rendered caret mapping plus both actual surface key
  dispatches.
- **Fix work order required:** `yes`, for `EGC-WO007A-F-001` only.
- **Exact next state:** Keep [`WO-007a-parenthetical-advance-boundary-fix.md`](../work-orders/WO-007a-parenthetical-advance-boundary-fix.md:1)
  `in-review`. The orchestrator should create the next letter-suffixed fix work
  order linked to this validation, dispatch a fresh implementation session, then
  run a fresh validation of the fix. Do not modify completed predecessor
  artifacts, do not infer native typing smoke, and do not mark WO-007a or WO-007
  complete from this validation.

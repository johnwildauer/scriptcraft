# Work Order: WO-008 — Dialogue Enter Split Policy

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer after design approval
- **Phase/order:** `8 of 8; amendment after WO-007 chain`
- **Initiative:** [`../editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md)
- **Predecessor:** [`WO-007-parenthetical-transition-and-boundary-policy.md`](WO-007-parenthetical-transition-and-boundary-policy.md)
- **Validation:** [`../validations/validation-WO-008-v2.md`](../validations/validation-WO-008-v2.md)

## Summary

Make Enter in a Dialogue element preserve the existing Dialogue→Action behavior only at the end of the Dialogue. When text remains after the caret, split into another Dialogue element instead of converting the following fragment to Action.

## In scope

- Dialogue Enter at `offset == text.len()` keeps the current approved behavior: the new element below uses the existing Dialogue→Action transition.
- Dialogue Enter at a valid interior UTF-8 boundary creates a new Dialogue element below; the suffix remains Dialogue text and the caret selects the new Dialogue at offset 0.
- Preserve prefix/suffix text, element order, stable IDs, UTF-8 boundaries, undo/redo, dirty/revision behavior, role casing policy, and layout/pagination determinism.
- Add core tests for empty, end-boundary, interior ASCII, interior Unicode, first/last element, repeated splits, undo/redo, and save/reopen; add app event coverage through Enter dispatch if needed.

## Out of scope

- Changing Dialogue behavior for Tab, Parenthetical policy, Scene Heading/Character/Transition boundaries, layout geometry, or performance caching.
- Reopening completed WO-005/WO-007 contracts except to prove compatibility.

## Requirements

- **REQ-WRITING-LOOP-001:** “As an individual screenwriter, I want to create and edit an ordered screenplay with explicit element roles, so that my draft retains screenplay meaning while I write.”
- **AC-WRITING-001.3:** “Given a valid selection, the system shall support insertion, deletion, split, join, role change, undo, and redo while preserving valid text boundaries and element order.”
- **AC-WRITING-001.4:** “Given a role transition such as scene heading → action or character → dialogue, the system shall apply an explicit, testable transition policy and make the operation undoable.”
- **AC-WRITING-001.5:** “Given malformed or out-of-bound input, the system shall return a bounded error without corrupting the project or session state.”
- **NFR-WRITING-001:** “Production paths should prefer typed errors and bounded inputs over panic-prone handling; a discovered crash or data-loss defect requires a regression test.”

## Exact behavior oracle

| Current role | Caret | Enter result | Resulting caret |
|---|---|---|---|
| Dialogue | collapsed at `text.len()` | Existing Dialogue→Action transition; split suffix is Action | New element, offset 0 |
| Dialogue | collapsed at an interior UTF-8 boundary | Split at caret; prefix remains Dialogue and suffix is Dialogue | New Dialogue suffix, offset 0 |
| Dialogue | non-collapsed selection | Existing invalid-selection behavior unless an approved future selection rule applies | No partial mutation |

The interior split must preserve the suffix exactly, including Unicode, and must not assign Action merely because Enter occurred inside Dialogue. Empty Dialogue has `text.len() == 0`, so it follows the end-boundary Dialogue→Action behavior.

## Observable state matrix

| Case | Prefix/suffix and roles | Selection after success | State/evidence |
|---|---|---|---|
| Empty Dialogue, first/last/repeated | Prefix empty; new suffix empty Action | New element, collapsed offset 0 | One history entry/revision increment; dirty follows mutation |
| Dialogue at end | Prefix is full original Dialogue; new empty Action | New element, offset 0 | Original stable ID retained; new bounded unique ID; undo/redo restore exact project/selection |
| Dialogue interior ASCII/Unicode | Prefix Dialogue before caret; suffix Dialogue after caret, byte-exact | New suffix element, offset 0 | Same one-history/revision rule and unique ID; Unicode boundary checked |
| Non-collapsed selection | No split | Selection/project/history/dirty/revision unchanged; existing error behavior retained | No partial mutation |
| Invalid UTF-8 boundary | No split | All session state unchanged; typed boundary error | No history/revision/dirty change |
| `MAX_ELEMENTS` capacity | No split | All session state unchanged; bounded capacity error | No history/revision/dirty change |

Save/reopen preserves both element IDs, roles, and exact prefix/suffix bytes; caret/history remain transient. Existing non-Dialogue Enter transitions and WO-007/parenthetical behavior remain unchanged.

## Implementation guidance

1. Add a narrow Dialogue branch in [`EditingSession::enter()`](../../../crates/scriptcraft-core/src/editing.rs:257): choose the next role from caret position before calling the shared split primitive.
2. Reuse transactional [`mutate()`](../../../crates/scriptcraft-core/src/editing.rs:478), UTF-8 validation, bounded IDs, and the existing split helper.
3. Keep the current role-transition matrix unchanged for all non-Dialogue roles and for Dialogue at the end boundary.
4. Add no new setting: this is a deterministic correction to the existing Enter policy.

## Files and systems

- **Update:** [`crates/scriptcraft-core/src/editing.rs`](../../../crates/scriptcraft-core/src/editing.rs), [`crates/scriptcraft-core/tests/wo008_dialogue_enter.rs`](../../../crates/scriptcraft-core/tests/wo008_dialogue_enter.rs), and [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs) only if a focused rendered/event dispatch assertion is needed.
- **Documentation:** Update [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md), this WO, and [`execution-log.md`](../execution-log.md).
- **Avoid changing:** Parenthetical settings/provenance, layout/pointer/blink/performance behavior, and completed predecessor history.

## Validation

- Test exact end-boundary behavior and interior split behavior for ASCII and Unicode.
- Test empty Dialogue, first/last element, repeated interior splits, undo/redo, save/reopen, bounded IDs, failed selection, and no changes to non-Dialogue transition roles.
- Run all required Cargo gates; native typing smoke is optional/unavailable unless performed.

## Acceptance and state matrix

- **WO008-AC-001:** End-boundary Dialogue Enter preserves the approved transition: prefix remains Dialogue, new suffix element is Action, selection is new element offset 0, one history entry, revision increments once, and dirty state follows the mutation.
- **WO008-AC-002:** Interior Dialogue Enter splits at the exact UTF-8-safe caret boundary: prefix and suffix both remain Dialogue, suffix bytes are unchanged, selection is suffix offset 0, stable IDs are unique/bounded, and one undo restores the original element/selection while redo restores the split.
- **WO008-AC-003:** Empty Dialogue follows end-boundary behavior; non-collapsed selection retains existing invalid-selection behavior with no project/history/dirty/revision mutation.
- **WO008-AC-004:** Scene Heading, Character, Parenthetical, Transition, Action, and Note Enter behavior is unchanged; all existing role-transition tests remain green.
- **WO008-AC-005:** Save/reopen preserves the split roles/text/IDs and deterministic layout; failed edits preserve session state.

## Durable-document handoff

- **Requirements sections authorized:** [`REQ-WRITING-LOOP-001`](../../../requirements/features/screenplay-writing-loop.md:38), [`AC-WRITING-001.3`](../../../requirements/features/screenplay-writing-loop.md:46), [`AC-WRITING-001.4`](../../../requirements/features/screenplay-writing-loop.md:47), and [`AC-WRITING-001.5`](../../../requirements/features/screenplay-writing-loop.md:50). Add a Dialogue-specific criterion only if the product owner elects to freeze this refinement beyond the initiative.
- **Blueprint sections authorized:** [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:30) EditingSession responsibilities and [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:78) editing/layout integration contracts.
- **Rationale:** This is a semantic editing transition correction; no persisted schema or layout geometry changes are expected.

## Durable-document impact

- **Requirements:** `update` — this freezes a more precise Dialogue Enter transition criterion.
- **Blueprint:** `update` — the editing/session transition contract must distinguish end-boundary Dialogue→Action from interior Dialogue→Dialogue splitting.
- **Rationale/evidence:** User-reported behavior on 2026-10-27; current [`EditingSession::enter()`](../../../crates/scriptcraft-core/src/editing.rs:257) always derives Dialogue→Action regardless of remaining suffix text.
- **Authorized documentation changes:** Add the exact end/interior transition rule and tests; do not modify completed WO-005/WO-007 history.
- **Exact requirement amendment:** Amend [`AC-WRITING-LOOP-001.4a`](../../../requirements/features/screenplay-writing-loop.md:48) so Dialogue Enter is Action at collapsed UTF-8-safe `text.len()` and Dialogue→Dialogue at an interior UTF-8 boundary; all other listed transitions remain unchanged.
- **Exact blueprint clauses:** Update the [`EditingSession`](../../../blueprints/components/screenplay-core.md:29) contract and feature integration contract in [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:78). [`format-rules.md`](../format-rules.md) remains unchanged.

## Implementation evidence — 2026-10-27

- Implemented the narrow Dialogue branch in [`EditingSession::enter()`](../../../crates/scriptcraft-core/src/editing.rs:257). End-boundary and empty Dialogue retain Action; interior UTF-8-safe positions split into Dialogue prefix/suffix. All other Enter and Tab policies are unchanged.
- Added deterministic core coverage in [`wo008_dialogue_enter.rs`](../../../crates/scriptcraft-core/tests/wo008_dialogue_enter.rs) for the state matrix, ASCII/Unicode/repeated splits, stable bounded IDs, undo/redo, rollback/error cases, save/reopen, and capacity. Added app Enter dispatch coverage in [`main.rs`](../../../apps/scriptcraft/src/main.rs:1322).
- Required Cargo gates and changed-file evidence are recorded in the initiative [`execution-log.md`](../execution-log.md). This is implementer evidence only; fresh independent validation is required and this work order remains `in-review`.

## Validation outcome

- Fresh validation [`validation-WO-008-v2.md`](../validations/validation-WO-008-v2.md) returned `PASS WITH WARNINGS`.
- Warnings cover recorded-but-not-rerun Cargo gates, unavailable native typing smoke, and indirect invalid-boundary/save evidence; no fix work order is required.
- State transition: `in-review → complete`.

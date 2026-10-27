# Work Order: WO-007 — Parenthetical Transition and Boundary Policy

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer after v2 amendment approval
- **Phase/order:** `7 of 7; amendment after WO-005/005a`
- **Initiative:** [`../editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md)
- **Predecessors:** [`WO-005-parenthetical-entry-policy.md`](WO-005-parenthetical-entry-policy.md), [`WO-005a-invalidate-parenthetical-provenance.md`](WO-005a-invalidate-parenthetical-provenance.md)
- **Validation:** [`../validations/validation-WO-007bc-v2.md`](../validations/validation-WO-007bc-v2.md)

## Summary

Correct parenthetical Tab/Enter/Delete behavior after text is typed and deleted, define caret-position-sensitive advancement for non-empty parentheticals, and make parenthetical automation and strict transition boundaries project-level settings defaulting on.

## In scope

- Add persisted bounded `PageProfile.auto_parenthetical: bool` and `PageProfile.strict_transition_boundaries: bool`, default `true` for new and legacy-missing profiles; no schema bump.
- Dialogue→Parenthetical Tab inserts `()` and places the caret between delimiters when automation is enabled.
- `auto_parenthetical` gates both generated insertion and generated cleanup. When false, no automatic delimiters or generated cleanup occur; user-authored/loaded/unmarked pairs remain ordinary text.
- Empty Parenthetical Tab/Enter converts to Dialogue and removes parentheses at any caret position in/around the empty pair when automation is enabled.
- Empty Parenthetical Delete with caret before `(` or between delimiters removes the element and moves to the prior element end; for first/only element it leaves empty Dialogue at index 0/offset 0. Delete after `)` removes only `)` and leaves the caret after `(`. Non-collapsed selection uses ordinary deletion.
- Accepted shapes are `()`, `(text`, `text)`, and `text`; at the inner-text end Tab/Enter repairs a missing delimiter and creates Dialogue below, while interior positions are no-op. Other malformed shapes remain ordinary text.
- Enter before `(` inserts an empty Parenthetical above the current element; automation on yields generated `()`/offset 1, off yields empty text/offset 0; current content remains unchanged.
- Strict boundary setting disables Tab/Enter for non-collapsed selections or collapsed carets not exactly at UTF-8-safe `text.len()` in Scene Heading, Character, or Transition. Empty elements at offset 0 qualify.
- Preserve transactional UTF-8 boundaries, undo/redo, save/reopen compatibility, generated-provenance safety, and existing role transition matrices at valid boundaries.

## Out of scope

- General bracket matching, arbitrary delimiter completion, new roles, rich text, user-facing settings UI, or changing Dialogue semantics outside the specified transitions.
- Reopening or changing completed WO-006 performance behavior.

## Requirements

- **REQ-WRITING-LOOP-001:** “As an individual screenwriter, I want to create and edit an ordered screenplay with explicit element roles, so that my draft retains screenplay meaning while I write.”
- **AC-WRITING-001.3:** “Given a valid selection, the system shall support insertion, deletion, split, join, role change, undo, and redo while preserving valid text boundaries and element order.”
- **AC-WRITING-001.4:** “Given a role transition such as scene heading → action or character → dialogue, the system shall apply an explicit, testable transition policy and make the operation undoable.”
- **AC-WRITING-001.4a:** “Enter shall use the approved matrix: General/Note→same role, Scene Heading→Action, Action→Action, Character→Dialogue, Dialogue→Action, Parenthetical→Dialogue, and Transition→Scene Heading. Tab shall use Scene Heading→Action, Action→Character, blank Character→Transition, Transition→Scene Heading, Dialogue→Parenthetical, and Parenthetical→Dialogue; other Tab cases are consumed no-ops with no literal tab, mutation, or undo entry.”
- **AC-WRITING-001.5:** “Given malformed or out-of-bound input, the system shall return a bounded error without corrupting the project or session state.”
- **NFR-WRITING-001:** “Production paths should prefer typed errors and bounded inputs over panic-prone handling; a discovered crash or data-loss defect requires a regression test.”

## Exact command oracle

| Context | Caret position | Tab/Enter | Delete |
|---|---|---|---|
| Dialogue | any valid position | Tab enters Parenthetical with `()` and inner caret when enabled | Existing behavior |
| Empty Parenthetical `()` | before `(`, between, or after `)` | Convert to Dialogue, remove both delimiters | Remove whole element to previous-element end, except after `)` removes only `)` |
| Non-empty Parenthetical | immediately before closing `)` | Preserve pair and create new Dialogue below | Existing character deletion behavior |
| Non-empty Parenthetical | interior, including before `(` or before inner text end | Disabled/no-op | Existing character deletion behavior |
| Non-empty Parenthetical with missing closing delimiter | at end of inner text | Repair `)` then create new Dialogue below | Existing character deletion behavior |
| Scene Heading/Character/Transition | not at valid text end, strict setting on | Disabled/no-op | Existing behavior |
| Scene Heading/Character/Transition | at valid text end, strict setting on | Existing approved transition matrix | Existing behavior |
| Any role | strict setting off | Existing approved transition matrix | Existing behavior |

When `auto_parenthetical` is off, generated insertion and cleanup are disabled independently of strict boundary enforcement. Settings are owned by persisted `PageProfile`, survive save/reopen, default true when absent, and reject malformed JSON types.

### Empty-delete matrix

| Caret/selection | Result | Caret/history |
|---|---|---|
| Collapsed before `(` or between `()` | Remove element; previous element end | One undoable mutation; first/only element becomes empty Dialogue at index 0/0 |
| Collapsed after `)` | Remove only `)` | Caret remains at offset 1 after `(`; one undoable mutation |
| Non-collapsed | Ordinary selected deletion | Existing editing semantics; no special parenthetical branch |

### Delimiter-shape matrix

| Shape | Valid end boundary | Repair/action |
|---|---|---|
| `()` | offset 1 | Empty conversion/cleanup policy |
| `(text` | offset `text.len() + 1` | Insert trailing `)` then create Dialogue |
| `text)` | offset `text.len()` | Insert leading `(` then create Dialogue with caret mapped after repaired inner text |
| `text` | offset `text.len()` | Insert `(` and `)` around text, then create Dialogue |
| Other malformed pattern | n/a | Preserve ordinary text; Tab/Enter no-op unless exact supported boundary |

## Implementation guidance

1. Extend the persisted profile/settings seam with backward-compatible booleans and typed validation.
2. Keep generated provenance transient and reuse the existing transactional snapshot boundary.
3. Centralize parenthetical shape analysis: opening/closing delimiter presence, inner range, empty status, and whether the caret is at the inner end.
4. Make disabled commands successful no-ops so the UI does not display an operation failure or create undo history.
5. Implement element removal transactionally with safe previous-element caret fallback at document start.

## Files and systems

- **Update:** [`crates/scriptcraft-core/src/lib.rs`](../../../crates/scriptcraft-core/src/lib.rs), [`crates/scriptcraft-core/src/editing.rs`](../../../crates/scriptcraft-core/src/editing.rs), [`crates/scriptcraft-core/tests/wo007_parenthetical_policy.rs`](../../../crates/scriptcraft-core/tests/wo007_parenthetical_policy.rs), [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs) only if key dispatch/disabled state needs presentation support.
- **Update docs:** [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md), this initiative, and the execution log.
- **Avoid changing:** layout geometry, caret blink, pointer nearest-line policy, performance cache, and export geometry.

## Validation

- Deterministic tests for every table row, all valid UTF-8 caret positions, missing delimiters, text insertion/deletion/reconstruction, empty-pair Delete, previous-element fallback, document-start fallback, undo/redo, save/reopen, disabled settings, and no-op/no-history behavior.
- App tests verify disabled Tab/Enter commands do not show an error or mutate the session.
- Run all required Cargo gates and record native typing smoke as unavailable unless performed.

## Durable-document impact

- **Requirements:** `update` — this adds explicit user-facing transition, parenthetical, Delete, and strict-boundary policies.
- **Blueprint:** `update` — profile settings, transactional command semantics, and compatibility behavior change.
- **Rationale/evidence:** User-confirmed failure and desired behavior on 2026-10-27; completed WO-005/005a established provenance but not caret-sensitive transitions or project-level settings.
- **Authorized documentation changes:** Update the v2 initiative, requirements, [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md), this WO, and execution evidence. Do not change completed WO-005/005a history.

## Amendment traceability

- **WO007-AC-001:** With `auto_parenthetical=true`, Dialogue→Parenthetical Tab creates generated `()` with caret at offset 1; with it false, no automatic pair is inserted and no generated cleanup is performed.
- **WO007-AC-002:** Empty-pair Tab/Enter cleanup, collapsed Delete branches, first/only-element fallback, and non-collapsed delegation match the empty-delete matrix.
- **WO007-AC-003:** Supported delimiter shapes and inner-end boundaries match the delimiter-shape matrix; unsupported malformed shapes are preserved and commands are no-ops.
- **WO007-AC-004:** Strict boundaries mean collapsed caret at UTF-8-safe `text.len()`; non-collapsed or interior Tab/Enter are no-ops with no history/error mutation for Scene Heading, Character, and Transition.
- **WO007-AC-005:** `auto_parenthetical` and `strict_transition_boundaries` are `PageProfile` JSON fields, schema version remains 1, missing fields default true, malformed types fail typed validation, and save/reopen preserves values.

## Reconciled edge-result matrix

### Empty Parenthetical Delete

All rows require a collapsed valid selection; a non-collapsed selection delegates to ordinary selected deletion. Each special mutation is one undo entry, increments revision, and marks the session dirty when the project changes.

| Shape/position | Resulting project | Result selection |
|---|---|---|
| `()` caret offset 0 or 1, with predecessor | Remove the Parenthetical element | Predecessor element, `text.len()` |
| `()` caret offset 0 or 1, first/only element | Replace the element in place with empty Dialogue; preserve a valid one-element model | Element 0, offset 0 |
| `()` caret offset 2 after `)` | Remove only `)`; retain Parenthetical text `(` | Same element, offset 1 |

### Supported delimiter repair and transition result

At each valid inner-end boundary, the current Parenthetical is repaired and remains above a new empty Dialogue; the resulting selection is the new Dialogue at offset 0. The current element's role remains Parenthetical and its repaired text is:

| Input shape | Boundary | Repaired text before split | Resulting current caret boundary |
|---|---|---|---|
| `()` | offset 1 | `()` | 1, before `)`; no repair |
| `(text` | offset `("(text").len()` | `(text)` | 6, before inserted `)` |
| `text)` | offset `"text".len()` | `(text)` | 5, before existing `)` after inserted `(` |
| `text` | offset `"text".len()` | `(text)` | 5, before inserted `)` |

All other delimiter arrangements are ordinary text and Tab/Enter are no-ops unless they satisfy a listed shape/boundary. Existing text bytes are preserved except for the explicitly inserted delimiters.

### Enter before opening delimiter

This special case triggers only for a Parenthetical whose text begins with `(` and whose collapsed caret is exactly offset 0. It inserts one element immediately before the current element, leaves the current element byte-for-byte unchanged, and creates one undo entry. With `auto_parenthetical=true`, the inserted element is Parenthetical text `()` with generated provenance and selection offset 1; with it false, it is Parenthetical text empty with no generated provenance and selection offset 0. If the opening delimiter is absent, ordinary boundary/no-op rules apply; no inferred insertion is triggered.

### Setting lifecycle and cache semantics

`auto_parenthetical` and `strict_transition_boundaries` are owned by persisted `PageProfile` JSON keys in schema version 1. New profiles write `true`; missing legacy keys decode as `true`; wrong JSON types return typed errors. They round-trip through serialization and `replace_with_saved()` as project/profile data. WO-007 does not add a settings UI or a session setter, so no direct dirty/revision/history mutation is introduced; any future profile mutation must use the existing transactional project boundary. `LayoutCache` may include the complete `PageProfile` key (and therefore naturally miss when settings change), although these booleans do not alter geometry.

## Validation traceability matrix

- `WO007-AC-001`: setting-on/off insertion and cleanup, legacy/malformed persistence.
- `WO007-AC-002`: empty-delete table, first/only fallback, after-close delimiter deletion, non-collapsed delegation, undo/revision/dirty assertions.
- `WO007-AC-003`: four supported delimiter shapes, exact repaired text/caret, malformed-shape no-op.
- `WO007-AC-004`: before-opening Enter, automation on/off provenance and undo/redo, missing-opening no-trigger.
- `WO007-AC-005`: strict boundary end definition, empty/non-collapsed/interior no-op rows for Scene Heading/Character/Transition, and setting lifecycle/cache/replace round trips.


## Implementation evidence — 2026-10-27

- `PageProfile` now persists both default-on WO-007 booleans with schema-version-1 backward-compatible decoding and typed wrong-type rejection. `EditingSession` implements strict end-boundary no-ops, setting-gated insertion/cleanup, supported delimiter repairs, before-opening Enter, empty-delete fallback, transactional history, and UTF-8-safe selection validation.
- Deterministic coverage is in `wo007_parenthetical_policy.rs`, with profile compatibility/cache identity, setting-off behavior, empty-delete matrix, delimiter repairs, strict Unicode boundaries, before-opening Enter, rollback, and persistence round-trip assertions.
- Focused WO-007 tests pass; full Cargo gates remain the required final evidence. Native typing smoke was not performed and is not inferred.

## WO-007c fix-chain evidence — 2026-10-27

- [`WO-007c-delimiter-only-parenthetical-delete.md`](WO-007c-delimiter-only-parenthetical-delete.md) formalizes the approved delimiter-only Delete/Tab delta discovered during the WO-007b rendered-route review. Exact `(`, `()`, and `)` text is covered at every valid collapsed caret position; Delete removes later elements to the predecessor end and uses the empty Dialogue index-0 fallback for first/only elements, while Tab converts in place to empty Dialogue.
- The fix preserves the original WO-007 empty-delete and setting precedence, the WO-007a non-empty inner-text-end repair/advance behavior, and the WO-007b rendered pointer/focused key route. Focused tests also prove one-step transactional undo/redo and retain non-collapsed delegation.
- WO-007 remains `in-review`; WO-007a, WO-007b, and WO-007c remain pending fresh independent validation and no work order is marked complete.

## WO-007b fix-chain evidence — 2026-10-27

- The WO-007a rendered-route hard stop was isolated to missing production evidence, not to the approved Parenthetical boundary policy. [`WO-007b-rendered-parenthetical-key-route.md`](WO-007b-rendered-parenthetical-key-route.md) adds only the rendered pointer-hit and focused Tab/Enter regression required by the failed validation.
- WO-007 semantics, settings, delimiter repair, strict boundaries, provenance, transaction, persistence, and rollback behavior are unchanged. WO-007 remains `in-review` pending fresh independent validation through the WO-007a → WO-007b chain.

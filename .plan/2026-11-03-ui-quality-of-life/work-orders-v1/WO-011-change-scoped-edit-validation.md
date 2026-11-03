# RS-WO-011 — Change-scoped managed edit path and validation

- **Status:** Implementation submitted 2026-11-03; user review and validation pending; not user-validated
- **Initiative:** [`ui-quality-of-life-v1-render-speed.md`](../ui-quality-of-life-v1-render-speed.md)
- **Depends on:** [`RS-WO-010`](WO-010-copy-on-write-preservation.md)
- **Validation owner:** User; no agent-run checks or independent validators

## Objective

Avoid whole-source-tree searches and complete validation for ordinary single-paragraph edits by using a managed-paragraph index and validating only the changed element/path against a valid baseline.

Complete validation remains required at open, structural/global operations, Save/Save As, document replacement, and explicit public validation boundaries.

## Implementation steps

1. Add a managed-paragraph location index to preservation state, keyed by transient semantic identity and kept consistent with source ownership.
2. Replace repeated whole-tree presence and mutation lookups used by [`PreservationState::replace_managed_text()`](../../../crates/scriptcraft-core/src/preservation.rs:588) and [`PreservationState::reconcile_body_structure()`](../../../crates/scriptcraft-core/src/preservation.rs:843) with indexed lookup.
3. Rebuild or update the index transactionally on import, replacement, insertion, split, join, deletion, role reconciliation, derived-scene updates, and save rebase.
4. Split [`Project::validate()`](../../../crates/scriptcraft-core/src/lib.rs:488) consumers into complete validation and explicitly preconditioned change-scoped validation.
5. For ordinary single-paragraph text/role edits, validate the changed semantic element, changed managed source path, selection, and affected identity invariants against an already valid baseline.
6. Retain complete project/source validation for structural operations affecting global identities/references, open, replacement, native Save/Save As, and arbitrary public builders/callers.
7. Avoid invoking complete validation again solely because the desktop requests a layout from the already validated session. Keep standalone builder behavior safe and explicit.
8. Preserve exact no-op/dirty behavior and all transactional failure outcomes.

## Structural acceptance

- Ordinary text insertion performs no whole-tree managed-paragraph search and no complete source-tree validation.
- Index updates are atomic and cannot reference removed or stale managed identities after structural edits.
- Full validation still runs at every required global boundary; no save or import safeguard is skipped.
- Invalid changed data fails before publication and leaves the previous valid session untouched.

## Regression coverage to author

Cover lookup/index consistency across every structural operation, role changes, undo/redo, derived refresh, save rebase, and document replacement. Differential-test complete versus change-scoped validation on valid and invalid candidates, including stale IDs, duplicate ownership, invalid UTF-8 boundaries, dangling references, and unrelated pre-existing invalid fixtures. Verify standalone unvalidated callers cannot use the trusted fast path accidentally.

## Durable documentation

Define complete versus change-scoped validation preconditions in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:51) and retain the transactional editing requirements in [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:24) without weakening them.

## Implementation record — 2026-11-03

`PreservationState` now maintains a transient semantic-ID-to-source-location index (including all
locations for ambiguous ownership) and a source-text-budget summary. Index lookup replaces the
recursive whole-tree presence/mutation searches on managed text/role updates, direct-formatting
lookup, and structural source removal. Indexes are created on FDX import and validated session
construction and rebuilt after structural edits, derived-scene changes, saved rebase/replacement,
and undo/redo restores. The index and budget summary are derived state and are excluded from
preservation semantic equality. WO-010's `SharedVec`/`SharedText`/`SharedBytes` wrappers remain in
use; the index resolves a location before mutable access, retaining path-only COW detachment.

Ordinary insertion, same-element deletion/backspace, and role assignment attempt the private
change-scoped validator only from a complete-valid session baseline. It requires unchanged target
identity/count and same-document/layout state, exact equality of every non-target semantic element
and the full title page, same-element selection endpoints, valid affected semantic data and
selection, unique stable source location, exact equality of all source-tree nodes outside that
paragraph, unchanged source topology/identity and opaque attachments, exact source role/text
projection, unchanged global source/anchor/identity/reference/board inputs,
and a source-text budget within the complete validator's limit. Structural edits and all unprovable
or changed-topology cases use detached structural reconciliation followed by complete validation;
candidate publication remains atomic. Public Project validation and
standalone layout builders remain complete. The desktop uses a session-bound cache entry point so
layout construction from the validated live session does not invoke complete Project validation a
second time. Native Save/Save As, imports, replacement, explicit public validation, and undo/redo
restore complete validation/rebuild boundaries.

Value-free counters now report scoped attempts/accepts, session candidate full validations and
fallbacks, complete Project validation calls, index rebuilds/source nodes scanned/index lookups/
entries examined/fallback nodes visited. The editing API also exposes whether the most recent
successful mutation used scoped validation; diagnostic state is excluded from session equality.
Authored unit coverage exercises scoped text and role edits against complete validation, uncertain
source-topology fallback and rollback, full-path structural split/join and undo/redo index rebuilds,
stale IDs, duplicate managed ownership, dangling global references, invalid UTF-8 selection
boundaries, invalid baselines, changed non-target/title-page/source-tree fallback preconditions,
index counters, and opaque sibling sharing/retention. Layout diagnostics coverage contrasts complete validation for
standalone/public paths with the session-trusted path while comparing layout output. Existing
preservation/COW/FDX/save/history/derived coverage remains applicable. Tests were authored but not
run; no build, lint, formatting, manual verification, implementation checks, reviewers, or validators
were run. No timing result or performance improvement is claimed.

**Public API/correctness caveats:** the session-bound layout entry point is public and accepts only
an `EditingSession`; standalone Project builders still validate fully. New diagnostic structs and
reset/read functions are public, value-free structural counters with process-wide core lifetime.
`PreservationState` now includes private transient derived fields, so downstream construction by
public struct literal is no longer available; its other public semantic fields remain unchanged.
RS-WO-010's existing `SharedVec`/`SharedText`/`SharedBytes` public field-type compatibility caveat
is unchanged. Safe fallback can still perform complete source-tree work on unusual run mappings or
source topology changes; this is intentional. No unresolved implementation blocker is recorded.

**RS-WO-012 dependency:** perform user-owned review/validation of WO-001–011 and integrated
semantic/FDX equivalence, inspect the counters, and capture any user measurements there. WO-012 is
not implemented here; no user measurements or acceptance are claimed.

## User validation

Run the complete workspace and all editing/preservation/FDX suites. Exercise valid and invalid edits, split/join/delete, role transitions, undo/redo, Save/Save As, reopen, and injected failures. Confirm ordinary typing avoids full-tree diagnostics while global validation still catches the same invalid fixtures.

## Handoff

RS-WO-012 integrates all prior orders for final structural equivalence, user measurements, and durable closeout.

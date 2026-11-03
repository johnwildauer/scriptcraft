# RS-WO-010 — Copy-on-write preservation snapshots

- **Status:** Authorized; implementation submitted 2026-11-03; user review and validation pending; not user-validated
- **Initiative:** [`ui-quality-of-life-v1-render-speed.md`](../ui-quality-of-life-v1-render-speed.md)
- **Depends on:** [`RS-WO-009`](WO-009-scoped-scene-reconciliation.md)
- **Validation owner:** User; no agent-run checks or independent validators

## Objective

Make preserved source snapshots structurally shared so ordinary edits clone only the managed path being changed instead of original source bytes and the complete preserved tree.

Transactional candidates and exact undo/redo restoration remain mandatory.

## Implementation steps

1. Introduce copy-on-write storage for expensive leaves owned by [`PreservationState`](../../../crates/scriptcraft-core/src/preservation.rs:274): original source bytes, preserved elements/nodes, text/CDATA/comment payloads, and image payloads.
2. Represent untouched subtrees with immutable shared references. Keep the existing semantic element API unless separate evidence justifies changing it.
3. On a managed paragraph mutation, clone only the section-to-paragraph path and necessary sibling vectors; leave unrelated nodes shared.
4. Preserve detached candidate behavior in [`EditingSession::mutate()`](../../../crates/scriptcraft-core/src/editing.rs:702): any error discards the candidate without changing live state.
5. Ensure clean state, history, redo, detached save candidates, and reopened baselines cannot observe later mutations through shared storage.
6. Use shared-subtree identity to reduce exact no-op/dirty comparison cost while retaining correct clean-state detection when the user manually restores saved content.
7. Keep writer re-read, normalized source-tree comparison, identity/reference checks, conflict detection, and atomic publication unchanged.
8. Record snapshot sharing and path-clone counters without exposing source content.

## Structural acceptance

- Ordinary insertion does not deep-copy original source bytes, image payloads, or untouched preserved subtrees.
- Every candidate/snapshot remains logically immutable from every other candidate/snapshot's perspective.
- Undo/redo restores exact semantic and preservation structure, including opaque nodes and nested scene data.
- Shared storage does not alter native serialization or normalized tree equality.

## Regression coverage to author

Add mutation-isolation tests across live, clean, history, redo, derived, and save candidates. Cover every edit operation, generated Parentheticals, split/join/delete, role transitions, nested arcs, unsupported opaque content, images, save rebases, and injected failures. Compare serialized and normalized structures with deep-clone controls.

## Durable documentation

Document copy-on-write ownership and immutability in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:152). State explicitly that sharing is ephemeral implementation storage and does not change FDX authority, persistence, or save gates.

## User validation

Run all preservation, editing, history, FDX, and failure-injection suites. Exercise long-document typing, undo/redo, derived refresh, Save/Save As, reopen, and conflict handling. Inspect memory behavior and verify that no snapshot changes after later edits.

## Handoff

RS-WO-011 completes the ordinary-edit fast path with managed lookup and change-scoped validation against the already valid shared baseline.

## Implementation record — 2026-11-03

Preservation section/child vectors, text-like payloads, source bytes, and image payloads now use immutable `Arc`-backed wrappers in [`preservation.rs`](../../../crates/scriptcraft-core/src/preservation.rs). Cloning a wrapper shares storage; mutable access uses `Arc::make_mut`, and managed paragraph lookup first records an immutable index path before detaching vectors only while traversing the selected section-to-paragraph path. Shared storage equality first accepts pointer-identical values and otherwise retains deep value comparison, so separately edited-and-restored content remains equal. The ordered FDX source tree remains authoritative; the wrappers do not change serialization or save gates.

Detached project candidates, editing snapshots/history, derived candidates, and save-candidate preparation therefore retain independent mutation semantics while sharing untouched preservation storage, including original bytes and unaffected nested subtrees. The scoped scene candidate behavior and per-heading deferrals from WO-009 remain in place. Existing full project validation, whole-source managed lookup outside this preservation path, writer re-read, normalized tree/identity/reference gates, conflict detection, and atomic publication are unchanged; ordinary-edit indexed lookup and change-scoped validation remain WO-011 scope.

Value-free process-wide core counters in `preservation.rs` count shared wrapper clones and COW detachments separately for vectors, text payloads, original source bytes, and image payloads. They do not expose document values and are not currently projected into desktop status-bar diagnostics; they are structural counters, not timing or speed claims. Authored but unrun coverage includes preservation mutation isolation and sharing, source/image payload detachment, edit/history sharing, and derived- and save-candidate source-byte/unrelated-subtree sharing. This coverage does not exhaustively enumerate every edit operation or compare all outputs against a previous-implementation golden.

The structural criterion documented by WO-009 is met by this implementation design: cloning the detached project candidate clones `Arc` handles rather than original bytes or entire preserved child vectors, and the edited scene/paragraph path detaches only as mutation requires. This is implementation evidence only, not user review, validation, or acceptance. No checks, tests, builds, lint, formatting, manual verification, reviews, or validators were run.

**Public API caveat:** the public preservation structs' `Vec`, `String`, and `Vec<u8>` fields now use public wrapper types (`SharedVec`, `SharedText`, `SharedBytes`). Vector-like dereference and conversions cover common call patterns but do not establish source compatibility for all external callers. This source-level API change requires user review; no compatibility promise is made.

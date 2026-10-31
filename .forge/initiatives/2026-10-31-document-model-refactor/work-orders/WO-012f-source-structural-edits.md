# Work Order: WO-012f — Make source-anchored edits safely serializable

- **Status:** `implemented`; implementer checks passed. No independent validator dispatched under the initiative-specific user direction.

## Outcome and scope
Close the gap between editing transactions and writer reconciliation for ordinary structural edits of imported FDX, preserving authored/unknown content or rejecting before state changes.

## Components and implementation
- Coordinate [editing](../../../../crates/scriptcraft-core/src/editing.rs), [preservation](../../../../crates/scriptcraft-core/src/preservation.rs), [FDX writer](../../../../crates/scriptcraft-core/src/fdx.rs), and cast-list update. Define source anchors, foreign ID generation and ordering for insert, split, join, delete, role change and cross-element replacement. Preserve unknown interleaving and source children; keep source and semantic authority synchronized through undo/redo.
- Support ordinary safe insert/split/join/role changes and re-read after save. Where unknown mixed content, namespace context or authored nested-scene data cannot be transformed safely, reject before the edit mutates selection/history/dirty state, or escalate a concrete irreconcilable case. Do not block ordinary script edits solely for script/board cross-domain links; preserve board entities and internal references, as directed.
- Ensure cast correction also cannot create an unserializable edited candidate; retain no-op import and title-preserve-only behavior. Do not weaken codec resource/reference checks to pass tests.

## Acceptance and checks
- Headless edit → save bytes → reopen tests cover insert, split, join, delete, role change, cross-element edit, adjacent unknown paragraphs, mixed runs, nested scenes, foreign IDs, board independence, undo/redo and safe-stop rollback. Supported operations produce matching role/text/order and retained FDX content; unsafe operations leave all session state unchanged and give actionable typed errors.
- Run `cargo test -p scriptcraft-core editing`, `cargo test -p scriptcraft-core fdx`, `cargo test -p scriptcraft-core cast_list`, `cargo test --workspace`, and applicable format/lint/build checks; record results. Use only synthetic payloads.

## Risks and dependency
Depends on WO-012e and WO-004/009. One attributable commit, implementer checks only. Unknown source references and destructive nested-scene data are genuine blockers, not grounds to invent mappings or blanket cross-domain blocking. Hand source transformations to WO-012g.

## Implementation results — 2026-10-31

- Editing transactions now reconcile body order, source anchors and foreign paragraph identities as part of the same staged project candidate. Existing source IDs stay on surviving paragraphs; inserted/split paragraphs get collision-checked `ID` values and fresh internal anchors. Join/cross-element replacement retain the first paragraph identity; removed paragraphs are removed from the managed source tree only when their contents are structurally understood. Undo/redo restore semantic and preservation state together.
- The writer permits only paragraph-identity-set changes while still requiring the original non-paragraph identities, references and board graph to remain unchanged. It re-reads output and compares the emitted identity set, semantic body, and retained ordered source structure. Session-local semantic IDs/anchors are excluded from source-tree equality.
- Text edits map ordered FDX `Text` children and preserve run attributes. Insertion/split/join/delete/cross-element edit/role change were exercised through save bytes and reopen, including unknown paragraph interleaving, mixed runs, foreign-ID collision avoidance, undo/redo and board payload retention. Corrected role changes update only the known `Type` mapping.
- Opaque nested paragraph children block structural removal, split/join/replacement, and scene-heading role changes before the candidate publishes. An exact attribute-value match to a removed paragraph ID outside recognized board sections returns typed `PotentialReferenceConflict`; board-owned cross-domain references do not block script edits and the board tree remains unchanged. Text edits crossing opaque paragraph children also reject atomically. No generic discovery of undocumented/text-encoded references is claimed.
- No sidecar, private FDX extension, board editing, title-page editing or extra FDX feature mapping was introduced. Tests use synthetic XML only. No real Final Draft execution, external interoperability validation, or independent validation was performed or claimed.
- Checks passed: `cargo test -p scriptcraft-core editing` (22 selected editing unit tests); `cargo test -p scriptcraft-core fdx`; `cargo test -p scriptcraft-core cast_list`; `cargo test -p scriptcraft-core --test wo012f_source_structural_edits` (6); `cargo test --workspace` (314 passed: 155 desktop unit, 114 core unit, 45 integration, 0 doc tests); `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo check --workspace`; `cargo build --workspace`; `git diff --check`.

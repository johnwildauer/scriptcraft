# Work Order: WO-012f — Make source-anchored edits safely serializable

- **Status:** `draft / not started`; depends on WO-012e.

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

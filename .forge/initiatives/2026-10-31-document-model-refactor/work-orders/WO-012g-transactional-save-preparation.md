# Work Order: WO-012g — Prepare and rebase transactional FDX saves

- **Status:** `draft / not started`; depends on WO-012f.

## Outcome and scope
Provide a headless, testable save transaction that produces a publishable saved session and validated FDX bytes before filesystem publication, ready for desktop cutover in WO-012.

## Components and implementation
- Coordinate [editing session](../../../../crates/scriptcraft-core/src/editing.rs), [cast lists](../../../../crates/scriptcraft-core/src/cast_list.rs), [FDX codec](../../../../crates/scriptcraft-core/src/fdx.rs), [FDX storage](../../../../crates/scriptcraft-core/src/fdx_storage.rs) and desktop adapter seams. Stage pending edits, correct cast on a detached candidate, serialize and re-read, and prebuild a consistent rebased preservation/identity/clean baseline. Preserve or explicitly map selection and undo/redo; ensure subsequent edits and second saves use the new source baseline, not stale original bytes.
- Prepare all fallible session work before same-directory replacement. On storage success publish the candidate, source baseline, path and clean state in a non-failing coherent transition; on preparation/conflict/write/flush/sync/rename failure retain live session, pending edits, selection, history, path, old baseline and destination. A cleanup failure *after* successful Save As installation is a successful publication with a separately reported cleanup warning, not a failed save that pretends the destination is unchanged; cleanup failure during a pre-publication error accompanies that error. Cover both Save and Save As, including new files, older-version in-place conversion and no-overwrite Save As. No public FDX switch yet.
- Respect the storage implementation's documented race, platform and durability limits; do not claim stronger atomic guarantees or destroy source as a workaround. Keep non-6 warnings and original source unchanged on Open.

## Acceptance and checks
- Inject candidate/serialization/re-read, conflict, temporary creation, partial write, flush, sync, replacement and both pre- and post-publication cleanup failures; assert the published receipt and live state agree in the latter case. Test initially clean and dirty sessions, timer/pending edits, candidate cast correction, Save As source isolation, conflict baselines, success-only publication, undo/redo and a second edit/save/reopen. Confirm failure diagnostics do not disclose content or private paths.
- Run `cargo test -p scriptcraft-core fdx_storage`, `cargo test -p scriptcraft-core cast_list`, `cargo test -p scriptcraft-core fdx`, `cargo test --workspace`, and applicable format/lint/build checks; record exact results and platform limitations.

## Risks and dependency
Depends on WO-012f and WO-009–011. One attributable commit, implementer checks only. Stop if a post-replacement failure can leave disk and live state inconsistent; hand the prepared transaction and remaining limits to WO-012 for the public GUI switch.

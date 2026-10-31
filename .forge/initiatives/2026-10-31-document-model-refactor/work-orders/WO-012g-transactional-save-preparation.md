# Work Order: WO-012g — Prepare and rebase transactional FDX saves

- **Status:** `implemented`; implementation checks recorded below. No independent validation was dispatched under the initiative-specific user direction.

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

## Implementation results — 2026-10-31

- Added a headless `save_session` transaction with a runtime `FdxSaveTarget` holding path and conflict baseline. Save and Save As stage corrected cast-list data, serialized/validated bytes, an independent codec re-read with semantic-projection comparison, and a detached publishable session before calling storage. The storage receipt baseline is allocated before publication as well. Only a successful storage receipt publishes the rebased session, target path/baseline and clean state; storage failures retain the live project, selection, history, dirty state, target and destination. No desktop FDX save switch was enabled.
- Successful publication rebases the active preservation source snapshot/tree from the re-read bytes, synchronizes candidate cast-list corrections to the live session, preserves selection/history and remaps generated-parenthetical tracking. UTF-8 selection offsets are clamped safely if a corrected value shortens the selected text. Subsequent saves use the newly published conflict baseline and preservation source. Save As never overwrites its target or mutates the source. Older-version in-place conversion remains explicit-save-only and outputs Version 6.
- Storage cleanup failure before publication remains attached to the failure; post-install Save As cleanup failure is reported as a warning on a successful transaction, with the destination, live session and baseline published coherently. The implementation retains the backend's documented same-directory/platform behavior, final conflict-check race and durability limits; it does not claim directory-entry power-loss durability, cross-platform atomic replacement, or Final Draft interoperability.
- Added failure-injection coverage for candidate correction, serialization, re-read, conflict, temporary creation, partial write, flush, permission, sync, replacement, pre-publication cleanup and post-publication Save As cleanup. Success coverage includes clean/dirty sessions, pending edits, cast correction, selection/history/undo/redo, Save and Save As, existing-target refusal, source isolation, older-version conversion, a second edit/save/reopen, and foreign source retention. Diagnostics omit document content and paths.
- Implementer checks passed (2026-10-31):
  - `cargo test -p scriptcraft-core fdx_storage` — 16 selected tests passed.
  - `cargo test -p scriptcraft-core cast_list` — 7 selected tests passed.
  - `cargo test -p scriptcraft-core fdx` — 38 core unit tests and 1 matching integration test passed.
  - `cargo test --workspace` — 321 passed, 0 failed (155 desktop unit, 121 core unit, 45 integration, 0 doc tests).
  - `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo check --workspace`, `cargo build --workspace`, and `git diff --check` — passed (exit 0).
- These are implementation checks only; no independent validation agent was dispatched, no manual Final Draft test was performed, and no Final Draft interoperability claim is made. The implementation commit hands the headless save seam to WO-012; native UI cutover and later orders are not part of this work.

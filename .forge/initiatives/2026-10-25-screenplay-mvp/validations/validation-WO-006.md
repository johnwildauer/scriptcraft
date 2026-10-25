# Validation: WO-006 — Native Persistence and Recovery

## Validation metadata

- **Review type:** `validation`
- **Status:** `complete`
- **Validator:** Independent fresh-context validation agent
- **Validator session/agent ID:** `validation-session-2026-10-25-wo-006`
- **Session exception:** `none`
- **Date:** 2026-10-25
- **Initiative:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Validation file:** `validation-WO-006.md`
- **Validated work order:** [`WO-006-persistence-recovery.md`](../work-orders/WO-006-persistence-recovery.md)
- **Compared commit/diff:** No commit exists; the initiative branch is unborn and
  fully uncommitted, so comparison used the WO changed-file record, execution
  evidence, authorized blueprints, current source, and independent probes.
- **Fix chain:** `none`

## Authorized scope

The comparison covered [`05-validation.md`](../../../process/05-validation.md),
the approved [`WO-006-persistence-recovery.md`](../work-orders/WO-006-persistence-recovery.md),
the execution log, the persistence portions of
[`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) and
[`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md),
and the reported implementation in
[`persistence.rs`](../../../crates/scriptcraft-core/src/persistence.rs),
[`editing.rs`](../../../crates/scriptcraft-core/src/editing.rs),
[`lib.rs`](../../../crates/scriptcraft-core/src/lib.rs), and
[`main.rs`](../../../apps/scriptcraft/src/main.rs).

The assessed boundary was versioned bounded native JSON, semantic round trip,
malformed/unknown-version rejection, injected filesystem atomic replacement,
destination preservation on failure, dirty-state behavior, bounded recovery
rotation and corruption filtering, and host Open/Save/Recovery hooks. No
product source was edited by this validation session.

## Validation checklist

- The reported work, changed-file record, execution evidence, and current source
  are consistent enough for meaningful comparison.
- The changed files and behavior are within the WO-006 persistence/recovery and
  host-integration boundary; no layout, export, or unrelated behavior was found.
- No deceptive, backdoor, malicious, nefarious, or otherwise rogue behavior was
  introduced.
- The required Cargo format/check/build/test/clippy gates passed.
- The durable-document impact is consistent with the documented persistence and
  desktop blueprint updates.

## Findings

### Blockers

- None.

### Warnings

- **Stale host status label — Severity: warning.** The app wires Open, Save, and
  Recovery actions through [`ScriptCraftApp::open_native()`](../../../apps/scriptcraft/src/main.rs:58),
  [`ScriptCraftApp::save_native()`](../../../apps/scriptcraft/src/main.rs:46),
  and [`ScriptCraftApp::inspect_recovery()`](../../../apps/scriptcraft/src/main.rs:79),
  but the bottom status text still says `Persistence host boundary: not connected`
  at [`ScriptCraftApp::update()`](../../../apps/scriptcraft/src/main.rs:249).
  This is presentation inconsistency, not a failed persistence acceptance
  criterion, and does not require a fix work order under the validation process.
- **Manual visible-window smoke — Severity: warning.** No visual desktop smoke
  result is claimed in this headless validation. The required automated gates
  and host-hook source inspection were available and passed; the unavailable
  visual path is recorded without treating it as a hard stop.
- **Repository comparison — Severity: warning.** The branch remains unborn and
  uncommitted, so there is no commit diff. The WO changed-file record, source,
  execution log, and fresh behavioral results provide the comparison instead.

### Notes

- The implementation uses [`serialize()`](../../../crates/scriptcraft-core/src/persistence.rs:56)
  and [`deserialize()`](../../../crates/scriptcraft-core/src/persistence.rs:63)
  over the existing bounded model JSON, preserving semantic project fields while
  excluding session caret/layout/undo state.
- [`atomic_save()`](../../../crates/scriptcraft-core/src/persistence.rs:72)
  writes a sibling temporary path and renames only after preparation succeeds;
  [`EditingSession::mark_saved()`](../../../crates/scriptcraft-core/src/editing.rs:109)
  is called by the host only after a successful replacement.
- [`save_recovery()`](../../../crates/scriptcraft-core/src/persistence.rs:103)
  bounds retained snapshots to three revision-named files, while
  [`discover_recovery()`](../../../crates/scriptcraft-core/src/persistence.rs:126)
  skips malformed/unknown candidates, filters saved revisions, and returns
  candidates for explicit host offer rather than silent replacement.

## Independent evidence

- Standalone `/tmp/wo006_probe.rs` linked against the built core library passed:
  semantic round trip including Unicode/metadata, malformed JSON rejection,
  unknown schema-version rejection, injected rename failure with preservation
  of the prior destination, five-snapshot rotation to three, corrupt-candidate
  skipping/filtering, and dirty-to-clean session behavior after
  [`EditingSession::mark_saved()`](../../../crates/scriptcraft-core/src/editing.rs:109).
  Output: `round-trip/malformed/unknown: PASS`; `atomic failure preserves
  destination: PASS`; `rotation/corrupt candidate filtering: PASS`;
  `dirty/clean session behavior: PASS`.
- `cargo fmt --all -- --check`: passed.
- `cargo check --workspace`: passed.
- `cargo build --workspace`: passed.
- `cargo test --workspace -- --nocapture`: passed, 2 app tests, 19 core tests,
  and doc-test targets with zero failures.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.

## Decision

- **Outcome:** `pass`
- **Required next state:** [`WO-006-persistence-recovery.md`](../work-orders/WO-006-persistence-recovery.md) → `complete`
- **Fix work order required:** `no`


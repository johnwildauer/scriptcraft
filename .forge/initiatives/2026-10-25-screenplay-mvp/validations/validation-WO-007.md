# Validation: WO-007 — PDF/TXT Exports and Integration

## Validation metadata

- **Review type:** `validation`
- **Status:** `complete`
- **Validator:** Independent fresh-context validation agent
- **Validator session/agent ID:** `validation-session-2026-10-25-wo-007`
- **Session exception:** `none`
- **Date:** 2026-10-25
- **Initiative:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Validation file:** `validation-WO-007.md`
- **Validated work order:** [`WO-007-exports-integration.md`](../work-orders/WO-007-exports-integration.md)
- **Compared commit/diff:** No commit exists; the initiative branch is unborn and uncommitted. Comparison used the WO changed-file record, execution log, authorized blueprints, current source, focused tests, and fresh independent probes.
- **Fix chain:** `none`

## Authorized scope

The comparison covered [`05-validation.md`](../../../process/05-validation.md),
the initiative, [`WO-007-exports-integration.md`](../work-orders/WO-007-exports-integration.md),
the execution log, [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md),
the relevant desktop/core blueprints, and the reported implementation in
[`exports.rs`](../../../crates/scriptcraft-core/src/exports.rs),
[`layout.rs`](../../../crates/scriptcraft-core/src/layout.rs),
[`lib.rs`](../../../crates/scriptcraft-core/src/lib.rs), and
[`main.rs`](../../../apps/scriptcraft/src/main.rs).

The assessed boundary was fixed-layout deterministic PDF output, basic PDF
structure and page count, deliberately lossy TXT role/ID/page-layout reporting,
failed-export isolation, successful-export native dirty-state preservation, and
desktop host actions. No product source was edited by this validation session.

## Validation checklist

- The reported implementation, changed-file record, execution evidence, and current source are consistent enough for meaningful comparison.
- The changed files and behavior are within the WO-007 export/integration and authorized blueprint boundary; no unrelated feature behavior was found.
- No deceptive, backdoor, malicious, nefarious, or otherwise rogue behavior was introduced.
- The required format/check/build/test/clippy gates passed independently.
- The durable-document impact matches the recorded deterministic PDF and explicit TXT loss contract.

## Findings

### Blockers

- None.

### Warnings

- **Manual visible-window smoke — Severity: warning.** This headless validation did not claim a visual desktop interaction result. Source inspection confirmed the two toolbar actions route to the export adapters and report success/failure; the required automated and behavioral evidence was available, so this unavailable optional smoke path is not a hard stop.
- **Repository comparison — Severity: warning.** The branch remains unborn and uncommitted, so no commit diff is available. The WO changed-file record, current source, execution log, tests, and fresh probes provide the comparison basis.

### Notes

- [`export_pdf()`](../../../crates/scriptcraft-core/src/exports.rs:68) builds a fresh [`ScreenplayLayout`](../../../crates/scriptcraft-core/src/layout.rs:51), emits a deterministic PDF 1.4 header, catalog/pages/font/page/content objects, xref, trailer, `startxref`, and EOF marker. The probe repeated the export byte-for-byte, verified the declared two-page `/Count`, two page objects, and that `startxref` points at the xref table.
- The PDF probe used a title-page plus screenplay elements and confirmed fixed-layout output contained the representative title and scene heading. It also verified deterministic output and basic structural validity without relying only on the implementation's unit test.
- [`export_text()`](../../../crates/scriptcraft-core/src/exports.rs:47) reports one lost role and stable ID per title/screenplay element, sets `page_layout_lost`, includes the explicit `LOSSY EXPORT` contract, and states the fixed-layout page count. The independent probe verified the three-element/ two-page case and repeated text contract.
- Both adapters borrow [`Project`](../../../crates/scriptcraft-core/src/lib.rs:171) and do not receive a mutable session. A dirty session remained equal to its pre-export clone, retained `dirty() == true`, and preserved its revision/project state after successful PDF and TXT exports.
- With an invalid page profile, both exports returned errors and a clean session's project and `dirty() == false` state were unchanged. This independently verifies failure isolation.
- [`ScriptCraftApp::export_pdf()`](../../../apps/scriptcraft/src/main.rs:101) and [`ScriptCraftApp::export_text()`](../../../apps/scriptcraft/src/main.rs:114) write only `scriptcraft.pdf`/`scriptcraft.txt`, clear errors only after successful output, and do not call [`EditingSession::mark_saved()`](../../../crates/scriptcraft-core/src/editing.rs:109). The toolbar exposes `Export PDF` and `Export TXT (lossy)`.

## Independent evidence

- Standalone `/tmp/wo007_probe.rs` linked against the built core library passed:
  deterministic repeated PDF bytes; PDF 1.4 signature and EOF; xref/startxref
  consistency; two-page `/Count` and page-object count; representative fixed
  layout content; TXT role/ID/page-layout loss contract and page count; dirty
  state unchanged after successful exports; and error/session isolation for an
  invalid page profile.
- `cargo fmt --all -- --check`: passed.
- `cargo check --workspace`: passed.
- `cargo build --workspace`: passed.
- `cargo test --workspace -- --nocapture`: passed, 2 app tests, 22 core tests,
  and doc-test targets with zero failures.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.

## Decision

- **Outcome:** `pass`
- **Required next state:** [`WO-007-exports-integration.md`](../work-orders/WO-007-exports-integration.md) → `complete`
- **Fix work order required:** `no`


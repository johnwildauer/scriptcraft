# Validation: WO-008 — End-to-End Hardening

## Validation metadata

- **Review type:** `validation`
- **Status:** `complete`
- **Validator:** Independent fresh-context validation agent
- **Validator session/agent ID:** `validation-session-2026-10-25-wo-008`
- **Session exception:** `none`
- **Date:** 2026-10-25
- **Initiative:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Validation file:** `validation-WO-008.md`
- **Validated work order:** [`WO-008-e2e-hardening.md`](../work-orders/WO-008-e2e-hardening.md)
- **Compared commit/diff:** No commit exists; the initiative branch is unborn and fully uncommitted. Comparison used the WO changed-file record, execution log, all prior validation artifacts, requirements, blueprints, current E2E source, and independent command results. No product source was edited.
- **Fix chain:** `none`

## Authorized scope and comparison basis

The validation process at [`05-validation.md`](../../../process/05-validation.md)
requires an independent fresh-context implementation-alignment and anti-rogue
check. I read that process, the WO-008 scope and implementation evidence, the
initiative requirements/context, the execution log, all prior validation
artifacts, and the relevant core, desktop, and writing-loop blueprints.

WO-008 authorizes one bounded headless representative scene loop, hostile-input
and failure-safety regression coverage, full quality gates, bounded desktop
smoke evidence, and cleanup of the accidental root probe. It does not authorize
new product scope or changes to prior contracts. The inspected E2E test is
[`wo008_e2e.rs`](../../../crates/scriptcraft-core/tests/wo008_e2e.rs); its three
tests exercise the same core boundaries used by the UI: semantic editing and
transitions, Unicode text, undo/redo, layout, native persistence/reopen,
recovery discovery, PDF/TXT export, dirty-state preservation, atomic-save
failure preservation, and bounded hostile input.

## Diagnostic hypothesis log

Possible failure sources considered before narrowing were:

1. The E2E fixture could omit a required seam or use a path different from the UI.
2. Role transitions, Unicode editing, or undo/redo could lose semantic state.
3. Repeated layout or export could be nondeterministic or mutate session state.
4. Save/reopen or recovery could lose semantic content or silently replace state.
5. Injected atomic replacement failure could overwrite valid bytes or clear dirty state.
6. Hostile positions, oversized input, or malformed native data could panic or corrupt state.
7. The full workspace quality gates or bounded desktop launch could fail independently.

The two highest-risk sources were (a) transactional state preservation across
the composed persistence/export/failure path and (b) bounded hostile-input
handling without panic. The independent E2E run, full test suite, and source
inspection narrowed both risks successfully: the failure-injection test kept
the prior destination and dirty session unchanged; the hostile sequence
repeated 64 bounded invalid operations and preserved the session after each
rejected operation. No fix was indicated, so there was no implementation to
apply and no confirmation-before-fix request was applicable.

## Validation checklist

- Reported implementation, changed-file record, execution evidence, and current
  test source support meaningful comparison: **pass**.
- Changed files and behavior are within WO-008's bounded E2E/hardening scope:
  **pass**. No later feature, excluded format, or prior contract redesign was
  found.
- No clearly deceptive, backdoor, malicious, nefarious, or otherwise rogue
  behavior was found: **pass**. The test is deterministic, in-memory where
  appropriate, and contains no network, hidden process, or unrelated product
  behavior.
- Explicit acceptance criteria and required project quality gates are addressed:
  **pass**. Headless E2E, failure injection, hostile-input coverage, format,
  check, build, test, and clippy all passed.
- Durable-document impact is not contradicted: **pass**. Requirements and
  blueprints remain unchanged; this WO adds regression evidence only.
- Temporary root probe cleanup is verified: **pass**. A root-level scan before
  and after the smoke attempt found no `*probe*`, `wo008*`, or `*tmp*` file;
  [`o007_probe.rs`](../../../o007_probe.rs) is absent.

## Independent acceptance evidence

All commands below were run independently in this fresh validation session.

| Check | Result | Evidence |
|---|---|---|
| `rustc -vV` | PASS | Rust `1.99.0`, host `aarch64-apple-darwin`. |
| `cargo test -p scriptcraft-core --test wo008_e2e -- --nocapture` | PASS | 3 WO-008 integration tests passed; 0 failed. |
| `cargo fmt --all -- --check` | PASS | Exit code 0. |
| `cargo check --workspace` | PASS | Exit code 0. |
| `cargo build --workspace` | PASS | Exit code 0. |
| `cargo test --workspace -- --nocapture` | PASS | 2 app tests, 22 core unit tests, 3 WO-008 integration tests, and doc-test target with 0 failures. |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | Exit code 0. |
| Root temporary-probe scan | PASS | No matching temporary root probe artifacts before or after testing/smoke. |
| `cargo run -p scriptcraft` bounded smoke | ATTEMPTED / LIMITED | Built and reached `target/debug/scriptcraft`; the terminal-only 5-second bound expired. No visible window or manual interaction could be observed, so visual/manual success is not claimed. |

### E2E and hostile-input evidence

- The representative test passed create/project initialization, scene-heading →
  action and character → dialogue transitions, Unicode insertion, undo/redo,
  repeated equal layout output, native save/reopen, explicit recovery candidate
  discovery, PDF/TXT export, and dirty-state preservation after export.
- The hostile test performed 64 bounded invalid-operation iterations covering an
  interior UTF-8 offset, an out-of-range element index, oversized insertion,
  and malformed native JSON. Rejected session operations returned errors and
  preserved the full session snapshot; the final semantic text remained
  `é🙂`. The run completed without a production panic.
- The injected rename-failure test preserved the old destination bytes, left
  the edited session unchanged, and retained `dirty() == true`.

## Findings

### Blockers

None. No hard-stop implementation finding, required-gate failure, scope
violation, or rogue behavior was identified. No lettered fix WO is required.

### Warnings

- **Manual visible-window smoke unavailable — severity: warning.** The bounded
  `cargo run -p scriptcraft` command built and started the native binary, but
  this terminal-only validation context exposed no window or UI interaction;
  the process was stopped by the five-second bound. This is recorded exactly
  as a limitation and is not claimed as a pass. The validation process makes an
  unavailable manual gate a hard stop only when project policy requires it; the
  available WO/project evidence does not establish such a required visual gate.
- **Uncommitted comparison evidence — severity: warning.** The branch has no
  commit, so commit-based diff comparison is unavailable. The changed-file
  record, direct source inspection, prior artifacts, and fresh command results
  provide a meaningful comparison basis.

## Decision

- **Outcome:** `pass`
- **Required next state:** [`WO-008-e2e-hardening.md`](../work-orders/WO-008-e2e-hardening.md) → `complete`
- **Fix work order required:** `no`

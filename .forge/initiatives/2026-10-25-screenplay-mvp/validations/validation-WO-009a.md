# Validation: WO-009a — Preserve Pointer Drag Selection Anchor

## Validation metadata

- **Review type:** `validation`
- **Status:** `pass`
- **Validator:** Independent fresh-context validator
- **Validator session/agent ID:** `validation-session-2026-10-25-wo-009a`
- **Session exception:** `none`
- **Date:** 2026-10-25
- **Initiative:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Validation file:** `validation-WO-009a.md`
- **Validated work order:** [`../work-orders/WO-009a-preserve-pointer-drag-selection-anchor.md`](../work-orders/WO-009a-preserve-pointer-drag-selection-anchor.md)
- **Compared commit/diff:** No committed baseline; comparison used the recorded changed-file record and current source.
- **Fix chain:** [`WO-009-unified-live-editing-surface.md`](../work-orders/WO-009-unified-live-editing-surface.md) → [`validation-WO-009.md`](validation-WO-009.md) → [`WO-009a-preserve-pointer-drag-selection-anchor.md`](../work-orders/WO-009a-preserve-pointer-drag-selection-anchor.md)

## Authorized scope and comparison basis

I read the validation process, the original failing validation, WO-009, WO-009a,
the initiative execution log, and the current app source and manifest. The
implementation comparison used the recorded fix changed-file evidence and
[`main.rs`](../../../apps/scriptcraft/src/main.rs). The fix is confined to
transient app-local pointer state and its focused regression test; no core,
persistence, export, WO-010, or unrelated product behavior was changed.

The explicit acceptance criterion assessed was that a pointer hit at drag start
remains the selection anchor through later `dragged()` frames while each focus
update is routed through `EditingSession::set_selection()`. The required Cargo
gates and the bounded native smoke were also run as recorded below.

## Validation checklist

- The implementation, changed-file record, prior validation, fix evidence, and
  current source support a meaningful comparison.
- The changed behavior is authorized by WO-009a and remains within the original
  live editing surface boundary.
- No deceptive, backdoor, malicious, nefarious, or otherwise rogue behavior was
  found.
- The explicit drag-anchor acceptance criterion and required quality gates are
  addressed.
- The transient app-state fix does not contradict the documented durable-
  document impact classification.

## Independent checks and evidence

### Pointer drag-start anchor

- **PASS.** [`ScriptCraftApp::apply_pointer_selection()`](../../../apps/scriptcraft/src/main.rs:147)
  stores the `drag_started` hit in `drag_anchor`, creates the initial caret,
  and uses the stored anchor for later `dragged` frames. Each resulting range
  is applied with `EditingSession::set_selection()`.
- **PASS.** The focused regression test
  [`pointer_drag_preserves_initial_hit_as_selection_anchor()`](../../../apps/scriptcraft/src/main.rs:540)
  independently executed a start at byte offset 1 followed by a later focus at
  byte offset 5 and observed the expected anchor/focus pair. Command result:
  `cargo test -p scriptcraft pointer_drag_preserves_initial_hit_as_selection_anchor -- --nocapture`;
  1 passed, 0 failed.
- **PASS.** The full app test run also executed all 5 app tests, including the
  drag regression, with zero failures. The non-drag path clears the transient
  anchor and preserves caret behavior.

### Required quality gates

- `cargo fmt --all -- --check` — PASS
- `cargo check --workspace` — PASS
- `cargo build --workspace` — PASS
- `cargo test --workspace -- --nocapture` — PASS: 5 app tests, 22 core unit
  tests, 3 WO-008 integration tests, and doc-test target with zero failures
- `cargo clippy --workspace --all-targets -- -D warnings` — PASS

### Bounded smoke

- `cargo run -p scriptcraft` reached `target/debug/scriptcraft` in the bounded
  terminal run. No visible-window or manual interaction result is claimed;
  this is the same headless limitation recorded by the prior validation and is
  not a required project-policy manual gate.

## Findings

### Blockers

- None. The prior WO-009 hard stop is directly covered by the stable
  `drag_anchor` path and focused regression test.

### Warnings

- **Headless visual smoke — Severity: warning.** The bounded native process was
  launched but terminal-only execution did not establish visual interaction.
  Recommendation: retain this limitation honestly; no fix work is required
  because project policy does not configure visible-window smoke as mandatory.
- **Repository comparison — Severity: warning.** The repository has no committed
  baseline, so comparison used current source, the changed-file record, tests,
  and execution evidence.

### Notes

- None.

## Decision

- **Outcome:** `pass`
- **Required next state:** WO-009a `complete`; original WO-009 `complete` through this passed fix chain.
- **Fix work order required:** `no`


# Validation: WO-001 — Seamless Native Titlebar

## Validation metadata

- **Review type:** `validation`
- **Status:** `complete — disposition recorded; outcome fail`
- **Validator:** `separate fresh-context validator`
- **Validator session/agent ID:** `not exposed by the execution interface`
- **Session exception:** `none`
- **Date:** `2026-10-28`
- **Initiative:** [`seamless-native-titlebar-v1.md`](../seamless-native-titlebar-v1.md:1)
- **Validation file:** `validation-WO-001.md`
- **Validated work order:** [`WO-001-seamless-native-titlebar.md`](../work-orders/WO-001-seamless-native-titlebar.md:1)
- **Compared commit/diff:** Current worktree comparison against `HEAD`: `git diff -- apps/scriptcraft/src/main.rs` (106-line diff; 91 insertions, 15 deletions). `git status --short --branch` showed branch `dev/2026-10-27-seamless-native-titlebar`, modified `apps/scriptcraft/src/main.rs`, and the initiative directory untracked. The recorded pre-edit base/HEAD is `65b1de6bc2ee2b87625a9ce543769e1920ee45fc`.
- **Fix chain:** `none`; WO-002 is an independently authorized successor and is not treated as a WO-001 fix.

## Authorized scope

Compared only WO-001's approved native titlebar configuration, bounded toolbar inset, native ownership/preservation constraints, explicit preservation behavior, required automated evidence, and mandatory six-item actual-window visual gate. The basis is the recorded changed-file inventory and the current source diff, alongside the exact user observation and implementation evidence in the work order and execution log. No application code was changed. Recorded implementation checks were examined as existing evidence only; this validator did not rerun tests, builds, or the app and did not independently inspect the window.

## Criteria assessment

| WO-001 criterion | Result | Evidence and limit |
|---|---|---|
| Authorized single decorated/opaque native viewport tuple; native mode handoff | **Aligned in source; recorded tests passed** | The diff in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:2602) selects decorations on, whole-window transparency off, title hidden, transparent titlebar, full-size content, and buttons shown. The work order/log record the focused tuple/handoff test passed. No visual effect is inferred from option values. |
| One bounded native-only toolbar inset and preserved full-width surface/geometry | **Aligned in source; recorded focused test passed** | The diff adds a 28-point native-only inset and a focused geometry test. The work order/log report the test passed. This does not establish actual clearance from native controls. |
| Native ownership; no app-owned title strip, maximize/restore, custom double-click, or transition policy on the selected native path | **Aligned in implementation** | The source conditionally renders the app-owned title response only for the custom mode; native mode uses the native options path. This confirms ownership intent, not successful native action behavior. |
| Retain working native OS titlebar double-click behavior (WO-001 native ownership/preservation constraint) | **FAIL — known defect** | The exact user observation recorded in the work order and execution log is “double click functionality does not work, everything else looks good”. This is direct evidence that the required behavior does not work. It is not expanded into a causal diagnosis. The absence of a custom app handler in native mode does not override the observed failure. |
| Visual 1: no native `ScriptCraft` title text beside traffic lights | **Not established** | The broad statement “everything else looks good” does not itemize this criterion. The hidden-title option is not visual evidence. |
| Visual 2: continuous titlebar/content surface, no distinct band or seam | **Not established** | The user did not itemize this criterion; source fill/configuration cannot establish macOS rendering. |
| Visual 3: native traffic lights visible and usable | **Not established** | No criterion-specific confirmation was supplied. The requested button option is not proof of visibility/usability. |
| Visual 4: toolbar legible/usable and clear of native controls in restored and maximized states | **Not established** | No state-specific confirmation was supplied. The 28-point deterministic geometry test does not establish native hit-region clearance. |
| Visual 5: maximize completes without swelling, shrinking, or final snap | **Not established for this implementation** | The statement does not itemize the maximize animation. V4 predecessor evidence is a preservation baseline, not evidence for this changed implementation. |
| Visual 6: restore completes without swelling, shrinking, or final snap | **Not established for this implementation** | The statement does not itemize the restore animation. V4 predecessor evidence is not substituted for the required post-change observation. |
| Required clean Cargo gates and deterministic/preservation tests | **Recorded pass; not rerun by validator** | The implementer recorded `cargo clean`, `cargo fmt --all -- --check`, `cargo test -p scriptcraft native_`, `cargo check --workspace`, `cargo test --workspace`, `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` as passing, plus `cargo run -p scriptcraft` launched. These remain implementer-attributed results, not validator reruns. |
| Changed files, comparison, scope, rogue behavior | **Aligned; no rogue behavior found** | The execution record identifies `apps/scriptcraft/src/main.rs` as the only implementation file changed; the inspected diff contains the native options/inset/test changes. Forge documentation changes are the recorded WO/log evidence. No unrelated, deceptive, backdoor, malicious, or nefarious behavior was found in the compared source diff. |
| Conditional requirements/blueprint updates | **Correctly unchanged pending full gate** | The work order makes those updates conditional on the complete approved visual gate. That gate is not established, so unchanged durable documents do not contradict the approved condition. |

## Validation checklist

- The recorded implementation, changed-file record, test/build evidence, and current source diff support meaningful comparison.
- The source changes are within WO-001 scope; no unrelated or rogue behavior was found.
- The recorded automated checks are attributed to the implementer; no checks are represented as rerun here.
- The reported native double-click failure is a genuine unmet preservation criterion. The mandatory visual gate is also incomplete because the six required observations were not individually confirmed.
- Conditional durable-document impact is consistent with leaving the requirements and blueprint unchanged while visual adoption remains unverified.

## Findings

### Blockers

- **Native double-click does not work — Severity: blocker.** Evidence: exact user observation, “double click functionality does not work, everything else looks good”, recorded in [`WO-001`](../work-orders/WO-001-seamless-native-titlebar.md:154) and [`execution-log.md`](../execution-log.md:82); WO-001 requires retaining native double-click behavior. Recommendation: retain this as a failed criterion; do not infer a cause or claim success. Required next state: `blocked`.
- **Required visual gate is not fully evidenced — Severity: blocker.** Evidence: the same broad user observation is not itemized against the six separate criteria; the work order explicitly requires all six and disallows substituting automated/source evidence. Recommendation: keep each unspecified visual result unverified; do not mark WO-001 complete. Required next state: `blocked`.

### Warnings

- The implementer-reported Cargo/build/test results were not independently rerun during this validation. They are preserved as recorded evidence, not validator results. Required next state: `blocked` due to the blockers above.

### Notes

- No implementation fix, additional validation run, runtime visual check, dispatch, stage, commit, or merge was performed.
- No WO-001a or other duplicate executable order was created. The user-authorized WO-002 remains a separate successor and is not deemed a WO-001 fix or validation result.
- Requirements and blueprint remain unchanged because the complete visual gate did not pass.

## Decision

- **Outcome:** `fail`
- **Required next state:** `blocked` — WO-001 is not complete; record its failed disposition and retain the unresolved visual gate.
- **Fix work order required:** `no new order created in this validation` — the user explicitly prohibited duplicate executable orders; WO-002 remains separate and is not represented as a WO-001 fix chain.

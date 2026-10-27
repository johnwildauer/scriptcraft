# Validation: WO-002 — Toolbar Item Width Tuning

## Validation metadata

- **Validation type:** Fresh-context implementation alignment
- **Status:** `fail`
- **Validator session/agent ID:** `fresh-validator-toolbar-interaction-polish-2026-10-27-04`
- **Date:** 2026-10-27
- **Initiative:** [`toolbar-interaction-polish-v1.md`](../toolbar-interaction-polish-v1.md:1)
- **Work order:** [`WO-002-toolbar-item-width-tuning.md`](../work-orders/WO-002-toolbar-item-width-tuning.md:1)
- **Implementation session:** `code-implementer-toolbar-interaction-polish-2026-10-27-03`
- **Session separation exception:** none
- **WO-001 evidence consulted:** [`validation-WO-001.md`](validation-WO-001.md:1), the WO-001 implementation/completion record in [`execution-log.md`](../execution-log.md:74), and the approved dependency/order context in [`toolbar-interaction-polish-v1.md`](../toolbar-interaction-polish-v1.md:83)

## Comparison basis and validation boundary

The comparison used [`00-overview.md`](../../../process/00-overview.md:1), [`05-validation.md`](../../../process/05-validation.md:1), [`project-manifest.md`](../../../project-manifest.md:1), the initiative plan, design review, WO-002, WO-001 validation evidence, the WO-002 implementation evidence and execution log, the current branch diff, focused source tests, and the required Cargo/diff gates.

This validation is limited to the narrowed normal Item width, the named manual controls `TOOLBAR_NORMAL_ITEM_WIDTH`, `TOOLBAR_ROW_PADDING`, and `TOOLBAR_ITEM_GAP`, allocator/renderer consistency, right-edge bounds, Flexible Space equality, normal hit/dispatch behavior as preserved by the existing renderer, narrow overflow, focused tests, authorized requirements/blueprint impact, and required gates. WO-001's already-validated icon grouping is treated as dependency evidence and is not revalidated here. No production code was modified by this validation.

## Diagnostic hypotheses considered

The following possible failure sources were considered:

1. The normal width may not have been narrowed consistently between the constant, allocator, and renderer.
2. Row padding or inter-Item gap may be duplicated or omitted between allocation arithmetic and rendering.
3. Flexible Space may no longer divide the remaining width equally.
4. The rightmost Item or overflow menu may exceed the available row bounds at wide or narrow widths.
5. Narrow allocation may retain the wrong Items or expose spacing-only entries through overflow.
6. Normal icon hit/dispatch behavior may have been changed while tuning the outer width.
7. The focused regression tests or durable-document updates may be stale, overbroad, or fail the project policy gates.

The two most likely sources after source inspection were (a) stale deterministic expected geometry after changing the normal width and (b) a newly added constant assertion that is incompatible with the required warnings-as-errors Clippy gate. Fresh test, build, Clippy, format, and diff commands were used to validate those assumptions rather than inferring from the implementation report.

## Evidence and findings by severity

### Blockers

- **Required test gate fails:** `cargo test --workspace` failed in [`toolbar_allocation_wide_and_exact_fit_are_deterministic()`](../../../../apps/scriptcraft/src/main.rs:1492). At [`main.rs:1497`](../../../../apps/scriptcraft/src/main.rs:1497), the test expected the first Flexible Space width to be `214.0`, but the implementation produced `166.0` after the normal width was changed to `72.0`. The focused test set otherwise reported 48 passed and 1 failed. This is a genuine required-gate failure and leaves the implementation/test evidence inconsistent. **Recommendation:** update the stale deterministic expectation only if it reflects the approved 72-point geometry, then rerun the full required gates. **Required next state:** `execute` through a scoped WO-002 fix work order, followed by fresh validation.

- **Required Clippy gate fails:** `cargo clippy --workspace --all-targets -- -D warnings` failed at [`main.rs:1542`](../../../../apps/scriptcraft/src/main.rs:1542) because `assert!(TOOLBAR_NORMAL_ITEM_WIDTH < 88.0)` triggers `clippy::assertions-on-constants` under `-D warnings`. **Recommendation:** replace the redundant constant assertion with a gate-compatible deterministic check or remove it while retaining the exact-value control test. **Required next state:** the same scoped fix/validation loop; do not treat the implementation as complete until Clippy passes.

### Warnings

- **Formatter gate fails:** `cargo fmt --all -- --check` reported an unformatted multiline assertion at [`main.rs:1603`](../../../../apps/scriptcraft/src/main.rs:1603). This is a required gate failure rather than a native-visual issue, but no production source was changed during validation. The implementer/fix session must format the authorized source and rerun the gate.

- **Native visual smoke unavailable:** No native visual session was available to confirm pixel-level Item tightness, visible right-edge placement, or pointer interaction in a running window. The manifest has no required manual/external gate, so this remains a warning and native visual success is not claimed.

### Notes

- **Narrowed controls and source alignment:** [`TOOLBAR_NORMAL_ITEM_WIDTH`](../../../../apps/scriptcraft/src/main.rs:21) is `72.0`; [`TOOLBAR_ROW_PADDING`](../../../../apps/scriptcraft/src/main.rs:23) and [`TOOLBAR_ITEM_GAP`](../../../../apps/scriptcraft/src/main.rs:24) are both `4.0`. [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:177) and [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:519) use those named controls, and [`toolbar_content_width()`](../../../../apps/scriptcraft/src/main.rs:166) applies the same row budget.

- **Allocation behavior:** The focused tests cover normal width, total bounds, wide rightmost fit, Flexible Space equality, and narrow leftmost-fit/overflow behavior at [`main.rs:1524`](../../../../apps/scriptcraft/src/main.rs:1524), [`main.rs:1560`](../../../../apps/scriptcraft/src/main.rs:1560), [`main.rs:1571`](../../../../apps/scriptcraft/src/main.rs:1571), and [`main.rs:1598`](../../../../apps/scriptcraft/src/main.rs:1598). Those focused assertions passed during the fresh test run except for the stale wide/exact-fit expectation identified above.

- **Normal hit behavior and overflow routing:** The width-only renderer path still invokes [`toolbar_action_button()`](../../../../apps/scriptcraft/src/main.rs:424) for normal action Items, while [`toolbar_overflow_menu()`](../../../../apps/scriptcraft/src/main.rs:481) retains action and Elements-role dispatch. The existing focused routing test passed. This source evidence does not constitute native pointer-smoke success.

- **Durable-document impact:** The changed requirements and desktop blueprint wording is authorized by WO-002 and limited to the 72-point normal width plus named 4-point padding/gap controls and deterministic allocation language. No core, persistence, export, page-layout, role-policy, or unrelated UI durable impact was observed in the reviewed diff.

- **Diff scope:** The comparable diff includes the authorized app source, requirements, blueprint, work-order, and initiative evidence surface. `git diff --check` passed. No rogue, deceptive, backdoor, malicious, or unrelated production behavior was observed within this validation boundary.

## Required Cargo and diff gates

Fresh validation command sequence was run on 2026-10-27 from the repository root. Results:

| Gate | Result | Evidence |
|---|---|---|
| `cargo fmt --all -- --check` | **FAIL** | One formatting diff at [`main.rs:1603`](../../../../apps/scriptcraft/src/main.rs:1603) |
| `cargo check --workspace` | PASS | Exit code 0 |
| `cargo test --workspace` | **FAIL** | 48 passed, 1 failed; stale Flexible Space expectation at [`main.rs:1497`](../../../../apps/scriptcraft/src/main.rs:1497) |
| `cargo build --workspace` | PASS | Exit code 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | **FAIL** | Constant assertion lint at [`main.rs:1542`](../../../../apps/scriptcraft/src/main.rs:1542) |
| `git diff --check` | PASS | Exit code 0 |

The command output was captured by the fresh validation session; no native visual smoke command was available or claimed.

## Result

**FAIL.** The implementation is directionally aligned with the authorized narrowed-width and geometry scope, and source inspection found the named controls, shared allocator/renderer assumptions, overflow behavior, Flexible Space logic, and authorized durable-document impact. However, two required quality gates fail (`cargo test` and Clippy), and the required format check also fails. These are concrete in-scope completion blockers, not design preferences. Native visual success is not claimed.

## Required next state

- Keep [`WO-002-toolbar-item-width-tuning.md`](../work-orders/WO-002-toolbar-item-width-tuning.md:1) `in-review`; this validation does not mark it complete.
- Create a letter-suffixed WO-002 fix work order scoped to the stale expected geometry, the Clippy-incompatible constant assertion, and formatting, then dispatch a new implementer session.
- Rerun all six required gates in a fresh validation session after the fix; require `cargo fmt`, `cargo test`, `cargo clippy`, and the other gates to pass before completion.
- Record this validation and its blocker findings in [`execution-log.md`](../execution-log.md:1). The orchestrator, not this validation report, owns the WO-002 status transition.
- Preserve the native visual limitation as a warning and do not claim native visual success unless an actual native smoke result becomes available.

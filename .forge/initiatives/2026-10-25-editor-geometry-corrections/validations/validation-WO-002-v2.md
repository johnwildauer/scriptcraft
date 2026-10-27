# Validation — WO-002 Continuous Caret Blink (v2)

## Result

- **Validation status:** `FAIL`
- **Validated work order:** [`WO-002-continuous-caret-blink.md`](../work-orders/WO-002-continuous-caret-blink.md:1)
- **Initiative:** [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:1)
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `architect-fresh-context-editor-geometry-corrections-v2-WO-002-validation-2026-10-26`
- **Session exception:** `none`
- **Work-order state at validation:** `in-review`; this validation did not mark WO-002 complete.
- **Fix work order:** `required` for the focus-loss/focus-regain reset defect identified below.

## Fresh-context statement and comparison basis

I independently read the validation rules in [`05-validation.md`](../../../process/05-validation.md:1), the authoritative v2 plan in [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:1), the reconciled design review in [`design-review-v2.md`](../reviews/design-review-v2.md:146), the WO-002 handoff and implementation evidence, the v2 execution log, the applicable writing-loop requirement, the desktop blueprint, the current [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1), and the current rendered tests. I did not modify product code, silently repair artifacts, or validate WO-003 through WO-006.

The comparison is meaningful. The work order is in review, the implementation evidence names the changed product file and rendered tests, the execution log records a matching WO-002 dispatch/completion sequence, and the current source contains the reported implementation. Available evidence is a work-order/file-level comparison rather than a direct branch diff; no product diff command result was available in this inspection context. That is an evidence warning, not a comparison blocker.

The assessed boundary is limited to WO-002: focused collapsed-caret 500 ms alternation, egui-owned repaint scheduling, focus/selection suppression, caret/revision reset semantics, app-only scope, requirements/blueprint classification, and the required Cargo gates. Geometry redesign, font work, pointer placement, casing, parenthetical behavior, performance caching, and native visual success were not assessed.

## Oracle and evidence assessment

| WO-002 obligation | Result | Evidence |
|---|---|---|
| Focused collapsed-caret phase alternates every 500 ms | **PASS** | [`caret_blink_visible()`](../../../../apps/scriptcraft/src/main.rs:861) derives a discrete phase from egui input time and uses [`CARET_BLINK_INTERVAL`](../../../../apps/scriptcraft/src/main.rs:19) of 500 ms. The rendered test [`rendered_caret_blink_alternates_on_controlled_egui_time()`](../../../../apps/scriptcraft/src/main.rs:1699) checks visible at 0/499 ms, hidden at 500/999 ms, and visible at 1000 ms. |
| Blink state is transient and not stored in the model/session | **PASS** | The phase is held by egui animation state through [`animate_bool_with_time()`](../../../../apps/scriptcraft/src/main.rs:879). No timer thread or model blink field was added. |
| Focused collapsed caret schedules the next repaint | **PASS** | [`caret_blink_visible()`](../../../../apps/scriptcraft/src/main.rs:861) calls [`request_repaint_after()`](../../../../apps/scriptcraft/src/main.rs:880) only after the focused/collapsed guard succeeds, with the delay computed by [`caret_blink_repaint_delay()`](../../../../apps/scriptcraft/src/main.rs:884). |
| Unfocused and range-selected states suppress caret painting and blink scheduling | **PASS by source; test evidence incomplete** | The early return at [`caret_blink_visible()`](../../../../apps/scriptcraft/src/main.rs:867) suppresses both painting and scheduling when `focused` or `collapsed` is false. However, [`rendered_caret_blink_schedules_only_focused_collapsed_phases()`](../../../../apps/scriptcraft/src/main.rs:1724) checks hidden visibility and a helper delay for the suppressed cases, but does not assert that the unfocused/range-selected frame output contains no repaint request. |
| Revision and caret identity changes reset the transient phase | **PASS** | The production [`blink_id`](../../../../apps/scriptcraft/src/main.rs:374) includes the editor ID, session revision, element, and offset. The rendered test [`rendered_caret_blink_identity_change_uses_a_distinct_entry()`](../../../../apps/scriptcraft/src/main.rs:1758) observes distinct animation entries for changed IDs. |
| Focus loss resets the blink and focus regain starts the approved reset phase | **FAIL** | The approved oracle explicitly requires focus loss to suppress/reset blink. The production ID at [`paint_page()`](../../../../apps/scriptcraft/src/main.rs:374) contains revision/caret identity but no focus-generation/reset identity. While unfocused, [`caret_blink_visible()`](../../../../apps/scriptcraft/src/main.rs:867) returns without changing the egui animation entry. On focus regain, the same ID and absolute egui time are reused, so the caret resumes the time-derived phase rather than demonstrably restarting from the reset phase. No rendered test exercises focused → unfocused → focused transition or asserts the reset visibility. |
| No unconditional repaint loop was introduced | **PASS** | The WO-002 change uses conditional [`request_repaint_after()`](../../../../apps/scriptcraft/src/main.rs:880) for the caret path. The existing input-dispatch repaint path at [`handle_surface_input()`](../../../../apps/scriptcraft/src/main.rs:630) is outside the blink-specific change and is not being treated as an idle blink loop. |

## Hard-stop finding

### EGC-V2-WO-002-F-001 — Focus loss does not reset the blink phase on focus regain

- **Severity:** hard stop; required criterion is unmet.
- **Evidence:** The approved plan requires that focus loss suppress/reset blink at [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:77), and the WO-002 observable oracle repeats that requirement at [`WO-002-continuous-caret-blink.md`](../work-orders/WO-002-continuous-caret-blink.md:48). The production animation identity is built from editor/revision/caret values at [`paint_page()`](../../../../apps/scriptcraft/src/main.rs:374); focus state is passed only as the boolean `blink_active` at [`paint_page()`](../../../../apps/scriptcraft/src/main.rs:381). The helper returns immediately for an unfocused frame at [`caret_blink_visible()`](../../../../apps/scriptcraft/src/main.rs:867), without resetting or replacing the animation identity. The rendered tests cover alternation, focused scheduling, suppression, and identity changes, but do not cover focus loss followed by focus regain at a controlled phase.
- **Impact:** A focused caret can regain focus in the phase determined by the prior absolute egui time/animation entry rather than the approved predictable reset phase. This is a direct mismatch with the WO-002 handoff oracle, not a design preference or optional native-evidence limitation.
- **Required correction:** Add a narrowly scoped focus-generation/reset boundary to the existing transient animation identity or otherwise reset the egui animation entry when focus is lost, then add a controlled rendered regression covering focused → unfocused → focused behavior and the expected reset phase/visibility. Preserve the 500 ms interval, collapsed-only gating, no model mutation, and no unconditional repaint loop. Re-run a fresh validation after the fix; do not broaden the work into geometry, font, pointer, or other v2 WOs.
- **Required next state:** Keep WO-002 `in-review`; create and dispatch a letter-suffixed fix work order from a new implementer session, then obtain a fresh validation artifact.

## Scope, rogue behavior, and durable-document classification

- **Scope:** **PASS.** The reported product change is confined to the existing app paint path and its focused rendered tests. The implementation evidence explicitly says WO-003 through WO-006 were not implemented. I found no second editor, hidden buffer, timer thread, model blink state, caret geometry redesign, native-font work, or unrelated product area in the inspected source/evidence.
- **Rogue behavior:** **PASS.** No deceptive, backdoor, malicious, nefarious, or otherwise rogue behavior was observed.
- **Requirements classification:** **PASS.** WO-002 marks requirements `verify unchanged`; the implementation is a transient presentation/repaint correction and does not contradict [`AC-WRITING-002.4`](../../../requirements/features/screenplay-writing-loop.md:61) or [`NFR-WRITING-002`](../../../requirements/features/screenplay-writing-loop.md:61).
- **Blueprint classification:** **PASS.** The authorized desktop blueprint update at [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:54) records the egui-only 500 ms contract, focused collapsed gating, identity boundary, suppression behavior, and absence of timer/model state. The focus-reset defect means the documented contract currently describes intended behavior more completely than the implementation; the blueprint must not be treated as proof that the missing reset is implemented.

## Required Cargo gates and manual boundary

The implementation evidence and execution log record the following required gates as passing for the v2 WO-002 handoff:

- [`cargo fmt --all -- --check`](../../../../Cargo.toml:1) — recorded pass.
- [`cargo check --workspace`](../../../../Cargo.toml:1) — recorded pass.
- [`cargo test --workspace`](../../../../Cargo.toml:1) — recorded pass; the v2 execution log records the preceding WO-001 suite at 76 tests plus 0 doc tests, while the WO-002 implementation evidence records the focused rendered tests. An exact post-WO-002 total is not stated in the v2 execution-log entry.
- [`cargo clippy --workspace --all-targets -- -D warnings`](../../../../Cargo.toml:1) — recorded pass.
- [`cargo build --workspace`](../../../../Cargo.toml:1) — recorded pass.
- [`git diff --check`](../../../../Cargo.toml:1) — recorded pass for the implementation handoff.

These are **recorded implementation results**, not commands independently executed by this inspection context. The available tools did not expose command execution. Preserve that evidence boundary; do not represent the recorded results as independently rerun. The project policy configures no required native/manual hard-stop gate at [`project-manifest.md`](../../../project-manifest.md:88). Native window inspection was not performed and remains optional/unavailable; it neither passes nor creates this failure.

## Verdict and exact next state

**FAIL.** The implementation is substantially aligned with WO-002's focused 500 ms egui blink, conditional repaint, transient state, identity-based revision/caret reset, scope, and durable-document boundary. It nevertheless fails the explicit focus-loss/reset oracle because focus loss suppresses the helper without resetting the reused animation identity, and the rendered suite does not establish focus-regain reset behavior. The recorded Cargo gates and unavailable native smoke do not cause the failure.

The orchestrator must keep [`WO-002-continuous-caret-blink.md`](../work-orders/WO-002-continuous-caret-blink.md:1) `in-review`, create a narrowly scoped letter-suffixed fix work order for focus-loss/focus-regain reset, dispatch a new implementer session, and then dispatch a separate fresh validator. Do not mark WO-002 `complete`, do not create fixes for optional native evidence, and do not dispatch or validate later v2 work orders from this result.


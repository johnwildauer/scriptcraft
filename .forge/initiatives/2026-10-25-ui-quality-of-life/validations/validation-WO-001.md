# Validation — WO-001 Centered Page Viewport

## Validation metadata

- **Validation type:** `implementation alignment and anti-rogue validation`
- **Status:** `PASS WITH WARNINGS`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `debug-fresh-context-ui-quality-of-life-WO-001-validation-2026-10-25T21:09:28Z`
- **Date:** 2026-10-25
- **Work order:** [`WO-001-centered-page-viewport.md`](../work-orders/WO-001-centered-page-viewport.md:1)
- **Initiative:** [`ui-quality-of-life-v1.md`](../ui-quality-of-life-v1.md:1)
- **Fresh-context statement:** This validation was performed as a separate fresh-context validator session. I did not implement WO-001, did not rely on implementer conversational memory, did not validate WO-002, WO-003, or WO-004, and did not mark WO-001 complete.

## Scope and comparison basis

This validation covers only the authorized centered/readable page viewport boundary: the 1.6x default presentation target, responsive centering and narrow-window behavior, preservation of fixed layout coordinates and pointer hit testing, caret-following scroll, the work order's checks/manual evidence, and the durable-document classification. It does not assess role transitions, visual-line navigation, long-document validity, or screenplay spacing.

Inspected:

- Validation rules in [`05-validation.md`](../../../process/05-validation.md:1) and lifecycle/scope rules in [`00-overview.md`](../../../process/00-overview.md:1).
- Initiative plan in [`ui-quality-of-life-v1.md`](../ui-quality-of-life-v1.md:1), design review in [`design-review.md`](../reviews/design-review.md:1), and the authorized work order in [`WO-001-centered-page-viewport.md`](../work-orders/WO-001-centered-page-viewport.md:1).
- Implementer evidence and execution events in [`execution-log.md`](../execution-log.md:1).
- Comparable working-tree diff: implementation changes are confined to [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1). The unrelated modified Forge baseline files were present in the working tree but were not attributed to the WO-001 implementation session.

## Findings

### W-001 — Native/manual interaction smoke was not completed

- **Severity:** warning
- **Evidence:** The recorded automated checks passed, and [`cargo run -p scriptcraft`](../work-orders/WO-001-centered-page-viewport.md:59) was attempted in this validation. The binary reached `Running target/debug/scriptcraft` and continued running, but this tool session provided no native-window interaction channel; the process was then terminated by the validator. Consequently, I did not directly observe normal/maximized/narrow resize behavior, multi-page caret reveal, or live pointer selection in a native window.
- **Recommendation:** Preserve this as an honest manual-evidence gap and obtain the focused native smoke through a user/operator with access to the window, if desired before closeout. Do not convert the launch attempt into a claim that the visual behaviors passed.
- **Required next state:** Note/follow-up only; no fix work order is required by project policy because [`project-manifest.md`](../../../project-manifest.md:88) configures no manual/external hard-stop gate. WO-001 remains `in-review` until the orchestrator applies the validation outcome.

### N-001 — Implementation alignment is supported by code and headless coverage

- **Severity:** note
- **Evidence:** [`presentation_geometry()`](../../../../apps/scriptcraft/src/main.rs:712) computes `360 × 1.6 = 576` CSS pixels for the default 612-unit page, clamps available width with 16-pixel side padding and a 240-pixel minimum, and centers the resulting page row. [`paint_page()`](../../../../apps/scriptcraft/src/main.rs:274) continues to allocate and paint fixed layout dimensions, while [`caret_from_pointer()`](../../../../apps/scriptcraft/src/main.rs:206) converts pointer coordinates through the rendered page rect and scale before calling the existing layout hit test. The caret reveal path uses [`Ui::scroll_to_rect()`](../../../../apps/scriptcraft/src/main.rs:427) on the computed caret rectangle, and the reveal flag is set after successful mutation or selection changes in [`mutate()`](../../../../apps/scriptcraft/src/main.rs:180) and [`apply_pointer_selection()`](../../../../apps/scriptcraft/src/main.rs:224).
- **Recommendation:** None for this validation. The focused regression [`presentation_geometry_scales_default_page_and_centers_it()`](../../../../apps/scriptcraft/src/main.rs:919) covers wide and narrow geometry; the existing rendered-surface input test was adapted to centered/scaled coordinates.
- **Required next state:** Record as aligned evidence; no implementation change required.

### N-002 — No rogue or unrelated WO-001 implementation behavior observed

- **Severity:** note
- **Evidence:** The comparable diff changes only presentation constants/state, viewport placement/reveal behavior, and app-level regression-test coordinates in [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:12). No core layout file, native project schema, persistence path, or WO-002/003/004 artifact was changed by the implementer session. No deceptive, backdoor, malicious, or nefarious behavior was observed.
- **Recommendation:** None.
- **Required next state:** Accept the implementation scope as aligned.

### N-003 — Durable-document classification is accurate

- **Severity:** note
- **Evidence:** The diff leaves fixed core layout and persistence unchanged and adds only transient app viewport state (`reveal_caret`) plus presentation geometry. This agrees with the work order's `Requirements impact: verify unchanged` and `Blueprint impact: verify unchanged` classification in [`WO-001-centered-page-viewport.md`](../work-orders/WO-001-centered-page-viewport.md:62). The implementation does not persist zoom, scroll, or caret coordinates.
- **Recommendation:** None; retain the classification and execution-log evidence.
- **Required next state:** Treat durable requirements and blueprint documents as verified unchanged for WO-001.

## Exact check results

Validator reran all four required automated checks from the work order:

| Check | Exact result | Outcome |
|---|---|---|
| `cargo fmt --all -- --check` | Exit 0; no output | PASS |
| `cargo check --workspace` | Exit 0; `Finished dev profile [unoptimized + debuginfo] target(s) in 0.23s` | PASS |
| `cargo test --workspace` | Exit 0; app 16 passed/0 failed, core 22 passed/0 failed, E2E 3 passed/0 failed, doc tests 0 passed/0 failed | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0; no diagnostics | PASS |
| `cargo run -p scriptcraft` | Process launched and printed `Running target/debug/scriptcraft`; no native interaction was available; validator terminated the still-running process | PARTIAL / MANUAL EVIDENCE UNAVAILABLE |

## Verdict and orchestrator next state

- **Verdict:** `PASS WITH WARNINGS`.
- **Why:** The implementation is aligned with WO-001's authorized viewport scope, the diff is comparable and bounded, all required automated checks passed independently, and the durable-document classification is not contradicted. The only warning is the uncompleted interactive native smoke, which is not a configured hard-stop policy gate.
- **Orchestrator action:** The orchestrator should set WO-001 to `complete` under the Forge pass-with-warnings rule, linking this validation and retaining W-001 as an honest manual-evidence follow-up. It should **not** create a fix work order and should not treat the native smoke as observed success.
- **Scope guard:** This result does not validate or complete WO-002, WO-003, or WO-004.

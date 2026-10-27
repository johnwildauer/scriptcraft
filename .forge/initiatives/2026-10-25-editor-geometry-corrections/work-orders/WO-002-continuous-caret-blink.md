# Work Order: WO-002 — Continuous Caret Blink

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer after WO-001 validation
- **Phase/order:** `2 of 6`
- **Initiative:** [`../editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md)
- **Validation:** [`../validations/validation-WO-002-v2.md`](../validations/validation-WO-002-v2.md)
- **Fixes:** [`WO-002a-focus-regain-blink-reset.md`](WO-002a-focus-regain-blink-reset.md)

## Summary

Make the insertion caret blink continuously whenever the painted editor retains focus, without storing animation state in the screenplay model.

## In scope

- Use egui animation/repaint APIs for focused collapsed-caret blink.
- Verify blink continues across idle frames and resets predictably after caret/revision changes.
- Ensure unfocused or range-selected states do not request caret blink repaint.
- Measure and record focused versus unfocused frame/repaint behavior.

## Out of scope

- Timer threads, model/session blink state, caret geometry redesign, or native font work.

## Requirements and blueprints

- **Requirements:** `AC-WRITING-002.4`, `NFR-WRITING-002` in [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md).
- **Blueprint:** [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md).

## Implementation guidance

Inspect the existing [`paint_page()`](../../../apps/scriptcraft/src/main.rs:309) animation identity and repaint scheduling. Keep blink state transient and keyed to stable editor/caret identity. Do not use unconditional frame requests as a substitute for the focused blink schedule.

## Validation

- Add rendered egui tests for focused idle blink, unfocused idle behavior, selection suppression, and caret/revision reset.
- Run all Cargo gates and record native manual limitations honestly.

## Durable-document impact

- **Requirements:** `verify unchanged`.
- **Blueprint:** `update` if the desktop caret/repaint contract becomes more precise.

## Reconciled handoff

- **Applicable requirements (verbatim):** `AC-WRITING-002.4`: “Given a document position and rendered geometry, the system shall support a safe caret/hit-test round trip at valid text boundaries.” `NFR-WRITING-002`: “Core editing, layout, and persistence behavior shall be exercisable without a desktop window for deterministic tests.”
- **Observable oracle:** Focused collapsed caret alternates on the existing 500 ms phase interval; repaint is scheduled only while focused. Focus loss, range selection, revision change, or caret identity change suppresses/reset blink. The test uses controlled egui time/repaint output, not an unbounded wall-clock claim.
- **Rationale/evidence:** [`paint_page()`](../../../apps/scriptcraft/src/main.rs:367) currently owns animation identity and 500 ms repaint scheduling; the change is transient desktop state only.
- **Authorized documentation changes:** Update [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) with the repaint/caret contract and this WO with controlled-frame evidence.
- **Native evidence boundary:** Rendered egui tests and Cargo gates are required; native blink observation is optional and unavailable evidence is a warning, not a pass claim.

## Implementation evidence — 2026-10-26

- **Status decision:** This fresh Code-mode session implemented only WO-002 and leaves this work order `in-review`; fresh independent validation remains required. WO-003 through WO-006 were not implemented.
- **Changed product file:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs) now derives the blink phase from egui-controlled time, stores the phase through egui's animation manager, and schedules the next repaint at the next 500 ms boundary only for a focused collapsed caret. The animation ID includes editor surface, revision, element, and caret offset, so revision/caret identity changes use a new transient animation entry; range selection and unfocused state return hidden without blink repaint scheduling.
- **Controlled rendered evidence:** `rendered_caret_blink_alternates_on_controlled_egui_time` observes visible phases at 0/499 ms, hidden phases at 500/999 ms, and visibility again at 1000 ms. `rendered_caret_blink_schedules_only_focused_collapsed_phases` observes a 500 ms repaint delay only while focused/collapsed and `Duration::MAX` for unfocused or range-selected states. `rendered_caret_blink_identity_change_uses_a_distinct_entry` covers caret identity reset behavior.
- **Authorized desktop documentation:** [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) records the 500 ms egui-only caret/repaint contract. Requirements remain verified unchanged.
- **Native/manual evidence:** No native window inspection was performed in this session; it remains optional/unavailable under project policy and is not inferred from rendered tests.
- **Follow-up:** Fresh validator must independently inspect WO-002, rerun all Cargo gates, and retain this work order at `in-review` unless validation authorizes a later transition.

## Fix-chain evidence — WO-002a — 2026-10-26

- **Hard-stop addressed:** [`validation-WO-002-v2.md`](../validations/validation-WO-002-v2.md) found that focus loss left the reused egui animation entry unchanged, allowing focus regain to resume a stale phase. WO-002 remains `in-review`; this fix does not complete it.
- **Fix boundary:** [`WO-002a-focus-regain-blink-reset.md`](WO-002a-focus-regain-blink-reset.md) adds only transient app/UI focus-generation and reset-origin state. The animation identity now includes the generation, and regain computes the existing 500 ms phase from the new origin, starting visible. No model/session/timer state was added.
- **Regression:** `rendered_caret_blink_focus_regain_starts_at_initial_visible_phase` advances to a hidden phase, verifies unfocused output is hidden with `Duration::MAX` repaint scheduling, then verifies focus regain is visible at the reset origin through 499 ms and hidden at 500 ms.
- **Changed product/documentation files:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md), this work order, [`WO-002a-focus-regain-blink-reset.md`](WO-002a-focus-regain-blink-reset.md), and [`execution-log.md`](../execution-log.md). WO-003 through WO-006 were not implemented.
- **Gate evidence:** `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace` (29 app tests, 40 core tests, 2 WO-002 integration tests, 5 WO-003 integration tests, 3 WO-008 integration tests, 0 doc-test failures), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace`, and `git diff --check` all passed. Native visual inspection remains optional/unavailable and is not inferred.

## Validation outcome

- Initial validation [`validation-WO-002-v2.md`](../validations/validation-WO-002-v2.md) found a hard-stop focus-regain reset defect.
- Fix [`WO-002a-focus-regain-blink-reset.md`](WO-002a-focus-regain-blink-reset.md) passed fresh validation [`validation-WO-002a-v2.md`](../validations/validation-WO-002a-v2.md) with warnings.
- State transition: `in-review → complete` through the WO-002a fix chain; no further fix is required.

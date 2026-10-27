# Work Order: WO-002a — Focus-Regain Blink Reset

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer after WO-002 validation failure
- **Phase/order:** `fix for 2 of 6`
- **Initiative:** [`../editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md)
- **Original work order:** [`WO-002-continuous-caret-blink.md`](WO-002-continuous-caret-blink.md)
- **Failing validation:** [`../validations/validation-WO-002-v2.md`](../validations/validation-WO-002-v2.md)
- **Validation:** [`../validations/validation-WO-002a-v2.md`](../validations/validation-WO-002a-v2.md)

## Summary

Close the WO-002 hard-stop: focus loss must reset the transient caret blink animation so focus regain starts from the defined initial phase rather than resuming stale animation state.

## In scope

- Make focus-loss → focus-regain a reset boundary for the existing egui animation identity/state.
- Preserve the approved 500 ms phase interval, focused-collapsed-only rendering, and repaint-only-while-focused policy.
- Add a deterministic rendered regression that advances blink, loses focus, regains focus, and verifies the initial phase/reset behavior.
- Record the fix chain and evidence in the original WO, this fix WO, and the v2 execution log.

## Out of scope

- New blink timing, timer threads, model/session state, caret geometry, or unrelated focus/input behavior.

## Requirements and blueprints

- **Requirements (verbatim):** `AC-WRITING-002.4`: “Given a document position and rendered geometry, the system shall support a safe caret/hit-test round trip at valid text boundaries.” `NFR-WRITING-002`: “Core editing, layout, and persistence behavior shall be exercisable without a desktop window for deterministic tests.”
- **Blueprint:** [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md).

## Implementation guidance

Use transient app/UI state or an egui animation identity that includes a focus-generation/reset token. Do not serialize the token or place it in [`EditingSession`](../../../crates/scriptcraft-core/src/editing.rs:59). Ensure the reset occurs only on an actual focus transition and does not create a perpetual repaint loop while unfocused.

## Validation

- Run all Cargo gates.
- Test phase alternation, focus-loss suppression, focus-regain reset, caret/revision reset, range-selection suppression, and repaint scheduling.
- Native visual inspection is optional under project policy; unavailable native evidence is recorded as unavailable.

## Durable-document impact

- **Requirements:** `verify unchanged` — this is a transient presentation fix within the existing blink contract.
- **Blueprint:** `update` only to clarify focus-regain reset evidence if needed.
- **Rationale/evidence:** [`validation-WO-002-v2.md`](../validations/validation-WO-002-v2.md) found one hard-stop mismatch: the reused animation identity does not reset on focus regain.
- **Authorized documentation changes:** Update the original WO, this fix WO, [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md), and [`execution-log.md`](../execution-log.md) with the fix-chain evidence.

## Implementation evidence — 2026-10-26

- **Status decision:** This fresh Code-mode session implemented only this narrowly scoped fix and leaves WO-002a `in-review`; fresh independent validation remains required. WO-002 remains `in-review`, and WO-003 through WO-006 were not implemented.
- **Changed product file:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs) now tracks a transient focus generation and reset origin in [`ScriptCraftApp`](../../../apps/scriptcraft/src/main.rs:38). An actual focus transition increments the generation; focus regain records the current egui time as the phase origin and requests one transition repaint. The blink identity includes the generation, and the existing 500 ms phase is evaluated relative to that origin. Unfocused/range-selected paths still return hidden without blink repaint scheduling.
- **Deterministic rendered regression:** `rendered_caret_blink_focus_regain_starts_at_initial_visible_phase` advances a focused caret to its hidden phase, verifies focus loss suppresses painting and reports `Duration::MAX`, then uses a new focus-generation identity to verify regain is visible at the reset origin and through 499 ms, followed by the normal hidden phase at 500 ms.
- **Preserved contract:** The approved 500 ms interval, focused collapsed-only rendering, conditional repaint policy, egui-owned transient animation state, caret/revision identity reset, and no model/session/timer state remain unchanged.
- **Changed files/evidence:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md), [`WO-002-continuous-caret-blink.md`](WO-002-continuous-caret-blink.md), this work order, and [`execution-log.md`](../execution-log.md).
- **Required Cargo gates:** `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace` (29 app tests, 40 core tests, 2 WO-002 integration tests, 5 WO-003 integration tests, 3 WO-008 integration tests, 0 doc-test failures), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace`, and `git diff --check` all passed. Native visual inspection was not performed and remains optional/unavailable.
- **Next state:** Fresh independent validation must assess this fix work order. No completion status is claimed.

## Validation outcome

- Fresh validation [`validation-WO-002a-v2.md`](../validations/validation-WO-002a-v2.md) returned `PASS WITH WARNINGS`.
- The focus-regain reset hard stop is resolved; no additional fix work order is required.
- State transition: `in-review → complete`.

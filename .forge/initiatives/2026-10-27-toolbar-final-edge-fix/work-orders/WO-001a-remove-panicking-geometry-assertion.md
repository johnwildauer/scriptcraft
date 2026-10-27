# Work Order: WO-001a — Remove Panicking Geometry Assertion

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh final-edge fix implementer
- **Phase/order:** Fix for WO-001; follows user-reported debug crash
- **Initiative:** [`../toolbar-final-edge-fix-v1.md`](../toolbar-final-edge-fix-v1.md:1)
- **Original work order:** [`WO-001-final-edge-child-budget.md`](WO-001-final-edge-child-budget.md:1)
- **Failure report:** User launch crash at [`debug_assert!`](../../../../apps/scriptcraft/src/main.rs:644)
- **Validation:** [`../validations/validation-WO-001a.md`](../validations/validation-WO-001a.md)
- **Implementer session/agent ID:** `code-toolbar-final-edge-assertion-fix-2026-10-27T0813Z`
- **Session exception:** none

## Summary

Fix the debug-build launch crash introduced by the final-edge work order. The current `debug_assert!` treats an egui child intrinsic-width expansion as an impossible invariant, but that expansion is exactly the behavior being contained/diagnosed. The assertion must be removed or converted to non-panicking diagnostic/test evidence.

## In scope

- Remove the panicking [`debug_assert!`](../../../../apps/scriptcraft/src/main.rs:644) from the runtime toolbar path.
- Preserve the child-budget containment fix and rectangle ledger/tests.
- If useful, retain a non-panicking debug-only diagnostic or move the invariant into a deterministic test assertion that does not execute during app startup.
- Run all required Cargo gates and verify `cargo run -p scriptcraft` no longer immediately panics from this assertion.

## Out of scope

- Changing user-local geometry constants, viewport flags, hover behavior, toolbar order, core, persistence, exports, or page layout.
- Claiming the native right-edge issue is visually fixed; the user must verify it after the crash is removed.

## Requirements and blueprints

- **Requirements:** Verify [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90) unchanged.
- **Blueprints:** Verify [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:68) unchanged.

## Implementation plan

1. Remove or replace the runtime `debug_assert!` at [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:622) with a non-panicking mechanism.
2. Preserve the existing rectangle ledger and exact-edge tests.
3. Run formatting, check, workspace tests, build, Clippy, diff check, and a bounded `cargo run -p scriptcraft` smoke attempt.

## Acceptance

`cargo run -p scriptcraft` no longer panics at startup because of the child response width assertion; all required gates pass; existing child-budget evidence remains; no unrelated behavior changes.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `verify unchanged`
- **Rationale:** This is a runtime safety correction to an over-strict diagnostic assertion within the existing implementation boundary.

## Implementation and validation evidence — 2026-10-27

- **Implementer session/agent ID:** `code-toolbar-final-edge-assertion-fix-2026-10-27T0813Z`.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:622), this work order, and [`execution-log.md`](../execution-log.md:70). The source change removes only the runtime child-response-width `debug_assert!` and its now-unused response binding. The [`ui.set_width(width)`](../../../../apps/scriptcraft/src/main.rs:632) child-budget containment fix, rectangle ledger, tests, user-local constants, and unrelated behavior remain unchanged.
- **Exact required gate results:** `cargo fmt --all -- --check` — PASS, exit 0; `cargo check --workspace` — PASS, exit 0; `cargo test --workspace` — PASS, exit 0 (62 app unit tests, 44 core unit tests, all listed workspace integration tests, and 0 doc-test failures); `cargo build --workspace` — PASS, exit 0; `cargo clippy --workspace --all-targets -- -D warnings` — PASS, exit 0; `git diff --check` — PASS, exit 0. An initial combined gate attempt reached Clippy and failed only because the removed assertion left an unused `child_response`; the binding was removed and the exact required sequence was rerun successfully.
- **Bounded implementation smoke:** `cargo run -p scriptcraft` reached `target/debug/scriptcraft` and was terminated by the five-second bound with status `143`; no immediate assertion panic appeared in the captured output. This is launch-smoke evidence only, not native-window or visual validation.
- **Native interaction limitation:** The terminal-only session could not observe or interact with the native window, so no claim is made that the user-visible rightmost toolbar edge is visually verified. No separate fresh validation beyond the requested implementation smoke was performed.

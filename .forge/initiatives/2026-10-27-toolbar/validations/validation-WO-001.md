# Validation: WO-001 — Toolbar Item Model and Default Actions

## Validation metadata

- **Review type:** `validation`
- **Status:** `in-review`
- **Validator:** Fresh-context Forge debug validator
- **Validator session/agent ID:** `debug-fresh-context-toolbar-WO-001-validation-2026-10-27T02:36:00Z`
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Initiative:** [`toolbar-v1.md`](../toolbar-v1.md:1)
- **Validation file:** `validation-WO-001.md`
- **Validated work order:** [`WO-001-toolbar-item-model-and-default-actions.md`](../work-orders/WO-001-toolbar-item-model-and-default-actions.md:1)
- **Compared commit/diff:** Working-tree file diff; no commit was available. The compared implementation boundary was [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1), plus the authorized durable-document updates in [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:91) and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:55).
- **Fix chain:** `none`

## Fresh-context statement and authorized scope

This validation was performed in a separate fresh context. I read the Forge
validation process, project manifest, toolbar initiative, approved design review,
WO-001, execution log, validation template, changed-file record, implementation
source, tests, and working-tree diff. I did not implement changes, did not mark
WO-001 complete, and did not validate or infer completion of WO-002 or WO-003.

The assessment is limited to WO-001: the desktop-owned typed Item model, exact
default order, New/Open/Save/Save As/Export PDF dispatch preservation, labels and
icon-first borderless-hover presentation seam, focused tests, durable-document
classification, and the required Cargo gates. Flexible-space measurement,
overflow extraction, Elements role synchronization/mutation, and nested menu
behavior are excluded because they belong to WO-002/WO-003.

## Comparison basis and checklist

- The work order was `in-review` and the execution log identifies the separate
  implementer session [`code-implementer-toolbar-wo001-2026-10-27-0233`](../work-orders/WO-001-toolbar-item-model-and-default-actions.md:13).
- The changed-file record and diff are consistent: production changes are in the
  desktop app, the two authorized durable documents were updated, and the Forge
  evidence artifacts were recorded. No core crate, persistence schema, layout
  constants, export implementation, WO-002, or WO-003 source was changed.
- The source comparison found a typed `ToolbarItem`, `ToolbarItemKind`, and
  `ToolbarAction` model, with required labels and stable IDs at
  [`ToolbarItem`](../../../../apps/scriptcraft/src/main.rs:69) and the exact
  composition in [`default_toolbar_items()`](../../../../apps/scriptcraft/src/main.rs:120).
- The action adapter at
  [`dispatch_toolbar_action()`](../../../../apps/scriptcraft/src/main.rs:283)
  routes the five authorized actions to the existing New/Open/Save/Save As/PDF
  operations. The existing save/open safety and fixed-layout PDF paths remain
  behind those operations; the New path resets the session and native path as
  before the toolbar seam.
- The action renderer uses icon-plus-label content, `.frame(false)`, hover text,
  and pointer feedback at [`toolbar_action_button()`](../../../../apps/scriptcraft/src/main.rs:309).
  This is aligned with the requested icon-first, borderless presentation seam;
  native lightness/hover appearance was not visually observed.
- Focused tests cover the exact order/labels/action icons, typed normal and
  flexible spacing, and New dispatch at
  [`default_toolbar_has_approved_order_and_labels()`](../../../../apps/scriptcraft/src/main.rs:1224),
  [`toolbar_model_keeps_normal_and_flexible_spacing_typed()`](../../../../apps/scriptcraft/src/main.rs:1254),
  and [`toolbar_new_dispatch_replaces_session_without_persisting_path()`](../../../../apps/scriptcraft/src/main.rs:1266).
- The durable-document `update` classifications are supported by the actual
  requirements addition and desktop blueprint toolbar contract. The updates are
  within the authorized documentation boundary and do not move semantic role
  ownership or persistence state into the desktop toolbar.

## Independently rerun checks

The following project-policy commands were rerun in this fresh validation
session and passed:

- `cargo fmt --all -- --check` — PASS
- `cargo check --workspace` — PASS
- `cargo test --workspace` — PASS; 35 app tests, 44 core unit tests, all listed
  workspace integration tests, and doc tests passed with zero failures.
- `cargo build --workspace` — PASS
- `cargo clippy --workspace --all-targets -- -D warnings` — PASS

A bounded `cargo run -p scriptcraft` attempt successfully built and launched
`target/debug/scriptcraft`, but the process remained running until the validator
terminated it after eight seconds. No native window interaction was observed.

## Findings

### Blockers

- **None.** The comparison was meaningful, the changed behavior is within the
  authorized WO-001 boundary, no rogue behavior was observed, and no required
  project-policy Cargo gate failed.

### Warnings

- **WO001-VAL-W-001 — Native visual hover/click evidence unavailable.** **Severity:** warning. **Evidence:** The bounded [`cargo run -p scriptcraft`](../../../../Cargo.toml:1) launch produced a running binary but no observable native interaction before the validator terminated it; the implementer recorded the same limitation in [`WO-001-toolbar-item-model-and-default-actions.md`](../work-orders/WO-001-toolbar-item-model-and-default-actions.md:93). **Recommendation:** Preserve this as an honest optional/manual evidence limitation. Do not claim native visual success from source inspection, launch, or rendered/headless tests. **Required next state:** `complete` after the orchestrator records this warning; no fix work order is required because the manifest configures no manual/external hard-stop gate.

- **WO001-VAL-W-002 — Dispatch regression coverage is asymmetric.** **Severity:** warning. **Evidence:** The source adapter explicitly maps all five actions at [`dispatch_toolbar_action()`](../../../../apps/scriptcraft/src/main.rs:283), but the new focused dispatch test directly exercises only New at [`toolbar_new_dispatch_replaces_session_without_persisting_path()`](../../../../apps/scriptcraft/src/main.rs:1266); Open/Save/Save As involve native dialogs and PDF export writes a file. **Recommendation:** Retain the source and existing operation tests as the current alignment evidence; add direct adapter-level coverage only in a future bounded test improvement if desired, without expanding this validation into WO-002/WO-003. **Required next state:** `complete`; this is not a hard-stop under the validation policy.

### Notes

- **WO001-VAL-N-001 — Exact model/composition boundary aligned.** **Severity:** note. **Evidence:** [`default_toolbar_items()`](../../../../apps/scriptcraft/src/main.rs:120) returns New, Open, Save, Save As, Flexible Space, Elements, Flexible Space, Export PDF, and the focused test asserts the labels and relevant typed kinds. **Recommendation:** Accept as aligned WO-001 evidence. **Required next state:** `complete`.

- **WO001-VAL-N-002 — WO-002/WO-003 seams remain unimplemented by design.** **Severity:** note. **Evidence:** The renderer intentionally skips spacing measurement/overflow extraction and renders Elements as a placeholder at [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:324), matching the WO-001 exclusions. **Recommendation:** Leave role synchronization, spacing allocation, overflow, and nested dropdown behavior to their authorized work orders. **Required next state:** `complete` for WO-001 only; do not use this report to validate later work orders.

- **WO001-VAL-N-003 — Durable-document classification is accurate.** **Severity:** note. **Evidence:** The implementation includes the authorized requirements and desktop-blueprint updates recorded in [`WO-001-toolbar-item-model-and-default-actions.md`](../work-orders/WO-001-toolbar-item-model-and-default-actions.md:62), and the diff does not alter core semantic or persistence ownership. **Recommendation:** Retain the `Requirements impact: update` and `Blueprint impact: update` classification. **Required next state:** `complete`.

## Decision

- **Outcome:** `PASS WITH WARNINGS`
- **Required next state:** The orchestrator may transition WO-001 from `in-review` to `complete` after linking this validation result in the execution record. Preserve both warnings, especially the unavailable native visual evidence, and keep WO-002 and WO-003 undispatched/unvalidated by this report.
- **Fix work order required:** `no`

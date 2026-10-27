# Work Order: WO-002a — Toolbar Chrome Color Gate Fix

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh desktop toolbar fix implementer
- **Phase/order:** Fix for WO-002; follows failed validation
- **Initiative:** [`../toolbar-final-fit-v1.md`](../toolbar-final-fit-v1.md:1)
- **Original work order:** [`WO-002-toolbar-chrome-color.md`](WO-002-toolbar-chrome-color.md:1)
- **Failing validation:** [`../validations/validation-WO-002.md`](../validations/validation-WO-002.md:1)
- **Validation:** [`../validations/validation-WO-002a.md`](../validations/validation-WO-002a.md)
- **Implementer session/agent ID:** `code-implementer-toolbar-final-fit-wo002a-2026-10-27T0438Z`
- **Session exception:** none

## Summary

Repair WO-002's incomplete toolbar chrome-color implementation. Resolve the undefined helpers, apply the documented egui panel-fill source through the actual toolbar path, and restore the required gates without changing geometry or global theme behavior.

## In scope

- Resolve the undefined [`toolbar_chrome_color()`](../../../../apps/scriptcraft/src/main.rs:1373) and [`toolbar_frame()`](../../../../apps/scriptcraft/src/main.rs:1377) references reported by validation.
- Ensure [`update()`](../../../../apps/scriptcraft/src/main.rs:1347) actually applies the documented `Visuals::panel_fill` source to the toolbar panel/frame.
- Preserve the native OS title-bar limitation and no pixel-equality claim.
- Run all required Cargo and diff gates.

## Out of scope

- Geometry, padding, Item grouping, command routing, global theme redesign, core, persistence, exports, and page layout.

## Requirements and blueprints

- **Requirements:** Verify [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90) unchanged.
- **Blueprints:** Verify the toolbar chrome section of [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:88) remains accurate.

## Implementation plan

1. Inspect the current source around the undefined references and make the smallest compile-safe helper/application correction.
2. Add focused deterministic tests if needed for the helper/frame source.
3. Run `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check`.

## Files and systems

- **Create:** This fix work order and validation artifact.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1347), this work order, and execution log.
- **Avoid changing:** geometry constants, core, persistence, exports, Item semantics, and global visuals.

## Acceptance

Given WO-002's intended `Visuals::panel_fill` contract, the toolbar must compile, apply the source through its actual `TopBottomPanel` path, pass all required gates, and honestly document native title-bar limitations.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `verify unchanged`
- **Rationale:** The fix restores the implementation promised by WO-002 without changing the contract.

## Implementation evidence

- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1367), this work order, and [`execution-log.md`](../execution-log.md:106). The existing cumulative [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:88) and requirements changes were not modified by WO-002a.
- **Implementation:** Added [`toolbar_chrome_color()`](../../../../apps/scriptcraft/src/main.rs:1369) as a direct read of `ctx.style().visuals.panel_fill`; added [`toolbar_frame()`](../../../../apps/scriptcraft/src/main.rs:1373) using `Frame::side_top_panel()` with that fill; and applied the frame through the actual [`TopBottomPanel::top("toolbar")`](../../../../apps/scriptcraft/src/main.rs:1381) path.
- **Preservation:** No geometry constants, padding, Item grouping, command routing, global visuals, core, persistence, exports, or page layout were changed by this fix. The focused existing tests cover source equality, no style mutation, and top-panel frame geometry defaults.
- **Required gate results:** `cargo fmt --all -- --check` — exit 0, no formatter diagnostics; `cargo check --workspace` — exit 0, dev profile finished; `cargo test --workspace` — exit 0, all workspace tests passed (52 app unit tests plus workspace/core integration and doc-test suites); `cargo build --workspace` — exit 0, dev profile finished; `cargo clippy --workspace --all-targets -- -D warnings` — exit 0, dev profile finished; `git diff --check` — exit 0, no whitespace errors.
- **Comparable diff:** The bounded source fix is the helper definitions and [`TopBottomPanel`](../../../../apps/scriptcraft/src/main.rs:1381) frame application in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1367); the working tree also contains cumulative WO-001/WO-002 initiative artifacts and is not treated as WO-002a-only diff evidence.
- **Native title-bar limitation:** eframe's default native decorations own the OS title bar and expose no sampleable title-bar color in this setup. Equality to egui's exposed `Visuals::panel_fill` application-surface source is claimed; native OS title-bar pixel/color equality is not claimed.
- **Requirements/blueprint evidence:** Requirements and the toolbar chrome section of the desktop blueprint were checked and remain accurate; no durable-document update is authorized for this fix.
- **Fresh validation:** Not performed in this implementation session. [`validation-WO-002.md`](../validations/validation-WO-002.md:1) remains the failed prior validation, and a separate fresh WO-002a validation is required. WO-002 remains `in-review`.
- **Next state:** `in-review`, awaiting fresh-context validation of WO-002a.

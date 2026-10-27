# Work Order: WO-002 — Toolbar Chrome Color

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop toolbar implementer (fresh session)
- **Phase/order:** 2 of 2; depends on WO-001
- **Initiative:** [`../toolbar-final-fit-v1.md`](../toolbar-final-fit-v1.md:1)
- **Validation:** [`../validations/validation-WO-002.md`](../validations/validation-WO-002.md)
- **Implementer session/agent ID:** `code-implementer-toolbar-final-fit-wo002-2026-10-27T0435Z`
- **Session exception:** none

## Summary

Make the toolbar background match the identified title-bar/application chrome color without altering the global theme or semantic toolbar behavior.

## In scope

- Identify whether the title bar is native OS chrome or an egui style surface.
- Apply the matching available color to the toolbar `TopBottomPanel`/frame.
- Add a focused color-source/style regression where deterministic and document platform limitations where native title-bar sampling is unavailable.

## Out of scope

- Geometry/padding, Item grouping, new commands, global theme redesign, core, persistence, exports, and editor layout.

## Requirements and blueprints

- **Requirements:** Verify REQ-WRITING-LOOP-005 unchanged; this is desktop presentation only.
- **Blueprints:** Update the toolbar chrome presentation section of [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58).

## Implementation plan

1. Inspect the eframe/native title-bar configuration and current egui visuals.
2. Apply one documented toolbar fill source in [`update()`](../../../../apps/scriptcraft/src/main.rs:1347) or a focused helper.
3. Add tests/documentation for the selected source and platform limitation.

## Files and systems

- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1347), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58), this work order, and execution log.
- **Avoid changing:** geometry, core, persistence, exports, and global theme behavior.

## Validation

- **Automated:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- **Acceptance:** The toolbar uses the same deterministic app-chrome color source as the title bar where the framework exposes it; unavailable native color comparison is recorded honestly.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `update`
- **Affected:** toolbar chrome section in [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58)
- **Rationale:** This changes desktop presentation ownership only.

## Implementation evidence

- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1367), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:88), this work order, and [`execution-log.md`](../execution-log.md:1).
- **Framework/source identification:** eframe `0.29.1` uses native window decorations by default through `NativeOptions::default()`; the native title bar has no framework-exposed sampleable color in this setup. egui does expose the deterministic application-surface source `ctx.style().visuals.panel_fill`, which is now returned by `toolbar_chrome_color()` and applied to the toolbar's explicit `Frame::side_top_panel()`.
- **Implementation:** `TopBottomPanel::top("toolbar")` now receives `toolbar_frame(ctx)`, preserving the existing top-panel margins, rounding, shadow, stroke, and all toolbar geometry while making the fill source explicit. No global style/theme mutation was added.
- **Focused deterministic coverage:** `toolbar_chrome_uses_the_style_panel_fill_without_mutating_style()` verifies style-source equality and `toolbar_frame_preserves_top_panel_geometry_defaults()` verifies the explicit frame does not alter panel geometry defaults.
- **Required checks:** Per the user's explicit instruction not to perform fresh validation in this implementation session, these exact checks were not run here and no results are claimed: `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`. Separate fresh validation is required.
- **Native visual limitation:** The toolbar can be proven equal to the egui `panel_fill` source deterministically. The native OS title bar is outside eframe/egui's exposed color API, so pixel equality to the OS title bar was not observed and is not claimed.
- **Requirements/blueprint evidence:** Requirements are verified unchanged because this is desktop presentation only. The desktop blueprint records the explicit app-chrome source and native-platform boundary above.
- **Comparable diff:** `git diff -- apps/scriptcraft/src/main.rs .forge/blueprints/containers/scriptcraft-desktop.md .forge/initiatives/2026-10-27-toolbar-final-fit/work-orders/WO-002-toolbar-chrome-color.md .forge/initiatives/2026-10-27-toolbar-final-fit/execution-log.md` is the bounded implementation/evidence diff boundary.
- **Next state:** Fresh-context WO-002 validation is required; this implementer session does not mark the work order complete.

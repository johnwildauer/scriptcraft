# Work Order: WO-001 — Exact Runtime Label Width

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop toolbar implementer (fresh session)
- **Phase/order:** 1 of 2
- **Initiative:** [`../toolbar-exact-width-hover-v1.md`](../toolbar-exact-width-hover-v1.md:1)
- **Validation:** [`../validations/validation-WO-001.md`](../validations/validation-WO-001.md)
- **Implementer session/agent ID:** `code-implementer-toolbar-exact-width-hover-2026-10-27-0734Z`
- **Session exception:** none

## Summary

Replace guessed `character_count × fixed_width` label sizing with exact runtime egui text measurement and use that measurement consistently in toolbar allocation, Flexible Space calculation, rendering, and right-edge containment.

## In scope

- Measure each Item label using the active egui font/style/galley path rather than a fixed character-width approximation.
- Propagate exact widths through [`toolbar_item_width()`](../../../../apps/scriptcraft/src/main.rs:171), [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:200), and [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:545).
- Preserve minimum control width, group padding, row gaps, overflow, Flexible Space, action routing, and role behavior.
- Add tests or a pure measurement seam proving Export PDF and other labels use exact/non-guessed widths.

## Out of scope

- Hover region behavior, tooltip removal, viewport, core, persistence, exports, page layout, and global theme.

## Requirements

- **AC-WRITING-005.5:** “Given a toolbar width, normal Items and Space shall consume one deterministic normal-item width, Flexible Space Items shall divide the remaining width equally, and a narrow toolbar shall retain the leftmost fitting Items while exposing non-spacing Items through a far-right `>>` text menu.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:68) — deterministic toolbar allocation.

## Validation

- **Automated:** format, check, tests, build, Clippy with `-D warnings`, and diff check.
- **Acceptance:** Allocation uses measured active-style label widths, not a character-count approximation; rendered width and measured width agree; rightmost group remains contained.

## Durable-document impact

- **Requirements impact:** `update`
- **Blueprint impact:** `update`
- **Rationale:** Exact runtime measurement changes the user-facing width/allocation contract.

## Implementation evidence

- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:114), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:72), and this work order plus the initiative execution log.
- **Implementation:** Replaced character-count sizing with active egui `Body` style/font galley measurement through `Ui::fonts`; passed the measurement seam through `toolbar_item_width()`, `allocate_toolbar()`, Flexible Space calculation, narrow overflow fitting, and rendered Item widths. Minimum width, 72-point Space, row padding/gaps, overflow reservation, action routing, role behavior, and clipped-right-edge allocation remain intact.
- **Deterministic evidence:** Tests use the same egui context-run/galley measurement seam, assert measured-width-plus-padding with minimum-width preservation, assert the fixed character-width guess is not used, exercise Flexible Space and narrow overflow, and assert the final row edge remains contained.
- **Validation attempts/results:** `cargo fmt --all -- --check` PASS; `cargo check --workspace` PASS; `cargo build --workspace` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `git diff --check` PASS. `cargo test --workspace` executed and returned 101 because four pre-existing/out-of-scope toolbar geometry tests fail: `toolbar_icon_hover_fill_uses_the_existing_hover_color_source_only`, `toolbar_icon_hover_region_is_centered_and_excludes_the_label_area`, `toolbar_manual_geometry_controls_are_narrow_and_explicit`, and `toolbar_vertical_padding_is_explicit_and_reduced`. All WO-001 exact-width/allocation tests pass (56 total passed, 4 unrelated failures).
- **Comparable diff evidence:** `git diff --check` passed; `git status --short` reported only the authorized requirements/blueprint updates, [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), and initiative artifacts. No native visual smoke was claimed; visible-window measurement is unavailable in this session and is a validator/manual follow-up limitation.
- **Follow-up:** Fresh validation must assess only WO-001 alignment and the recorded four unrelated failures; WO-002 remains pending until validation completes.

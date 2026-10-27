# Work Order: WO-002 — Toolbar Width and Gap Correction

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop toolbar implementer (one fresh session)
- **Phase/order:** 2 of 2; depends on WO-001
- **Initiative:** [`../toolbar-polish-v1.md`](../toolbar-polish-v1.md:1)
- **Generated filename:** `WO-002-toolbar-width-and-gap-correction.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-002.md`](../validations/validation-WO-002.md)
- **Fixes:** none
- **Implementer session/agent ID:** `forge-wo-002-implementer-2026-10-27T03:16:51Z`
- **Session exception:** none

## Summary

Correct toolbar width allocation so the rendered row never clips the rightmost Item, even at wide/maximized sizes, and reduce the default gap between adjacent Items by half. The allocator and egui renderer must share one geometry budget that includes gaps, padding, and overflow control width.

## In scope

- Identify the actual egui spacing/padding contribution and reserve it in allocation before assigning Item and Flexible Space widths.
- Make the rendered horizontal gap half the current default while preserving normal hit widths.
- Ensure the complete default composition fits without overflow when available width is sufficient and the rightmost label is fully visible.
- Preserve leftmost-fit overflow and `>>` reservation when width is insufficient.
- Add pure allocation/render regression coverage for wide, maximized-equivalent, exact-fit, narrow, and right-edge cases.

## Out of scope

- Icon/label hierarchy; WO-001 owns presentation.
- New overflow behavior, commands, customization, title-bar/theme work, core semantics, persistence, page layout, exports, and editor geometry.

## Requirements

- **AC-WRITING-005.5:** “Given a toolbar width, normal Items and Space shall consume one deterministic normal-item width, Flexible Space Items shall divide the remaining width equally, and a narrow toolbar shall retain the leftmost fitting Items while exposing non-spacing Items through a far-right `>>` text menu. Action entries and Elements role choices in that menu shall remain executable without persisting toolbar layout state.”
- **NFR-WRITING-001:** “Production paths should prefer typed errors and bounded inputs over panic-prone handling; a discovered crash or data-loss defect requires a regression test.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:68) — toolbar allocation is transient/deterministic and must account for the actual rendered row.

## Implementation plan

1. Add shared toolbar geometry constants/helper in [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:20) for row padding, reduced gap, normal Item width, and overflow reservation.
2. Make [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:166) and [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:489) consume the same width/gap budget and keep the row inside `ui.available_width()`.
3. Add regressions asserting total rendered allocation bounds, equal Flexible Space widths, half-gap behavior, and fully fitting rightmost action at wide widths.

## Files and systems

- **Create:** Tests only if needed.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:20), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:68), this work order, and [`execution-log.md`](../execution-log.md:1).
- **Avoid changing:** [`crates/scriptcraft-core/`](../../../../crates/scriptcraft-core/:1), role/session behavior, persistence, exports, page layout, and WO-001's semantic/presentation behavior except for geometry integration.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- **Manual/external checks:** wide/maximized and narrow native smoke; explicitly inspect the fully visible rightmost label and reduced gaps, without claiming unavailable native interaction.
- **Acceptance test:** Given a wide toolbar with enough room for all default Items, when allocated and rendered, then the row's right edge remains inside the available width, Export PDF is fully visible, and the gap between adjacent normal Items is half the previous default; given a narrow width, overflow remains usable.

## Durable-document impact

- **Requirements impact:** `update`
- **Blueprint impact:** `update`
- **Affected documents/IDs:** REQ-WRITING-LOOP-005 / AC-WRITING-005.5 and the toolbar allocation section of [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:68)
- **Rationale and evidence:** This corrects the user-facing responsive toolbar contract and documents the geometry budget that prevents clipping.
- **Authorized documentation changes:** Record the shared gap/padding/overflow budget and right-edge fitting guarantee only.

## Completion checklist

- [x] Implementation complete.
- [x] Required checks and validation evidence recorded.
- [x] Durable-document impact reconciled against the diff.
- [x] Execution log includes dispatch/completion session IDs.
- [x] Work order is `in-review` pending separate validation.

## Implementation evidence

- **Session/agent ID:** `forge-wo-002-implementer-2026-10-27T03:16:51Z`
- **Dispatch:** WO-002 dispatched after the separate WO-001 validation returned `PASS WITH WARNINGS`; no WO-001 fix work was required.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:20), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:113), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:70), this work order, and [`execution-log.md`](../execution-log.md:1).
- **Geometry implementation:** Allocation and rendering share 4-point row padding, 4-point horizontal gap, 88-point normal hit width, and 32-point overflow reservation. Flexible widths divide only the post-budget remainder. Rendering uses the same explicit gap/padding budget, and Elements receives the allocated normal hit width.
- **Semantic preservation:** Normal Item widths remain 88 points, Flexible Space widths remain equal, narrow allocation remains leftmost-fit with spacing Items excluded from overflow, and overflow action/Elements routing is unchanged. No core, persistence, page-layout, export, or unrelated UI files changed.
- **Deterministic regressions:** Added total-width bounds, wide/maximized-equivalent Export PDF fit, exact-fit, half-gap (4 points versus egui's 8-point default), equal Flexible Space, and narrow overflow coverage.

## Required command evidence

The initial combined attempt stopped at `cargo fmt --all -- --check` because formatter output was required. `cargo fmt --all` applied only formatter wrapping, then the exact required command chain completed with exit code `0`:

```text
cargo fmt --all -- --check                 PASS (exit 0)
cargo check --workspace                    PASS (exit 0)
cargo test --workspace                     PASS (exit 0; app 46, core 44, integration/doc-test groups passed)
cargo build --workspace                    PASS (exit 0)
cargo clippy --workspace --all-targets -- -D warnings
                                             PASS (exit 0; no diagnostics)
git diff --check                            PASS (exit 0; no whitespace errors)
```

- **Native smoke limitation:** No fresh native visual smoke was performed by this implementation session. A visible-window interaction channel is unavailable, so wide/maximized and narrow visual inspection is not claimed and remains a separate validator/manual limitation.
- **Next state:** `in-review`, pending separate fresh WO-002 validation.

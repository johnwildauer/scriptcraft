# Work Order: WO-002 — Toolbar Item Width Tuning

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop toolbar implementer (one fresh session)
- **Phase/order:** 2 of 2; depends on WO-001
- **Initiative:** [`../toolbar-interaction-polish-v1.md`](../toolbar-interaction-polish-v1.md:1)
- **Generated filename:** `WO-002-toolbar-item-width-tuning.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-002.md`](../validations/validation-WO-002.md)
- **Fixes:** none
- **Implementer session/agent ID:** `code-implementer-toolbar-interaction-polish-2026-10-27-03`
- **Session exception:** none

## Summary

Make toolbar Items visibly narrower while keeping their spacing and boundaries predictable. Preserve the named manual controls for normal Item width, row padding, and inter-Item gap, and verify that allocation/rendering stay within available width after the nested grouping change.

## In scope

- Select and record a narrower `TOOLBAR_NORMAL_ITEM_WIDTH` appropriate for the icon-plus-label group and current hit-target contract.
- Preserve and clearly expose [`TOOLBAR_ROW_PADDING`](../../../../apps/scriptcraft/src/main.rs:22) and [`TOOLBAR_ITEM_GAP`](../../../../apps/scriptcraft/src/main.rs:23) as the manual style/spacing controls.
- Ensure allocator and renderer use the same width/gap/padding values and maintain right-edge bounds, Flexible Space equality, and narrow overflow.
- Add deterministic tests for the narrower width, total row bounds, manual constants, wide rightmost fit, and narrow overflow.

## Out of scope

- Icon-only hover/group structure; WO-001 owns it.
- New overflow semantics, commands, themes, persistence, core semantics, exports, page layout, and editor geometry.

## Requirements

- **AC-WRITING-005.5:** “Given a toolbar width, normal Items and Space shall consume one deterministic normal-item width, Flexible Space Items shall divide the remaining width equally, and a narrow toolbar shall retain the leftmost fitting Items while exposing non-spacing Items through a far-right `>>` text menu. Action entries and Elements role choices in that menu shall remain executable without persisting toolbar layout state.”
- **NFR-WRITING-001:** “Production paths should prefer typed errors and bounded inputs over panic-prone handling; a discovered crash or data-loss defect requires a regression test.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:68) — toolbar allocation is transient, deterministic, and separate from project state.

## Implementation plan

1. Narrow [`TOOLBAR_NORMAL_ITEM_WIDTH`](../../../../apps/scriptcraft/src/main.rs:20) and keep row padding/gap constants explicit and co-used by allocation/rendering.
2. Verify [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:180) and [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:515) maintain identical geometry assumptions.
3. Add/adjust geometry tests and update the authorized requirement/blueprint wording with the final numeric width/gap values.

## Files and systems

- **Create:** Tests only if needed.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:20), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:113), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:68), this work order, and execution log.
- **Avoid changing:** core, persistence, exports, page layout, role/session semantics, and unrelated UI.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- **Manual/external checks:** wide/maximized and narrow native smoke, specifically inspect visible group boundaries and rightmost fit; record unavailable interaction honestly.
- **Acceptance test:** Given the final nested Item groups, when the toolbar is allocated at wide and narrow widths, then normal Items use the recorded narrower width, row padding/gap are visibly adjustable at the named constants, the total row fits its available width, and overflow remains executable.

## Durable-document impact

- **Requirements impact:** `update`
- **Blueprint impact:** `update`
- **Affected documents/IDs:** REQ-WRITING-LOOP-005 / AC-WRITING-005.5 and toolbar allocation in [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:68)
- **Rationale and evidence:** This changes the user-facing Item geometry and manual tuning contract while preserving semantic toolbar behavior.
- **Authorized documentation changes:** Record the narrower normal width and named row padding/gap controls only.

## Completion checklist

- [x] Implementation complete.
- [x] Required checks and validation evidence recorded.
- [x] Durable-document impact reconciled.
- [x] Execution log includes dispatch/completion session IDs.
- [x] Work order is `in-review` pending separate validation.

## Implementation evidence

- **Final geometry values:** `TOOLBAR_NORMAL_ITEM_WIDTH = 72.0`,
  `TOOLBAR_ROW_PADDING = 4.0`, and `TOOLBAR_ITEM_GAP = 4.0`. The constants are
  grouped under the named `Manual toolbar geometry controls` surface in
  `apps/scriptcraft/src/main.rs`.
- **Changed files:** `apps/scriptcraft/src/main.rs`,
  `.forge/requirements/features/screenplay-writing-loop.md`,
  `.forge/blueprints/containers/scriptcraft-desktop.md`, this work order, and
  [`../execution-log.md`](../execution-log.md:1).
- **Implementation:** Normal Item groups now use the narrower 72-point width;
  allocator and renderer continue to share the named width, 4-point padding,
  and 4-point gap assumptions. Flexible Space remains equal, fully fitting rows
  remain bounded, narrow allocation retains leftmost fitting Items and the far-
  right overflow menu, and existing hit/dispatch behavior is unchanged.
- **Focused tests:** Added deterministic coverage for narrower width, manual
  constants, total bounds, wide rightmost fit, Flexible Space equality, and
  narrow overflow; updated deterministic width expectations.
- **Required checks:** Per the instruction not to perform fresh validation in
  this implementation session, the exact required commands were recorded but
  not run here: `cargo fmt --all -- --check`; `cargo check --workspace`;
  `cargo test --workspace`; `cargo build --workspace`;
  `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
  Fresh validation remains the separate validation gate.
- **Native smoke limitation:** No native visual smoke was performed or claimed;
  visible group tightness, right-edge fit, and pointer interaction remain for
  the separate native validation path if available.

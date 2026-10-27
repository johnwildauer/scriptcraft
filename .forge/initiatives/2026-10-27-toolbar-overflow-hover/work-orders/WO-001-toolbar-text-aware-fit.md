# Work Order: WO-001 — Toolbar Text-Aware Fit

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop toolbar implementer (fresh session)
- **Phase/order:** 1 of 2; foundational
- **Initiative:** [`../toolbar-overflow-hover-v1.md`](../toolbar-overflow-hover-v1.md:1)
- **Validation:** [`../validations/validation-WO-001.md`](../validations/validation-WO-001.md)
- **Implementer session/agent ID:** `code-implementer-toolbar-overflow-hover-2026-10-27T0506Z`
- **Session exception:** none

## Summary

Correct toolbar allocation using the full rendered Item group extent rather than only the fixed icon/control width. Export PDF's icon and label must fit inside the available row at wide/maximized widths while narrow overflow remains deterministic.

## In scope

- Measure or deterministically account for label width, icon/control width, centered group bounds, gaps, padding, and overflow reservation.
- Update allocation/rendering so the rightmost Export PDF group is fully inside the row.
- Preserve current Item grouping, Flexible Space, overflow, action, and role behavior.
- Add tests for text-aware width, wide fit, total bounds, exact fit, and narrow overflow.

## Out of scope

- Hover fill behavior and widget color source; WO-002 owns those.
- New commands, ordering, core, persistence, exports, page layout, or global theme changes.

## Requirements

- **AC-WRITING-005.5:** “Given a toolbar width, normal Items and Space shall consume one deterministic normal-item width, Flexible Space Items shall divide the remaining width equally, and a narrow toolbar shall retain the leftmost fitting Items while exposing non-spacing Items through a far-right `>>` text menu.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:68) — transient deterministic toolbar allocation.

## Implementation plan

1. Add a pure text-aware width helper or measured allocation path in [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:159).
2. Use the same group width in [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:177) and [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:529).
3. Add geometry regressions and authorized wording updates.

## Validation

- **Automated:** format, check, tests, build, Clippy with `-D warnings`, and `git diff --check`.
- **Acceptance:** Wide/maximized allocation fully contains Export PDF's icon and label; narrow allocation remains leftmost-fit and overflow remains usable.

## Durable-document impact

- **Requirements impact:** `update`
- **Blueprint impact:** `update`
- **Rationale:** The user-facing toolbar allocation contract changes to include complete rendered group extent.

## Implementation evidence

- **Changed files:** [`main.rs`](../../../../apps/scriptcraft/src/main.rs:20), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:114), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:71), this work order, and [`execution-log.md`](../execution-log.md:1).
- **Implementation:** Added deterministic label-aware group width accounting and used the same widths in wide allocation, narrow leftmost-fit allocation, and rendering. Preserved Item order, Flexible Space, spacing, overflow reservation, action routing, and role behavior.
- **Tests added/updated:** text-aware width, wide/maximized Export PDF fit, total row bounds, exact-fit allocation, and narrow overflow coverage.
- **Cargo gates:** First combined attempt stopped at `cargo fmt --all -- --check` on the expected rustfmt line-wrap correction; after correction, the combined command passed: `cargo fmt --all -- --check` passed; `cargo check --workspace` passed; `cargo test --workspace` passed (53 app unit tests, 44 core unit tests, 2+5+4+6+2+10+5+3 integration tests, 0 doc tests); `cargo build --workspace` passed; `cargo clippy --workspace --all-targets -- -D warnings` passed; `git diff --check` passed. An earlier combined test attempt also exposed the stale exact-fit fixture; it was corrected to use the shared text-aware helper and the final run passed.
- **Diff evidence:** Changes are limited to the files listed above; no WO-002 hover or toolbar-color changes were made.

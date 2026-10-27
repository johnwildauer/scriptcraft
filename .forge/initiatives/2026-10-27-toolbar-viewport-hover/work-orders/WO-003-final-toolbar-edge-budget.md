# Work Order: WO-003 — Final Toolbar Edge Budget

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop toolbar implementer (fresh session)
- **Phase/order:** 3 of 3; depends on WO-001 and WO-002
- **Initiative:** [`../toolbar-viewport-hover-v1.md`](../toolbar-viewport-hover-v1.md:1)
- **Validation:** [`../validations/validation-WO-003.md`](../validations/validation-WO-003.md)
- **Implementer session/agent ID:** `code-implementer-toolbar-viewport-hover-2026-10-27T0555Z`
- **Session exception:** none

## Summary

Reconcile the final toolbar row width with the actual available viewport/content rect so the rightmost Item and label are fully visible, not merely within a logical allocation estimate.

## In scope

- Investigate actual `TopBottomPanel`/content available width, outer insets, row width, label metrics, and child minimums.
- Correct the shared row budget or renderer bounds so the full final group is inside the visible content area.
- Add tests for measured row extent, right-edge containment, wide/maximized, exact-fit, and narrow overflow.

## Out of scope

- Viewport flags, hover response, new commands, core, persistence, exports, page layout, and global theme.

## Requirements

- **AC-WRITING-005.5:** “Given a toolbar width, normal Items and Space shall consume one deterministic normal-item width, Flexible Space Items shall divide the remaining width equally, and a narrow toolbar shall retain the leftmost fitting Items while exposing non-spacing Items through a far-right `>>` text menu.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:68) — deterministic transient toolbar allocation.

## Validation

- **Automated:** format, check, tests, build, Clippy with `-D warnings`, and diff check.
- **Acceptance:** At wide/maximized widths, the final Item group's right edge is <= the actual row/content right edge; narrow overflow remains usable.

## Durable-document impact

- **Requirements impact:** `update`
- **Blueprint impact:** `update`
- **Rationale:** Final actual-content edge containment is a user-facing allocation contract.

## Implementation evidence

- **Changed files:**
  - [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:193) — derives the toolbar budget from the intersection of the actual panel content rect and UI clip rect, then uses that same budget for allocation and rendering; added deterministic actual-edge, wide/maximized, exact-fit, and narrow-overflow tests.
  - [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:121) — authorized requirement update now specifies the actual clipped content edge and the final label's containment.
  - [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:88) — authorized blueprint update records the shared clipped `TopBottomPanel` edge budget and outer-inset reconciliation.
  - [`WO-003-final-toolbar-edge-budget.md`](WO-003-final-toolbar-edge-budget.md:1) — recorded this implementation evidence and gate results; status set to `in-review`.
  - [`execution-log.md`](../execution-log.md:124) — appended dispatch/completion evidence.
- **Requirements/blueprint impact:** Applied the authorized `update` classifications. No viewport flags, hover region, core, persistence, exports, page layout, or unrelated UI behavior changed.
- **Deterministic coverage:** `toolbar_shared_row_edge_contains_final_group_at_actual_content_edge` verifies the clipped content edge; `toolbar_shared_row_edge_contains_wide_and_exact_fit_allocations` covers wide/maximized and exact-fit rows; `toolbar_shared_row_edge_preserves_narrow_overflow_reservation` verifies the `>>` reservation and leftmost overflow semantics. Existing flexible-space tests remain unchanged and pass.

## Required gate results

The first chained attempt stopped at `cargo fmt --all -- --check` because the new test formatting had not yet been applied. After running `cargo fmt --all`, the final chained run completed with exit code 0:

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | `PASS` — exit 0 |
| `cargo check --workspace` | `PASS` — exit 0 |
| `cargo test --workspace` | `PASS` — exit 0; 141 tests passed, 0 failed, 0 ignored across the workspace, including the three new edge-budget tests |
| `cargo build --workspace` | `PASS` — exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | `PASS` — exit 0 |
| `git diff --check` | `PASS` — exit 0; no whitespace errors |

- **Comparable diff:** `git status --short` reported modified [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:193), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:121), and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:88), plus the initiative evidence directory as untracked. `git diff --stat` reported 3 tracked files with 182 insertions and 20 deletions; the complete scoped evidence file list is recorded above.
- **Native smoke limitation:** No fresh native-window smoke validation was performed in this implementation session and no native visual result is claimed. This preserves WO-001/WO-002's documented limitation; the project manifest has no required manual/external gate. Fresh validation remains the next lifecycle step.
- **Follow-up:** Fresh-context WO-003 validation is required. The work order is `in-review`, not `complete`.

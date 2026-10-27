# Work Order: WO-002 — Icon-Only Hover Region

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop toolbar implementer (fresh session)
- **Phase/order:** 2 of 3; depends on WO-001
- **Initiative:** [`../toolbar-viewport-hover-v1.md`](../toolbar-viewport-hover-v1.md:1)
- **Validation:** [`../validations/validation-WO-002.md`](../validations/validation-WO-002.md)
- **Implementer session/agent ID:** `code-implementer-toolbar-viewport-hover-2026-10-27T0539Z`
- **Session exception:** none

## Summary

Limit toolbar hover illumination to the icon control and a small surrounding padding region. The label remains outside that response and does not receive hover fill; clicking the icon continues to dispatch the action.

## In scope

- Split the full Item interaction into an icon-only padded response and a non-interactive label in [`toolbar_action_button()`](../../../../apps/scriptcraft/src/main.rs:446).
- Preserve centered layout, tooltip/accessibility label, action dispatch, and group geometry.
- Add deterministic response/fill tests.

## Out of scope

- Viewport configuration, right-edge allocation, commands, core, persistence, exports, and global theme changes.

## Requirements

- **AC-WRITING-005.3:** “Action Items shall use icon-first, borderless button presentation with a lightness-only hover affordance and retain text labels for tooltip and accessibility use.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58) — transient Item presentation and semantic command routing.

## Validation

- **Automated:** format, check, tests, build, Clippy with `-D warnings`, and diff check.
- **Acceptance:** Hovering icon/padding changes only the icon-control fill; hovering the label does not; icon click dispatch remains intact.

## Durable-document impact

- **Requirements impact:** `update`
- **Blueprint impact:** `update`
- **Rationale:** Hover ownership is a user-facing toolbar interaction contract.

## Implementation evidence

- **Changed files:**
  - [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:446) — action Items now allocate an independent icon-control-plus-2-point-padding response. Only that response drives the existing hover fill and click dispatch; the centered label is rendered separately outside the response and retains normal text color. Tooltip text remains attached to the icon response.
  - [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:104) — authorized AC-WRITING-005.3 wording now names the icon-plus-padding hover/click response and explicitly excludes the label from that response.
  - [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:66) — authorized desktop presentation wording now records the separate icon response and label boundary.
  - [`WO-002-icon-only-hover-region.md`](WO-002-icon-only-hover-region.md:1) — recorded implementation, gate, diff, and limitation evidence; status set to `in-review`.
  - [`execution-log.md`](../execution-log.md:75) — appended dispatch and completion events for this implementer session.
- **Requirements verification/update:** AC-WRITING-005.3 was updated as authorized. Other toolbar order, command routing, role behavior, overflow, viewport settings, and color-source requirements remain unchanged.
- **Blueprint verification/update:** The desktop blueprint was updated as authorized to describe independently allocated egui response regions. No viewport, row-budget, edge, or global-theme contract was changed. WO-003 remains untouched.
- **Deterministic coverage:** `toolbar_icon_hover_region_is_centered_and_excludes_the_label_area` verifies the centered 24-point icon control plus explicit 2-point padding remains within the group and excludes the lower label area. `toolbar_icon_hover_fill_uses_the_existing_hover_color_source_only` verifies the existing hovered widget fill is the only fill source and the non-hover state remains transparent.

## Required gate results

The first chained attempt failed only in the new geometry test because the expected top edge was recorded as `22.0` while the helper correctly produced `20.0` for the in-group padded response. The assertion was corrected; no production behavior was changed by the correction. The final chained run completed with exit code 0:

| Gate | Result |
|---|---|
| `cargo fmt --all` | `PASS` — exit 0 |
| `cargo fmt --all -- --check` | `PASS` — exit 0 |
| `cargo check --workspace` | `PASS` — exit 0 |
| `cargo test --workspace` | `PASS` — exit 0; 138 passed, 0 failed, 0 ignored across workspace test targets (57 app unit, 44 core unit, 2+5+4+6+2+10+5+3 integration, 0 doc tests); new hover tests passed |
| `cargo build --workspace` | `PASS` — exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | `PASS` — exit 0 |
| `git diff --check` | `PASS` — exit 0; no whitespace errors |

- **Comparable diff:** `git status --short` reported modified [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:446), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:104), and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:66), plus the initiative evidence directory as untracked. `git diff --stat` reported 3 tracked files with 92 insertions and 16 deletions; the complete scoped evidence file list is recorded above.
- **Native smoke limitation:** No fresh native-window smoke validation was performed in this implementation session and no native visual result is claimed. This preserves WO-001's documented limitation; the project manifest has no required manual/external gate. Fresh validation remains the next lifecycle step.
- **Scope boundary:** WO-003 edge correction was not implemented. No stop-propagation mechanism was introduced; the implementation uses separately allocated/interacted egui rectangles/responses.
- **Follow-up:** Fresh-context validation is required. The work order is `in-review`, not `complete`.

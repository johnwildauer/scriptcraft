# Work Order: WO-002 — Toolbar Group Hover and Color

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop toolbar implementer (fresh session)
- **Phase/order:** 2 of 2; depends on WO-001
- **Initiative:** [`../toolbar-overflow-hover-v1.md`](../toolbar-overflow-hover-v1.md:1)
- **Validation:** [`../validations/validation-WO-002.md`](../validations/validation-WO-002.md)
- **Implementer session/agent ID:** `code-implementer-toolbar-overflow-hover-2026-10-27T0511Z`
- **Session exception:** none

## Summary

Make the complete Item group the hover/click surface so its icon/button area illuminates as intended across the group instead of leaving the label outside the hover response. Switch the toolbar frame color source to `style.widgets.open.weak_bg_fill`.

## In scope

- Apply hover styling to the full outer Item response/group, keeping icon and label centered and preserving click dispatch.
- Keep label text color stable while the group background/interactive surface changes on hover.
- Change [`toolbar_chrome_color()`](../../../../apps/scriptcraft/src/main.rs:1367) to use the requested `style.widgets.open.weak_bg_fill` source.
- Add deterministic hover-response/style-source tests and update the desktop blueprint.

## Out of scope

- Text-aware width or allocation changes; WO-001 owns geometry.
- New commands, core, persistence, exports, page layout, or native OS title-bar sampling.

## Requirements

- **AC-WRITING-005.3:** “Action Items shall use icon-first, borderless button presentation with a lightness-only hover affordance and retain text labels for tooltip and accessibility use.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58) — desktop owns transient Item presentation and toolbar chrome.

## Implementation plan

1. Build the full group response from the final WO-001 bounds in [`toolbar_action_button()`](../../../../apps/scriptcraft/src/main.rs:434).
2. Apply a subtle hover fill to the group response without changing label color or semantic dispatch.
3. Switch the toolbar frame source to `style.widgets.open.weak_bg_fill` and add tests/documentation.

## Validation

- **Automated:** format, check, tests, build, Clippy with `-D warnings`, and `git diff --check`.
- **Acceptance:** Hovering any point within an Item group changes the group surface, not only glyph pixels; the toolbar source equals the requested open-widget weak fill; click/role behavior remains intact.

## Durable-document impact

- **Requirements impact:** `update`
- **Blueprint impact:** `update`
- **Rationale:** User-facing hover ownership and chrome color source change.

## Implementation evidence

- **Changed files:** [`main.rs`](../../../../apps/scriptcraft/src/main.rs:446), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:104), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:66), this work order, and [`execution-log.md`](../execution-log.md:1).
- **Implementation:** The action Item's allocated outer group now owns the click and hover response, paints the hovered-widget fill across the complete group rectangle, and renders the icon and unchanged-color label in the existing centered layout. Existing action dispatch, Item ordering, overflow routing, role semantics, and accessibility tooltip text are preserved. `toolbar_chrome_color()` now reads `style.widgets.open.weak_bg_fill`; no global style mutation was added.
- **Tests added/updated:** Deterministic open-widget style-source and outer-group hover-fill tests; the hover helper returns transparent for the non-hovered state and the hovered-widget fill for the hovered state without changing label text styling.
- **Durable-document evidence:** The authorized requirement now specifies the outer Item/group hover/click surface, and the desktop blueprint records the centered group affordance and `Widgets::open::weak_bg_fill` source. No native OS pixel-equality claim was added.
- **Required gate results:** Not run in this implementer session because the user explicitly prohibited fresh validation. The required sequence remains `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check`; fresh validation is required to record exact results.
- **Native smoke limitation:** No visible desktop test surface is available; no native hover/color or pixel-level OS chrome claim is made.
- **Diff evidence:** Changes are limited to the toolbar implementation/tests, the authorized requirement and blueprint wording, this evidence, and the execution log. WO-001 geometry behavior was not changed.

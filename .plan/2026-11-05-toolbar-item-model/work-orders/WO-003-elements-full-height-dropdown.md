# WO-003 — Elements full-height dropdown

- **Status:** Implementation submitted; user review and validation pending; not accepted
- **Initiative:** [`toolbar-item-model-v1.md`](../toolbar-item-model-v1.md)
- **Depends on:** [`WO-002`](WO-002-complete-temporary-toolbar-composition.md)
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or independent validators

## Outcome

Make the Elements role popup extend to the full measured height of its 17-entry list at ordinary usable window heights instead of stopping at a shorter popup region. Apply one consistent vertical sizing policy to both the central text dropdown and the nested Elements menu in toolbar overflow.

The only intentional height limit is the actual available viewport/popup bounds. On displays too short for the complete list, retain safe bounded scrolling or clipping rather than allowing inaccessible off-screen entries.

## Scope

1. Trace the active egui 0.34 popup path used by [`elements_toolbar_item()`](../../../apps/scriptcraft/src/toolbar.rs:406), including the stock [`ComboBox`](../../../apps/scriptcraft/src/toolbar.rs:423), its menu/scroll area, item spacing, and available screen rectangle. Record the actual source of the short height rather than assuming the 128-point closed-control width is responsible.
2. Replace any short default or fixed popup-height constraint with a content-derived desired height for all entries in [`SUPPORTED_SCREENPLAY_ROLES`](../../../apps/scriptcraft/src/toolbar.rs:100), including row heights, relevant menu spacing, and popup padding.
3. Use supported egui 0.34 popup/scroll APIs and avoid an arbitrary numeric height chosen to fit only the current 17 labels. The desired height should follow the actual entry count and active menu typography.
4. At ordinary usable heights, show the complete list without an unnecessary inner scroll region or a second short nested scrolling surface. At genuinely constrained heights, bound the popup to the usable viewport and provide a safe way to reach every role.
5. Preserve popup anchoring to the Elements control, selected-role display, current selected value, pointer and keyboard navigation, outside-click/Escape dismissal, default cursor behavior, and the closed Elements caption.
6. Apply the same content-height/viewport policy to the Elements submenu in [`toolbar_overflow_menu()`](../../../apps/scriptcraft/src/toolbar.rs:483). Do not fix only the central control while leaving the overflow role list short.
7. Keep role selection routed through [`change_role_from_toolbar()`](../../../apps/scriptcraft/src/toolbar.rs:401) and the existing transactional session operation. Rejection behavior, status/error handling, canonical text, selection, dirty state, and undo history remain unchanged.
8. Preserve the final WO-002 composition, disabled items, Material graphics, allocation, overflow trigger, closed-control width, toolbar panel height, and right-edge fitting. Opening a taller popup must not resize the toolbar or shift document layout.

## Non-goals

- No role additions, removals, ordering changes, role-policy changes, or replacement of the Elements interaction model.
- No fixed promise that all entries fit on every viewport regardless of physical height.
- No generic redesign of every application popup or menu.
- No new toolbar items, enablement changes, export behavior, customization, or persistence.
- No screenplay-core, FDX, pagination, editor, status-bar, or global theme changes.
- No new automated desktop-test target and no agent execution of validation.

## Structural acceptance for user review

1. The central Elements popup derives its desired vertical extent from the complete role list and active menu metrics, not from the closed control's 128-point width or an arbitrary short fixed height.
2. All 17 supported labels are simultaneously visible at ordinary usable window heights, subject only to real viewport constraints.
3. On a short viewport, every role remains reachable through bounded popup behavior without off-screen or negative geometry.
4. The overflow Elements submenu follows the same policy and all its role choices remain executable.
5. Current-role synchronization, transactional role changes, popup dismissal, keyboard behavior, toolbar geometry, disabled items, and enabled commands remain intact.

## Durable records

- Extend [`AC-WRITING-005.4`](../../../.forge/requirements/features/screenplay-writing-loop.md:534) with complete-list content-derived height, viewport-only limitation, constrained-viewport reachability, and overflow-submenu parity.
- Update the toolbar popup contract in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:20) with the measured-height and viewport-bound policy. Do not claim native behavior before user observation.
- Append the diagnosed popup-height source, exact implementation changes, and evidence limitations to this work order and [`toolbar-item-model-v1.md`](../toolbar-item-model-v1.md).

## User-owned review requests

- Open Elements at normal window sizes and confirm Scene Heading through Summary are all visible in the defined order without a short inner popup.
- Move the caret through representative roles, select replacements from the top, middle, and bottom of the list, and confirm display and document behavior remain transactional.
- Resize to a genuinely short window and confirm all roles remain reachable without popup overflow or toolbar/document resizing.
- Repeat through the narrow-toolbar overflow submenu; verify pointer and keyboard navigation and dismissal.
- Recheck disabled-item inertness, Material icons, flexible spacing, overflow fitting, and enabled commands for regressions.
- Run any desired workspace compilation and behavior checks and record outcomes separately; no agent result is claimed.

## Implementation handoff — 2026-11-05

WO-003 implementation is submitted for user review. This handoff does not mark
the work order complete, accepted, or validated. No subsequent work order was
started.

- **Checkout/branch:** Read-only `git status --short --branch` observed
  `dev/2026-11-05-toolbar-item-model`. Existing WO-001/WO-002 and initiative
  changes remained in place; nothing was staged, committed, merged, switched, or
  discarded.
- **Diagnosed egui 0.34 height limit:** egui 0.34.3's stock `ComboBox` resolves
  its menu height from `Spacing::combo_height` (200 logical points by default)
  and passes that value to the inner vertical `ScrollArea::max_height`. The
  central list therefore gets a 200-point scroll viewport, which is shorter
  than its 17 menu rows at the active menu metrics. The closed control's
  128-point width is unrelated. The overflow Elements `MenuButton` uses a
  `Popup::menu` without an automatically height-bounded role-list scroll area;
  the old submenu body had no shared role-list sizing policy.
- **Source changes:** Added one Elements role-list height helper in
  `apps/scriptcraft/src/toolbar.rs`. It sums each supported label's active Button
  font galley height against egui's interaction minimum and vertical button
  padding, adds the active inter-row spacing, then caps that desired inner scroll
  height at the egui content viewport height less the actual popup frame margins.
  The central ComboBox now receives that value through its supported `.height()`
  API; the overflow submenu wraps the same 17 choices in a vertical ScrollArea
  with the same computed cap. Thus ordinary viewports request the complete
  measured content, while short viewports remain scrollable within safe bounds.
  Existing anchors, role labels/order and selection state, close/keyboard
  routing, `change_role_from_toolbar()` transactional session dispatch, closed
  caption/control sizing, toolbar allocation, and document layout code were not
  intentionally changed.
- **Authorized durable records:** Extended only AC-WRITING-005.4 with the
  content-derived full-list/viewport-bounded role-menu behavior and overflow
  parity. Updated the toolbar popup contract in `technical-context.md` with the
  egui content-viewport and frame-margin cap. WO-001 and WO-002 handoffs and
  their context remain preserved.
- **Evidence limitations and uncertainty:** No build, check, test, lint,
  formatting command, manual verification, review, or validator was run. The
  height diagnosis is based on the installed egui 0.34.3 source: ComboBox uses
  the 200-point default `Spacing::combo_height` as its `ScrollArea::max_height`.
  The implementation has not been compiled or exercised; popup positioning,
  scroll reachability, keyboard/dismissal behavior, full-list visibility,
  viewport-edge behavior, and preservation of toolbar/document geometry remain
  unverified and user-owned. No automated coverage was added.

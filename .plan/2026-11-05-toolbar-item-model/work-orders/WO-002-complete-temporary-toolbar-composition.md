# WO-002 — Complete temporary toolbar composition

- **Status:** Implementation submitted; user review and validation pending; not accepted
- **Initiative:** [`toolbar-item-model-v1.md`](../toolbar-item-model-v1.md)
- **Depends on:** [`WO-001`](WO-001-declarative-toolbar-items-and-presentation.md)
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or independent validators

## Outcome

Expand the WO-001 catalog-driven renderer to the complete Toolbar Temporary composition. Show/Hide and Export Text are visible, subdued, and entirely inert. Replace the literal double-arrow overflow trigger with the assigned outlined Material icon while preserving leftmost-fit allocation and executable overflow entries.

## Required composition

The ordered catalog composition in [`toolbar_items.rs`](../../../apps/scriptcraft/src/toolbar_items.rs) must contain:

1. Flexible Space with a unique leading identity
2. New
3. Open
4. Save
5. Save As
6. Flexible Space with a unique pre-Elements identity
7. Elements
8. Flexible Space with a unique post-Elements identity
9. Show/Hide, disabled
10. Export Text, disabled
11. Export PDF
12. Flexible Space with a unique trailing identity
13. Overflow Indicator, conditionally rendered only when allocation has overflow entries

The four Flexible Spaces divide the post-budget remainder equally at widths where every ordinary item fits. At narrow widths they remain excluded from overflow and do not consume slots, matching the existing allocation policy. The conditional overflow entry must not add permanent width or appear as a visible command when nothing overflows.

## Scope

1. Replace the intermediate eight-entry composition with the complete ordered composition above. Keep all definitions in [`toolbar_items.rs`](../../../apps/scriptcraft/src/toolbar_items.rs); do not reconstruct the order in [`toolbar()`](../../../apps/scriptcraft/src/toolbar.rs:521).
2. Add Show/Hide with an icon-dropdown graphic containing independently specified outlined Visibility and Keyboard Arrow Down icons. Its caption remains Show/Hide. It has no assigned activation behavior in this initiative.
3. Add Export Text with the outlined File Export icon and its visible caption. Leave it disabled and do not call [`export_text_with_layout_inputs()`](../../../crates/scriptcraft-core/src/exports.rs:52), open a file dialog, write a file, or alter status/dirty state.
4. Render every graphic and caption of both disabled items through WO-001's disabled foreground resolver. The default result must visibly reduce alpha relative to the enabled foreground while retaining legibility across all Appearance choices.
5. Disabled items must have no hover fill, hover foreground change, pointer activation, Enter/Space activation, focus acquisition, popup, command dispatch, or misleading enabled accessibility state. Merely painting a disabled color over an active response is insufficient.
6. If either disabled item moves into the text overflow menu, show its label with the same disabled color and no hover/activation behavior. It must not become enabled merely because its presentation changed from toolbar graphic to menu entry.
7. Replace [`toolbar_overflow_menu()`](../../../apps/scriptcraft/src/toolbar.rs:483)'s literal double-arrow trigger with the catalog's outlined Double Arrows Material icon. Use an icon-only visible control with a stable accessible name such as Toolbar overflow; retain the existing bounded overflow allocation and menu contents.
8. Give the enabled overflow indicator the ordinary toolbar icon hover treatment without adding a visible caption. Preserve default cursor, two-point corner radius, and the existing overflow popup routing.
9. Use the generic WO-001 measurement path for Show/Hide's two graphics and every caption. Keep the shared allocation/render width identical and preserve the actual clipped right-edge guarantee.
10. Preserve all enabled New/Open/Save/Save As/Export PDF actions and Elements role behavior exactly. Do not repair unrelated command-safety gaps or add new behavior under this presentation order.

## Temporary icon assignments

| Item | Graphic | State |
|---|---|---|
| New | Outlined Draft | Enabled |
| Open | Outlined File Open | Enabled |
| Save | Outlined Save | Enabled |
| Save As | Outlined Save As | Enabled |
| Elements | Existing selected-role text dropdown | Enabled |
| Show/Hide | Outlined Visibility plus independently styled outlined Keyboard Arrow Down | Disabled |
| Export Text | Outlined File Export | Disabled |
| Export PDF | Outlined Picture As Pdf | Enabled |
| Overflow Indicator | Outlined Double Arrows; no visible caption | Enabled only when shown |

Use typed constants from [`egui_material_icons`](../../../apps/scriptcraft/Cargo.toml:10). Retain the accepted Regular-weight implementation and do not introduce filled variants or weight-selection claims.

## Non-goals

- No Show/Hide choices, menu, visibility setting, or screenplay behavior.
- No plain-text export route, destination, file write, loss-report presentation, or status behavior.
- No future Toolbar WIP entries, drag-and-drop customization, item persistence, or dynamic user enablement.
- No Elements dropdown-height correction; WO-003 owns it.
- No screenplay-core, FDX, editor, pagination, status-bar, document-font, or global Appearance changes.
- No new automated desktop-test target and no agent execution of validation.

## Structural acceptance for user review

1. The visible wide-width order matches the reference, with four equal-share Flexible Spaces and no permanent overflow indicator.
2. Show/Hide displays both assigned icons and its caption; Export Text displays its assigned icon and caption; both use the disabled foreground in every Appearance.
3. Neither disabled item responds to hover, pointer, Enter, or Space in either the main toolbar or overflow menu.
4. Export PDF and every pre-existing enabled command retain behavior. No plain-text file or Show/Hide state can be produced by the disabled controls.
5. Narrow widths retain leftmost fitting items, omit Flexible Spaces from overflow, show the outlined Double Arrows trigger only when needed, and retain enabled overflow actions and Elements choices.
6. Complete graphic/caption measurement keeps all visible content inside the same allocated item widths and the toolbar's clipped content edge.

## Durable records

- Revise [`AC-WRITING-005.1`](../../../.forge/requirements/features/screenplay-writing-loop.md:523) to the exact complete temporary order and conditional overflow behavior.
- Extend [`AC-WRITING-005.3`](../../../.forge/requirements/features/screenplay-writing-loop.md:530) with the disabled icon/caption and no-hover/no-activation contract.
- Reconcile [`AC-WRITING-005.5`](../../../.forge/requirements/features/screenplay-writing-loop.md:539) with four Flexible Spaces, complete graphic measurement, disabled overflow labels, and the Material overflow trigger instead of the literal text trigger.
- Update the toolbar catalog paragraph in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:20) with final composition and disabled/overflow behavior.
- Annotate the implemented assignments and two disabled entries in [`icon_assignement.md`](../../../.reference/icon_assignement.md:12) without changing the future WIP table.
- Append exact implementation details and evidence limitations to this work order and [`toolbar-item-model-v1.md`](../toolbar-item-model-v1.md).

## User-owned review requests

- Compare the complete visible order and spacing with the Toolbar Temporary table at wide and narrow widths.
- Hover, click, Tab to, and press Enter/Space on Show/Hide and Export Text in both direct and overflow presentations; confirm no visual hover or behavior occurs.
- Review disabled legibility in System, Day, Night, and Midnight appearances; adjust the global disabled color/opacity and individual icon overrides to confirm resolution precedence.
- Open overflow and activate every enabled action and Elements role choice; verify disabled labels remain inert.
- Confirm the overflow icon appears only when required and retains an accessible name without a visible caption.
- Run any desired workspace compilation and behavior checks and record outcomes separately; no agent result is claimed.

## Implementation handoff — 2026-11-05

WO-002 implementation is submitted for user review. This handoff does not mark
the work order complete, accepted, or validated. WO-003 has not started.

- **Checkout/branch:** `git status --short --branch` observed
  `dev/2026-11-05-toolbar-item-model`. The pre-existing WO-001 and initiative
  files remained modified/untracked as already recorded; no work was staged,
  committed, merged, switched, or discarded.
- **Catalog and renderer:** `toolbar_items.rs` now owns the ordered 13-entry
  composition: leading Flexible Space; New; Open; Save; Save As; pre-Elements
  Flexible Space; Elements; post-Elements Flexible Space; Show/Hide; Export
  Text; Export PDF; trailing Flexible Space; and the catalog-only overflow
  indicator. The four spaces have unique IDs. The allocator excludes the
  indicator from the all-fit width and displays it only when non-spacing items
  actually overflow; at narrow widths Flexible Spaces remain excluded from
  overflow and consume no slots. The existing leftmost-fit policy and overflow
  reservation remain in use.
- **Disabled presentation and interaction:** Show/Hide uses independently
  specified outlined Visibility and Keyboard Arrow Down icons; Export Text uses
  the typed outlined `ICON_DRIVE_EXPORT` Material glyph for the File Export
  assignment. Both items use the disabled resolver for each icon and caption.
  Their toolbar response is hover-only with disabled `WidgetInfo`; there is no
  hover fill/foreground change, focus, click/keyboard activation, popup, or
  command behavior. In overflow they are disabled-color plain labels, not
  buttons or menu actions. No Show/Hide behavior or plain-text export route was
  added. The installed Material crate exposes `ICON_DRIVE_EXPORT` rather than a
  literal `ICON_FILE_EXPORT`, so the assigned File Export glyph mapping remains
  subject to user appearance review.
- **Overflow and preserved routing:** The catalog overflow spec uses the
  outlined `ICON_DOUBLE_ARROW` Material glyph with no visible caption and the
  accessible name “Toolbar overflow.” Its control uses the toolbar's default
  cursor, hover fill, and two-point corner radius. Existing enabled actions,
  Elements role selection, popup contents, and command/session adapters remain
  routed through their existing paths.
- **Authorized durable records:** Updated AC-WRITING-005.1, AC-WRITING-005.3,
  and AC-WRITING-005.5 in `screenplay-writing-loop.md`; replaced the intermediate
  composition description in `technical-context.md`; and added the implementation
  assignment note to `icon_assignement.md` without changing the Toolbar (WIP)
  table. No other durable policy file was changed.
- **Evidence limitations and unresolved review:** No build, check, test, lint,
  formatting command, manual verification, review, or validator was run. The
  Material glyph appearance/mapping, disabled interaction/accessibility behavior,
  Appearance legibility, allocation/clipping, overflow presentation, and all
  preserved command behavior remain unverified and user-owned for review and
  validation. No automated test target or test coverage was added.

# WO-004 — Declarative menu catalog and preview policy

- **Status:** Implementation submitted; user validation pending
- **Initiative:** [`menu-system-v1.md`](../menu-system-v1.md)
- **Depends on:** [`WO-003`](WO-003-canonical-clipboard-commands.md)
- **Validation owner:** User; no agent-run checks, reviews, or validators

## Single outcome

Define the complete initial File/Edit catalog once and establish a development policy that keeps the in-window Windows-style preview enabled on macOS.

## Scope

1. Create [`menu_items.rs`](../../../apps/scriptcraft/src/menu_items.rs) with ordered top-level menu and entry specifications, stable string identities, labels/accessibility names, optional semantic icons, logical shortcuts, and shared command identities.
2. Encode exactly the uncommented entries in [`initial-menus.md`](../../../.reference/initial-menus.md): File groups New/Open.../Close and Save/Save As.../Save As PDF...; Edit groups Undo/Redo, Cut/Copy/Paste, and Delete/Select All.
3. Remove empty groups and dangling/duplicate separators caused by commented reference entries. Do not implement submenus or any commented feature.
4. Map Save As PDF... to the shared existing PDF-export command. Use Shift+Primary+Z for advertised Redo and retain Primary+Y only as an unadvertised Windows alias.
5. Create [`menu.rs`](../../../apps/scriptcraft/src/menu.rs) with a documented source switch named for the in-window preview, initially set to true. When true, macOS shows the egui strip alongside the native menu once WO-008 installs it; Windows always shows the egui strip.
6. Keep native macOS enablement independent of this preview switch. Automatic production OS-only selection and turning the preview off are deferred beyond this batch.
7. Define a single pending-command queue handoff for later egui/native activation. Do not render a panel or construct native items in this order.

## Structural acceptance for user review

- Adding a top-level menu or ordinary item is a catalog edit, not a renderer composition edit.
- Enablement remains application-derived rather than static catalog data.
- Every initial item maps to an implemented shared command.
- The preview switch is true and no production-switch-off work is included.

## User-owned behavior review

- Compare the catalog mechanically with the visible reference entries, labels, order, grouping, and shortcuts.
- Verify no empty separator groups, submenu placeholders, dynamic Undo action labels, or additional top-level menus appear.
- Inspect that Windows and macOS policy both retain the in-window preview during this development batch.

## Handoff requirements

Record the catalog schema, stable IDs, initial switch name/value, and deferred automatic-production decision. No visual or native behavior is claimed by this order.

## WO-004 implementation handoff — 2026-11-06

- **State and checkout:** Implementation is submitted for user review and validation only; WO-004 is not marked accepted or validated. Work remained on the existing `dev/2026-11-06-menu-system` branch. Existing WO-001–WO-003 source and all pre-existing work were preserved. No branch was created, and no work was stashed, discarded, staged, or committed. WO-005 and later orders have not started.
- **Catalog schema and stable IDs:** Added [`menu_items.rs`](../../../apps/scriptcraft/src/menu_items.rs) with an ordered `MENU_CATALOG` of `MenuSpec { id, label, accessibility_name, entries }`. Entries are `Item(MenuItemSpec)` or `Separator`; each item has stable `id`, exact visible `label`, `accessibility_name`, optional `SemanticIcon`, optional `LogicalShortcut { key, primary, shift }`, a separately represented `windows_alias`, and the shared `AppCommand`. The menu IDs are `file` and `edit`. File item IDs, in order, are `file.new`, `file.open`, `file.close`, `file.save`, `file.save-as`, `file.save-as-pdf`; Edit item IDs are `edit.undo`, `edit.redo`, `edit.cut`, `edit.copy`, `edit.paste`, `edit.delete`, `edit.select-all`. Separators occur only between the three declared nonempty reference groups; there are no submenus or trailing/empty groups. Save As PDF maps to the existing `AppCommand::ExportPdf`. Advertised Redo is Primary+Shift+Z; `edit.redo` alone carries Primary+Y in `windows_alias`, which is explicitly not advertised.
- **Shared command identity integration:** Added `Cut`, `Copy`, and `Paste` variants to the shared `AppCommand` vocabulary in [`app_commands.rs`](../../../apps/scriptcraft/src/app_commands.rs), matching the existing clipboard dispatcher arms and enabling every catalog row to refer to its shared command.
- **Policy and handoff:** Added [`menu.rs`](../../../apps/scriptcraft/src/menu.rs). Its documented source switch is `IN_WINDOW_MENU_PREVIEW_ENABLED = true`. `in_window_menu_enabled()` returns true unconditionally for Windows, follows that switch on macOS, and is false for other targets. `NATIVE_MACOS_MENU_ENABLED` is independently derived from the macOS target and does not depend on the preview switch. Automatic production OS-only switching remains deferred; the preview was not disabled. `PendingMenuCommands` is the single FIFO `VecDeque<AppCommand>` stored in `ScriptCraftApp`: future egui and native activation producers enqueue into it, and `dispatch_pending_menu_commands()` detaches and dispatches pending commands in activation order through the shared dispatcher, which rechecks fresh availability. No activation producer or queue drain call was added in this order.
- **Changed paths and scope boundary:** Added `apps/scriptcraft/src/menu_items.rs` and `apps/scriptcraft/src/menu.rs`; updated [`main.rs`](../../../apps/scriptcraft/src/main.rs) to register the modules and own the pending queue, and [`app_commands.rs`](../../../apps/scriptcraft/src/app_commands.rs) for the shared clipboard command variants. Updated this order and [`menu-system-v1.md`](../menu-system-v1.md). No menu UI was rendered, no native menu was constructed, and shortcuts were not consolidated.
- **Evidence limitations:** No build, check, test, lint, formatting command, application launch, manual verification, review, or independent validator was run, as required by repository policy. Catalog compilation, structural behavior, platform policy at runtime, and queue integration are therefore unverified; no visual/native behavior or acceptance is claimed. The user owns validation.

### WO-010 integrated-source reconciliation

`IN_WINDOW_MENU_PREVIEW_ENABLED` remains `true`. Windows always uses the in-window catalog and does
not construct the macOS backend; macOS keeps the in-window File/Edit preview clickable alongside
the native Application/File/Edit menus. Native macOS owns accelerators while installed, including
dual preview. Automatic production OS-only selection and switch-off remain deferred and were not
implemented by WO-010. Platform runtime behavior remains unverified.

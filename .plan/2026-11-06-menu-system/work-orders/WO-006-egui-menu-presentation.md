# WO-006 — Reference-aligned egui menu presentation

- **Status:** Implementation submitted; user validation pending
- **Initiative:** [`menu-system-v1.md`](../menu-system-v1.md)
- **Depends on:** [`WO-005`](WO-005-egui-menu-panel-and-activation.md)
- **Validation owner:** User; no agent-run checks, reviews, or validators

## WO-006 implementation handoff — 2026-11-06

- **State and checkout:** WO-006 implementation is submitted for user review and validation only; it is not accepted or validated. Work stayed on the existing `dev/2026-11-06-menu-system` branch. Existing source changes and untracked plan/reference artifacts were preserved; no branch or commit was created. The in-window preview switch remains `true`. WO-007 has not started.
- **Changed paths and boundaries:** Updated [`menu.rs`](../../../apps/scriptcraft/src/menu.rs), this work order, and the initiative plan. The existing `main.rs` panel still consumes `MENU_PANEL_HEIGHT`, now sourced from menu-local metrics. No toolbar, status, document typography, command routing, keyboard shortcut ownership, native-menu, or global-theme implementation was changed.
- **Initial agent-default menu metrics (logical points):** Bar height **28**; menu-bar text **15**; menu-bar item gap **8 × 0**; item-label text **14**; shortcut text **13**; icon glyph **18**; row height **28**; fixed icon slot **28**; menu-bar button padding **8 × 0**; row button padding **4 × 0**; row atom spacing **6 × 2**; popup padding **10 horizontal × 8 vertical**; separator spacing **9** with **1**-point stroke; menu-bar corner radius **7**, row radius **4**, popup radius **8**. The vertical popup inset contracts with the usable content viewport for very short windows. These are implementation defaults, not measurements or pixel-parity claims. Separator color, row enabled/hovered/disabled colors, focused outline, and popup fill/border follow active egui/application visuals rather than fixed RGB values.
- **Icon assignments:** The catalog's typed semantic assignments render through the existing app-owned native-font adapter using its default filled face and axes (weight **W200**, optical size **24**, grade **0**): New → Material `ADD`; Open → `FOLDER_OPEN`; Close → `CLOSE`; Save → `SAVE`; Save As → `SAVE_AS`; Save As PDF → `PICTURE_AS_PDF`; Undo → `UNDO`; Redo → `REDO`; Cut → `CONTENT_CUT`; Copy → `CONTENT_COPY`; Paste → `CONTENT_PASTE`; Delete → `DELETE`; Select All → `SELECT_ALL`. The optional icon atom uses the same fixed slot dimensions when absent; all current catalog entries have an icon, so the blank-slot path is implemented but not currently exercised by a catalog row.
- **Rows and theme behavior:** Egui `Button` atoms now provide the fixed left icon atom, label, and built-in weak `shortcut_text` grow/right alignment; even rows with no shortcut use an empty shortcut atom so the columns remain shared. Full-row buttons retain framework-owned interaction and accessibility metadata. Disabled availability still goes through `add_enabled`; active theme visuals provide enabled, hover, and disabled treatments. A focused row receives an outline using the active selection stroke. Logical shortcuts are unchanged and the preview keeps Windows `Ctrl` labels on every OS.
- **Popup sizing and framework limitations:** The existing egui `MenuBar`/`menu_button` popup owns opening, ordinary dismissal, and menu interaction. Popup content is wrapped in vertical `ScrollArea` with a maximum based on `Context::content_rect()` height minus vertical popup insets and frame allowance; popup insets shrink for very short content viewports. `min_scrolled_height(0)` plus the framework's visible-when-needed scrollbar policy keeps scrolling dormant when the contents fit. The popup remains naturally sized in width; no independent popup-width cap or custom narrow-window reflow was added. Egui has no separate row-focus widget palette state in this adapter, so focus is represented by the selection-stroke outline while keyboard navigation remains framework-owned. Framework separator spacing/stroke is themed, but its line spans the popup's padded content width rather than using a separate measured reference inset.
- **Visual caveats:** [`file.png`](../../../.reference/file.png) depicts Final Draft's dark macOS presentation and Command-symbol shortcuts. This preview intentionally shows Ctrl labels and follows ScriptCraft's active Appearance mode, so Day mode and other theme colors differ by design. Icon identities, font metrics, row/popup geometry, bar height, popup width, and scale behavior remain agent-default approximations pending side-by-side user review; no pixel parity is claimed.
- **Evidence limitations:** No build, check, test, lint, formatting command, launch, manual verification, review, or independent validator was run. Compilation, runtime focus/scroll behavior, exact geometry, theme contrast, click bounds, narrow/short-window behavior, and visual matching remain unverified and user-owned. This handoff records source intent only.

### WO-010 integrated-source reconciliation

The initial metrics above remain the source values: bar height 28; bar/item/shortcut text 15/14/13;
icon size, row height, and icon slot 18/28/28; bar gap 8×0; bar and row padding 8×0 and 4×0; row
spacing 6×2; popup padding 10×8; separator spacing/stroke 9/1; and corner radii 7/4/8 logical
points. They remain agent defaults pending user review, not reference measurements. Popup width is
content-driven without custom narrow reflow; no pixel parity or visual acceptance is claimed.

## Single outcome

Make the in-window menus closely follow the supplied reference's icon column, label column, separators, and subdued right shortcut column without changing menu behavior.

## Scope

1. Extend [`menu.rs`](../../../apps/scriptcraft/src/menu.rs) with menu-local presentation metrics for bar height, text and icon sizes, row height, icon-slot width, popup padding, separator treatment, and corner radius. Mark initial values as agent-defaults.
2. Use egui button atoms or the equivalent framework row adapter to reserve a fixed optional-icon slot, place the label after it, and show weak right-aligned shortcut text through the built-in shortcut-text support.
3. Render suitable typed Material icons through the existing app-owned native-font adapter in [`material_font.rs`](../../../apps/scriptcraft/src/material_font.rs). Reserve equivalent blank space when no icon is assigned.
4. Style enabled, hovered, focused, and disabled rows from the active application/egui palette. Shortcut text remains lower emphasis and disabled rows remain noninteractive.
5. Display Windows-style Ctrl shortcut labels in the in-window preview, including while it runs on macOS. Keep the logical shortcut definitions unchanged.
6. Bound popup height to the usable egui content viewport and retain framework scrolling only when genuinely necessary. No submenu caret or custom submenu implementation is added.
7. Do not alter toolbar/status numeric presentation, document typography, global panel geometry outside the menu strip, or command routing.

## Structural acceptance for user review

- Every row shares aligned icon, label, and shortcut columns, including rows without icons.
- Full-row interaction and accessible labels remain framework-owned.
- Menu-local settings do not leak into toolbar, status, or Script Workspace styling.
- The catalog remains the only source of item content and order.

## User-owned behavior review

- Compare File/Edit popups side by side with [`file.png`](../../../.reference/file.png) in every Appearance mode.
- Review icon identity/alignment, shortcut opacity, separator weight, disabled contrast, hover/focus treatment, popup width, and row hit bounds.
- Check long shortcut labels, narrow windows, short viewports, and representative native display scales.
- Recheck all WO-005 activation behavior for regressions.

## Handoff requirements

Record every initial metric, icon assignment, framework limitation, and visual difference from the reference. Pixel parity is not claimed.

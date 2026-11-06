# WO-005 — Egui menu panel and activation shell

- **Status:** Implementation submitted; user validation pending
- **Initiative:** [`menu-system-v1.md`](../menu-system-v1.md)
- **Depends on:** [`WO-004`](WO-004-menu-catalog-and-preview-policy.md)
- **Validation owner:** User; no agent-run checks, reviews, or validators

## WO-005 implementation handoff — 2026-11-06

- **State and checkout:** Implementation is submitted for user review and validation only; it is not accepted or validated. Work remained on the existing `dev/2026-11-06-menu-system` branch. Earlier initiative work and pre-existing untracked files were preserved; no branch or commit was created. The preview switch remains enabled. WO-006 and later orders have not started.
- **Renderer and panel:** [`menu.rs`](../../../apps/scriptcraft/src/menu.rs) renders every top-level menu and catalog entry from `MENU_CATALOG` through egui `Ui::menu_button`, `Ui::add_enabled`, `Ui::separator`, and `Ui::close`; the framework owns popup focus, ordinary dismissal, and popup interaction behavior. Enabled state is projected from current `AppCommandAvailability`; unavailable Edit items are disabled. A pointer click enqueues one shared command, closes the popup, and requests repaint. Basic Windows-style `Ctrl` shortcut text appears as weak right-side row content, including on macOS preview. No native-menu code was added.
- **Panel and composition:** [`main.rs`](../../../apps/scriptcraft/src/main.rs) inserts an egui top panel immediately before `titlebar-toolbar` only when `in_window_menu_enabled()` is true. `MENU_PANEL_HEIGHT` is exactly **28 logical points**; the panel paints its own lower divider. The existing toolbar height/boundary, Script Workspace, status panel, sprint timer, and Focus Mode composition were left unchanged. The existing migration-modal branch omits application panels while consent is pending.
- **Queue drain and freshness:** Catalog pointer activations enter the existing FIFO. `dispatch_pending_menu_commands()` now drains at the beginning of `ScriptCraftApp::update()`, immediately after native-window-state update and before ordinary editor input routing. Since the panel renders later in the same update, a click is dispatched on the next update. Each command then passes through the shared dispatcher, which recomputes availability; this retains edit-command migration gating and rejects stale unavailable Edit commands.
- **Deferred presentation and focus work:** Shortcut text is intentionally basic and not column-aligned; there are no icon slots, detailed theme-local menu metrics, reference-calibrated popup/row styling, keyboard shortcut interception/ownership changes, or explicit editor-focus restoration. Those remain for WO-006/WO-007. Keyboard navigation and dismissal are delegated to egui but have not been behavior-reviewed.
- **Changed paths and integration inspection:** Updated [`menu.rs`](../../../apps/scriptcraft/src/menu.rs), [`main.rs`](../../../apps/scriptcraft/src/main.rs), this order, and the initiative plan. A source search found no remaining `ToolbarAction` or `dispatch_toolbar_action` references in `apps/scriptcraft/src`; toolbar and other application composition paths were not otherwise changed.
- **Evidence limitations:** Repository policy prohibits agent-run builds, checks, tests, lint, formatting, manual verification, reviews, and independent validators; none were run. Consequently, compilation, runtime popup behavior, exact rendered geometry, activation counts, modality, and visual appearance remain unverified. This handoff records source changes only and does not claim acceptance or behavior verification.

### WO-010 integrated-source reconciliation

The 28-point menu panel remains immediately before the toolbar, with the panel order menu → toolbar →
Script Workspace and the existing bottom status panel. Native handler IDs are drained into the same
FIFO before its per-update dispatch; egui clicks queued during panel rendering are handled on a later
update. Availability is rechecked at dispatch. This final flow does not establish click counts,
runtime behavior, or acceptance; user evidence remains pending.

## Single outcome

Show functional File and Edit popups in an egui panel immediately above the toolbar, using the shared catalog and pending-command dispatcher.

## Scope

1. Implement the basic in-window renderer in [`menu.rs`](../../../apps/scriptcraft/src/menu.rs) using the active egui menu-button/popup facilities rather than hand-rolling popup focus and dismissal.
2. Insert an exact-height top panel before [`titlebar-toolbar`](../../../apps/scriptcraft/src/main.rs:1065) in [`show_application_panels()`](../../../apps/scriptcraft/src/main.rs:1051) only when the in-window preview policy is active.
3. Render every catalog top-level label and ordinary item with its current availability. Queue selected commands once, request repaint, and close the activated popup.
4. Drain queued commands at one defined application-update boundary, rechecking current availability before dispatch. Avoid executing a command repeatedly while rendering a popup.
5. Keep the migration modal's command gating intact. Preserve existing toolbar, Script Workspace, status, timer, and Focus Mode composition.
6. Provide functional labels, separators, and basic right shortcut text sufficient for behavior review. WO-006 owns detailed visual alignment and theme metrics.
7. Leave keyboard shortcut interception and menu-focus restoration to WO-007.

## Structural acceptance for user review

- Panel order is menu, toolbar, Script Workspace, with no permanent panel when the preview is disabled on macOS.
- Popup content comes only from the shared catalog.
- Pointer activation enqueues and dispatches each command once through the shared path.
- Toolbar height and lower-boundary behavior are not changed.

## User-owned behavior review

- Open each menu and activate every enabled item, including after document replacement and while a popup has taken focus from the editor.
- Confirm disabled Undo/Redo and selection-dependent Cut/Copy cannot activate.
- Check popup dismissal, outside clicks, narrow windows, migration-modal gating, and basic keyboard navigation supplied by egui.
- Verify no duplicate file actions or edits occur.

## Handoff requirements

Record panel height, queue drain point, framework menu APIs, and any focus or visual limitations explicitly deferred to WO-006/WO-007.

# WO-007 — Shortcut ownership and menu-command focus

- **Status:** Implementation submitted for user review; not accepted or validated
- **Initiative:** [`menu-system-v1.md`](../menu-system-v1.md)
- **Depends on:** [`WO-006`](WO-006-egui-menu-presentation.md)
- **Validation owner:** User; no agent-run checks, reviews, or validators

## Single outcome

Route in-window menu shortcuts through the shared dispatcher exactly once and make menu-initiated editing independent of transient egui popup focus.

## Scope

1. Extend [`raw_input_hook()`](../../../apps/scriptcraft/src/main.rs:1213) to recognize pressed logical menu shortcuts only when the in-window backend owns accelerators.
2. Enqueue recognized commands and remove consumed key events before ordinary editor dispatch. Preserve unrelated keys, IME, text, paste, scrolling, and existing native page-zoom handling.
3. Remove duplicated command-key handling from [`key_event()`](../../../apps/scriptcraft/src/script_view.rs:754). Retain ordinary Delete, navigation, Return, Tab, and non-menu editing behavior there.
4. Support all advertised shortcuts plus the unadvertised Windows Primary+Y Redo alias. Do not intercept key releases or unrelated modifier combinations.
5. Define the single editing target as the application screenplay session rather than whichever widget currently holds egui focus. After an editing command, restore focus to the recorded editor surface when available and request the appropriate reveal/repaint behavior.
6. Recheck availability when consuming queued shortcuts, including migration-modal state and selection/history changes since the input frame.
7. Until WO-009 adds native arbitration, document the temporary in-window ownership state. The final dual-preview rule will make native macOS the sole accelerator owner.

## Structural acceptance for user review

- One keypress creates at most one shared command and one semantic mutation/file action.
- Menu pointer activation and menu shortcuts both work after the editor loses egui focus to a popup.
- Existing non-menu editor keys and zoom input are not consumed.
- Native macOS and in-window ownership are separately selectable for WO-009.

## User-owned behavior review

- Exercise every advertised shortcut from the editor, toolbar, status controls, and an open menu where the OS/framework permits.
- Check Shift+Primary+Z and Primary+Y, key releases, repeated keypresses, IME sessions, and migration-modal gating.
- Verify selection restoration/reveal after Undo/Redo, Cut, Paste, Delete, and Select All.
- Confirm no double Undo/Redo, Paste, Save, or New action.

## Handoff requirements

Record consumed-event rules, focus restoration behavior, temporary ownership limitations, and any framework-specific keyboard discrepancies.

## WO-007 implementation handoff — 2026-11-06

- **State and checkout:** WO-007 implementation is submitted for user review and validation only; it is not accepted or validated. Work stayed on the existing `dev/2026-11-06-menu-system` branch. Existing WO-001 through WO-006 source changes and untracked plan/reference artifacts were preserved. No branch or commit was created. The in-window preview switch remains `true`. WO-008 and later work have not started.
- **Shortcut routing and exact consumption:** `raw_input_hook()` resolves logical key plus event-local modifiers against the declarative catalog. It queues the mapped shared command and removes only a matching `Event::Key { pressed: true, .. }` from `RawInput.events`. Advertised Primary shortcuts use Command on macOS and Ctrl on non-macOS targets; Shift must exactly match the catalog, Alt is rejected, and the other platform's Command/Ctrl modifier is rejected. Windows additionally accepts Primary+Y for Redo without advertising it. Key releases, unrecognized keys, Delete/navigation/Return/Tab, and all other events remain untouched. A pressed repeat event is handled once per event; there is no extra key-repeat suppression. Availability is not used to decide consumption; the queued dispatcher rechecks current session selection/history and migration-modal availability at dispatch time, so stale/unavailable commands do not mutate.
- **IME and event preservation:** Shortcut extraction is suspended while the app's IME state is active and for any raw-input frame carrying an IME event. It does not remove text, paste, IME, wheel/scroll, or other events. Existing native `Event::Zoom` page-zoom processing and removal remains unchanged. Recognized keydown events alone are consumed; releases are preserved.
- **Edit target and focus:** Every shared Edit command continues to operate on `ScriptCraftApp.session`, independent of egui's current popup/widget focus. After each queued Edit command, including a command that has become unavailable, the app requests focus for the recorded editor-surface ID when one exists and requests repaint. Existing mutation paths set conditional caret/selection reveal intent for Undo, Redo, Cut, Paste, Delete, and Select All; Copy leaves selection/reveal unchanged. Egui surface response updates focused state when the surface is next rendered. If no editor ID has yet been recorded, there is no focus target to request.
- **Temporary ownership limitation:** WO-007 assigns accelerator ownership to the enabled in-window menu presentation on both Windows and macOS, including the macOS dual preview. `NATIVE_MACOS_MENU_ENABLED` remains only a target-policy indicator: WO-008 has not constructed a native menu and WO-009 has not wired native event/accelerator arbitration. This temporary ownership must be replaced by native-only macOS ownership in WO-009; do not interpret it as the final dual-preview rule.
- **Changed paths:** `apps/scriptcraft/src/main.rs` (raw input hook), `apps/scriptcraft/src/menu.rs` (catalog shortcut matching, temporary ownership, queued focus restoration), `apps/scriptcraft/src/app_commands.rs` (editor focus request helper), `apps/scriptcraft/src/script_view.rs` (remove duplicate command-key dispatch), this work order, and the initiative plan.
- **Evidence limitations:** No build, check, test, lint, formatting, launch, manual verification, review, or independent validator was run, as required by repository policy. Shortcut matching, framework modifier values, focus transfer, event interaction with IME/native input, runtime platform ownership, and command behavior remain unverified and require user-owned validation. This handoff is an implementation submission only, not acceptance or a runtime claim.

### WO-010 integrated-source reconciliation

The temporary WO-007 ownership described above was superseded by WO-009: macOS native menu
accelerators are now the sole owner whenever that backend is installed, even while the in-window
preview remains visible; Windows retains in-window accelerator ownership. Raw-input interception
consumes recognized pressed events only when that backend owns shortcuts and preserves IME/unmatched
input. Edit commands target the application session and request focus restoration when an editor ID
has been recorded. These source facts remain unverified at runtime.

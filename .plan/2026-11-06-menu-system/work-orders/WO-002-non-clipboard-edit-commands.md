# WO-002 — Non-clipboard Edit commands and availability

- **Status:** Implementation submitted; user validation pending
- **Initiative:** [`menu-system-v1.md`](../menu-system-v1.md)
- **Depends on:** [`WO-001`](WO-001-shared-file-command-dispatcher.md)
- **Validation owner:** User; no agent-run checks, reviews, or validators

## Single outcome

Expose Undo, Redo, Delete, and Select All through the shared command dispatcher with one fresh availability projection for future menu backends.

## Scope

1. Extend the shared command vocabulary in [`app_commands.rs`](../../../apps/scriptcraft/src/app_commands.rs) with Undo, Redo, Delete, and Select All.
2. Route Undo and Redo through [`undo()`](../../../crates/scriptcraft-core/src/editing.rs:1013) and [`redo()`](../../../crates/scriptcraft-core/src/editing.rs:1049), using the existing application mutation/status adapter rather than rewriting history behavior.
3. Route Delete through the same forward-delete operation used by [`key_event()`](../../../apps/scriptcraft/src/script_view.rs:754), preserving its collapsed-caret and cross-paragraph behavior.
4. Implement Select All with the existing first-position/document-end geometry and [`set_selection()`](../../../crates/scriptcraft-core/src/editing.rs:597). Selection changes must not manufacture semantic content, dirty state, or an undo entry.
5. Add a command-availability projection that initially reports Undo/Redo from [`can_undo()`](../../../crates/scriptcraft-core/src/editing.rs:590) and [`can_redo()`](../../../crates/scriptcraft-core/src/editing.rs:593), with appropriate Delete/Select All availability and migration-modal gating.
6. Keep existing keyboard behavior operational until WO-007 owns shortcut consolidation. Do not add menu UI or clipboard commands here.

## Structural acceptance for user review

- All four operations use the shared dispatcher and existing session APIs.
- Availability is computed from current application/session state rather than stored as static catalog metadata.
- No history labels are invented; the future visible labels remain Undo and Redo.
- No screenplay-core API or semantic behavior changes are required.

## User-owned behavior review

- Verify Undo/Redo enablement after edits, history exhaustion, New/Open, and Save.
- Exercise Delete with a collapsed caret, ordinary selection, reversed selection, and cross-paragraph Scene Heading cases.
- Confirm Select All covers the complete body and preserves content/history/dirty semantics.
- Confirm pending FDX migration blocks commands that would conflict with the modal.

## Handoff requirements

Record exact availability rules and any focus limitations left for WO-007. Do not describe menu items as visible or keyboard routing as consolidated after this order.

## WO-002 implementation handoff — 2026-11-06

- **State:** Implementation submitted for user review and validation only. It is not marked accepted or validated. WO-003 and later work orders have not started.
- **Checkout and preservation:** Work was performed on the existing `dev/2026-11-06-menu-system` branch. Before editing, the checkout already contained WO-001's modified source and untracked initiative/reference files; these were preserved. No branch was created, and no work was stashed, discarded, staged, or committed.
- **Exact changes:** [`app_commands.rs`](../../../apps/scriptcraft/src/app_commands.rs) adds shared Undo, Redo, Delete, and Select All command identities, plus `AppCommandAvailability` and a fresh `app_command_availability()` projection. The dispatcher invokes `EditingSession::undo()`, `redo()`, `delete()`, and `set_selection()` respectively, routes results through the existing common mutation/status adapter, requests repaint, and rejects edit commands that are unavailable at dispatch time. Select All uses the first body position through the final element's text end. No changes were made to [`script_view.rs`](../../../apps/scriptcraft/src/script_view.rs), [`main.rs`](../../../apps/scriptcraft/src/main.rs), [`toolbar.rs`](../../../apps/scriptcraft/src/toolbar.rs), [`toolbar_items.rs`](../../../apps/scriptcraft/src/toolbar_items.rs), or screenplay-core.
- **Availability rules:** Undo and Redo reflect current session history via `can_undo()` and `can_redo()`. Delete is available when a selection is noncollapsed, the caret precedes a character in its element, or the caret is in one of the delimiter-only Parenthetical elements handled specially by forward Delete (`(`, `()`, `)`). Select All is available when the project has a body element. Each of these four commands is unavailable while an FDX migration is pending; the dispatcher recomputes the projection before activation and silently ignores a stale/unavailable edit command. The existing file commands are not gated by this edit-command projection.
- **Semantics and focus boundary:** Delete delegates to the existing transactional forward-delete session operation, preserving its selection, collapsed-caret, and cross-paragraph behavior. Select All only changes session selection through `set_selection()`; it does not directly mutate semantic content or history. Existing focused-surface keyboard handling remains unchanged for WO-007; Undo/Redo, Select All, and Delete still follow their pre-existing editor-focus routing. No menu UI exists, keyboard ownership is not consolidated, and no focus-restoration behavior was added.
- **Evidence and blocker:** No build, check, test, lint, formatting command, launch, manual verification, review, or independent validator was run; user validation remains pending. During editing, the environment surfaced an existing Rust diagnostic in [`toolbar_items.rs`](../../../apps/scriptcraft/src/toolbar_items.rs): `ToolbarBehavior::Action(ToolbarAction)` refers to a type not found in that file. It is outside WO-002 and was left untouched rather than expanding scope; this may block compilation until WO-001's source issue is addressed. No runtime behavior or acceptance is claimed.
- **Changed paths for this order:** [`app_commands.rs`](../../../apps/scriptcraft/src/app_commands.rs), this work order, and [`menu-system-v1.md`](../menu-system-v1.md). No commit was created.

### Follow-up integration cleanup — 2026-11-06

At WO-003 start, corrected the reported stale type reference in [`toolbar_items.rs`](../../../apps/scriptcraft/src/toolbar_items.rs) from `ToolbarAction` to the shared `AppCommand`. This only resolves the identified migration remnant in source; no compilation or other validation was run, so the absence of further diagnostics is not asserted.

### WO-010 integrated-source reconciliation

The shared availability projection still derives Edit enablement from current session and selection
state, and the dispatcher rechecks it when processing queued activation. In the final integrated
source, `AppCommandAvailability::is_enabled()` also rejects every catalog command while the FDX
migration modal is pending, including File commands; this supersedes WO-002's interim note that File
commands were outside its edit-only migration gate. No behavior has been runtime-verified by this
addendum.

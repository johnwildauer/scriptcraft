# WO-001 — Shared File command dispatcher

- **Status:** Implementation submitted; user validation pending
- **Initiative:** [`menu-system-v1.md`](../menu-system-v1.md)
- **Depends on:** None
- **Validation owner:** User; no agent-run checks, reviews, or validators

## Single outcome

Move the existing New, Open, Save, Save As, and PDF export operations behind one toolbar-independent desktop command dispatcher, and add Close through the ordinary viewport command path.

## Scope

1. Create [`app_commands.rs`](../../../apps/scriptcraft/src/app_commands.rs) and register it from [`main.rs`](../../../apps/scriptcraft/src/main.rs:21).
2. Define shared command identities for New, Open, Close, Save, Save As, and Save As PDF. Leave Edit-command variants to WO-002 and WO-003.
3. Move the existing operation match arms out of [`dispatch_toolbar_action()`](../../../apps/scriptcraft/src/toolbar.rs:507) into one shared application command dispatcher.
4. Change [`ToolbarAction`](../../../apps/scriptcraft/src/toolbar_items.rs:15) and its call sites to reference the shared identities, or replace that enum with them if this is cleaner. Do not duplicate file-operation logic in the toolbar.
5. Implement Close by sending [`ViewportCommand::Close`](../../../apps/scriptcraft/src/main.rs:1161) through the egui context. Do not add a second native-window destruction mechanism.
6. Preserve existing New/Open/Save/Save As/PDF status, error, persistence, document-replacement, and export behavior. Do not claim to repair the pre-existing New dirty-session consent discrepancy recorded in the initiative.
7. Keep menu presentation, keyboard shortcuts, clipboard operations, native dependencies, and screenplay-core changes out of this order.

## Structural acceptance for user review

- The toolbar invokes the same dispatcher that later menu backends will call.
- No file dialog, save, export, or document-replacement implementation is duplicated.
- Close follows the existing egui viewport lifecycle.
- The dispatcher can receive the context it needs without making catalog data depend on egui.

## User-owned behavior review

- Exercise every existing toolbar action, including Save on an unsaved document, failed Open, failed Save As, and PDF export cancellation/failure.
- Confirm status/error behavior and document state match the pre-refactor path.
- Exercise Close and record its unsaved-document behavior without assuming a confirmation safeguard exists.
- Run any desired workspace checks and record outcomes separately.

## Handoff requirements

Append exact source changes, dispatcher inputs, and any behavior differences to this order and the initiative. Record that no visible menu exists yet and no validation is implied by implementation submission.

## WO-001 implementation handoff — 2026-11-06

- **State:** Implementation submitted for user review and validation. This does not mark WO-001 accepted or validated. WO-002 and all later work orders have not started.
- **Branch/checkout evidence:** Before branch creation, `git status --short --branch` reported branch `master` at `0d72f8f3c0258a09c99ef00078c97df263661783`, with no tracked/staged modifications and these pre-existing untracked paths: `.plan/2026-11-06-menu-system/`, `.reference/file.png`, `.reference/final-draft-13-menus.md`, and `.reference/initial-menus.md`. No local `dev/2026-11-06-menu-system` branch existed. `git switch -c dev/2026-11-06-menu-system` succeeded; the resulting checkout reported that exact branch and the same base commit. No changes were stashed, discarded, overwritten, staged, or committed. The plan and reference artifacts remained in the working tree.
- **Implementation:** Created [`app_commands.rs`](../../../apps/scriptcraft/src/app_commands.rs) with shared `AppCommand` identities for New, Open, Close, Save, Save As, and Save As PDF, plus one `ScriptCraftApp::dispatch_app_command(command, ctx)` dispatcher. The New/Open/Save/Save As/PDF operation branches were moved from the toolbar adapter without changing their existing operation code or status/error/persistence/export paths. Close sends `egui::ViewportCommand::Close` through the supplied egui context. [`toolbar_items.rs`](../../../apps/scriptcraft/src/toolbar_items.rs) now stores shared `AppCommand` values, and [`toolbar.rs`](../../../apps/scriptcraft/src/toolbar.rs) routes toolbar and overflow activations through that dispatcher; it contains no second file-operation implementation. [`main.rs`](../../../apps/scriptcraft/src/main.rs) registers the new module and imports the shared command identity.
- **Changed source paths:** `apps/scriptcraft/src/app_commands.rs` (new), `apps/scriptcraft/src/main.rs`, `apps/scriptcraft/src/toolbar.rs`, and `apps/scriptcraft/src/toolbar_items.rs`.
- **Behavior/scope boundary:** No menu presentation, Edit command, keyboard/shortcut, clipboard, native-menu dependency, or core change was made. No visible menu exists yet. The known New dirty-session consent discrepancy remains pre-existing and is not fixed or claimed fixed. Close uses ordinary egui viewport-close behavior; no unsaved-close confirmation was added.
- **Evidence limitations:** No build, check, test, lint, formatting command, application launch, manual verification, review, or independent validator was run. The user owns all validation. Existing toolbar behavior and command routing are implementation intent, not verified outcomes; no runtime behavior or acceptance is claimed.
- **Commit state:** No commit was created; the branch was created at base `0d72f8f3c0258a09c99ef00078c97df263661783`. Existing untracked initiative/reference work was preserved.

### WO-010 integrated-source reconciliation

The shared command identity/dispatcher remains the single route used by toolbar, in-window catalog,
native catalog IDs, and owned shortcuts. Later orders add Edit commands and queue/presentation
adapters without restoring toolbar-owned file-operation logic. The known New dirty-session consent
discrepancy remains present and uncorrected; no Close confirmation was added. This does not change
WO-001's historical implementation-submission-only status or imply validation.

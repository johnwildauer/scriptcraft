# WO-012 — SF Symbols Edit-menu completion and durable contracts

- **Status:** Implementation submitted for user review/validation; not accepted or validated
- **Initiative:** [`menu-system-v1.md`](../menu-system-v1.md)
- **Depends on:** [`WO-011`](WO-011-appkit-sf-symbols-file-menu.md)
- **Validation owner:** User; no agent-run checks, reviews, or validators
- **Authorization:** User explicitly authorized WO-012 implementation sequentially after WO-011 on the existing `dev/2026-11-06-menu-system` branch, without waiting for WO-011 review/validation.

## Single outcome

Complete the initial native Edit-menu symbol assignments and reconcile the SF Symbols bridge with the native controller lifecycle and durable project records.

## Scope

1. Extend the adapter mapping in [`macos_sf_symbols.rs`](../../../apps/scriptcraft/src/macos_sf_symbols.rs) for Edit items: Undo = arrow.uturn.backward; Redo = arrow.uturn.forward; Cut = scissors; Copy = doc.on.doc; Paste = doc.on.clipboard.
2. Leave Delete and Select All image-free unless the user selects a specific SF Symbol from Apple's catalog. Do not invent icons merely to fill the left column.
3. Remove any remaining obsolete NativeIcon-based item variant/imports from [`macos_menu.rs`](../../../apps/scriptcraft/src/macos_menu.rs) once all requested images use the SF Symbols adapter. Keep ordinary muda text items and retained native item references synchronized with the same command IDs.
4. Ensure the adapter's retained native references live exactly as long as the muda controller requires and remain main-thread confined. Label/enablement synchronization continues through muda; no additional per-frame image assignment or title-based lookup is introduced.
5. Preserve the enabled in-window preview and its Material icons. Native SF Symbol selection is independent and must not alter [`menu.rs`](../../../apps/scriptcraft/src/menu.rs), [`material_font.rs`](../../../apps/scriptcraft/src/material_font.rs), or cross-platform [`SemanticIcon`](../../../apps/scriptcraft/src/menu_items.rs) assignments.
6. Update the native-menu requirement in [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md), native adapter ownership and the narrow unsafe boundary in [`technical-context.md`](../../../.forge/blueprints/technical-context.md), and dependency/implementation-submission facts in [`project-manifest.md`](../../../.forge/project-manifest.md).
7. Reconcile [`menu-system-v1.md`](../menu-system-v1.md), WO-011, and this order with actual symbol assignments, dependency versions, warnings, and evidence limitations. Explicitly replace the obsolete no-direct-AppKit/non-SF-Symbol wording rather than leaving contradictory non-goals.

## Structural acceptance for user review

- All requested native icons come from one typed semantic-to-SF-Symbol mapping.
- Delete and Select All remain blank unless explicitly assigned later.
- No obsolete AppKit named-image route or duplicate native item model remains.
- The unsafe exception is limited to the documented bridge; muda command/event behavior and egui rendering remain unchanged.
- Durable documents describe the actual API and do not claim symbol availability or visual acceptance.

## User-owned behavior review

- Compare every native File/Edit icon in Light and Dark modes, including disabled Undo/Redo and Cut/Copy states.
- Verify template coloring, alignment, 18-point sizing, blank Delete/Select All rows, and absence of duplicate accessibility labels.
- Exercise all native and in-window commands, shortcuts, focus restoration, clipboard behavior, and event wakeups after the adapter change.
- Confirm startup remains functional when a symbol cannot be resolved and review the static warning.
- Run desired workspace checks on macOS and Windows and record outcomes separately; no cross-platform behavior is inferred from source alone.

## Handoff requirements

Record final symbol mappings, removed named-image code, retention/lifecycle details, unsafe-call inventory, documentation changes, and pending user evidence. Implementation submission is not validation or acceptance.

## WO-012 implementation handoff — 2026-11-06

- **State and checkout:** WO-012 implementation is submitted for user review and validation only; it is not accepted or validated. Work followed WO-011 on the existing `dev/2026-11-06-menu-system` branch. Existing work was preserved; no branch or commit was created. The user explicitly authorized this order to proceed without waiting for WO-011 validation.
- **Exact symbol mappings:** [`macos_sf_symbols.rs`](../../../apps/scriptcraft/src/macos_sf_symbols.rs) now maps File New (`plus`), Open (`arrow.up.right`), Close (`xmark`), Save (`square.and.arrow.down`), Save As (`square.and.arrow.down.on.square`), plus Edit Undo (`arrow.uturn.backward`), Redo (`arrow.uturn.forward`), Cut (`scissors`), Copy (`doc.on.doc`), and Paste (`doc.on.clipboard`). Save As PDF, Delete, and Select All have no requested symbol and remain image-free. Images continue to use AppKit's system-symbol initializer, 18-point size, and template rendering. The shared catalog, cross-platform `SemanticIcon` assignments, egui Material icons, and enabled in-window preview were not changed.
- **Resolution, retention, and synchronization:** The adapter derives symbol requests from the shared File/Edit catalog IDs but keeps symbol names native-only. After muda installs the root menu, it resolves each requested top-level submenu and direct catalog item row once, collects item handles and images before assignment, and retains the root `NSMenu` plus resolved `NSMenuItem` references. The adapter is held by `NativeMenuController`, uses an `Rc` marker for main-thread confinement, and performs no per-frame hierarchy lookup or image assignment. Muda continues to own construction, IDs, commands, event handling, labels, enablement, and synchronization cadence. `macos_menu.rs` already had ordinary muda text items and no `NativeIcon` import/variant after WO-011 removed the Add/Folder route, so no additional cleanup was necessary there.
- **Dependencies and warnings:** No dependency or lockfile change was made in WO-012. The existing macOS-target direct dependencies are objc2 0.6.5, objc2-app-kit 0.3.2, and objc2-foundation 0.3.2, with muda 0.17.2 resolved; WO-011's fetch was dependency setup evidence only. Missing/unavailable menu items, symbols, or main-thread/root handles leave images absent and do not block startup or command activation. A static warning identifies only the catalog item ID and SF Symbol name; at most ten warnings can be emitted per installation, one per requested File/Edit mapping.
- **Unsafe boundary:** The sole unsafe operation remains `Retained::retain(muda_root.ns_menu().cast::<NSMenu>())` in `retain_native_menu()`, with one function-local `#[allow(unsafe_code)]` and a safety comment for borrowed-handle validity, verified main-thread use, and owned retention. No raw Objective-C message calls or module/crate-wide unsafe allowances were added; package unsafe denial remains in force elsewhere.
- **Durable records and changed paths:** Updated the native-menu requirement, technical-context adapter/lifecycle/unsafe-boundary description, project-manifest dependency and implementation-submission facts, the initiative plan, WO-011 reconciliation, and this handoff. Source change is limited to `macos_sf_symbols.rs`; no changes were made to `macos_menu.rs`, `menu.rs`, `material_font.rs`, or cross-platform catalog icon mappings because their post-WO-011 behavior already satisfies the scope. No automated desktop tests were added.
- **Evidence limits and pending user evidence:** No build, check, test, lint, formatting command, launch, manual verification, review, or independent validator was run under repository policy. Compilation, target OS symbol availability, native hierarchy resolution and fallback behavior, 18-point visual alignment/template appearance in Light/Dark modes and disabled states, command/shortcut/wakeup/enablement synchronization, and dual-preview behavior remain unverified. Implementation submission is not validation or acceptance.

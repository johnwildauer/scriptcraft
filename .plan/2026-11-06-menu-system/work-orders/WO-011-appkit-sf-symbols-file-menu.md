# WO-011 — AppKit SF Symbols bridge and File-menu icons

- **Status:** Implementation submitted for user review; not accepted or validated
- **Initiative:** [`menu-system-v1.md`](../menu-system-v1.md)
- **Depends on:** [`WO-010`](WO-010-durable-contracts-and-development-closeout.md)
- **Validation owner:** User; no agent-run checks, reviews, or validators
- **Authorization:** User explicitly authorized WO-011 implementation on the existing initiative branch only.

## Single outcome

Add a narrow macOS-only AppKit SF Symbols adapter and use it to render the reference-style native File-menu images with template coloring, replacing the current limited AppKit named-image assignments.

## Feasibility and required boundary

The resolved muda 0.17.2 native-icon path maps a closed variant set to AppKit named images. It does not expose arbitrary SF Symbol names or individual native menu-item handles. It does expose the installed root native menu handle. The implementation must therefore add a small AppKit bridge rather than pretending an SF Symbol string is an existing [`NativeIcon`](../../../apps/scriptcraft/src/macos_menu.rs:276) variant.

Use the dependency generation already selected by muda: objc2 0.6.5, objc2-app-kit 0.3.2, and objc2-foundation 0.3.2. Do not introduce a second incompatible AppKit generation or change eframe/egui.

## Scope

1. Add macOS-target dependencies for the matching objc2, objc2-app-kit, and objc2-foundation generations in [`apps/scriptcraft/Cargo.toml`](../../../apps/scriptcraft/Cargo.toml). Resolve the lockfile if needed as setup evidence only.
2. Add macOS-only [`macos_sf_symbols.rs`](../../../apps/scriptcraft/src/macos_sf_symbols.rs), owned by the native menu controller and confined to the main thread. Keep SF Symbol names out of [`menu_items.rs`](../../../apps/scriptcraft/src/menu_items.rs), which remains toolkit-independent.
3. Create native images through AppKit's system-symbol initializer, using each symbol's catalog name and no redundant accessibility description because the menu item already has an accessible label. Set an 18-point initial image size and enable template rendering so AppKit supplies normal/disabled and Light/Dark foreground behavior.
4. After muda installs the root menu, resolve requested native menu items once through the installed native menu hierarchy using the shared top-level/item catalog labels. Retain the resolved native item references for later use; do not traverse the hierarchy every frame.
5. Make item resolution deterministic and complete before assigning images. If a requested item or symbol cannot be resolved, leave that item image-free and return a bounded, value-free warning identifying only the static catalog/symbol ID. Never block application startup or command activation solely because an optional decorative image is unavailable.
6. Keep every required raw native-handle dereference or Objective-C message in this narrow adapter with function-local safety comments and the smallest possible unsafe lint allowance. Do not add a module-wide or crate-wide unsafe allowance. The existing package deny policy remains everywhere else; this follow-up explicitly supersedes the earlier no-unsafe-bridge non-goal only for the documented adapter calls.
7. Replace the current Add/Folder named-image route with SF Symbols for File items: New = plus; Open = arrow.up.right; Close = xmark; Save = square.and.arrow.down; Save As = square.and.arrow.down.on.square. Leave Save As PDF image-free to match the supplied reference.
8. Retain muda ownership of menu construction, IDs, commands, events, labels, enablement, and accelerators. The adapter assigns images only and does not replace native menu behavior or the egui icon renderer.

## Structural acceptance for user review

- Exact SF Symbol names are native-adapter metadata, not cross-platform catalog data.
- Native item references are resolved once and retained on the main thread.
- Images are template-rendered at one named size and do not affect command state.
- Unsafe calls are individually justified and scoped; no broad unsafe exemption or dependency-version duplication is introduced.
- Missing optional symbols/images cannot prevent the writing loop from starting.

## User-owned behavior review

- Inspect the native File menu in Light and Dark Appearance modes and compare New/Open/Close/Save/Save As with [`file.png`](../../../.reference/file.png).
- Verify icon vertical alignment, size, template foreground, disabled appearance, and absence of an image on Save As PDF.
- Confirm all File commands, shortcuts, event wakeups, and enablement still behave as before.
- Exercise an unavailable-symbol/item-path case if practical and confirm the app remains usable with a bounded static warning.
- Run any desired compilation and runtime checks and record outcomes separately.

## Handoff requirements

Record exact dependency versions, native item-resolution mechanism, unsafe call inventory and justifications, symbol assignments, warning behavior, and all unverified visual/runtime behavior. Do not begin WO-012 within this order.

## WO-011 implementation handoff — 2026-11-06

- **State and checkout (at WO-011 submission):** Implementation is submitted for user review and validation only; it is not accepted or validated. Work remained on the existing `dev/2026-11-06-menu-system` branch. The pre-edit status showed the initiative's existing modified/untracked files; they were preserved. No branch or commit was created. WO-012 had not started at that time; its later authorization and implementation are recorded in the closeout reconciliation below.
- **Exact dependency and lockfile setup:** [`apps/scriptcraft/Cargo.toml`](../../../apps/scriptcraft/Cargo.toml) now declares macOS-target-only `objc2 = { version = "=0.6.5", default-features = false, features = ["std"] }`, `objc2-app-kit = { version = "=0.3.2", default-features = false, features = ["std", "NSImage", "NSMenu", "NSMenuItem"] }`, and `objc2-foundation = { version = "=0.3.2", default-features = false, features = ["std", "NSGeometry", "NSString"] }`, alongside the existing `muda = { version = "0.17", default-features = false }` (locked 0.17.2). `cargo fetch --target aarch64-apple-darwin` completed successfully as dependency-resolution/setup evidence only. [`Cargo.lock`](../../../Cargo.lock) now includes the direct ScriptCraft edges to the three existing compatible objc2 packages; Cargo also resolved the target dependency graph. This is not compilation or runtime evidence. eframe/egui versions were not changed.
- **Adapter and exact AppKit API (at WO-011 handoff):** New macOS-only [`macos_sf_symbols.rs`](../../../apps/scriptcraft/src/macos_sf_symbols.rs) is registered only under `cfg(target_os = "macos")`. After `Menu::init_for_nsapp()`, `SfSymbolsAdapter::install()` checks `MainThreadMarker::new()`, uses `muda::ContextMenu::ns_menu()` to obtain the installed root's borrowed native handle, and retains that `NSMenu`. It resolves the File top-level item and then the requested direct File item rows by unique title comparisons against `MENU_CATALOG` labels (`NSMenu::numberOfItems`, `itemAtIndex`, `NSMenuItem::title`, and `submenu`). All requested handles and symbol images are resolved before any `setImage` call. The adapter retains the root and every successfully resolved requested `NSMenuItem`; it is main-thread-only and performs no menu hierarchy traversal or image assignment per frame. WO-012 extends the same one-time resolution to Edit rows; see the closeout addendum below.
- **Image construction and assignments (at WO-011 handoff):** Each symbol uses `NSImage::imageWithSystemSymbolName_accessibilityDescription(name, None)`, then `setSize(NSSize::new(18.0, 18.0))` and `setTemplate(true)`. Native-only mappings are New (`file.new`) → `plus`; Open (`file.open`) → `arrow.up.right`; Close (`file.close`) → `xmark`; Save (`file.save`) → `square.and.arrow.down`; Save As (`file.save-as`) → `square.and.arrow.down.on.square`. Save As PDF (`file.save-as-pdf`) is deliberately not requested and remains image-free. SF Symbol names are confined to the native adapter; `menu_items.rs` and egui Material icons were not changed. WO-012 subsequently adds the Edit mappings recorded in its handoff.
- **Optional failure/warnings (at WO-011 handoff):** A missing/ambiguous File menu or item, unavailable system symbol, or unavailable main-thread/root handle leaves affected items without images and does not fail menu installation, startup, or command activation. Installation emits at most one bounded warning for each of the five requested catalog/symbol pairs, containing only the static catalog item ID and SF Symbol name; no dynamic error text or user/document data is logged. Save As PDF produces no warning because it has no requested symbol. With WO-012, the final combined File/Edit request set is ten mappings and can emit at most ten such warnings per installation.
- **Unsafe inventory and policy:** The adapter contains exactly one unsafe operation: `Retained::retain(muda_root.ns_menu().cast::<NSMenu>())` in `retain_native_menu()`. Its single function-local `#[allow(unsafe_code)]` covers only that function. The safety comment records the muda borrowed-handle lifetime, verified AppKit main-thread call, and owned retain. No raw Objective-C message calls, module/crate-wide unsafe allowances, or root/workspace lint changes were added; the package's `unsafe_code = "deny"` and source-level deny policy remain.
- **Preserved behavior and changed paths:** `macos_menu.rs` now creates ordinary muda `MenuItem`s without the prior `NativeIcon::Add`/`NativeIcon::Folder` path and retains existing native IDs, commands, labels, accelerators, events, enablement synchronization, and controller lifecycle. The in-window preview remains enabled. Changed paths: `apps/scriptcraft/Cargo.toml`, `Cargo.lock`, `apps/scriptcraft/src/main.rs`, `apps/scriptcraft/src/macos_menu.rs`, new `apps/scriptcraft/src/macos_sf_symbols.rs`, this work order, and the initiative plan. Durable requirement/blueprint/project-manifest records were inspected but not edited in WO-011; their SF Symbols and unsafe-boundary reconciliation remains explicit WO-012 scope.
- **Evidence limitations and WO-012 handoff (at WO-011 submission):** No build, check, test, lint, formatting command, launch, manual verification, review, or independent validator was run under repository policy. The use-sites are based on inspection of the cached objc2 0.6.5 / objc2-app-kit 0.3.2 / objc2-foundation 0.3.2 and muda 0.17.2 APIs; compilation is unverified. Symbol availability on the target macOS release, actual menu lookup, visual size/alignment/template foreground in Light/Dark modes, disabled rendering, startup fallback, command/shortcut/wakeup behavior, and in-window preview interaction remain unverified and user-owned. At this handoff WO-012 was next and unstarted; it was subsequently explicitly authorized and implemented on the same branch without waiting for WO-011 review/validation, as recorded below.

## WO-012 closeout reconciliation — 2026-11-06

WO-012 completes the initial native File/Edit SF Symbol mapping. The final native assignments are File New (`plus`), Open (`arrow.up.right`), Close (`xmark`), Save (`square.and.arrow.down`), Save As (`square.and.arrow.down.on.square`); Edit Undo (`arrow.uturn.backward`), Redo (`arrow.uturn.forward`), Cut (`scissors`), Copy (`doc.on.doc`), Paste (`doc.on.clipboard`). Save As PDF, Delete, and Select All remain image-free. Images are 18-point AppKit system-symbol templates; unavailable symbols or item paths are nonfatal and may emit one static catalog-ID/symbol warning per requested mapping, bounded at ten warnings per installation.

The adapter now resolves requested File and Edit top-level submenus and catalog item rows during installation, retaining the root `NSMenu` and resolved `NSMenuItem` handles in the controller-owned, main-thread-confined `SfSymbolsAdapter`. The adapter assigns images only during installation; muda remains authoritative for native IDs, commands/events, label and enablement synchronization. `macos_menu.rs` already used ordinary muda `MenuItem`s after WO-011 removed its `NativeIcon::Add`/`NativeIcon::Folder` route, so WO-012 required no additional `NativeIcon` import/variant cleanup there. The egui preview remains enabled and its Material icon assignments are unchanged.

Dependency facts remain macOS-target-only objc2 0.6.5, objc2-app-kit 0.3.2, and objc2-foundation 0.3.2, alongside muda 0.17.2; WO-012 added no dependencies. The only unsafe operation remains the function-local `Retained::retain` of muda's borrowed root `NSMenu` handle, guarded by a local allowance and safety comment. No raw Objective-C message calls or wider unsafe allowances were added. No compilation or other validation is claimed.

Durable changes: native-menu requirement AC-WRITING-011.6 in `screenplay-writing-loop.md`; adapter ownership, retained references, symbol mapping, dependency generation, and the narrow unsafe boundary in `technical-context.md`; dependency/implementation-submission facts in `project-manifest.md`; and reconciled initiative/WO-011/WO-012 handoffs. No build, check, test, lint, formatting, launch, manual verification, review, or independent validator was run. User evidence remains pending for compilation, target OS symbol availability, actual item resolution and fallback, Light/Dark/disabled appearance and alignment, native menu command/shortcut/event/enablement behavior, and in-window preview interactions. Neither WO-011 nor WO-012 is marked accepted or validated.

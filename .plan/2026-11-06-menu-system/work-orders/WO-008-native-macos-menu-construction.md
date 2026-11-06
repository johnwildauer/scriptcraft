# WO-008 — Native macOS menu construction

- **Status:** Implementation submitted for user review; not accepted or validated
- **Initiative:** [`menu-system-v1.md`](../menu-system-v1.md)
- **Depends on:** [`WO-007`](WO-007-shortcut-ownership-and-focus.md)
- **Validation owner:** User; no agent-run checks, reviews, or validators

## Single outcome

Construct and install the shared File/Edit catalog as a retained native macOS application menu while leaving the in-window preview switch enabled.

## Scope

1. Add `muda` as a macOS-target dependency in [`apps/scriptcraft/Cargo.toml`](../../../apps/scriptcraft/Cargo.toml), with default features disabled to avoid unrelated GTK backends.
2. Create macOS-only [`macos_menu.rs`](../../../apps/scriptcraft/src/macos_menu.rs) and confine native menu construction/mutation to the main thread.
3. Build File/Edit from [`menu_items.rs`](../../../apps/scriptcraft/src/menu_items.rs) with stable native IDs, native separators, custom command items, logical accelerators, and current availability.
4. Use suitable native-icon menu items only where the semantic match is clear; leave other images absent. Record exact assignments rather than forcing misleading parity with Material icons.
5. Add a macOS-only standard Application menu with predefined native About, Hide, Hide Others, Show All, and Quit items. Do not use predefined native Edit responder items for ScriptCraft's custom editor.
6. Install the root menu during the existing creation callback in [`main()`](../../../apps/scriptcraft/src/main.rs:1396) and retain the menu/item controller in [`ScriptCraftApp`](../../../apps/scriptcraft/src/main.rs:381).
7. Preserve the application unsafe-code deny policy and the enabled in-window preview. Native event handling and dual-backend shortcut arbitration belong to WO-009.

## Structural acceptance for user review

- Native menu content is generated from the same catalog as egui, not a parallel hand-written File/Edit list.
- Native objects remain alive for the application lifetime and no Windows build constructs them.
- No direct Objective-C bridge or unsafe application code is introduced.
- The in-window preview remains enabled after native installation.

## User-owned behavior review

- Launch on macOS and inspect native Application/File/Edit menus, labels, grouping, icons, and native shortcut glyphs.
- Compare native and in-window menus side by side for catalog equivalence and initial enablement.
- Confirm standard Application-menu items behave natively. Item command activation is reviewed fully after WO-009.
- Run any desired compilation/dependency checks and record results separately.

## Handoff requirements

Record dependency resolution, native APIs, item-retention design, icon assignments, and the intentional absence of completed native event routing. Dependency resolution is not build or runtime evidence.

## WO-008 implementation handoff — 2026-11-06

- **State and checkout:** WO-008 is submitted for user review and validation only; it is not accepted or validated. Work remained on the existing `dev/2026-11-06-menu-system` branch, preserving prior work. No branch or commit was created. The in-window preview switch remains enabled (`IN_WINDOW_MENU_PREVIEW_ENABLED = true`). WO-009 has not started.
- **Dependency and setup evidence:** [`apps/scriptcraft/Cargo.toml`](../../../apps/scriptcraft/Cargo.toml) adds target-specific `muda = { version = "0.17", default-features = false }` under `cfg(target_os = "macos")`. The permitted setup-only `cargo fetch --target aarch64-apple-darwin` completed successfully and resolved muda **0.17.2** in [`Cargo.lock`](../../../Cargo.lock), adding it to the ScriptCraft package dependency set. This is dependency-resolution evidence only; it is not compilation or platform behavior evidence. No Linux backend dependency was added to the application target configuration.
- **Native construction and installation:** [`macos_menu.rs`](../../../apps/scriptcraft/src/macos_menu.rs) is macOS-only. During eframe's existing creation callback, `ScriptCraftApp::new()` is followed by native construction using the app's initial `AppCommandAvailability`; `Menu::new()`, `Submenu::with_id()`, catalog-backed item constructors, and `Menu::init_for_nsapp()` perform installation. File/Edit top-level IDs and every custom item ID are the shared catalog strings. Catalog separators become native `PredefinedMenuItem::separator()` entries. File/Edit command rows are custom `MenuItem`/`IconMenuItem` instances, never predefined Edit responder items. `Accelerator` values map logical Primary to macOS Command (`Modifiers::SUPER`), preserve Shift, and map catalog keys to keyboard `Code`s. Initial Undo/Redo/Cut/Copy/Paste/Delete/Select All enabled states use the current availability projection; File commands initialize enabled.
- **Application menu and icons:** The first root submenu is the macOS-only `ScriptCraft` Application menu with predefined About, Hide, Hide Others, Show All, and Quit, with standard grouping separators and About metadata from the package version. Native images are intentionally limited to catalog `DocumentNew` → `NativeIcon::Add` and `FolderOpen` → `NativeIcon::Folder`; all other items have no native image because no clearer native semantic match was selected.
- **Retention and confinement:** `ScriptCraftApp` retains an `Option<NativeMenuController>` on macOS. The controller owns the root `Menu`, submenu handles, every custom text/icon item handle, and predefined items; an `Rc` marker makes main-thread-only ownership explicit. The construction/installation uses safe muda APIs and adds no unsafe application code; the crate and package unsafe-code denial policies remain unchanged.
- **Deliberately deferred:** This work does not receive native events, enqueue native commands, request repaint/wakeups, synchronize availability after construction, or arbitrate accelerators against the in-window preview. Those remain WO-009 scope. The preview remains enabled and currently retains WO-007's temporary in-window shortcut ownership.
- **Changed paths:** `apps/scriptcraft/Cargo.toml`, `Cargo.lock`, `apps/scriptcraft/src/macos_menu.rs`, `apps/scriptcraft/src/main.rs`, this work order, and the initiative plan. Prior source changes and untracked plan/reference work were preserved.
- **Evidence limitations:** No build, check, test, lint, formatting command, application launch, manual verification, review, or independent validator was run, as repository policy requires. Catalog construction, native API compatibility, macOS menu appearance/behavior, startup installation, initial enablement, and retention are not runtime-verified. Dependency fetching is the only command evidence; user-owned validation remains pending.

### WO-010 integrated-source reconciliation

The current macOS dependency declaration remains `muda = { version = "0.17", default-features =
false }`, with lockfile version 0.17.2. WO-009 supplies the event and synchronization integration:
the installed event handler requests repaint and feeds native IDs to an inbox drained by the app;
with this `muda` handler installed, `MenuEvent::receiver()` does not receive those events. Native
startup, event delivery, appearance, and item behavior remain user-unverified.

# Component Blueprint: User Settings

> This component describes the approved target ownership and persistence boundary for global user preferences. It is a technical contract, not evidence that the implementation or any validation has occurred.

## Capability and ownership

The desktop application owns the user's app-wide appearance choice and default new-document template ID. These values are not Project, EditingSession, FDX, layout, command-wire, or export state. Requirements are defined in [User Settings](../../requirements/features/user-settings.md).

One app-private [`settings.rs`](../../../apps/scriptcraft/src/settings.rs) module, declared by [`main.rs`](../../../apps/scriptcraft/src/main.rs), owns the Serde/TOML `Settings` value, `ThemeChoice`, and `SettingsStore`. The schema value contains a version, the theme choice, and the raw template ID string; the settings module does not own egui presentation policy or template content. It introduces no package or internal dependency edge. The desktop app directly depends on Serde `=1.0.229` with derive support, TOML `=1.2.1`, and the current stable `directories` 6.x API.

## Location, loading, and recovery

Resolve the config directory with `directories::ProjectDirs` using qualifier `com`, organization `ScriptCraft`, and application `ScriptCraft`, then append `settings.toml`. Conventional locations are `~/Library/Application Support/com.ScriptCraft.ScriptCraft/settings.toml` on macOS and `%APPDATA%\ScriptCraft\ScriptCraft\config\settings.toml` on Windows; the exact resolved path is authoritative and is logged on each startup. If project directories are unavailable, use in-memory defaults, disable persistence for that process, and warn; do not use the repository or working directory. A missing file loads defaults without creating a file on launch. Struct-level defaults accept partial files, and unknown keys do not reject a file; a later ordinary serialization need not preserve unknown keys during alpha.

Schema version 1 is the only emitted version. Missing schema version defaults to 1. Unknown theme strings resolve to System without discarding other valid fields. Unknown template IDs remain raw in the settings snapshot; the FDX catalog's [`BuiltinTemplateId::from_stable_id()`](../../../crates/scriptcraft-fdx/src/builtin_templates.rs) validates effective use, and [`instantiate_builtin_template()`](../../../crates/scriptcraft-fdx/src/builtin_templates.rs) remains the full loader. Unknown/retired IDs warn and use Screenplay only as a non-mutating effective fallback. No ordinary load or fallback automatically writes settings.

Malformed, unreadable, or incompatible existing files are preserved under `settings.toml.bak`, then the first available bounded numeric suffix. Backup creation makes a same-directory hard link at the first unused candidate and then unlinks the original name; this no-clobber sequence never overwrites an existing backup. If backup fails, defaults remain in memory and ordinary persistence is disabled for that process so later changes cannot overwrite unrecovered data. Startup recovery is synchronous before presentation; ordinary writes are not performed on the UI thread.

## Store and writer

One `SettingsStore` is owned by `ScriptCraftApp`. Its load constructor is `load()`; the app-thread API comprises `settings()`, `config_path()`, `theme()`, `effective_template()`, no-op-aware `set_theme()` and `set_template()`, `reset_to_defaults()`, and `persistence_available()`; filesystem and writer internals remain private to `settings.rs`. UI reads and updates stay on the app thread. Narrow no-op-aware setters publish owned snapshots over a standard unbounded MPSC channel to one named dedicated writer thread. A real update waits for a 350 ms quiet period, restarting the interval as newer snapshots arrive, then saves only the latest snapshot. No async runtime, `Arc<RwLock<_>>`, writer-driven repaint, or UI wait is used. A failed send or thread spawn leaves memory-only operation and logs once. Save failures warn without rolling back the active in-memory choice. Channel disconnect exits without a final flush; there is no shutdown command, Drop join, or shutdown flush.

Each save creates the config directory if needed, serializes the complete snapshot, creates a unique same-directory temporary file using process ID and a bounded create-new counter, writes and syncs it, and renames it over the destination. It never follows a pre-existing temporary path or deletes the old destination first. Failure logs a bounded warning, best-effort removes only the new temporary file, and retains the prior destination while the writer continues. Atomic replacement does not promise power-loss durability or cross-process transaction isolation.

An explicit `--reset-settings` resets to System/Screenplay and queues a defaults snapshot even when in-memory values are already defaults, provided persistence remains available. If backup recovery disabled persistence, reset is memory-only and logs why.

## Integration boundaries

- Startup loads and recovers settings before first rendered content, resolves the effective appearance/template, and installs appearance through the eframe creation callback. Existing `configure_appearance()` System tracking remains the live Day/Night authority; Midnight is explicit only.
- A supplied startup document path wins over the global default. The persisted default is not an explicit template argument and does not pre-finalize the macOS cold-open initial slot. Existing staged-open activation remains authoritative, and opened document definitions/content are never changed by a global preference.
- No-document launch and ordinary File → New use the effective saved ID; an unknown ID falls back to Screenplay without rewriting the raw value. New-document selector presentation starts from the effective saved default, while Apply retains its existing Screenplay initial presentation.
- New from Template updates the parent preference only after successful independent child spawn. Its explicit child ID remains authoritative even if the parent's debounced save has not completed. Browsing, Cancel, Apply, failed spawn, and ordinary File New do not write the preference.
- Appearance selection remains projected into existing app/UI state and persists only a real change applied through the existing typed command; it does not alter document semantics, dirty state, undo/history, layout identity, or export.
- The sole ordered built-in catalog and loader remain in `scriptcraft-fdx`; settings add no registry, parser, template-content ownership, file watcher, merge, lock, process broker, or live preference broadcast.

Independent processes are best-effort: last completed atomic save wins. A child launched within the parent's debounce interval may observe older settings. No shutdown flush is promised, so a very recent preference may be lost on exit while an older valid file remains intact.

## Related documents

- [User settings requirements](../../requirements/features/user-settings.md)
- [Desktop adapters](desktop-adapters.md)
- [ScriptCraft Desktop container](../containers/scriptcraft-desktop.md)

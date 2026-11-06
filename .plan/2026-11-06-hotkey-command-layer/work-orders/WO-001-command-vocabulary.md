# WO-001 — Stable command vocabulary and serializable envelopes

- **Status:** Implemented; user-owned validation pending
- **Initiative:** [Hotkeys and consolidated command layer v1](../hotkey-command-layer-v1.md)
- **Depends on:** User approval of the initiative
- **Validation owner:** User; no agent-run checks, reviews, or validators

## Single outcome

Create a UI-independent command package that gives every enabled ScriptCraft-owned action a stable ID, typed parameters, serializable wire representation, and origin metadata without changing application behavior.

## Scope

1. Add [`crates/scriptcraft-commands/`](../../../crates/scriptcraft-commands/) to the Cargo workspace with no desktop-framework, dialog, clipboard, native-menu, or windowing dependency.
2. Define the typed command inventory listed in the initiative, including document, Edit, text, deletion, Enter/Tab, element Role, Selection, navigation, focus, view/status, and sprint-timer commands.
3. Assign stable dotted IDs and one authoritative descriptor catalog. Reject duplicate IDs, unknown IDs, malformed payloads, invalid enum values, invalid UTF-8 selections, and out-of-range bounded values before execution.
4. Define command origin categories for menu, toolbar, native menu, shortcut, keyboard, pointer, IME commit, internal follow-up, CLI, MCP, and tests. Origin is diagnostic/routing metadata and must not alter semantic results.
5. Provide explicit serialization/deserialization for command envelopes and typed parameters. Use stable snake_case role values already exposed by [`Role::as_str()`](../../../crates/scriptcraft-core/src/lib.rs:198) and checked UTF-8 byte positions from the core Selection model. Do not serialize egui values, native handles, filesystem-dialog objects, clipboard objects, or FDX source state.
6. Keep static menu/toolbar catalogs compatible with owned command payloads by storing a stable command ID or parameterless command factory rather than requiring a dynamic command value to be Copy.
7. Record exact new dependency/version choices and generated lockfile changes. Serialization dependencies are implementation setup only and are not evidence of runtime behavior.

## Out of scope

- No dispatcher integration, hotkey activation, new application action, CLI binary, MCP server, macro recorder, or command palette.
- No FDX field, screenplay metadata, private sidecar, persisted keymap, or user preferences format.
- No commands for disabled menu placeholders, disabled toolbar items, or reference-only features.
- No semantic core algorithm changes. Serialization derives or narrow wire conversions on existing model types are permitted only where needed and must not change values or persistence.

## Structural acceptance for user review

- Every command ID in the initiative inventory has exactly one descriptor and one typed construction/parsing path.
- Round-trip representations preserve IDs, parameters, and origin without screenplay-core persistence changes.
- Command definitions compile without eframe/egui and can be consumed by headless tests and future non-GUI clients.
- Parameterless menu/native rows can still be declared statically; parameterized actions are constructed only at activation or external-input parsing.
- Unknown or unavailable commands have a typed rejection path rather than becoming no-op product features.

## User-owned checks

- Run the applicable workspace format, check, build, Clippy, and test commands after implementation.
- Review descriptor uniqueness, malformed/unknown envelope rejection, all 17 Role values, Unicode text payloads, path payloads, Selection boundaries, timer/view enums, and origin round trips.
- Inspect the dependency boundary to confirm the new crate does not import desktop or native UI frameworks.

## Durable-document impact

Update the technical context with the new command package and stable wire boundary. Requirements impact is update because stable command identity and non-GUI reachability become explicit product architecture contracts. Do not claim that runtime routing exists until later orders implement it.

## WO-001 implementation handoff — 2026-11-06

- Added `crates/scriptcraft-commands` to the Cargo workspace with no desktop-framework dependency.
- Defined 41 stable dotted IDs, typed commands and parameters, command origins, a versioned envelope, explicit Serde conversions for core Role values, validated document paths and zoom values, document-aware selection/UTF-8-boundary validation, and a descriptor catalog with coverage/duplicate validation.
- Added exact `serde = 1.0.229` with the `derive` feature; Serde was already represented in the lockfile, so the lockfile change is the new workspace-package dependency entry only.
- Updated the technical context to record the implemented vocabulary while preserving the target/runtime distinction. No application dispatcher or semantic editing behavior was changed.
- Files changed: `Cargo.toml`, `Cargo.lock`, `crates/scriptcraft-commands/Cargo.toml`, `crates/scriptcraft-commands/src/lib.rs`, `.forge/blueprints/technical-context.md`, and this work order.
- No build, test, check, lint, formatting, or other validation was run. The 41-entry mapping, Serde round trips, malformed-input rejection, dependency boundary, and compilation remain for user-owned validation.

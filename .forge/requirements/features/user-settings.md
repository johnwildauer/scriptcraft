# Requirements Area: User Settings

> This document specifies the established user-facing contract for global preferences. It describes desired behavior, not implementation status, validation, or acceptance.

## Area purpose

User settings retain a writer's small set of app-wide preferences across launches without making those preferences part of a screenplay or native FDX project.

## Global preference scope and schema

- **REQ-WRITING-LOOP-013: Persistent global user settings.** ScriptCraft shall retain the user's supported global appearance and default new-document template choices while keeping them separate from screenplay documents and editing state.
  - **AC-WRITING-013.1:** Schema version 1 contains exactly the `schema_version`, `theme`, and `template` fields, defaulting to System and `screenplay`. Missing fields use those version-1 defaults. Additional future global preferences are additive and defaulted; removing or renaming a field, theme variant, or template ID requires an explicit migration note and step.
  - **AC-WRITING-013.2:** The persisted theme choice is exactly one of System, Day, Night, or Midnight, encoded as the lowercase values `system`, `day`, `night`, or `midnight`. Only the choice is stored. System resolves to Day or Night from egui's supplied OS theme and follows live changes where egui supplies them; it never resolves to Midnight. Midnight is selected explicitly.
  - **AC-WRITING-013.3:** The persisted default template is a stable ID from the packaged FDX built-in catalog, not a display name, path, or template contents. The FDX catalog remains the sole registry and validator. Other application globals are outside the initial schema.

## Startup and launch precedence

- **AC-WRITING-013.4:** Settings load before the first rendered application content. A no-document launch and ordinary File → New use the saved default template. An explicit internal New from Template selection wins for that child launch. A supplied FDX or FDXT document path takes precedence over the saved template; opening never applies the preference to the document or changes its definitions, content, native path, or baseline. An unknown saved template ID is retained as stored, produces a warning, and uses Screenplay only as the effective launch/selector fallback without rewriting settings because of fallback. A persisted default does not consume or finalize the macOS cold-open initial document slot; only the explicit internal template argument retains that existing behavior.

## Automatic persistence and state boundaries

- **AC-WRITING-013.5:** Real preference changes save automatically without a manual Save action or success dialog. Appearance changes persist when applied through the existing typed command. New from Template persists the selected ID only after the independent child process launches successfully; browsing, Preview, Cancel, Apply Changed/NoOp, failed spawn, and ordinary File New do not independently update it. Successful spawn is the boundary even if the child later fails to open its document. Preference changes do not dirty screenplay state or alter undo/history, layout identity, or exports.

## Recovery, reset, and process limits

- **AC-WRITING-013.6:** Missing files load defaults without creating a file merely because the app launched; partial files use defaults for missing fields; unknown keys are tolerated. An unknown theme resolves to System while retaining other valid values. An unknown template remains raw in memory and is not sanitized by an unrelated save. Malformed, unreadable, or unsupported-version files are backed up before defaults are used. Backup names are non-destructive (`settings.toml.bak`, then available bounded numeric suffixes); if preservation fails, ordinary persistence is disabled for that process. Missing schema version means version 1; only version 1 is emitted. Ordinary load and fallback do not themselves save.
  - **AC-WRITING-013.6a:** `--reset-settings` restores System and Screenplay and queues a defaults write without a manual save. Reset is not a document operation and displays no success dialog.
  - **AC-WRITING-013.6b:** No shutdown flush is promised; a change made inside the debounce interval may be lost on exit. Independent processes neither watch nor merge the file, and the last completed atomic save wins. A child launched during debounce may read older on-disk globals, while its explicit template argument remains authoritative.

## Schema changelog

- **Version 1:** Initial schema containing `schema_version`, `theme`, and `template`; theme choices are System, Day, Night, and Midnight, and template is a stable built-in FDX catalog ID. Field, variant, or template-ID removal/renaming requires an explicit migration note and step. During alpha, unknown keys are tolerated on read but are not promised to survive a later rewrite.

## Related documents

- [Product overview](../product-overview.md)
- [Desktop workspace requirements](desktop-workspace.md)
- [Native document lifecycle requirements](native-document-lifecycle.md)
- [User settings component blueprint](../../blueprints/components/user-settings.md)

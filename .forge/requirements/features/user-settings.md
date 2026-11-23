# Requirements Area: User Settings

> This document specifies the established user-facing contract for global preferences. It describes desired behavior, not implementation status, validation, or acceptance.

## Area purpose

User settings retain a writer's small set of app-wide preferences across launches without making those preferences part of a screenplay or native FDX project.

## Global preference scope and schema

- **REQ-WRITING-LOOP-013: Persistent global user settings.** ScriptCraft shall retain the user's supported global appearance, default new-document template, ruler-visibility, and main/Title Page toolbar-composition choices while keeping them separate from screenplay documents and editing state.
  - **AC-WRITING-013.1:** Schema version 1 contains exactly the `schema_version`, `theme`, `template`, `show_ruler`, `toolbar`, and `title_page_toolbar` fields, defaulting to System, `screenplay`, shown (`true`), and the exact current-layout toolbar ID arrays specified below. Missing fields use those version-1 defaults, including when an older settings file lacks `show_ruler` or either toolbar array. Removing or renaming a field, theme variant, template ID, or built-in toolbar item ID requires an explicit migration note and step.
  - **AC-WRITING-013.2:** The persisted theme choice is exactly one of System, Day, Night, or Midnight, encoded as the lowercase values `system`, `day`, `night`, or `midnight`. Only the choice is stored. System resolves to Day or Night from egui's supplied OS theme and follows live changes where egui supplies them; it never resolves to Midnight. Midnight is selected explicitly.
  - **AC-WRITING-013.3:** The persisted default template is a stable ID from the packaged FDX built-in catalog, not a display name, path, or template contents. The FDX catalog remains the sole registry and validator. The two toolbar arrays are global presentation preferences and do not define new commands or item presentation.

## Startup and launch precedence

- **AC-WRITING-013.4:** Settings load before the first rendered application content. A no-document launch and ordinary File → New use the saved default template. An explicit internal New from Template selection wins for that child launch. A supplied FDX or FDXT document path takes precedence over the saved template; opening never applies the preference to the document or changes its definitions, content, native path, or baseline. An unknown saved template ID is retained as stored, produces a warning, and uses Screenplay only as the effective launch/selector fallback without rewriting settings because of fallback. A persisted default does not consume or finalize the macOS cold-open initial document slot; only the explicit internal template argument retains that existing behavior.

## Automatic persistence and state boundaries

- **AC-WRITING-013.5:** Real preference changes save automatically without a manual Save action or success dialog. Appearance changes persist when applied through the existing typed command. New from Template persists the selected ID only after the independent child process launches successfully; browsing, Preview, Cancel, Apply Changed/NoOp, failed spawn, and ordinary File New do not independently update it. Successful spawn is the boundary even if the child later fails to open its document. Preference changes do not dirty screenplay state or alter undo/history, layout identity, or exports.

## Recovery, reset, and process limits

  - **AC-WRITING-013.6:** Missing files load defaults without creating a file merely because the app launched; partial files use defaults for missing fields; unknown keys are tolerated. An unknown theme resolves to System while retaining other valid values. Unknown template and toolbar IDs remain raw in memory and are not sanitized by an unrelated save. Toolbar resolution omits unknown IDs from the effective ordered list, preserves known order and repetitions, and emits at most one bounded startup warning per affected toolbar containing only the omitted count and surface (never raw IDs); if omission leaves no entries, the corresponding default list is used. Missing or explicitly empty toolbar arrays also use their corresponding current-layout default. Wrong toolbar field types and non-string array members are ordinary malformed-TOML inputs and follow whole-file backup/default recovery without coercion. Malformed, unreadable, or unsupported-version files are backed up before defaults are used. Backup names are non-destructive (`settings.toml.bak`, then available bounded numeric suffixes); if preservation fails, ordinary persistence is disabled for that process. Missing schema version means version 1; only version 1 is emitted. Ordinary load, filtering, and fallback do not themselves save; a later unrelated complete-snapshot write retains the raw toolbar arrays held in settings.
  - **AC-WRITING-013.6a:** `--reset-settings` restores System, Screenplay, shown ruler visibility, and both exact default toolbar arrays specified below, and queues a defaults write without a manual save. Reset is not a document operation and displays no success dialog.
  - **AC-WRITING-013.6b:** No shutdown flush is promised; a change made inside the debounce interval may be lost on exit. Independent processes neither watch nor merge the file, and the last completed atomic save wins. A child launched during debounce may read older on-disk globals, while its explicit template argument remains authoritative.
  - **AC-WRITING-013.7:** The `show_ruler` preference is global and defaults to shown. The View and toolbar Show/Hide Ruler actions change it through the same real toggle; `--reset-settings` also restores shown visibility as specified in AC-WRITING-013.6a. Loading an older or partial file without the field yields shown visibility; merely loading settings or rendering the ruler does not write settings. A real toggle uses the existing no-op-aware settings update and complete-snapshot writer, without adding a write path. Ruler preference changes do not alter a document, session history, layout identity, or exports; the existing recovery and process limits in AC-WRITING-013.6–.6b remain unchanged.

## Schema changelog

- **Version 1:** Schema containing exactly `schema_version`, `theme`, `template`, `show_ruler`, `toolbar`, and `title_page_toolbar`; theme choices are System, Day, Night, and Midnight, template is a stable built-in FDX catalog ID, and `show_ruler` defaults to `true`. The ordered toolbar arrays contain stable built-in item IDs and default to the exact current layouts below. Missing fields use their version-1 defaults. Toolbar unknown IDs are omitted only from the nonmutating effective projection; raw values survive unrelated complete-snapshot writes, while wrong types/non-string members follow existing whole-file recovery. Field, variant, template-ID, or toolbar-item-ID removal/renaming requires an explicit migration note and step. During alpha, unrelated unknown keys are tolerated on read but are not promised to survive a later rewrite.

## Toolbar composition preference

The `toolbar` and `title_page_toolbar` arrays are independent, process-global ordered selections of built-in toolbar item IDs. They control item membership and order only. Captions, accessibility names, graphics, commands, special dropdown/group behavior, initial disabled placeholders, measurements, and presentation overrides remain built in. Flexible space is an ordinary list entry; repeated known entries and their order are retained, and no flexible spacing is injected outside the arrays. The current defaults are:

```toml
toolbar = [
  "flexible-space",
  "new",
  "open",
  "save",
  "save-as",
  "flexible-space",
  "title-page",
  "elements",
  "flexible-space",
  "show-hide",
  "export-text",
  "export-pdf",
  "flexible-space",
  "overflow",
]
title_page_toolbar = [
  "flexible-space",
  "styles",
  "alignment",
  "flexible-space",
  "close-title-page",
  "overflow",
]
```

The arrays are global preferences, not Project, EditingSession, FDX, command-wire, layout, export, dirty/history, or native-save state. Resolution is nonmutating and occurs once during app construction; the existing toolbar continues to project live availability and dispatch behavior. Overflow is itself a built-in item ID: when a custom list omits it, nonfitting items have no overflow affordance and no static indicator is synthesized; other menus and command entry points remain unchanged. Repeated rendered entries use ordered position as well as item ID for widget identity. No toolbar editor, drag/drop customization, live reload, or user-defined item presentation is included.

## Related documents

- [Product overview](../product-overview.md)
- [Desktop workspace requirements](desktop-workspace.md)
- [Native document lifecycle requirements](native-document-lifecycle.md)
- [User settings component blueprint](../../blueprints/components/user-settings.md)

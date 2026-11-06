# WO-013 — Complete disabled placeholder menu catalog

- **Status:** Implementation submitted for user review/validation; not accepted or validated
- **Initiative:** [`menu-system-v1.md`](../menu-system-v1.md)
- **Depends on:** [`WO-012`](WO-012-sf-symbols-edit-menu-and-closeout.md)
- **Validation owner:** User; no agent-run checks, reviews, or validators

## Single outcome

Expose the complete structure in [`final-draft-13-menus.md`](../../../.reference/final-draft-13-menus.md) through both existing renderers, while preserving every currently functional command and making every newly added entry visibly disabled.

One order is sufficient because native macOS and in-window egui already consume the same [`menu_items.rs`](../../../apps/scriptcraft/src/menu_items.rs) catalog. Backend work is limited to rendering the new placeholder metadata.

## Scope

1. Extend the catalog model with a clear behavior distinction between an application command and a disabled placeholder. Do not create placeholder AppCommand variants or no-op command handlers. Existing items retain their current commands, availability, labels, icons, and shortcuts.
2. Add the full ordered top-level sequence File, Edit, View, Format, Insert, Document, Tools, Production, Window, and Help. The native macOS Application menu remains a separate native-only standard menu; the in-window strip does not add it.
3. Populate every new entry and separator from the reference in its existing group/order. Keep all newly added items disabled even if similar functionality exists elsewhere, including Focus Mode, Appearance, and Sprint Timer. Existing toolbar/status functionality remains available through its current controls.
4. Preserve the current functional File/Edit rows rather than duplicating or replacing them with snapshot-specific labels. In particular, keep current Undo/Redo labels and behavior instead of using the reference's Undo Change View and Can't Redo state text.
5. Support disclosure-only placeholders for reference entries ending in `>`. Show a disabled submenu/caret row in both renderers, but do not invent child contents or implement functional submenu navigation. Native may use an empty disabled submenu; egui may show a disabled row with a trailing disclosure glyph.
6. Represent the checked placeholders for Script - Page View and blank document.fdx as disabled checked rows. Do not make view/document state dynamic in this order. Preserve those visible checkmarks in native and egui presentations.
7. Represent the Help search-field marker as a disabled, noninteractive Search placeholder with an accessible name. Do not add a text input, search behavior, focus target, or command.
8. Expand display-only shortcut metadata to cover every referenced key and modifier form, including Control, Option, Shift, Return/Enter, punctuation, brackets, backslash/pipe, and right-arrow forms. Native disabled items may carry accelerators for subdued native display. Egui shows the corresponding disabled shortcut text.
9. Keep placeholder shortcuts out of raw-input command recognition, native ID-to-command mapping, pending queues, and all other dispatch paths. Disabled placeholders must not emit events that can be mapped to an application command.
10. Leave all new placeholder icons unassigned unless the reference and current native/egui mapping already provide one. Preserve existing SF Symbols and Material icons on functional rows. Do not add new SF Symbols as part of this order.
11. Preserve top-level click/hover switching, row alignment, popup height bounds, accessibility naming, and current menu panel geometry. All new top-level menus remain clickable so users can inspect their disabled contents.
12. Update the menu requirement and technical context with the complete placeholder catalog contract and the limited disclosure/checked/search placeholder capabilities. Record implementation-submission facts in the project manifest and append the WO-013 handoff to the initiative.

## Explicit boundaries

- No functionality behind any newly added placeholder.
- No functional submenu contents, recursive menu navigation, or new application command.
- No dynamic recent-file, document-tab, Undo-label, view-mode, or checked-state projection.
- No new native Application-menu changes and no production preview switch change.
- No screenplay-core, FDX, persistence, pagination, export, toolbar, or status behavior changes.

## Structural acceptance for user review

- Both backends render the same complete ordered catalog from one source.
- Every pre-existing functional row behaves exactly as before; every new row is visibly and behaviorally disabled.
- Disclosure, checked, and Search placeholders carry no command or interactive behavior.
- All reference shortcut labels appear without becoming live shortcuts.
- No placeholder can enter the shared command queue or native command map.

## User-owned behavior review

- Compare every top-level menu and item against the reference for order, labels, separators, disclosure markers, checkmarks, and shortcut text.
- Verify all current New/Open/Save/Edit commands and shortcuts still work exactly once.
- Attempt pointer, keyboard, and accessibility activation of every placeholder and confirm no operation occurs.
- Inspect native and egui disabled colors, checked rows, caret rows, Help Search, popup height/scrolling, and top-level hover switching.
- Check ordinary and narrow window widths, all Appearance modes, and native display scales; record any top-level crowding without silently redesigning the bar.

## Handoff requirements

Record exact model changes, placeholder counts/categories, new shortcut metadata, backend rendering adaptations, any reference interpretation such as Search or static document text, and all unverified behavior. Implementation submission is not validation or acceptance.

## WO-013 implementation handoff — 2026-11-06

- **Checkout and state:** Implemented on the existing `dev/2026-11-06-menu-system` branch as authorized. Existing work was preserved; no branch or commit was created. This is an implementation submission only.
- **Model and counts:** The shared model distinguishes `MenuItemBehavior::Command(AppCommand)` from `MenuItemBehavior::DisabledPlaceholder(PlaceholderKind::{Item, Disclosure, Checked, Search})`. It contains ten top-level menus in reference order: File, Edit, View, Format, Insert, Document, Tools, Production, Window, Help. Thirteen existing File/Edit command entries remain functional with their prior labels, icons, availability, shortcut metadata, aliases, and behavior. The other 132 rows are disabled: 99 ordinary, 30 disclosure-only, two statically checked, and one Search placeholder. Script - Page View and `blank document.fdx` retain static checkmarks. `>` rows have no children; Search is a disabled accessible text label rather than a field or focus target. No placeholder AppCommand, no-op handler, or icon was added.
- **Shortcut metadata and routing:** Thirty-three placeholder shortcut labels are stored as display/native accelerator metadata for Command/primary, Control, Option, Shift, alphabetic and special key forms including Return, punctuation, brackets, backslash/pipe, and right arrow. Egui raw-input matching accepts command behavior only; the native command map likewise contains only real commands, so placeholder shortcuts/IDs do not reach the shared pending command queue or dispatcher. Native disabled rows may carry accelerators solely for subdued display.
- **Backend changes:** Egui uses the existing aligned row/shortcut treatment, with disabled caret disclosure, static checkmark decoration, and disabled Search row; the existing ten-menu click/hover switching, menu bar height, and popup height bounds are retained. Native macOS renders disabled ordinary/Search items, checked placeholders with disabled check items, and disabled empty disclosure submenus. The native standard Application menu remains separate and native-only. Existing Material and SF Symbol mappings are unchanged.
- **Durable records:** Updated REQ-WRITING-LOOP-011 and technical context for the full shared catalog and placeholder contract, project-manifest submission facts, and the initiative handoff. Reconciled the initial File/Edit-only and no-disclosure non-goals: WO-013 permits only empty disabled disclosure rows; functional submenu contents/navigation remain out of scope.
- **Changed source paths:** `apps/scriptcraft/src/menu_items.rs`, `apps/scriptcraft/src/menu.rs`, and `apps/scriptcraft/src/macos_menu.rs`. No dependency, lockfile, core, toolbar, status, persistence, or production preview-switch change was made.
- **Evidence limitations:** Repository policy prohibited agent-run builds, checks, tests, lint, formatting, launches, manual verification, reviews, and independent validators; none was run. Compilation, native menu rendering/accelerators, accessibility, disabled activation behavior, and egui visual/interaction behavior remain unverified and user-owned. No acceptance or validation is claimed.

### User-authorized Git closeout — 2026-11-06

The user explicitly authorized committing the complete current ScriptCraft menu-system change set on `dev/2026-11-06-menu-system` and merging that branch back to `master`. This authorization includes the initiative's source, dependency and lockfile, font/documentation/reference/plan, and other untracked initiative files; no current work is to be discarded. The requested merge uses ordinary non-destructive Git behavior, with no force, history rewrite, remote, or pull request; divergence or conflicts require stopping rather than overriding master. The resulting implementation commit and merge/final-master evidence are to be taken from Git history after those operations; no commit hash or merge result is asserted here before Git creates it. This Git closeout does not change the user-owned validation policy or imply review, validation, or acceptance.

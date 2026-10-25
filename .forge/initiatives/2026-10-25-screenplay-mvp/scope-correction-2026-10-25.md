# Post-MVP Scope Correction: Live Editing Surface and Basic Persistence UI

## Status

- **Artifact type:** Concise initiative plan amendment / design note
- **Requested by:** User, 2026-10-25
- **Decision status:** Scoped for planning; not approved for implementation
- **Implementation status:** Not implemented
- **Validation status:** No new validation claimed

## User-requested correction

The post-MVP review found that the current desktop shell does not provide the
intended screenplay writing surface. The next work must consolidate editing into
one primary live document surface with a caret, direct typing, selection,
deletion, and role-aware editing. Page layout remains the default visual
presentation; it must not be split into a separate non-editable page-preview
mode, and the duplicate per-element text-entry UI must be removed.

The existing core APIs already provide the relevant transactional primitives:
`EditingSession` supports selection, insertion, deletion, backspace, role change,
Enter transitions, undo, and redo; `ScreenplayLayout` provides source mappings,
hit testing, and caret geometry. The correction is therefore scoped primarily to
the desktop adapter and its interaction tests, not a redesign of the semantic
core or layout algorithm.

The review also found that the current Open and Save actions always use the
working-directory `scriptcraft.json`. A separate narrow work order is justified
for basic user-selected/named Open, Save, and Save As behavior. Recovery remains
an outside-the-UI/core mechanism for this correction; no recovery browser or
recovery workflow is added to the UI. Persistence UI scope is deliberately
limited to basic named file selection and status/error handling.

## Dependency decision

Create two sequential draft work orders after WO-008:

1. **WO-009 — Unified live document editing surface:** depends on WO-004, WO-005,
   and WO-008; it establishes the final single editing/rendering surface.
2. **WO-010 — Basic named persistence UI:** depends on WO-006, WO-009, and WO-008;
   it adds only user-selected/named Open, Save, and Save As integration to the
   unified surface.

The split is intentional: live editing interaction and persistence path
selection have distinct ownership and validation boundaries, while WO-010 is
ordered after WO-009 so it cannot reintroduce a second editor or toolbar mode.

## Evidence preserved

The already-passed validations for WO-001 through WO-008 remain historical
evidence and are not invalidated or rewritten by this scope correction. They
validated the implemented prior boundaries; they did not validate the newly
requested post-MVP UI behavior. No approval, implementation, or validation is
claimed for WO-009 or WO-010.


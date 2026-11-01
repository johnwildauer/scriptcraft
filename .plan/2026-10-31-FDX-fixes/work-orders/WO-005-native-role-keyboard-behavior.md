# WO-005 — Native role keyboard behavior

- **Status:** Implementation authored; user-owned review and validation pending
- **Initiative:** [FDX fixes](../fdx-fixes.md)
- **Depends on:** [WO-001](WO-001-shared-element-formatting-and-native-defaults.md)
- **Execution preference:** Execute after WO-004 for simple sequential ownership, although it does not technically depend on WO-002 through WO-004
- **Validation owner:** User; no agent-run checks or independent validators

## Outcome

Align application-default Enter and Tab transitions with the native Behavior defaults evidenced in both FDX references, while preserving established Dialogue interior splitting, blank-Character Tab behavior, automatic parentheticals, strict boundaries, undo/redo, canonical text, and imported Behavior XML.

## Current evidence

- [`enter_transition()`](../../../crates/scriptcraft-core/src/editing.rs:1365) retains most added roles on Enter.
- [`tab_transition()`](../../../crates/scriptcraft-core/src/editing.rs:1389) consumes Tab as a no-op for most added roles.
- Native Behavior values are present in both [the blank reference](../../../.reference/reference.fdx:61) and [the populated reference](../../../.reference/reference2.fdx:172), and WO-001 includes them in the shared default table.
- Existing Dialogue interior Enter handling in [`EditingSession::enter()`](../../../crates/scriptcraft-core/src/editing.rs:449) and the special policies in [AC-WRITING-001.4a](../../../.forge/requirements/features/screenplay-writing-loop.md:36) must remain intact.

## Scope

1. Use the shared native Behavior defaults for application-default ReturnKey and TabKey role targets instead of maintaining a divergent added-role mapping.
2. Implement the exact default matrix below for all 17 roles:

| Current role | Enter target | Tab target |
|---|---|---|
| General | General | General |
| Scene Heading | Action | Action |
| Action | Action | Character |
| Character | Dialogue | Transition only when current text is empty; otherwise consumed no-op |
| Parenthetical | Dialogue | Dialogue |
| Dialogue | Action at collapsed end; Dialogue split at interior | Parenthetical |
| Transition | Scene Heading | Scene Heading |
| Shot | Action | Action |
| Cast List | Action | Action |
| New Act | Scene Heading | Scene Heading |
| Sequence | Scene Heading | Scene Heading |
| End of Act | New Act | New Act |
| Summary | Summary | Outline 1 |
| Outline 1 | Summary | Outline 2 |
| Outline 2 | Summary | Outline 3 |
| Outline 3 | Summary | Note |
| Note | Summary | Summary |

3. Preserve all existing special handling around the matrix:
   - Dialogue Enter splits at a collapsed interior UTF-8 boundary and uses Action only at the collapsed end.
   - Character Tab changes to Transition only for empty current text.
   - Same-role Tab targets are consumed no-ops. Same-role Enter targets still use the normal split/edit rule and create an undoable paragraph split.
   - Automatic Parenthetical generation/cleanup and strict transition-boundary checks remain governed by the existing runtime policies.
4. Make every successful role-changing transition transactional and undoable, with selection placed by the existing edit operation.
5. Keep imported per-document Behavior values preserved but do not let them change application editing behavior in this order. Runtime transitions use application defaults only.
6. Update the durable editing matrix and technical contract to reflect the newly approved defaults.

## Explicit non-goals

- No shortcut-key handling, macros, WinShortcut, PaginateAs, SmartType, or outline navigation implementation.
- No custom imported Behavior execution or formatting-settings editor.
- No change to cue-casing policy, AllCaps presentation, automatic-parenthetical policy, strict boundaries, clipboard behavior, or vertical navigation.
- No change to native serialization of imported Behavior or to WO-001 default emission.
- No geometry, typography, direct paragraph, title-page, or PDF changes.

## Detailed behavior contract

### Application defaults versus document settings

The shared table supplies default targets for this application session policy. It does not make Behavior a persisted ScriptCraft editing-policy field and does not authorize runtime customization from imported files. Imported Behavior remains source-owned FDX and round-trips unchanged.

### Enter

- Preserve the existing collapsed-selection and UTF-8 validation requirements.
- Dialogue interior Enter retains the exact prefix in the original element and exact suffix in a new Dialogue, including existing run/source safety rules. Dialogue Enter at the end creates Action.
- For all other roles, use the matrix target through the existing split/transition path.
- Enter from Note now creates Summary; Enter from the outline roles creates Summary; Enter from End of Act creates New Act.
- A same-role Enter such as General, Summary, or Action retains that role for the newly split element; it is not a no-op when text splitting is otherwise required.

### Tab

- Tab is consumed and never inserts a literal tab.
- A real role change is transactional and undoable. Same-role targets and undefined/special-case no-ops do not mutate the project or add history.
- Character retains the established empty-text condition even though native Behavior names Transition without that condition.
- Dialogue and Parenthetical retain automatic-parenthetical behavior under the current policy.
- Outline Tab follows Outline 1 → Outline 2 → Outline 3 → Note, and Note → Summary. Summary → Outline 1 completes the cycle.

### Safety and preservation

- Rejected non-collapsed selections, invalid UTF-8 boundaries, capacity failures, and unsupported source intersections preserve the complete prior session state.
- Undo/redo restores role, text, selection, run data, and preservation state together.
- Changing runtime keyboard defaults does not rewrite imported Behavior XML, source paragraph attributes, or canonical text casing.

## Suggested implementation steps

1. Replace the added-role self/no-op mappings in [editing.rs](../../../crates/scriptcraft-core/src/editing.rs:1360) with lookups into the WO-001 shared Behavior defaults, retaining explicit application special cases.
2. Keep Dialogue interior Enter handling ahead of the generic end-of-element transition.
3. Preserve same-role/no-op distinctions in the session mutation methods rather than treating every matrix target as an unconditional mutation.
4. Add exhaustive all-role Enter/Tab tests plus special-case, undo/redo, source-preservation, and failure-atomicity regressions.
5. Update [AC-WRITING-001.4a](../../../.forge/requirements/features/screenplay-writing-loop.md:36) and [the technical context](../../../.forge/blueprints/technical-context.md:43) with the approved matrix and application-default/import-preservation distinction.

## Allowed implementation surface

Product paths:

- [editing.rs](../../../crates/scriptcraft-core/src/editing.rs)
- [formatting.rs](../../../crates/scriptcraft-core/src/formatting.rs) only for the shared Behavior lookup contract
- Focused core editing and integration tests

Durable records:

- [FDX fixes](../fdx-fixes.md)
- [Writing-loop requirements](../../../.forge/requirements/features/screenplay-writing-loop.md)
- [Technical context](../../../.forge/blueprints/technical-context.md)
- [Product overview](../../../.forge/requirements/product-overview.md) only if its editing summary requires reconciliation

Do not modify layout, FDX parsing/writing, preservation, desktop painting, exports, font assets, or native filesystem storage.

## Acceptance criteria

1. Every role has the exact Enter and Tab target shown in the matrix, subject only to the stated Dialogue and blank-Character special cases.
2. Enter from Shot/Cast List/New Act/Sequence produces Action/Action/Scene Heading/Scene Heading respectively; End of Act produces New Act.
3. Outline and Note transitions follow Summary/Outline/Note cycling exactly, and same-role operations have the specified split versus no-op behavior.
4. Dialogue interior Enter retains its existing source-safe transactional split; Dialogue end Enter produces Action.
5. Nonempty Character Tab remains a consumed no-op; empty Character Tab produces Transition.
6. Automatic Parenthetical behavior and strict transition boundaries remain unchanged under both enabled and disabled runtime policies where currently supported.
7. Every accepted role change is undoable/redoable with correct selection and complete preservation state; every rejected operation is atomic.
8. Imported custom Behavior values remain unchanged after editing and save/reopen and do not alter the application-default matrix.
9. No literal tab, canonical-case rewrite, geometry, typography, or export behavior changes occur.

## Required test authoring and user validation handoff

Author exhaustive matrix tests for all 17 roles, collapsed/interior/end Dialogue positions, empty/nonempty Character, same-role splits/no-ops, automatic Parenthetical interactions, strict boundaries, UTF-8 safety, undo/redo, capacity failure, and imported custom Behavior preservation. Do not run them under repository policy.

The user should run the applicable workspace checks and manually exercise one complete outline chain, Note → Summary → Outline 1, act/sequence transitions, ordinary dialogue entry, and undo/redo after each new role transition. Reopen an imported file with customized Behavior and confirm both unchanged XML and the approved application-default keyboard behavior.

## Implementation handoff — 2026-11-01

- **State:** Runtime keyboard implementation and focused test authoring are present. WO-001–004 remain unvalidated; this record does not establish acceptance of WO-005 or any preceding order.
- **Implementation:** Enter and Tab obtain their target labels from the shared `formatting_for_role()` Behavior defaults and resolve labels against the shared native-order formatting table. Imported Behavior is not projected into runtime editing policy. Existing Dialogue interior splitting, blank-Character Tab handling, automatic Parenthetical behavior, strict boundaries, transactional mutation, history, and source reconciliation remain in the session paths.
- **Tests authored:** [`wo005_native_role_keyboard_behavior.rs`](../../../crates/scriptcraft-core/tests/wo005_native_role_keyboard_behavior.rs) covers the exact 17-role Enter/Tab matrices, same-role split versus no-op, Dialogue interior/end, empty/nonempty Character, Parenthetical policy, enabled/disabled special policies, strict boundaries, invalid UTF-8 and capacity failure atomicity, undo/redo, imported custom Behavior, and WO-004 direct-attribute split identity/no-copy preservation.
- **Verification owner:** Repository policy prohibits agent-run tests, builds, lint, formatting checks, implementation checks, manual verification, and validator dispatch. All such verification remains user-owned and unrun here.

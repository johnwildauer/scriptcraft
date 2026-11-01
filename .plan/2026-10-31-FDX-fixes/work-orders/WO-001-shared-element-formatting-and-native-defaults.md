# WO-001 — Shared element formatting and native default emission

- **Status:** Implementation authored; user-owned verification outstanding (not validated)
- **Initiative:** [FDX fixes](../fdx-fixes.md)
- **Depends on:** None
- **Execution preference:** Complete before WO-002, WO-003, WO-004, and WO-005
- **Validation owner:** User; no agent-run checks or independent validators

## Outcome

Create one typed, complete, ordered formatting table for all 17 directed screenplay roles and use it to emit native FDX ElementSettings in newly created files. Establish the shared module and fallback contracts needed by the later geometry, typography, direct-override, and behavior orders without rewriting imported source settings.

## Current evidence

- [`Role::ALL`](../../../crates/scriptcraft-core/src/lib.rs:150) defines all 17 roles, but formatting knowledge is split between [`LayoutInputs`](../../../crates/scriptcraft-core/src/lib.rs:390), [`SourceLayoutSettings`](../../../crates/scriptcraft-core/src/lib.rs:460), layout helpers, and editing helpers.
- [`make_new_document_nodes()`](../../../crates/scriptcraft-core/src/fdx.rs:808) emits no ElementSettings for a new native document.
- [`read_source_layout_settings()`](../../../crates/scriptcraft-core/src/fdx.rs:1789) currently interprets only HiddenText and StartsNewPage. Imported XML remains authoritative through [`PreservationState`](../../../crates/scriptcraft-core/src/preservation.rs:271).
- Both [the blank reference](../../../.reference/reference.fdx:61) and [the populated reference](../../../.reference/reference2.fdx:172) contain matching complete role defaults.

## Scope

1. Add [formatting.rs](../../../crates/scriptcraft-core/src/formatting.rs) as the core owner of typed element formatting and defaults.
2. Represent the four native setting categories needed by this initiative:
   - FontSpec: family, size, style tokens, color, and the other evidenced attributes required for deterministic native emission.
   - ParagraphSpec: alignment, first/left/right indents, leading, space before, spacing, page-start behavior, and type.
   - Behavior: pagination label, ReturnKey, TabKey, and platform shortcut values.
   - Outline: CanHide and Level.
3. Encode the exact default values evidenced by both references for every role. Use the native reference order: General, Scene Heading, Action, Character, Parenthetical, Dialogue, Transition, Shot, Cast List, New Act, Sequence, End of Act, Summary, Outline 1, Outline 2, Outline 3, Note.
4. Provide role-based lookup without duplicating a second role-to-format table elsewhere. Keep native role spelling and FDX label conversion compatible with [`role_to_fdx()`](../../../crates/scriptcraft-core/src/fdx.rs:901).
5. Generate ordered ElementSettings nodes from that table in the new-document branch of [`write_v6()`](../../../crates/scriptcraft-core/src/fdx.rs:370). Emit FontSpec, ParagraphSpec, Behavior, and Outline children with deterministic attribute order.
6. Use the shared defaults for the two already interpreted runtime behaviors—HiddenText and StartsNewPage—when no imported override exists. This keeps those two behaviors consistent before and after a new document is first saved and reopened.
7. Keep the remaining new default categories dormant until their owning orders: geometry and spacing in WO-002, typography and editor/print visibility in WO-003, direct paragraph overrides in WO-004, and runtime Enter/Tab alignment in WO-005.
8. Move or re-export shared formatting types cleanly so consumers do not depend on duplicate definitions in the core root module.

## Explicit non-goals

- No imported-setting regeneration, normalization, or insertion into files that already have preservation state.
- No new geometry, spacing, font rendering, direct paragraph override, or Enter/Tab behavior beyond the currently recognized defaults described above.
- No custom element editor, formatting UI, private FDX extension, sidecar, title-page formatting, run-level styling, or Final Draft compatibility claim.
- No change to atomic filesystem save behavior.

## Implementation contract

### Shared default table

The table is the single source for native emission and application fallback values. It must include the complete values shown in the investigation's native-default table, including:

- Courier Final Draft 12-point defaults for the non-outline roles.
- Arial and the evidenced sizes/styles for Note, Summary, and Outline 1–3.
- All native indentation, SpaceBefore, alignment, and StartsNewPage values.
- All native ReturnKey, TabKey, Shortcut, WinShortcut, PaginateAs, CanHide, and Level values.
- Native color and other evidenced attributes needed to reproduce the default ElementSettings structurally, even when runtime interpretation is deferred.

Do not derive defaults from only one role or reuse Action as a fallback for added roles. Missing imported values resolve through the table on a per-field basis in later orders.

### New native documents

When the project has no preservation state, insert the generated settings into the new root in deterministic order without disturbing Content or optional TitlePage generation. The resulting document must pass the existing bounded re-read and semantic comparisons in [`write_v6()`](../../../crates/scriptcraft-core/src/fdx.rs:370).

When the project has preservation state, follow the existing clone/reconcile path unchanged. A runtime projection or a change in recognized fields must not alter preserved ElementSettings.

### Existing recognized defaults

A new in-memory project and the same project after native save/reopen must agree that:

- New Act starts a new page by default.
- Note, Summary, and Outline 1–3 carry native HiddenText defaults.

Until WO-003 introduces separate editor and print purposes, the existing shared-layout visibility behavior may remain. WO-003 must replace it with the approved visible-in-editor/hidden-in-print contract.

## Suggested implementation steps

1. Add the formatting module and register it from [the core module root](../../../crates/scriptcraft-core/src/lib.rs:6).
2. Define typed settings, defaults, role lookup, and deterministic native-order iteration with bounded values.
3. Move SourceLayoutSettings or provide a compatibility re-export so existing preservation/layout consumers use the shared formatting definitions.
4. Add native node construction helpers in [the FDX codec](../../../crates/scriptcraft-core/src/fdx.rs:808), delegating values and ordering to the formatting module.
5. Resolve currently recognized HiddenText and StartsNewPage defaults through the shared table.
6. Add headless regression coverage for table completeness, exact selected values, native emission, imported preservation, and pre/post-save default equivalence.
7. Update durable documents only where the approved native default contract changes their statements.

## Allowed implementation surface

Product paths:

- [formatting.rs](../../../crates/scriptcraft-core/src/formatting.rs)
- [lib.rs](../../../crates/scriptcraft-core/src/lib.rs)
- [fdx.rs](../../../crates/scriptcraft-core/src/fdx.rs)
- [preservation.rs](../../../crates/scriptcraft-core/src/preservation.rs) only if type movement requires it
- [layout.rs](../../../crates/scriptcraft-core/src/layout.rs) only for the currently recognized default visibility/page-start fallback
- Focused core unit/integration tests and synthetic fixtures

Durable records:

- [FDX fixes](../fdx-fixes.md)
- [Product overview](../../../.forge/requirements/product-overview.md)
- [Writing-loop requirements](../../../.forge/requirements/features/screenplay-writing-loop.md)
- [Technical context](../../../.forge/blueprints/technical-context.md)

Do not edit desktop rendering, PDF export, editing transitions, font assets, Cargo dependencies, or filesystem storage in this order.

## Acceptance criteria

1. One shared table exposes complete defaults for exactly 17 roles in the specified native order.
2. A newly saved FDX document contains one ElementSettings block per role, with FontSpec, ParagraphSpec, Behavior, and Outline children matching the two reference defaults.
3. Reopening that output preserves every emitted setting and produces no duplicate or ambiguous role settings.
4. Saving an imported document does not add missing defaults or rewrite existing source ElementSettings.
5. Unknown source attributes, style tokens, namespaces, and unrelated sections remain preserved under the existing writer contracts.
6. New Act page-start and native hidden-role defaults are equivalent before and after a new document's first save/reopen.
7. Geometry, typography, direct paragraph formatting, and Enter/Tab behavior not owned by this order remain unchanged.
8. The formatting table and writer behavior are deterministic for identical input.

## Required test authoring and user validation handoff

Author focused tests for the acceptance criteria, including a full 17-role audit and an imported-custom-settings preservation case. Do not run those tests or any other checks under repository policy.

The user should run the applicable workspace checks and inspect:

- A new native file's complete ElementSettings structure against both references.
- New-document save/reopen behavior for New Act and hidden roles.
- An imported file with customized settings to confirm source values are unchanged after save.
- Optional external Final Draft open/reopen of a disposable generated file, without inferring interoperability from XML structure alone.

## Handoff

WO-002 consumes the shared ParagraphSpec types and defaults. WO-003 consumes FontSpec types and defaults. WO-004 extends the same per-field fallback model to direct paragraph attributes. WO-005 consumes the native Behavior defaults but does not require WO-002 through WO-004 technically; sequential execution is still recommended to avoid shared-file conflicts.

## Implementation status and user verification handoff — 2026-11-01

- **Implementation state:** Source changes and focused regression-test authoring for this order are present. This is an implementation status only; no acceptance criterion is represented as validated.
- **Shared interface:** [`formatting.rs`](../../../crates/scriptcraft-core/src/formatting.rs) owns `FontSpec`, `ParagraphSpec`, `Behavior`, `Outline`, `RoleFormatting`, the ordered `ROLE_FORMATTING_DEFAULTS`, role lookup, and recognized fallback settings. `SourceLayoutSettings` and `PrintVisibility` remain available through core-root re-exports. WO-002 can consume `ParagraphSpec` and `formatting_for_role()` / `formatting_in_native_order()` without deriving a parallel defaults table.
- **Scope boundary:** Native settings are emitted only in the no-preservation new-document path. Source-backed ElementSettings remain on the existing clone/reconcile path. Runtime currently applies only the shared HiddenText and StartsNewPage defaults/recognized overrides; geometry, spacing, typography, direct paragraph formatting, and Enter/Tab behavior remain deferred to their owning orders. Hidden roles continue using the current shared-layout behavior until WO-003 implements the approved editor/print distinction.
- **Authored tests:** The regression suite covers the 17-role order, structural comparison against both checked-in references, new-document save/reopen runtime equivalence for hidden roles and New Act, and preservation of custom imported ElementSettings. These tests have **not** been run.
- **Policy and verification:** Under [the current project policy](../../../.forge/project-manifest.md), no agent review, validator dispatch, tests, builds, lint, formatting checks, implementation checks, or manual verification were performed. The user owns those checks. Before accepting this order, inspect the complete generated ElementSettings against both references, new-document save/reopen behavior, and imported customized-setting preservation. Optional external Final Draft inspection remains user-owned and is not implied by structural tests.
- **Limitations:** No Final Draft compatibility, rendering parity, or successful validation is claimed. Existing in-product save, re-read comparison, preservation, and failure-safety paths were not weakened or intentionally altered.
- **WO-002 handoff:** The shared paragraph interface and role defaults are now available for the sequential geometry/spacing order. WO-002 may be implemented against that interface, while WO-001's user-owned verification remains explicitly outstanding; this handoff does not mean WO-001 has passed acceptance.

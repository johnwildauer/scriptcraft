# WO-004 — Direct paragraph formatting overrides

- **Status:** Approved; implementation and focused test authoring recorded below, with validation user-owned and unrun
- **Initiative:** [FDX fixes](../fdx-fixes.md)
- **Depends on:** [WO-003](WO-003-role-typography-and-print-visibility.md)
- **Execution preference:** Complete after the role-level geometry and typography resolvers are stable
- **Validation owner:** User; no agent-run checks or independent validators

## Outcome

Honor validated formatting attributes stored directly on managed body Paragraph nodes, with correct precedence over role ElementSettings and application defaults, while preserving every source attribute and keeping title-page and run-level formatting outside this order.

## Current evidence

- The role-level source projection in [`read_source_layout_settings()`](../../../crates/scriptcraft-core/src/fdx.rs:1789) does not model direct paragraph overrides.
- Two Summary paragraphs in [the populated reference](../../../.reference/reference2.fdx:107) carry direct Alignment, FirstIndent, Leading, LeftIndent, RightIndent, SpaceBefore, Spacing, and StartsNewPage attributes. Their direct values associate them visually with the preceding outline level and differ from the Summary role default.
- Managed body paragraphs already have source provenance and transient semantic identities through [`make_semantic_element()`](../../../crates/scriptcraft-core/src/fdx.rs:1727), while their complete attributes remain in the preservation tree.

## Scope

1. Project supported direct formatting attributes from managed body Content Paragraph nodes:
   - Alignment
   - FirstIndent
   - Leading
   - LeftIndent
   - RightIndent
   - SpaceBefore
   - Spacing
   - StartsNewPage
2. Store the ephemeral direct-paragraph projection in a way that is keyed to the paragraph's current semantic identity and participates in layout cache equality.
3. Implement the approved effective-format precedence:
   - Valid direct paragraph value.
   - Valid role ElementSettings value.
   - Shared application default.
4. Apply precedence per field, not as an all-or-nothing paragraph choice. A paragraph with only SpaceBefore direct still inherits its other geometry and all typography from role/default settings.
5. Reuse WO-002 parsing, fixed-point conversion, validation, alignment, wrapping, SpaceBefore, StartsNewPage, and page-fit behavior. Reuse WO-003 role typography and visibility without duplicating them per paragraph.
6. Ensure newly created paragraphs with no direct attributes use role/default formatting.
7. Preserve direct attributes and their order through save/reopen. Runtime interpretation must not normalize, copy, remove, or regenerate them.

## Explicit non-goals

- No TitlePage override projection; title-page formatting remains preservation-only.
- No direct Text-run Font/Size/Style interpretation or mixed inline-style renderer.
- No OutlineLevel behavior, outline hierarchy, board, scene, or navigator changes.
- No new paragraph-level formatting UI or managed serialization of edited formatting.
- No Behavior, PaginateAs, custom element, or imported-settings rewriting.
- No support for direct attributes on opaque/unknown paragraphs beyond continued preservation.

## Detailed behavior contract

### Projection and identity

- Read direct attributes only from managed Paragraph nodes at body Content depth, using the same namespace and managed-role rules as the existing semantic reader.
- Associate parsed values with the same transient semantic element identity produced for that paragraph. Do not key overrides by ordinal alone, foreign ID string alone, or visible line index.
- Rebuild the projection deterministically on open; transient internal identity need not be durable.
- Preserve source attributes even when their values are unsupported, ambiguous, or outside runtime budgets.

### Parsing and ambiguity

- Reuse the WO-002 accepted values, bounds, and fallback behavior for each shared field.
- An absent direct attribute has no override and defers to role/default resolution.
- An unsupported or invalid direct value has no runtime override and remains preserved; it must not erase a valid role value.
- Unknown attributes and OutlineLevel are ignored by runtime layout and preserved unchanged.
- A direct StartsNewPage value overrides the role value only when explicitly present and valid.
- Do not infer direct font formatting from paragraph attributes or promote text-run attributes into paragraph settings.

### Layout and editing behavior

- Direct values affect only their owning paragraph, not adjacent paragraphs of the same role.
- SpaceBefore is controlled by the incoming paragraph's direct value when present; page-top suppression remains unchanged.
- FirstIndent applies only to that paragraph's first visual line. Alignment and line wrapping affect only that paragraph.
- HiddenText remains role-level in this order; direct paragraph geometry cannot make a hidden role print-visible or change its editor visibility.
- Splitting an element must not silently invent direct XML attributes for either output paragraph. If current structural-edit rules cannot safely define inherited direct formatting, retain the existing safe stop rather than duplicating or dropping source attributes.
- Role changes leave source-owned direct attributes preserved under existing reconciliation rules; the effective resolver still applies valid direct geometry above the newly selected role.

## Suggested implementation steps

1. Extend [formatting.rs](../../../crates/scriptcraft-core/src/formatting.rs) with a direct-paragraph optional settings projection and field-level resolver support.
2. During managed body paragraph import in [fdx.rs](../../../crates/scriptcraft-core/src/fdx.rs:1727), parse the direct attributes and associate them with the semantic element.
3. Extend [preservation.rs](../../../crates/scriptcraft-core/src/preservation.rs:270) only if needed to hold the ephemeral paragraph projection alongside the existing role projection; keep source XML authoritative.
4. Update [layout.rs](../../../crates/scriptcraft-core/src/layout.rs:147) to resolve each element through direct, role, then default values and include the projection in cache identity.
5. Confirm desktop and PDF consumers require no independent precedence logic; they continue consuming shared layout.
6. Add synthetic and reference-derived fixtures for field-level precedence, imported fallback, cache invalidation, editing boundaries, and source preservation.
7. Update durable technical contracts to distinguish direct paragraph overrides from role defaults.

## Allowed implementation surface

Product paths:

- [formatting.rs](../../../crates/scriptcraft-core/src/formatting.rs)
- [fdx.rs](../../../crates/scriptcraft-core/src/fdx.rs)
- [preservation.rs](../../../crates/scriptcraft-core/src/preservation.rs)
- [lib.rs](../../../crates/scriptcraft-core/src/lib.rs) only for the ephemeral paragraph association if required
- [layout.rs](../../../crates/scriptcraft-core/src/layout.rs)
- [editing.rs](../../../crates/scriptcraft-core/src/editing.rs) only if an existing split/join safety rule must explicitly preserve or reject direct-format intersections
- Focused core tests and synthetic fixtures

Durable records:

- [FDX fixes](../fdx-fixes.md)
- [Product overview](../../../.forge/requirements/product-overview.md)
- [Writing-loop requirements](../../../.forge/requirements/features/screenplay-writing-loop.md)
- [Technical context](../../../.forge/blueprints/technical-context.md)

Do not change desktop painting, PDF generation, fonts/assets, title-page interpretation, native storage, or keyboard behavior unless a small consumer correction is required by the shared resolver; any such correction must be recorded in the implementation handoff.

## Acceptance criteria

1. A direct valid value overrides only the same field for its owning body paragraph; all other fields continue through role/default resolution.
2. The two overridden Summary paragraphs from the populated reference use their direct left/right/SpaceBefore values while retaining Summary role typography and visibility.
3. Adjacent paragraphs with the same role but different direct attributes have independently correct geometry, wrapping, spacing, carets, and page behavior.
4. Unsupported or invalid direct values fall back to the role/default value without being removed or rewritten on save.
5. New paragraphs without direct attributes use role/default formatting.
6. Direct StartsNewPage overrides role behavior only for its paragraph and retains body-page-top SpaceBefore suppression.
7. Direct overrides participate in cache equality, and changing them cannot reuse stale geometry.
8. Save/reopen preserves source attribute order, values, unknown attributes, semantic roles/text, and effective override behavior.
9. Title-page paragraphs and opaque/unknown body paragraphs remain preservation-only.
10. Structural edits that intersect direct formatting either preserve it under a proven rule or stop safely without duplicating/dropping source data.

## Required test authoring and user validation handoff

Author tests using both minimal synthetic paragraphs and a fixture derived from the two direct Summary overrides in [reference2.fdx](../../../.reference/reference2.fdx:107). Cover per-field precedence, adjacent same-role paragraphs, every supported direct field, invalid fallback, source preservation, new paragraphs, page starts, cache identity, and split/join safety. Do not run them under repository policy.

The user should run the applicable workspace checks and inspect the populated reference's Outline 2/Summary and Outline 3/Summary groups in the editor and PDF. Confirm that direct geometry is honored while hidden Summary content remains visible/subdued only in the editor. External Final Draft comparison remains optional user-owned evidence.

## Handoff

This completes the planned native formatting precedence chain for body paragraphs: direct paragraph, role ElementSettings, then shared defaults. WO-005 is independent of this resolver and aligns application-default keyboard transitions without changing imported Behavior XML.

## Implementation handoff — 2026-11-01

- Direct supported body fields are projected into `SourceLayoutSettings` by transient managed semantic element ID. The map is part of layout/cache equality; title-page paragraphs and run-level attributes are not projected.
- Each supported field resolves direct > role > shared default independently. Unsupported direct values remain in preserved source XML and defer to role/default values. Role typography and HiddenText visibility continue to use the role resolver.
- Splitting keeps the original paragraph's direct attributes attached to the surviving original identity and emits no copied attributes for the new paragraph. A join is rejected transactionally if its removed/right paragraph has any supported direct-format attribute, including invalid-valued instances; explicitly deleting a paragraph retains existing delete semantics. Role changes preserve direct source attributes and continue resolving them for the same semantic element.
- Focused regression tests were authored in `crates/scriptcraft-core/tests/wo004_direct_paragraph_overrides.rs`, including the populated reference. No tests, builds, lint, formatting, implementation checks, manual verification, or validators were run. No acceptance or compatibility claim is made.
- WO-005 may proceed against the completed shared resolver interfaces; WO-001–004 remain user-unvalidated. User-owned checks and editor/PDF inspection remain outstanding.

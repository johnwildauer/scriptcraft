# WO-002 — FDX paragraph geometry and spacing

- **Status:** Approved; implementation authored; user validation pending
- **Initiative:** [FDX fixes](../fdx-fixes.md)
- **Depends on:** [WO-001](WO-001-shared-element-formatting-and-native-defaults.md)
- **Execution preference:** Complete before WO-003 and WO-004
- **Validation owner:** User; no agent-run checks or independent validators

## Outcome

Apply the shared native paragraph defaults and validated imported ParagraphSpec values to all 17 body roles. Introduce signed fixed-point geometry, per-role SpaceBefore, first-line indentation, alignment, and page-start behavior while preserving deterministic pagination, source mappings, canonical text, and source XML.

## Current evidence

- [`RoleBox`](../../../crates/scriptcraft-core/src/lib.rs:378) stores only unsigned integer x/width. Ten added roles receive a neutral full-width fallback in [`role_box()`](../../../crates/scriptcraft-core/src/layout.rs:486).
- [`role_boundary_gap()`](../../../crates/scriptcraft-core/src/layout.rs:509) applies a global gap rather than native per-role SpaceBefore.
- [`LayoutLine`](../../../crates/scriptcraft-core/src/layout.rs:35) has no first-line, alignment, or fixed-point representation.
- The native values include signed FirstIndent and fractional positions such as 0.88 and 7.10 in [the reference ParagraphSpec blocks](../../../.reference/reference2.fdx:172).
- The user approved signed fixed-point geometry as part of this order.

## Scope

1. Introduce a bounded signed fixed-point geometry representation with exactly 1/100-point precision for stored effective values and layout arithmetic. Native two-decimal inch values converted through 72 points per inch must not be truncated to whole points.
2. Project these role-level ParagraphSpec fields from imported FDX:
   - Alignment
   - FirstIndent
   - LeftIndent
   - RightIndent
   - Leading
   - SpaceBefore
   - Spacing
   - StartsNewPage
3. Resolve every projected field independently: a valid imported role value overrides the shared default; an absent or unsupported value retains the default. Direct paragraph attributes remain deferred to WO-004.
4. Apply native defaults for all 17 roles, replacing the neutral full-width fallback.
5. Interpret LeftIndent and RightIndent for these body settings as positions in inches from the page's left edge. Convert them to fixed-point coordinates and derive the available paragraph measure from right minus left.
6. Apply FirstIndent only to the first visual line of an element. Support negative values without unsigned underflow and adjust first-line available width accordingly.
7. Support native Left, Right, and Center alignment for body paragraphs. Alignment affects display geometry only; it never adds spaces, tabs, or other content to canonical text.
8. Replace the global incoming-role boundary gap with effective per-role SpaceBefore. Wrapped lines within one element continue to use the effective line advance.
9. Use actual fixed-point vertical extents for page-fit decisions rather than treating every nonzero gap as one logical line. Preserve bounded oversized-element termination and keep-with-next behavior.
10. Carry the effective geometry through desktop painting, caret/hit testing, selection, vertical navigation, PDF export, and layout caching.

## Explicit non-goals

- No direct paragraph overrides, title-page formatting, run-level formatting, or custom paragraph UI.
- No font-family/style/size mapping; WO-003 owns those changes.
- No native PaginateAs interpretation or change to existing keep-with-next roles.
- No imported Behavior mapping or Enter/Tab changes.
- No PageLayout migration, header/footer layout, scene numbering, dual dialogue, or exact Final Draft pagination claim.

## Detailed behavior contract

### Parsing and fallback

- Accept bounded decimal inch values with an explicit sign where allowed. Reject overflow and non-finite forms from the effective projection; preserve their source XML and use the default for that field.
- Require usable paragraph bounds after conversion: right must be greater than left, values must fit model/page budgets, and first-line geometry must remain calculable without overflow.
- Support Yes and No for StartsNewPage using the existing semantics.
- Treat Leading Regular and Spacing 1 as the currently mapped values. Preserve other values and use the shared default line behavior rather than guessing their meaning.
- Support Left, Right, and Center. Preserve unsupported alignment values and use Left as the deterministic application fallback.
- If ParagraphSpec Type is present and conflicts with its parent ElementSettings role, do not project that ambiguous specification; retain it unchanged in source.
- Preserve existing duplicate-role and structurally ambiguous input failures.

### Geometry and wrapping

- Keep paragraph bounds distinct from text-line origin/extent so right and center alignment do not corrupt hit testing or caret calculations.
- Compute wrapping from the available display width for the first versus subsequent lines. No canonical text transformation is permitted.
- For Left alignment, text begins at the applicable left edge. For Right alignment, it ends at the paragraph's right edge. For Center alignment, it is centered within the paragraph bounds.
- Empty elements retain a valid line and caret position. Right/center alignment of an empty line must produce a deterministic caret anchor.
- All geometry conversions to desktop floating-point coordinates and PDF numbers must use one explicit fixed-point conversion rule. Do not independently round in multiple consumers.

### SpaceBefore and pagination

- Apply SpaceBefore between adjacent body elements on the same page according to the incoming element's effective setting.
- Suppress SpaceBefore at the top of a new body page, including after an explicit StartsNewPage break. This preserves the current no-gap-at-page-top rule.
- Include SpaceBefore in same-page fit calculations and in the vertical distance between element origins.
- Use fixed-point line heights and content bounds to determine whether lines fit. A page break discards no source element and creates no unbounded blank pages.
- Preserve current keep-with-next roles and minimum-line grouping. If a group moves, apply the incoming SpaceBefore only when a predecessor remains on the destination page.
- StartsNewPage continues to force a new body page only when the current body page is nonempty.

### Cache and output parity

Expand cache keys through the existing complete source-settings value and runtime inputs. Two layouts with different effective paragraph settings must not share a stale result. Desktop and print use identical paragraph geometry; WO-003 will differentiate only visibility and presentation purpose.

## Suggested implementation steps

1. Add fixed-point types, checked conversion, validation, and formatting helpers to [formatting.rs](../../../crates/scriptcraft-core/src/formatting.rs).
2. Extend the source paragraph-settings projection in [fdx.rs](../../../crates/scriptcraft-core/src/fdx.rs:1789) without changing preserved XML.
3. Adapt [the core model](../../../crates/scriptcraft-core/src/lib.rs:377) and legacy JSON boundary so old integer runtime values convert explicitly to fixed point; do not persist private fixed-point fields into FDX.
4. Refactor [layout](../../../crates/scriptcraft-core/src/layout.rs:147) to resolve per-role paragraph formats and produce fixed-point paragraph/text geometry.
5. Update [desktop painting and navigation](../../../apps/scriptcraft/src/main.rs:707) to consume the shared geometry without independent rounding.
6. Update [PDF export](../../../crates/scriptcraft-core/src/exports.rs:74) for fixed-point coordinates while retaining its current noncanonical export boundary.
7. Add focused fixtures for every role, imported overrides/fallbacks, pagination, source mapping, and output parity.
8. Reconcile durable layout contracts with the approved behavior.

## Allowed implementation surface

Product paths:

- [formatting.rs](../../../crates/scriptcraft-core/src/formatting.rs)
- [lib.rs](../../../crates/scriptcraft-core/src/lib.rs)
- [layout.rs](../../../crates/scriptcraft-core/src/layout.rs)
- [fdx.rs](../../../crates/scriptcraft-core/src/fdx.rs)
- [preservation.rs](../../../crates/scriptcraft-core/src/preservation.rs) only for typed source-projection storage
- [exports.rs](../../../crates/scriptcraft-core/src/exports.rs) only for fixed-point coordinate consumption
- [main.rs](../../../apps/scriptcraft/src/main.rs) only for shared geometry consumption
- Focused core/app tests and synthetic fixtures

Durable records:

- [FDX fixes](../fdx-fixes.md)
- [Product overview](../../../.forge/requirements/product-overview.md)
- [Writing-loop requirements](../../../.forge/requirements/features/screenplay-writing-loop.md)
- [Technical context](../../../.forge/blueprints/technical-context.md)

Do not change font registration/assets, editing behavior, title-page interpretation, or native filesystem storage.

## Acceptance criteria

1. Every role has explicit effective paragraph geometry and SpaceBefore from the shared defaults; no added role uses the former neutral fallback.
2. Native fractional and negative values survive parsing and layout at 1/100-point precision without unsigned underflow or whole-point truncation.
3. A valid imported role ParagraphSpec overrides defaults per field; absent, unsupported, or ambiguous values retain defaults and remain unchanged in saved source XML.
4. FirstIndent affects only the first visual line and changes its available width correctly.
5. Left, Right, and Center alignment produce deterministic text geometry without modifying canonical text or source ranges.
6. SpaceBefore values 0, 12, 14, 16, and 24 are applied by incoming role and suppressed at body-page tops.
7. Page-fit calculations use actual vertical extents, preserve keep-with-next behavior, and terminate safely for oversized elements.
8. New Act starts a new page by default in new and reopened documents; imported StartsNewPage overrides behave equivalently before and after save.
9. Caret placement, hit testing, selection, vertical navigation, desktop painting, and PDF coordinates agree with the same fixed-point layout.
10. Layout caching differentiates changed effective paragraph settings, and identical inputs remain deterministic.

## Required test authoring and user validation handoff

Author structural fixtures for all roles and every supported alignment, fractional/negative indent, SpaceBefore value, page-top suppression, explicit page start, imported fallback, cache invalidation, oversized content, caret/source round trips, and desktop/PDF coordinate parity. Do not execute them under repository policy.

The user should run the applicable workspace checks and manually compare representative new/imported pages, paying particular attention to Character, Dialogue, Transition, New Act, Sequence, End of Act, and outline indents. Any Final Draft visual comparison is external, user-owned evidence and must use disposable files.

## Handoff

WO-003 builds role typography on this fixed-point layout and introduces editor versus print visibility. WO-004 inserts direct paragraph values ahead of role settings in the resolver established here. WO-005 is behavior-only and does not depend on this geometry implementation.

## Implementation handoff — 2026-11-01

- **State:** WO-002 implementation and focused regression-test authoring are present. This is not a review, validation, or acceptance result.
- **Fixed-point contract:** `formatting::FixedPoint` stores signed hundredths of a point. Decimal inches convert through 7,200 hundredths of a point per inch and round half away from zero to the stored 1/100-point quantum. FDX decimal parsing is bounded to six fractional digits; non-finite, exponent, overflow, negative left/right positions, and coordinates outside the current US Letter page budget do not enter the effective projection. `FixedPoint::decimal_string()` is the PDF numeric spelling; desktop coordinates use `to_points_f32()` from the same stored value.
- **Projection/fallback contract:** each supported ParagraphSpec field is projected independently. Missing/unsupported geometry, spacing, leading, and spacing tokens use role defaults; unsupported Alignment deterministically projects Left. A present conflicting ParagraphSpec Type excludes that complete spec. Right/left bounds that do not produce a usable in-page measure fall back together to the shared role bounds. Source XML remains the serialization authority. Explicit StartsNewPage Yes/No is recognized; unsupported tokens use the role default.
- **Layout contract:** paragraph bounds and aligned text extents are distinct fixed-point geometries. FirstIndent changes only the first visual line's measure and origin. Per-role incoming SpaceBefore is suppressed at body-page tops. Pagination tests both logical capacity and actual fixed vertical extents, while retaining bounded oversized-element placement and the existing keep-with-next role set.
- **Compatibility boundary:** the transitional legacy integer-point RoleBox values remain unchanged at the model/JSON boundary and are converted explicitly to fixed point for legacy layouts. No private fixed-point values are persisted to FDX. Desktop paint, pointer/caret mapping, selection, vertical navigation, PDF coordinates, and cache inputs consume the precise effective layout. Hidden roles intentionally retain their existing omission from shared editor layout until WO-003; typography and direct paragraph overrides remain deferred.
- **Authored coverage:** fixed-point conversion, imported per-field projection/fallback and source preservation, all-role default measures, alignment, signed first indent, spacing/page-start, cache invalidation, and fixed PDF coordinate formatting have focused test artifacts. They remain unrun.
- **Verification owner:** repository policy prohibits agent-run tests, builds, lint, formatting, review, manual verification, implementation checks, and validation. The user owns all verification. No acceptance, interoperability, or Final Draft layout claim is made.
- **WO-003 handoff:** the fixed-point layout interfaces are authored for sequential typography work, but this implementation is unvalidated. WO-003 may proceed as the next approved implementation order against these interfaces; it must not treat this handoff as evidence of WO-002 acceptance or correctness.

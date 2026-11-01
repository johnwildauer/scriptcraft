# WO-003 — Role typography and editor/print visibility

- **Status:** Approved; implementation authored; user-owned verification outstanding (not validated)
- **Initiative:** [FDX fixes](../fdx-fixes.md)
- **Depends on:** [WO-002](WO-002-fdx-paragraph-geometry-and-spacing.md)
- **Execution preference:** Complete before WO-004
- **Validation owner:** User; no agent-run checks or independent validators

## Outcome

Apply native role FontSpec defaults and validated imported values to desktop and PDF output using deterministic fixed-metric fallbacks. Implement the approved visibility contract: HiddenText roles remain visible and subdued in the writing surface but are omitted from print/PDF and other noncanonical exports.

## Current evidence

- [`ScriptCraftApp::paint_page()`](../../../apps/scriptcraft/src/main.rs:707) uses one global regular font and palette color for every role.
- [`font_definitions()`](../../../apps/scriptcraft/src/document_font.rs:9) already registers Courier Prime regular, bold, italic, and bold-italic faces.
- [`page_content()`](../../../crates/scriptcraft-core/src/exports.rs:154) uses one PDF font resource and global size.
- [`ScreenplayLayout::build_with_effective_inputs()`](../../../crates/scriptcraft-core/src/layout.rs:165) currently omits hidden roles from the same layout used by editor and exports.
- Both references declare role-specific family, size, style, color, and HiddenText values in their FontSpec blocks: [blank](../../../.reference/reference.fdx:61) and [populated](../../../.reference/reference2.fdx:172).
- The user explicitly approved visible/subdued hidden roles in the editor and hidden roles omitted from print/PDF.

## Scope

1. Project role-level FontSpec family, Size, Style, and Color as optional source values. Resolve each field through the WO-001 defaults when absent or unsupported.
2. Recognize the style tokens Bold, Italic, Underline, AllCaps, and HiddenText. Unknown tokens remain preserved and do not prevent known-token fallback behavior.
3. Carry effective text style and required per-line metrics through fixed-point layout, desktop painting, PDF export, and caching.
4. Introduce an explicit layout purpose for editor versus export/print. Include the purpose in cache identity and make every caller choose deliberately.
5. Editor layout includes HiddenText roles with a theme-aware subdued treatment. Print/PDF and plain-text export layouts omit them without changing source element indexes.
6. Map native font declarations to deterministic application fonts:
   - Courier Final Draft maps to bundled Courier Prime.
   - Arial and unknown families use the documented Courier Prime fixed-cell fallback while preserving the original family in FDX.
   - Bold, Italic, and Bold+Italic select the corresponding already bundled faces where available.
7. Scale deterministic fixed-cell metrics from the effective role size: 7 units of character width and 12 units of line advance at 12 points, scaled by size ÷ 12 and rounded through the WO-002 fixed-point rule. Do not use host font advances for pagination.
8. Apply Underline as presentation geometry, not canonical text.
9. Apply AllCaps as a display transform with explicit display-to-source mapping. Preserve canonical text and correct source offsets even when uppercase conversion expands one source character into multiple display characters.
10. Update wrapping, alignment, caret/hit testing, selection, vertical navigation, desktop painting, and PDF generation to use display text and its source map rather than assuming display length equals canonical length.
11. Extend PDF font resources and per-line text state for the supported family/size/weight/slant subset; draw underline separately. Omit HiddenText roles from PDF.

## Explicit non-goals

- No host font database, new font downloads, proportional-font measurement, or exact Arial/Courier Final Draft visual parity.
- No native source-color fidelity requirement in this order. Preserve Color and use the deterministic theme/fallback colors below.
- No direct Text-run style interpretation, mixed inline-style renderer, rich-format editing, or direct paragraph override work.
- No title-page typography, custom formatting UI, custom elements, or PDF font embedding redesign.
- No repair of unrelated pre-existing PDF non-ASCII substitution; do not claim that this order resolves it.
- No Behavior, PaginateAs, or Outline-level structural editing.

## Detailed behavior contract

### FontSpec parsing and fallback

- Treat FontSpec fields independently. Missing family, size, style, or color uses the corresponding shared default.
- Accept bounded positive sizes that fit the model's fixed-point and page budgets. Unsupported values remain preserved and use the default size.
- Parse Style as plus-separated tokens. Known tokens are idempotent; unknown tokens are retained in source but ignored by the effective application style.
- HiddenText controls visibility only; it is not a font face or slant.
- If duplicate or conflicting FontSpec children make a role's values ambiguous, preserve the source and use the role default rather than choosing one by position.
- Source Color remains source-owned. This order does not guess the semantics of the references' extended color encoding.

### Editor and export visibility

- Editor purpose includes every semantic body element, including HiddenText roles, in source order.
- Print purpose excludes HiddenText roles from PDF and fixed-print layout. Plain-text export also excludes them while retaining its existing loss report.
- Hidden roles in the editor use a theme-aware subdued color distinct from ordinary body text. Normal and selected text must continue to meet the existing contrast requirements in [AC-WRITING-006.11](../../../.forge/requirements/features/screenplay-writing-loop.md:316).
- A role made visible by imported settings uses ordinary effective role styling rather than the hidden-role subdued treatment.
- Visibility never renumbers semantic source element indexes. Navigation and selection must be able to reach hidden roles in the editor.

### Deterministic font and line metrics

- The declared FDX family remains unchanged in native output regardless of application font substitution.
- Use bundled Courier Prime faces for all currently supported runtime families and styles. A missing face falls back to regular Courier Prime without dropping the remaining style geometry.
- Per-role font size affects fixed character width, line height, wrapping, alignment, caret/selection extents, and page fit through the same fixed-point calculations.
- Do not allow a larger role to reuse the global 12-point line box or a smaller role to produce a zero-width/zero-height cell.
- Viewport zoom remains a desktop-only multiplier after fixed layout; it does not change native settings, pagination, or export geometry.

### AllCaps and source mapping

- Keep canonical text and run attributes unchanged on import and edit.
- Build display segments that record each source byte range and its transformed display text. An expansion such as one source character becoming multiple uppercase characters must remain associated with the original source range.
- Wrap and align using display cell counts, but map caret and selection operations only at valid canonical source boundaries.
- When multiple display cells belong to one source character, caret placement selects a deterministic source boundary; it must not create a caret inside the source character or use a display byte offset as a source offset.
- PDF uses display text for the supported AllCaps presentation. Plain-text export retains canonical text while applying HiddenText visibility.

### Desktop and PDF styling

- Desktop uses effective role size, face, underline, display text, and theme/fallback color. Underline does not change source ranges or line height.
- PDF declares deterministic resources for the supported regular/bold/italic/bold-italic fallback variants and applies effective size per line. The output remains noncanonical and does not claim embedded-font or Final Draft parity.
- Source colors are not emitted as authoritative PDF colors under an unverified conversion. Use the documented default print color.
- Unsupported styling remains preserved in FDX even when omitted from display/PDF.

## Suggested implementation steps

1. Extend [formatting.rs](../../../crates/scriptcraft-core/src/formatting.rs) with optional source FontSpec projection, effective style resolution, and deterministic fallback definitions.
2. Parse role FontSpec values in [fdx.rs](../../../crates/scriptcraft-core/src/fdx.rs:1789) without changing preserved nodes.
3. Add layout purpose, per-line effective style/metrics, and display-source segments in [layout.rs](../../../crates/scriptcraft-core/src/layout.rs:35).
4. Update [desktop font selection](../../../apps/scriptcraft/src/document_font.rs:9) and [painting/navigation](../../../apps/scriptcraft/src/main.rs:707) to consume effective style and display mappings.
5. Update [PDF and text exports](../../../crates/scriptcraft-core/src/exports.rs:46) for print visibility, per-line style, underline, and display/canonical text distinctions.
6. Expand cache keys for purpose and complete effective source font settings.
7. Add focused headless and rendered-egui regressions for typography, visibility, source mapping, and cache behavior.
8. Update durable requirements and blueprints with the approved editor/print and deterministic fallback contracts.

## Allowed implementation surface

Product paths:

- [formatting.rs](../../../crates/scriptcraft-core/src/formatting.rs)
- [lib.rs](../../../crates/scriptcraft-core/src/lib.rs) only for shared style/purpose types
- [layout.rs](../../../crates/scriptcraft-core/src/layout.rs)
- [fdx.rs](../../../crates/scriptcraft-core/src/fdx.rs)
- [preservation.rs](../../../crates/scriptcraft-core/src/preservation.rs) only for typed source projection
- [exports.rs](../../../crates/scriptcraft-core/src/exports.rs)
- [main.rs](../../../apps/scriptcraft/src/main.rs)
- [document_font.rs](../../../apps/scriptcraft/src/document_font.rs)
- Focused core/app tests and synthetic fixtures

Durable records:

- [FDX fixes](../fdx-fixes.md)
- [Product overview](../../../.forge/requirements/product-overview.md)
- [Writing-loop requirements](../../../.forge/requirements/features/screenplay-writing-loop.md)
- [Technical context](../../../.forge/blueprints/technical-context.md)

Do not modify supplied font files/licenses, editing transitions, title-page preservation, native filesystem storage, or add dependencies without a separate approved amendment.

## Acceptance criteria

1. Every role uses its shared default font/style when no imported override exists, and valid imported FontSpec values override defaults per field after save/reopen.
2. HiddenText roles are visible, editable, and subdued in the editor but absent from PDF, print layout, and plain-text export; source indexes remain stable.
3. Visible imported Note/Summary/outline roles use ordinary rather than hidden-role presentation.
4. Courier Final Draft and Arial declarations remain unchanged in FDX while desktop/PDF use the documented deterministic fallbacks.
5. Bold, Italic, Bold+Italic, Underline, role size, and AllCaps affect only their intended presentation paths.
6. A 16-point Outline 1 and 14-point Outline 2 produce coherent fixed-point wrapping, line extents, page fit, carets, and PDF text state without zero metrics or clipping caused by a shared 12-point box.
7. Mixed-case imported text remains byte-for-byte canonical while AllCaps display uses the native role style.
8. Unicode uppercase expansion cannot corrupt source ranges, caret positions, selection, wrapping, or vertical navigation.
9. Unknown font/style/color values remain preserved and produce deterministic fallback output rather than import failure or source rewriting.
10. Editor and print layouts cannot share a stale cache entry across different purposes or effective font settings.

## Required test authoring and user validation handoff

Author focused tests for every native style token and role size, visible/hidden source overrides, editor versus print visibility, source-index stability, all four bundled faces, underline geometry, mixed-case and expanding-Unicode AllCaps, wrapping/alignment/caret/selection mapping, cache separation, PDF resources, and canonical plain-text output. Do not run them under repository policy.

The user should run the applicable workspace checks and visually inspect a document containing all 17 roles in Day and Night modes, including long Outline 1/2 text, mixed-case AllCaps roles, and hidden roles. Inspect PDF separately for omission of hidden roles and supported style output. Any Final Draft comparison remains user-owned external evidence on disposable files.

## Handoff

WO-004 adds direct paragraph geometry above the role paragraph settings while leaving this order's role typography resolver intact. WO-005 may use shared Behavior defaults independently and does not alter editor/print visibility or typography.

## Implementation handoff — 2026-11-01

- **State:** WO-003 product changes and focused test authoring are present. No acceptance, review, or validation result is claimed. The repository policy prohibits agent-run checks and validator dispatch; the user owns all verification.
- **Source projection:** `formatting::SourceFontSpec` stores optional family, size, known style flags, and source color per role. `read_source_layout_settings()` projects one unambiguous role `FontSpec`; duplicate FontSpec children leave the complete font projection at role defaults. Style recognizes Bold, Italic, Underline, AllCaps, and HiddenText; unknown tokens and unsupported source values remain owned by preserved FDX. Sizes are accepted as integers 1–72 points. `SourceLayoutSettings::text_style(role)` combines source fields independently with shared defaults. Original family/color values are retained in the effective projection but are not treated as verified rendering/color semantics.
- **Typography/layout API:** `layout::LayoutPurpose::{Editor, Print}` is explicit at the desktop cache caller and export callers. `LayoutCache::get_or_build_with_inputs_for_purpose()` includes purpose in its cache identity alongside inputs and the complete source-settings projection. `ScreenplayLayout::{build_with_inputs_for_purpose, build_with_effective_inputs_for_purpose}` support purpose-aware construction. Each `LayoutLine` retains canonical `text` and source byte range separately from `display_text`, `display_source_map`, `text_style`, and fixed-point `char_width`; mapping methods translate only between valid canonical boundaries and display-character boundaries. The fixed-cell 7/12 metrics scale by role size from their 12-point base, with no host font advance in wrapping or pagination.
- **Visibility/export API:** Editor-purpose layout includes hidden roles at their original `element` indexes, with `text_style.hidden_text` available to the desktop subdued palette. Print-purpose layout omits those roles; plain text emits the canonical `line.text` from this layout and PDF emits display-transformed role text. Visible overrides use normal role color. PDF uses standard non-embedded Courier regular/bold/oblique/bold-oblique resources, per-line size and horizontal fixed-cell scaling, and separate underline strokes.
- **Authored tests:** Added source projection/fallback/duplicate-FontSpec cases; role-size/style and Unicode expanding-AllCaps/source-boundary, cache-purpose, and print/editor visibility coverage; rendered egui font-family/size/hidden palette coverage; contrast assertions for subdued text in Day/Night/Midnight; and PDF style-resource/underline plus canonical plain-text/hidden export assertions. None have been run.
- **Durable records:** Product overview, writing-loop requirements, and technical context now state the bounded typography, canonical/display separation, and editor/print visibility contracts. The existing requirement that PDF preserve non-ASCII remains unchanged; this work does not fix the current substitution behavior.
- **Limitations:** No exact Final Draft/Arial font parity, source-color fidelity, font embedding, non-ASCII PDF improvement, full FDX interoperability, or successful visual/user validation is claimed. Theme fallback colors and PDF's standard Courier resources are deterministic application output, not evidence of external-app parity.
- **WO-004 handoff:** WO-004 may safely proceed against the new `LayoutPurpose`, `SourceLayoutSettings::text_style()`, `LayoutLine` display/source mapping, and fixed-point geometry interfaces. Direct paragraph values must take precedence only for paragraph geometry and must not replace the role typography resolver. WO-001 and WO-002 remain user-unvalidated; this handoff does not establish their acceptance or this order's acceptance.

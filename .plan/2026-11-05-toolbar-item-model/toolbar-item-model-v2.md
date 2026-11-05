# Toolbar Item Model v2 — Variable icon weight research

- **Status:** Research draft; no font integration authorized or implemented
- **Date:** 2026-11-05
- **Parent plan:** [`toolbar-item-model-v1.md`](toolbar-item-model-v1.md)
- **Research method:** Browser inspection of the Google Fonts CSS endpoint and the published `egui_material_icons` 0.6.0 documentation/source
- **Validation owner:** User; no project build, check, test, lint, formatting command, or manual app verification was run

## Outcome

Determine whether ScriptCraft can honor the existing weight-200 Material icon preference without replacing its current icon catalog or substantially rewriting toolbar/status rendering.

## Findings

### Material Symbols Rounded has a variable weight axis

The Google Fonts stylesheet for [Material Symbols Rounded](https://fonts.googleapis.com/css2?family=Material+Symbols+Rounded:opsz,wght,FILL,GRAD@20..48,100..700,0..1,-50..200&display=block) advertises `font-weight: 100 700`. Its variable axes include optical size (`opsz`), weight (`wght`), fill (`FILL`), and grade (`GRAD`). The requested weight 200 is within the published range. This matches the Rounded family used by the current crate more closely than switching to Material Symbols Outlined.

The Google Fonts endpoint returned a WOFF2 resource. This is evidence that the family/weight range exists, not evidence that the exact response format can be loaded by the app's current egui font path. A bundled TTF/OTF source or an offline-generated static font may be preferable; accepted format and licensing/redistribution details must be confirmed before an implementation decision.

### Current crate integration boundary

The published [`egui_material_icons` 0.6.0 source](https://docs.rs/egui_material_icons/0.6.0/src/egui_material_icons/lib.rs.html) embeds `MaterialSymbolsRounded-Regular.ttf`. Its public `MaterialIcon` contains the icon codepoint and style. The crate's `rich_text()` selects one of two named font families: `material-icons` for filled and `material-icons-outlined` for outlined. Its initialization function registers the embedded Regular font(s) under these families.

ScriptCraft already preserves icon identity as a typed Material icon in [`toolbar_items.rs`](../../apps/scriptcraft/src/toolbar_items.rs) and [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs). The toolbar and status controls render `RichText` returned from the icon values; the project does not currently own a separate icon-to-glyph mapping table.

## Candidate integration approaches

### Preferred low-renderer-change prototype: same-family static font replacement

1. Obtain an appropriate Material Symbols Rounded variable-font source in a format supported by the app.
2. Produce two static instances at a selected weight such as 200: one with `FILL=0` for outlined and one with `FILL=1` for filled. Keep a documented `opsz` and `GRAD` choice.
3. Replace the crate's embedded-Regular font initialization with app-owned registration under the exact family names used by the crate: `material-icons` and `material-icons-outlined`.
4. Keep the existing crate `MaterialIcon` constants, their codepoints, `.outlined()`/`.filled()` selection, and current toolbar/status `rich_text()` rendering paths.

This approach could avoid rewriting item definitions or control renderers, because the crate's `RichText` family names would resolve to the app-registered fonts. However, whether egui's font insertion/priority behavior permits a clean replacement under those names must be proven in a small prototype. Do not initialize both the crate's Regular fonts and custom replacements without confirming which font wins.

### Explicit custom family adapter

Register custom named families for each supported style/weight, then construct icon text from the existing `MaterialIcon` codepoint while choosing the custom family. This gives ScriptCraft direct control over weight/style routing and avoids relying on duplicate family-name precedence, but requires changing toolbar and status rendering paths and deciding how per-icon weight overrides map to available font instances.

### Runtime variable-axis selection

The Google Fonts file is variable, but the examined `egui_material_icons` API exposes style selection (filled/outlined), not weight-axis selection. Runtime `wght` selection through egui's font backend has not been established. Do not assume a variable font loaded into egui automatically exposes CSS-style axis controls. If runtime arbitrary weights are essential, first investigate the epaint/font parser and shaping path, including `wght`, `FILL`, `opsz`, and `GRAD` handling.

## Recommendation

Start with a narrowly scoped font-registration prototype rather than a renderer rewrite. The prototype should attempt one outlined and one filled static Rounded font instance at weight 200, registered under the crate's current family names. It should render the existing icon constants in both toolbar and status presentation at their existing sizes and compare glyph identity, weight, baseline, clipping, and visual consistency with the embedded Regular fonts.

If family replacement works and the font/codepoint mapping is compatible, keep the current icon crate for typed constants and style selection while taking font asset ownership in the desktop app. If the registration API does not safely replace the families, use the explicit custom-family adapter. Treat a runtime arbitrary weight slider as a separate scope decision, not a prerequisite for selecting one application-wide weight.

## Open questions before implementation

- Does the selected upstream Rounded variable-font source provide the same glyph codepoints and desired glyph designs as the crate's embedded Regular font for every currently used icon?
- Which font file format can be loaded through the locked egui 0.34 stack without additional runtime conversion dependencies?
- Can custom `FontInsert` entries safely replace the crate's named font-family mappings, or must ScriptCraft stop calling the crate's initializer and register its own families?
- Is a single compile-time/static weight choice sufficient, or does product intent require users to change weight at runtime? If runtime changes are desired, which discrete weights are needed, or is full variable-axis support a requirement?
- What fixed `opsz` and `GRAD` values best preserve the current 18-point icon appearance? The CSS axis ranges do not prove that egui applies automatic optical sizing.
- Confirm font license and redistribution requirements for the selected downloadable source and any generated static instances.

## Scope and non-claims

- This is research/planning only. It does not change Cargo dependencies, embedded assets, font initialization, icon codepoints, toolbar/status rendering, requirements, or the v1 plan's implementation status.
- No production font was downloaded or added to the repository.
- No egui font-registration prototype, build, check, test, lint, formatting command, visual comparison, or runtime verification was performed.
- The proposed same-family registration is a low-change hypothesis, not a demonstrated drop-in replacement.
- Weight 200 is available in the Google Fonts variable range; that does not establish that egui can adjust the axis at runtime.

## Research references

- [Google Fonts CSS for Material Symbols Rounded](https://fonts.googleapis.com/css2?family=Material+Symbols+Rounded:opsz,wght,FILL,GRAD@20..48,100..700,0..1,-50..200&display=block) — returned `font-weight: 100 700` and a variable WOFF2 source.
- [`egui_material_icons` 0.6.0 source](https://docs.rs/egui_material_icons/0.6.0/src/egui_material_icons/lib.rs.html) — embedded Rounded Regular font data, public glyph/style representation, named filled/outlined families, `rich_text()` mapping, and font initialization.
- [`icon_assignement.md`](../../.reference/icon_assignement.md) — existing weight-200 design preference and documented crate Regular-weight exception.

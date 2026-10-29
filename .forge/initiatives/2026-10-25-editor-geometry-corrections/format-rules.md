# Screenplay Formatting Rules

Standard screenplay formatting rules require specific font, margin, and line spacing settings to ensure that one page equals roughly one minute of screen time.

## Font and Text Size

• Font Family: Courier Prime (12-point) is the default for new projects. Courier and Courier New remain accepted for explicitly saved legacy profiles. Courier Prime is a screenplay-oriented monospaced face; ScriptCraft's fixed character-cell geometry, not platform font advances, remains authoritative for layout.
• Font Size: 12-point.

## Page Margins (US Letter)

• Left Margin: 1.5 inches (extra space for 3-hole punch binding).
• Right Margin: 1.0 inch.
• Top Margin: 1.0 inch (with page numbers placed flush right at 0.5 inches from the top edge).
• Bottom Margin: 1.0 inch.

## Element Indents (from left edge of the page)

• Scene Heading (Slugline): 1.5 inches (left-aligned to the main margin)
• Action / Description Lines: 1.5 inches (spans across to the 7.5-inch mark / 1" right margin)
• Character Name (Cues): ~3.5 to 3.7 inches (all caps)
• Parentheticals: ~3.0 inches (indented narrower than the name, wrapping around 3.0" to 5.5")
• Dialogue: ~2.5 inches from the left edge, ending at ~5.5 inches from the left edge (narrower block)
• Transitions (e.g., CUT TO:): ~6.0 inches (flush right or heavily indented)

## Leading and Line Spacing

• Line Spacing: Single-spaced within individual elements (action paragraphs and dialogue blocks).
• Blank Lines (Double Spacing): Use a blank single line break (double space equivalent) to separate different screenplay elements—such as dropping down a line between an action block and a new scene heading, or between a character name and their dialogue.
• Lines per Page: Roughly 55 lines of text per standard page.

## Body Role-Pair Boundary Spacing

Boundary spacing is derived from the incoming body role, not inserted into
canonical text. For every predecessor role (`Scene Heading`, `Action`,
`Character`, `Dialogue`, `Parenthetical`, `Transition`, or `Note`), an
incoming `Dialogue` or `Parenthetical` uses zero extra boundary gap. Every
other body transition uses the profile's `boundary_gap`, including same-role
transitions. Wrapped lines within one element use `line_advance`; title-page
elements never use body boundary gaps. Page-capacity accounting consumes one
logical boundary unit only when the selected extra gap is nonzero.

## Durable Reference and Future Project Overrides

This document is the human-readable reference for ScriptCraft's default
screenplay format. Layout code converts inch-defined values to typographic
points using `72 × inches`; screen pixels and viewport zoom are never format
inputs. New profiles persist the exact family name `Courier Prime` and the
12-point base metric. The desktop bundles and renders the supplied regular
Courier Prime face; it uses the established monospace renderer for projects
whose explicitly saved family is `Courier` or `Courier New`. The bundled font
files and their SIL Open Font License notices remain unmodified at
`fonts/CourierPrime/`. PDF keeps its existing deterministic Type1 family-name
mapping; this does not claim that the PDF embeds the TrueType font.

The native project/profile model should preserve a named format identity and a
bounded, versioned override seam for future project-specific changes. Future
overrides should replace explicit format fields (margins, role boxes, leading,
and boundary gaps) rather than storing page pixels or duplicating formatting on
each screenplay element. Building an override editor is outside the current
work order; the implementation must only avoid making this seam impossible.

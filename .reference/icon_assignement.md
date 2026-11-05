# App Icons to Material Icons Assignment 

This app will use Material icons from `egui_material_icons` 0.6.0 with its default `filled` and `compressed` features plus `outline`. We will default to "outline" icons, using "filled" for only a handful (see below). Our default font weight preference remains 200.

**WO-001 implementation exception (user-approved 2026-11-04):** The crate's
embedded filled and outlined fonts render at Regular weight and expose no weight
selection API. Regular is accepted for this implementation only; it does not
replace the weight-200 design preference above. Material font initialization
follows Courier Prime registration. This foundation does not change the WIP
assignments below or claim that status-bar icon substitutions are implemented.

## Toolbar Temporary 

| App Icon | Material Icon |
| --- | --- |
| (Flexible Space) |  |
| New | Draft |
| Open | File Open |
| Save | Save |
| Save As | Save As |
| (Flexible Space) |  |
| Elements | (Dropdown) |
| (Flexible Space) |  |
| Show/Hide | Visibility + Keyboard Arrow Down |
| Export Text | File Export |
| Export PDF | Picture As Pdf |
| (Flexible Space) |  |
| (Overflow Indicator) | Double Arrows |

**WO-002 implementation handoff:** The temporary composition above is now
represented by the toolbar catalog with the four distinct Flexible Space entries
and a conditional icon-only outlined Double Arrows overflow control. Show/Hide
uses independently specified outlined Visibility and Keyboard Arrow Down glyphs;
Export Text uses the crate's outlined `ICON_DRIVE_EXPORT` glyph for the File
Export assignment. Show/Hide and Export Text are intentionally disabled and
remain inert in both the toolbar and overflow. This records implementation only;
user review and validation remain pending. The future Toolbar (WIP) assignments
below are unchanged.

**Toolbar presentation defaults (2026-11-05 follow-up):** Toolbar Material icons
resolve to the outlined font variant unless an individual icon explicitly
overrides it. Toolbar item text uses the named 14-point logical-size default;
these toolbar-local settings do not change the status bar or screenplay text.


## Toolbar (WIP)

| App Icon | Material Icon |
| --------- | --------- |
| Elements | (Dropdown) |
| Views | Web Asset + Keyboard Arrow Down |
| Normal View | |
| Page View | |
| Speed View |  |
| Beat Board | Dashboard |
| Scene View |  |
| Index Cards - Script |  |
| Index Cards - Summary |  |
| Split |  |
| Split Vertically |  |
| Split Horizontallly |  |
| Unsplit Panels |  |
| Swap Panels |  |
| Collaboration |  |
| Title Page |  |
| Insert Image |  |
| Show/Hide |  |
| Navigator | Table |
| Feedback |  |
| Save | Download |
| Print |  |
| Cut |  |
| Copy |  |
| Paste |  |
| Undo |  |
| Redo |  |
| Find |  |
| Find Next |  |
| Spelling |  |
| Thesaurus |  |
| Highlighter |  |
| New Scene |  |
| Reformat |  |
| Send to Script |  |
| Add Alt |  |
| Dual Dialogue |  |
| Revert Paragraph |  |
| Invisibles |  |
| Tags Mode |  |
| Styles |  |
| Alignment |  |
| Bookmarks |  |
| Bookmark |  |
| tOGGLE cASE |  |
| Typewriter |  |
| Revision Mode |  |
| Revisions |  |
| New Beat |  |
| Track Changes |  |
| Writing Stats |  |
| Emoji |  |
| Structure Lines |  |
| Space |  |
| Flexible Space |  |
|  |  |

## Status Bar

| Status Bar | Material Icon |
| --- | --- |
| Sprint Timer | "Timer" |
| Focus Mode | "Location Searching" |
| Appearence | "Mode Night (filled)" |

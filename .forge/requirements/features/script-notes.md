# Requirements Area: ScriptNotes

> This document specifies desired user-facing behavior in established-state voice. Stable identifiers make each requirement and acceptance criterion independently traceable; this is not implementation status, evidence, progress, or a backlog.

## Area purpose

ScriptNotes are screenplay-editor annotations attached to ranges or points in the screenplay Body. They support a writer's own drafting and revision workflow without becoming screenplay text, changing pagination, or entering screenplay output.

## Creation and anchoring

- **REQ-SCRIPT-NOTE-001: Body-anchored note creation.** The system shall create ScriptNotes from the current Body selection or caret without taking ownership from higher-priority editor surfaces.
  - **AC-SCRIPT-NOTE-001.1:** Insert → ScriptNote, the Body secondary-click context action Insert ScriptNote, and the primary+Shift+K shortcut create from the current ordered Body selection. A secondary click inside an existing selection preserves that selection; otherwise its hit position becomes the caret before creation.
  - **AC-SCRIPT-NOTE-001.2:** The selection is stored as a half-open UTF-16 range in flattened Body order. A collapsed selection creates a zero-length point note. The Body's semantic paragraph boundaries contribute one virtual separator each; positions remain independent of UTF-8 storage offsets.
  - **AC-SCRIPT-NOTE-001.3:** Creation is unavailable on the Title Page, Beat Board, or while another modal, replacement, migration, native-output, or higher-priority input owner controls editing.
  - **AC-SCRIPT-NOTE-001.4:** A created note has generated identities, empty title, Body, WriterID, and WriterName, DateTime and DateModified initialized from the same supplied local wall-clock value, and the transient current-session Color/Type defaults. The note opens with Body focus; it does not replace the source Body selection.
  - **AC-SCRIPT-NOTE-001.5:** Editing or deleting anchored screenplay text never implicitly deletes a note. Only the note card's Trash action deletes a note.

## Range maintenance

- **REQ-SCRIPT-NOTE-002: Stable Body anchoring.** The system shall maintain each note's UTF-16 half-open Body range through semantic Body edits while retaining the note itself.
  - **AC-SCRIPT-NOTE-002.1:** Flattened offsets count UTF-16 code units in canonical paragraph text, including stored CR, LF, or CRLF units, and one virtual separator between adjacent Body paragraphs. Supplementary Unicode characters occupy two units; a boundary inside a surrogate pair is invalid.
  - **AC-SCRIPT-NOTE-002.2:** Insertion at or before a range start shifts both endpoints; insertion strictly inside shifts only the end; insertion at the end or later leaves the range unchanged. At a point note, insertion at the point moves it and a later insertion does not.
  - **AC-SCRIPT-NOTE-002.3:** Deletion wholly before a range leaves it unchanged; deletion wholly after shifts it left. Overlap clips the affected edge. Full coverage collapses the range to the post-deletion caret boundary. A point at the deletion start remains there; a point at its end or strictly within the deleted interval collapses to the start.
  - **AC-SCRIPT-NOTE-002.4:** Replacement applies deletion followed by insertion at the deletion start. A fully replaced range remains as a point and follows insertion-at-start affinity; it is not deleted.
  - **AC-SCRIPT-NOTE-002.5:** Paragraph split inserts one virtual separator; join removes one. Paragraph removal removes its text and exactly one appropriate adjacent separator. Paste replacement, structural edits, owner removal, and derived Body corrections use the same flattened replacement rules.
  - **AC-SCRIPT-NOTE-002.6:** Automatic range shifts, save, and save rebase do not change DateTime or DateModified. Undo and Redo restore the note-containing semantic snapshot.

## Markers, highlight, and navigation

- **REQ-SCRIPT-NOTE-003: Margin markers and range presentation.** The system shall present ScriptNotes in the Body editor without changing screenplay layout or ordinary selection.
  - **AC-SCRIPT-NOTE-003.1:** A note is assigned to the first canonical Body layout baseline of the Body paragraph containing its range start, even when the exact start is on a later wrapped line or page. One marker groups notes that share that paragraph-first baseline.
  - **AC-SCRIPT-NOTE-003.2:** The filled Chat Bubble marker's right edge is 18 canonical points from the paper edge and its glyph baseline aligns to the canonical line baseline. The marker is 12 points normally and 13 points on hover; page zoom scales these measurements.
  - **AC-SCRIPT-NOTE-003.3:** A grouped marker shows its count centered in 7-point Courier using the page-paper color. Its marker color is the first note's color resolved through the shared three-state projection: theme-default uses active page text color, explicit palette colors remain fixed, and unsupported raw colors present as Black. A single note has no count. Supported note colors are Black and the existing twelve-color palette.
  - **AC-SCRIPT-NOTE-003.4:** The open note's half-open range receives an independent translucent highlight beneath glyphs and ordinary selection. Its color uses the same three-state projection as markers and closed-card swatches. Virtual paragraph separators and point ranges paint no text highlight. Highlighting does not replace or modify Body selection.
  - **AC-SCRIPT-NOTE-003.5:** Previous and Next follow deterministic global order by range start, range end, then source/creation order. Navigation does not wrap. It selects the target range start through ordinary Body caret/reveal behavior; same-baseline navigation does not force scrolling, while a different line or page follows ordinary reveal behavior. The open card is not dismissed solely because the destination marker baseline is temporarily offscreen during that reveal; this exception ends when the open note's marker becomes visible, and all other dismissal rules remain in force.

## Native card and text

- **REQ-SCRIPT-NOTE-004: Fixed note card.** The system shall present one fixed native card for the open note and keep its editing separate from screenplay text input.
  - **AC-SCRIPT-NOTE-004.1:** The card is 296 × 260 logical points, nonresizable, and independent of screenplay zoom. Placement prefers the marker-right origin, capped at the root window's right edge, whenever the complete card fits within the valid monitor/work right edge; the card may extend beyond the root's right edge when monitor space allows. If the right candidate does not fit, placement uses the left-of-marker candidate when it fits, otherwise the more-room fallback. Monitor/work bounds govern full-card horizontal fitting and vertical clamping; valid root bounds cap the card's left origin but do not require full containment within the root. With no valid work bounds, valid root bounds provide the horizontal screen constraint; with neither, horizontal placement remains unbounded. New Type and property-menu placement retain their separate policies.
  - **AC-SCRIPT-NOTE-004.2:** The top bar presents WriterName and DateModified as MM/DD/YYYY HH:MM AM/PM. Invalid timestamps remain source data and have no formatted display fallback. Empty title/body and writer fields use subdued presentation-only placeholders.
  - **AC-SCRIPT-NOTE-004.3:** Title is plain, single-line text. Body is rich text with ordered paragraphs and runs, canonical text editing, clipboard and IME support, independent selection/caret, and a vertical scrollbar when Body content exceeds its fixed viewport. Placeholder text never enters document data.
  - **AC-SCRIPT-NOTE-004.4:** The Trash icon is the sole note deletion path and uses full red RGB, 255, 0, 0, on hover or active state. Backspace, Delete, screenplay editing, and range collapse do not delete the note.
  - **AC-SCRIPT-NOTE-004.5:** The card follows its marker as the page scrolls or zooms while the anchor baseline remains visible. It closes on outside click, application focus loss, a hidden anchor baseline, deletion or missing note, document activation, or higher-priority Title, Beat, modal, replacement, migration, or native-output ownership. During a successful Previous/Next caret reveal, temporary destination-baseline invisibility alone does not dismiss the open card; that exception ends when its marker becomes visible. Root, note, Color/Type menus, and New Type dialog form one application focus family.
  - **AC-SCRIPT-NOTE-004.6:** Title and Body edits, selection, caret, multiline paste, scrollbar, clipboard, IME, and shared Undo/Redo operate on canonical note content. A focused note owns its input; screenplay shortcuts and IME routing do not consume that input.

## Color, Type, and transient defaults

- **REQ-SCRIPT-NOTE-005: Note properties.** The system shall provide exact, bounded Color and Type properties and transient creation defaults.
  - **AC-SCRIPT-NOTE-005.1:** Color choices are None first, then Black followed by Pink, Red, Orange, Yellow, Olive, Green, Mint, Blue, Violet, Purple, Brown, and Gray. None uses the shared no-color circle, is checked exactly for theme-default, and writes the default sentinel **#RRRRGGGGBBBB**; absent/empty Color and that exact raw sentinel are theme-default and resolve to the active page text color. New notes created with default None color carry the sentinel in their ScriptNote Color attribute. Existing absent/empty values are not migrated merely by opening a note or changing only its Type. Explicit palette values, including Black, remain distinct and fixed. Unsupported nonempty raw Color values other than that exact sentinel are preserved unchanged, display as Black, leave None and every explicit palette row unchecked, and do not become a new-note default. The closed-card swatch, marker, and range highlight share this three-state projection. Appearance changes affect theme-default presentation without rewriting note data.
  - **AC-SCRIPT-NOTE-005.2:** The Type menu contains None, each nonempty current Type in first-use order, a separator, and New Type. It deduplicates exact strings only and checks exactly the current Type or None. No separate Type registry is persisted.
  - **AC-SCRIPT-NOTE-005.3:** New Type is a separate fixed 320 × 170 logical-point dialog with Enter Type, Cancel, and OK. Escape and Cancel close without semantic mutation; Enter submits. A successful selection returns focus to Body.
  - **AC-SCRIPT-NOTE-005.4:** Exactly empty input and exactly capitalized None clear Type. All other values retain exact characters, case, and surrounding whitespace. An exact existing value selects that value; case- or whitespace-distinct strings remain distinct.
  - **AC-SCRIPT-NOTE-005.5:** A real property change updates DateModified once. A same-value selection does not create history or update DateModified, but still updates the transient creation default to the resulting complete Color/Type combination. Initial and document-activation-reset defaults are theme-default/None; theme-default resolves to the active page text color. Defaults belong only to the current document session, are not stored in settings, FDX, or history, and are not rolled back by Undo/Redo.
  - **AC-SCRIPT-NOTE-005.6:** Changing Color or Type does not alter WriterID, WriterName, DateTime, title, Body, or range.
  - **AC-SCRIPT-NOTE-005.7:** Create and property command payloads carry optional Color. For property updates, missing Color means Preserve, the ThemeDefault command-wire value means write the default sentinel, and existing palette values mean Explicit. Type-only changes preserve the note's existing raw Color exactly, including empty theme-default and unsupported nonempty source values. ThemeDefault is serialized as `none` and exists only as a ScriptNote command-wire value; it is not an InlineColor palette entry or a screenplay formatting-menu choice. ScriptNote command IDs 76–83 and command-envelope version 2 remain unchanged.

## Persistence and output boundaries

- **REQ-SCRIPT-NOTE-006: Bounded native persistence.** The system shall persist supported ScriptNotes through the native FDX lifecycle while leaving unsupported source opaque.
  - **AC-SCRIPT-NOTE-006.1:** The editable projection is limited to supported direct ScriptNote children of the first direct root-namespace ScriptNotes container. Supported ranges must be unique, ordered, in-bounds flattened UTF-16 boundaries. Supported direct Paragraph/Text content, rich runs, direct settings, properties, and durable identities round-trip through detached native output.
  - **AC-SCRIPT-NOTE-006.2:** Unsupported or malformed ranges, unsupported ancestry, foreign-namespace lookalikes, later containers, non-note children, and legacy inline ScriptNote structures remain opaque and are not clamped or rewritten as managed notes. This bounded support makes no universal compatibility guarantee.
  - **AC-SCRIPT-NOTE-006.3:** Save and Save As verify note semantics, identity/source projection, and selection rebase before publication. Failure preserves the prior valid destination and live session under the existing native-save safeguards.
  - **AC-SCRIPT-NOTE-006.4:** ScriptNotes are excluded from screenplay pagination, PDF, native print, plain-text output, and Fountain output.
  - **AC-SCRIPT-NOTE-006.5:** Absent or empty FDX Color and the exact **#RRRRGGGGBBBB** sentinel mean theme-default; an explicit FDX Black remains explicit Black. New default None-color notes emit that sentinel, while existing absent/empty values are not migrated merely by Open or Type-only edits. This bounded source contract does not promise interoperability with other applications.

## Related documents

- **Product overview:** [`../product-overview.md`](../product-overview.md)
- **Technical composition:** [`../../blueprints/features/script-notes.md`](../../blueprints/features/script-notes.md)
- **Native lifecycle boundary:** [`native-document-lifecycle.md`](native-document-lifecycle.md)

# Component Blueprint: Screenplay Core

## Capability

Screenplay Core provides the reusable, headless semantic screenplay model and document operations consumed by ScriptCraft Desktop. It owns canonical project data, transactional editing sessions, deterministic fixed-profile layout, versioned native project serialization, and noncanonical export adapters. Its contracts are independent of window widgets; the desktop owns presentation, input, file selection, and platform interaction.

The capability supports the [screenplay-writing-loop requirements](../../requirements/features/screenplay-writing-loop.md) and composes into the [ScriptCraft Desktop container](../containers/scriptcraft-desktop.md) and [Screenplay Writing Loop feature](../features/screenplay-writing-loop.md).

## Composition

### ProjectModel

```component
name: ProjectModel
container: Reusable Screenplay Core library
responsibilities:
  - Own versioned project metadata, title-page elements, ordered screenplay elements, stable identifiers, roles, and fixed PageProfile inputs.
  - Preserve canonical UTF-8 text and validate bounded project structure and serialization.
```

### EditingSession

```component
name: EditingSession
container: Reusable Screenplay Core library
responsibilities:
  - Own the semantic project, checked positions and selections, role-transition policies, dirty state, revision, and bounded undo/redo.
  - Apply edits transactionally and preserve session invariants on rejected, failed, or no-op operations.
```

### ScreenplayLayout

```component
name: ScreenplayLayout
container: Reusable Screenplay Core library
responsibilities:
  - Derive immutable, deterministic page geometry and source mappings from a validated project and complete fixed PageProfile.
  - Provide role-aware wrapping, pagination, caret geometry, and hit testing independent of viewport presentation scale.
```

### NativePersistence

```component
name: NativePersistence
container: Reusable Screenplay Core library
responsibilities:
  - Serialize and parse the bounded, versioned canonical native project representation.
  - Prepare native saves for atomic destination replacement while preserving failure and dirty-state guarantees.
```

### ExportAdapters

```component
name: ExportAdapters
container: Reusable Screenplay Core library
responsibilities:
  - Produce noncanonical PDF output from fixed screenplay layout while preserving non-ASCII characters.
  - Produce deliberately lossy plain text with an explicit account of semantic and layout information not retained.
```

`ProjectModel` is the canonical semantic source. `EditingSession` applies edits to it and owns selection and editing history. `ScreenplayLayout` reads the project and complete profile to produce immutable derived geometry; it does not mutate canonical text. `NativePersistence` reads and writes semantic project content, not transient caret, layout, undo, or generated-parenthetical provenance. Export adapters derive review outputs without replacing or changing the native source. Desktop input delegates semantic operations to the session, and desktop presentation, hit testing, caret geometry, visual-line navigation, and PDF generation consume the appropriate shared core layout contracts. The core has no widget dependency; window chrome, including the OS-owned macOS header, remains at the desktop boundary.

## Contracts

### Semantic model and transactional editing

- A project contains versioned metadata, title-page elements, an ordered list of screenplay elements, and a fixed page profile. Each element has one of seven explicit semantic roles: Scene Heading, Action, Character, Dialogue, Parenthetical, Transition, or Note. Role is not inferred from appearance. Canonical text is ordered UTF-8, and each element has a unique opaque stable identifier independent of list position and page geometry.
- A new project contains exactly one empty Scene Heading. A new editing session starts with a valid caret at element index 0, byte offset 0. Text positions and selections use checked UTF-8 boundaries.
- Project structure and serialization are bounded to at most 10,000 elements, 262,144 UTF-8 text bytes per element, and 1,048,576 native JSON bytes. Malformed or out-of-bound data yields a typed, recoverable failure.
- Mutations and bounded paste are transactional. A bounded paste is one undo entry; embedded newlines, carriage returns, whitespace, and Unicode remain canonical text in the active element rather than creating roles or elements. Rejected selections, invalid boundaries, capacity failures, and other failed mutations preserve project, selection, history, revision, and dirty state. Successful changes support bounded collision-checked split identifiers, undo, and redo.
- Enter and Tab use explicit role policies. Enter maps General/Note to the same role, Scene Heading to Action, Action to Action, Character to Dialogue, Dialogue at its collapsed UTF-8-safe end to Action, Parenthetical to Dialogue, and Transition to Scene Heading. A collapsed Dialogue caret at an interior UTF-8 boundary splits the element: the original identifier and exact prefix remain, a distinct bounded identifier owns the exact suffix as Dialogue, and the caret moves to the new element start. Empty Dialogue is an end-boundary case. Rejected noncollapsed selections, invalid boundaries, and capacity failures preserve the session.
- Tab maps Scene Heading to Action, Action to Character, blank Character to Transition, Transition to Scene Heading, Dialogue to Parenthetical, and Parenthetical to Dialogue. Unlisted Tab cases are consumed no-ops and never insert literal tab text or create history. Role transitions are transactional and undoable.
- The persisted `uppercase_cues` policy defaults to enabled for new projects and profiles missing the legacy field. When enabled, transactional insertion, paste, role assignment, and role transitions normalize only Scene Heading and Character text using Unicode uppercase and UTF-8-safe transformed-prefix selection mapping. Opening a project does not rewrite stored text; other roles preserve source casing.

### Parenthetical editing

- The persisted `auto_parenthetical` and `strict_transition_boundaries` profile policies default to enabled for new projects and profiles missing either legacy field. Malformed field types are rejected, and both settings round-trip with the profile.
- When automatic generation applies to a transition or assignment into Parenthetical, it creates an empty `()` pair with the caret between the delimiters. Its transient provenance is keyed by stable element identifier, participates in undo/redo, and is excluded from native project content. A successful content-changing edit invalidates the mark; a failed edit does not.
- The Enter-specific generated-pair cleanup applies only when the marked element is still Parenthetical and contains exactly `()`. Parenthetical Enter behavior also supports insertion before the opening delimiter and configured delimiter repair; strict-boundary rejection is a no-op, and edits preserve transactional undo/rollback behavior. These Enter rules do not make provenance a prerequisite for normal later Tab or Delete handling.
- Pasted parentheses remain intact at paste time and then behave as ordinary document delimiters. With automatic parentheticals enabled, Tab handles delimiter-only `(`, `)`, and `()` as an empty Dialogue transition regardless of provenance. Delete handles delimiter-only Parentheticals using the previous-element fallback, or the first/only-element fallback when no previous element exists. Nonempty authored content is preserved. These explicit commands do not make every authored or partial delimiter permanently immune to handling. Disabling automatic parentheticals honors the setting without changing the distinct generated-pair Enter rules.

### Fixed layout and source mapping

- The named, versioned, bounded `us_letter_screenplay` profile defines US Letter pages of 612 × 792 points; left, right, top, and bottom margins of 108, 72, 72, and 72 points; seven-point fixed character cells; twelve-point same-element line advance; twelve-point boundary gap; and 55 nominal lines.
- The profile's role boxes are Scene Heading, Action, and Note 108–540 points; Character 259–434; Parenthetical 216–396; Dialogue 180–396; and right-aligned Transition 432–540. New projects default to Courier Prime at 12 points. Supported existing Courier and Courier New profiles remain unchanged when opened and saved.
- Layout is deterministic for the same semantic project and complete profile and is independent of viewport scaling. Text wraps by role without inserting formatting into canonical text; keep relationships, title-page separation, and a safe oversized-element escape preserve bounded layout behavior. LF, CRLF, and standalone CR are equivalent paragraph delimiters for layout without creating phantom carriage-return lines. Source ranges remain UTF-8 byte based.
- For body spacing, each incoming Dialogue or Parenthetical receives zero additional boundary gap; every other incoming body role uses the profile boundary gap. Wrapped lines and title-page boundaries do not add body boundary spacing.
- Layout provides page/content bounds, role-aware line geometry, page assignment, keep-with-next and oversized indicators, source byte ranges, and UTF-8-safe caret and hit-test mappings. Body pointer hit testing chooses the document-global nearest rendered body line by vertical distance, preferring the earlier line and then the lower page index on exact ties. It clamps horizontal misses to the line's source start or end, delegates in-line positions to the source mapping, excludes title pages from the body hit-test domain, falls through empty pages to nonempty body lines, and clamps document-span clicks to the first or last body line.

### Layout cache and desktop integration

- Cached layout is immutable derived data and is valid only for the current semantic project and its complete profile. A semantic revision or profile change invalidates it, and replacing a project or editing session must not reuse geometry derived from the prior project even when revision and profile values match. Viewport scaling is presentation-only and does not change fixed pagination or source mappings.
- Desktop paint, hit testing, caret geometry, and visual-line navigation consume the appropriate shared layout and source mappings. Preferred horizontal movement goal is transient and is not persisted. PDF consumes fixed print layout rather than viewport layout.

### Native persistence and exports

- The native representation is schema version 1 and round-trips semantic roles, identifiers, metadata, canonical text, title-page content, and page-profile inputs. Missing legacy profile fields use their specified defaults; malformed data and unsupported schema versions are rejected safely. The contract does not promise cross-version migrations.
- Native serialization and parsing enforce the project bounds and UTF-8 validity. Native content excludes caret, selection, derived layout, undo history, and transient generated-parenthetical provenance.
- Saving prepares bounded native data and atomically replaces the named destination. Failed preparation, write, or replacement preserves the last valid destination and the session's unsaved/dirty state. The session is marked clean only after successful replacement. No recovery-snapshot behavior is part of this capability contract.
- PDF and plain-text outputs are noncanonical and do not replace the native source or mark it clean. PDF uses fixed layout and preserves non-ASCII characters. Plain text is deliberately lossy and communicates that screenplay roles, stable identifiers, and page layout are not preserved.

## Decisions

- **The semantic native project is canonical; layout and exports are derived.** This keeps editing, saved state, and stable identity anchored in one project while allowing fixed geometry and review outputs to be regenerated without changing that source.
- **The core is a reusable headless capability, not a desktop widget or independently runnable container.** Desktop and other suitable hosts delegate semantic operations to it; platform interaction and window chrome remain outside its boundary.
- **Schema version 1 defines the supported native compatibility boundary.** Legacy missing fields receive their specified defaults and unsupported versions fail safely; no cross-version migration commitment is implied.

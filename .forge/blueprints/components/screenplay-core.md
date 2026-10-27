# Component Blueprint: Screenplay Core

## Capability Summary

Screenplay Core is the desired reusable capability behind the desktop shell.
It owns semantic project data, explicit screenplay roles, checked positions and
selections, transactional editing/session behavior, fixed-profile layout, and
native persistence/recovery contracts. It must be usable by headless tests and
must not depend on window widgets.

## Document status

- **Status:** `draft` — desired MVP component; implementation evidence exists for model, editing, and layout; independent blueprint review/WO-004 validation remains separate
- **Owner:** Forge initialization agent
- **Last updated:** 2026-10-25
- **Containers:** [`../containers/scriptcraft-desktop.md`](../containers/scriptcraft-desktop.md)
- **Evidence basis:** `.analysis` recommendations; no current source implementation observed

## Core Components

```component
name: ProjectModel
container: ScriptCraft Desktop Application
responsibilities:
  - Own versioned Project metadata, title-page elements, ordered screenplay elements, stable IDs, roles, and PageProfile inputs.
  - Validate bounded structure and preserve canonical UTF-8 text.
```

```component
name: EditingSession
container: ScriptCraft Desktop Application
responsibilities:
  - Own positions, anchor/focus selection, semantic edit operations, role transitions, dirty state, revision, and bounded undo/redo.
  - Apply mutations transactionally and return typed errors without corrupting state.
```

```component
name: ScreenplayLayout
container: ScriptCraft Desktop Application
responsibilities:
  - Resolve role rules through one fixed profile, wrap text, paginate, and return source mappings/caret geometry.
  - Produce the same semantic/page inputs for continuous writing, page preview, and print export.
```

```component
name: NativePersistence
container: ScriptCraft Desktop Application
responsibilities:
  - Serialize/parse the versioned native project with bounds and migrations.
  - Perform atomic save and maintain bounded recovery candidates without silently overwriting the named project.
```

`ProjectModel` is the durable semantic source. `EditingSession` changes it and
invalidates derived layout. `ScreenplayLayout` reads a valid project and emits
temporary geometry; it does not mutate the model. `NativePersistence` reads and
writes semantic project data, not caret/layout/undo state. The desktop container
adapts user input and consumes these results.

## System Contracts

### Key Contracts

- Role is semantic data, not inferred appearance.
- Element identity is stable and opaque; array position and page coordinates are not durable identity.
- Valid edit offsets are checked UTF-8 boundaries.
- A failed mutation restores project, selection, history, and dirty-state invariants.
- `EditingSession` owns explicit Enter and Tab role policies; unlisted Tab cases are consumed no-ops and never insert literal tab text. Role transitions are transactional and undoable.
- Dialogue Enter is boundary-sensitive: a collapsed UTF-8-safe caret at the element end uses Dialogue→Action, while an interior collapsed boundary splits prefix and suffix into two Dialogue elements. The original stable ID is retained, the inserted ID is bounded/collision-checked, and failed selections/capacity errors leave project, selection, history, revision, and dirty state unchanged.
- Layout is deterministic for the same project/profile/font inputs and has a safe oversized-element escape hatch. Body spacing uses an explicit incoming-role policy: every incoming Dialogue or Parenthetical has zero extra boundary gap, while all other body transitions use `PageProfile.boundary_gap`; wrapped lines and title-page elements remain excluded from body boundary spacing.
- Body pointer hit testing is a document-global nearest-line query over rendered body pages. It chooses by absolute vertical distance, prefers the earlier line and then lower page index on exact ties, clamps horizontal misses to the line source start/end, and delegates in-line positions to the UTF-8-safe source mapping. Title pages are not a body hit-test domain; empty pages fall through to non-empty body lines, and document-span clicks clamp to the first/last body line.
- Native save failure preserves the last valid destination; recovery is a candidate, never a silent overwrite.
- The persisted `PageProfile.uppercase_cues` policy defaults true for new projects and for legacy profiles missing the field. Opening a project never rewrites existing text; transactional insertion, paste, role assignment, and role transitions normalize only Scene Heading and Character text when enabled using Rust Unicode uppercase with UTF-8-safe transformed-prefix caret/selection mapping. Other roles preserve source casing, and the setting is not a UI concern in this scope.
- Newly entered Parenthetical elements receive transiently marked generated `()` text with the caret at byte offset 1. The mark is keyed by stable element ID, participates in undo/redo snapshots, and is excluded from native persistence. A successful edit that changes a marked element invalidates its mark before later cleanup decisions; failed edits do not mutate it. Enter/Tab removes the text only when the marked element is still Parenthetical and exactly `()`; user-authored, pasted, partial, and non-empty parentheses remain canonical text.

### Integration Contracts

- Desktop UI calls semantic operations and consumes pages/display records; it does not directly edit a parallel string.
- PDF consumes print layout/display output; export does not become the native source.
- Headless tests call the same core operations as the UI.
- The desktop adapter derives visual-line vertical movement from `ScreenplayLayout` source ranges and geometry. Its preferred horizontal goal is transient UI/session state and is never serialized.

## Architecture Decision Records

### ADR-001: One small semantic core before crate splitting

**Context:** `.analysis` considers several possible crates, but the current checkout has no verified package baseline and early boundaries are still being discovered.

**Decision:** Begin with a deliberately bounded core capability whose modules have clear ownership; split into crates only after an independently demonstrated reuse, dependency, or compile-time need.

**Consequences:** Faster discovery and fewer premature public APIs, while preserving a path to separate model/layout/render/persistence packages later.

## WO-006 verified persistence facts

- Native persistence is implemented in [`crates/scriptcraft-core/src/persistence.rs`](../../../crates/scriptcraft-core/src/persistence.rs) without filesystem dependencies in the semantic model. The supported schema remains version `1` and the existing bounded JSON model is the native representation.
- [`atomic_save`](../../../crates/scriptcraft-core/src/persistence.rs) prepares bounded bytes and replaces the destination through a sibling temporary path; a failed write/rename leaves the prior destination intact. [`EditingSession::mark_saved`](../../../crates/scriptcraft-core/src/editing.rs) is called only after replacement succeeds.
- [`save_recovery`](../../../crates/scriptcraft-core/src/persistence.rs) retains three revision-named snapshots. [`discover_recovery`](../../../crates/scriptcraft-core/src/persistence.rs) skips malformed/unknown candidates, filters candidates at or below the saved revision, and returns newer candidates for explicit host offer rather than silent replacement.

## Completion evidence placeholder

`TODO: link implementation symbols, headless tests, persistence fixtures, pagination fixtures, and independent blueprint review.`

## WO-006 measured responsiveness boundary

- The core benchmark fixture [`wo006_responsiveness.rs`](../../../crates/scriptcraft-core/tests/wo006_responsiveness.rs) measures complete deterministic layout builds at 1, 5, 15, 50, 120, and 180 pages with three warmups, ten samples, and beginning/middle/end edits. It retains only generated fixture dimensions and timing summaries; document text is not reported.
- The reusable core exposes the session-owned `LayoutCache` adapter used by the desktop and the headless benchmark. It is keyed by editing-session revision and the complete persisted `PageProfile`; the profile equality comparison covers paper dimensions, margins, line/page metrics, boundary policy, font identity/size/character metric, role boxes, and casing policy. Viewport scale is presentation-only. A revision or profile change misses the cache, while unchanged reads share the immutable layout.
- The same cached layout is consumed by page paint, hit testing, caret geometry, and vertical navigation. PDF export continues to request a fresh core layout, preserving deterministic output independent of transient desktop state.

## Revision history

| Date | Change | Evidence/rationale |
|---|---|---|
| 2026-10-25 | Created the only necessary shared MVP capability blueprint | `.analysis` architecture synthesis and project-structure proposal; desired behavior clearly labeled |
| 2026-10-25 | Established one reusable foundation crate | WO-001 kept the one-core direction and proved no UI/window dependency in the reusable crate |

## WO-001 verified foundation facts

- `crates/scriptcraft-core` is the single reusable foundation crate.
- Its headless startup status and unit test are implemented in [`src/lib.rs`](../../../crates/scriptcraft-core/src/lib.rs).
- Semantic model, editing/session, layout, and persistence behavior are intentionally not implemented by WO-001.

## WO-004 verified layout facts

- [`ScreenplayLayout`](../../../crates/scriptcraft-core/src/layout.rs:51) is a dependency-free, backend-neutral derived view over a validated project.
- The approved deterministic format is named/versioned `us_letter_screenplay` profile v1: US Letter `612 × 792` points (`72 points/inch`), margins left/right/top/bottom `108/72/72/72`, seven-unit fixed character cells, twelve-unit same-element line advance, and a twelve-unit boundary gap. Role boxes are scene/action/note `108..540`, character `259..434`, parenthetical `216..396`, dialogue `180..396`, and right-aligned transition `432..540`; notes inherit action geometry. Explicit bounded profile values are persisted without UI pixels or per-element formatting. New profiles default to the `Courier` family; existing supported stored families remain unchanged.
- Layout emits page/content bounds, role-aware line rectangles, source byte ranges, page assignment, title-page separation, keep-with-next flags, oversized flags, and safe hit/caret mappings. It never mutates canonical model text.

## WO-003 verified long-document facts

- Bounded pasted text remains transactional at the model/editing boundary: an oversized insertion returns `EditError::TextTooLong` before session state, selection, history, revision, or dirty state changes.
- The existing bounds remain in force (`MAX_ELEMENTS = 10,000`, `MAX_TEXT_BYTES = 262,144`, `MAX_JSON_BYTES = 1,048,576`); `Project::to_json()` now enforces the JSON bound at the model serialization boundary as well as persistence serialization.
- Layout treats LF, CRLF, and standalone CR as equivalent paragraph delimiters without emitting a phantom carriage-return line. Source ranges remain byte-based, so a bounded multi-page paste reaches its final-page caret and round-trips through native persistence without semantic changes.
- Invalid layout/model and persistence results are reported as recoverable errors; the desktop surface presents layout failures through the same unchanged-document error path rather than replacing the last valid session.

## WO-002 verified paste facts

- Clipboard insertion is an explicit transactional editing boundary. Embedded LF, CRLF, CR, whitespace, and Unicode remain canonical text in the active semantic element; layout interprets line delimiters for visual wrapping without inventing roles or elements.
- A bounded paste is one edit-history entry. An oversized paste returns the typed text-size error before committing project, selection, history, revision, or dirty-state changes, and a subsequent valid edit remains possible.

Evidence: [`wo002_paste_safety.rs`](../../../crates/scriptcraft-core/tests/wo002_paste_safety.rs) and [`editing.rs`](../../../crates/scriptcraft-core/src/editing.rs).

## Follow-up: bounded split identity and health probes

- Paragraph splitting generates bounded collision-checked IDs rather than recursively extending the source ID. This preserves the stable-ID limit during long sequences of Enter operations.
- [`EditingSession::validate()`](../../../crates/scriptcraft-core/src/editing.rs:109) provides a non-mutating host health probe for live project, selection, and serialization diagnostics.

Evidence: [`wo003_long_document.rs`](../../../crates/scriptcraft-core/tests/wo003_long_document.rs), layout/model changes in [`layout.rs`](../../../crates/scriptcraft-core/src/layout.rs) and [`lib.rs`](../../../crates/scriptcraft-core/src/lib.rs), and desktop error routing in [`main.rs`](../../../apps/scriptcraft/src/main.rs).

## WO-002a verified profile and renderer facts

- `PageProfile` is the single bounded, versioned owner of the Courier-family target (`font_family`, `font_size`), deterministic character metric, line advance, boundary gap, and all seven role boxes. Native JSON round trips preserve default and alternate profiles; invalid role geometry and metric values are rejected before layout.
- `ScreenplayLayout` derives wrapping, line rectangles, role edges, hit testing, caret rectangles, pagination, and page capacity from the persisted profile. The desktop renderer uses the profile's 12-point base size and the built-in monospace family as the deterministic fallback when a named Courier face is not installed; viewport scale is presentation-only.
- PDF export consumes the same line rectangles and persisted font size. No override editor or per-element pixel formatting exists.

Evidence: [`lib.rs`](../../../crates/scriptcraft-core/src/lib.rs), [`layout.rs`](../../../crates/scriptcraft-core/src/layout.rs), [`exports.rs`](../../../crates/scriptcraft-core/src/exports.rs), [`main.rs`](../../../apps/scriptcraft/src/main.rs), and focused profile/parity tests.

## WO-004 verified spacing and export facts

- The fixed profile is `612 × 792` points with top/bottom/left/right margins `72/72/108/72`, `55` nominal line capacity, seven-unit fixed character cells, twelve-unit same-element line boxes, and twelve-unit inter-element boundary gaps. The renderer uses a 12-point Courier-family target and keeps viewport scaling out of print geometry.
- Structural regressions cover content bounds, explicit role edges, adjacent same-element and boundary advances, line boxes, caret heights, source mappings, empty/wrapped/keep-with-next lines, and page-capacity resets. The desktop regression confirms viewport scale transforms fixed layout coordinates without changing page structure.
- PDF export consumes the same `ScreenplayLayout` line rectangles; regression coverage verifies x positions and the PDF coordinate transform `paper_height - y - height`, including twelve-unit adjacent-line spacing. The shared screen/PDF contract therefore remains unchanged.

Evidence: WO-004 regression coverage in [`layout.rs`](../../../crates/scriptcraft-core/src/layout.rs), [`exports.rs`](../../../crates/scriptcraft-core/src/exports.rs), and [`main.rs`](../../../apps/scriptcraft/src/main.rs); implementer evidence in [`WO-004-screenplay-spacing-contract.md`](../../initiatives/2026-10-25-ui-quality-of-life/work-orders/WO-004-screenplay-spacing-contract.md).

## WO-007 verified policy facts

`PageProfile` schema version 1 owns `auto_parenthetical` and `strict_transition_boundaries`; both default true for new and legacy-missing profiles, reject malformed JSON types, round-trip through native persistence and `replace_with_saved()`, and participate in complete profile cache identity. Parenthetical shape repairs, empty-delete fallback, before-opening Enter, and strict UTF-8 end-boundary transitions preserve transactional rollback and no-op/history invariants.

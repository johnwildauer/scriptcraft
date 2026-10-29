# Feature Blueprint: Dependable Screenplay Writing Loop

## Feature Summary

The feature flows from a semantic `Project` through a transactional editing
session and a fixed-profile layout pipeline to desktop/page/PDF consumers, with
native persistence and bounded recovery at the side boundary. The blueprint is
an intended technical composition, not a claim that these components exist.

## Document status

- **Status:** `draft` — desired MVP composition; independent blueprint review not performed
- **Owner:** Forge initialization agent
- **Last updated:** 2026-10-25
- **Requirements:** [`../../requirements/features/screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md)
- **Evidence basis:** `.analysis` research and user-approved scope; current implementation unknown

## Component Blueprint Composition

Compose [`#ProjectModel`](../components/screenplay-core.md#projectmodel),
[`#EditingSession`](../components/screenplay-core.md#editingsession),
[`#ScreenplayLayout`](../components/screenplay-core.md#screenplaylayout), and
[`#NativePersistence`](../components/screenplay-core.md#nativepersistence)
inside [`@ScriptCraft Desktop Application`](../containers/scriptcraft-desktop.md).
The UI sends semantic input to `#EditingSession`; the session mutates and
validates `#ProjectModel`; `#ScreenplayLayout` derives writing/page geometry;
and `#NativePersistence` handles the native source and recovery boundary.

## Feature-Specific Components

```component
name: RoleTransitionPolicy
container: ScriptCraft Desktop Application
responsibilities:
  - Map explicit current roles and editing actions to predictable next roles.
  - Keep transitions testable and undoable; never infer roles from appearance.
```

```component
name: ExportLossBoundary
container: ScriptCraft Desktop Application
responsibilities:
  - Request fixed print layout for PDF and emit a deliberate loss report/status for plain text.
  - Keep exports separate from native save cleanliness and canonical project identity.
```

## Models and contracts

```model
name: Project
store: versioned native project file
description: Canonical semantic screenplay project and fixed page-profile inputs.
fields:
  - id: opaque stable project identifier
  - schema_version: supported native schema integer
  - metadata: title, author, contact, timestamps
  - title_page: explicit title-page elements
  - page_profile: named fixed paper/role layout inputs
  - elements: ordered ScreenplayElement list
constraints:
  - roles are explicit and text is canonical UTF-8
  - transient selection, layout coordinates, undo history, and UI state are not required content
```

```model
name: ScreenplayElement
store: Project.elements
description: One ordered semantic writing item.
fields:
  - id: opaque stable element identifier
  - role: scene_heading | action | character | dialogue | parenthetical | transition | note
  - text: canonical UTF-8 string
constraints:
  - element order is writing order
  - role meaning is independent of typography/indentation
```

### Key Contracts

- `REQ-WRITING-LOOP-001` behavior is satisfied by explicit `ScreenplayElement` roles, checked editing operations, and transactional session state.
- New `Project` instances seed exactly one empty `SceneHeading`; the initial valid caret is session state, while subsequent Enter/Tab transitions remain owned by `RoleTransitionPolicy`/`EditingSession`.
- `REQ-WRITING-LOOP-002` behavior is satisfied by one role-aware layout/display path with a fixed profile and source mappings.
- `REQ-WRITING-LOOP-003` behavior is satisfied by a versioned native schema, atomic replacement, and bounded recovery candidates.
- `REQ-WRITING-LOOP-004` behavior is satisfied by export adapters that consume print layout and report loss without becoming canonical save.

### Integration Contracts

- Model/session → layout: valid semantic project plus profile; output is temporary pages/lines/source mappings.
- Layout → desktop/PDF: backend-neutral display data; consumers do not independently paginate.
- The layout display data uses the named/versioned `us_letter_screenplay` profile: inch-defined values become points at the format boundary, with explicit role boxes, source byte ranges, page assignment, valid-boundary hit/caret geometry, same-element single spacing, and an explicit body role-pair gap policy. Incoming Dialogue and Parenthetical elements have zero extra gap from every body predecessor; other incoming roles use the profile boundary gap. Title-page boundaries are excluded and viewport zoom is presentation-only.
- Persistence → session: successful native save changes clean/path state only after replacement succeeds; failures preserve prior state.
- The persisted profile is also the geometry/font seam: layout, caret/hit testing, desktop selection/IME rendering, pagination, and PDF consume its role boxes, character metric, line advance, boundary gap, and font size. New projects default to `Courier Prime`; explicit supported stored families remain unchanged on open/save, with `Courier` and `Courier New` retaining their established renderer path. The desktop embeds the supplied regular Courier Prime face at build time; fixed character cells remain authoritative and viewport scale is presentation-only. PDF retains the existing deterministic Type1 family mapping and does not embed the supplied TTF.

## Requirement alignment

- `REQ-WRITING-LOOP-001` → `#ProjectModel`, `#EditingSession`, and `RoleTransitionPolicy`.
- `REQ-WRITING-LOOP-002` → `#ScreenplayLayout` and the shared display boundary.
- `REQ-WRITING-LOOP-003` → `#NativePersistence` and session save/recovery integration.
- `REQ-WRITING-LOOP-004` → `ExportLossBoundary` and print-layout integration.

## Architecture Decision Records

### ADR-001: Native semantic project is the source of truth

**Context:** DOCX/FDX and visual exports are broad or lossy and are explicitly out of the MVP boundary.

**Decision:** Use a ScriptCraft-owned versioned native project for semantic roles, IDs, metadata, and page-profile inputs; treat PDF/plain text as explicit outputs.

**Consequences:** The MVP can validate the writing loop without compatibility scope, but schema migration and export-loss contracts must be designed before implementation.

### WO-003 verified integration facts

- Multi-page editing, layout, and native persistence share the same validated semantic project. Controlled fixtures cover independent element count, text size, wrapped-line/page count, LF/CRLF/CR paste patterns, failed edits, invalid layout input, final-page caret geometry, and save/reopen semantics.
- The bounded safety contract is preserved: no model limits were raised and schema version `1` was unchanged. `Project::to_json()` rejects encoded output over `MAX_JSON_BYTES`, while failed session/layout/persistence operations leave the prior valid state or destination usable.
- The desktop layout error path now uses the existing recoverable error presentation (`Operation failed; document unchanged`) instead of only painting an untracked raw layout label.

## Completion evidence placeholder

`TODO: link implementation symbols, acceptance test evidence for AC-WRITING-001.1 through AC-WRITING-004.3, comparable diff evidence, and independent blueprint review.`

## WO-006 verified responsiveness facts

- Long-document responsiveness is measured rather than inferred: controlled 1/5/15/50/120/180-page generated fixtures use three warmups and ten samples, report p50/p95 layout milliseconds, and exercise edits near the beginning, middle, and end.
- The desktop's revision/profile-keyed layout cache is a derived optimization only. Complete profile equality and session revision provide invalidation; the shared result remains authoritative for screen hit/caret/navigation geometry, while export builds its deterministic layout independently.
- The unchanged 180-page read measured an approximately 100% layout-build p95 reduction in the bounded cache path (18.998 ms uncached versus below the 0.001 ms timer resolution cached in this run). No incremental pagination was added because the benchmark measured complete layout cost, not downstream pagination dominance.

## Revision history

| Date | Change | Evidence/rationale |
|---|---|---|
| 2026-10-25 | Created feature counterpart for the stable MVP requirement | Requirements and `.analysis` translated into a bounded technical composition; no implementation/review claimed |
| 2026-10-25 | Resolved WO-007 export adapter contract | `scriptcraft-core::exports` uses a dependency-free deterministic PDF 1.4 consumer over `ScreenplayLayout`; plain text carries an in-band `LossReport` for roles, stable IDs, and page layout. Outputs borrow the project and never become native save state. |

### WO-007 verified editing facts

The persisted profile owns default-on `auto_parenthetical` and `strict_transition_boundaries` without a schema bump. Editing consumes the approved parenthetical delimiter/empty-delete matrices and disables strict Scene Heading, Character, and Transition Tab/Enter commands away from collapsed UTF-8-safe text ends as successful no-ops.

### WO-008 Dialogue Enter facts

Dialogue Enter is boundary-sensitive: at a collapsed UTF-8-safe element end it retains Dialogue→Action, while at an interior collapsed boundary it splits and preserves the suffix as a new Dialogue element. The original ID remains on the prefix and the new ID is bounded and collision-checked. The transaction preserves source bytes, collapsed selection, undo/redo, dirty/revision/history behavior, and deterministic save/reopen semantics; rejected non-collapsed, invalid-boundary, and capacity operations are unchanged no-ops/errors.

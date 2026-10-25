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
- `REQ-WRITING-LOOP-002` behavior is satisfied by one role-aware layout/display path with a fixed profile and source mappings.
- `REQ-WRITING-LOOP-003` behavior is satisfied by a versioned native schema, atomic replacement, and bounded recovery candidates.
- `REQ-WRITING-LOOP-004` behavior is satisfied by export adapters that consume print layout and report loss without becoming canonical save.

### Integration Contracts

- Model/session → layout: valid semantic project plus profile; output is temporary pages/lines/source mappings.
- Layout → desktop/PDF: backend-neutral display data; consumers do not independently paginate.
- The initial layout display data uses fixed integer metrics and carries source byte ranges, page assignment, role boxes, and valid-boundary hit/caret geometry; viewport zoom is not an input.
- Persistence → session: successful native save changes clean/path state only after replacement succeeds; failures preserve prior state.

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

## Completion evidence placeholder

`TODO: link implementation symbols, acceptance test evidence for AC-WRITING-001.1 through AC-WRITING-004.3, comparable diff evidence, and independent blueprint review.`

## Revision history

| Date | Change | Evidence/rationale |
|---|---|---|
| 2026-10-25 | Created feature counterpart for the stable MVP requirement | Requirements and `.analysis` translated into a bounded technical composition; no implementation/review claimed |
| 2026-10-25 | Resolved WO-007 export adapter contract | `scriptcraft-core::exports` uses a dependency-free deterministic PDF 1.4 consumer over `ScreenplayLayout`; plain text carries an in-band `LossReport` for roles, stable IDs, and page layout. Outputs borrow the project and never become native save state. |

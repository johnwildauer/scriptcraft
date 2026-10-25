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
- Layout is deterministic for the same project/profile/font inputs and has a safe oversized-element escape hatch.
- Native save failure preserves the last valid destination; recovery is a candidate, never a silent overwrite.

### Integration Contracts

- Desktop UI calls semantic operations and consumes pages/display records; it does not directly edit a parallel string.
- PDF consumes print layout/display output; export does not become the native source.
- Headless tests call the same core operations as the UI.

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
- The initial deterministic metrics are `ScriptCraft Fixed Metrics`, seven units per character cell, twelve units per line, and the model's fixed US-letter profile (`612 × 792`, margins `90/72/72/72`, nominal 55 columns/lines).
- Layout emits page/content bounds, role-aware line rectangles, source byte ranges, page assignment, title-page separation, keep-with-next flags, oversized flags, and safe hit/caret mappings. It never mutates canonical model text.

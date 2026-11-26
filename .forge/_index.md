# Software Forge

`.forge/` is the portable, Markdown-native workflow for the durable understanding of a host repository.

## Start here

At the user's explicit direction, adapt and record project-policy changes in the appropriate durable project document; an orchestrator may dispatch an architect or coding agent to adapt host-project process documents. Respect applicable external rules and checks: do not silently discard or weaken them, and communicate genuine uncertainty or blockers. 

## Directory map

| Directory | Purpose |
|---|---|
| [`project-manifest.md`](project-manifest.md) | Durable project state: intake answers, involvement level, repository baseline, working agreement, project policy, roadmap, and revision history. |
| [`requirements/`](requirements/) | Durable product intent, user-facing capabilities, and testable acceptance criteria. |
| [`blueprints/`](blueprints/) | Durable technical context, architecture, component behavior, contracts, and feature composition. |

## Beat Board initiative

- **User-facing behavior:** [`requirements/features/beat-board.md`](requirements/features/beat-board.md), with lifecycle, workspace, screenplay-editing, page-layout, settings, and export boundaries linked from that feature.
- **Technical composition:** [`blueprints/features/beat-board.md`](blueprints/features/beat-board.md), supported by the Document Model, Editing Session, FDX Preservation and Storage, Command Contract, Desktop Adapters, Layout and Pagination, User Settings, desktop container, and [technical context](blueprints/technical-context.md).
- **Execution and user handoff:** [approved plan and work-order guide](../.plan/2026-11-24-beat-board/beat-board.md); [unexecuted user-owned review worksheet](../.plan/2026-11-24-beat-board/final-user-review.md). The worksheet is not acceptance evidence.

## ScriptNotes initiative

- **User-facing behavior:** [ScriptNotes requirements](requirements/features/script-notes.md), alongside its native-document lifecycle and export boundaries.
- **Technical composition:** [ScriptNotes blueprint](blueprints/features/script-notes.md), with the current integration map in [technical context](blueprints/technical-context.md).
- **Execution and user handoff:** [ScriptNotes work-order plan](../.plan/2026-11-25-script-notes/work-orders/README.md); [unchecked, user-owned review worksheet](../.plan/2026-11-25-script-notes/final-user-review.md). The worksheet is unexecuted and is not acceptance evidence.

## Alt Dialogue initiative

- **User-facing behavior:** [Alt Dialogue requirements](requirements/features/alt-dialogue.md), alongside the linked screenplay-editing, dual-dialogue, and native-document lifecycle boundaries.
- **Technical composition:** [Alt Dialogue blueprint](blueprints/features/alt-dialogue.md), with the current source map in [technical context](blueprints/technical-context.md).
- **Execution and user handoff:** [Alt Dialogue execution guide](../.plan/2026-11-26-alt-dialogue/investigation.md); [unchecked, user-owned review worksheet](../.plan/2026-11-26-alt-dialogue/final-user-review.md). The worksheet is a handoff, not evidence.

## Core boundaries

- **Requirements** describe durable desired product behavior: what users and stakeholders need and why. Cohesive area files may group related capabilities and retain stable requirement/acceptance IDs; they do not prescribe internal implementation or carry lifecycle status, implementation evidence, progress, or history.
- **Blueprints** describe durable technical architecture and contracts in established-state voice. The technical context maps the final repository's active packages, production source/resource locations, and zero-permanent-test policy; these source maps are not execution or acceptance evidence. Other intended paths may precede code. Approval describes the target, not proof that current code implements it.

When a planned change needs a durable requirements or blueprint update, a narrow documentation-first work order before related implementation is a useful default, not a requirement. Link dependent work orders and record expected document/code lag once in the initiative execution handoff. Use this only where there is meaningful durable impact; if no relevant durable change is needed, do not create a documentation work order. Existing code may lag target documents until dependent implementation is complete. Initialization reviews remain required by default and their findings belong in separate concise review artifacts; review outcomes do not require status fields in requirements or blueprints.

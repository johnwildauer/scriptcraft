# Product Overview — ScriptCraft

## Document status

- **Status:** `draft` — initialized baseline; independent requirements review not performed
- **Owner:** Forge initialization agent
- **Last updated:** 2026-10-25
- **Evidence basis:** both — user-approved product intent plus repository `.analysis/` research; no implementation is claimed

## Business problem

Individual screenwriters need a calm, dependable writing loop rather than a
general-purpose document suite. Losing semantic element roles, predictable page
flow, or the last saved version interrupts creative work and makes a draft hard
to trust. ScriptCraft should protect the draft first and add breadth only when a
screenplay-specific need justifies it.

## Current state

- **Observed:** The repository contains `.analysis/` research and proposal documents plus Forge workflow scaffolding. Workspace inventory found no source files under `apps/scriptcraft/` or `crates/`.
- **Observed:** Git is initialized on `master`; there are no commits yet and no remote entries were returned by the recorded Git check.
- **Inferred:** The product implementation is not yet established in this checkout; the analysis is planning input, not proof of existing runtime behavior.
- **Desired:** A desktop-first Rust application with a headless-capable core and a dependable screenplay writing loop.

## Personas and stakeholders

| Persona/stakeholder | Goals | Pain points | Success looks like |
|---|---|---|---|
| Individual screenwriter | Draft and revise scenes quickly while preserving screenplay meaning | Data loss, role ambiguity, unstable pagination, distracting breadth | Can write a scene, undo safely, preview pages, save/reopen, recover, and export without losing intent |
| Product owner | Establish a small trustworthy MVP | Scope drift toward Word parity or future platform concerns | A reviewable vertical slice with explicit boundaries and evidence |
| Maintainer/agent | Extend the app without weakening safety or boundaries | Unknown baseline and unverified commands | Durable requirements/blueprints, stable IDs, bounded changes, and fresh validation |

## Product description

ScriptCraft is a focused desktop screenplay editor for one writer. Its source
of truth is a semantic project containing title-page information, an ordered
sequence of screenplay elements, stable element identity, and a fixed page
profile. The editor session owns transient selection, undo/redo, dirty state,
and derived layout. A shared layout/display path should support continuous
writing, page preview, and PDF output; native persistence and recovery remain
separate from lossy exports.

## MVP and boundaries

### Workspace MVP

The workspace MVP is the first runnable and verifiable Rust development
foundation: selected package structure, setup instructions, baseline checks, and
a smoke/manual path. It is not permission to create product work orders in this
initialization task.

### Product MVP

The product MVP is the smallest useful dependable writing loop for one desktop
screenwriter. Its scope is derived from `.analysis/06-screenplay-mvp-project-structure.md`
and related Stage 2 reports, but it is expressed here as product behavior, not
as commands or an automatic implementation plan.

### In scope

- Create a blank screenplay project with title-page metadata.
- Edit an ordered semantic element sequence with explicit roles: scene heading, action, character, dialogue, parenthetical, transition, and authored note.
- Move caret/selection safely through UTF-8 text; support split, join, delete, undo, and redo.
- Use explicit role-aware transitions rather than inferring meaning from capitalization, spaces, tabs, or indentation.
- Provide continuous writing mode and deterministic fixed-profile page preview from shared layout semantics.
- Save and reopen a versioned native project atomically; preserve roles, IDs, metadata, and page-profile inputs.
- Offer bounded recent recovery snapshots after interrupted sessions.
- Export PDF from print layout and a deliberately lossy plain-text copy with loss communicated.
- Keep core behavior headless-capable and testable without a desktop window.

### Out of scope

- DOCX and FDX import/export or native compatibility.
- Collaboration, cloud sync, tracked revisions, comments, or revision comparison.
- MCP, remote control, network automation, or a command catalog as a product feature.
- Planning, beat-board, outline-editor, structure-line, production breakdown, scheduling, budgeting, or character databases.
- Broad Word-like formatting, tables, fields, headers/footers, footnotes, shapes, images, equations, arbitrary nested stories, dual dialogue, scene numbering, colored pages, or locked pages.
- Web support, multi-platform polish, pixel-perfect editor parity, font databases, and unmeasured incremental pagination.

## Success metrics

| Metric | Baseline | Target | Measurement method |
|---|---|---|---|
| Representative writing-loop completion | Not implemented / not measured | Create → edit roles → paginate → undo/redo → save/reopen → recover/export without semantic loss | Headless acceptance test plus desktop smoke evidence |
| Native round-trip semantic preservation | No verified ScriptCraft format | Roles, text, stable IDs, metadata, and page profile remain equivalent after save/reopen | Deterministic serialization and semantic comparison tests |
| Failed-operation safety | No verified ScriptCraft behavior | Failed edits/saves leave prior valid in-memory or on-disk state intact | Failure-injection and rollback tests |
| Pagination determinism | No verified ScriptCraft behavior | Same project/profile produces stable structural page results independent of viewport zoom | Structural pagination fixtures |

## Product constraints

- Desktop-first; exact supported OS and UI framework are not yet verified.
- Rust is user-confirmed; Cargo is an agent-default pending manifest verification.
- Native project data must remain semantic and screen-independent; page coordinates, caret geometry, UI state, and undo stacks are derived/transient.
- No implementation or command is authorized merely by its appearance in `.analysis/`.
- Safety and no-data-loss behavior take priority over broad feature count.

## Feature map

- [`features/screenplay-writing-loop.md`](features/screenplay-writing-loop.md) — dependable semantic screenplay editing, layout, persistence, recovery, and MVP export loop

## Open questions

- Which desktop OS is the first supported target, and which UI framework best fits the verified workspace baseline?
- What native file extension and exact PDF/plain-text loss-report contract will be selected?
- What font/profile inputs are pinned for deterministic layout?
- Which baseline commands are actually available once a manifest and package structure exist?
- What independent requirements and blueprint review findings will be recorded before the manifest can be unblocked?

## Revision history

| Date | Change | Evidence/rationale |
|---|---|---|
| 2026-10-25 | Replaced template with ScriptCraft product baseline | User-approved goal/scope and `.analysis` research; current source behavior intentionally not claimed |

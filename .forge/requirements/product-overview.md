# Product Overview — ScriptCraft

## Business problem

Individual screenwriters need a calm, dependable writing loop rather than a
general-purpose document suite. Losing screenplay meaning, predictable page flow,
or the last saved version interrupts creative work and makes a draft hard to
trust. ScriptCraft protects the draft first and adds breadth only when a
screenplay-specific need justifies it.

## Audience and stakeholder needs

| Audience | Need | Desired outcome |
|---|---|---|
| Individual screenwriter | Draft and revise scenes without losing screenplay meaning or control of the work | Write, revise, undo safely, inspect page layout, save and reopen the native project, and export review copies with their limitations clear |
| Product steward | Keep the product focused on a dependable screenplay-writing loop | A small, coherent product boundary that prioritizes safety over broad feature count |

## Product description

ScriptCraft is a focused desktop screenplay editor for one writer. A screenplay
project has a semantic source of truth containing title-page information, an
ordered sequence of screenplay elements with stable identities, fixed page
profile inputs, and retained FDX source content. Editing preserves explicit roles and canonical UTF-8 text;
transient selection, undo/redo, dirty state, and derived layout do not replace
that source.

The writing experience supports continuous editing and a predictable fixed-page
preview from shared layout semantics. The native project preserves screenplay
meaning across FDX save and reopen. PDF is a page-layout export; plain-text export is
deliberately lossy and communicates that roles, identities, and page layout are
not preserved. Neither export replaces the native project as the source of
truth.

## Product scope

The product is the smallest useful, dependable writing loop for one desktop
screenwriter.

### In scope

- Create a screenplay project with title-page information and the FDX content
  elements specified in the [writing-loop requirements](features/screenplay-writing-loop.md).
- Edit and navigate UTF-8 text safely, with split, join, delete, undo, and redo.
- Apply explicit role-aware transitions rather than infer meaning from
  capitalization, spaces, tabs, or indentation.
- Support continuous writing and deterministic fixed-profile page preview from
  shared layout semantics.
- Save and reopen native Version 6 FDX while preserving supported semantic data
  and safely retaining unsupported source content. Non-6 imports have the
  explicitly bounded conversion behavior specified in the writing-loop requirements.
  Save operations are atomic and failed operations preserve the prior valid work.
- Export PDF from fixed print layout and provide a deliberately lossy plain-text
  copy that communicates its loss.
- Keep core behavior headless-capable and testable without a desktop window.

### Out of scope

- DOCX compatibility and blanket Final Draft compatibility.
- Collaboration, cloud sync, tracked revisions, comments, or revision comparison.
- MCP, remote control, network automation, or a command catalog as a product
  feature.
- Planning, beat-board, outline-editor, structure-line, production breakdown,
  scheduling, budgeting, or character databases.
- Editing broad Word-like formatting, tables, fields, headers/footers, footnotes,
  shapes, images, equations, arbitrary nested stories, dual dialogue, scene numbering,
  colored pages, or locked pages. Retaining unsupported imported data is distinct
  from providing these editing features.
- Web support, multi-platform polish, pixel-perfect editor parity, font
  databases, and unmeasured incremental pagination.

## Success outcomes

| Outcome | Desired result | Measurement approach |
|---|---|---|
| Complete writing loop | A writer can create and edit screenplay roles, inspect pages, undo/redo, save and reopen, and produce PDF or plain-text exports without confusing exports with the native project | Headless acceptance coverage and desktop scenario checks |
| Native semantic preservation | Roles, text, stable identities, metadata, and page-profile inputs remain equivalent after native save and reopen | Deterministic serialization and semantic comparison checks |
| Failed-operation safety | A failed edit or save leaves the prior valid in-memory or on-disk state intact | Failure-injection and rollback checks |
| Predictable pagination | The same project and fixed page profile produce stable structural page results regardless of viewport zoom | Structural pagination fixtures |
| Honest export boundaries | PDF uses fixed print layout; plain-text output communicates that roles, identities, and page layout are lost | Export acceptance checks |

## Product constraints

- Desktop-first and focused on a single writer's project.
- Native project data remains semantic and independent of screen presentation;
  page coordinates, caret geometry, UI state, and undo stacks are derived or
  transient.
- Safety and preservation of the last valid native project take priority over
  breadth of features.

## Feature map

- [`features/screenplay-writing-loop.md`](features/screenplay-writing-loop.md) — semantic screenplay editing, fixed-profile layout, native persistence, and explicit exports.

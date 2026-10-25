# Feature Requirements: Dependable Screenplay Writing Loop

## Document status

- **Status:** `draft` — stable MVP requirement baseline; independent requirements review not performed
- **Owner:** Forge initialization agent
- **Last updated:** 2026-10-25
- **Product overview:** [`../product-overview.md`](../product-overview.md)
- **Blueprint counterpart:** [`../../blueprints/features/screenplay-writing-loop.md`](../../blueprints/features/screenplay-writing-loop.md)
- **Evidence basis:** Desired product behavior derived from user-approved scope and `.analysis`; current implementation is unknown.

## Overview

This feature is the smallest coherent product boundary for ScriptCraft: one
screenwriter can create and edit semantic screenplay elements, see predictable
writing/page views, and preserve work through native save, reopen, and bounded
recovery. It protects meaning and recoverability before compatibility or broad
formatting.

## Observed, inferred, and desired distinctions

- **Observed:** The checkout has planning analysis describing this loop; no ScriptCraft implementation files were found in the inspected app/crate directories.
- **Inferred:** The analysis represents the intended product direction and a useful architecture hypothesis, not a current contract.
- **Desired:** The behaviors and acceptance criteria below become the stable MVP contract after independent review.

## Terminology

| Term | Definition |
|---|---|
| Semantic element | An ordered screenplay text item with an explicit role and canonical text |
| Role | Screenplay meaning such as `scene_heading` or `dialogue`, independent of visual formatting |
| Native project | ScriptCraft-owned versioned project format and source of truth |
| Recovery snapshot | Separate bounded candidate used after interruption; never a silent overwrite |
| Page profile | Named fixed inputs used to derive deterministic screenplay pagination |

## Requirements

### REQ-WRITING-LOOP-001: Semantic screenplay editing

**User Story:** As an individual screenwriter, I want to create and edit an ordered screenplay with explicit element roles, so that my draft retains screenplay meaning while I write.

**Acceptance Criteria:**

- **AC-WRITING-001.1:** Given a new project, the system shall provide a valid editable starting element and title-page metadata without requiring visual formatting tricks.
- **AC-WRITING-001.2:** Given an element, the system shall preserve its explicit role and canonical UTF-8 text independently of capitalization, spaces, tabs, indentation, or font appearance.
- **AC-WRITING-001.3:** Given a valid selection, the system shall support insertion, deletion, split, join, role change, undo, and redo while preserving valid text boundaries and element order.
- **AC-WRITING-001.4:** Given a role transition such as scene heading → action or character → dialogue, the system shall apply an explicit, testable transition policy and make the operation undoable.
- **AC-WRITING-001.5:** Given malformed or out-of-bound input, the system shall return a bounded error without corrupting the project or session state.

### REQ-WRITING-LOOP-002: Shared deterministic writing and page views

**User Story:** As an individual screenwriter, I want continuous writing and fixed-page preview to agree about semantic content and layout inputs, so that I can write comfortably and inspect a predictable page result.

**Acceptance Criteria:**

- **AC-WRITING-002.1:** Given the same project and fixed page profile, the system shall derive stable page structure and source mappings independent of window zoom.
- **AC-WRITING-002.2:** Given role-aware layout, the system shall apply role measures and keep relationships without inserting spaces or tabs into canonical text.
- **AC-WRITING-002.3:** Given long text or an oversized element, the system shall wrap or split it safely without an infinite loop or unbounded blank-page behavior.
- **AC-WRITING-002.4:** Given a document position and rendered geometry, the system shall support a safe caret/hit-test round trip at valid text boundaries.

### REQ-WRITING-LOOP-003: Durable native save and recovery

**User Story:** As an individual screenwriter, I want to save, reopen, and recover my project without losing the last valid version, so that an interrupted session does not destroy my draft.

**Acceptance Criteria:**

- **AC-WRITING-003.1:** Given a valid project, the system shall serialize a versioned native representation containing semantic content, stable IDs, metadata, and page-profile inputs, while excluding transient caret/layout/undo state from required content.
- **AC-WRITING-003.2:** Given a save preparation or replacement failure, the system shall preserve the prior valid destination and retain an unsaved/dirty indication.
- **AC-WRITING-003.3:** Given a successfully saved native project, reopening it shall preserve semantic roles, text, stable IDs, metadata, and reproducible pagination inputs.
- **AC-WRITING-003.4:** Given a newer valid bounded recovery snapshot after interruption, the system shall offer it as a candidate and shall not silently overwrite the named project.

### REQ-WRITING-LOOP-004: Explicit MVP exports

**User Story:** As an individual screenwriter, I want to export a reviewable PDF and a plainly lossy text copy, so that I can inspect or share output without confusing exports with the native source.

**Acceptance Criteria:**

- **AC-WRITING-004.1:** Given a project, PDF export shall use fixed print layout rather than transient viewport layout.
- **AC-WRITING-004.2:** Given a PDF or plain-text export, the system shall not silently replace the native source of truth or falsely mark the native project clean.
- **AC-WRITING-004.3:** Given a lossy plain-text export, the result shall communicate that roles, IDs, and page layout are not preserved as a lossless round trip.

## Non-functional requirements

- **NFR-WRITING-001:** Production paths should prefer typed errors and bounded inputs over panic-prone handling; a discovered crash or data-loss defect requires a regression test.
- **NFR-WRITING-002:** Core editing, layout, and persistence behavior shall be exercisable without a desktop window for deterministic tests.
- **NFR-WRITING-003:** Native serialization and structural pagination outputs shall be deterministic for the same semantic project/profile inputs.

## Out of scope

- DOCX/FDX compatibility, collaboration, MCP, planning/beat-board, broad formatting, web/multi-platform polish, and all adjacent non-MVP features listed in the product overview.

## Traceability notes

- **Related product goal:** Dependable screenplay writing loop.
- **Dependencies:** Workspace foundation; verified Rust/Cargo package boundary; selected desktop host; fixed page profile and native schema decisions.
- **Known risks:** Unknown current implementation baseline; font metric drift; role/layout coupling; recovery semantics; unverified commands.
- **Acceptance evidence placeholders:** `TODO: link headless tests, desktop smoke evidence, round-trip fixtures, and recovery failure-injection evidence after implementation.`

## Revision history

| Date | Change | Evidence/rationale |
|---|---|---|
| 2026-10-25 | Created stable MVP feature baseline and acceptance IDs | User-approved product goal and `.analysis` recommendations translated into implementation-agnostic requirements |

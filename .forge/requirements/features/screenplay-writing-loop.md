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
- **AC-WRITING-001.4a:** Enter shall use the approved matrix: General/Note→same role, Scene Heading→Action, Action→Action, Character→Dialogue, Dialogue→Action at the collapsed UTF-8-safe end of the Dialogue, Dialogue→Dialogue when the collapsed caret is at an interior UTF-8 boundary, Parenthetical→Dialogue, and Transition→Scene Heading. Tab shall use Scene Heading→Action, Action→Character, blank Character→Transition, Transition→Scene Heading, Dialogue→Parenthetical, and Parenthetical→Dialogue; other Tab cases are consumed no-ops with no literal tab, mutation, or undo entry.

**Approved WO-008 refinement:** The Dialogue interior branch is a transactional split: the original element retains the exact prefix and stable ID, the new Dialogue retains the exact suffix with a bounded distinct ID, and the collapsed selection moves to the new element at byte offset zero. Empty Dialogue is an end-boundary case; rejected non-collapsed, invalid-boundary, and capacity operations preserve session state.
- **AC-WRITING-001.4b:** Up/Down shall navigate adjacent wrapped visual lines using layout source ranges, retain a preferred horizontal goal across successful repeated moves, clamp to shorter lines and document/page edges, and support Shift selection. The transient goal resets after horizontal movement, edits, pointer selection, Home/End, role changes, undo/redo, or a boundary no-op.
- **AC-WRITING-001.5:** Given malformed or out-of-bound input, the system shall return a bounded error without corrupting the project or session state.

**Approved WO-005 refinement:** Entering a Parenthetical through an explicit role transition or assignment creates transient generated `()` with the caret between delimiters. Enter/Tab removes that pair only while its stable element remains marked, Parenthetical, and exactly `()`; user-authored or pasted delimiters, partial edits, and non-empty text are preserved, and the provenance is not serialized.

**Approved WO-004 refinement:** The canonical-text rule remains semantic and UTF-8-safe. For new projects, the persisted `uppercase_cues` policy defaults enabled and applies only at transactional edit boundaries to Scene Heading and Character text; opening legacy or existing projects does not rewrite stored text, and other roles preserve source casing. The setting is bounded and decodes as enabled when absent for legacy compatibility.

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

### REQ-WRITING-LOOP-005: Desktop command toolbar

**User Story:** As an individual screenwriter, I want desktop commands in a
consistent toolbar, so common project actions are discoverable without changing
the semantic editing model.

**Acceptance Criteria:**

- **AC-WRITING-005.1:** Given the desktop app is running, the toolbar shall
  present the typed default Item order New, Open, Save, Save As, Flexible Space,
  Elements, Flexible Space, and Export PDF.
- **AC-WRITING-005.2:** Given an actionable toolbar Item is activated, the
  desktop shall route it through existing application/session adapters and
  preserve command safety, status, persistence, and fixed-layout PDF behavior.
- **AC-WRITING-005.3:** Action Items shall use an icon-only, borderless button
  as the hover/click surface, with a separate centered text label beneath it
  that retains its normal text color; text labels remain available for tooltip
  and accessibility use.
- **AC-WRITING-005.4:** Given the current caret/focus is within a screenplay
  element, the Elements Item shall display that element's explicit role using
  the seven supported labels Scene Heading, Action, Character, Dialogue,
  Parenthetical, Transition, and Note; choosing a label shall apply the role
  through the existing transactional session operation and preserve the
  document on rejection.
- **AC-WRITING-005.5:** Given a toolbar width, normal Items and Space shall
  consume one deterministic 72-point normal-item width, Flexible Space Items
  shall divide the remaining width equally after the shared 4-point row
  padding and 4-point inter-item gap budget, and a narrow toolbar shall retain
  the leftmost fitting Items while exposing non-spacing Items through a far-right
  `>>` text menu.
  Action entries and Elements role choices in that menu shall remain executable
  without persisting toolbar layout state; a fully fitting composition shall
  keep its rightmost Item inside the available width.

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

### Approved WO-007 refinement

`PageProfile` schema version 1 persists default-on `auto_parenthetical` and `strict_transition_boundaries` booleans; absent legacy keys default true and malformed types are typed load errors. Parenthetical Tab/Enter/Delete behavior, supported delimiter repair, before-opening insertion, and strict UTF-8-safe Scene Heading/Character/Transition boundaries are transactional, undoable, and rejected commands are successful no-ops without history.

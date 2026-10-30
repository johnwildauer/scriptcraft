# Feature Requirements: Dependable Screenplay Writing Loop

## Document status

- **Status:** `draft` — stable MVP requirement baseline; independent requirements review not performed
- **Owner:** Forge initialization agent
- **Last updated:** 2026-10-30
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

- **Observed at initialization (2026-10-25):** The checkout had planning analysis describing this loop; no ScriptCraft implementation files were found in the inspected app/crate directories.
- **Current implementation evidence:** A native Rust desktop app and reusable core now exist. The 2026-10-30 UI Polish work orders independently validated the four desktop behaviors documented in REQ-WRITING-LOOP-006; this does not establish native OS notification/fullscreen delivery or validate unrelated requirements.
- **Inferred:** The analysis represents the intended product direction and a useful architecture hypothesis, not a current contract.
- **Desired:** The behaviors and acceptance criteria below are the product contract. This document remains `draft`; no independent requirements review is claimed by the UI Polish validations.

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

- **AC-WRITING-001.1:** Given a new project, the system shall provide exactly one empty, valid editable Scene Heading starting element and title-page metadata without requiring visual formatting tricks. The initial caret shall be valid at element 0, byte offset 0.
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
- **AC-WRITING-005.3:** Action Items shall use an icon-first, borderless icon
  control with a small explicit padding region as the hover/click surface. The
  centered text label beneath it remains outside that response, retains its
  normal text color, remains available for accessibility use, and does not add a
  redundant visible-label tooltip.
- **AC-WRITING-005.4:** Given the current caret/focus is within a screenplay
  element, the Elements Item shall display that element's explicit role using
  the seven supported labels Scene Heading, Action, Character, Dialogue,
  Parenthetical, Transition, and Note; choosing a label shall apply the role
  through the existing transactional session operation and preserve the
  document on rejection.
- **AC-WRITING-005.5:** Given a toolbar width, Space shall consume one
  deterministic 72-point width, rendered normal Items shall consume their
  complete group width including label text measured at runtime with the active
  egui style/font, and Flexible Space Items
  shall divide the remaining width equally after the shared 4-point horizontal
  row-padding and 4-point inter-item-gap budget, and a narrow toolbar shall
  retain the leftmost fitting Items while exposing non-spacing Items through a
  far-right `>>` text menu.
  Action entries and Elements role choices in that menu shall remain executable
  without persisting toolbar layout state; a fully fitting composition shall
  keep its rightmost Item, including its label, inside the actual clipped
  toolbar content right edge after panel, frame, and viewport insets are
  applied. Items with an approved typed width override, including the Elements
  control, shall use that same resolved width for allocation and rendering
  without changing the shared padding, gap, flexible-space, overflow, or
  clipped-edge budget.
- **AC-WRITING-005.6:** Given the desktop app is running, a dedicated compact
  title strip shall display centered bold `ScriptCraft` placeholder text above
  the toolbar. A double-click shall request maximize/unmaximize only when the
  viewport's maximized state is known; unavailable or ignored state shall use
  a bounded no-op/native fallback rather than an unsafe or exact-restoration
  claim that the platform cannot support.
- **AC-WRITING-005.7:** Toolbar action controls and the Elements control/label
  surface shall request the standard arrow cursor, while the existing whole-
  document editing response shall request the text/I-beam cursor. Decorative
  labels and painted page rectangles shall not create competing cursor owners.

> **Unresolved implementation discrepancy:** AC-WRITING-005.6 remains the product requirement for a separate app-owned title strip. The current desktop source renders the toolbar directly below the native titlebar and does not render that app-owned strip. The status-bar initiative's WO-008 removed only the distinct project-metadata heading below the toolbar; it did not remove or disposition AC-WRITING-005.6. This discrepancy is not resolved by the UI Polish work orders.

### REQ-WRITING-LOOP-006: Desktop status bar and visible page

**User Story:** As an individual screenwriter, I want the desktop status bar to
show the page I can currently see and concise, trustworthy document/operation
state, so that I can understand my viewport and project state without moving my
caret or losing access to errors.

**Acceptance Criteria:**

- **AC-WRITING-006.1:** The desktop status presentation shall support distinct
  noninteractive-text, clickable-text, and clickable-icon Item capabilities.
  Interactive Items shall have explicit accessible names and support keyboard
  focus and pointer or keyboard activation; noninteractive text shall not own
  focus, activation, pointer response, or cursor behavior. The right-hand
  controls shall be Sprint Timer, Focus Mode, Appearance, and page zoom in that
  order, whether presented inline or through the status overflow disclosure.
  Each control shall appear as one actionable representation at a time; no other
  status action or icon assignment shall be introduced without authorization.
- **AC-WRITING-006.2:** The status bar shall report the live editing session's
  clean/unsaved state and current operation status, without displaying a
  cumulative frame count or project path. A clean in-memory project without a
  named path shall not be described as previously saved; it may be labeled
  `Clean` without asserting that it has been saved.
- **AC-WRITING-006.3:** Operation errors shall remain visibly identifiable, and
  bounded diagnostics shall remain reachable without exposing screenplay text,
  source text, or serialized project contents. Diagnostics is retained for the
  current release scope, with its removal deferred until pre-release.
- **AC-WRITING-006.4:** At narrow widths where the status controls remain usable,
  a deterministic bounded fallback shall retain access to visible-page state,
  clean/unsaved state, operation/error status, privacy-safe Diagnostics, Sprint
  Timer, Focus Mode, Appearance, and both page-zoom controls through inline
  content or the accessible overflow disclosure. Error state shall remain
  identifiable and its details reachable. Status text shall clip horizontally as
  single-line content rather than wrap or grow the footer. Below the minimum
  usable viewport, the interface shall remain bounded and avoid overlapping or
  unsafe interactions; no clickable-control guarantee is made for a physically
  unusable or zero-width viewport.
- **AC-WRITING-006.5:** Given the actual rendered sheet rectangles and actual
  editor scroll clip, the visible-page Item shall use one-based numbering and
  select the sheet with the greatest vertical overlap; exact overlap ties shall
  select the earlier rendered sheet. If no sheet overlaps, it shall select the
  nearest vertical interval, with exact-distance ties selecting the earlier
  sheet. The total shall include every rendered sheet, including an optional
  title page.
- **AC-WRITING-006.6:** Visible-page state shall be independent of caret/source
  position and remain current after scrolling, resize, presentation-scale
  changes, layout reflow, and conditional caret reveal. If layout or geometry is
  invalid, missing, or stale, the status bar shall show an explicit unavailable
  state until fresh geometry is published. Page-status updates shall not scroll
  the editor, reveal the caret, steal focus, or change editor input ownership.
- **AC-WRITING-006.7:** The desktop status bar shall provide one accessible,
  apparently continuous whole-percent page zoom slider from 75% through 300%
  and an adjacent menu containing exactly 75%, 90%, 100%, 125%, 150%, 175%,
  200%, 250%, and 300%. Both controls shall share one transient per-window value
  and show every intermediate integer accurately. At 100%, the page shall have a
  fixed 360-logical-point presentation width independent of viewport width; the
  selected percentage shall be retained across resizes. The page shall be
  centered within the actual editor viewport when it and its 16-point exterior
  side buffers fit, and remain reachable by horizontal as well as vertical
  navigation when they do not. Zoom shall scale only page presentation without
  changing editor source mapping or focus/IME ownership. Zoom shall not persist
  or alter native project data, session revision, saved/dirty or undo state,
  fixed pagination/source ranges, or fixed-layout PDF output, and shall not
  globally scale toolbar/status controls.
- **AC-WRITING-006.8:** With status content and viewport size unchanged, the
  bottom status panel shall remain height-stable across 60 consecutive egui
  frames at normal (1200 × 820 logical points) and compact (480 × 820 logical
  points) widths. After frame 1, the panel-height range across frames 2–60 shall
  be no greater than 1 logical point, with no monotonic growth; status controls
  and content shall remain available and the panel shall leave both toolbar and
  editor visible.
- **AC-WRITING-006.9:** The rendered status content and expanded Diagnostics
  summary shall remain single-line and clipped to their allocated regions at
  wide, breakpoint-adjacent, compact, and narrow widths. Changes in long status
  text or diagnostic counters shall not add rows or increase stable footer
  height; opening Diagnostics may add only its fixed disclosure row.
- **AC-WRITING-006.10:** A supported native inward/outward pinch event shall
  adjust the same transient per-window whole-percent page zoom as the status
  slider and presets, preserving the inclusive 75–300% limits and all document,
  session, pagination, export, and editor-input invariants in AC-WRITING-006.7.
  Inward pinch is the negative direction of the backend's supported zoom event;
  delivery is platform/backend dependent and no distinct squeeze event is
  promised. A valid finite positive OS-native pixels-per-point value, including
  1.0, shall be applied once with UI zoom 1.0; when the native value is absent or
  invalid, the effective UI scale shall use the 2.3 fallback. A later valid
  native value shall replace the fallback. UI scale is independent of page zoom
  and shall not multiply native scale twice.
- **AC-WRITING-006.11:** Appearance shall offer exactly System, Day Mode, Night
  Mode, and Midnight Mode through an accessible control and begin each app
  window in transient System mode. System shall follow the theme resolved by the
  desktop framework, including changes reported while selected; when no system
  preference is reported, the framework's resolved fallback shall be used
  without claiming an observed OS theme. Day shall use a light page and
  Night/Midnight dark pages; Midnight's page and major app surfaces shall be
  measurably darker than Night. Page text and IME text shall retain at least
  4.5:1 contrast against their composited backgrounds, selected text at least
  4.5:1 against the selection, and the caret and visible selection boundary at
  least 3:1 against adjacent page colors. Appearance changes shall not persist
  and shall not change screenplay data, layout/source mappings, session
  revision, dirty/undo state, page zoom, or PDF/plain-text output.
- **AC-WRITING-006.12:** Sprint Timer shall be an accessible per-window status
  action with mutually exclusive Countdown and Stopwatch modes. Countdown shall
  default to 25 minutes, accept whole minutes from 1 through 1440, preserve
  invalid input without starting, and support Start, Pause, Resume, End,
  completion, and Reset. Ending an active/paused countdown shall restore the
  captured input; running/paused timers shall continue across popup dismissal,
  while idle/completed dismissal and Reset shall restore the 25-minute default.
  Stopwatch shall support Start, Pause, Resume, and End and display elapsed
  hours without wrapping. Timer state shall not persist across app relaunch or
  change screenplay/project, editor, zoom, save, undo, pagination, or export
  state. Completion shall show the captured duration and request at most one
  best-effort OS notification per completed sprint; an accepted request shall
  not be described as delivered, and notification unavailability shall not
  remove the in-app completion result.
- **AC-WRITING-006.13:** Focus Mode shall be an accessible per-window status
  action that requests entry or exit only when the window reports a known
  fullscreen state. An accepted request shall be represented as pending until
  subsequent reports permit reconciliation; unknown, pending, or unresolved
  state shall not be treated as a successful transition, automatically retried,
  or toggled by repeated activation. Subsequent window reports, including
  OS-originated changes, shall determine the available action. Focus Mode shall
  not change screenplay/project data, editor state, page zoom, pagination, or
  exports. Native transition completion, green-button equivalence, and exact
  window/monitor restoration are not guaranteed by a reported state or command
  request.

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

| 2026-10-28 | Added desktop status-bar and viewport-visible-page acceptance criteria (`REQ-WRITING-LOOP-006`) | Approved status-bar initiative WO-002 adds desktop-only item, status, accessibility, narrow-width, and actual-viewport page contracts while preserving fixed core layout/source-mapping requirements |
| 2026-10-28 | Added transient desktop page-zoom acceptance criterion (`AC-WRITING-006.7`) | Approved status-bar initiative WO-003 implementation; page-only 75–300% responsive zoom, exact presets, two-axis access, and fixed document/export invariants |
| 2026-10-29 | Revised desktop status content and clipping criteria (`AC-WRITING-006.2`–`AC-WRITING-006.4`, `AC-WRITING-006.9`) | WO-005 removes the visible frame/path/initial-placeholder claims, retains Diagnostics until pre-release, and defines clipped single-line content while preserving page, zoom, state, and error behavior |
| 2026-10-30 | Reconciled desktop status, appearance, timer, focus, scale, and page-presentation criteria (`AC-WRITING-006.1`, `.4`, `.7`, `.10`–`.13`) | UI Polish WO-001–004 implementations and separate passing validations; rendered/source evidence does not establish native notification delivery, fullscreen completion, or native visual smoke |

### Approved WO-007 refinement

`PageProfile` schema version 1 persists default-on `auto_parenthetical` and `strict_transition_boundaries` booleans; absent legacy keys default true and malformed types are typed load errors. Parenthetical Tab/Enter/Delete behavior, supported delimiter repair, before-opening insertion, and strict UTF-8-safe Scene Heading/Character/Transition boundaries are transactional, undoable, and rejected commands are successful no-ops without history.

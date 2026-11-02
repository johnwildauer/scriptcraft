# Requirements Area: Dependable Screenplay Writing Loop

## Area purpose

ScriptCraft enables an individual screenwriter to create and edit a semantically structured
screenplay, inspect predictable fixed-page layout, and preserve the native project through safe save
and reopen. The writing loop protects screenplay meaning and the last valid native project while
keeping review exports distinct from that source of truth.

## Capability groups

### Semantic screenplay editing

Writers create and revise an ordered screenplay whose explicit roles and canonical text remain
meaningful independently of visual formatting. Editing and navigation preserve valid text boundaries
and predictable role transitions.

- **REQ-WRITING-LOOP-001: Semantic screenplay editing.** The system shall let an individual
    screenwriter create and edit an ordered screenplay with explicit element roles so the draft
    retains screenplay meaning while they write.
  - **AC-WRITING-001.1:** Given a new project, the system shall provide exactly one empty, valid
    editable Scene Heading starting element and an initial title-page structure without requiring
    visual formatting tricks. The initial caret shall be valid at element 0, byte offset 0.
  - **AC-WRITING-001.2:** Given an element, the system shall preserve its explicit role and exact
    canonical UTF-8 text independently of capitalization, spaces, tabs, indentation, or font
    appearance. Typing, paste, split, join, role assignment, Tab transitions, undo, and redo shall
    retain the writer's or imported source's exact text casing; editing shall not normalize cue
    text. `AllCaps` is display-only and shall not rewrite canonical text, native FDX text, or source
    offsets, including when Unicode uppercase expands a source scalar. Opening an existing project
    shall not rewrite stored text, and source FontSpec overrides without `AllCaps` shall display
    canonical case.
  - **AC-WRITING-001.3:** Given a valid selection, the system shall support insertion, deletion,
    split, join, role change, undo, and redo while preserving valid text boundaries and element
    order.
  - **AC-WRITING-001.4:** Given a role transition such as Scene Heading → Action or Character →
    Dialogue, the system shall apply an explicit, testable transition policy and make the
    operation undoable.
  - **AC-WRITING-001.4a:** Application-default Enter targets shall be General → General, Scene
    Heading → Action, Action → Action, Character → Dialogue, Parenthetical → Dialogue, Dialogue →
    Action at the collapsed end or Dialogue at a collapsed interior UTF-8 boundary, Transition →
    Scene Heading, Shot → Action, Cast List → Action, New Act → Scene Heading, Sequence → Scene
    Heading, End of Act → New Act, Summary → Summary, Outline 1/2/3 → Summary, and Note → Summary.
    Except for Dialogue's interior rule, Enter uses the existing transactional split path, including
    when the target role equals the current role. The Dialogue interior branch shall retain the
    exact prefix and existing in-session identity in the original element, retain the exact suffix
    in a new Dialogue with a bounded distinct in-session identity, and move the collapsed selection
    to the new element at byte offset zero. Empty Dialogue shall be an end-boundary case.
    Application-default Tab targets shall be General → General, Scene Heading → Action, Action →
    Character, blank Character → Transition, Parenthetical → Dialogue, Dialogue → Parenthetical,
    Transition → Scene Heading, Shot → Action, Cast List → Action, New Act → Scene Heading,
    Sequence → Scene Heading, End of Act → New Act, Summary → Outline 1, Outline 1 → Outline 2,
    Outline 2 → Outline 3, Outline 3 → Note, and Note → Summary. Tab is always consumed and never
    inserts a literal tab; same-role targets and existing special-case no-ops do not mutate or add
    history. Runtime uses shared application defaults, not imported per-document Behavior;
    imported Behavior remains source-preserved. Rejected non-collapsed selections, invalid
    boundaries, and capacity operations shall preserve session state. Automatic parentheticals and strict transition
    boundaries are runtime editing policies that default to enabled for a new session; they are
    not persisted as document fields. When automatic parentheticals are enabled and an explicit
    role transition or assignment creates a Parenthetical, automatic behavior shall create
    transient generated `()` with the caret between the delimiters and provenance keyed to the
    current in-session identity;
    provenance shall participate in undo/redo but not be serialized. Successful content-changing
    edits shall invalidate the mark, and failed edits shall not. Generated-pair Enter cleanup
    shall apply only while the marked element remains Parenthetical and contains exactly `()`;
    this behavior is distinct from normal Tab/Delete handling. Pasted parentheses shall remain
    intact during paste and thereafter follow normal document Tab/Delete handling, without
    provenance-based protection. With automatic parentheticals enabled, Tab shall handle
    delimiter-only `(`, `)`, and `()` as an empty Dialogue transition regardless of provenance;
    Delete shall handle delimiter-only Parentheticals using the preceding-element or
    first/only-element fallback. Nonempty authored content shall be preserved. Supported
    delimiter repair and before-opening insertion shall follow the configured parenthetical
    policy. Parenthetical Tab/Enter/Delete behavior and strict UTF-8-safe Scene
    Heading/Character/Transition boundaries shall be transactional and undoable; rejected
    commands shall be successful no-ops without history. Disabling automatic parentheticals
    shall honor the configured setting without changing the distinct generated-pair Enter rules.
  - **AC-WRITING-001.4b:** Up/Down shall navigate adjacent wrapped visual lines using layout source
    ranges, retain a preferred horizontal goal across successful repeated moves, clamp to
    shorter lines and document/page edges, and support Shift selection. The transient goal shall
    reset after horizontal movement, edits, pointer selection, Home/End, role changes,
    undo/redo, or a boundary no-op.
  - **AC-WRITING-001.5:** Given malformed or out-of-bound input, the system shall return a bounded
    error without corrupting the project or session state.

### Shared deterministic writing and page views

Continuous writing and fixed-page preview use shared layout semantics so the same semantic content
and effective runtime layout inputs produce repeatable pagination and source mapping independent of
viewport presentation. This does not promise exact layout fidelity for every FDX document.

- **REQ-WRITING-LOOP-002: Shared deterministic writing and page views.** The system shall let a
    screenwriter write continuously and inspect a predictable fixed-page result that agrees about
    semantic content and layout inputs.
  - **AC-WRITING-002.1:** Given the same semantic content and effective runtime layout inputs, the
    system shall derive repeatable page structure and source mappings independent of window zoom.
    This does not promise exact layout fidelity for every imported FDX document.
  - **AC-WRITING-002.2:** Given role-aware layout, the system shall apply role measures and keep
    relationships without inserting spaces or tabs into canonical text.
  - **AC-WRITING-002.3:** Given long text or an oversized element, the system shall wrap or split it
    safely without an infinite loop or unbounded blank-page behavior.
  - **AC-WRITING-002.4:** Given a document position and rendered geometry, the system shall support
    a safe caret/hit-test round trip at valid text boundaries.
  - **AC-WRITING-002.5:** Given a body role and source-backed role `ParagraphSpec`, layout shall
    resolve supported alignment, signed first-line indent, left/right paragraph positions,
    `SpaceBefore`, mapped `Leading`/`Spacing`, and `StartsNewPage` independently against the
    shared defaults. Effective geometry and vertical fit shall use signed fixed-point hundredths
    of a point; desktop editing, hit testing, navigation, selection, PDF coordinates, and cached
    layouts shall consume the same geometry. Unsupported, conflicting, out-of-budget, or unusable
    fields shall remain preserved in FDX and fall back deterministically. Alignment and indentation
    shall not modify canonical UTF-8 text or source ranges. This remains a bounded interpretation
    of the supported role fields, not a promise of exact Final Draft pagination.
  - **AC-WRITING-002.6:** Given a body role and source-backed `FontSpec`, the system shall resolve
    supported family, bounded positive size, and the `Bold`, `Italic`, `Underline`, `AllCaps`, and
    `HiddenText` style tokens independently over the shared role defaults. Desktop typography shall
    use the bundled Courier Prime faces with fixed-cell metrics (7 character-width units and 12 line
    advance units at 12 points, scaled by effective role size); Courier Final Draft maps to Courier
    Prime, while Arial and unsupported families use the same deterministic fallback without
    rewriting the source family. Unsupported values and unknown tokens remain preserved and use
    deterministic fallback. This is not a claim of proportional-font measurement or exact FDX
    visual parity.
  - **AC-WRITING-002.7:** `AllCaps` shall transform display text only, retaining canonical text
    and valid source-byte boundaries when Unicode uppercase expands a source scalar. HiddenText roles
    shall remain visible, editable, and contrast-safe/subdued in editor-purpose layout, but be
    omitted from print/PDF and plain-text export-purpose layout without renumbering source elements.
    A role made visible by source settings shall use ordinary role presentation. PDF may apply only
    the supported role style subset using deterministic non-embedded Courier resources; this does
    not establish Final Draft interoperability or change the existing non-ASCII substitution
    limitation.

  - **AC-WRITING-002.8:** For managed body paragraphs, each supported direct `Paragraph` field
    (`Alignment`, `FirstIndent`, `Leading`, `LeftIndent`, `RightIndent`, `SpaceBefore`, `Spacing`,
    and `StartsNewPage`) shall override only that field's role setting, which in turn overrides the
    shared default. Direct values shall be associated with the paragraph's managed semantic identity;
    invalid values shall remain source-preserved and fall back independently. Direct geometry shall
    not alter role typography, visibility, title-page interpretation, or run formatting. Splitting
    shall not copy direct XML attributes to a new paragraph, and a join that would remove a paragraph
    carrying supported direct-format attributes shall stop transactionally rather than lose them.

### Durable native project persistence

The versioned native project is the semantic source of truth. Successful save and reopen preserve
project meaning; failures preserve the prior valid destination and unsaved state.

- **REQ-WRITING-LOOP-003: Durable native save and reopen.** The system shall let an individual
    screenwriter save and reopen a project without losing its last valid version.
  - **AC-WRITING-003.1:** Given a valid project, the system shall serialize native FDX Version 6
    containing semantic content and retained FDX-owned source data, including source metadata,
    foreign paragraph identities, title-page structure, document settings, and supported opaque
    content, while excluding transient caret, layout, undo state, and internal editing identities
    from required content. Newly created native documents shall include one ordered ElementSettings
    block for each of the 17 directed roles, using the shared defaults evidenced by the approved
    native references; imported source-backed settings remain authoritative and shall not be
    regenerated merely because runtime interpretation changes. Generated sections shall be ordered
    Content, then the 17 ElementSettings blocks in directed role order, then TitlePage when present.
    Serialized output shall begin with exactly one reference-compatible UTF-8 standalone XML
    declaration; this lexical declaration is emitted once on each save/re-serialization. The
    generated section order shall not reorder imported top-level or opaque source content.
    Newly generated ScriptCraft structure shall use LF line endings, two ASCII spaces per markup
    indentation level, blank lines between direct FinalDraft children and adjacent ElementSettings,
    no blank lines between Content paragraphs, and one final LF. Generated whitespace may be added
    only in element-only generated structure. Newly inserted or split managed Content paragraphs
    shall receive LF/two-space sibling indentation only where the Content child sequence is
    unambiguously Paragraph elements plus structural whitespace and no effective `xml:space="preserve"`
    applies. Any detached structural-whitespace normalization shall be applied equally to the
    expected source-tree comparison; re-read integrity checks remain strict. Canonical Text content,
    including empty/whitespace-only values, tabs, LF and CR, and CDATA, comments, processing
    instructions, mixed content, namespaces, unknown imported structures, and source order shall not
    be rewritten. Ambiguous mixed-content and `xml:space` contexts are preserved conservatively;
    imported opaque structures are never globally pretty-printed. If a new/split managed paragraph
    requires sibling formatting in an ambiguous or `xml:space`-preserving context, serialization
    shall fail safely rather than emit ambiguous formatting or alter source content. This formatting
    boundary does not promise lexical identity or blanket Final Draft compatibility.
    ScriptCraft-only project identity/schema, metadata, layout inputs, and
    editing policies are not persisted as FDX fields; no private extension, sidecar, or legacy JSON
    migration is required. A source field without a safe supported representation shall not be
    silently lost.
  - **AC-WRITING-003.2:** Given a save preparation, conflict or replacement failure, the system
    shall preserve the prior valid destination and the active session, path and clean baseline;
    the live session's dirty/clean state shall remain accurate, including retaining dirty state for
    previously modified work and retaining clean state for an initially clean session. The failure
    shall be reported with document-unchanged wording and shall not be presented as a successful
    save to the attempted destination. The status presentation does not duplicate session-owned
    dirty/clean state with a separate Clean/Dirty indicator.
  - **AC-WRITING-003.3:** Given a successfully saved native project, reopening it shall preserve
    semantic roles, text, retained FDX-owned metadata, foreign paragraph identities, title-page
    structure, supported document settings, and retained source content subject to the explicit
    version and independence boundaries below. Transient ScriptCraft editing identities may be
    regenerated; ScriptCraft-only legacy metadata, schema, nondefault layout configuration, and
    editing policies may reset and are not a persistence guarantee. Pagination shall be repeatable
    for the same effective runtime layout inputs, without promising exact fidelity to every FDX
    layout. Absent supported fields shall use their specified defaults; malformed input shall fail safely.
    Foreign paragraph identity lifecycle follows AC-WRITING-003.6: established IDs remain stable
    through ordinary edits, role and scene-cast refresh, Save/Save As, and reopen; structural edits
    remove or create only the affected paragraph identities, and session undo/redo restores its exact
    prior identity snapshot.
  - **AC-WRITING-003.5:** Every managed Scene Heading shall contain `SceneProperties` and
    `SceneArcBeats` before its first `Text` child. Missing properties shall use Length `1/8`, Page
    `1`, and empty Title; existing values and unknown attributes remain source-owned, and Page or
    Length is not recalculated. Scene-cast arcs are a derived projection of nonblank Character
    elements until the next heading: outer Unicode whitespace is trimmed, duplicate names are
    compared with Unicode lowercase keys, and first spelling/occurrence order is retained. Each
    supported entry contains the reference-shaped nested Paragraph; its source ID remains opaque
    and is registered in the preservation identity graph without becoming a managed screenplay
    element or anchor. Matching entries retain their nested Paragraph ID. Removal requires
    positively tracked ScriptCraft creation ownership, a supported entry shape, and no known or
    source-tree ID reference; source entries whose ownership cannot be established, opaque nested
    authored payloads, ambiguous entries, or unsafe references remain intact and stop scene
    reconciliation. Ownership is not inferred from shape on reopen. Existing Scene Heading
    split/join attachment safe-stops remain unchanged.
  - **AC-WRITING-003.6:** Every newly generated FDX paragraph identity (managed body and title-page
    paragraphs and nested scene-arc Paragraphs) shall be a lowercase, hyphenated UUID version 4.
    Allocation shall use secure operating-system randomness, reject collisions against current
    preservation identities and source ID attributes, and stop safely after a bounded retry limit
    or entropy failure. Existing imported identities, including non-UUID values, remain verbatim;
    no migration or historical tombstone registry is used. Newly allocated identities become
    source-owned once established by save/re-read and remain unchanged in later edits and saves.
    Serializing the same never-saved, preservation-free project more than once may allocate
    different first-time identities; identity stability is promised after source identities have
    been established, not as byte-for-byte determinism for first-time output.
    Ordinary text edits, role changes, scene-cast refresh, Save, Save As, and reopen retain
    established paragraph IDs. Splitting retains the original/left ID and allocates one ID for the
    right paragraph; joining or deleting removes only the removed paragraph's ID. Undo/redo within
    the active session restores the exact identity state in its history snapshots. Content recreated
    after deletion receives a new allocation, but with no historical tombstone registry the
    allocator cannot guarantee uniqueness against every ID deleted in the past. Matching scene-arc
    names retain nested IDs; new or renamed names receive fresh allocations and safely owned removed
    entries lose their IDs. If a stale entry is encountered after a separate reopen without creation
    proof, reconciliation stops atomically and preserves it rather than guessing at ownership.

### FDX document content and preservation

- **REQ-WRITING-LOOP-007: FDX document model.** The system shall represent and edit the ordered
  main-content types Scene Heading, Action, Character, Parenthetical, Dialogue, Transition, Note,
  New Act, End of Act, Cast List, General, Shot, Sequence, Outline 1, Outline 2, Outline 3 and
  Summary without silently changing their type, source text, case or ordering on import.
  - **AC-WRITING-007.1:** Each type remains distinct through editing and Version 6 save/reopen;
    explicit role assignment and safe application-default Enter/Tab policies are available, with
    role transitions following AC-WRITING-001.4a. Absent recognized role visibility and page-start settings use the
    shared native defaults; explicit imported values remain source-backed. Hidden-text settings are honored in print output rather than
    exposing hidden notes and outline content.
  - **AC-WRITING-007.2:** Ordered runs, styles, attributes and unknown content remain available
    across safe edits, undo/redo and save/reopen. Unsupported intersections are safely stopped
    or escalated when preservation cannot be established; no authored material is silently removed.
  - **AC-WRITING-007.3:** An imported title page retains its ordered paragraphs, blank lines,
    alignment, styles and separate header/footer. Title-page editing is deferred; metadata is not
    inferred from paragraph positions.
  - **AC-WRITING-007.4:** Beat boards and script content, including outlines and scene beats, are
    independent: changing one never changes the other. Cross-domain links may be silently orphaned
    on import and do not block script edits. Board items, images and their internal relationships
    remain preserved where possible; an actual conflict between authored-data preservation and
    independence is escalated, not silently resolved by deletion.

- **REQ-WRITING-LOOP-008: Native FDX versions and safety.** The system shall open FDX and save
  Version 6, without claiming universal Final Draft compatibility.
  - **AC-WRITING-008.1:** An explicit non-6 version triggers a Continue/Cancel warning before a
    Version 6 importer attempt: updating may contain errors, lose data and has no compatibility
    guarantee. Previous versions import only title and content, converting unrecognized elements
    to General when possible; missing content or unconvertible elements produce an incompatible
    notification and leave the active session and source unchanged. No partial salvage is implied.
  - **AC-WRITING-008.2:** Newer explicit versions also receive the warning and importer attempt;
    newer-specific conversion is not guaranteed. Missing, malformed or duplicate version data and
    invalid input fail safely rather than being silently assigned a version.
  - **AC-WRITING-008.3:** Opening never changes the source. Explicit Save may replace the original
    with Version 6; Save As may select another destination. Failed preparation, conflict or
    replacement leaves the destination, active model, path and dirty state unchanged. Legacy JSON
    migration and continued JSON writing are not promised.
  - **AC-WRITING-008.4:** Untrusted input is bounded and cannot trigger external entity, network
    or local image-path access; diagnostics do not expose document text, image payloads or paths.

- **REQ-WRITING-LOOP-009: Scene cast lists.** Imported Cast List text is assigned as-is. The
  internal updater collects Character names after each Scene Heading to the next heading or end,
  deduplicates and alphabetically sorts them, and periodically updates managed lists.
  - **AC-WRITING-009.1:** Direct edits may be silently corrected on refresh; Save and Save As check
    and silently correct the serialized candidate before emitting its text as-is.
  - **AC-WRITING-009.2:** After a successful save the live document and clean baseline match the
    corrected output; a failed save publishes no candidate correction. Refresh, edit history and
    pending saves are transactional and deterministic.

### Secondary Fountain conversion

- **REQ-WRITING-LOOP-010: Fountain conversion.** Fountain import uses a fixed documented
  conversion protocol to create a new unsaved native document; export produces a separate lossy
  copy without changing native path or clean state. Import retains as much parseable writer data
  as the protocol supports. Potential conversion loss does not require a warning; fatal failures
  are reported. Fountain is not the native format.

### Explicit noncanonical exports

PDF and plain-text copies support review and sharing without replacing or changing the saved state
of the native source. Plain text communicates the semantic and layout information it loses.

- **REQ-WRITING-LOOP-004: Explicit MVP exports.** The system shall provide a reviewable PDF and a
    plainly lossy text copy without confusing either export with the native source.
  - **AC-WRITING-004.1:** Given a project, PDF export shall use fixed print layout rather than
    transient viewport layout and shall preserve non-ASCII characters.
  - **AC-WRITING-004.2:** Given a PDF or plain-text export, the system shall not silently replace
    the native source of truth or falsely mark the native project clean.
  - **AC-WRITING-004.3:** Given a lossy plain-text export, the result shall communicate that roles,
    IDs, and page layout are not preserved as a lossless round trip.
  - **AC-WRITING-004.4:** Plain-text export shall emit each fixed print-layout line's display text,
    including the effective role-level `AllCaps` presentation and Unicode uppercase expansions;
    roles without effective `AllCaps` shall retain their canonical casing. This export-only
    presentation shall not mutate semantic text or native FDX, and shall retain print-layout
    wrapping and HiddenText omission while continuing to report its losses under AC-WRITING-004.3.

### Desktop command toolbar and native header

The desktop toolbar exposes common commands and role selection without changing the semantic editing
model. Native window chrome remains owned by macOS where applicable.

- **REQ-WRITING-LOOP-005: Desktop command toolbar.** The desktop application shall present common
    project commands in a consistent toolbar while preserving command safety and the semantic
    editing model.
  - **AC-WRITING-005.1:** Given the desktop app is running, the toolbar shall present the typed
    default Item order New, Open, Save, Save As, Flexible Space, Elements, Flexible Space, and
    Export PDF.
  - **AC-WRITING-005.2:** Given an actionable toolbar Item is activated, the desktop shall route it
    through application/session adapters and preserve command safety, status, persistence, and
    fixed-layout PDF behavior. New shall not replace a dirty session without the writer's
    explicit informed consent, without prescribing a particular dialog or action sequence.
  - **AC-WRITING-005.3:** Action Items shall use an icon-first, borderless icon control with a small
    explicit padding region as the hover/click surface. The centered text label beneath it shall
    remain outside that response, retain its normal text color, remain available for
    accessibility use, and not add a redundant visible-label tooltip.
  - **AC-WRITING-005.4:** Given the current caret/focus is within a screenplay element, the Elements
    Item shall display that element's explicit role using the supported labels in
    REQ-WRITING-LOOP-007; choosing a label
    shall apply the role through the transactional session operation and preserve the document
    on rejection.
  - **AC-WRITING-005.5:** Given a toolbar width, Space shall consume one deterministic 72-point
    width, rendered normal Items shall consume their complete group width including label text
    measured at runtime with the active egui style/font, and Flexible Space Items shall divide
    the remaining width equally after the shared 4-point horizontal row-padding and 4-point
    inter-item-gap budget. A narrow toolbar shall retain the leftmost fitting Items while
    exposing non-spacing Items through a far-right `>>` text menu. Action entries and Elements
    role choices in that menu shall remain executable without persisting toolbar layout state. A
    fully fitting composition shall keep its rightmost Item, including its label, inside the
    actual clipped toolbar content right edge after panel, frame, and viewport insets are
    applied. Items with an approved typed width override, including the Elements control, shall
    use the same resolved width for allocation and rendering without changing the shared
    padding, gap, flexible-space, overflow, or clipped-edge budget.
  - **AC-WRITING-005.6:** On macOS, the desktop shall use the OS-owned native window header and
    place the toolbar at the top of the window content area below it. Header interactions shall
    remain OS-owned. This criterion does not prescribe a particular OS double-click outcome or
    promise a native visual result.
  - **AC-WRITING-005.7:** Toolbar action controls and the Elements control/label surface shall
    request the standard arrow cursor, while the whole-document editing response shall request
    the text/I-beam cursor. Decorative labels and painted page rectangles shall not create
    competing cursor owners.

### Desktop status bar and visible page

The status bar communicates the live editing session, operation/error state, and page actually
visible in the editor without taking ownership of editing input or exposing private screenplay
content.

- **REQ-WRITING-LOOP-006: Desktop status bar and visible page.** The desktop status bar shall show
    the page currently visible and concise, trustworthy document and operation state while
    preserving access to errors and controls.
  - **AC-WRITING-006.1:** The desktop status presentation shall support distinct
    noninteractive-text, clickable-text, and clickable-icon Item capabilities. Interactive Items
    shall have explicit accessible names and support keyboard focus and pointer or keyboard
    activation; noninteractive text shall not own focus, activation, pointer response, or cursor
    behavior. The right-hand controls shall be Sprint Timer, Focus Mode, Appearance, and page
    zoom in that order, whether presented inline or through the status overflow disclosure. Each
    control shall appear as one actionable representation at a time; no other status action or
    icon assignment shall be introduced without authorization.
  - **AC-WRITING-006.2:** The status bar shall report the current operation status without
    displaying a cumulative frame count, project path, or separate Clean/Dirty indicator. Clean
    and dirty state remain owned by the live editing session and shall not be changed by status
    presentation. A clean in-memory project without a named path shall not be described as
    previously saved.
  - **AC-WRITING-006.3:** Operation errors shall remain visibly identifiable, and bounded
    diagnostics shall remain reachable without exposing screenplay text, source text, serialized
    project contents, or unnecessary filesystem paths. Save errors shall retain a concise
    privacy-safe category distinguishing session preparation, codec serialization/re-read,
    storage, and missing-baseline failures. Save-failure wording shall state that the document is
    unchanged. Diagnostics is retained for the current release scope, with its removal deferred
    until pre-release.
  - **AC-WRITING-006.4:** At narrow widths where the status controls remain usable, a deterministic
    bounded fallback shall retain access to visible-page state, operation/error status,
    privacy-safe Diagnostics, Sprint Timer, Focus Mode, Appearance, and
    both page-zoom controls through inline content or the accessible overflow disclosure. Error
    state shall remain identifiable and its details reachable. Status text shall clip
    horizontally as single-line content rather than wrap or grow the footer. Below the minimum
    usable viewport, the interface shall remain bounded and avoid overlapping or unsafe
    interactions; no clickable-control guarantee is made for a physically unusable or zero-width
    viewport.
  - **AC-WRITING-006.5:** Given the actual rendered sheet rectangles and actual editor scroll clip,
    the visible-page Item shall use one-based numbering and select the sheet with the greatest
    vertical overlap; exact overlap ties shall select the earlier rendered sheet. If no sheet
    overlaps, it shall select the nearest vertical interval, with exact-distance ties selecting
    the earlier sheet. The total shall include every rendered sheet, including an optional title
    page.
  - **AC-WRITING-006.6:** Visible-page state shall be independent of caret/source position and
    remain current after scrolling, resize, presentation-scale changes, layout reflow, and
    conditional caret reveal. If layout or geometry is invalid, missing, or stale, the status
    bar shall show an explicit unavailable state until fresh geometry is published. Page-status
    updates shall not scroll the editor, reveal the caret, steal focus, or change editor input
    ownership.
  - **AC-WRITING-006.7:** The desktop status bar shall provide one accessible, apparently continuous
    whole-percent page zoom slider from 75% through 300% and an adjacent menu containing exactly
    75%, 90%, 100%, 125%, 150%, 175%, 200%, 250%, and 300%. Both controls shall share one
    transient per-window value and show every intermediate integer accurately. At 100%, the page
    shall have a fixed 360-logical-point presentation width independent of viewport width; the
    selected percentage shall be retained across resizes. The page shall be centered within the
    actual editor viewport when it and its 16-point exterior side buffers fit, and remain
    reachable by horizontal as well as vertical navigation when they do not. Zoom shall scale
    only page presentation without changing editor source mapping or focus/IME ownership. Zoom
    shall not persist or alter native project data, session revision, saved/dirty or undo state,
    fixed pagination/source ranges, or fixed-layout PDF output, and shall not globally scale
    toolbar/status controls.
  - **AC-WRITING-006.8:** With status content and viewport size unchanged, the bottom status panel
    shall remain height-stable across 60 consecutive egui frames at normal (1200 × 820 logical
    points) and compact (480 × 820 logical points) widths. After frame 1, the panel-height range
    across frames 2–60 shall be no greater than 1 logical point, with no monotonic growth;
    status controls and content shall remain available and the panel shall leave both toolbar
    and editor visible.
  - **AC-WRITING-006.9:** The rendered status content and expanded Diagnostics summary shall remain
    single-line and clipped to their allocated regions at wide, breakpoint-adjacent, compact,
    and narrow widths. Changes in long status text or diagnostic counters shall not add rows or
    increase stable footer height; opening Diagnostics may add only its fixed disclosure row.
  - **AC-WRITING-006.10:** A supported native inward/outward pinch event shall adjust the same
    transient per-window whole-percent page zoom as the status slider and presets, preserving
    the inclusive 75–300% limits and all document, session, pagination, export, and editor-input
    invariants in AC-WRITING-006.7. Inward pinch is the negative direction of the backend's
    supported zoom event; delivery is platform/backend dependent and no distinct squeeze event
    is promised. A valid finite positive OS-native pixels-per-point value, including 1.0, shall
    be applied once with UI zoom 1.0; when the native value is absent or invalid, the effective
    UI scale shall use the 2.3 fallback. A later valid native value shall replace the fallback.
    UI scale is independent of page zoom and shall not multiply native scale twice.
  - **AC-WRITING-006.11:** Appearance shall offer exactly System, Day Mode, Night Mode, and Midnight
    Mode through an accessible control and begin each app window in transient System mode.
    System shall follow the theme resolved by the desktop framework, including changes reported
    while selected; when no system preference is reported, the framework's resolved fallback
    shall be used without claiming an observed OS theme. Day shall use a light page and
    Night/Midnight dark pages; Midnight's page and major app surfaces shall be measurably darker
    than Night. Page text and IME text shall retain at least 4.5:1 contrast against their
    composited backgrounds, selected text at least 4.5:1 against the selection, and the caret
    and visible selection boundary at least 3:1 against adjacent page colors. Appearance changes
    shall not persist and shall not change screenplay data, layout/source mappings, session
    revision, dirty/undo state, page zoom, or PDF/plain-text output.
  - **AC-WRITING-006.12:** Sprint Timer shall be an accessible per-window status action with
    mutually exclusive Countdown and Stopwatch modes. Countdown shall default to 25 minutes,
    accept whole minutes from 1 through 1440, preserve invalid input without starting, and
    support Start, Pause, Resume, End, completion, and Reset. Ending an active/paused countdown
    shall restore the captured input; running/paused timers shall continue across popup
    dismissal, while idle/completed dismissal and Reset shall restore the 25-minute default.
    Stopwatch shall support Start, Pause, Resume, and End and display elapsed hours without
    wrapping. Timer state shall not persist across app relaunch or change screenplay/project,
    editor, zoom, save, undo, pagination, or export state. Completion shall show the captured
    duration and request at most one best-effort OS notification per completed sprint; an
    accepted request shall not be described as delivered, and notification unavailability shall
    not remove the in-app completion result.
  - **AC-WRITING-006.13:** Focus Mode shall be an accessible per-window status action that requests
    entry or exit only when the window reports a known fullscreen state. An accepted request
    shall be represented as pending until subsequent reports permit reconciliation; unknown,
    pending, or unresolved state shall not be treated as a successful transition, automatically
    retried, or toggled by repeated activation. Subsequent window reports, including
    OS-originated changes, shall determine the available action. Focus Mode shall not change
    screenplay/project data, editor state, page zoom, pagination, or exports. Native transition
    completion, green-button equivalence, and exact window/monitor restoration are not
    guaranteed by a reported state or command request.

## Constraints

- Native project data remains the semantic source of truth; transient caret, layout, undo state, and
    presentation scale do not replace it.
- Pagination and PDF output use fixed runtime layout inputs, independent of transient viewport
  zoom; preservation of source layout settings does not promise exact fidelity to every FDX layout.
- PDF and plain-text exports are noncanonical and shall not replace or mark the native project
    clean. Plain-text export is deliberately lossy and communicates its loss.
- Desktop-only and native macOS behavior is limited to the explicit criteria above.
    Platform-dependent event delivery and unverified native interactions are not guaranteed beyond
    their stated conditions.

## Non-functional requirements

- **NFR-WRITING-001:** Production paths should prefer typed errors and bounded inputs over
    panic-prone handling; a discovered crash or data-loss defect requires a regression test.
- **NFR-WRITING-002:** Core editing, layout, and persistence behavior shall be exercisable without a
    desktop window for deterministic tests.
- **NFR-WRITING-003:** Native serialization and structural pagination outputs shall be deterministic
    for the same semantic content and effective runtime layout inputs.

## Out of scope

- DOCX and blanket Final Draft compatibility, collaboration, MCP, planning/beat-board editing, broad formatting,
    web/multi-platform polish, and all adjacent non-MVP features listed in the product overview.

## Related documents

- **Product overview:** [`../product-overview.md`](../product-overview.md)
- **Technical blueprint:** [`../../blueprints/technical-context.md`](../../blueprints/technical-context.md)
